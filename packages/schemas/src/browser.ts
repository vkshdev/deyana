export const BROWSER_PROTOCOL_VERSION = 1 as const;

export type BrowserContextMode = "selection" | "main" | "visible";
export type BrowserConnectionState = "disconnected" | "connecting" | "connected" | "error";
export type BrowserRequestStatus = "completed" | "permission_required" | "unavailable" | "failed";
export type BrowserPermissionKind = "temporary_active_tab" | "optional_origin";
export type BrowserAuditDecision = "allowed" | "blocked" | "failed";

export interface BrowserLandmark {
  role: string;
  name: string;
  text: string;
}

export interface BrowserAvailableAction {
  actionId: string;
  capability: "page.read" | "tab.open";
  label: string;
}

export interface BrowserPageContext {
  pageSessionId: string;
  origin: string;
  url: string;
  title: string;
  adapterId: string;
  adapterVersion: number;
  mode: BrowserContextMode;
  capturedAt: string;
  visibleText: string;
  selectionText?: string | null;
  mainText?: string | null;
  landmarks: BrowserLandmark[];
  availableActions: BrowserAvailableAction[];
  characterCount: number;
  truncated: boolean;
}

export interface BrowserSession {
  id: string;
  origin: string;
  url: string;
  title: string;
  adapterId: string;
  mode: BrowserContextMode;
  characterCount: number;
  truncated: boolean;
  createdAt: string;
  updatedAt: string;
  expiresAt: string;
}

export interface BrowserSessionListResponse {
  items: BrowserSession[];
  total: number;
}

export interface BrowserPermission {
  origin: string;
  kind: BrowserPermissionKind;
  granted: boolean;
  grantedAt?: string | null;
  detail: string;
}

export interface BrowserPermissionListResponse {
  items: BrowserPermission[];
  total: number;
}

export interface BrowserPermissionRequest {
  origin?: string | null;
  kind?: BrowserPermissionKind;
}

export interface BrowserPermissionResponse {
  status: BrowserRequestStatus;
  permission?: BrowserPermission | null;
  instruction: string;
}

export interface BrowserStatusResponse {
  state: BrowserConnectionState;
  connected: boolean;
  browserName?: string | null;
  browserVersion?: string | null;
  extensionVersion?: string | null;
  extensionOrigin?: string | null;
  protocolVersion: number;
  activeSessions: number;
  permissions: number;
  lastConnectedAt?: string | null;
  lastMessageAt?: string | null;
  lastError?: string | null;
  credentialPath: string;
}

export interface BrowserContextReadRequest {
  mode?: BrowserContextMode;
  userApproved?: boolean;
}

export interface BrowserContextReadResponse {
  status: BrowserRequestStatus;
  context?: BrowserPageContext | null;
  instruction?: string | null;
}

export interface BrowserContextSummaryRequest {
  mode?: BrowserContextMode;
  instruction?: string;
  userApproved?: boolean;
}

export interface BrowserContextSummaryResponse {
  status: BrowserRequestStatus;
  summary: string;
  model?: string | null;
  latencyMs: number;
  context?: BrowserPageContext | null;
  instruction?: string | null;
}

export interface BrowserSearchRequest {
  query: string;
  limit?: number;
  userApproved?: boolean;
}

export interface BrowserSearchItem {
  title: string;
  summary: string;
  url?: string | null;
  source?: string | null;
}

export interface BrowserSearchResponse {
  status: BrowserRequestStatus;
  query: string;
  items: BrowserSearchItem[];
  summary: string;
}

export interface BrowserOpenTabRequest {
  url: string;
  active?: boolean;
  userApproved?: boolean;
}

export interface BrowserOpenTabResponse {
  status: BrowserRequestStatus;
  url: string;
  tabId?: number | null;
  instruction?: string | null;
}

export interface BrowserDisconnectResponse {
  disconnected: boolean;
  pageSessionId: string;
}

export interface BrowserAuditEvent {
  id: string;
  eventType: string;
  decision: BrowserAuditDecision;
  operation: string;
  origin?: string | null;
  pageSessionId?: string | null;
  detail: string;
  payloadSha256?: string | null;
  payloadCharacterCount: number;
  createdAt: string;
}

export interface BrowserAuditListResponse {
  items: BrowserAuditEvent[];
  total: number;
}

export interface BrowserBridgeEnvelope<TPayload = Record<string, unknown>> {
  protocolVersion: number;
  requestId: string;
  type: string;
  timestamp: string;
  pageSessionId?: string | null;
  payload: TPayload;
}

export interface BrowserBridgeReadyPayload {
  browserName: string;
  browserVersion?: string | null;
  extensionVersion: string;
  permissions: BrowserPermission[];
}

export type BrowserConnectionChangedEvent = {
  id: string;
  type: "browser.connection.changed";
  timestamp: string;
  payload: BrowserStatusResponse;
};

export type BrowserPageContextUpdatedEvent = {
  id: string;
  type: "browser.page.context.updated";
  timestamp: string;
  payload: {
    context: BrowserPageContext;
    session: BrowserSession;
  };
};

export type BrowserPageSessionClosedEvent = {
  id: string;
  type: "browser.page.session.closed";
  timestamp: string;
  payload: { pageSessionId: string };
};

export type BrowserPermissionChangedEvent = {
  id: string;
  type: "browser.permission.changed";
  timestamp: string;
  payload: { permissions: BrowserPermission[] };
};

export type BrowserCoreEvent =
  | BrowserConnectionChangedEvent
  | BrowserPageContextUpdatedEvent
  | BrowserPageSessionClosedEvent
  | BrowserPermissionChangedEvent;
