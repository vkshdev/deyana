use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::{Mutex, RwLock};
use chrono::Utc;
use serde_json::Value;

use crate::db::DbPool;
use super::types::*;

pub struct LocalVoiceService {
    pub data_dir: PathBuf,
    pub settings_path: PathBuf,
    pub settings: RwLock<VoiceSettings>,
    pub db_pool: DbPool,
    pub active_tts_process: Mutex<Option<Child>>,
}

impl LocalVoiceService {
    pub fn new(data_dir: PathBuf, db_pool: DbPool) -> Self {
        let settings_path = data_dir.join("voice-settings.json");
        let initial_settings = if settings_path.exists() {
            match fs::read_to_string(&settings_path) {
                Ok(content) => serde_json::from_str::<VoiceSettings>(&content)
                    .unwrap_or_else(|_| VoiceSettings::default()),
                Err(_) => VoiceSettings::default(),
            }
        } else {
            VoiceSettings::default()
        };

        let service = Self {
            data_dir,
            settings_path,
            settings: RwLock::new(initial_settings),
            db_pool,
            active_tts_process: Mutex::new(None),
        };

        // Resolve voice selection on init
        let resolved = service.resolve_voice(&service.read_settings());
        if let Ok(mut lock) = service.settings.write() {
            *lock = resolved;
        }

        service
    }

    pub fn read_settings(&self) -> VoiceSettings {
        self.settings
            .read()
            .map(|s| s.clone())
            .unwrap_or_else(|_| VoiceSettings::default())
    }

    pub fn patch_settings(&self, patch: VoiceSettingsPatch) -> Result<VoiceSettings, String> {
        let current = self.read_settings();
        let mut tts_voice = current.tts_voice.clone();

        if let Some(v) = patch.tts_voice {
            tts_voice = self.validate_voice_selection(Some(&v))?;
        }

        let next = VoiceSettings {
            enabled: patch.enabled.unwrap_or(current.enabled),
            muted: patch.muted.unwrap_or(current.muted),
            tts_enabled: patch.tts_enabled.unwrap_or(current.tts_enabled),
            transcript_retention: patch.transcript_retention.unwrap_or(current.transcript_retention),
            stt_engine: current.stt_engine,
            tts_engine: current.tts_engine,
            language: patch.language.unwrap_or(current.language),
            listen_seconds: patch.listen_seconds.unwrap_or(current.listen_seconds),
            tts_voice,
            tts_rate: patch.tts_rate.unwrap_or(current.tts_rate),
            tts_volume: patch.tts_volume.unwrap_or(current.tts_volume),
            updated_at: Utc::now().to_rfc3339(),
        };

        self.write_settings(&next)?;
        if let Ok(mut lock) = self.settings.write() {
            *lock = next.clone();
        }

        Ok(next)
    }

    pub fn write_settings(&self, settings: &VoiceSettings) -> Result<(), String> {
        let _ = fs::create_dir_all(&self.data_dir);
        let content = serde_json::to_string_pretty(settings)
            .map_err(|e| format!("Failed to serialize voice settings: {}", e))?;
        fs::write(&self.settings_path, content)
            .map_err(|e| format!("Failed to save voice settings: {}", e))?;
        Ok(())
    }

    pub fn status(&self) -> VoiceStatusResponse {
        let settings = self.read_settings();
        let catalog = self.voice_catalog();
        let provider = self.provider_status();

        let mut stt_status = provider.clone();
        let mut tts_status = provider.clone();
        let mut detail = "Windows local speech APIs are available.".to_string();

        if !settings.enabled {
            stt_status = "disabled".to_string();
            tts_status = "disabled".to_string();
            detail = "Voice is disabled until the user enables it.".to_string();
        } else if settings.muted {
            stt_status = "muted".to_string();
            detail = "Microphone input is muted.".to_string();
        } else if provider != "available" {
            detail = "No supported local speech engine is available on this machine.".to_string();
        }

        if settings.enabled && !settings.tts_enabled {
            tts_status = "disabled".to_string();
        } else if settings.enabled && settings.tts_enabled && settings.tts_voice.is_none() {
            tts_status = "missing".to_string();
            detail = "No installed female text-to-speech voice is available.".to_string();
        }

        VoiceStatusResponse {
            enabled: settings.enabled,
            muted: settings.muted,
            tts_enabled: settings.tts_enabled,
            stt_status,
            tts_status,
            stt_engine: settings.stt_engine,
            tts_engine: settings.tts_engine,
            language: settings.language,
            active_tts_voice: settings.tts_voice,
            available_tts_voices: catalog,
            raw_audio_stored: false,
            detail,
            checked_at: Utc::now().to_rfc3339(),
        }
    }

