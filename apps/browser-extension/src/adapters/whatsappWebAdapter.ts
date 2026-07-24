import type {
  BrowserClickActionRequest,
  BrowserClickActionResponse,
  BrowserContextMode,
  BrowserLandmark,
  BrowserPageContext
} from "@deyana/schemas";
import type { BrowserSiteAdapter } from "./BrowserSiteAdapter";
import { genericPageAdapter } from "./genericPageAdapter";

const MAX_CONTEXT_CHARACTERS = 80 * 1024;
const MAX_MESSAGES = 30;

class WhatsAppWebAdapter implements BrowserSiteAdapter {
  readonly id = "whatsapp_web";
  readonly version = 1;

  matches(url: URL): boolean {
    return url.origin === "https://web.whatsapp.com";
  }

  extract(mode: BrowserContextMode): BrowserPageContext {
    const url = new URL(window.location.href);
    if (!this.matches(url)) {
      throw new Error("Open WhatsApp Web before using the WhatsApp adapter.");
    }

    const target = visibleConversationTarget();
    const conversationText = visibleConversationText();
    if (!conversationText) {
      throw new Error("Open a visible WhatsApp conversation before asking Deyana to read it.");
    }

    const boundedText = conversationText.slice(0, MAX_CONTEXT_CHARACTERS);
    const fields = this.listWritableFields();
    const landmarks: BrowserLandmark[] = [
      {
        role: "conversation",
        name: target.name,
        text: boundedText
      }
    ];

    return {
      pageSessionId: genericPageAdapter.extract("visible").pageSessionId,
      origin: window.location.origin,
      url: `${url.origin}${url.pathname}`,
      title: target.name
        ? `${target.isGroup ? "WhatsApp Group" : "WhatsApp"} - ${target.name}`
        : "WhatsApp Web",
      adapterId: this.id,
      adapterVersion: this.version,
      mode,
      capturedAt: new Date().toISOString(),
      visibleText: boundedText,
      selectionText: null,
      mainText: boundedText,
      landmarks,
      availableActions: [
        {
          actionId: "message_draft_reply",
          capability: "message.draft_reply",
          label: "Draft reply for visible WhatsApp conversation"
        },
        {
          actionId: "browser_fill_field",
          capability: "field.fill",
          label: "Insert reviewed draft into WhatsApp composer"
        },
        {
          actionId: "whatsapp_send_message",
          capability: "message.send_reply",
          label: "Send reviewed WhatsApp draft"
        }
      ],
      writableFields: fields,
      adapterHealth: {
        adapterId: this.id,
        adapterVersion: this.version,
        state: fields.length ? "available" : "degraded",
        detail: fields.length
          ? "Visible WhatsApp conversation and composer were detected."
          : "Visible conversation was detected, but the composer is not available.",
        capabilities: [
          {
            id: "page.read",
            label: "Read visible conversation",
            available: true,
            detail: "Reads only the currently open WhatsApp Web conversation."
          },
          {
            id: "field.detect",
            label: "Detect composer",
            available: fields.length > 0,
            detail: "Finds the visible composer while excluding search, OTP, password, and hidden fields."
          },
          {
            id: "field.fill",
            label: "Fill composer",
            available: fields.length > 0,
            detail: "Inserts reviewed draft text into the composer without sending."
          },
          {
            id: "message.draft_reply",
            label: "Draft reply",
            available: true,
            detail: "Generates a local reply bound to the visible WhatsApp target."
          },
          {
            id: "message.send_reply",
            label: "Confirmed send",
            available: fields.length > 0,
            detail: "Sends only after a Deyana action preview and confirmation."
          }
        ]
      },
      characterCount: boundedText.length,
      truncated: conversationText.length > MAX_CONTEXT_CHARACTERS
    };
  }

  listWritableFields() {
    return genericPageAdapter
      .listWritableFields()
      .filter((field) => /message|type|text|visible/i.test(field.label) || field.kind === "contenteditable")
      .slice(0, 2);
  }

  fillField = genericPageAdapter.fillField.bind(genericPageAdapter);

  clearField = genericPageAdapter.clearField.bind(genericPageAdapter);

