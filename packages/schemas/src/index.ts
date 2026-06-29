export const ASSISTANT_STATES = [
  "BOOTING",
  "ONBOARDING",
  "IDLE",
  "COMPACT_FLOATING",
  "EXPANDED_PANEL",
  "LISTENING",
  "TRANSCRIBING",
  "THINKING",
  "RETRIEVING_MEMORY",
  "SUMMARIZING",
  "SEARCHING_WEB",
  "READING_FILE",
  "CODING",
  "SYNCING",
  "SPEAKING",
  "WAITING_FOR_CONFIRMATION",
  "BLOCKED_BY_PRIVACY",
  "CONNECTOR_ERROR",
  "MODEL_MISSING",
  "OFFLINE",
  "ERROR",
  "SHUTTING_DOWN"
] as const;

export type AssistantState = (typeof ASSISTANT_STATES)[number];

export const UI_MODES = ["compact", "expanded"] as const;

export type UiMode = (typeof UI_MODES)[number];

export type ConnectorStatus =
  | "not_connected"
  | "connected"
  | "syncing"
  | "paused"
  | "error";

export type ConnectorSyncRunStatus =
  | "queued"
  | "running"
  | "completed"
  | "failed"
  | "skipped";

export type ModelStatus =
  | "available"
  | "missing"
  | "checking"
  | "offline";

export type SyncStatus = "idle" | "syncing" | "paused" | "error";
export type PrivacyDecision = "allow" | "block";
export type PrivacyDestinationCategory =
  | "local"
  | "public_web"
  | "oauth_connector"
  | "cloud_ai"
  | "hosted_embedding"
  | "hosted_reranker"
  | "cloud_stt"
  | "cloud_tts"
  | "unknown_external";
export type PrivacyDataCategory =
  | "public_query"
  | "public_content"
  | "oauth_token"
  | "connector_metadata"
  | "private_memory"
  | "memory_summary"
  | "embedding_text"
  | "audio"
  | "transcript"
  | "source_code"
  | "local_file"
  | "chat_history"
  | "unknown";
export type PrivacyRequestPurpose =
  | "public_web_fetch"
  | "oauth_api_fetch"
  | "connector_api_fetch"
  | "cloud_ai"
  | "embedding"
  | "reranking"
  | "speech_to_text"
  | "text_to_speech"
  | "unknown";
export type ToolId =
  | "web_search"
  | "fetch_page"
  | "read_file"
  | "git_status"
  | "git_diff"
  | "commit_message"
  | "code_task"
  | "day_planner";
export type ToolRisk = "low" | "public_web" | "local_file" | "source_code" | "dangerous";
export type ToolRunStatus = "completed" | "permission_required" | "confirmation_required" | "blocked" | "failed";
export type VoiceEngineStatus = "available" | "disabled" | "muted" | "unsupported" | "missing" | "error";
export type ReleaseCheckStatus = "ready" | "warning" | "missing" | "blocked";
export type ConnectorHealthStatus = "healthy" | "not_connected" | "paused" | "syncing" | "error" | "attention";

export const BACKEND_LIFECYCLES = [
  "starting",
  "running",
  "stopping",
  "stopped",
  "crashed",
  "unavailable"
] as const;

export type BackendLifecycle = (typeof BACKEND_LIFECYCLES)[number];

export interface BackendProcessStatus {
  lifecycle: BackendLifecycle;
  endpoint: string;
  pid?: number;
  startedAtMs?: number;
  updatedAtMs: number;
  restartCount: number;
  lastError?: string;
}

export interface BackendHealthResponse {
  status: "ok";
  service: "deyana-core";
  version: string;
  lifecycle: "running" | "stopping";
  uptimeSeconds: number;
  timestamp: string;
}

export interface BackendDependencyStatus {
  name: string;
  status: "available" | "missing" | "not_configured" | "deferred";
  detail: string;
}

