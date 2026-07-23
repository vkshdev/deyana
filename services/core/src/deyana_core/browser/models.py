from __future__ import annotations

from typing import Any, Literal

from pydantic import Field

from ..models import ApiModel, ToolResultItem


BROWSER_PROTOCOL_VERSION = 1
BrowserContextMode = Literal["selection", "main", "visible"]
BrowserConnectionState = Literal["disconnected", "connecting", "connected", "error"]
BrowserRequestStatus = Literal["completed", "permission_required", "unavailable", "failed"]
BrowserPermissionKind = Literal["temporary_active_tab", "optional_origin"]
BrowserAuditDecision = Literal["allowed", "blocked", "failed"]
BrowserFieldKind = Literal["textarea", "text_input", "contenteditable"]
BrowserAdapterHealthState = Literal["available", "degraded", "unsupported"]
BrowserDraftTone = Literal["reply", "regenerate", "shorten", "formalize"]
BrowserActionKind = Literal["open_tab", "fill_field", "click"]
BrowserActionPlanStatus = Literal[
    "pending_confirmation",
    "confirmed",
    "policy_authorized",
    "executing",
    "completed",
    "cancelled",
    "expired",
    "failed",
    "blocked",
]
WhatsAppBusyDecision = Literal["allowed", "blocked", "requires_confirmation"]
WhatsAppBusyCategory = Literal[
    "normal",
    "urgent",
    "otp",
    "password",
    "payment",
    "legal",
    "medical",
    "security",
    "unknown",
]
BrowserPersonalityPreset = Literal["concise", "supportive", "playful", "professional", "custom"]
BrowserVoiceIntent = Literal["read_page", "summarize_page", "draft_reply", "search_web", "open_url", "unknown"]


class BrowserLandmark(ApiModel):
    role: str
    name: str = ""
    text: str


class BrowserAvailableAction(ApiModel):
    action_id: str
    capability: Literal["page.read", "tab.open", "field.detect", "field.fill", "message.draft_reply", "message.send_reply"]
    label: str


class BrowserAdapterCapability(ApiModel):
    id: Literal["page.read", "tab.open", "field.detect", "field.fill", "message.draft_reply", "message.send_reply"]
    label: str
    available: bool
    detail: str


class BrowserAdapterHealth(ApiModel):
    adapter_id: str
    adapter_version: int
    state: BrowserAdapterHealthState
    detail: str
    capabilities: list[BrowserAdapterCapability] = Field(default_factory=list)


class BrowserWritableField(ApiModel):
    handle: str
    kind: BrowserFieldKind
    label: str
    placeholder: str | None = None
    value_preview: str = ""
    value_character_count: int = 0
    max_length: int | None = None
    required: bool = False
    disabled: bool = False
    captured_at: str


class BrowserPageContext(ApiModel):
    page_session_id: str
    origin: str
    url: str
    title: str
    adapter_id: str = "generic_page"
    adapter_version: int = 1
    mode: BrowserContextMode
    captured_at: str
    visible_text: str = ""
    selection_text: str | None = None
    main_text: str | None = None
    landmarks: list[BrowserLandmark] = Field(default_factory=list)
    available_actions: list[BrowserAvailableAction] = Field(default_factory=list)
    writable_fields: list[BrowserWritableField] = Field(default_factory=list)
    adapter_health: BrowserAdapterHealth = Field(
        default_factory=lambda: BrowserAdapterHealth(
            adapter_id="generic_page",
            adapter_version=1,
            state="degraded",
            detail="No adapter health was reported by the browser extension.",
            capabilities=[],
        )
    )
    character_count: int = 0
    truncated: bool = False


class BrowserSession(ApiModel):
    id: str
    origin: str
    url: str
    title: str
    adapter_id: str
    mode: BrowserContextMode
    character_count: int
    truncated: bool
    created_at: str
    updated_at: str
    expires_at: str


