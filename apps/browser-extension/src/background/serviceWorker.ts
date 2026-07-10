import type {
  BrowserBridgeEnvelope,
  BrowserClickActionResponse,
  BrowserContextMode,
  BrowserFillFieldResponse,
  BrowserOpenTabResponse,
  BrowserPageContext,
  BrowserPermission,
  BrowserPermissionResponse
} from "@deyana/schemas";
import {
  NATIVE_HOST_NAME,
  createEnvelope,
  isBridgeEnvelope,
  permissionPayload,
  type CapturePageResult,
  type ClickActionResult,
  type FillFieldResult,
  type PopupState
} from "../protocol/messages";

const SUPPORTED_OPTIONAL_ORIGINS = new Set([
  "https://web.whatsapp.com/*",
  "https://mail.google.com/*",
  "https://www.linkedin.com/*",
  "https://app.slack.com/*",
  "https://github.com/*"
]);

let nativePort: chrome.runtime.Port | null = null;
let nativeConnected = false;
const pageSessions = new Map<number, string>();

chrome.runtime.onInstalled.addListener(() => {
  void configureContextMenu();
  void connectNativeHost();
});

chrome.runtime.onStartup.addListener(() => {
  void connectNativeHost();
});

chrome.commands.onCommand.addListener((command) => {
  if (command === "capture-active-page") {
    void captureAndPublish("main");
  }
});

chrome.contextMenus.onClicked.addListener((info) => {
  if (info.menuItemId === "deyana-read-page") {
    void captureAndPublish(info.selectionText ? "selection" : "main");
  }
});

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  const candidate = message as { type?: string; mode?: BrowserContextMode };
  if (candidate.type === "popup.state") {
    sendResponse({
      connected: nativeConnected,
      browserName: browserName(),
      instruction: nativeConnected
        ? "Connected to local Deyana."
        : "Start Deyana, then reconnect from this popup."
    } satisfies PopupState);
    return false;
  }

  if (candidate.type === "popup.reconnect") {
    void connectNativeHost().then(() => {
      sendResponse({
        connected: nativeConnected,
        browserName: browserName(),
        instruction: nativeConnected
          ? "Connected to local Deyana."
          : "The Deyana native messaging host is unavailable."
      } satisfies PopupState);
    });
    return true;
  }

  if (candidate.type === "popup.capture") {
    void captureAndPublish(candidate.mode ?? "main").then(sendResponse);
    return true;
  }
  return false;
});

chrome.tabs.onUpdated.addListener((tabId, changeInfo) => {
  if (!changeInfo.url && changeInfo.status !== "loading") {
    return;
  }
  void closePageSession(tabId, "The page navigated or reloaded.");
});

chrome.tabs.onRemoved.addListener((tabId) => {
  void closePageSession(tabId, "The browser tab was closed.");
});

async function connectNativeHost(): Promise<void> {
  if (nativePort) {
    return;
  }
  try {
    const port = chrome.runtime.connectNative(NATIVE_HOST_NAME);
    nativePort = port;
    port.onMessage.addListener((message) => {
      void handleNativeMessage(message);
    });
    port.onDisconnect.addListener(() => {
      void chrome.runtime.lastError;
      if (nativePort === port) {
        nativePort = null;
        nativeConnected = false;
      }
    });
    nativeConnected = true;
    port.postMessage(
      createEnvelope("browser.bridge.ready", {
        browserName: browserName(),
        browserVersion: browserVersion(),
        extensionVersion: chrome.runtime.getManifest().version,
        permissions: await currentPermissions()
      })
    );
  } catch {
    nativePort = null;
    nativeConnected = false;
  }
}

async function configureContextMenu(): Promise<void> {
  await chrome.contextMenus.removeAll();
  chrome.contextMenus.create({
    id: "deyana-read-page",
    title: "Read with Deyana",
    contexts: ["page", "selection"]
  });
}