export interface BackendStatusResponse {
  service: "deyana-core";
  version: string;
  lifecycle: "running" | "stopping";
  bootId: string;
  pid: number;
  uptimeSeconds: number;
  host: string;
  port: number;
  dependencies: BackendDependencyStatus[];
  featureFlags: Record<string, boolean>;
  timestamp: string;
}

export interface BackendEvent<TType extends string = string, TPayload = Record<string, unknown>> {
  id: string;
  type: TType;
  timestamp: string;
  payload: TPayload;
}

export type AppReadyEvent = BackendEvent<
  "app.ready",
  {
    service: "deyana-core";
    version: string;
    lifecycle: "running";
    bootId: string;
  }
>;

export type BackendHeartbeatEvent = BackendEvent<
  "backend.heartbeat",
  {
    lifecycle: "running" | "stopping";
    uptimeSeconds: number;
  }
>;

export type BackendLifecycleEvent = BackendEvent<
  "backend.lifecycle.changed",
  {
    lifecycle: BackendLifecycle;
    reason: string;
  }
>;

export type CoreWebSocketEvent = AppReadyEvent | BackendHeartbeatEvent | BackendLifecycleEvent;

export const PRIVACY_MODES = ["local_only"] as const;
export type PrivacyMode = (typeof PRIVACY_MODES)[number];

export const MODEL_PROFILES = ["low_spec", "balanced", "power"] as const;
export type ModelProfile = (typeof MODEL_PROFILES)[number];

export const SYNC_MODES = ["manual", "low_frequency"] as const;
export type SyncMode = (typeof SYNC_MODES)[number];

export const ONBOARDING_STEPS = ["welcome", "privacy", "local_ai", "vault", "complete"] as const;
export type OnboardingStep = (typeof ONBOARDING_STEPS)[number];

export type VaultStatus = "not_selected" | "ready" | "missing" | "error";
export type MemoryType =
  | "chat"
  | "note"
  | "connector_summary"
  | "file_summary"
  | "git_summary"
  | "daily_summary"
  | "project_summary"
  | "decision"
  | "action_item";

export interface CoreAppSettings {
  privacyMode: PrivacyMode;
  modelProfile: ModelProfile;
  selectedChatModel: string;
  selectedEmbeddingModel: string;
  syncMode: SyncMode;
  vaultPath?: string | null;
  onboardingCompleted: boolean;
  updatedAt: string;
}

export interface SettingsPatch {
  privacyMode?: PrivacyMode;
  modelProfile?: ModelProfile;
  selectedChatModel?: string;
  selectedEmbeddingModel?: string;
  syncMode?: SyncMode;
}

export interface OnboardingState {
  completed: boolean;
  completedAt?: string | null;
  currentStep: OnboardingStep;
  selectedVaultPath?: string | null;
  selectedPrivacyMode: PrivacyMode;
  selectedModelProfile: ModelProfile;
  vaultStatus: VaultStatus;
  vaultError?: string | null;
  vaultFolders: string[];
}

export interface VaultSelectRequest {
  path: string;
}

export interface VaultSelectResponse {
  state: OnboardingState;
  settings: CoreAppSettings;
  vaultPath: string;
  createdFolders: string[];
}

export interface OnboardingCompleteRequest {
  privacyMode: PrivacyMode;
  modelProfile: ModelProfile;
  vaultPath?: string | null;
}

export interface OnboardingCompleteResponse {
  state: OnboardingState;
  settings: CoreAppSettings;
}

export interface MemoryEntity {
  id: string;
  memoryId: string;
  memoryTitle?: string | null;
  sourceType?: string | null;
  sourceId?: string | null;
  sourceUri?: string | null;
  name: string;
  entityType: string;
  sourceText: string;
  createdAt: string;
}

export type MemoryInsightType = "action_item" | "decision";

