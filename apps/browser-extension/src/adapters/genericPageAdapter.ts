import type {
  BrowserClearFieldRequest,
  BrowserClickActionRequest,
  BrowserClickActionResponse,
  BrowserFillFieldRequest,
  BrowserFillFieldResponse,
  BrowserContextMode,
  BrowserLandmark,
  BrowserPageContext,
  BrowserWritableField
} from "@deyana/schemas";
import type { BrowserSiteAdapter } from "./BrowserSiteAdapter";

const MAX_CONTEXT_CHARACTERS = 80 * 1024;
const MAX_LANDMARK_CHARACTERS = 12 * 1024;
const MAX_SINGLE_LANDMARK_CHARACTERS = 1500;
const MAX_LANDMARKS = 12;
const MAX_WRITABLE_FIELDS = 8;
const MAX_FIELD_PREVIEW_CHARACTERS = 300;
const FIELD_HANDLE_TTL_MS = 5 * 60 * 1000;
const TEXT_SELECTORS = [
  "h1",
  "h2",
  "h3",
  "h4",
  "h5",
  "h6",
  "p",
  "li",
  "blockquote",
  "pre",
  "figcaption",
  "td",
  "th",
  "[role='heading']",
  "[role='article']",
  "[role='main']"
].join(",");
const WRITABLE_FIELD_SELECTORS = [
  "textarea",
  "input[type='text']",
  "input[type='search']",
  "input[type='email']",
  "input[type='url']",
  "input[type='tel']",
  "input:not([type])",
  "[contenteditable='true']",
  "[role='textbox']"
].join(",");

interface FieldRecord {
  handle: string;
  element: HTMLElement;
  pageSessionId: string;
  capturedAtMs: number;
  capturedValue: string;
  originalValue: string;
  lastWrittenValue?: string;
  dirty: boolean;
  listenerAttached: boolean;
}

class GenericPageAdapter implements BrowserSiteAdapter {
  readonly id = "generic_page";
  readonly version = 1;

  matches(url: URL): boolean {
    return url.protocol === "http:" || url.protocol === "https:";
  }

  extract(mode: BrowserContextMode): BrowserPageContext {
    const url = new URL(window.location.href);
    if (!this.matches(url)) {
      throw new Error("This page type cannot be shared with Deyana.");
    }

    const selectionText = normalizeText(window.getSelection()?.toString() ?? "") || null;
    if (mode === "selection" && !selectionText) {
      throw new Error("Select text on the page before using selected-text mode.");
    }
    const mainRoot = document.querySelector<HTMLElement>("main, [role='main'], article");
    const mainText = mainRoot
      ? collectSemanticText(mainRoot, MAX_CONTEXT_CHARACTERS + 1)
      : null;
    const visibleText = collectSemanticText(document.body, MAX_CONTEXT_CHARACTERS + 1);
    const selectedSource =
      mode === "selection" && selectionText
        ? selectionText
        : mode === "main" && mainText
          ? mainText
          : visibleText;
    const truncated = selectedSource.length > MAX_CONTEXT_CHARACTERS;
    const boundedSource = selectedSource.slice(0, MAX_CONTEXT_CHARACTERS);

    return {
      pageSessionId: pageSessionId(),
      origin: window.location.origin,
      url: sanitizedPageUrl(url),
      title: document.title || window.location.hostname,
      adapterId: this.id,
      adapterVersion: this.version,
      mode,
      capturedAt: new Date().toISOString(),
      visibleText: mode === "visible" ? boundedSource : "",
      selectionText: mode === "selection" ? boundedSource : null,
      mainText: mode === "main" ? boundedSource : null,
      landmarks: collectLandmarks(),
      availableActions: [
        {
          actionId: "page_read",
          capability: "page.read",
          label: "Read visible page context"
        },
        {
          actionId: "message_draft_reply",
          capability: "message.draft_reply",
          label: "Draft a local reply"
        },
        {
          actionId: "browser_fill_field",
          capability: "field.fill",
          label: "Insert a reviewed draft into a visible text field"
        }
      ],
      writableFields: this.listWritableFields(),
      adapterHealth: {
        adapterId: this.id,
        adapterVersion: this.version,
        state: "available",
        detail: "Generic visible text-field adapter is available on this page.",
        capabilities: [
          {
            id: "page.read",
            label: "Read semantic page text",
            available: true,
            detail: "Visible semantic text can be shared after activeTab approval."
          },
          {
            id: "field.detect",
            label: "Detect visible text fields",
            available: true,
            detail: "Password, OTP, payment, hidden, disabled, and readonly fields are excluded."
          },
          {
            id: "field.fill",
            label: "Insert reviewed drafts",
            available: true,
            detail: "Insertion dispatches input/change events but never submits a form."
          },
          {
            id: "message.draft_reply",
            label: "Draft reply",
            available: true,
            detail: "Drafts are generated locally by Deyana and require review before insertion."
          }
        ]
      },
      characterCount: boundedSource.length,
      truncated
    };
  }