async function handleNativeMessage(message: unknown): Promise<void> {
  if (!isBridgeEnvelope(message)) {
    return;
  }
  const envelope = message as BrowserBridgeEnvelope<Record<string, unknown>>;
  if (envelope.type === "browser.context.read.requested") {
    const mode = normalizeMode(envelope.payload.mode);
    const result = await captureActivePage(mode);
    postNative(
      createEnvelope(
        result.ok ? "browser.context.read.completed" : "browser.context.read.failed",
        result.ok
          ? { status: "completed", context: result.context }
          : {
              status: result.instruction?.includes("toolbar")
                ? "permission_required"
                : "failed",
              instruction: result.instruction
            },
        envelope.requestId,
        result.context?.pageSessionId ?? null
      )
    );
    return;
  }

  if (envelope.type === "browser.tab.open.requested") {
    const response = await openTab(
      String(envelope.payload.url ?? ""),
      envelope.payload.active !== false
    );
    postNative(
      createEnvelope("browser.tab.open.completed", response, envelope.requestId)
    );
    return;
  }

  if (envelope.type === "browser.permission.requested") {
    const response = await requestOptionalPermission(String(envelope.payload.origin ?? ""));
    postNative(
      createEnvelope("browser.permission.request.completed", response, envelope.requestId)
    );
    await publishPermissionChange();
    return;
  }

  if (envelope.type === "browser.permission.revoke.requested") {
    const response = await revokeOptionalPermission(String(envelope.payload.origin ?? ""));
    postNative(
      createEnvelope("browser.permission.revoke.completed", response, envelope.requestId)
    );
    await publishPermissionChange();
    return;
  }

  if (envelope.type === "browser.session.disconnect.requested") {
    const requestedSession = String(envelope.payload.pageSessionId ?? envelope.pageSessionId ?? "");
    const entry = [...pageSessions.entries()].find(([, sessionId]) => sessionId === requestedSession);
    if (entry) {
      const [tabId] = entry;
      try {
        await chrome.tabs.sendMessage(tabId, {
          type: "deyana.disconnect-page",
          pageSessionId: requestedSession
        });
      } catch {
        // The tab may already be gone; local session removal is still authoritative.
      }
      pageSessions.delete(tabId);
    }
    postNative(
      createEnvelope(
        "browser.session.disconnect.completed",
        { disconnected: Boolean(entry), pageSessionId: requestedSession },
        envelope.requestId,
        requestedSession || null
      )
    );
    return;
  }

  if (envelope.type === "browser.field.fill.requested") {
    const response = await fillField(envelope);
    postNative(
      createEnvelope("browser.field.fill.completed", response, envelope.requestId, envelope.pageSessionId)
    );
    return;
  }

  if (envelope.type === "browser.field.clear.requested") {
    const response = await clearField(envelope);
    postNative(
      createEnvelope("browser.field.clear.completed", response, envelope.requestId, envelope.pageSessionId)
    );
    return;
  }

  if (envelope.type === "browser.action.click.requested") {
    const response = await clickAction(envelope);
    postNative(
      createEnvelope("browser.action.click.completed", response, envelope.requestId, envelope.pageSessionId)
    );
  }
}

async function captureAndPublish(mode: BrowserContextMode): Promise<CapturePageResult> {
  await connectNativeHost();
  const result = await captureActivePage(mode);
  if (result.ok && result.context) {
    postNative(
      createEnvelope(
        "browser.page.context.updated",
        { context: result.context },
        undefined,
        result.context.pageSessionId
      )
    );
  }
  return result;
}

async function captureActivePage(mode: BrowserContextMode): Promise<CapturePageResult> {
  const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
  const tab = tabs[0];
  if (!tab?.id || !isSupportedPageUrl(tab.url)) {
    return {
      ok: false,
      instruction: "Open a normal HTTP or HTTPS page before asking Deyana to read it."
    };
  }

  try {
    await ensureContentBridge(tab.id);
    const result = await chrome.tabs.sendMessage<CapturePageResult>(tab.id, {
      type: "deyana.capture-page",
      mode
    });
    if (result.ok && result.context) {
      pageSessions.set(tab.id, result.context.pageSessionId);
    }
    return result;
  } catch {
    return {
      ok: false,
      instruction:
        "Use the Deyana extension toolbar action, context menu, or Alt+Shift+D in this tab to grant temporary activeTab access."
    };
  }
}

