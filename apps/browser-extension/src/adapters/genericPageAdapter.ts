import type {
  BrowserContextMode,
  BrowserLandmark,
  BrowserPageContext
} from "@deyana/schemas";
import type { BrowserSiteAdapter } from "./BrowserSiteAdapter";

const MAX_CONTEXT_CHARACTERS = 80 * 1024;
const MAX_LANDMARK_CHARACTERS = 12 * 1024;
const MAX_SINGLE_LANDMARK_CHARACTERS = 1500;
const MAX_LANDMARKS = 12;
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
        }
      ],
      characterCount: boundedSource.length,
      truncated
    };
  }
}

let currentPageSessionId = crypto.randomUUID();

export const genericPageAdapter = new GenericPageAdapter();

export function resetPageSession(): void {
  currentPageSessionId = crypto.randomUUID();
}

function pageSessionId(): string {
  return `page_session_${currentPageSessionId.replaceAll("-", "")}`;
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
  if (element.closest("input[type='password'], input[autocomplete*='one-time-code'], [data-deyana-private]")) {
    return true;
  }
  const autocomplete = element.getAttribute("autocomplete")?.toLowerCase() ?? "";
  return autocomplete.includes("password") || autocomplete.includes("cc-");
}

function normalizeText(value: string): string {
  return value.replace(/\s+/g, " ").trim();
}

function sanitizedPageUrl(url: URL): string {
  return `${url.origin}${url.pathname}`;
}