  listWritableFields(): BrowserWritableField[] {
    purgeExpiredFieldHandles();
    const fields: BrowserWritableField[] = [];
    const candidates = document.querySelectorAll<HTMLElement>(WRITABLE_FIELD_SELECTORS);
    for (const element of candidates) {
      if (!isSupportedWritableElement(element)) {
        continue;
      }
      fields.push(describeWritableField(element));
      if (fields.length >= MAX_WRITABLE_FIELDS) {
        break;
      }
    }
    return fields;
  }

  fillField(request: BrowserFillFieldRequest): BrowserFillFieldResponse {
    const record = fieldRegistry.get(request.fieldHandle);
    if (!record || record.pageSessionId !== request.pageSessionId) {
      return {
        status: "failed",
        fieldHandle: request.fieldHandle,
        inserted: false,
        instruction: "The target field handle expired or belongs to a different page session."
      };
    }

    const validation = validateFieldRecord(record);
    if (validation) {
      return {
        status: "failed",
        fieldHandle: request.fieldHandle,
        inserted: false,
        instruction: validation
      };
    }

    const currentValue = fieldValue(record.element);
    if (
      currentValue !== record.capturedValue &&
      (!record.lastWrittenValue || currentValue !== record.lastWrittenValue)
    ) {
      record.dirty = true;
      return {
        status: "failed",
        fieldHandle: request.fieldHandle,
        inserted: false,
        instruction: "The target field changed after Deyana captured it. Review the page again."
      };
    }

    const previousValue = currentValue;
    setFieldValue(record.element, request.value);
    record.lastWrittenValue = request.value;
    record.capturedValue = request.value;
    record.dirty = false;
    showInlineDraftUi(record, request.value);

    return {
      status: "completed",
      fieldHandle: request.fieldHandle,
      inserted: true,
      previousValuePreview: previousValue.slice(0, MAX_FIELD_PREVIEW_CHARACTERS),
      instruction: "Draft inserted into the visible field. No submit action was performed."
    };
  }

  clearField(request: BrowserClearFieldRequest): BrowserFillFieldResponse {
    const record = fieldRegistry.get(request.fieldHandle);
    if (!record || record.pageSessionId !== request.pageSessionId) {
      return {
        status: "failed",
        fieldHandle: request.fieldHandle,
        inserted: false,
        instruction: "The target field handle expired or belongs to a different page session."
      };
    }
    const validation = validateFieldRecord(record);
    if (validation) {
      return {
        status: "failed",
        fieldHandle: request.fieldHandle,
        inserted: false,
        instruction: validation
      };
    }
    const nextValue = request.restoreOriginal !== false ? record.originalValue : "";
    setFieldValue(record.element, nextValue);
    record.lastWrittenValue = nextValue;
    record.capturedValue = nextValue;
    record.dirty = false;
    removeInlineDraftUi(record.handle);
    return {
      status: "completed",
      fieldHandle: request.fieldHandle,
      inserted: true,
      previousValuePreview: nextValue.slice(0, MAX_FIELD_PREVIEW_CHARACTERS),
      instruction: request.restoreOriginal !== false ? "Original field value restored." : "Draft cleared."
    };
  }

  async clickAction(request: BrowserClickActionRequest): Promise<BrowserClickActionResponse> {
    return {
      status: "failed",
      actionId: request.actionId,
      clicked: false,
      verified: false,
      instruction: "Generic pages do not support adapter-declared click actions."
    };
  }
}

