use std::collections::HashMap;
use serde::{Deserialize, Serialize};

pub const BROWSER_PROTOCOL_VERSION: i32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserLandmark {
    pub role: String,
    #[serde(default)]
    pub name: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserAvailableAction {
    pub action_id: String,
    pub capability: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserAdapterCapability {
    pub id: String,
    pub label: String,
    pub available: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserAdapterHealth {
    pub adapter_id: String,
    pub adapter_version: i32,
    pub state: String,
    pub detail: String,
    #[serde(default)]
    pub capabilities: Vec<BrowserAdapterCapability>,
}

impl Default for BrowserAdapterHealth {
    fn default() -> Self {
        Self {
            adapter_id: "generic_page".to_string(),
            adapter_version: 1,
            state: "degraded".to_string(),
            detail: "No adapter health was reported by the browser extension.".to_string(),
            capabilities: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserWritableField {
    pub handle: String,
    pub kind: String,
    pub label: String,
    pub placeholder: Option<String>,
    #[serde(default)]
    pub value_preview: String,
    #[serde(default)]
    pub value_character_count: usize,
    pub max_length: Option<usize>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub disabled: bool,
    pub captured_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserPageContext {
    pub page_session_id: String,
    pub origin: String,
    pub url: String,
    pub title: String,
    #[serde(default = "default_adapter_id")]
    pub adapter_id: String,
    #[serde(default = "default_adapter_version")]
    pub adapter_version: i32,
    pub mode: String,
    pub captured_at: String,
    #[serde(default)]
    pub visible_text: String,
    pub selection_text: Option<String>,
    pub main_text: Option<String>,
    #[serde(default)]
    pub landmarks: Vec<BrowserLandmark>,
    #[serde(default)]
    pub available_actions: Vec<BrowserAvailableAction>,
    #[serde(default)]
    pub writable_fields: Vec<BrowserWritableField>,
    #[serde(default)]
    pub adapter_health: BrowserAdapterHealth,
    #[serde(default)]
    pub character_count: usize,
    #[serde(default)]
    pub truncated: bool,
}

fn default_adapter_id() -> String {
    "generic_page".to_string()
}

fn default_adapter_version() -> i32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserSession {
    pub id: String,
    pub origin: String,
    pub url: String,
    pub title: String,
    pub adapter_id: String,
    pub mode: String,
    pub character_count: usize,
    pub truncated: bool,
    pub created_at: String,
    pub updated_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserSessionListResponse {
    pub items: Vec<BrowserSession>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserPermission {
    pub origin: String,
    pub kind: String,
    pub granted: bool,
    pub granted_at: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserPermissionListResponse {
    pub items: Vec<BrowserPermission>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserPermissionRequest {
    pub origin: Option<String>,
    #[serde(default = "default_permission_kind")]
    pub kind: String,
}

fn default_permission_kind() -> String {
    "temporary_active_tab".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserPermissionResponse {
    pub status: String,
    pub permission: Option<BrowserPermission>,
    pub instruction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserStatusResponse {
    pub state: String,
    pub connected: bool,
    pub browser_name: Option<String>,
    pub browser_version: Option<String>,
    pub extension_version: Option<String>,
    pub extension_origin: Option<String>,
    #[serde(default = "default_protocol_version")]
    pub protocol_version: i32,
    pub active_sessions: usize,
    pub permissions: usize,
    pub last_connected_at: Option<String>,
    pub last_message_at: Option<String>,
    pub last_error: Option<String>,
    pub credential_path: String,
}

fn default_protocol_version() -> i32 {
    BROWSER_PROTOCOL_VERSION
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserContextReadRequest {
    pub page_session_id: Option<String>,
    pub origin: Option<String>,
    #[serde(default = "default_read_mode")]
    pub mode: String,
    #[serde(default)]
    pub user_approved: bool,
}

fn default_read_mode() -> String {
    "main".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserContextReadResponse {
    pub status: String,
    pub context: Option<BrowserPageContext>,
    pub instruction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserContextSummaryRequest {
    #[serde(default = "default_read_mode")]
    pub mode: String,
    #[serde(default = "default_summary_instruction")]
    pub instruction: String,
    #[serde(default)]
    pub user_approved: bool,
}

fn default_summary_instruction() -> String {
    "Summarize the important information on this page.".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserContextSummaryResponse {
    pub status: String,
    pub summary: String,
    pub model: Option<String>,
    pub latency_ms: u64,
    pub context: Option<BrowserPageContext>,
    pub instruction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserSearchRequest {
    pub query: String,
    pub limit: Option<usize>,
    #[serde(default)]
    pub user_approved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserSearchItem {
    pub title: String,
    pub summary: String,
    pub url: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserSearchResponse {
    pub status: String,
    pub query: String,
    #[serde(default)]
    pub items: Vec<BrowserSearchItem>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserOpenTabRequest {
    pub url: String,
    #[serde(default = "default_true")]
    pub active: bool,
    #[serde(default)]
    pub user_approved: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserOpenTabResponse {
    pub status: String,
    pub url: String,
    pub tab_id: Option<i64>,
    pub instruction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserDraftReplyRequest {
    pub instruction: String,
    #[serde(default = "default_draft_mode")]
    pub mode: String,
    pub page_session_id: Option<String>,
    pub field_handle: Option<String>,
    #[serde(default = "default_draft_tone")]
    pub tone: String,
    #[serde(default)]
    pub user_approved: bool,
}

fn default_draft_mode() -> String {
    "visible".to_string()
}

fn default_draft_tone() -> String {
    "reply".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserDraftReplyResponse {
    pub status: String,
    #[serde(default)]
    pub draft: String,
    pub field: Option<BrowserWritableField>,
    pub context: Option<BrowserPageContext>,
    pub model: Option<String>,
    #[serde(default)]
    pub latency_ms: u64,
    pub instruction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserFillFieldRequest {
    pub page_session_id: String,
    pub field_handle: String,
    pub value: String,
    #[serde(default)]
    pub user_approved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserFillFieldResponse {
    pub status: String,
    pub field_handle: String,
    #[serde(default)]
    pub inserted: bool,
    pub previous_value_preview: Option<String>,
    pub instruction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserClearFieldRequest {
    pub page_session_id: String,
    pub field_handle: String,
    #[serde(default = "default_true")]
    pub restore_original: bool,
    #[serde(default)]
    pub user_approved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserActionStep {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub origin: Option<String>,
    pub page_session_id: Option<String>,
    pub field_handle: Option<String>,
    pub url: Option<String>,
    pub value: Option<String>,
    pub action_id: Option<String>,
    pub target_label: Option<String>,
    pub verification: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserActionPlan {
    pub id: String,
    pub status: String,
    pub summary: String,
    pub preview_markdown: String,
    pub origin: Option<String>,
    pub page_session_id: Option<String>,
    pub steps: Vec<BrowserActionStep>,
    pub confirmation_token: Option<String>,
    pub expires_at: String,
    pub created_at: String,
    pub updated_at: String,
    pub result_detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserActionPlanCreateRequest {
    pub chain: String,
    pub url: Option<String>,
    pub page_session_id: Option<String>,
    pub field_handle: Option<String>,
    pub value: Option<String>,
    pub target_label: Option<String>,
    #[serde(default)]
    pub user_approved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserActionPlanResponse {
    pub status: String,
    pub plan: Option<BrowserActionPlan>,
    pub instruction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserActionPlanListResponse {
    pub items: Vec<BrowserActionPlan>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserActionConfirmRequest {
    pub plan_id: String,
    pub confirmation_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserEmergencyStopResponse {
    pub stopped: bool,
    pub cancelled_plans: usize,
    pub instruction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhatsAppBusyModePolicy {
    #[serde(default)]
    pub enabled: bool,
    pub enabled_platforms: Option<Vec<String>>,
    #[serde(default)]
    pub allowlisted_contacts: Vec<String>,
    #[serde(default)]
    pub allow_groups: bool,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    #[serde(default = "default_window_start")]
    pub window_start: String,
    #[serde(default = "default_window_end")]
    pub window_end: String,
    #[serde(default = "default_cooldown_minutes")]
    pub cooldown_minutes: i64,
    #[serde(default = "default_daily_limit")]
    pub daily_limit: i64,
    #[serde(default = "default_whatsapp_template")]
    pub template: String,
    pub platform_templates: Option<HashMap<String, String>>,
    #[serde(default)]
    pub emergency_stopped: bool,
    #[serde(default = "default_permission_origin")]
    pub permission_origin: String,
    #[serde(default)]
    pub permission_granted: bool,
    pub updated_at: String,
}

fn default_timezone() -> String {
    "local".to_string()
}
fn default_window_start() -> String {
    "09:00".to_string()
}
fn default_window_end() -> String {
    "18:00".to_string()
}
fn default_cooldown_minutes() -> i64 {
    60
}
fn default_daily_limit() -> i64 {
    5
}
fn default_whatsapp_template() -> String {
    "I am Deyana, Vikash's assistant. Vikash is busy right now. Please tell me if this is necessary or urgent.".to_string()
}
fn default_permission_origin() -> String {
    "https://web.whatsapp.com/*".to_string()
}

impl Default for WhatsAppBusyModePolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            enabled_platforms: Some(vec![
                "whatsapp".to_string(),
                "messenger".to_string(),
                "instagram".to_string(),
                "discord".to_string(),
                "gmail".to_string(),
                "telegram".to_string(),
                "linkedin".to_string(),
                "slack".to_string(),
            ]),
            allowlisted_contacts: vec![],
            allow_groups: false,
            timezone: default_timezone(),
            window_start: default_window_start(),
            window_end: default_window_end(),
            cooldown_minutes: default_cooldown_minutes(),
            daily_limit: default_daily_limit(),
            template: default_whatsapp_template(),
            platform_templates: None,
            emergency_stopped: false,
            permission_origin: default_permission_origin(),
            permission_granted: false,
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhatsAppBusyModePolicyPatch {
    pub enabled: Option<bool>,
    pub allowlisted_contacts: Option<Vec<String>>,
    pub allow_groups: Option<bool>,
    pub timezone: Option<String>,
    pub window_start: Option<String>,
    pub window_end: Option<String>,
    pub cooldown_minutes: Option<i64>,
    pub daily_limit: Option<i64>,
    pub template: Option<String>,
    pub reset_emergency_stop: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhatsAppBusyModePolicyResponse {
    pub status: String,
    pub policy: WhatsAppBusyModePolicy,
    pub permission: Option<BrowserPermissionResponse>,
    pub instruction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhatsAppBusyModeEvaluationRequest {
    pub page_session_id: Option<String>,
    pub contact_label: Option<String>,
    pub is_group: Option<bool>,
    pub latest_message_text: Option<String>,
    #[serde(default)]
    pub user_approved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhatsAppBusyModeEvaluationResponse {
    pub status: String,
    pub allowed: bool,
    pub decision: String,
    pub reason: String,
    pub category: String,
    #[serde(default)]
    pub urgency_detected: bool,
    #[serde(default)]
    pub owner_notification: bool,
    pub target_label: Option<String>,
    pub draft: Option<String>,
    pub policy: WhatsAppBusyModePolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhatsAppBusyModeSendRequest {
    pub page_session_id: String,
    pub field_handle: Option<String>,
    pub latest_message_text: Option<String>,
    #[serde(default)]
    pub user_approved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhatsAppBusyModeSendResponse {
    pub status: String,
    pub evaluation: WhatsAppBusyModeEvaluationResponse,
    pub plan: Option<BrowserActionPlan>,
    pub instruction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserPersonalityProfile {
    #[serde(default = "default_preset")]
    pub preset: String,
    #[serde(default = "default_display_name")]
    pub display_name: String,
    #[serde(default)]
    pub custom_instruction: String,
    #[serde(default = "default_temperature")]
    pub writer_temperature: f64,
    #[serde(default = "default_max_draft_chars")]
    pub max_draft_characters: usize,
    #[serde(default = "default_disclosure")]
    pub automation_disclosure: String,
    pub updated_at: String,
}

fn default_preset() -> String {
    "supportive".to_string()
}
fn default_display_name() -> String {
    "Supportive".to_string()
}
fn default_temperature() -> f64 {
    0.42
}
fn default_max_draft_chars() -> usize {
    900
}
fn default_disclosure() -> String {
    "I am Deyana, Vikash's assistant.".to_string()
}

impl Default for BrowserPersonalityProfile {
    fn default() -> Self {
        Self {
            preset: default_preset(),
            display_name: default_display_name(),
            custom_instruction: "".to_string(),
            writer_temperature: default_temperature(),
            max_draft_characters: default_max_draft_chars(),
            automation_disclosure: default_disclosure(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserPersonalityProfilePatch {
    pub preset: Option<String>,
    pub display_name: Option<String>,
    pub custom_instruction: Option<String>,
    pub writer_temperature: Option<f64>,
    pub max_draft_characters: Option<usize>,
    pub automation_disclosure: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserContactTonePreference {
    pub adapter_id: String,
    pub contact_label: String,
    pub tone_instruction: String,
    #[serde(default = "default_true")]
    pub approved: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserContactTonePreferenceRequest {
    pub adapter_id: String,
    pub contact_label: String,
    pub tone_instruction: String,
    #[serde(default = "default_true")]
    pub approved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserMoodHint {
    pub label: String,
    pub confidence: f64,
    pub expires_at: String,
    #[serde(default)]
    pub persisted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserMoodInferRequest {
    pub text: String,
    pub ttl_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserPersonalitySettingsResponse {
    pub profile: BrowserPersonalityProfile,
    #[serde(default)]
    pub contact_tones: Vec<BrowserContactTonePreference>,
    pub mood_hint: Option<BrowserMoodHint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserPersonalityPreviewRequest {
    pub sample_text: Option<String>,
    pub contact_label: Option<String>,
    pub adapter_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserPersonalityPreviewResponse {
    pub preview: String,
    pub profile: BrowserPersonalityProfile,
    pub contact_tone: Option<BrowserContactTonePreference>,
    pub mood_hint: Option<BrowserMoodHint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserVoiceCommandRequest {
    pub transcript: String,
    #[serde(default = "default_draft_mode")]
    pub mode: String,
    pub page_session_id: Option<String>,
    #[serde(default)]
    pub user_approved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserVoiceCommandResponse {
    pub status: String,
    pub transcript_preview: String,
    pub intent: String,
    pub instruction: String,
    pub summary: Option<BrowserContextSummaryResponse>,
    pub draft: Option<BrowserDraftReplyResponse>,
    pub search: Option<BrowserSearchResponse>,
    pub action_plan: Option<BrowserActionPlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserClickActionRequest {
    pub page_session_id: String,
    pub action_id: String,
    pub expected_text: Option<String>,
    pub target_label: Option<String>,
    #[serde(default)]
    pub user_approved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserClickActionResponse {
    pub status: String,
    pub action_id: String,
    #[serde(default)]
    pub clicked: bool,
    #[serde(default)]
    pub verified: bool,
    pub instruction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserDisconnectResponse {
    pub disconnected: bool,
    pub page_session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserAuditEvent {
    pub id: String,
    pub event_type: String,
    pub decision: String,
    pub operation: String,
    pub origin: Option<String>,
    pub page_session_id: Option<String>,
    pub detail: String,
    pub payload_sha256: Option<String>,
    #[serde(default)]
    pub payload_character_count: usize,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserAuditListResponse {
    pub items: Vec<BrowserAuditEvent>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserBridgeEnvelope {
    #[serde(default = "default_protocol_version")]
    pub protocol_version: i32,
    pub request_id: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub timestamp: String,
    pub page_session_id: Option<String>,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserBridgeReadyPayload {
    pub browser_name: String,
    pub browser_version: Option<String>,
    pub extension_version: String,
    #[serde(default)]
    pub permissions: Vec<BrowserPermission>,
}
