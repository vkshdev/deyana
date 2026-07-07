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


class BrowserLandmark(ApiModel):
    role: str
    name: str = ""
    text: str


class BrowserAvailableAction(ApiModel):
    action_id: str
    capability: Literal["page.read", "tab.open"]
    label: str


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
