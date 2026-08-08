use tauri::State;

use super::types::*;
use super::VoiceState;

#[tauri::command]
pub fn get_voice_settings(state: State<'_, VoiceState>) -> Result<VoiceSettings, String> {
    Ok(state.service.read_settings())
}

#[tauri::command]
pub fn patch_voice_settings(
    state: State<'_, VoiceState>,
    patch: VoiceSettingsPatch,
) -> Result<VoiceSettings, String> {
    state.service.patch_settings(patch)
}

#[tauri::command]
pub fn get_voice_status(state: State<'_, VoiceState>) -> Result<VoiceStatusResponse, String> {
    Ok(state.service.status())
}

#[tauri::command]
pub fn toggle_voice_mode(
    state: State<'_, VoiceState>,
    enabled: Option<bool>,
) -> Result<VoiceStatusResponse, String> {
    state.service.toggle_voice_mode(enabled)
}

#[tauri::command]
pub fn start_wake_listener(
    app: tauri::AppHandle,
    state: State<'_, VoiceState>,
) -> Result<(), String> {
    state.service.start_wake_listener(app)
}

#[tauri::command]
pub fn stop_wake_listener(state: State<'_, VoiceState>) -> Result<(), String> {
    state.service.stop_wake_listener()
}

#[tauri::command]
pub fn transcribe_voice(
    state: State<'_, VoiceState>,
    request: Option<VoiceTranscriptRequest>,
    listen_seconds: Option<u32>,
) -> Result<VoiceTranscriptResponse, String> {
    let req = match (request, listen_seconds) {
        (Some(r), _) => Some(r),
        (None, Some(sec)) => Some(VoiceTranscriptRequest { listen_seconds: Some(sec) }),
        (None, None) => None,
    };
    state.service.transcribe(req)
}

#[tauri::command]
pub fn transcribe_audio(
    state: State<'_, VoiceState>,
    request: Option<VoiceTranscriptRequest>,
    listen_seconds: Option<u32>,
) -> Result<VoiceTranscriptResponse, String> {
    transcribe_voice(state, request, listen_seconds)
}

#[tauri::command]
pub fn speak_voice(
    state: State<'_, VoiceState>,
    request: Option<VoiceSpeakRequest>,
    text: Option<String>,
) -> Result<VoiceSpeakResponse, String> {
    let speak_req = if let Some(req) = request {
        req
    } else if let Some(t) = text {
        VoiceSpeakRequest { text: t }
    } else {
        return Err("Speech text is required.".to_string());
    };
    state.service.speak(speak_req)
}

#[tauri::command]
pub fn speak_text(
    state: State<'_, VoiceState>,
    request: Option<VoiceSpeakRequest>,
    text: Option<String>,
) -> Result<VoiceSpeakResponse, String> {
    speak_voice(state, request, text)
}

#[tauri::command]
pub fn interrupt_voice(state: State<'_, VoiceState>) -> Result<VoiceInterruptResponse, String> {
    Ok(state.service.interrupt_speech())
}
