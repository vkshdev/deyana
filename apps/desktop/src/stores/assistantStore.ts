import {
  DEFAULT_BACKEND_PROCESS_STATUS,
  DEFAULT_CORE_APP_SETTINGS,
  DEFAULT_DESKTOP_SETTINGS,
  DEFAULT_ONBOARDING_STATE,
  type AssistantState,
  type AppCoreEvent,
  type BackendProcessStatus,
  type BackendStatusResponse,
  type ConnectorPreview,
  type CoreAppSettings,
  type MemoryCreateRequest,
  type MemoryItem,
  type ModelProfile,
  type MemoryPreviewItem,
  type ModelStatus,
  type OnboardingState,
  type OnboardingStep,
  type PrivacyMode,
  type DesktopSettings,
  type QuickAction,
  type SyncStatus,
  type UiMode
} from "@deyana/schemas";
import { useSyncExternalStore } from "react";
import { backendClient, type BackendEventConnection } from "../services/backendClient";
import { tauriClient } from "../services/tauriClient";

export interface AssistantSnapshot {
  assistantState: AssistantState;
  settings: DesktopSettings;
  coreSettings: CoreAppSettings;
  onboarding: OnboardingState;
  onboardingStep: OnboardingStep;
  onboardingVaultPath: string;
  onboardingBusy: boolean;
  modelStatus: ModelStatus;
  syncStatus: SyncStatus;
  backend: BackendProcessStatus;
  backendStatus?: BackendStatusResponse;
  backendEventStreamConnected: boolean;
  lastBackendEventType?: string;
  connectors: ConnectorPreview[];
  memoryPreview: MemoryPreviewItem[];
  memoryItems: MemoryItem[];
  memoryQuery: string;
  memoryDraft: {
    title: string;
    summary: string;
    contentMarkdown: string;
  };
  memoryBusy: boolean;
  memoryExportedAt?: string;
  quickActions: QuickAction[];
  error?: string;
}

