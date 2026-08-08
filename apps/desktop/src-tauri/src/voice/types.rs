use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceWakePayload {
    pub keyword: String,
    pub score: f32,
    pub timestamp: u64,
}

fn default_true() -> bool {
    true
}

fn default_sensitivity() -> f32 {
    0.6
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceSettings {
    pub enabled: bool,
    pub muted: bool,
    pub tts_enabled: bool,
    #[serde(default = "default_true")]
    pub wake_word_enabled: bool,
    #[serde(default = "default_sensitivity")]
    pub wake_word_sensitivity: f32,
    pub transcript_retention: String,
    pub stt_engine: String,
    pub tts_engine: String,
    pub language: String,
    pub listen_seconds: u32,
    pub tts_voice: Option<String>,
    pub tts_rate: i32,
    pub tts_volume: u32,
    pub updated_at: String,
}

impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            muted: false,
            tts_enabled: true,
            wake_word_enabled: true,
            wake_word_sensitivity: 0.6,
            transcript_retention: "none".to_string(),
            stt_engine: "windows_speech".to_string(),
            tts_engine: "windows_speech".to_string(),
            language: "en-US".to_string(),
            listen_seconds: 5,
            tts_voice: None,
            tts_rate: 0,
            tts_volume: 100,
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceSettingsPatch {
    pub enabled: Option<bool>,
    pub muted: Option<bool>,
    pub tts_enabled: Option<bool>,
    pub wake_word_enabled: Option<bool>,
    pub wake_word_sensitivity: Option<f32>,
    pub transcript_retention: Option<String>,
    pub language: Option<String>,
    pub listen_seconds: Option<u32>,
    pub tts_voice: Option<String>,
    pub tts_rate: Option<i32>,
    pub tts_volume: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceOption {
    pub name: String,
    pub gender: String,
    pub language: String,
    pub is_system_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceStatusResponse {
    pub enabled: bool,
    pub muted: bool,
    pub tts_enabled: bool,
    pub stt_status: String,
    pub tts_status: String,
    pub stt_engine: String,
    pub tts_engine: String,
    pub language: String,
    pub active_tts_voice: Option<String>,
    pub available_tts_voices: Vec<VoiceOption>,
    pub raw_audio_stored: bool,
    pub detail: String,
    pub checked_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceTranscriptRequest {
    pub listen_seconds: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceTranscriptResponse {
    pub transcript: String,
    pub engine: String,
    pub language: String,
    pub duration_seconds: u32,
    pub raw_audio_stored: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceTranscriptPayload {
    pub transcript: String,
    pub engine: String,
    pub language: String,
    pub duration_seconds: f64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceSpeakRequest {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceSpeakResponse {
    pub spoken: bool,
    pub engine: String,
    pub characters: usize,
    pub raw_audio_stored: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceInterruptResponse {
    pub interrupted: bool,
    pub engine: String,
    pub detail: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceSpeakingPayload {
    pub text: String,
}

