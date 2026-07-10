import type { BrowserContextMode } from "@deyana/schemas";
import { resetPageSession } from "../adapters/genericPageAdapter";
import { adapterForCurrentPage } from "../adapters/registry";
import type {
  CapturePageMessage,
  CapturePageResult,
  ClickActionResult,
  FillFieldResult
} from "../protocol/messages";

interface ContentBridgeMessage {
  type?:
    | CapturePageMessage["type"]
    | "deyana.disconnect-page"
    | "deyana.fill-field"
    | "deyana.clear-field"
    | "deyana.click-action";
  mode?: BrowserContextMode;
  pageSessionId?: string;
  fieldHandle?: string;
  value?: string;
  restoreOriginal?: boolean;
  actionId?: string;
  expectedText?: string | null;
  targetLabel?: string | null;
  userApproved?: boolean;
}

const INSTALLATION_MARKER = "data-deyana-content-bridge";

if (!document.documentElement.hasAttribute(INSTALLATION_MARKER)) {
  document.documentElement.setAttribute(INSTALLATION_MARKER, "1");
  chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
    const candidate = message as ContentBridgeMessage;
    if (candidate.type === "deyana.capture-page") {
      try {
        const context = adapterForCurrentPage().extract(
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

    if (candidate.type === "deyana.fill-field") {
      const response = adapterForCurrentPage().fillField({
        pageSessionId: String(candidate.pageSessionId ?? ""),
        fieldHandle: String(candidate.fieldHandle ?? ""),
        value: String(candidate.value ?? ""),
        userApproved: Boolean(candidate.userApproved)
      });
      sendResponse({ ok: response.status === "completed", ...response } satisfies FillFieldResult);
      return true;
    }

    if (candidate.type === "deyana.clear-field") {
      const response = adapterForCurrentPage().clearField({
        pageSessionId: String(candidate.pageSessionId ?? ""),
        fieldHandle: String(candidate.fieldHandle ?? ""),
        restoreOriginal: candidate.restoreOriginal !== false,
        userApproved: Boolean(candidate.userApproved)
      });
      sendResponse({ ok: response.status === "completed", ...response } satisfies FillFieldResult);
      return true;
    }

    if (candidate.type === "deyana.click-action") {
      void adapterForCurrentPage()
        .clickAction({
          pageSessionId: String(candidate.pageSessionId ?? ""),
          actionId: String(candidate.actionId ?? ""),
          expectedText: candidate.expectedText ?? null,
          targetLabel: candidate.targetLabel ?? null,
          userApproved: Boolean(candidate.userApproved)
        })
        .then((response) => {
          sendResponse({ ok: response.status === "completed", ...response } satisfies ClickActionResult);
        });
      return true;
    }
    return false;
  });
}
