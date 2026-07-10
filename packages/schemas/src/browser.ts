export const BROWSER_PROTOCOL_VERSION = 1 as const;

export type BrowserContextMode = "selection" | "main" | "visible";
export type BrowserConnectionState = "disconnected" | "connecting" | "connected" | "error";
export type BrowserRequestStatus = "completed" | "permission_required" | "unavailable" | "failed";
export type BrowserPermissionKind = "temporary_active_tab" | "optional_origin";
export type BrowserAuditDecision = "allowed" | "blocked" | "failed";
export type BrowserFieldKind = "textarea" | "text_input" | "contenteditable";
export type BrowserAdapterHealthState = "available" | "degraded" | "unsupported";
export type BrowserDraftTone = "reply" | "regenerate" | "shorten" | "formalize";
export type BrowserActionKind = "open_tab" | "fill_field" | "click";
export type BrowserActionPlanStatus =
  | "pending_confirmation"
  | "confirmed"
  | "policy_authorized"
  | "executing"
  | "completed"
  | "cancelled"
  | "expired"
  | "failed"
  | "blocked";
export type WhatsAppBusyDecision = "allowed" | "blocked" | "requires_confirmation";
export type WhatsAppBusyCategory =
  | "normal"
  | "urgent"
  | "otp"
  | "password"
  | "payment"
  | "legal"
  | "medical"
  | "security"
  | "unknown";
export type BrowserPersonalityPreset = "concise" | "supportive" | "playful" | "professional" | "custom";

export interface BrowserLandmark {
  role: string;
  name: string;
  text: string;
}

export interface BrowserAvailableAction {
  actionId: string;
  capability: "page.read" | "tab.open" | "field.detect" | "field.fill" | "message.draft_reply" | "message.send_reply";
  label: string;
}

export interface BrowserAdapterCapability {
  id: "page.read" | "tab.open" | "field.detect" | "field.fill" | "message.draft_reply" | "message.send_reply";
  label: string;
  available: boolean;
  detail: string;
}

export interface BrowserAdapterHealth {
  adapterId: string;
  adapterVersion: number;
  state: BrowserAdapterHealthState;
  detail: string;
  capabilities: BrowserAdapterCapability[];
}