let currentPageSessionId = crypto.randomUUID();
const fieldRegistry = new Map<string, FieldRecord>();
const elementHandles = new WeakMap<HTMLElement, string>();

export const genericPageAdapter = new GenericPageAdapter();

export function resetPageSession(): void {
  currentPageSessionId = crypto.randomUUID();
  fieldRegistry.clear();
}

function pageSessionId(): string {
  return `page_session_${currentPageSessionId.replaceAll("-", "")}`;
}

function describeWritableField(element: HTMLElement): BrowserWritableField {
  const handle = handleForElement(element);
  const value = fieldValue(element);
  const input = element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement ? element : null;
  return {
    handle,
    kind: fieldKind(element),
    label: fieldLabel(element),
    placeholder: input?.placeholder || element.getAttribute("aria-placeholder") || null,
    valuePreview: value.slice(0, MAX_FIELD_PREVIEW_CHARACTERS),
    valueCharacterCount: value.length,
    maxLength: input?.maxLength && input.maxLength > 0 ? input.maxLength : null,
    required: Boolean(input?.required || element.getAttribute("aria-required") === "true"),
    disabled: Boolean(input?.disabled || element.getAttribute("aria-disabled") === "true"),
    capturedAt: new Date().toISOString()
  };
}

function handleForElement(element: HTMLElement): string {
  const existingHandle = elementHandles.get(element);
  const existingRecord = existingHandle ? fieldRegistry.get(existingHandle) : undefined;
  if (existingRecord && validateFieldRecord(existingRecord) === null) {
    existingRecord.capturedAtMs = Date.now();
    existingRecord.capturedValue = fieldValue(element);
    existingRecord.dirty = false;
    return existingRecord.handle;
  }

  const handle = `field_${crypto.randomUUID().replaceAll("-", "")}`;
  const record: FieldRecord = {
    handle,
    element,
    pageSessionId: pageSessionId(),
    capturedAtMs: Date.now(),
    capturedValue: fieldValue(element),
    originalValue: fieldValue(element),
    dirty: false,
    listenerAttached: false
  };
  attachDirtyListener(record);
  fieldRegistry.set(handle, record);
  elementHandles.set(element, handle);
  return handle;
}

function attachDirtyListener(record: FieldRecord): void {
  if (record.listenerAttached) {
    return;
  }
  record.listenerAttached = true;
  record.element.addEventListener(
    "input",
    () => {
      const currentValue = fieldValue(record.element);
      if (record.lastWrittenValue && currentValue === record.lastWrittenValue) {
        return;
      }
      record.dirty = currentValue !== record.capturedValue;
    },
    { passive: true }
  );
}

function purgeExpiredFieldHandles(): void {
  const now = Date.now();
  for (const [handle, record] of fieldRegistry.entries()) {
    if (now - record.capturedAtMs > FIELD_HANDLE_TTL_MS || !record.element.isConnected) {
      fieldRegistry.delete(handle);
      removeInlineDraftUi(handle);
    }
  }
}

function validateFieldRecord(record: FieldRecord): string | null {
  if (Date.now() - record.capturedAtMs > FIELD_HANDLE_TTL_MS) {
    fieldRegistry.delete(record.handle);
    removeInlineDraftUi(record.handle);
    return "The target field handle expired. Review the page again.";
  }
  if (!record.element.isConnected) {
    fieldRegistry.delete(record.handle);
    removeInlineDraftUi(record.handle);
    return "The target field is no longer on the page.";
  }
  if (!isSupportedWritableElement(record.element)) {
    return "The target field is no longer a safe visible text field.";
  }
  return null;
}

function isSupportedWritableElement(element: HTMLElement): boolean {
  if (!isVisible(element) || isSensitive(element)) {
    return false;
  }
  if (element instanceof HTMLTextAreaElement) {
    return !element.disabled && !element.readOnly;
  }
  if (element instanceof HTMLInputElement) {
    const type = (element.getAttribute("type") || "text").toLowerCase();
    const allowedTypes = new Set(["text", "search", "email", "url", "tel"]);
    return allowedTypes.has(type) && !element.disabled && !element.readOnly;
  }
  if (element.isContentEditable || element.getAttribute("role") === "textbox") {
    return element.getAttribute("aria-readonly") !== "true" && element.getAttribute("aria-disabled") !== "true";
  }
  return false;
}

