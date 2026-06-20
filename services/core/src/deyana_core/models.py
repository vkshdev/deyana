from __future__ import annotations

from typing import Any, Literal

from pydantic import BaseModel, ConfigDict, Field


def to_camel(value: str) -> str:
    parts = value.split("_")
    return parts[0] + "".join(part.capitalize() for part in parts[1:])


class ApiModel(BaseModel):
    model_config = ConfigDict(alias_generator=to_camel, populate_by_name=True)


class HealthResponse(BaseModel):
    status: Literal["ok"] = "ok"
    service: Literal["deyana-core"] = "deyana-core"
    version: str
    lifecycle: Literal["running", "stopping"]
    uptime_seconds: float = Field(serialization_alias="uptimeSeconds")
    timestamp: str

    model_config = ConfigDict(populate_by_name=True)


class DependencyStatus(BaseModel):
    name: str
    status: Literal["available", "missing", "not_configured", "deferred"]
    detail: str


class StatusResponse(BaseModel):
    service: Literal["deyana-core"] = "deyana-core"
    version: str
    lifecycle: Literal["running", "stopping"]
    boot_id: str = Field(serialization_alias="bootId")
    pid: int
    uptime_seconds: float = Field(serialization_alias="uptimeSeconds")
    host: str
    port: int
    dependencies: list[DependencyStatus]
    feature_flags: dict[str, bool] = Field(serialization_alias="featureFlags")
    timestamp: str

    model_config = ConfigDict(populate_by_name=True)


class CoreEvent(BaseModel):
    id: str
    type: str
    timestamp: str
    payload: dict[str, Any]


PrivacyMode = Literal["local_only"]
ModelProfile = Literal["low_spec", "balanced", "power"]
SyncMode = Literal["manual", "low_frequency"]
OnboardingStep = Literal["welcome", "privacy", "local_ai", "vault", "complete"]
VaultStatus = Literal["not_selected", "ready", "missing", "error"]
MemoryType = Literal[
    "chat",
    "note",
    "connector_summary",
    "file_summary",
    "git_summary",
    "daily_summary",
    "project_summary",
    "decision",
    "action_item",
]


class AppSettings(ApiModel):
    privacy_mode: PrivacyMode = "local_only"
    model_profile: ModelProfile = "low_spec"
    selected_chat_model: str = "qwen3:1.7b"
    selected_embedding_model: str = "all-minilm:latest"
    sync_mode: SyncMode = "manual"
    vault_path: str | None = None
    onboarding_completed: bool = False
    updated_at: str


class SettingsPatch(ApiModel):
    privacy_mode: PrivacyMode | None = None
    model_profile: ModelProfile | None = None
    selected_chat_model: str | None = None
    selected_embedding_model: str | None = None
    sync_mode: SyncMode | None = None


class OnboardingState(ApiModel):
    completed: bool = False
    completed_at: str | None = None
    current_step: OnboardingStep = "welcome"
    selected_vault_path: str | None = None
    selected_privacy_mode: PrivacyMode = "local_only"
    selected_model_profile: ModelProfile = "low_spec"
    vault_status: VaultStatus = "not_selected"
    vault_error: str | None = None
    vault_folders: list[str] = []


class VaultSelectRequest(ApiModel):
    path: str


class VaultSelectResponse(ApiModel):
    state: OnboardingState
    settings: AppSettings
    vault_path: str
    created_folders: list[str]


class OnboardingCompleteRequest(ApiModel):
    privacy_mode: PrivacyMode = "local_only"
    model_profile: ModelProfile = "low_spec"
    vault_path: str | None = None


class OnboardingCompleteResponse(ApiModel):
    state: OnboardingState
    settings: AppSettings


class MemoryEntity(ApiModel):
    id: str
    memory_id: str
    memory_title: str | None = None
    source_type: str | None = None
    source_id: str | None = None
    source_uri: str | None = None
    name: str
    entity_type: str
    source_text: str
    created_at: str


MemoryInsightType = Literal["action_item", "decision"]


class MemoryInsight(ApiModel):
    id: str
    memory_id: str
    memory_title: str | None = None
    source_type: str | None = None
    source_id: str | None = None
    source_uri: str | None = None
    type: MemoryInsightType
    title: str
    detail: str
    status: str = "open"
    due_at: str | None = None
    created_at: str