export interface BrowserWritableField {
  handle: string;
  kind: BrowserFieldKind;
  label: string;
  placeholder?: string | null;
  valuePreview: string;
  valueCharacterCount: number;
  maxLength?: number | null;
  required: boolean;
  disabled: boolean;
  capturedAt: string;
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
  writableFields: BrowserWritableField[];
  adapterHealth: BrowserAdapterHealth;
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

export interface BrowserDraftReplyRequest {
  instruction: string;
  mode?: BrowserContextMode;
  pageSessionId?: string | null;
  fieldHandle?: string | null;
  tone?: BrowserDraftTone;
  userApproved?: boolean;
}

export interface BrowserDraftReplyResponse {
  status: BrowserRequestStatus;
  draft: string;
  field?: BrowserWritableField | null;
  context?: BrowserPageContext | null;
  model?: string | null;
  latencyMs: number;
  instruction?: string | null;
}

export interface BrowserFillFieldRequest {
  pageSessionId: string;
  fieldHandle: string;
  value: string;
  userApproved?: boolean;
}

export interface BrowserFillFieldResponse {
  status: BrowserRequestStatus;
  fieldHandle: string;
  inserted: boolean;
  previousValuePreview?: string | null;
  instruction?: string | null;
}

export interface BrowserClearFieldRequest {
  pageSessionId: string;
  fieldHandle: string;
  restoreOriginal?: boolean;
  userApproved?: boolean;
}

export interface BrowserActionStep {
  id: string;
  kind: BrowserActionKind;
  label: string;
  origin?: string | null;
  pageSessionId?: string | null;
  fieldHandle?: string | null;
  url?: string | null;
  value?: string | null;
  actionId?: string | null;
  targetLabel?: string | null;
  verification: string;
}

export interface BrowserActionPlan {
  id: string;
  status: BrowserActionPlanStatus;
  summary: string;
  previewMarkdown: string;
  origin?: string | null;
  pageSessionId?: string | null;
  steps: BrowserActionStep[];
  confirmationToken?: string | null;
  expiresAt: string;
  createdAt: string;
  updatedAt: string;
  resultDetail?: string | null;
}

export interface BrowserActionPlanCreateRequest {
  chain: "open_url" | "fill_field" | "whatsapp_send";
  url?: string | null;
  pageSessionId?: string | null;
  fieldHandle?: string | null;
  value?: string | null;
  targetLabel?: string | null;
  userApproved?: boolean;
}

export interface BrowserClickActionRequest {
  pageSessionId: string;
  actionId: string;
  expectedText?: string | null;
  targetLabel?: string | null;
  userApproved?: boolean;
}

export interface BrowserClickActionResponse {
  status: BrowserRequestStatus;
  actionId: string;
  clicked: boolean;
  verified: boolean;
  instruction?: string | null;
}

export interface BrowserActionPlanResponse {
  status: BrowserRequestStatus;
  plan?: BrowserActionPlan | null;
  instruction?: string | null;
}

export interface BrowserActionPlanListResponse {
  items: BrowserActionPlan[];
  total: number;
}

export interface BrowserActionConfirmRequest {
  planId: string;
  confirmationToken: string;
}

export interface BrowserEmergencyStopResponse {
  stopped: boolean;
  cancelledPlans: number;
  instruction: string;
}

export interface WhatsAppBusyModePolicy {
  enabled: boolean;
  allowlistedContacts: string[];
  allowGroups: boolean;
  timezone: string;
  windowStart: string;
  windowEnd: string;
  cooldownMinutes: number;
  dailyLimit: number;
  template: string;
  emergencyStopped: boolean;
  permissionOrigin: string;
  permissionGranted: boolean;
  updatedAt: string;
}

export interface WhatsAppBusyModePolicyPatch {
  enabled?: boolean;
  allowlistedContacts?: string[];
  allowGroups?: boolean;
  timezone?: string;
  windowStart?: string;
  windowEnd?: string;
  cooldownMinutes?: number;
  dailyLimit?: number;
  template?: string;
  resetEmergencyStop?: boolean;
}

export interface WhatsAppBusyModePolicyResponse {
  status: BrowserRequestStatus;
  policy: WhatsAppBusyModePolicy;
  permission?: BrowserPermissionResponse | null;
  instruction?: string | null;
}

export interface WhatsAppBusyModeEvaluationRequest {
  pageSessionId?: string | null;
  contactLabel?: string | null;
  isGroup?: boolean | null;
  latestMessageText?: string | null;
  userApproved?: boolean;
}

export interface WhatsAppBusyModeEvaluationResponse {
  status: BrowserRequestStatus;
  allowed: boolean;
  decision: WhatsAppBusyDecision;
  reason: string;
  category: WhatsAppBusyCategory;
  urgencyDetected: boolean;
  ownerNotification: boolean;
  targetLabel?: string | null;
  draft?: string | null;
  policy: WhatsAppBusyModePolicy;
}

export interface WhatsAppBusyModeSendRequest {
  pageSessionId: string;
  fieldHandle?: string | null;
  latestMessageText?: string | null;
  userApproved?: boolean;
}

export interface WhatsAppBusyModeSendResponse {
  status: BrowserRequestStatus;
  evaluation: WhatsAppBusyModeEvaluationResponse;
  plan?: BrowserActionPlan | null;
  instruction?: string | null;
}

export interface BrowserPersonalityProfile {
  preset: BrowserPersonalityPreset;
  displayName: string;
  customInstruction: string;
  writerTemperature: number;
  maxDraftCharacters: number;
  automationDisclosure: string;
  updatedAt: string;
}

export interface BrowserPersonalityProfilePatch {
  preset?: BrowserPersonalityPreset;
  displayName?: string;
  customInstruction?: string;
  writerTemperature?: number;
  maxDraftCharacters?: number;
  automationDisclosure?: string;
}

export interface BrowserContactTonePreference {
  adapterId: string;
  contactLabel: string;
  toneInstruction: string;
  approved: boolean;
  updatedAt: string;
}

export interface BrowserContactTonePreferenceRequest {
  adapterId: string;
  contactLabel: string;
  toneInstruction: string;
  approved?: boolean;
}

export interface BrowserMoodHint {
  label: string;
  confidence: number;
  expiresAt: string;
  persisted: boolean;
}

export interface BrowserMoodInferRequest {
  text: string;
  ttlSeconds?: number;
}

export interface BrowserPersonalitySettingsResponse {
  profile: BrowserPersonalityProfile;
  contactTones: BrowserContactTonePreference[];
  moodHint?: BrowserMoodHint | null;
}

export interface BrowserPersonalityPreviewRequest {
  sampleText?: string;
  contactLabel?: string | null;
  adapterId?: string | null;
}

export interface BrowserPersonalityPreviewResponse {
  preview: string;
  profile: BrowserPersonalityProfile;
  contactTone?: BrowserContactTonePreference | null;
  moodHint?: BrowserMoodHint | null;
}

export type BrowserVoiceIntent = "read_page" | "summarize_page" | "draft_reply" | "search_web" | "open_url" | "unknown";

export interface BrowserVoiceCommandRequest {
  transcript: string;
  mode?: BrowserContextMode;
  pageSessionId?: string | null;
  userApproved?: boolean;
}

export interface BrowserVoiceCommandResponse {
  status: BrowserRequestStatus;
  transcriptPreview: string;
  intent: BrowserVoiceIntent;
  instruction: string;
  summary?: BrowserContextSummaryResponse | null;
  draft?: BrowserDraftReplyResponse | null;
  search?: BrowserSearchResponse | null;
  actionPlan?: BrowserActionPlan | null;
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