export interface MemoryInsight {
  id: string;
  memoryId: string;
  memoryTitle?: string | null;
  sourceType?: string | null;
  sourceId?: string | null;
  sourceUri?: string | null;
  type: MemoryInsightType;
  title: string;
  detail: string;
  status: string;
  dueAt?: string | null;
  createdAt: string;
}

export interface MemoryItem {
  id: string;
  type: MemoryType;
  title: string;
  summary: string;
  contentMarkdown: string;
  markdownPath?: string | null;
  sourceType: string;
  sourceId?: string | null;
  sourceUri?: string | null;
  importance: number;
  tags: string[];
  entities: MemoryEntity[];
  actionItems: MemoryInsight[];
  decisions: MemoryInsight[];
  createdAt: string;
  updatedAt: string;
  deletedAt?: string | null;
}

export interface MemoryCreateRequest {
  type?: MemoryType;
  title: string;
  summary?: string;
  contentMarkdown?: string | null;
  sourceType?: string;
  sourceId?: string | null;
  sourceUri?: string | null;
  importance?: number;
  tags?: string[];
}

export interface MemoryUpdateRequest {
  title?: string;
  summary?: string;
  contentMarkdown?: string;
  importance?: number;
  tags?: string[];
}

export interface MemoryListResponse {
  items: MemoryItem[];
  total: number;
  query?: string | null;
}

export interface MemoryEntityListResponse {
  items: MemoryEntity[];
  total: number;
  query?: string | null;
  sourceType?: string | null;
  sourceId?: string | null;
  date?: string | null;
}

export interface MemoryInsightListResponse {
  items: MemoryInsight[];
  total: number;
  query?: string | null;
  type?: MemoryInsightType | null;
  status?: string | null;
  sourceType?: string | null;
  sourceId?: string | null;
  date?: string | null;
}

export interface MemoryDeleteResponse {
  deleted: boolean;
  id: string;
}

export interface MemoryReindexResponse {
  reindexed: number;
  missingMarkdown: number;
}

export interface DailySummaryRequest {
  date?: string | null;
}

export interface ProjectSummaryRequest {
  project: string;
}

export interface MemoryExportResponse {
  exportedAt: string;
  items: MemoryItem[];
}

export type LocalModelRole = "chat" | "embedding" | "unknown";
export type ModelTask =
  | "chat"
  | "summarization"
  | "compression"
  | "planning"
  | "classification"
  | "embedding"
  | "coding";

export interface LocalModelInfo {
  name: string;
  role: LocalModelRole;
  installed: boolean;
  recommended: boolean;
  profile?: ModelProfile | null;
  sizeBytes?: number | null;
  detail: string;
}

export interface LocalModelStatusResponse {
  provider: "ollama";
  status: ModelStatus;
  endpoint: string;
  selectedChatModel: string;
  selectedEmbeddingModel: string;
  recommendedChatModel: string;
  recommendedEmbeddingModel: string;
  chatModelAvailable: boolean;
  embeddingModelAvailable: boolean;
  availableModels: LocalModelInfo[];
  setupModels: LocalModelInfo[];
  maxParallelModelJobs: number;
  think: boolean;
  message: string;
  checkedAt: string;
}

export interface ModelSelectionRequest {
  chatModel?: string | null;
  embeddingModel?: string | null;
  profile?: ModelProfile | null;
}

export interface ModelSelectionResponse {
  settings: CoreAppSettings;
  status: LocalModelStatusResponse;
}

export interface ModelTestRequest {
  prompt?: string;
}

export interface ModelTestResponse {
  ok: boolean;
  model: string;
  response: string;
  latencyMs: number;
  detail: string;
}

export interface ChatMessageRequest {
  content: string;
  useMemory?: boolean;
}

export interface MemorySourceReference {
  id: string;
  title: string;
  label: string;
  markdownPath?: string | null;
  sourceType: string;
  sourceUri?: string | null;
  snippet: string;
  score: number;
  updatedAt: string;
}

