use tauri::State;

use super::types::*;
use super::BrowserState;

#[tauri::command]
pub fn get_browser_status(state: State<'_, BrowserState>) -> Result<BrowserStatusResponse, String> {
    state.service.status()
}

#[tauri::command]
pub fn list_browser_sessions(state: State<'_, BrowserState>) -> Result<BrowserSessionListResponse, String> {
    state.service.list_sessions()
}

#[tauri::command]
pub fn disconnect_browser_session(
    state: State<'_, BrowserState>,
    page_session_id: String,
) -> Result<BrowserDisconnectResponse, String> {
    state.service.disconnect_session(&page_session_id)
}

#[tauri::command]
pub fn list_browser_permissions(state: State<'_, BrowserState>) -> Result<BrowserPermissionListResponse, String> {
    state.service.list_permissions()
}

#[tauri::command]
pub fn request_browser_permission(
    state: State<'_, BrowserState>,
    request: BrowserPermissionRequest,
) -> Result<BrowserPermissionResponse, String> {
    state.service.request_permission(request)
}

#[tauri::command]
pub fn revoke_browser_permission(
    state: State<'_, BrowserState>,
    origin: String,
) -> Result<BrowserPermissionResponse, String> {
    state.service.revoke_permission(&origin)
}

#[tauri::command]
pub fn update_browser_context(
    state: State<'_, BrowserState>,
    context: BrowserPageContext,
) -> Result<BrowserSession, String> {
    state.service.update_page_context(context)
}

#[tauri::command]
pub fn read_browser_context(
    state: State<'_, BrowserState>,
    request: BrowserContextReadRequest,
) -> Result<BrowserContextReadResponse, String> {
    state.service.read_context(request)
}

#[tauri::command]
pub fn summarize_browser_context(
    state: State<'_, BrowserState>,
    request: BrowserContextSummaryRequest,
) -> Result<BrowserContextSummaryResponse, String> {
    state.service.summarize_context(request)
}

#[tauri::command]
pub fn browser_search(
    state: State<'_, BrowserState>,
    request: BrowserSearchRequest,
) -> Result<BrowserSearchResponse, String> {
    state.service.search(request)
}

#[tauri::command]
pub fn open_browser_tab(
    state: State<'_, BrowserState>,
    request: BrowserOpenTabRequest,
) -> Result<BrowserOpenTabResponse, String> {
    state.service.open_tab(request)
}

#[tauri::command]
pub fn draft_browser_reply(
    state: State<'_, BrowserState>,
    request: BrowserDraftReplyRequest,
) -> Result<BrowserDraftReplyResponse, String> {
    state.service.draft_reply(request)
}

#[tauri::command]
pub fn fill_browser_field(
    state: State<'_, BrowserState>,
    request: BrowserFillFieldRequest,
) -> Result<BrowserFillFieldResponse, String> {
    state.service.fill_field(request)
}

#[tauri::command]
pub fn clear_browser_field(
    state: State<'_, BrowserState>,
    request: BrowserClearFieldRequest,
) -> Result<BrowserFillFieldResponse, String> {
    state.service.clear_field(request)
}

#[tauri::command]
pub fn list_browser_audit(
    state: State<'_, BrowserState>,
    limit: Option<usize>,
) -> Result<BrowserAuditListResponse, String> {
    state.service.list_audit(limit)
}

#[tauri::command]
pub fn list_browser_action_plans(
    state: State<'_, BrowserState>,
    limit: Option<usize>,
) -> Result<BrowserActionPlanListResponse, String> {
    state.service.list_action_plans(limit)
}

#[tauri::command]
pub fn create_browser_action_plan(
    state: State<'_, BrowserState>,
    request: BrowserActionPlanCreateRequest,
) -> Result<BrowserActionPlanResponse, String> {
    state.service.create_action_plan(request)
}

#[tauri::command]
pub fn confirm_browser_action_plan(
    state: State<'_, BrowserState>,
    request: BrowserActionConfirmRequest,
) -> Result<BrowserActionPlanResponse, String> {
    state.service.confirm_action_plan(request)
}

#[tauri::command]
pub fn execute_browser_action_plan(
    state: State<'_, BrowserState>,
    plan_id: String,
) -> Result<BrowserActionPlanResponse, String> {
    state.service.execute_action_plan(&plan_id)
}

#[tauri::command]
pub fn cancel_browser_action_plan(
    state: State<'_, BrowserState>,
    plan_id: String,
) -> Result<BrowserActionPlanResponse, String> {
    state.service.cancel_action_plan(&plan_id)
}

#[tauri::command]
pub fn browser_emergency_stop(
    state: State<'_, BrowserState>,
) -> Result<BrowserEmergencyStopResponse, String> {
    state.service.emergency_stop()
}

#[tauri::command]
pub fn get_whatsapp_busy_mode_policy(
    state: State<'_, BrowserState>,
) -> Result<WhatsAppBusyModePolicy, String> {
    state.service.get_whatsapp_busy_policy()
}

#[tauri::command]
pub fn patch_whatsapp_busy_mode_policy(
    state: State<'_, BrowserState>,
    request: WhatsAppBusyModePolicyPatch,
) -> Result<WhatsAppBusyModePolicyResponse, String> {
    state.service.patch_whatsapp_busy_policy(request)
}

#[tauri::command]
pub fn evaluate_whatsapp_busy_mode(
    state: State<'_, BrowserState>,
    request: WhatsAppBusyModeEvaluationRequest,
) -> Result<WhatsAppBusyModeEvaluationResponse, String> {
    state.service.evaluate_whatsapp_busy_mode(request)
}

#[tauri::command]
pub fn send_whatsapp_busy_reply(
    state: State<'_, BrowserState>,
    request: WhatsAppBusyModeSendRequest,
) -> Result<WhatsAppBusyModeSendResponse, String> {
    state.service.send_whatsapp_busy_reply(request)
}

#[tauri::command]
pub fn get_browser_personality(
    state: State<'_, BrowserState>,
) -> Result<BrowserPersonalitySettingsResponse, String> {
    state.service.get_personality()
}

#[tauri::command]
pub fn patch_browser_personality_profile(
    state: State<'_, BrowserState>,
    request: BrowserPersonalityProfilePatch,
) -> Result<BrowserPersonalityProfile, String> {
    state.service.patch_personality_profile(request)
}

#[tauri::command]
pub fn save_browser_contact_tone(
    state: State<'_, BrowserState>,
    request: BrowserContactTonePreferenceRequest,
) -> Result<BrowserContactTonePreference, String> {
    state.service.save_contact_tone(request)
}

#[tauri::command]
pub fn infer_browser_mood(
    state: State<'_, BrowserState>,
    request: BrowserMoodInferRequest,
) -> Result<BrowserMoodHint, String> {
    state.service.infer_mood(request)
}

#[tauri::command]
pub fn preview_browser_personality(
    state: State<'_, BrowserState>,
    request: BrowserPersonalityPreviewRequest,
) -> Result<BrowserPersonalityPreviewResponse, String> {
    state.service.preview_personality(request)
}

#[tauri::command]
pub fn route_browser_voice_command(
    state: State<'_, BrowserState>,
    request: BrowserVoiceCommandRequest,
) -> Result<BrowserVoiceCommandResponse, String> {
    state.service.route_voice_command(request)
}