async function ensureContentBridge(tabId: number): Promise<void> {
  await chrome.scripting.executeScript({
    target: { tabId },
    files: ["content-bridge.js"]
  });
}

function tabIdForPageSession(pageSessionId: string): number | null {
  const entry = [...pageSessions.entries()].find(([, sessionId]) => sessionId === pageSessionId);
  return entry?.[0] ?? null;
}

async function openTab(url: string, active: boolean): Promise<BrowserOpenTabResponse> {
  if (!isSafeHttpUrl(url)) {
    return {
      status: "failed",
      url,
      instruction: "Only HTTP and HTTPS URLs without embedded credentials can be opened."
    };
  }
  try {
    const tab = await chrome.tabs.create({ url, active });
    return { status: "completed", url, tabId: tab.id ?? null };
  } catch {
    return { status: "failed", url, instruction: "The browser could not open this URL." };
  }
}

async function fillField(
  envelope: BrowserBridgeEnvelope<Record<string, unknown>>
): Promise<BrowserFillFieldResponse> {
  const pageSessionId = String(envelope.payload.pageSessionId ?? envelope.pageSessionId ?? "");
  const fieldHandle = String(envelope.payload.fieldHandle ?? "");
  const tabId = tabIdForPageSession(pageSessionId);
  if (!tabId) {
    return {
      status: "failed",
      fieldHandle,
      inserted: false,
      instruction: "The target page session is no longer active."
    };
  }

  try {
    await ensureContentBridge(tabId);
    const result = await chrome.tabs.sendMessage<FillFieldResult>(tabId, {
      type: "deyana.fill-field",
      pageSessionId,
      fieldHandle,
      value: String(envelope.payload.value ?? ""),
      userApproved: Boolean(envelope.payload.userApproved)
    });
    return result;
  } catch {
    return {
      status: "failed",
      fieldHandle,
      inserted: false,
      instruction: "Deyana could not reach the target field in the browser tab."
    };
  }
}

async function clearField(
  envelope: BrowserBridgeEnvelope<Record<string, unknown>>
): Promise<BrowserFillFieldResponse> {
  const pageSessionId = String(envelope.payload.pageSessionId ?? envelope.pageSessionId ?? "");
  const fieldHandle = String(envelope.payload.fieldHandle ?? "");
  const tabId = tabIdForPageSession(pageSessionId);
  if (!tabId) {
    return {
      status: "failed",
      fieldHandle,
      inserted: false,
      instruction: "The target page session is no longer active."
    };
  }

  try {
    await ensureContentBridge(tabId);
    const result = await chrome.tabs.sendMessage<FillFieldResult>(tabId, {
      type: "deyana.clear-field",
      pageSessionId,
      fieldHandle,
      restoreOriginal: envelope.payload.restoreOriginal !== false,
      userApproved: Boolean(envelope.payload.userApproved)
    });
    return result;
  } catch {
    return {
      status: "failed",
      fieldHandle,
      inserted: false,
      instruction: "Deyana could not restore or clear the target field."
    };
  }
}

async function clickAction(
  envelope: BrowserBridgeEnvelope<Record<string, unknown>>
): Promise<BrowserClickActionResponse> {
  const pageSessionId = String(envelope.payload.pageSessionId ?? envelope.pageSessionId ?? "");
  const actionId = String(envelope.payload.actionId ?? "");
  const tabId = tabIdForPageSession(pageSessionId);
  if (!tabId) {
    return {
      status: "failed",
      actionId,
      clicked: false,
      verified: false,
      instruction: "The target page session is no longer active."
    };
  }

  try {
    await ensureContentBridge(tabId);
    const result = await chrome.tabs.sendMessage<ClickActionResult>(tabId, {
      type: "deyana.click-action",
      pageSessionId,
      actionId,
      expectedText: String(envelope.payload.expectedText ?? ""),
      targetLabel: String(envelope.payload.targetLabel ?? ""),
      userApproved: Boolean(envelope.payload.userApproved)
    });
    return result;
  } catch {
    return {
      status: "failed",
      actionId,
      clicked: false,
      verified: false,
      instruction: "Deyana could not execute the adapter-declared browser action."
    };
  }
}