const initialSnapshot: AssistantSnapshot = {
  assistantState: "COMPACT_FLOATING",
  settings: DEFAULT_DESKTOP_SETTINGS,
  coreSettings: DEFAULT_CORE_APP_SETTINGS,
  onboarding: DEFAULT_ONBOARDING_STATE,
  onboardingStep: "welcome",
  onboardingVaultPath: "",
  onboardingBusy: false,
  modelStatus: "checking",
  syncStatus: "idle",
  backend: DEFAULT_BACKEND_PROCESS_STATUS,
  backendEventStreamConnected: false,
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
  memoryItems: [],
  memoryQuery: "",
  memoryDraft: {
    title: "",
    summary: "",
    contentMarkdown: ""
  },
  memoryBusy: false,
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
  private coreStatusUnlisten?: () => void;
  private backendConnection?: BackendEventConnection;
  private backendReconnectTimer?: number;
  private intentionalBackendDisconnect = false;

  getSnapshot = () => this.snapshot;

  subscribe = (listener: Listener) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  hydrate = async () => {
    try {
      const [settings, backend] = await Promise.all([
        tauriClient.getDesktopSettings(),
        tauriClient.getCoreStatus()
      ]);
      this.setSnapshot({
        settings,
        backend,
        assistantState: settings.uiMode === "expanded" ? "EXPANDED_PANEL" : "COMPACT_FLOATING",
        error: undefined
      });
      await this.subscribeToCoreStatus();
      await this.refreshBackendStatus();
      this.connectBackendEvents();
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

  restartBackend = async () => {
    this.disconnectBackendEvents();
    this.setSnapshot({
      backend: {
        ...this.snapshot.backend,
        lifecycle: "starting",
        updatedAtMs: Date.now(),
        lastError: undefined
      },
      backendEventStreamConnected: false,
      error: undefined
    });

    try {
      const backend = await tauriClient.restartCore();
      this.setSnapshot({ backend });
      this.scheduleBackendReconnect(700);
    } catch (error) {
      this.setSnapshot({
        backend: {
          ...this.snapshot.backend,
          lifecycle: "unavailable",
          updatedAtMs: Date.now(),
          lastError: error instanceof Error ? error.message : "Unable to restart backend"
        },
        error: error instanceof Error ? error.message : "Unable to restart backend"
      });
    }
  };

  setOnboardingStep = (onboardingStep: OnboardingStep) => {
    this.setSnapshot({ onboardingStep, assistantState: "ONBOARDING" });
  };

  setOnboardingPrivacyMode = (privacyMode: PrivacyMode) => {
    this.setSnapshot({
      onboarding: {
        ...this.snapshot.onboarding,
        selectedPrivacyMode: privacyMode
      }
    });
  };

  setOnboardingModelProfile = (modelProfile: ModelProfile) => {
    this.setSnapshot({
      onboarding: {
        ...this.snapshot.onboarding,
        selectedModelProfile: modelProfile
      }
    });
  };

  setOnboardingVaultPath = (onboardingVaultPath: string) => {
    this.setSnapshot({ onboardingVaultPath });
  };

  chooseVaultFolder = async () => {
    const selected = await tauriClient.chooseVaultFolder();
    if (selected) {
      this.setOnboardingVaultPath(selected);
    }
  };

  completeOnboarding = async () => {
    const vaultPath = this.snapshot.onboardingVaultPath.trim() || this.snapshot.onboarding.selectedVaultPath;

    if (!vaultPath) {
      this.setSnapshot({ error: "Choose a local vault folder before continuing." });
      return;
    }

    this.setSnapshot({ onboardingBusy: true, error: undefined });

    try {
      await backendClient.selectVault({ path: vaultPath });
      const result = await backendClient.completeOnboarding({
        privacyMode: this.snapshot.onboarding.selectedPrivacyMode,
        modelProfile: this.snapshot.onboarding.selectedModelProfile,
        vaultPath
      });
      this.setSnapshot({
        onboarding: result.state,
        coreSettings: result.settings,
        onboardingStep: "complete",
        onboardingVaultPath: result.state.selectedVaultPath ?? vaultPath,
        onboardingBusy: false,
        assistantState: this.snapshot.settings.uiMode === "expanded" ? "EXPANDED_PANEL" : "COMPACT_FLOATING",
        memoryPreview: [
          {
            id: "vault",
            title: "Vault created",
            source: result.state.selectedVaultPath ?? vaultPath,
            updatedLabel: "Local"
          },
          ...this.snapshot.memoryPreview.filter((item) => item.id !== "vault")
        ]
      });
    } catch (error) {
      this.setSnapshot({
        onboardingBusy: false,
        error: error instanceof Error ? error.message : "Unable to complete onboarding"
      });
    }
  };

  setMemoryQuery = (memoryQuery: string) => {
    this.setSnapshot({ memoryQuery });
  };

  setMemoryDraft = (patch: Partial<AssistantSnapshot["memoryDraft"]>) => {
    this.setSnapshot({ memoryDraft: { ...this.snapshot.memoryDraft, ...patch } });
  };

  loadMemory = async (query = this.snapshot.memoryQuery) => {
    try {
      const response = await backendClient.listMemory(query);
      this.setSnapshot({
        memoryItems: response.items,
        memoryPreview: response.items.length
          ? response.items.slice(0, 2).map((item) => ({
              id: item.id,
              title: item.title,
              source: item.markdownPath ?? item.sourceType,
              updatedLabel: "Local"
            }))
          : this.snapshot.memoryPreview,
        error: undefined
      });
    } catch (error) {
      this.setSnapshot({
        error: error instanceof Error ? error.message : "Unable to load memory"
      });
    }
  };

  createMemory = async () => {
    const draft = this.snapshot.memoryDraft;
    const title = draft.title.trim();
    const summary = draft.summary.trim();

    if (!title || !summary) {
      this.setSnapshot({ error: "Memory needs a title and summary." });
      return;
    }

    this.setSnapshot({ memoryBusy: true, error: undefined });

    const request: MemoryCreateRequest = {
      type: "note",
      title,
      summary,
      contentMarkdown: draft.contentMarkdown.trim() || summary,
      sourceType: "manual",
      tags: ["manual"]
    };

    try {
      await backendClient.createMemory(request);
      this.setSnapshot({
        memoryDraft: { title: "", summary: "", contentMarkdown: "" },
        memoryBusy: false
      });
      await this.loadMemory();
    } catch (error) {
      this.setSnapshot({
        memoryBusy: false,
        error: error instanceof Error ? error.message : "Unable to create memory"
      });
    }
  };

  deleteMemory = async (id: string) => {
    this.setSnapshot({ memoryBusy: true, error: undefined });
    try {
      await backendClient.deleteMemory(id);
      this.setSnapshot({
        memoryItems: this.snapshot.memoryItems.filter((item) => item.id !== id),
        memoryBusy: false
      });
      await this.loadMemory();
    } catch (error) {
      this.setSnapshot({
        memoryBusy: false,
        error: error instanceof Error ? error.message : "Unable to delete memory"
      });
    }
  };

  reindexMemory = async () => {
    this.setSnapshot({ memoryBusy: true, error: undefined });
    try {
      await backendClient.reindexMemory();
      this.setSnapshot({ memoryBusy: false });
      await this.loadMemory();
    } catch (error) {
      this.setSnapshot({
        memoryBusy: false,
        error: error instanceof Error ? error.message : "Unable to reindex memory"
      });
    }
  };

  exportMemory = async () => {
    this.setSnapshot({ memoryBusy: true, error: undefined });
    try {
      const exported = await backendClient.exportMemory();
      this.setSnapshot({
        memoryBusy: false,
        memoryExportedAt: exported.exportedAt
      });
    } catch (error) {
      this.setSnapshot({
        memoryBusy: false,
        error: error instanceof Error ? error.message : "Unable to export memory"
      });
    }
  };

  openVault = async () => {
    const vaultPath = this.snapshot.coreSettings.vaultPath;
    if (!vaultPath) {
      this.setSnapshot({ error: "Choose a vault before opening it." });
      return;
    }

    try {
      await tauriClient.openVaultFolder(vaultPath);
    } catch (error) {
      this.setSnapshot({
        error: error instanceof Error ? error.message : "Unable to open vault"
      });
    }
  };

  private subscribeToCoreStatus = async () => {
    if (this.coreStatusUnlisten) {
      return;
    }

    this.coreStatusUnlisten = await tauriClient.onCoreStatus((backend) => {
      this.setSnapshot({
        backend,
        error: backend.lifecycle === "crashed" ? backend.lastError ?? "Backend core crashed" : undefined
      });

      if (backend.lifecycle === "running") {
        this.scheduleBackendReconnect(200);
      }
    });
  };

  private refreshBackendStatus = async () => {
    try {
      const [backendStatus, coreSettings, onboarding] = await Promise.all([
        backendClient.getStatus(),
        backendClient.getSettings(),
        backendClient.getOnboardingState()
      ]);
      const onboardingVaultPath = onboarding.selectedVaultPath ?? coreSettings.vaultPath ?? "";
      const memoryPreview = onboarding.completed && onboardingVaultPath
        ? [
            {
              id: "vault",
              title: "Vault ready",
              source: onboardingVaultPath,
              updatedLabel: "Local"
            },
            ...this.snapshot.memoryPreview.filter((item) => item.id !== "vault")
          ]
        : this.snapshot.memoryPreview;
      this.setSnapshot({
        backendStatus,
        coreSettings,
        onboarding,
        onboardingStep: onboarding.completed ? "complete" : onboarding.currentStep,
        onboardingVaultPath,
        memoryPreview,
        backend: {
          ...this.snapshot.backend,
          lifecycle: "running",
          updatedAtMs: Date.now(),
          lastError: undefined
        },
        assistantState: onboarding.completed
          ? this.snapshot.settings.uiMode === "expanded"
            ? "EXPANDED_PANEL"
            : "COMPACT_FLOATING"
          : "ONBOARDING",
        modelStatus: backendStatus.featureFlags.models ? "available" : "checking",
        error: undefined
      });

      if (!onboarding.completed && this.snapshot.settings.uiMode !== "expanded") {
        void this.setFloatingMode("expanded");
      }
      if (onboarding.completed) {
        void this.loadMemory();
      }
    } catch {
      if (this.snapshot.backend.lifecycle === "running") {
        this.setSnapshot({
          backend: {
            ...this.snapshot.backend,
            lifecycle: "starting",
            updatedAtMs: Date.now()
          }
        });
      }
      this.scheduleBackendReconnect(900);
    }
  };

  private connectBackendEvents = () => {
    this.disconnectBackendEvents();

    try {
      this.backendConnection = backendClient.connectEvents(
        (event) => this.handleBackendEvent(event),
        (reason) => this.handleBackendClose(reason)
      );
    } catch {
      this.scheduleBackendReconnect(1200);
    }
  };

  private handleBackendEvent = (event: AppCoreEvent) => {
    if (event.type === "app.ready") {
      this.setSnapshot({
        backend: {
          ...this.snapshot.backend,
          lifecycle: "running",
          updatedAtMs: Date.now(),
          lastError: undefined
        },
        backendEventStreamConnected: true,
        lastBackendEventType: event.type,
        error: undefined
      });
      void this.refreshBackendStatus();
      return;
    }

    if (event.type === "backend.heartbeat") {
      this.setSnapshot({
        backend: {
          ...this.snapshot.backend,
          lifecycle: event.payload.lifecycle,
          updatedAtMs: Date.now(),
          lastError: undefined
        },
        backendEventStreamConnected: true,
        lastBackendEventType: event.type,
        error: undefined
      });
      return;
    }

    if (event.type === "backend.lifecycle.changed") {
      this.setSnapshot({
        backend: {
          ...this.snapshot.backend,
          lifecycle: event.payload.lifecycle,
          updatedAtMs: Date.now(),
          lastError: undefined
        },
        backendEventStreamConnected: true,
        lastBackendEventType: event.type
      });
      return;
    }

    if (event.type === "settings.changed") {
      this.setSnapshot({
        coreSettings: event.payload.settings,
        lastBackendEventType: event.type
      });
      return;
    }

    if (event.type === "vault.selected") {
      this.setSnapshot({
        coreSettings: event.payload.settings,
        onboarding: event.payload.state,
        onboardingVaultPath: event.payload.vaultPath,
        lastBackendEventType: event.type
      });
      return;
    }

    if (event.type === "onboarding.state.changed") {
      this.setSnapshot({
        coreSettings: event.payload.settings,
        onboarding: event.payload.state,
        onboardingStep: event.payload.state.currentStep,
        onboardingVaultPath: event.payload.state.selectedVaultPath ?? "",
        lastBackendEventType: event.type
      });
      return;
    }

    if (event.type === "memory.item.created" || event.type === "memory.item.updated") {
      const item = event.payload.item;
      const existing = this.snapshot.memoryItems.filter((memory) => memory.id !== item.id);
      this.setSnapshot({
        memoryItems: [item, ...existing],
        lastBackendEventType: event.type
      });
      return;
    }

    if (event.type === "memory.item.deleted") {
      this.setSnapshot({
        memoryItems: this.snapshot.memoryItems.filter((item) => item.id !== event.payload.id),
        lastBackendEventType: event.type
      });
      return;
    }

    if (event.type === "memory.reindexed") {
      this.setSnapshot({ lastBackendEventType: event.type });
      void this.loadMemory();
      return;
    }

    const _exhaustive: never = event;
    return _exhaustive;
  };

  private handleBackendClose = (reason: string) => {
    if (this.intentionalBackendDisconnect) {
      this.intentionalBackendDisconnect = false;
      return;
    }

    if (this.snapshot.backend.lifecycle === "stopping" || this.snapshot.backend.lifecycle === "stopped") {
      return;
    }

    this.setSnapshot({
      backendEventStreamConnected: false,
      backend: {
        ...this.snapshot.backend,
        lifecycle: this.snapshot.backend.lifecycle === "crashed" ? "crashed" : "unavailable",
        updatedAtMs: Date.now(),
        lastError: reason
      }
    });
    this.scheduleBackendReconnect(1400);
  };

  private scheduleBackendReconnect = (delayMs: number) => {
    if (this.backendReconnectTimer) {
      window.clearTimeout(this.backendReconnectTimer);
    }

    this.backendReconnectTimer = window.setTimeout(() => {
      void this.refreshBackendStatus();
      this.connectBackendEvents();
    }, delayMs);
  };

  private disconnectBackendEvents = () => {
    if (this.backendConnection) {
      this.intentionalBackendDisconnect = true;
      this.backendConnection.disconnect();
      this.backendConnection = undefined;
    }
  };

  private setSnapshot = (patch: Partial<AssistantSnapshot>) => {
    this.snapshot = { ...this.snapshot, ...patch };
    this.listeners.forEach((listener) => listener());
  };
}

export const assistantStore = new AssistantStore();

export const useAssistantSnapshot = () =>
  useSyncExternalStore(assistantStore.subscribe, assistantStore.getSnapshot);