class BrowserSessionListResponse(ApiModel):
    items: list[BrowserSession]
    total: int


class BrowserPermission(ApiModel):
    origin: str
    kind: BrowserPermissionKind
    granted: bool
    granted_at: str | None = None
    detail: str


class BrowserPermissionListResponse(ApiModel):
    items: list[BrowserPermission]
    total: int


class BrowserPermissionRequest(ApiModel):
    origin: str | None = None
    kind: BrowserPermissionKind = "temporary_active_tab"


class BrowserPermissionResponse(ApiModel):
    status: BrowserRequestStatus
    permission: BrowserPermission | None = None
    instruction: str


class BrowserStatusResponse(ApiModel):
    state: BrowserConnectionState
    connected: bool
    browser_name: str | None = None
    browser_version: str | None = None
    extension_version: str | None = None
    extension_origin: str | None = None
    protocol_version: int = BROWSER_PROTOCOL_VERSION
    active_sessions: int = 0
    permissions: int = 0
    last_connected_at: str | None = None
    last_message_at: str | None = None
    last_error: str | None = None
    credential_path: str


class BrowserContextReadRequest(ApiModel):
    mode: BrowserContextMode = "main"
    user_approved: bool = False


class BrowserContextReadResponse(ApiModel):
    status: BrowserRequestStatus
    context: BrowserPageContext | None = None
    instruction: str | None = None


class BrowserContextSummaryRequest(ApiModel):
    mode: BrowserContextMode = "main"
    instruction: str = "Summarize the important information on this page."
    user_approved: bool = False


class BrowserContextSummaryResponse(ApiModel):
    status: BrowserRequestStatus
    summary: str
    model: str | None = None
    latency_ms: int = 0
    context: BrowserPageContext | None = None
    instruction: str | None = None


class BrowserSearchRequest(ApiModel):
    query: str
    limit: int = Field(default=5, ge=1, le=10)
    user_approved: bool = False


class BrowserSearchResponse(ApiModel):
    status: BrowserRequestStatus
    query: str
    items: list[ToolResultItem] = Field(default_factory=list)
    summary: str


class BrowserOpenTabRequest(ApiModel):
    url: str
    active: bool = True
    user_approved: bool = False


class BrowserOpenTabResponse(ApiModel):
    status: BrowserRequestStatus
    url: str
    tab_id: int | None = None
    instruction: str | None = None


class BrowserDraftReplyRequest(ApiModel):
    instruction: str = Field(min_length=1, max_length=4000)
    mode: BrowserContextMode = "visible"
    page_session_id: str | None = None
    field_handle: str | None = None
    tone: BrowserDraftTone = "reply"
    user_approved: bool = False


class BrowserDraftReplyResponse(ApiModel):
    status: BrowserRequestStatus
    draft: str = ""
    field: BrowserWritableField | None = None
    context: BrowserPageContext | None = None
    model: str | None = None
    latency_ms: int = 0
    instruction: str | None = None


class BrowserFillFieldRequest(ApiModel):
    page_session_id: str
    field_handle: str
    value: str = Field(max_length=8000)
    user_approved: bool = False


class BrowserFillFieldResponse(ApiModel):
    status: BrowserRequestStatus
    field_handle: str
    inserted: bool = False
    previous_value_preview: str | None = None
    instruction: str | None = None


class BrowserClearFieldRequest(ApiModel):
    page_session_id: str
    field_handle: str
    restore_original: bool = True
    user_approved: bool = False


class BrowserActionStep(ApiModel):
    id: str
    kind: BrowserActionKind
    label: str
    origin: str | None = None
    page_session_id: str | None = None
    field_handle: str | None = None
    url: str | None = None
    value: str | None = None
    action_id: str | None = None
    target_label: str | None = None
    verification: str