function fieldKind(element: HTMLElement): BrowserWritableField["kind"] {
  if (element instanceof HTMLTextAreaElement) {
    return "textarea";
  }
  if (element instanceof HTMLInputElement) {
    return "text_input";
  }
  return "contenteditable";
}

function fieldValue(element: HTMLElement): string {
  if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement) {
    return element.value;
  }
  return element.innerText || element.textContent || "";
}

function setFieldValue(element: HTMLElement, value: string): void {
  if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement) {
    element.focus();
    element.value = value;
  } else {
    element.focus();
    element.textContent = value;
  }
  element.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertText", data: value }));
  element.dispatchEvent(new Event("change", { bubbles: true }));
}

function fieldLabel(element: HTMLElement): string {
  const ariaLabel = normalizeText(element.getAttribute("aria-label") ?? "");
  if (ariaLabel) {
    return ariaLabel;
  }

  const labelledBy = element.getAttribute("aria-labelledby");
  if (labelledBy) {
    const label = labelledBy
      .split(/\s+/)
      .map((id) => normalizeText(document.getElementById(id)?.textContent ?? ""))
      .filter(Boolean)
      .join(" ");
    if (label) {
      return label;
    }
  }

  if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement) {
    if (element.id) {
      const label = normalizeText(document.querySelector<HTMLLabelElement>(`label[for="${cssEscape(element.id)}"]`)?.innerText ?? "");
      if (label) {
        return label;
      }
    }
    const wrappedLabel = normalizeText(element.closest("label")?.textContent ?? "");
    if (wrappedLabel) {
      return wrappedLabel;
    }
    if (element.placeholder) {
      return element.placeholder;
    }
    if (element.name) {
      return element.name.replace(/[-_]+/g, " ");
    }
  }

  return "Visible text field";
}

function showInlineDraftUi(record: FieldRecord, draft: string): void {
  removeInlineDraftUi(record.handle);
  const host = document.createElement("div");
  host.setAttribute("data-deyana-inline-draft", record.handle);
  host.style.position = "fixed";
  host.style.zIndex = "2147483647";
  const bounds = record.element.getBoundingClientRect();
  host.style.left = `${Math.min(Math.max(8, bounds.left), window.innerWidth - 272)}px`;
  host.style.top = `${Math.min(Math.max(8, bounds.bottom + 8), window.innerHeight - 132)}px`;

  const shadow = host.attachShadow({ mode: "closed" });
  const style = document.createElement("style");
  style.textContent = `
    :host { all: initial; }
    .card {
      width: 260px;
      box-sizing: border-box;
      display: grid;
      gap: 8px;
      padding: 10px;
      border: 1px solid rgba(131,169,212,.35);
      border-radius: 10px;
      background: rgba(9,19,33,.96);
      color: #f3f1ec;
      box-shadow: 0 16px 42px rgba(0,0,0,.45);
      font: 12px/1.35 Inter, ui-sans-serif, system-ui, sans-serif;
    }
    strong { font-size: 12px; }
    p { max-height: 72px; margin: 0; overflow: auto; color: #a8b2c0; white-space: pre-wrap; }
    .actions { display: flex; gap: 6px; justify-content: flex-end; }
    button {
      border: 1px solid rgba(208,221,230,.18);
      border-radius: 7px;
      padding: 5px 8px;
      background: rgba(255,255,255,.07);
      color: #f3f1ec;
      font: inherit;
      cursor: pointer;
    }
  `;
  const card = document.createElement("div");
  card.className = "card";
  const title = document.createElement("strong");
  title.textContent = "Deyana draft inserted";
  const preview = document.createElement("p");
  preview.textContent = draft;
  const actions = document.createElement("div");
  actions.className = "actions";
  const restore = document.createElement("button");
  restore.type = "button";
  restore.textContent = "Restore";
  restore.addEventListener("click", () => {
    genericPageAdapter.clearField({
      pageSessionId: record.pageSessionId,
      fieldHandle: record.handle,
      restoreOriginal: true,
      userApproved: true
    });
  });
  const clear = document.createElement("button");
  clear.type = "button";
  clear.textContent = "Clear";
  clear.addEventListener("click", () => {
    genericPageAdapter.clearField({
      pageSessionId: record.pageSessionId,
      fieldHandle: record.handle,
      restoreOriginal: false,
      userApproved: true
    });
  });
  actions.append(restore, clear);
  card.append(title, preview, actions);
  shadow.append(style, card);
  document.documentElement.append(host);
}

