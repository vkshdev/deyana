use std::sync::Mutex;
use tts::Tts;

pub struct TtsEngine {
    tts: Mutex<Option<Tts>>,
}

impl TtsEngine {
    pub fn new() -> Self {
        let instance = Tts::default().ok();
        Self {
            tts: Mutex::new(instance),
        }
    }

    pub fn speak(
        &self,
        text: &str,
        voice: Option<&str>,
        _rate: i32,
        volume: u32,
    ) -> Result<(), String> {
        let mut guard = self.tts.lock().map_err(|e| e.to_string())?;
        if let Some(ref mut tts) = *guard {
            if let Some(voice_name) = voice {
                if let Ok(voices) = tts.voices() {
                    if let Some(v) = voices.into_iter().find(|v| v.name() == voice_name) {
                        let _ = tts.set_voice(&v);
                    }
                }
            }
            let _ = tts.set_volume((volume as f32 / 100.0).clamp(0.0, 1.0));
            tts.speak(text, true)
                .map_err(|e| format!("Native TTS synthesis failed: {}", e))?;
            Ok(())
        } else {
            Err("Native TTS engine is not initialized.".to_string())
        }
    }

    pub fn stop(&self) {
        if let Ok(mut guard) = self.tts.lock() {
            if let Some(ref mut tts) = *guard {
                let _ = tts.stop();
            }
        }
    }

    pub fn is_speaking(&self) -> bool {
        if let Ok(mut guard) = self.tts.lock() {
            if let Some(ref mut tts) = *guard {
                return tts.is_speaking().unwrap_or(false);
            }
        }
        false
    }
}

impl Default for TtsEngine {
    fn default() -> Self {
        Self::new()
    }
}