class MemoryItem(ApiModel):
    id: str
    type: MemoryType
    title: str
    summary: str
    content_markdown: str
    markdown_path: str | None = None
    source_type: str
    source_id: str | None = None
    source_uri: str | None = None
    importance: int = 3
    tags: list[str] = []
    entities: list[MemoryEntity] = []
    action_items: list[MemoryInsight] = []
    decisions: list[MemoryInsight] = []
    created_at: str
    updated_at: str
    deleted_at: str | None = None


class MemoryCreateRequest(ApiModel):
    type: MemoryType = "note"
    title: str
    summary: str = ""
    content_markdown: str | None = None
    source_type: str = "manual"
    source_id: str | None = None
    source_uri: str | None = None
    importance: int = 3
    tags: list[str] = []


class MemoryUpdateRequest(ApiModel):
    title: str | None = None
    summary: str | None = None
    content_markdown: str | None = None
    importance: int | None = None
    tags: list[str] | None = None


class MemoryListResponse(ApiModel):
    items: list[MemoryItem]
    total: int
    query: str | None = None


class MemoryEntityListResponse(ApiModel):
    items: list[MemoryEntity]
    total: int
    query: str | None = None
    source_type: str | None = None
    source_id: str | None = None
    date: str | None = None


class MemoryInsightListResponse(ApiModel):
    items: list[MemoryInsight]
    total: int
    query: str | None = None
    type: MemoryInsightType | None = None
    status: str | None = None
    source_type: str | None = None
    source_id: str | None = None
    date: str | None = None


class DailySummaryRequest(ApiModel):
    date: str | None = None


class ProjectSummaryRequest(ApiModel):
    project: str


class MemoryDeleteResponse(ApiModel):
    deleted: bool
    id: str


class MemoryReindexResponse(ApiModel):
    reindexed: int
    missing_markdown: int


class MemoryExportResponse(ApiModel):
    exported_at: str
    items: list[MemoryItem]


ModelProviderStatus = Literal["available", "missing", "offline"]
LocalModelRole = Literal["chat", "embedding", "unknown"]
ModelTask = Literal[
    "chat",
    "summarization",
    "compression",
    "planning",
    "classification",
    "embedding",
    "coding",
]
ChatRole = Literal["user", "assistant"]
PrivacyDecision = Literal["allow", "block"]
PrivacyDestinationCategory = Literal[
    "local",
    "public_web",
    "oauth_connector",
    "cloud_ai",
    "hosted_embedding",
    "hosted_reranker",
    "cloud_stt",
    "cloud_tts",
    "unknown_external",
]
PrivacyDataCategory = Literal[
    "public_query",
    "public_content",
    "oauth_token",
    "connector_metadata",
    "private_memory",
    "memory_summary",
    "embedding_text",
    "audio",
    "transcript",
    "source_code",
    "local_file",
    "chat_history",
    "unknown",
]
PrivacyRequestPurpose = Literal[
    "public_web_fetch",
    "oauth_api_fetch",
    "connector_api_fetch",
    "cloud_ai",
    "embedding",
    "reranking",
    "speech_to_text",
    "text_to_speech",
    "unknown",
]


class LocalModelInfo(ApiModel):
    name: str
    role: LocalModelRole
    installed: bool
    recommended: bool = False
    profile: ModelProfile | None = None
    size_bytes: int | None = None
    detail: str


class LocalModelStatusResponse(ApiModel):
    provider: Literal["ollama"] = "ollama"
    status: ModelProviderStatus
    endpoint: str
    selected_chat_model: str
    selected_embedding_model: str
    recommended_chat_model: str
    recommended_embedding_model: str
    chat_model_available: bool
    embedding_model_available: bool
    available_models: list[LocalModelInfo]
    setup_models: list[LocalModelInfo]
    max_parallel_model_jobs: int
    think: bool
    message: str
    checked_at: str


class ModelSelectionRequest(ApiModel):
    chat_model: str | None = None
    embedding_model: str | None = None
    profile: ModelProfile | None = None


class ModelSelectionResponse(ApiModel):
    settings: AppSettings
    status: LocalModelStatusResponse


class ModelTestRequest(ApiModel):
    prompt: str = "Reply with exactly: DEYANA_READY"


class ModelTestResponse(ApiModel):
    ok: bool
    model: str
    response: str
    latency_ms: int
    detail: str


class ChatMessageRequest(ApiModel):
    content: str
    use_memory: bool = True


