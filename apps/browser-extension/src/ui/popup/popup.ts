import type { BrowserContextMode } from "@deyana/schemas";
import type { CapturePageResult, PopupState } from "../../protocol/messages";

const statusElement = requireElement<HTMLParagraphElement>("status");
const modeElement = requireElement<HTMLSelectElement>("mode");
const captureButton = requireElement<HTMLButtonElement>("capture");
const reconnectButton = requireElement<HTMLButtonElement>("reconnect");

void refreshState();

captureButton.addEventListener("click", () => {
  void runCapture();
});

reconnectButton.addEventListener("click", () => {
  void reconnect();
});

async function refreshState(): Promise<void> {
  const state = await chrome.runtime.sendMessage<PopupState>({ type: "popup.state" });
  renderState(state);
}

async function reconnect(): Promise<void> {
  setBusy(true, "Reconnecting to local Deyana...");
  const state = await chrome.runtime.sendMessage<PopupState>({ type: "popup.reconnect" });
  renderState(state);
  setBusy(false);
}

async function runCapture(): Promise<void> {
  setBusy(true, "Reading bounded semantic page context...");
  const result = await chrome.runtime.sendMessage<CapturePageResult>({
    type: "popup.capture",
    mode: modeElement.value as BrowserContextMode
  });
  if (result.ok && result.context) {
    statusElement.textContent = `Shared ${result.context.characterCount.toLocaleString()} characters from ${result.context.title}.`;
  } else {
    statusElement.textContent = result.instruction ?? "Unable to share this page.";
  }
  setBusy(false);
}

function renderState(state: PopupState): void {
  statusElement.textContent = state.instruction;
  reconnectButton.hidden = state.connected;
}

function setBusy(busy: boolean, message?: string): void {
  captureButton.disabled = busy;
  reconnectButton.disabled = busy;
  modeElement.disabled = busy;
  if (message) {
    statusElement.textContent = message;
  }
}

function requireElement<T extends HTMLElement>(id: string): T {
  const element = document.getElementById(id);
  if (!element) {
    throw new Error(`Missing popup element: ${id}`);
  }
  return element as T;
}
