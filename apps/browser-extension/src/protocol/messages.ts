import {
  BROWSER_PROTOCOL_VERSION,
  type BrowserBridgeEnvelope,
  type BrowserContextMode,
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

export interface CapturePageResult {
  ok: boolean;
  context?: BrowserPageContext;
  instruction?: string;
}

export interface PopupState {
  connected: boolean;
  browserName: string;
  instruction: string;
}

export function createEnvelope<TPayload extends Record<string, unknown>>(
  type: string,
  payload: TPayload,
  requestId = crypto.randomUUID(),
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
