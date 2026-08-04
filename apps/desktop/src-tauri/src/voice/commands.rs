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
pub fn transcribe_voice(
    state: State<'_, VoiceState>,
    request: Option<VoiceTranscriptRequest>,
) -> Result<VoiceTranscriptResponse, String> {
    state.service.transcribe(request)
}

#[tauri::command]
pub fn speak_voice(
    state: State<'_, VoiceState>,
    request: VoiceSpeakRequest,
) -> Result<VoiceSpeakResponse, String> {
    state.service.speak(request)
}

#[tauri::command]
pub fn interrupt_voice(state: State<'_, VoiceState>) -> Result<VoiceInterruptResponse, String> {
    Ok(state.service.interrupt_speech())
}
