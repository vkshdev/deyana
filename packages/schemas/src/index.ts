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


export const DEFAULT_BACKEND_PROCESS_STATUS: BackendProcessStatus = {
  lifecycle: "unavailable",
  endpoint: "http://127.0.0.1:8765",
  updatedAtMs: 0,
  restartCount: 0
};