    pub fn transcribe(&self, request: Option<VoiceTranscriptRequest>) -> Result<VoiceTranscriptResponse, String> {
        let settings = self.read_settings();
        self.require_stt_ready(&settings)?;

        let req = request.unwrap_or(VoiceTranscriptRequest { listen_seconds: None });
        let duration = req.listen_seconds.unwrap_or(settings.listen_seconds);

        let transcript = run_windows_stt(&settings.language, duration)?;

        Ok(VoiceTranscriptResponse {
            transcript,
            engine: settings.stt_engine,
            language: settings.language,
            duration_seconds: duration,
            raw_audio_stored: false,
            created_at: Utc::now().to_rfc3339(),
        })
    }

    pub fn speak(&self, request: VoiceSpeakRequest) -> Result<VoiceSpeakResponse, String> {
        let settings = self.read_settings();
        let text = request.text.trim();
        if text.is_empty() {
            return Err("Speech text is required.".to_string());
        }

        self.require_tts_ready(&settings)?;
        let _ = self.interrupt_speech();

        let child = run_windows_tts(
            text,
            settings.tts_voice.as_deref(),
            settings.tts_rate,
            settings.tts_volume,
        )?;

        {
            if let Ok(mut lock) = self.active_tts_process.lock() {
                *lock = Some(child);
            }
        }

        // Wait for TTS completion or interruption without holding lock during sleep/wait
        let wait_result = loop {
            let status_opt = {
                let mut lock = self.active_tts_process.lock();
                if let Ok(ref mut guard) = lock {
                    if let Some(ref mut c) = **guard {
                        match c.try_wait() {
                            Ok(Some(status)) => Ok(Some(status)),
                            Ok(None) => Ok(None),
                            Err(e) => Err(format!("Text-to-speech execution error: {}", e)),
                        }
                    } else {
                        Err("Speech was cancelled.".to_string())
                    }
                } else {
                    Err("Speech synchronization error.".to_string())
                }
            };

            match status_opt {
                Ok(Some(status)) => {
                    if let Ok(mut guard) = self.active_tts_process.lock() {
                        *guard = None;
                    }
                    break Ok(status);
                }
                Ok(None) => {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(err) => {
                    if let Ok(mut guard) = self.active_tts_process.lock() {
                        *guard = None;
                    }
                    break Err(err);
                }
            }
        };

        match wait_result {
            Ok(status) => {
                if status.success() {
                    Ok(VoiceSpeakResponse {
                        spoken: true,
                        engine: settings.tts_engine,
                        characters: text.len(),
                        raw_audio_stored: false,
                        created_at: Utc::now().to_rfc3339(),
                    })
                } else {
                    Err("Local text-to-speech failed.".to_string())
                }
            }
            Err(e) => Err(e),
        }
    }

    pub fn interrupt_speech(&self) -> VoiceInterruptResponse {
        let child = {
            let lock = self.active_tts_process.lock();
            if let Ok(mut guard) = lock {
                guard.take()
            } else {
                None
            }
        };

        if let Some(mut child) = child {
            let _ = child.kill();
            let _ = child.wait();
            return VoiceInterruptResponse {
                interrupted: true,
                engine: "windows_speech".to_string(),
                detail: "Local speech was interrupted.".to_string(),
                created_at: Utc::now().to_rfc3339(),
            };
        }

        VoiceInterruptResponse {
            interrupted: false,
            engine: "windows_speech".to_string(),
            detail: "No active local speech was running.".to_string(),
            created_at: Utc::now().to_rfc3339(),
        }
    }

    pub fn provider_status(&self) -> String {
        if !cfg!(target_os = "windows") {
            return "unsupported".to_string();
        }
        if find_powershell().is_none() {
            return "missing".to_string();
        }
        "available".to_string()
    }

    fn require_stt_ready(&self, settings: &VoiceSettings) -> Result<(), String> {
        if !settings.enabled {
            return Err("Voice is disabled.".to_string());
        }
        if settings.muted {
            return Err("Microphone input is muted.".to_string());
        }
        if self.provider_status() != "available" {
            return Err("A supported local speech recognition engine is not available.".to_string());
        }
        Ok(())
    }

    fn require_tts_ready(&self, settings: &VoiceSettings) -> Result<(), String> {
        if !settings.enabled {
            return Err("Voice is disabled.".to_string());
        }
        if !settings.tts_enabled {
            return Err("Text-to-speech is disabled.".to_string());
        }
        if self.provider_status() != "available" {
            return Err("A supported local text-to-speech engine is not available.".to_string());
        }
        if settings.tts_voice.is_none() {
            return Err("No installed female text-to-speech voice is available.".to_string());
        }
        Ok(())
    }

    pub fn voice_catalog(&self) -> Vec<VoiceOption> {
        discover_windows_voice_catalog()
    }

    pub fn resolve_voice(&self, settings: &VoiceSettings) -> VoiceSettings {
        let catalog = self.voice_catalog();
        let mut copy = settings.clone();

        if let Some(ref voice) = settings.tts_voice {
            if let Some(canonical) = canonical_voice_name(voice, &catalog) {
                copy.tts_voice = Some(canonical);
                return copy;
            }
        }

        copy.tts_voice = preferred_female_voice(&catalog);
        copy
    }

    pub fn validate_voice_selection(&self, requested: Option<&str>) -> Result<Option<String>, String> {
        let catalog = self.voice_catalog();
        match requested {
            None => Ok(preferred_female_voice(&catalog)),
            Some(req) if req.trim().is_empty() => Ok(preferred_female_voice(&catalog)),
            Some(req) => {
                if let Some(canonical) = canonical_voice_name(req, &catalog) {
                    Ok(Some(canonical))
                } else {
                    Err(format!(
                        "The female local voice '{}' is not installed or is not selectable.",
                        req
                    ))
                }
            }
        }
    }
}

fn preferred_female_voice(catalog: &[VoiceOption]) -> Option<String> {
    let female: Vec<_> = catalog.iter().filter(|v| v.gender == "female").collect();
    if let Some(zira) = female.iter().find(|v| v.name.to_lowercase().contains("zira")) {
        return Some(zira.name.clone());
    }
    if let Some(en) = female.iter().find(|v| v.language.to_lowercase().starts_with("en")) {
        return Some(en.name.clone());
    }
    female.first().map(|v| v.name.clone())
}

fn canonical_voice_name(requested: &str, catalog: &[VoiceOption]) -> Option<String> {
    let norm = requested.trim().to_lowercase();
    catalog
        .iter()
        .find(|v| v.name.to_lowercase() == norm)
        .map(|v| v.name.clone())
}

fn discover_windows_voice_catalog() -> Vec<VoiceOption> {
    if !cfg!(target_os = "windows") {
        return vec![];
    }
    let powershell = match find_powershell() {
        Some(exe) => exe,
        None => return vec![],
    };

    let script = r#"
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::UTF8
Add-Type -AssemblyName System.Speech
$speaker = New-Object System.Speech.Synthesis.SpeechSynthesizer
try {
  $defaultVoice = $speaker.Voice.Name
  $voices = @(
    $speaker.GetInstalledVoices() |
      Where-Object { $_.Enabled } |
      ForEach-Object {
        [PSCustomObject]@{
          name = $_.VoiceInfo.Name
          gender = $_.VoiceInfo.Gender.ToString()
          language = $_.VoiceInfo.Culture.Name
          isSystemDefault = ($_.VoiceInfo.Name -eq $defaultVoice)
        }
      }
  )
  [PSCustomObject]@{
    systemDefault = $defaultVoice
    voices = $voices
  } | ConvertTo-Json -Depth 4 -Compress
} finally {
  $speaker.Dispose()
}
"#;

    let output = Command::new(&powershell)
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script])
        .output();

    let mut voices = Vec::new();
    if let Ok(out) = output {
        if out.status.success() {
            let stdout_str = String::from_utf8_lossy(&out.stdout);
            if let Ok(v) = serde_json::from_str::<Value>(&stdout_str) {
                let raw_voices = v.get("voices").cloned().unwrap_or(Value::Array(vec![]));
                let list = if raw_voices.is_array() {
                    raw_voices.as_array().unwrap().clone()
                } else if raw_voices.is_object() {
                    vec![raw_voices]
                } else {
                    vec![]
                };
                for rv in list {
                    let name = rv.get("name").and_then(|n| n.as_str()).unwrap_or("").trim();
                    let gender = rv.get("gender").and_then(|g| g.as_str()).unwrap_or("").to_lowercase();
                    let language = rv.get("language").and_then(|l| l.as_str()).unwrap_or("").trim();
                    let is_default = rv.get("isSystemDefault").and_then(|d| d.as_bool()).unwrap_or(false);

                    if !name.is_empty() && gender == "female" {
                        voices.push(VoiceOption {
                            name: name.to_string(),
                            gender: "female".to_string(),
                            language: if language.is_empty() { "unknown".to_string() } else { language.to_string() },
                            is_system_default: is_default,
                        });
                    }
                }
            }
        }
    }
    voices
}

