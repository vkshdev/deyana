import type {
  BrowserClearFieldRequest,
  BrowserClickActionRequest,
  BrowserClickActionResponse,
  BrowserContextMode,
  BrowserFillFieldRequest,
  BrowserFillFieldResponse,
  BrowserLandmark,
  BrowserPageContext,
  BrowserWritableField
} from "@deyana/schemas";
import type { BrowserSiteAdapter } from "./BrowserSiteAdapter";
import { genericPageAdapter } from "./genericPageAdapter";

const MAX_CONTEXT_CHARACTERS = 80 * 1024;
const MAX_ELEMENTS = 60;

export interface DraftFirstAdapterConfig {
  id: string;
  origin?: string;
  allowedOrigins?: readonly string[];
  allowedHostnames?: readonly string[];
  pathPrefixes?: readonly string[];
  titlePrefix: string;
  readLabel: string;
  draftLabel: string;
  fieldLabel: string;
  selectors: readonly string[];
  titleSelectors: readonly string[];
}

export function createDraftFirstAdapter(config: DraftFirstAdapterConfig): BrowserSiteAdapter {
  return new DraftFirstAdapter(config);
}

class DraftFirstAdapter implements BrowserSiteAdapter {
  readonly version = 1;

  constructor(private readonly config: DraftFirstAdapterConfig) {}

  get id(): string {
    return this.config.id;
  }

  get allowedOrigins(): readonly string[] | undefined {
    return this.config.allowedOrigins ?? (this.config.origin ? [this.config.origin] : undefined);
  }

  get allowedHostnames(): readonly string[] | undefined {
    return this.config.allowedHostnames;
  }

  get pathPrefixes(): readonly string[] | undefined {
    return this.config.pathPrefixes;
  }

  matches(url: URL): boolean {
    if (this.config.allowedOrigins?.length && !this.config.allowedOrigins.includes(url.origin)) {
      if (!this.config.allowedHostnames?.length) {
        return false;
      }
    }
    if (this.config.allowedHostnames?.length && !this.config.allowedHostnames.includes(url.hostname)) {
      return false;
    }
    if (this.config.origin && !this.config.allowedOrigins && url.origin !== this.config.origin) {
      return false;
    }
    if (this.config.pathPrefixes?.length && !this.config.pathPrefixes.some((prefix) => url.pathname.startsWith(prefix))) {
      return false;
    }
    return true;
  }

  extract(mode: BrowserContextMode): BrowserPageContext {
    const url = new URL(window.location.href);
    if (!this.matches(url)) {
      throw new Error(`Open ${this.config.titlePrefix} before using this adapter.`);
    }

    const visibleText = collectVisibleText(this.config.selectors).slice(0, MAX_CONTEXT_CHARACTERS);
    const fields = this.listWritableFields();
    const title = adapterTitle(this.config);
    const landmarks: BrowserLandmark[] = [
      {
        role: "main",
        name: title,
        text: visibleText
      }
    ];

    return {
      pageSessionId: genericPageAdapter.extract("visible").pageSessionId,
      origin: window.location.origin,
      url: `${url.origin}${url.pathname}`,
      title,
      adapterId: this.id,
      adapterVersion: this.version,
      mode,
      capturedAt: new Date().toISOString(),
      visibleText,
      selectionText: window.getSelection()?.toString().trim() || null,
      mainText: visibleText,
      landmarks,
      availableActions: [
        {
          actionId: `${this.id}_draft_reply`,
          capability: "message.draft_reply",
          label: this.config.draftLabel
        },
        {
          actionId: `${this.id}_fill_field`,
          capability: "field.fill",
          label: this.config.fieldLabel
        }
      ],
      writableFields: fields,
      adapterHealth: {
        adapterId: this.id,
        adapterVersion: this.version,
        state: visibleText ? "available" : "degraded",
        detail: visibleText
          ? `${this.config.titlePrefix} visible context was detected. Confirmed send remains disabled until adapter fixtures pass.`
          : `${this.config.titlePrefix} context was not detected clearly.`,
        capabilities: [
          {
            id: "page.read",
            label: this.config.readLabel,
            available: Boolean(visibleText),
            detail: "Reads only visible semantic text from the current tab."
          },
          {
            id: "field.detect",
            label: "Detect safe reply fields",
            available: fields.length > 0,
            detail: "Uses the generic field registry with password, OTP, payment, hidden, disabled, and readonly exclusions."
          },
          {
            id: "field.fill",
            label: "Fill reviewed draft",
            available: fields.length > 0,
            detail: "Inserts reviewed text without submitting."
          },
          {
            id: "message.draft_reply",
            label: "Draft reply",
            available: Boolean(visibleText),
            detail: "Generates a local draft from visible context."
          },
          {
            id: "message.send_reply",
            label: "Confirmed send",
            available: false,
            detail: "Disabled until adapter-specific selectors, duplicate-write prevention, and verification fixtures are implemented."
          }
        ]
      },
      characterCount: visibleText.length,
      truncated: visibleText.length >= MAX_CONTEXT_CHARACTERS
    };
  }

  listWritableFields(): BrowserWritableField[] {
    return genericPageAdapter.listWritableFields().slice(0, 4);
  }

  fillField(request: BrowserFillFieldRequest): BrowserFillFieldResponse {
    return genericPageAdapter.fillField(request);
  }

  clearField(request: BrowserClearFieldRequest): BrowserFillFieldResponse {
    return genericPageAdapter.clearField(request);
  }

  async clickAction(request: BrowserClickActionRequest): Promise<BrowserClickActionResponse> {
    return {
      status: "failed",
      actionId: request.actionId,
      clicked: false,
      verified: false,
      instruction: `${this.config.titlePrefix} confirmed send is disabled until adapter-specific verification passes.`
    };
  }
}

function adapterTitle(config: DraftFirstAdapterConfig): string {
  for (const selector of config.titleSelectors) {
    const element = document.querySelector<HTMLElement>(selector);
    const text = normalizeText(element?.innerText || element?.textContent || element?.getAttribute("aria-label") || "");
    if (text) {
      return `${config.titlePrefix} - ${text}`.slice(0, 180);
    }
  }
  return document.title ? `${config.titlePrefix} - ${normalizeText(document.title)}` : config.titlePrefix;
}

function collectVisibleText(selectors: readonly string[]): string {
  const candidates = selectors.flatMap((selector) => [...document.querySelectorAll<HTMLElement>(selector)]);
  const messages: string[] = [];
  const seen = new Set<string>();
  for (const element of candidates) {
    if (!isVisible(element)) {
      continue;
    }
    const text = normalizeText(element.innerText || element.textContent || "");
    if (!text || text.length < 2 || seen.has(text)) {
      continue;
    }
    seen.add(text);
    messages.push(text);
    if (messages.length >= MAX_ELEMENTS) {
      break;
    }
  }
  return messages.join("\n");
}

function isVisible(element: HTMLElement): boolean {
  const style = window.getComputedStyle(element);
  const bounds = element.getBoundingClientRect();
  return style.display !== "none" && style.visibility !== "hidden" && bounds.width > 0 && bounds.height > 0;
}

function normalizeText(value: string): string {
  return value.replace(/\s+/g, " ").trim();
}
