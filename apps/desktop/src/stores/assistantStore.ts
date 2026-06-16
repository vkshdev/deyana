import {
  DEFAULT_PHASE1_SETTINGS,
  type AssistantState,
  type ConnectorPreview,
  type MemoryPreviewItem,
  type ModelStatus,
  type Phase1Settings,
  type QuickAction,
  type SyncStatus,
  type UiMode
} from "@deyana/schemas";
import { useSyncExternalStore } from "react";
import { tauriClient } from "../services/tauriClient";

export interface AssistantSnapshot {
  assistantState: AssistantState;
  settings: Phase1Settings;
  modelStatus: ModelStatus;
  syncStatus: SyncStatus;
  connectors: ConnectorPreview[];
  memoryPreview: MemoryPreviewItem[];
  quickActions: QuickAction[];
  error?: string;
}

const initialSnapshot: AssistantSnapshot = {
  assistantState: "COMPACT_FLOATING",
  settings: DEFAULT_PHASE1_SETTINGS,
  modelStatus: "available",
  syncStatus: "idle",
  connectors: [
    {
      id: "gmail",
      name: "Gmail",
      status: "not_connected",
      lastSyncLabel: "Local sync off"
    },
    {
      id: "calendar",
      name: "Calendar",
      status: "not_connected",
      lastSyncLabel: "Local sync off"
    },
    {
      id: "github",
      name: "GitHub",
      status: "not_connected",
      lastSyncLabel: "Local sync off"
    }
  ],
  memoryPreview: [
    {
      id: "vault",
      title: "Vault setup waits for Phase 3",
      source: "Local memory",
      updatedLabel: "Ready"
    },
    {
      id: "model",
      title: "Low-spec model profile selected",
      source: "qwen3:1.7b",
      updatedLabel: "Local"
    }
  ],
  quickActions: [
    {
      id: "memory",
      label: "Memory",
      state: "RETRIEVING_MEMORY"
    },
    {
      id: "search",
      label: "Search",
      state: "SEARCHING_WEB"
    },
    {
      id: "code",
      label: "Code",
      state: "CODING"
    }
  ]
};

type Listener = () => void;

class AssistantStore {
  private listeners = new Set<Listener>();
  private snapshot = initialSnapshot;

  getSnapshot = () => this.snapshot;

  subscribe = (listener: Listener) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  hydrate = async () => {
    try {
      const settings = await tauriClient.getPhase1Settings();
      this.setSnapshot({
        settings,
        assistantState: settings.uiMode === "expanded" ? "EXPANDED_PANEL" : "COMPACT_FLOATING",
        error: undefined
      });
    } catch (error) {
      this.setSnapshot({
        error: error instanceof Error ? error.message : "Unable to load local settings"
      });
    }
  };

  setAssistantState = (assistantState: AssistantState) => {
    this.setSnapshot({ assistantState });

    window.setTimeout(() => {
      const current = this.snapshot.settings.uiMode;
      this.setSnapshot({
        assistantState: current === "expanded" ? "EXPANDED_PANEL" : "COMPACT_FLOATING"
      });
    }, 1400);
  };

  setFloatingMode = async (uiMode: UiMode) => {
    const optimisticSettings = { ...this.snapshot.settings, uiMode };
    this.setSnapshot({
      settings: optimisticSettings,
      assistantState: uiMode === "expanded" ? "EXPANDED_PANEL" : "COMPACT_FLOATING",
      error: undefined
    });

    try {
      const settings = await tauriClient.setFloatingMode(uiMode);
      this.setSnapshot({ settings });
    } catch (error) {
      this.setSnapshot({
        error: error instanceof Error ? error.message : "Unable to resize floating window"
      });
    }
  };

  setAlwaysOnTop = async (alwaysOnTop: boolean) => {
    this.setSnapshot({
      settings: { ...this.snapshot.settings, alwaysOnTop },
      error: undefined
    });

    try {
      const settings = await tauriClient.setAlwaysOnTop(alwaysOnTop);
      this.setSnapshot({ settings });
    } catch (error) {
      this.setSnapshot({
        error: error instanceof Error ? error.message : "Unable to update window preference"
      });
    }
  };

  hideWindow = async () => {
    await tauriClient.hideMainWindow();
  };

  private setSnapshot = (patch: Partial<AssistantSnapshot>) => {
    this.snapshot = { ...this.snapshot, ...patch };
    this.listeners.forEach((listener) => listener());
  };
}

export const assistantStore = new AssistantStore();

export const useAssistantSnapshot = () =>
  useSyncExternalStore(assistantStore.subscribe, assistantStore.getSnapshot);