fn run_windows_stt(language: &str, listen_seconds: u32) -> Result<String, String> {
    if !cfg!(target_os = "windows") {
        return Err("STT is only supported on Windows.".to_string());
    }
    let powershell = find_powershell().ok_or_else(|| "PowerShell is required for local voice.".to_string())?;

    let script = r#"
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::UTF8
Add-Type -AssemblyName System.Speech
$seconds = [Math]::Max(2, [Math]::Min(20, [int]$env:DEYANA_STT_SECONDS))
$cultureName = $env:DEYANA_STT_LANGUAGE
$recognizer = $null
try {
  $culture = [System.Globalization.CultureInfo]::GetCultureInfo($cultureName)
  $recognizer = New-Object System.Speech.Recognition.SpeechRecognitionEngine($culture)
} catch {
  $recognizer = New-Object System.Speech.Recognition.SpeechRecognitionEngine
}
try {
  $recognizer.LoadGrammar((New-Object System.Speech.Recognition.DictationGrammar))
  $recognizer.SetInputToDefaultAudioDevice()
  $result = $recognizer.Recognize([TimeSpan]::FromSeconds($seconds))
  if ($null -ne $result) { $result.Text }
} finally {
  if ($null -ne $recognizer) { $recognizer.Dispose() }
}
"#;

    let output = Command::new(&powershell)
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script])
        .env("DEYANA_STT_LANGUAGE", language)
        .env("DEYANA_STT_SECONDS", listen_seconds.to_string())
        .output()
        .map_err(|e| format!("Failed to run speech recognition: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if err.is_empty() { "Local speech recognition failed.".to_string() } else { err });
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_windows_tts(
    text: &str,
    voice: Option<&str>,
    rate: i32,
    volume: u32,
) -> Result<Child, String> {
    if !cfg!(target_os = "windows") {
        return Err("TTS is only supported on Windows.".to_string());
    }
    let powershell = find_powershell().ok_or_else(|| "PowerShell is required for local voice.".to_string())?;

    let script = r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Speech
$speaker = New-Object System.Speech.Synthesis.SpeechSynthesizer
try {
  $speaker.Rate = [int]$env:DEYANA_TTS_RATE
  $speaker.Volume = [int]$env:DEYANA_TTS_VOLUME
  if ($env:DEYANA_TTS_VOICE) {
    $speaker.SelectVoice($env:DEYANA_TTS_VOICE)
  }
  $speaker.Speak($env:DEYANA_TTS_TEXT)
} finally {
  $speaker.Dispose()
}
"#;

    Command::new(&powershell)
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script])
        .env("DEYANA_TTS_TEXT", text)
        .env("DEYANA_TTS_VOICE", voice.unwrap_or(""))
        .env("DEYANA_TTS_RATE", rate.to_string())
        .env("DEYANA_TTS_VOLUME", volume.to_string())
        .spawn()
        .map_err(|e| format!("Failed to spawn text-to-speech process: {}", e))
}

