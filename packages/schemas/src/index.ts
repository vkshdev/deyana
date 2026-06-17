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

export type ModelStatus =
  | "available"
  | "missing"
  | "checking"
  | "offline";

export type SyncStatus = "idle" | "syncing" | "paused" | "error";

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
  syncMode: SyncMode;
  vaultPath?: string | null;
  onboardingCompleted: boolean;
  updatedAt: string;
}

export interface SettingsPatch {
  privacyMode?: PrivacyMode;
  modelProfile?: ModelProfile;
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
  createdAt: string;
  updatedAt: string;
  deletedAt?: string | null;
}

export interface MemoryCreateRequest {
  type?: MemoryType;
  title: string;
  summary: string;
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

export interface MemoryDeleteResponse {
  deleted: boolean;
  id: string;
}

export interface MemoryReindexResponse {
  reindexed: number;
  missingMarkdown: number;
}

export interface MemoryExportResponse {
  exportedAt: string;
  items: MemoryItem[];
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

export type Phase3CoreEvent = SettingsChangedEvent | VaultSelectedEvent | OnboardingStateChangedEvent;
export type Phase4CoreEvent =
  | MemoryItemCreatedEvent
  | MemoryItemUpdatedEvent
  | MemoryItemDeletedEvent
  | MemoryReindexedEvent;

export type AppCoreEvent = CoreWebSocketEvent | Phase3CoreEvent | Phase4CoreEvent;

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