async function requestOptionalPermission(origin: string): Promise<BrowserPermissionResponse> {
  const pattern = originPattern(origin);
  if (!pattern || !SUPPORTED_OPTIONAL_ORIGINS.has(pattern)) {
    return {
      status: "failed",
      instruction: "This origin is not declared as an optional Deyana permission."
    };
  }
  const granted = await chrome.permissions.request({ origins: [pattern] });
  return {
    status: granted ? "completed" : "permission_required",
    permission: {
      origin: pattern,
      kind: "optional_origin",
      granted,
      grantedAt: granted ? new Date().toISOString() : null,
      detail: granted
        ? "Optional site permission granted in the browser."
        : "The browser permission request was declined."
    },
    instruction: granted
      ? "Optional site permission granted."
      : "Approve the optional site permission in the browser."
  };
}

async function revokeOptionalPermission(origin: string): Promise<BrowserPermissionResponse> {
  const pattern = originPattern(origin);
  if (!pattern) {
    return { status: "failed", instruction: "A valid HTTP or HTTPS origin is required." };
  }
  const removed = await chrome.permissions.remove({ origins: [pattern] });
  return {
    status: removed ? "completed" : "failed",
    permission: {
      origin: pattern,
      kind: "optional_origin",
      granted: false,
      grantedAt: null,
      detail: removed ? "Optional site permission revoked." : "Permission was not granted."
    },
    instruction: removed ? "Optional site permission revoked." : "No matching permission was granted."
  };
}

async function currentPermissions(): Promise<BrowserPermission[]> {
  const current = await chrome.permissions.getAll();
  return permissionPayload(current.origins ?? []);
}

async function publishPermissionChange(): Promise<void> {
  postNative(
    createEnvelope("browser.permission.changed", {
      permissions: await currentPermissions()
    })
  );
}

async function closePageSession(tabId: number, reason: string): Promise<void> {
  const pageSessionId = pageSessions.get(tabId);
  if (!pageSessionId) {
    return;
  }
  pageSessions.delete(tabId);
  postNative(
    createEnvelope(
      "browser.page.session.closed",
      { pageSessionId, reason },
      undefined,
      pageSessionId
    )
  );
}

function postNative(message: BrowserBridgeEnvelope<object>): void {
  if (!nativePort) {
    return;
  }
  try {
    nativePort.postMessage(message);
  } catch {
    nativePort = null;
    nativeConnected = false;
  }
}

function normalizeMode(value: unknown): BrowserContextMode {
  return value === "selection" || value === "visible" ? value : "main";
}

function isSupportedPageUrl(url?: string): boolean {
  if (!url) {
    return false;
  }
  try {
    const parsed = new URL(url);
    return parsed.protocol === "http:" || parsed.protocol === "https:";
  } catch {
    return false;
  }
}

function isSafeHttpUrl(value: string): boolean {
  try {
    const parsed = new URL(value);
    return (
      (parsed.protocol === "http:" || parsed.protocol === "https:") &&
      !parsed.username &&
      !parsed.password
    );
  } catch {
    return false;
  }
}

function originPattern(value: string): string | null {
  try {
    const parsed = new URL(value.replace(/\/\*$/, ""));
    if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
      return null;
    }
    return `${parsed.origin}/*`;
  } catch {
    return null;
  }
}

function browserName(): string {
  return navigator.userAgent.includes("Edg/") ? "Microsoft Edge" : "Google Chrome";
}

function browserVersion(): string | null {
  const match = navigator.userAgent.match(/(?:Edg|Chrome)\/(\d+(?:\.\d+)*)/);
  return match?.[1] ?? null;
}