class MemorySourceReference(ApiModel):
    id: str
    title: str
    label: str
    markdown_path: str | None = None
    source_type: str
    source_uri: str | None = None
    snippet: str
    score: float
    updated_at: str


class ChatMessageItem(ApiModel):
    id: str
    role: ChatRole
    content: str
    model: str | None = None
    source_references: list[MemorySourceReference] = []
    created_at: str


class ChatRetrievalSummary(ApiModel):
    query: str
    retrieved: int
    compressed_characters: int
    context_tokens_estimate: int


class ChatMessageResponse(ApiModel):
    user_message: ChatMessageItem
    assistant_message: ChatMessageItem
    model: str
    latency_ms: int
    sources: list[MemorySourceReference] = []
    retrieval: ChatRetrievalSummary


class ChatHistoryResponse(ApiModel):
    messages: list[ChatMessageItem]


class ChatHistoryDeleteResponse(ApiModel):
    deleted: int


class PrivacyCheckRequest(ApiModel):
    url: str
    method: str = "GET"
    purpose: PrivacyRequestPurpose = "unknown"
    data_category: PrivacyDataCategory | None = None
    payload_preview: str | None = None
    user_approved: bool = False
    connector_id: str | None = None
    external_write: bool = False


class PrivacyAuditEvent(ApiModel):
    id: str
    event_type: str
    decision: PrivacyDecision
    reason: str
    destination: str
    destination_category: PrivacyDestinationCategory
    data_category: PrivacyDataCategory
    purpose: PrivacyRequestPurpose
    method: str
    user_approved: bool
    connector_id: str | None = None
    safe_alternative: str
    payload_sha256: str | None = None
    payload_character_count: int = 0
    created_at: str


class PrivacyCheckResponse(ApiModel):
    allowed: bool
    decision: PrivacyDecision
    reason: str
    destination: str
    destination_category: PrivacyDestinationCategory
    data_category: PrivacyDataCategory
    purpose: PrivacyRequestPurpose
    safe_alternative: str
    audit_event: PrivacyAuditEvent


class PrivacyAuditListResponse(ApiModel):
    events: list[PrivacyAuditEvent]
    total: int


class PrivacyStatusResponse(ApiModel):
    mode: PrivacyMode
    enforced: bool
    audit_events: int
    blocked_events: int
    allowed_events: int
    last_blocked: PrivacyAuditEvent | None = None
    blocked_categories: list[PrivacyDestinationCategory]


class PrivacyAuditDeleteResponse(ApiModel):
    deleted: int


ConnectorStatus = Literal["not_connected", "connected", "syncing", "paused", "error"]
ConnectorSyncRunStatus = Literal["queued", "running", "completed", "failed", "skipped"]


class ConnectorItem(ApiModel):
    id: str
    name: str
    status: ConnectorStatus
    enabled: bool
    scopes: list[str]
    oauth_configured: bool = False
    real_sync_supported: bool = True
    sync_interval_minutes: int
    last_sync_at: str | None = None
    next_sync_at: str | None = None
    token_stored: bool
    token_updated_at: str | None = None
    last_error: str | None = None
    updated_at: str


class ConnectorListResponse(ApiModel):
    items: list[ConnectorItem]


class ConnectorSettingsPatch(ApiModel):
    enabled: bool | None = None
    sync_interval_minutes: int | None = Field(default=None, ge=15, le=1440)


class ConnectorOAuthStartRequest(ApiModel):
    redirect_uri: str | None = None


class ConnectorOAuthStartResponse(ApiModel):
    connector: ConnectorItem
    authorization_url: str
    state: str
    scopes: list[str]
    redirect_uri: str
    expires_at: str
    mock: bool = True
    oauth_configured: bool = False


class ConnectorOAuthCompleteRequest(ApiModel):
    state: str
    code: str
    user_approved: bool = True


class ConnectorDisconnectResponse(ApiModel):
    connector: ConnectorItem
    token_deleted: bool


class ConnectorSyncRequest(ApiModel):
    reason: str = "manual"


class ConnectorSyncRun(ApiModel):
    id: str
    connector_id: str
    status: ConnectorSyncRunStatus
    reason: str
    started_at: str
    completed_at: str | None = None
    items_seen: int = 0
    items_written: int = 0
    error_message: str | None = None


class ConnectorSyncResponse(ApiModel):
    connector: ConnectorItem
    run: ConnectorSyncRun


class ConnectorSyncRunsResponse(ApiModel):
    items: list[ConnectorSyncRun]
    total: int