  async clickAction(request: BrowserClickActionRequest): Promise<BrowserClickActionResponse> {
    if (request.actionId !== "whatsapp_send_message") {
      return {
        status: "failed",
        actionId: request.actionId,
        clicked: false,
        verified: false,
        instruction: "This WhatsApp adapter action is not supported."
      };
    }

    const currentTarget = visibleConversationTarget();
    if (
      request.targetLabel &&
      normalizeText(request.targetLabel).toLowerCase() !== normalizeText(currentTarget.name).toLowerCase()
    ) {
      return {
        status: "failed",
        actionId: request.actionId,
        clicked: false,
        verified: false,
        instruction: "The visible WhatsApp conversation changed before send."
      };
    }

    const button = findSendButton();
    if (!button) {
      return {
        status: "failed",
        actionId: request.actionId,
        clicked: false,
        verified: false,
        instruction: "WhatsApp send button is not available."
      };
    }

    const beforeClickText = visibleConversationText();
    button.click();
    const verified = await waitForOutgoingMessage(request.expectedText ?? "", beforeClickText, 5000);
    return {
      status: verified ? "completed" : "failed",
      actionId: request.actionId,
      clicked: true,
      verified,
      instruction: verified
        ? "Outgoing WhatsApp message appeared in the visible conversation."
        : "Send was clicked, but Deyana could not verify the outgoing message."
    };
  }
}

export const whatsappWebAdapter = new WhatsAppWebAdapter();

function visibleConversationTarget(): { name: string; isGroup: boolean } {
  const main = document.querySelector<HTMLElement>("#main");
  const header = main?.querySelector<HTMLElement>("header");
  const title =
    header?.querySelector<HTMLElement>("[title]")?.getAttribute("title") ||
    header?.querySelector<HTMLElement>("span[dir='auto']")?.textContent ||
    "";
  const subtitle = normalizeText(header?.innerText || "");
  return {
    name: normalizeText(title) || "visible WhatsApp conversation",
    isGroup: /participants|members/i.test(subtitle)
  };
}

function visibleConversationText(): string {
  const main = document.querySelector<HTMLElement>("#main");
  if (!main) {
    return "";
  }

  const candidates = [
    ...main.querySelectorAll<HTMLElement>("[data-testid='msg-container']"),
    ...main.querySelectorAll<HTMLElement>("div[role='row']")
  ];
  const messages: string[] = [];
  const seen = new Set<string>();
  for (const element of candidates) {
    if (!isVisible(element)) {
      continue;
    }
    const text = normalizeText(element.innerText || element.textContent || "");
    if (!text || seen.has(text) || text.length < 2) {
      continue;
    }
    seen.add(text);
    messages.push(text);
    if (messages.length >= MAX_MESSAGES) {
      break;
    }
  }

  if (messages.length) {
    return messages.join("\n");
  }
  return normalizeText(main.innerText || main.textContent || "");
}

function isVisible(element: HTMLElement): boolean {
  const style = window.getComputedStyle(element);
  const bounds = element.getBoundingClientRect();
  return style.display !== "none" && style.visibility !== "hidden" && bounds.width > 0 && bounds.height > 0;
}

function normalizeText(value: string): string {
  return value.replace(/\s+/g, " ").trim();
}

function findSendButton(): HTMLButtonElement | HTMLElement | null {
  const main = document.querySelector<HTMLElement>("#main");
  if (!main) {
    return null;
  }
  const ariaButton = [...main.querySelectorAll<HTMLElement>("button, [role='button']")].find((button) => {
    const label = `${button.getAttribute("aria-label") ?? ""} ${button.getAttribute("title") ?? ""}`.toLowerCase();
    return isVisible(button) && /\bsend\b/.test(label);
  });
  if (ariaButton) {
    return ariaButton;
  }
  const icon = main.querySelector<HTMLElement>("span[data-icon='send'], [data-testid='send']");
  return icon?.closest<HTMLElement>("button, [role='button']") ?? null;
}

async function waitForOutgoingMessage(
  expectedText: string,
  beforeClickText: string,
  timeoutMs: number
): Promise<boolean> {
  const normalizedExpected = normalizeText(expectedText).toLowerCase();
  const normalizedBefore = normalizeText(beforeClickText).toLowerCase();
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const currentText = normalizeText(visibleConversationText()).toLowerCase();
    if (currentText !== normalizedBefore && (!normalizedExpected || currentText.includes(normalizedExpected))) {
      return true;
    }
    await new Promise((resolve) => window.setTimeout(resolve, 250));
  }
  return false;
}
