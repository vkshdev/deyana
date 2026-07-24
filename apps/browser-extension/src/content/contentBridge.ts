import type { BrowserContextMode } from "@deyana/schemas";
import { genericPageAdapter, resetPageSession } from "../adapters/genericPageAdapter";
import type {
  CapturePageMessage,
  CapturePageResult,
  DisconnectPageMessage
} from "../protocol/messages";

const INSTALLATION_MARKER = "data-deyana-content-bridge";

if (!document.documentElement.hasAttribute(INSTALLATION_MARKER)) {
  document.documentElement.setAttribute(INSTALLATION_MARKER, "1");
  chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
    const candidate = message as Partial<CapturePageMessage | DisconnectPageMessage>;
    if (candidate.type === "deyana.capture-page") {
      try {
        const context = genericPageAdapter.extract(
          (candidate.mode as BrowserContextMode | undefined) ?? "main"
        );
        sendResponse({ ok: true, context } satisfies CapturePageResult);
      } catch (error) {
        sendResponse({
          ok: false,
          instruction: error instanceof Error ? error.message : "Unable to read this page."
        } satisfies CapturePageResult);
      }
      return true;
    }

    if (candidate.type === "deyana.disconnect-page") {
      resetPageSession();
      sendResponse({ ok: true } satisfies CapturePageResult);
      return true;
    }
    return false;
  });
}