export interface ChatMessageItem {
  id: string;
  role: "user" | "assistant";
  content: string;
  model?: string | null;
  sourceReferences: MemorySourceReference[];
  createdAt: string;
}

export interface ChatRetrievalSummary {
  query: string;
  retrieved: number;
  compressedCharacters: number;
  contextTokensEstimate: number;
}

export interface ChatMessageResponse {
  userMessage: ChatMessageItem;
  assistantMessage: ChatMessageItem;
  model: string;
  latencyMs: number;
  sources: MemorySourceReference[];
  retrieval: ChatRetrievalSummary;
}

export interface ChatHistoryResponse {
  messages: ChatMessageItem[];
}

export interface ChatHistoryDeleteResponse {
  deleted: number;
}

export interface PrivacyCheckRequest {
  url: string;
  method?: string;
  purpose?: PrivacyRequestPurpose;
  dataCategory?: PrivacyDataCategory | null;
  payloadPreview?: string | null;
  userApproved?: boolean;
  connectorId?: string | null;
  externalWrite?: boolean;
}

export interface PrivacyAuditEvent {
  id: string;
  eventType: "privacy.request.blocked" | "privacy.request.allowed" | string;
  decision: PrivacyDecision;
  reason: string;
  destination: string;
  destinationCategory: PrivacyDestinationCategory;
  dataCategory: PrivacyDataCategory;
  purpose: PrivacyRequestPurpose;
  method: string;
  userApproved: boolean;
  connectorId?: string | null;
  safeAlternative: string;
  payloadSha256?: string | null;
  payloadCharacterCount: number;
  createdAt: string;
}

export interface PrivacyCheckResponse {
  allowed: boolean;
  decision: PrivacyDecision;
  reason: string;
  destination: string;
  destinationCategory: PrivacyDestinationCategory;
  dataCategory: PrivacyDataCategory;
  purpose: PrivacyRequestPurpose;
  safeAlternative: string;
  auditEvent: PrivacyAuditEvent;
}

export interface PrivacyAuditListResponse {
  events: PrivacyAuditEvent[];
  total: number;
}

export interface PrivacyStatusResponse {
  mode: PrivacyMode;
  enforced: boolean;
  auditEvents: number;
  blockedEvents: number;
  allowedEvents: number;
  lastBlocked?: PrivacyAuditEvent | null;
  blockedCategories: PrivacyDestinationCategory[];
}

export interface PrivacyAuditDeleteResponse {
  deleted: number;
}

export interface ToolManifest {
  toolId: ToolId;
  name: string;
  description: string;
  risk: ToolRisk;
  requiresApproval: boolean;
  dangerous: boolean;
  appliesChanges: boolean;
}

export interface ToolListResponse {
  tools: ToolManifest[];
}

export interface ToolResultItem {
  title: string;
  summary: string;
  url?: string | null;
  source?: string | null;
}

export interface ToolRunResponse {
  toolId: ToolId;
  status: ToolRunStatus;
  title: string;
  summary: string;
  content: string;
  items: ToolResultItem[];
  permissionRequired: boolean;
  confirmationRequired: boolean;
  appliesChanges: boolean;
  privacy?: PrivacyCheckResponse | null;
}

export interface WebSearchRequest {
  query: string;
  limit?: number;
  userApproved?: boolean;
}

export interface WebFetchRequest {
  url: string;
  userApproved?: boolean;
  maxCharacters?: number;
}

export interface FileReadRequest {
  path: string;
  allowedRoot: string;
  userApproved?: boolean;
  maxCharacters?: number;
}

export interface GitReadRequest {
  repoPath: string;
  userApproved?: boolean;
  maxCharacters?: number;
}

export interface CodeTaskRequest {
  goal: string;
  context?: string;
  userApproved?: boolean;
}

export interface DayPlannerRequest {
  date?: string | null;
  commitments?: string[];
  focus?: string[];
  notes?: string;
}