function removeInlineDraftUi(handle: string): void {
  document.querySelector(`[data-deyana-inline-draft="${cssEscape(handle)}"]`)?.remove();
}

function collectSemanticText(root: HTMLElement, limit: number): string {
  const blocks: string[] = [];
  const seen = new Set<string>();
  let characterCount = 0;
  const elements = root.matches(TEXT_SELECTORS)
    ? [root, ...root.querySelectorAll<HTMLElement>(TEXT_SELECTORS)]
    : [...root.querySelectorAll<HTMLElement>(TEXT_SELECTORS)];

  for (const element of elements) {
    if (!isVisible(element) || isSensitive(element)) {
      continue;
    }
    const text = normalizeText(element.innerText || element.textContent || "");
    if (!text || seen.has(text)) {
      continue;
    }
    seen.add(text);
    blocks.push(text);
    characterCount += text.length + (blocks.length > 1 ? 1 : 0);
    if (characterCount >= limit) {
      break;
    }
  }

  if (!blocks.length && isVisible(root) && !isSensitive(root)) {
    return normalizeText(root.innerText || root.textContent || "").slice(0, limit);
  }
  return blocks.join("\n").slice(0, limit);
}

function collectLandmarks(): BrowserLandmark[] {
  const landmarks: BrowserLandmark[] = [];
  let characterCount = 0;
  const candidates = document.querySelectorAll<HTMLElement>(
    "main, article, nav, aside, header, footer, [role='main'], [role='article'], [role='navigation'], [role='complementary']"
  );
  for (const element of candidates) {
    if (!isVisible(element) || isSensitive(element)) {
      continue;
    }
    const role = element.getAttribute("role") || element.tagName.toLowerCase();
    const name = normalizeText(
      element.getAttribute("aria-label") ||
        element.getAttribute("aria-labelledby") ||
        element.querySelector("h1, h2, h3")?.textContent ||
        ""
    );
    const text = collectSemanticText(element, MAX_SINGLE_LANDMARK_CHARACTERS);
    if (text) {
      landmarks.push({ role, name, text });
      characterCount += role.length + name.length + text.length;
    }
    if (landmarks.length >= MAX_LANDMARKS || characterCount >= MAX_LANDMARK_CHARACTERS) {
      break;
    }
  }
  return landmarks;
}

function isVisible(element: HTMLElement): boolean {
  if (element.hidden || element.getAttribute("aria-hidden") === "true") {
    return false;
  }
  const style = window.getComputedStyle(element);
  if (style.display === "none" || style.visibility === "hidden" || Number(style.opacity) === 0) {
    return false;
  }
  const bounds = element.getBoundingClientRect();
  return bounds.width > 0 && bounds.height > 0;
}

function isSensitive(element: HTMLElement): boolean {
  if (
    element.closest(
      "input[type='password'], input[type='hidden'], input[type='file'], input[type='number'], input[autocomplete*='one-time-code'], [data-deyana-private]"
    )
  ) {
    return true;
  }
  const autocomplete = element.getAttribute("autocomplete")?.toLowerCase() ?? "";
  const name = `${element.getAttribute("name") ?? ""} ${element.getAttribute("id") ?? ""}`.toLowerCase();
  return (
    autocomplete.includes("password") ||
    autocomplete.includes("one-time-code") ||
    autocomplete.includes("cc-") ||
    autocomplete.includes("transaction") ||
    /password|passwd|otp|2fa|mfa|card|cvv|cvc|iban|routing|account/.test(name)
  );
}

function normalizeText(value: string): string {
  return value.replace(/\s+/g, " ").trim();
}

function sanitizedPageUrl(url: URL): string {
  return `${url.origin}${url.pathname}`;
}

function cssEscape(value: string): string {
  return typeof CSS !== "undefined" && CSS.escape ? CSS.escape(value) : value.replace(/["\\]/g, "\\$&");
}
