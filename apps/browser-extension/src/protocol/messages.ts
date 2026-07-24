import {
  BROWSER_PROTOCOL_VERSION,
  type BrowserBridgeEnvelope,
  type BrowserClearFieldRequest,
  type BrowserClickActionRequest,
  type BrowserClickActionResponse,
  type BrowserContextMode,
  type BrowserFillFieldRequest,
  type BrowserFillFieldResponse,
  type BrowserPageContext,
  type BrowserPermission
} from "@deyana/schemas";

export const NATIVE_HOST_NAME = "app.deyana.browser";

export interface CapturePageMessage {
  type: "deyana.capture-page";
  mode: BrowserContextMode;
}

export interface DisconnectPageMessage {
  type: "deyana.disconnect-page";
  pageSessionId: string;
}

export interface FillFieldMessage extends BrowserFillFieldRequest {
  type: "deyana.fill-field";
}

export interface ClearFieldMessage extends BrowserClearFieldRequest {
  type: "deyana.clear-field";
}

export interface ClickActionMessage extends BrowserClickActionRequest {
  type: "deyana.click-action";
}

export interface CapturePageResult {
  ok: boolean;
  context?: BrowserPageContext;
  instruction?: string;
}

export interface FillFieldResult extends BrowserFillFieldResponse {
  ok: boolean;
}

export interface ClickActionResult extends BrowserClickActionResponse {
  ok: boolean;
}

export interface PopupState {
  connected: boolean;
  browserName: string;
  instruction: string;
}

export function createEnvelope<TPayload extends object>(
  type: string,
  payload: TPayload,
  requestId: string = crypto.randomUUID(),
  pageSessionId?: string | null
): BrowserBridgeEnvelope<TPayload> {
  return {
    protocolVersion: BROWSER_PROTOCOL_VERSION,
    requestId,
    type,
    timestamp: new Date().toISOString(),
    pageSessionId,
    payload
  };
}

export function isBridgeEnvelope(value: unknown): value is BrowserBridgeEnvelope {
  if (!value || typeof value !== "object") {
    return false;
  }
  const candidate = value as Partial<BrowserBridgeEnvelope>;
  return (
    candidate.protocolVersion === BROWSER_PROTOCOL_VERSION &&
    typeof candidate.requestId === "string" &&
    typeof candidate.type === "string" &&
    typeof candidate.timestamp === "string" &&
    typeof candidate.payload === "object" &&
    candidate.payload !== null
  );
}

export function permissionPayload(origins: string[]): BrowserPermission[] {
  const grantedAt = new Date().toISOString();
  return origins.map((origin) => ({
    origin,
    kind: "optional_origin",
    granted: true,
    grantedAt,
    detail: "Optional site permission granted in the browser."
  }));
}