export interface VoiceSettings {
  enabled: boolean;
  muted: boolean;
  ttsEnabled: boolean;
  transcriptRetention: "none" | "memory";
  sttEngine: "windows_speech";
  ttsEngine: "windows_speech";
  language: string;
  listenSeconds: number;
  ttsVoice?: string | null;
  ttsRate: number;
  ttsVolume: number;
  updatedAt: string;
}

export interface VoiceSettingsPatch {
  enabled?: boolean;
  muted?: boolean;
  ttsEnabled?: boolean;
  transcriptRetention?: "none" | "memory";
  language?: string;
  listenSeconds?: number;
  ttsVoice?: string | null;
  ttsRate?: number;
  ttsVolume?: number;
}

export interface VoiceOption {
  name: string;
  gender: "female" | "male" | "neutral" | "unknown";
  language: string;
  isSystemDefault: boolean;
}

export interface VoiceStatusResponse {
  enabled: boolean;
  muted: boolean;
  ttsEnabled: boolean;
  sttStatus: VoiceEngineStatus;
  ttsStatus: VoiceEngineStatus;
  sttEngine: string;
  ttsEngine: string;
  language: string;
  activeTtsVoice?: string | null;
  availableTtsVoices: VoiceOption[];
  rawAudioStored: boolean;
  detail: string;
  checkedAt: string;
}

export interface VoiceTranscriptRequest {
  listenSeconds?: number | null;
}

export interface VoiceTranscriptResponse {
  transcript: string;
  engine: string;
  language: string;
  durationSeconds: number;
  rawAudioStored: boolean;
  createdAt: string;
}

export interface VoiceSpeakRequest {
  text: string;
}

export interface VoiceSpeakResponse {
  spoken: boolean;
  engine: string;
  characters: number;
  rawAudioStored: boolean;
  createdAt: string;
}

export interface ConnectorItem {
  id: string;
  name: string;
  status: ConnectorStatus;
  enabled: boolean;
  scopes: string[];
  oauthConfigured: boolean;
  realSyncSupported: boolean;
  syncIntervalMinutes: number;
  lastSyncAt?: string | null;
  nextSyncAt?: string | null;
  tokenStored: boolean;
  tokenUpdatedAt?: string | null;
  lastError?: string | null;
  updatedAt: string;
}

export interface ConnectorListResponse {
  items: ConnectorItem[];
}

export interface ConnectorSettingsPatch {
  enabled?: boolean;
  syncIntervalMinutes?: number;
}

export interface ConnectorOAuthStartRequest {
  redirectUri?: string | null;
}

export interface ConnectorOAuthStartResponse {
  connector: ConnectorItem;
  authorizationUrl: string;
  state: string;
  scopes: string[];
  redirectUri: string;
  expiresAt: string;
  mock: boolean;
  oauthConfigured: boolean;
}

export interface ConnectorOAuthCompleteRequest {
  state: string;
  code: string;
  userApproved?: boolean;
}

export interface ConnectorDisconnectResponse {
  connector: ConnectorItem;
  tokenDeleted: boolean;
}

export interface ConnectorSyncRequest {
  reason?: string;
}

export interface ConnectorSyncRun {
  id: string;
  connectorId: string;
  status: ConnectorSyncRunStatus;
  reason: string;
  startedAt: string;
  completedAt?: string | null;
  itemsSeen: number;
  itemsWritten: number;
  errorMessage?: string | null;
}

export interface ConnectorSyncResponse {
  connector: ConnectorItem;
  run: ConnectorSyncRun;
}

export interface ConnectorSyncRunsResponse {
  items: ConnectorSyncRun[];
  total: number;
}

export interface ReleaseReadinessItem {
  id: string;
  label: string;
  status: ReleaseCheckStatus;
  detail: string;
}