fn find_powershell() -> Option<String> {
    if cfg!(target_os = "windows") {
        Some("powershell.exe".to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use r2d2::Pool;
    use r2d2_sqlite::SqliteConnectionManager;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    fn create_test_pool() -> DbPool {
        let manager = SqliteConnectionManager::memory();
        Pool::builder().max_size(1).build(manager).unwrap()
    }

    #[test]
    fn test_interrupt_speech_when_no_process() {
        let temp_dir = std::env::temp_dir().join(format!("deyana_test_{}", uuid::Uuid::new_v4()));
        let service = LocalVoiceService::new(temp_dir.clone(), create_test_pool());

        let res = service.interrupt_speech();
        assert!(!res.interrupted);
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_interrupt_speech_kills_running_process() {
        let temp_dir = std::env::temp_dir().join(format!("deyana_test_{}", uuid::Uuid::new_v4()));
        let service = LocalVoiceService::new(temp_dir.clone(), create_test_pool());

        let child = if cfg!(target_os = "windows") {
            Command::new("powershell.exe")
                .args(["-NoProfile", "-Command", "Start-Sleep -Seconds 10"])
                .spawn()
                .unwrap()
        } else {
            Command::new("sleep").arg("10").spawn().unwrap()
        };

        {
            let mut lock = service.active_tts_process.lock().unwrap();
            *lock = Some(child);
        }

        let res = service.interrupt_speech();
        assert!(res.interrupted);

        let lock = service.active_tts_process.lock().unwrap();
        assert!(lock.is_none());

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_speak_loop_unlocks_during_wait_and_allows_interruption() {
        let temp_dir = std::env::temp_dir().join(format!("deyana_test_{}", uuid::Uuid::new_v4()));
        let service = Arc::new(LocalVoiceService::new(temp_dir.clone(), create_create_pool_dummy(temp_dir.as_path())));

        fn create_create_pool_dummy(_path: &std::path::Path) -> DbPool {
            create_test_pool()
        }

        // Manually place a long-running process into active_tts_process
        let child = if cfg!(target_os = "windows") {
            Command::new("powershell.exe")
                .args(["-NoProfile", "-Command", "Start-Sleep -Seconds 10"])
                .spawn()
                .unwrap()
        } else {
            Command::new("sleep").arg("10").spawn().unwrap()
        };

        {
            let mut lock = service.active_tts_process.lock().unwrap();
            *lock = Some(child);
        }

        // Spawn a thread to interrupt speech after 100ms
        let service_clone = service.clone();
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));
            service_clone.interrupt_speech()
        });

        // Simulating the speak loop waiting on active_tts_process
        let wait_result = loop {
            let status_opt = {
                let mut lock = service.active_tts_process.lock();
                if let Ok(ref mut guard) = lock {
                    if let Some(ref mut c) = **guard {
                        match c.try_wait() {
                            Ok(Some(status)) => Ok(Some(status)),
                            Ok(None) => Ok(None),
                            Err(e) => Err(format!("Text-to-speech execution error: {}", e)),
                        }
                    } else {
                        Err("Speech was cancelled.".to_string())
                    }
                } else {
                    Err("Speech synchronization error.".to_string())
                }
            };

            match status_opt {
                Ok(Some(status)) => {
                    if let Ok(mut guard) = service.active_tts_process.lock() {
                        *guard = None;
                    }
                    break Ok(status);
                }
                Ok(None) => {
                    thread::sleep(Duration::from_millis(20));
                }
                Err(err) => {
                    if let Ok(mut guard) = service.active_tts_process.lock() {
                        *guard = None;
                    }
                    break Err(err);
                }
            }
        };

        let interrupt_res = handle.join().unwrap();
        assert!(interrupt_res.interrupted);
        assert_eq!(wait_result.unwrap_err(), "Speech was cancelled.");

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