class BrowserActionPlan(ApiModel):
    id: str
    status: BrowserActionPlanStatus
    summary: str
    preview_markdown: str
    origin: str | None = None
    page_session_id: str | None = None
    steps: list[BrowserActionStep]
    confirmation_token: str | None = None
    expires_at: str
    created_at: str
    updated_at: str
    result_detail: str | None = None


class BrowserActionPlanCreateRequest(ApiModel):
    chain: Literal["open_url", "fill_field", "whatsapp_send"]
    url: str | None = None
    page_session_id: str | None = None
    field_handle: str | None = None
    value: str | None = None
    target_label: str | None = None
    user_approved: bool = False


class BrowserActionPlanResponse(ApiModel):
    status: BrowserRequestStatus
    plan: BrowserActionPlan | None = None
    instruction: str | None = None


class BrowserActionPlanListResponse(ApiModel):
    items: list[BrowserActionPlan]
    total: int


class BrowserActionConfirmRequest(ApiModel):
    plan_id: str
    confirmation_token: str


class BrowserEmergencyStopResponse(ApiModel):
    stopped: bool
    cancelled_plans: int
    instruction: str


class WhatsAppBusyModePolicy(ApiModel):
    enabled: bool = False
    allowlisted_contacts: list[str] = Field(default_factory=list)
    allow_groups: bool = False
    timezone: str = "local"
    window_start: str = "09:00"
    window_end: str = "18:00"
    cooldown_minutes: int = Field(default=60, ge=1, le=1440)
    daily_limit: int = Field(default=5, ge=1, le=50)
    template: str = (
        "I am Deyana, Vikash's assistant. Vikash is busy right now. "
        "Please tell me if this is necessary or urgent."
    )
    emergency_stopped: bool = False
    permission_origin: str = "https://web.whatsapp.com/*"
    permission_granted: bool = False
    updated_at: str


class WhatsAppBusyModePolicyPatch(ApiModel):
    enabled: bool | None = None
    allowlisted_contacts: list[str] | None = None
    allow_groups: bool | None = None
    timezone: str | None = None
    window_start: str | None = None
    window_end: str | None = None
    cooldown_minutes: int | None = Field(default=None, ge=1, le=1440)
    daily_limit: int | None = Field(default=None, ge=1, le=50)
    template: str | None = Field(default=None, max_length=500)
    reset_emergency_stop: bool | None = None


class WhatsAppBusyModePolicyResponse(ApiModel):
    status: BrowserRequestStatus
    policy: WhatsAppBusyModePolicy
    permission: BrowserPermissionResponse | None = None
    instruction: str | None = None


class WhatsAppBusyModeEvaluationRequest(ApiModel):
    page_session_id: str | None = None
    contact_label: str | None = None
    is_group: bool | None = None
    latest_message_text: str | None = None
    user_approved: bool = False


class WhatsAppBusyModeEvaluationResponse(ApiModel):
    status: BrowserRequestStatus
    allowed: bool
    decision: WhatsAppBusyDecision
    reason: str
    category: WhatsAppBusyCategory
    urgency_detected: bool = False
    owner_notification: bool = False
    target_label: str | None = None
    draft: str | None = None
    policy: WhatsAppBusyModePolicy


class WhatsAppBusyModeSendRequest(ApiModel):
    page_session_id: str
    field_handle: str | None = None
    latest_message_text: str | None = None
    user_approved: bool = False


class WhatsAppBusyModeSendResponse(ApiModel):
    status: BrowserRequestStatus
    evaluation: WhatsAppBusyModeEvaluationResponse
    plan: BrowserActionPlan | None = None
    instruction: str | None = None


class BrowserPersonalityProfile(ApiModel):
    preset: BrowserPersonalityPreset = "supportive"
    display_name: str = "Supportive"
    custom_instruction: str = ""
    writer_temperature: float = Field(default=0.42, ge=0.0, le=1.0)
    max_draft_characters: int = Field(default=900, ge=80, le=4000)
    automation_disclosure: str = "I am Deyana, Vikash's assistant."
    updated_at: str