export interface ReleaseReadinessResponse {
  installerReady: boolean;
  updatePlanReady: boolean;
  checkedAt: string;
  items: ReleaseReadinessItem[];
}

export interface ReleaseUpdatePlanResponse {
  currentVersion: string;
  channel: "manual";
  automaticUpdatesEnabled: boolean;
  plan: string[];
  checkedAt: string;
}

export interface ReleaseLogFile {
  path: string;
  name: string;
  sizeBytes: number;
  modifiedAt: string;
}

export interface ReleaseLogListResponse {
  files: ReleaseLogFile[];
  total: number;
}

export interface ReleaseLogReadResponse {
  path: string;
  content: string;
  truncated: boolean;
  sizeBytes: number;
  modifiedAt: string;
}

export interface ReleasePrivacyExportResponse {
  exportedAt: string;
  schemaVersion: number;
  sections: Record<string, unknown>;
  counts: Record<string, number>;
  notes: string[];
}

export interface DeleteLocalDataRequest {
  confirmationPhrase: string;
  includeVault?: boolean;
}

export interface DeleteLocalDataResponse {
  deleted: boolean;
  deletedPaths: string[];
  vaultDeleted: boolean;
  recreatedStores: string[];
  restartRecommended: boolean;
  detail: string;
}

export interface ConnectorHealthItem {
  connectorId: string;
  name: string;
  health: ConnectorHealthStatus;
  status: ConnectorStatus;
  enabled: boolean;
  tokenStored: boolean;
  oauthConfigured: boolean;
  lastSyncAt?: string | null;
  nextSyncAt?: string | null;
  lastError?: string | null;
  detail: string;
}

export interface ConnectorHealthResponse {
  checkedAt: string;
  items: ConnectorHealthItem[];
  healthy: number;
  attention: number;
  errors: number;
}

export interface PerformanceMetric {
  name: string;
  value: number;
  unit: string;
  detail: string;
}

export interface PerformanceProfileResponse {
  capturedAt: string;
  uptimeSeconds: number;
  metrics: PerformanceMetric[];
}

export interface CrashRecoveryResponse {
  currentSessionId: string;
  previousCrashDetected: boolean;
  startedAt: string;
  lastStartedAt?: string | null;
  lastCleanShutdownAt?: string | null;
  recoveryActions: string[];
}

export type SettingsChangedEvent = BackendEvent<
  "settings.changed",
  {
    settings: CoreAppSettings;
  }
>;

export type VaultSelectedEvent = BackendEvent<
  "vault.selected",
  {
    state: OnboardingState;
    settings: CoreAppSettings;
    vaultPath: string;
    createdFolders: string[];
  }
>;

export type OnboardingStateChangedEvent = BackendEvent<
  "onboarding.state.changed",
  {
    state: OnboardingState;
    settings: CoreAppSettings;
  }
>;

export type MemoryItemCreatedEvent = BackendEvent<"memory.item.created", { item: MemoryItem }>;
export type MemoryItemUpdatedEvent = BackendEvent<"memory.item.updated", { item: MemoryItem }>;
export type MemoryItemDeletedEvent = BackendEvent<"memory.item.deleted", { id: string; deleted: boolean }>;
export type MemoryReindexedEvent = BackendEvent<"memory.reindexed", MemoryReindexResponse>;
export type MemorySummaryGeneratedEvent = BackendEvent<
  "memory.summary.generated",
  {
    item: MemoryItem;
    summaryType: "daily" | "project";
  }
>;

export type ModelsStatusChangedEvent = BackendEvent<"models.status.changed", LocalModelStatusResponse>;
export type ModelsTestCompletedEvent = BackendEvent<"models.test.completed", ModelTestResponse>;
export type ChatMessageCreatedEvent = BackendEvent<"chat.message.created", ChatMessageResponse>;
export type ChatHistoryDeletedEvent = BackendEvent<"chat.history.deleted", ChatHistoryDeleteResponse>;
export type PrivacyRequestBlockedEvent = BackendEvent<
  "privacy.request.blocked",
  {
    reason: string;
    destination: string;
    destinationCategory: PrivacyDestinationCategory;
    dataType: PrivacyDataCategory;
    safeAlternative: string;
    auditEvent: PrivacyAuditEvent;
  }
>;
export type PrivacyRequestAllowedEvent = BackendEvent<
  "privacy.request.allowed",
  {
    reason: string;
    destination: string;
    destinationCategory: PrivacyDestinationCategory;
    dataType: PrivacyDataCategory;
    safeAlternative: string;
    auditEvent: PrivacyAuditEvent;
  }
>;
export type PrivacyAuditDeletedEvent = BackendEvent<"privacy.audit.deleted", PrivacyAuditDeleteResponse>;
export type ConnectorStatusChangedEvent = BackendEvent<
  "connector.status.changed",
  {
    connector: ConnectorItem;
  }
>;
export type ConnectorOAuthStartedEvent = BackendEvent<
  "connector.oauth.started",
  ConnectorOAuthStartResponse
>;
export type ConnectorOAuthCompletedEvent = BackendEvent<
  "connector.oauth.completed",
  {
    connector: ConnectorItem;
  }
>;
export type ConnectorSyncStartedEvent = BackendEvent<"connector.sync.started", ConnectorSyncResponse>;
export type ConnectorSyncCompletedEvent = BackendEvent<"connector.sync.completed", ConnectorSyncResponse>;
export type ConnectorSyncFailedEvent = BackendEvent<"connector.sync.failed", ConnectorSyncResponse>;
export type ConnectorSyncSkippedEvent = BackendEvent<"connector.sync.skipped", ConnectorSyncResponse>;
export type ToolCompletedEvent = BackendEvent<"tool.completed", ToolRunResponse>;
export type ToolFailedEvent = BackendEvent<"tool.failed", ToolRunResponse>;
export type ToolPermissionRequiredEvent = BackendEvent<"tool.permission.required", ToolRunResponse>;
export type VoiceSettingsUpdatedEvent = BackendEvent<"voice.settings.updated", VoiceSettings>;
export type VoiceRecordingStartedEvent = BackendEvent<"voice.recording.started", { rawAudioStored: boolean }>;
export type VoiceRecordingStoppedEvent = BackendEvent<"voice.recording.stopped", { rawAudioStored: boolean }>;
export type VoiceTranscriptionStartedEvent = BackendEvent<"voice.transcription.started", { engine: string }>;
export type VoiceTranscriptionCompletedEvent = BackendEvent<"voice.transcription.completed", VoiceTranscriptResponse>;
export type VoiceTranscriptionFailedEvent = BackendEvent<
  "voice.transcription.failed",
  { reason: string; rawAudioStored: boolean }
>;
export type TtsStartedEvent = BackendEvent<"tts.started", { engine: string }>;
export type TtsCompletedEvent = BackendEvent<"tts.completed", VoiceSpeakResponse>;
export type TtsFailedEvent = BackendEvent<"tts.failed", { reason: string; rawAudioStored: boolean }>;
export type ReleaseLocalDataDeletedEvent = BackendEvent<"release.local_data.deleted", DeleteLocalDataResponse>;

export type Phase3CoreEvent = SettingsChangedEvent | VaultSelectedEvent | OnboardingStateChangedEvent;
export type Phase4CoreEvent =
  | MemoryItemCreatedEvent
  | MemoryItemUpdatedEvent
  | MemoryItemDeletedEvent
  | MemoryReindexedEvent
  | MemorySummaryGeneratedEvent;
export type Phase5CoreEvent =
  | ModelsStatusChangedEvent
  | ModelsTestCompletedEvent
  | ChatMessageCreatedEvent
  | ChatHistoryDeletedEvent;
export type Phase7CoreEvent =
  | PrivacyRequestBlockedEvent
  | PrivacyRequestAllowedEvent
  | PrivacyAuditDeletedEvent;