class BrowserPersonalityProfilePatch(ApiModel):
    preset: BrowserPersonalityPreset | None = None
    display_name: str | None = Field(default=None, max_length=80)
    custom_instruction: str | None = Field(default=None, max_length=1000)
    writer_temperature: float | None = Field(default=None, ge=0.0, le=1.0)
    max_draft_characters: int | None = Field(default=None, ge=80, le=4000)
    automation_disclosure: str | None = Field(default=None, max_length=300)


class BrowserContactTonePreference(ApiModel):
    adapter_id: str
    contact_label: str
    tone_instruction: str
    approved: bool = True
    updated_at: str


class BrowserContactTonePreferenceRequest(ApiModel):
    adapter_id: str = Field(min_length=1, max_length=80)
    contact_label: str = Field(min_length=1, max_length=160)
    tone_instruction: str = Field(min_length=1, max_length=500)
    approved: bool = True


class BrowserMoodHint(ApiModel):
    label: str
    confidence: float = Field(ge=0.0, le=1.0)
    expires_at: str
    persisted: bool = False


class BrowserMoodInferRequest(ApiModel):
    text: str = Field(min_length=1, max_length=4000)
    ttl_seconds: int = Field(default=900, ge=60, le=3600)


class BrowserPersonalitySettingsResponse(ApiModel):
    profile: BrowserPersonalityProfile
    contact_tones: list[BrowserContactTonePreference] = Field(default_factory=list)
    mood_hint: BrowserMoodHint | None = None


class BrowserPersonalityPreviewRequest(ApiModel):
    sample_text: str = "Can you reply that I am busy but will respond soon?"
    contact_label: str | None = None
    adapter_id: str | None = None


class BrowserPersonalityPreviewResponse(ApiModel):
    preview: str
    profile: BrowserPersonalityProfile
    contact_tone: BrowserContactTonePreference | None = None
    mood_hint: BrowserMoodHint | None = None


class BrowserVoiceCommandRequest(ApiModel):
    transcript: str = Field(min_length=1, max_length=4000)
    mode: BrowserContextMode = "visible"
    page_session_id: str | None = None
    user_approved: bool = False


class BrowserVoiceCommandResponse(ApiModel):
    status: BrowserRequestStatus
    transcript_preview: str
    intent: BrowserVoiceIntent
    instruction: str
    summary: BrowserContextSummaryResponse | None = None
    draft: BrowserDraftReplyResponse | None = None
    search: BrowserSearchResponse | None = None
    action_plan: BrowserActionPlan | None = None


class BrowserClickActionRequest(ApiModel):
    page_session_id: str
    action_id: str
    expected_text: str | None = None
    target_label: str | None = None
    user_approved: bool = False


class BrowserClickActionResponse(ApiModel):
    status: BrowserRequestStatus
    action_id: str
    clicked: bool = False
    verified: bool = False
    instruction: str | None = None


class BrowserDisconnectResponse(ApiModel):
    disconnected: bool
    page_session_id: str


class BrowserAuditEvent(ApiModel):
    id: str
    event_type: str
    decision: BrowserAuditDecision
    operation: str
    origin: str | None = None
    page_session_id: str | None = None
    detail: str
    payload_sha256: str | None = None
    payload_character_count: int = 0
    created_at: str


class BrowserAuditListResponse(ApiModel):
    items: list[BrowserAuditEvent]
    total: int


class BrowserBridgeEnvelope(ApiModel):
    protocol_version: int = BROWSER_PROTOCOL_VERSION
    request_id: str
    type: str
    timestamp: str
    page_session_id: str | None = None
    payload: dict[str, Any] = Field(default_factory=dict)


class BrowserBridgeReadyPayload(ApiModel):
    browser_name: str
    browser_version: str | None = None
    extension_version: str
    permissions: list[BrowserPermission] = Field(default_factory=list)


class BrowserBridgeErrorPayload(ApiModel):
    code: str
    message: str