export type Phase8CoreEvent =
  | ConnectorStatusChangedEvent
  | ConnectorOAuthStartedEvent
  | ConnectorOAuthCompletedEvent
  | ConnectorSyncStartedEvent
  | ConnectorSyncCompletedEvent
  | ConnectorSyncFailedEvent
  | ConnectorSyncSkippedEvent;
export type Phase11CoreEvent = ToolCompletedEvent | ToolFailedEvent | ToolPermissionRequiredEvent;
export type Phase13CoreEvent =
  | VoiceSettingsUpdatedEvent
  | VoiceRecordingStartedEvent
  | VoiceRecordingStoppedEvent
  | VoiceTranscriptionStartedEvent
  | VoiceTranscriptionCompletedEvent
  | VoiceTranscriptionFailedEvent
  | TtsStartedEvent
  | TtsCompletedEvent
  | TtsFailedEvent;
export type Phase15CoreEvent = ReleaseLocalDataDeletedEvent;

export type AppCoreEvent =
  | CoreWebSocketEvent
  | Phase3CoreEvent
  | Phase4CoreEvent
  | Phase5CoreEvent
  | Phase7CoreEvent
  | Phase8CoreEvent
  | Phase11CoreEvent
  | Phase13CoreEvent
  | Phase15CoreEvent;

export interface FloatingWindowPosition {
  x: number;
  y: number;
  monitor?: string;
}

export interface DesktopSettings {
  uiMode: UiMode;
  alwaysOnTop: boolean;
  lowPowerMode: boolean;
  reduceMotion: boolean;
  lastPosition?: FloatingWindowPosition;
}

export interface AssistantStateEvent {
  type: "assistant.state.changed";
  payload: {
    from: AssistantState;
    to: AssistantState;
    timestamp: string;
  };
}

export interface UiFloatingPositionEvent {
  type: "ui.floating.position.updated";
  payload: {
    position: FloatingWindowPosition;
    timestamp: string;
  };
}

export type Phase1Event = AssistantStateEvent | UiFloatingPositionEvent;

export interface ConnectorPreview {
  id: string;
  name: string;
  status: ConnectorStatus;
  lastSyncLabel: string;
}

export interface MemoryPreviewItem {
  id: string;
  title: string;
  source: string;
  updatedLabel: string;
}

export interface QuickAction {
  id: string;
  label: string;
  state: AssistantState;
}

export const DEFAULT_DESKTOP_SETTINGS: DesktopSettings = {
  uiMode: "compact",
  alwaysOnTop: true,
  lowPowerMode: true,
  reduceMotion: false
};

export const DEFAULT_CORE_APP_SETTINGS: CoreAppSettings = {
  privacyMode: "local_only",
  modelProfile: "low_spec",
  selectedChatModel: "qwen3:1.7b",
  selectedEmbeddingModel: "all-minilm:latest",
  syncMode: "manual",
  vaultPath: null,
  onboardingCompleted: false,
  updatedAt: ""
};

export const DEFAULT_ONBOARDING_STATE: OnboardingState = {
  completed: false,
  completedAt: null,
  currentStep: "welcome",
  selectedVaultPath: null,
  selectedPrivacyMode: "local_only",
  selectedModelProfile: "low_spec",
  vaultStatus: "not_selected",
  vaultError: null,
  vaultFolders: [
    "Daily",
    "Projects",
    "People",
    "Meetings",
    "Emails",
    "GitHub",
    "Slack",
    "Tasks",
    "Decisions",
    "Stripe",
    "Sources",
    "Inbox"
  ]
};


export const DEFAULT_BACKEND_PROCESS_STATUS: BackendProcessStatus = {
  lifecycle: "unavailable",
  endpoint: "http://127.0.0.1:8765",
  updatedAtMs: 0,
  restartCount: 0
};
