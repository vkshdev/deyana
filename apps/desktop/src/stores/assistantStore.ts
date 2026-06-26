import {
  DEFAULT_BACKEND_PROCESS_STATUS,
  DEFAULT_CORE_APP_SETTINGS,
  DEFAULT_DESKTOP_SETTINGS,
  DEFAULT_ONBOARDING_STATE,
  type AssistantState,
  type AppCoreEvent,
  type BackendProcessStatus,
  type BackendStatusResponse,
  type ChatMessageItem,
  type ChatMessageResponse,
  type ConnectorItem,
  type ConnectorSyncRun,
  type CoreAppSettings,
  type LocalModelStatusResponse,
  type MemoryCreateRequest,
  type MemoryEntity,
  type MemoryInsight,
  type MemoryItem,
  type ModelSelectionRequest,
  type ModelProfile,
  type ModelTestResponse,
  type MemoryPreviewItem,
  type ModelStatus,
  type OnboardingState,
  type OnboardingStep,
  type PrivacyAuditEvent,
  type PrivacyMode,
  type PrivacyStatusResponse,
  type ConnectorHealthResponse,
  type CrashRecoveryResponse,
  type DesktopSettings,
  type PerformanceProfileResponse,
  type QuickAction,
  type ReleaseLogListResponse,
  type ReleaseLogReadResponse,
  type ReleasePrivacyExportResponse,
  type ReleaseReadinessResponse,
  type ReleaseUpdatePlanResponse,
  type SyncStatus,
  type ToolId,
  type ToolRunResponse,
  type UiMode,
  type VoiceSettings,
  type VoiceSettingsPatch,
  type VoiceStatusResponse,
  type VoiceTranscriptResponse
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
  connectors: ConnectorItem[];
  connectorSyncRuns: ConnectorSyncRun[];
  connectorBusy: Record<string, boolean>;
  connectorOAuth: Record<string, { state: string; authorizationUrl: string; expiresAt: string }>;
  connectorOAuthCodes: Record<string, string>;
  memoryPreview: MemoryPreviewItem[];
  memoryItems: MemoryItem[];
  memoryEntities: MemoryEntity[];
  memoryActionItems: MemoryInsight[];
  memoryDecisions: MemoryInsight[];
  memoryExtractionView: MemoryExtractionView;
  memoryQuery: string;
  memoryProjectDraft: string;
  memoryDraft: {
    title: string;
    summary: string;
    contentMarkdown: string;
  };
  memoryBusy: boolean;
  memoryExportedAt?: string;
  modelStatusDetail?: LocalModelStatusResponse;
  modelTestBusy: boolean;
  modelTestResponse?: ModelTestResponse;
  chatMessages: ChatMessageItem[];
  chatDraft: string;
  chatBusy: boolean;
  privacyStatus?: PrivacyStatusResponse;
  privacyAuditEvents: PrivacyAuditEvent[];
  privacyBusy: boolean;
  toolActive: ToolId;
  toolInput: string;
  toolApproved: boolean;
  toolBusy: boolean;
  toolResult?: ToolRunResponse;
  voiceSettings?: VoiceSettings;
  voiceStatus?: VoiceStatusResponse;
  voiceTranscript?: VoiceTranscriptResponse;
  voiceBusy: boolean;
  releaseReadiness?: ReleaseReadinessResponse;
  releaseUpdatePlan?: ReleaseUpdatePlanResponse;
  releaseLogs?: ReleaseLogListResponse;
  releaseSelectedLog?: ReleaseLogReadResponse;
  releasePrivacyExport?: ReleasePrivacyExportResponse;
  releaseConnectorHealth?: ConnectorHealthResponse;
  releasePerformance?: PerformanceProfileResponse;
  releaseCrashRecovery?: CrashRecoveryResponse;
  releaseDeletePhrase: string;
  releaseDeleteIncludeVault: boolean;
  releaseBusy: boolean;
  quickActions: QuickAction[];
  error?: string;
}

export type MemoryExtractionView = "items" | "actions" | "decisions" | "entities";

const defaultConnectors = (): ConnectorItem[] =>
  [
    ["gmail", "Gmail"],
    ["calendar", "Calendar"],
    ["github", "GitHub"],
    ["drive", "Google Drive"],
    ["slack", "Slack"],
    ["notion", "Notion"],
    ["jira", "Jira"],
    ["linear", "Linear"],
    ["stripe", "Stripe"]
  ].map(([id, name]) => ({
    id,
    name,
    status: "not_connected",
    enabled: false,
    scopes: [],
    oauthConfigured: false,
    realSyncSupported: true,
    syncIntervalMinutes: 360,
    lastSyncAt: null,
    nextSyncAt: null,
    tokenStored: false,
    tokenUpdatedAt: null,
    lastError: null,
    updatedAt: ""
  }));

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
  connectors: defaultConnectors(),
  connectorSyncRuns: [],
  connectorBusy: {},
  connectorOAuth: {},
  connectorOAuthCodes: {},
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
  memoryEntities: [],
  memoryActionItems: [],
  memoryDecisions: [],
  memoryExtractionView: "items",
  memoryQuery: "",
  memoryProjectDraft: "",
  memoryDraft: {
    title: "",
    summary: "",
    contentMarkdown: ""
  },
  memoryBusy: false,
  modelTestBusy: false,
  chatMessages: [],
  chatDraft: "",
  chatBusy: false,
  privacyAuditEvents: [],
  privacyBusy: false,
  toolActive: "web_search",
  toolInput: "",
  toolApproved: false,
  toolBusy: false,
  voiceBusy: false,
  releaseDeletePhrase: "",
  releaseDeleteIncludeVault: false,
  releaseBusy: false,
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

  setLowPowerMode = async (lowPowerMode: boolean) => {
    this.setSnapshot({
      settings: { ...this.snapshot.settings, lowPowerMode },
      error: undefined
    });

    try {
      const settings = await tauriClient.setLowPowerMode(lowPowerMode);
      this.setSnapshot({ settings });
    } catch (error) {
      this.setSnapshot({
        error: error instanceof Error ? error.message : "Unable to update low-power mode"
      });
    }
  };

  setReduceMotion = async (reduceMotion: boolean) => {
    this.setSnapshot({
      settings: { ...this.snapshot.settings, reduceMotion },
      error: undefined
    });

    try {
      const settings = await tauriClient.setReduceMotion(reduceMotion);
      this.setSnapshot({ settings });
    } catch (error) {
      this.setSnapshot({
        error: error instanceof Error ? error.message : "Unable to update motion preference"
      });
    }
  };

  dockFloatingWindow = async (edge: "left" | "right") => {
    try {
      const settings = await tauriClient.dockFloatingWindow(edge);
      this.setSnapshot({ settings, error: undefined });
    } catch (error) {
      this.setSnapshot({
        error: error instanceof Error ? error.message : "Unable to dock floating window"
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

  setMemoryExtractionView = (memoryExtractionView: MemoryExtractionView) => {
    this.setSnapshot({ memoryExtractionView });
  };

  setMemoryProjectDraft = (memoryProjectDraft: string) => {
    this.setSnapshot({ memoryProjectDraft });
  };

  setMemoryDraft = (patch: Partial<AssistantSnapshot["memoryDraft"]>) => {
    this.setSnapshot({ memoryDraft: { ...this.snapshot.memoryDraft, ...patch } });
  };

  setToolActive = (toolActive: ToolId) => {
    this.setSnapshot({ toolActive, toolResult: undefined, error: undefined });
  };

  setToolInput = (toolInput: string) => {
    this.setSnapshot({ toolInput });
  };

  setToolApproved = (toolApproved: boolean) => {
    this.setSnapshot({ toolApproved });
  };

  runActiveTool = async () => {
    const input = this.snapshot.toolInput.trim();
    const userApproved = this.snapshot.toolApproved;
    if (!input && this.snapshot.toolActive !== "day_planner") {
      this.setSnapshot({ error: "Tool input is required." });
      return;
    }

    this.setSnapshot({ toolBusy: true, error: undefined });
    try {
      const repoPath = this.coreRepoPath();
      const result = await this.executeTool(this.snapshot.toolActive, input, userApproved, repoPath);
      this.setSnapshot({ toolResult: result, toolBusy: false });
    } catch (error) {
      this.setSnapshot({
        toolBusy: false,
        error: error instanceof Error ? error.message : "Unable to run tool"
      });
    }
  };

  private executeTool = async (tool: ToolId, input: string, userApproved: boolean, repoPath: string) => {
    switch (tool) {
      case "web_search":
        return backendClient.webSearch({ query: input, userApproved });
      case "fetch_page":
        return backendClient.fetchPage({ url: input, userApproved });
      case "read_file":
        return backendClient.readFileTool({ path: input, allowedRoot: approvedRootFromPath(input), userApproved });
      case "git_status":
        return backendClient.gitStatusTool({ repoPath: input || repoPath, userApproved });
      case "git_diff":
        return backendClient.gitDiffTool({ repoPath: input || repoPath, userApproved });
      case "commit_message":
        return backendClient.commitMessageTool({ repoPath: input || repoPath, userApproved });
      case "code_task":
        return backendClient.codeTaskTool({ goal: input, userApproved });
      case "day_planner":
        return backendClient.dayPlannerTool({ focus: input ? [input] : [] });
      default:
        return backendClient.webSearch({ query: input, userApproved });
    }
  };

  private coreRepoPath = () => this.snapshot.coreSettings.vaultPath ?? "D:\\de'yana";

  private restingAssistantState = (): AssistantState =>
    this.snapshot.settings.uiMode === "expanded" ? "EXPANDED_PANEL" : "COMPACT_FLOATING";

  loadMemory = async (query = this.snapshot.memoryQuery) => {
    try {
      const queryValue = query.trim();
      const [response, entities, actions, decisions] = await Promise.all([
        backendClient.listMemory(queryValue),
        backendClient.listMemoryEntities({ query: queryValue }),
        backendClient.listMemoryInsights({ query: queryValue, type: "action_item", status: "open" }),
        backendClient.listMemoryInsights({ query: queryValue, type: "decision" })
      ]);
      this.setSnapshot({
        memoryItems: response.items,
        memoryEntities: entities.items,
        memoryActionItems: actions.items,
        memoryDecisions: decisions.items,
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
    const contentMarkdown = draft.contentMarkdown.trim();

    if (!title || (!summary && !contentMarkdown)) {
      this.setSnapshot({ error: "Memory needs a title and note body." });
      return;
    }

    this.setSnapshot({ memoryBusy: true, error: undefined });

    const request: MemoryCreateRequest = {
      type: "note",
      title,
      summary,
      contentMarkdown: contentMarkdown || summary,
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

  generateDailySummary = async () => {
    this.setSnapshot({ memoryBusy: true, error: undefined });
    try {
      const item = await backendClient.generateDailySummary();
      this.setSnapshot({
        memoryItems: mergeMemoryItem(this.snapshot.memoryItems, item),
        memoryBusy: false
      });
      await this.loadMemory();
    } catch (error) {
      this.setSnapshot({
        memoryBusy: false,
        error: error instanceof Error ? error.message : "Unable to generate daily summary"
      });
    }
  };

  generateProjectSummary = async () => {
    const project = this.snapshot.memoryProjectDraft.trim();
    if (!project) {
      this.setSnapshot({ error: "Project summary needs a project name." });
      return;
    }

    this.setSnapshot({ memoryBusy: true, error: undefined });
    try {
      const item = await backendClient.generateProjectSummary({ project });
      this.setSnapshot({
        memoryItems: mergeMemoryItem(this.snapshot.memoryItems, item),
        memoryQuery: project,
        memoryProjectDraft: "",
        memoryBusy: false
      });
      await this.loadMemory(project);
    } catch (error) {
      this.setSnapshot({
        memoryBusy: false,
        error: error instanceof Error ? error.message : "Unable to generate project summary"
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

  loadModelStatus = async () => {
    try {
      const modelStatusDetail = await backendClient.getModelStatus();
      this.setSnapshot({
        modelStatusDetail,
        modelStatus: modelStatusDetail.status,
        error: undefined
      });
    } catch (error) {
      this.setSnapshot({
        modelStatus: "offline",
        error: error instanceof Error ? error.message : "Unable to load model status"
      });
    }
  };

  selectModel = async (request: ModelSelectionRequest) => {
    this.setSnapshot({ modelStatus: "checking", error: undefined });
    try {
      const response = await backendClient.selectModel(request);
      this.setSnapshot({
        coreSettings: response.settings,
        modelStatusDetail: response.status,
        modelStatus: response.status.status
      });
    } catch (error) {
      this.setSnapshot({
        modelStatus: "missing",
        error: error instanceof Error ? error.message : "Unable to select model"
      });
    }
  };

  testModel = async () => {
    this.setSnapshot({ modelTestBusy: true, modelTestResponse: undefined, error: undefined });
    try {
      const modelTestResponse = await backendClient.testModel({
        prompt: "Reply with exactly: DEYANA_READY"
      });
      this.setSnapshot({
        modelTestBusy: false,
        modelTestResponse,
        modelStatus: "available"
      });
    } catch (error) {
      this.setSnapshot({
        modelTestBusy: false,
        error: error instanceof Error ? error.message : "Unable to test local model"
      });
      await this.loadModelStatus();
    }
  };

  loadChatHistory = async () => {
    try {
      const response = await backendClient.getChatHistory();
      this.setSnapshot({ chatMessages: response.messages, error: undefined });
    } catch (error) {
      this.setSnapshot({
        error: error instanceof Error ? error.message : "Unable to load chat history"
      });
    }
  };

  setChatDraft = (chatDraft: string) => {
    this.setSnapshot({ chatDraft });
  };

  sendChatMessage = async () => {
    const content = this.snapshot.chatDraft.trim();
    if (!content) {
      this.setSnapshot({ error: "Chat message cannot be empty." });
      return;
    }

    try {
      await this.sendChatContent(content, true);
    } catch {
      // sendChatContent already restores UI state and records the user-facing error.
    }
  };

  loadVoice = async () => {
    try {
      const [voiceSettings, voiceStatus] = await Promise.all([
        backendClient.getVoiceSettings(),
        backendClient.getVoiceStatus()
      ]);
      this.setSnapshot({ voiceSettings, voiceStatus, error: undefined });
    } catch (error) {
      this.setSnapshot({
        error: error instanceof Error ? error.message : "Unable to load local voice status"
      });
    }
  };

  patchVoiceSettings = async (patch: VoiceSettingsPatch) => {
    try {
      const voiceSettings = await backendClient.patchVoiceSettings(patch);
      const voiceStatus = await backendClient.getVoiceStatus();
      this.setSnapshot({ voiceSettings, voiceStatus, error: undefined });
    } catch (error) {
      this.setSnapshot({
        error: error instanceof Error ? error.message : "Unable to update local voice settings"
      });
    }
  };

  runPushToTalk = async () => {
    if (this.snapshot.voiceBusy) {
      return;
    }

    this.setSnapshot({
      voiceBusy: true,
      voiceTranscript: undefined,
      assistantState: "LISTENING",
      error: undefined
    });

    try {
      const voiceTranscript = await backendClient.transcribeVoice({
        listenSeconds: this.snapshot.voiceSettings?.listenSeconds
      });
      const transcript = voiceTranscript.transcript.trim();
      this.setSnapshot({
        voiceTranscript,
        assistantState: "TRANSCRIBING"
      });

      if (!transcript) {
        this.setSnapshot({
          voiceBusy: false,
          assistantState: this.restingAssistantState(),
          error: "No local speech was recognized."
        });
        return;
      }

      const response = await this.sendChatContent(transcript, false);
      if (this.snapshot.voiceSettings?.ttsEnabled && response.assistantMessage.content.trim()) {
        this.setSnapshot({ assistantState: "SPEAKING" });
        await backendClient.speakVoice({ text: response.assistantMessage.content });
      }

      this.setSnapshot({
        voiceBusy: false,
        assistantState: this.restingAssistantState()
      });
    } catch (error) {
      this.setSnapshot({
        voiceBusy: false,
        assistantState: this.restingAssistantState(),
        error: error instanceof Error ? error.message : "Unable to run local voice"
      });
      await this.loadVoice();
    }
  };

  speakLastAssistantMessage = async () => {
    const message = [...this.snapshot.chatMessages].reverse().find((item) => item.role === "assistant");
    if (!message?.content.trim()) {
      this.setSnapshot({ error: "No assistant response is available for speech." });
      return;
    }

    this.setSnapshot({ voiceBusy: true, assistantState: "SPEAKING", error: undefined });
    try {
      await backendClient.speakVoice({ text: message.content });
      this.setSnapshot({ voiceBusy: false, assistantState: this.restingAssistantState() });
    } catch (error) {
      this.setSnapshot({
        voiceBusy: false,
        assistantState: this.restingAssistantState(),
        error: error instanceof Error ? error.message : "Unable to speak locally"
      });
      await this.loadVoice();
    }
  };

  loadReleaseQuality = async () => {
    this.setSnapshot({ releaseBusy: true, error: undefined });
    try {
      const [
        releaseReadiness,
        releaseUpdatePlan,
        releaseLogs,
        releasePrivacyExport,
        releaseConnectorHealth,
        releasePerformance,
        releaseCrashRecovery
      ] = await Promise.all([
        backendClient.getReleaseReadiness(),
        backendClient.getReleaseUpdatePlan(),
        backendClient.listReleaseLogs(),
        backendClient.getPrivacyExport(),
        backendClient.getConnectorHealth(),
        backendClient.getPerformanceProfile(),
        backendClient.getCrashRecovery()
      ]);
      this.setSnapshot({
        releaseReadiness,
        releaseUpdatePlan,
        releaseLogs,
        releasePrivacyExport,
        releaseConnectorHealth,
        releasePerformance,
        releaseCrashRecovery,
        releaseBusy: false,
        error: undefined
      });
    } catch (error) {
      this.setSnapshot({
        releaseBusy: false,
        error: error instanceof Error ? error.message : "Unable to load release quality status"
      });
    }
  };

  readReleaseLog = async (path: string) => {
    this.setSnapshot({ releaseBusy: true, error: undefined });
    try {
      const releaseSelectedLog = await backendClient.readReleaseLog(path);
      this.setSnapshot({ releaseSelectedLog, releaseBusy: false });
    } catch (error) {
      this.setSnapshot({
        releaseBusy: false,
        error: error instanceof Error ? error.message : "Unable to read release log"
      });
    }
  };

  setReleaseDeletePhrase = (releaseDeletePhrase: string) => {
    this.setSnapshot({ releaseDeletePhrase });
  };

  setReleaseDeleteIncludeVault = (releaseDeleteIncludeVault: boolean) => {
    this.setSnapshot({ releaseDeleteIncludeVault });
  };

  deleteLocalData = async () => {
    this.setSnapshot({ releaseBusy: true, error: undefined });
    try {
      await backendClient.deleteLocalData({
        confirmationPhrase: this.snapshot.releaseDeletePhrase,
        includeVault: this.snapshot.releaseDeleteIncludeVault
      });
      this.setSnapshot({
        releaseBusy: false,
        releaseDeletePhrase: "",
        releaseDeleteIncludeVault: false,
        memoryItems: [],
        memoryEntities: [],
        memoryActionItems: [],
        memoryDecisions: [],
        chatMessages: [],
        privacyAuditEvents: [],
        connectors: defaultConnectors(),
        connectorSyncRuns: [],
        onboarding: DEFAULT_ONBOARDING_STATE,
        coreSettings: DEFAULT_CORE_APP_SETTINGS,
        onboardingStep: "welcome"
      });
      await this.loadReleaseQuality();
    } catch (error) {
      this.setSnapshot({
        releaseBusy: false,
        error: error instanceof Error ? error.message : "Unable to delete local data"
      });
    }
  };

  private sendChatContent = async (content: string, restoreDraftOnError: boolean): Promise<ChatMessageResponse> => {
    const trimmed = content.trim();
    if (!trimmed) {
      throw new Error("Chat message cannot be empty.");
    }

    this.setSnapshot({
      chatBusy: true,
      chatDraft: "",
      assistantState: "THINKING",
      error: undefined
    });

    try {
      const response = await backendClient.sendChatMessage({ content: trimmed });
      this.setSnapshot({
        chatBusy: false,
        assistantState: this.restingAssistantState(),
        chatMessages: mergeChatResponse(this.snapshot.chatMessages, response)
      });
      return response;
    } catch (error) {
      this.setSnapshot({
        chatBusy: false,
        chatDraft: restoreDraftOnError ? trimmed : this.snapshot.chatDraft,
        assistantState: this.restingAssistantState(),
        error: error instanceof Error ? error.message : "Unable to send local chat message"
      });
      await this.loadModelStatus();
      throw error;
    }
  };

  clearChatHistory = async () => {
    this.setSnapshot({ chatBusy: true, error: undefined });
    try {
      await backendClient.clearChatHistory();
      this.setSnapshot({ chatMessages: [], chatBusy: false });
    } catch (error) {
      this.setSnapshot({
        chatBusy: false,
        error: error instanceof Error ? error.message : "Unable to clear chat history"
      });
    }
  };

  loadPrivacyAudit = async () => {
    try {
      const [privacyStatus, audit] = await Promise.all([
        backendClient.getPrivacyStatus(),
        backendClient.listPrivacyAudit()
      ]);
      this.setSnapshot({
        privacyStatus,
        privacyAuditEvents: audit.events,
        error: undefined
      });
    } catch (error) {
      this.setSnapshot({
        error: error instanceof Error ? error.message : "Unable to load privacy audit"
      });
    }
  };

  testPrivacyFirewall = async () => {
    this.setSnapshot({ privacyBusy: true, error: undefined });
    try {
      await backendClient.checkPrivacyRequest({
        url: "https://api.openai.com/v1/chat/completions",
        method: "POST",
        purpose: "cloud_ai",
        dataCategory: "private_memory",
        payloadPreview: "Private memory summary"
      });
      await this.loadPrivacyAudit();
      this.setSnapshot({ privacyBusy: false });
    } catch (error) {
      this.setSnapshot({
        privacyBusy: false,
        error: error instanceof Error ? error.message : "Unable to test privacy firewall"
      });
    }
  };

  clearPrivacyAudit = async () => {
    this.setSnapshot({ privacyBusy: true, error: undefined });
    try {
      await backendClient.clearPrivacyAudit();
      this.setSnapshot({
        privacyAuditEvents: [],
        privacyBusy: false
      });
      await this.loadPrivacyAudit();
    } catch (error) {
      this.setSnapshot({
        privacyBusy: false,
        error: error instanceof Error ? error.message : "Unable to clear privacy audit"
      });
    }
  };

  loadConnectors = async () => {
    try {
      const [connectors, syncRuns] = await Promise.all([
        backendClient.listConnectors(),
        backendClient.listConnectorSyncRuns()
      ]);
      this.setSnapshot({
        connectors: connectors.items,
        connectorSyncRuns: syncRuns.items,
        syncStatus: deriveSyncStatus(connectors.items),
        error: undefined
      });
    } catch (error) {
      this.setSnapshot({
        error: error instanceof Error ? error.message : "Unable to load connectors"
      });
    }
  };

  connectConnector = async (connectorId: string) => {
    this.setConnectorBusy(connectorId, true);
    try {
      const started = await backendClient.startConnectorOAuth(connectorId, {
        redirectUri: "deyana://oauth/callback"
      });
      if (!started.mock) {
        window.open(started.authorizationUrl, "_blank", "noopener,noreferrer");
        this.setSnapshot({
          connectorOAuth: {
            ...this.snapshot.connectorOAuth,
            [connectorId]: {
              state: started.state,
              authorizationUrl: started.authorizationUrl,
              expiresAt: started.expiresAt
            }
          },
          error: undefined
        });
        return;
      }
      const connector = await backendClient.completeConnectorOAuth(connectorId, {
        state: started.state,
        code: `mock-ui-${window.crypto.randomUUID()}`,
        userApproved: true
      });
      const connectors = mergeConnector(this.snapshot.connectors, connector);
      this.setSnapshot({
        connectors,
        syncStatus: deriveSyncStatus(connectors),
        error: undefined
      });
      await this.loadConnectors();
    } catch (error) {
      this.setSnapshot({
        error: error instanceof Error ? error.message : "Unable to connect local connector"
      });
    } finally {
      this.setConnectorBusy(connectorId, false);
    }
  };

  setConnectorOAuthCode = (connectorId: string, code: string) => {
    this.setSnapshot({
      connectorOAuthCodes: {
        ...this.snapshot.connectorOAuthCodes,
        [connectorId]: code
      }
    });
  };

  completeConnectorOAuth = async (connectorId: string) => {
    const pending = this.snapshot.connectorOAuth[connectorId];
    const code = this.snapshot.connectorOAuthCodes[connectorId]?.trim();
    if (!pending || !code) {
      this.setSnapshot({ error: "Paste the connector OAuth code before completing setup." });
      return;
    }

    this.setConnectorBusy(connectorId, true);
    try {
      const connector = await backendClient.completeConnectorOAuth(connectorId, {
        state: pending.state,
        code,
        userApproved: true
      });
      const connectors = mergeConnector(this.snapshot.connectors, connector);
      const { [connectorId]: _pending, ...connectorOAuth } = this.snapshot.connectorOAuth;
      const { [connectorId]: _code, ...connectorOAuthCodes } = this.snapshot.connectorOAuthCodes;
      this.setSnapshot({
        connectors,
        connectorOAuth,
        connectorOAuthCodes,
        syncStatus: deriveSyncStatus(connectors),
        error: undefined
      });
      await this.loadConnectors();
    } catch (error) {
      this.setSnapshot({
        error: error instanceof Error ? error.message : "Unable to complete connector OAuth"
      });
    } finally {
      this.setConnectorBusy(connectorId, false);
    }
  };

  disconnectConnector = async (connectorId: string) => {
    this.setConnectorBusy(connectorId, true);
    try {
      const response = await backendClient.disconnectConnector(connectorId);
      const connectors = mergeConnector(this.snapshot.connectors, response.connector);
      this.setSnapshot({
        connectors,
        syncStatus: deriveSyncStatus(connectors),
        error: undefined
      });
      await this.loadConnectors();
    } catch (error) {
      this.setSnapshot({
        error: error instanceof Error ? error.message : "Unable to disconnect connector"
      });
    } finally {
      this.setConnectorBusy(connectorId, false);
    }
  };

  syncConnector = async (connectorId: string) => {
    this.setConnectorBusy(connectorId, true);
    this.setSnapshot({ syncStatus: "syncing", assistantState: "SYNCING", error: undefined });
    try {
      const response = await backendClient.syncConnector(connectorId, { reason: "manual" });
      const connectors = mergeConnector(this.snapshot.connectors, response.connector);
      const syncRuns = mergeSyncRun(this.snapshot.connectorSyncRuns, response.run);
      this.setSnapshot({
        connectors,
        connectorSyncRuns: syncRuns,
        syncStatus: deriveSyncStatus(connectors),
        assistantState: this.snapshot.settings.uiMode === "expanded" ? "EXPANDED_PANEL" : "COMPACT_FLOATING",
        error: undefined
      });
      await this.loadConnectors();
      await this.loadMemory();
    } catch (error) {
      this.setSnapshot({
        syncStatus: "error",
        assistantState: "CONNECTOR_ERROR",
        error: error instanceof Error ? error.message : "Unable to sync connector"
      });
      await this.loadConnectors();
    } finally {
      this.setConnectorBusy(connectorId, false);
    }
  };

  updateConnectorSettings = async (
    connectorId: string,
    patch: { enabled?: boolean; syncIntervalMinutes?: number }
  ) => {
    this.setConnectorBusy(connectorId, true);
    try {
      const connector = await backendClient.updateConnectorSettings(connectorId, patch);
      const connectors = mergeConnector(this.snapshot.connectors, connector);
      this.setSnapshot({
        connectors,
        syncStatus: deriveSyncStatus(connectors),
        error: undefined
      });
    } catch (error) {
      this.setSnapshot({
        error: error instanceof Error ? error.message : "Unable to update connector settings"
      });
    } finally {
      this.setConnectorBusy(connectorId, false);
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
      const [
        backendStatus,
        coreSettings,
        onboarding,
        modelStatusDetail,
        chatHistory,
        privacyStatus,
        privacyAudit,
        connectors,
        connectorSyncRuns,
        voiceSettings,
        voiceStatus
      ] = await Promise.all([
        backendClient.getStatus(),
        backendClient.getSettings(),
        backendClient.getOnboardingState(),
        backendClient.getModelStatus(),
        backendClient.getChatHistory(),
        backendClient.getPrivacyStatus(),
        backendClient.listPrivacyAudit(),
        backendClient.listConnectors(),
        backendClient.listConnectorSyncRuns(),
        backendClient.getVoiceSettings(),
        backendClient.getVoiceStatus()
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
        modelStatusDetail,
        modelStatus: modelStatusDetail.status,
        chatMessages: chatHistory.messages,
        privacyStatus,
        privacyAuditEvents: privacyAudit.events,
        connectors: connectors.items,
        connectorSyncRuns: connectorSyncRuns.items,
        voiceSettings,
        voiceStatus,
        syncStatus: deriveSyncStatus(connectors.items),
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

    if (
      event.type === "memory.item.created" ||
      event.type === "memory.item.updated" ||
      event.type === "memory.summary.generated"
    ) {
      const item = event.payload.item;
      this.setSnapshot({
        memoryItems: mergeMemoryItem(this.snapshot.memoryItems, item),
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

    if (event.type === "models.status.changed") {
      this.setSnapshot({
        modelStatusDetail: event.payload,
        modelStatus: event.payload.status,
        lastBackendEventType: event.type
      });
      return;
    }

    if (event.type === "models.test.completed") {
      this.setSnapshot({
        modelTestResponse: event.payload,
        modelStatus: "available",
        lastBackendEventType: event.type
      });
      return;
    }

    if (event.type === "chat.message.created") {
      this.setSnapshot({
        chatMessages: mergeChatResponse(this.snapshot.chatMessages, event.payload),
        lastBackendEventType: event.type
      });
      return;
    }

    if (event.type === "chat.history.deleted") {
      this.setSnapshot({
        chatMessages: [],
        lastBackendEventType: event.type
      });
      return;
    }

    if (event.type === "privacy.request.blocked" || event.type === "privacy.request.allowed") {
      const existing = this.snapshot.privacyAuditEvents.filter(
        (item) => item.id !== event.payload.auditEvent.id
      );
      const blockedDelta = event.payload.auditEvent.decision === "block" ? 1 : 0;
      const allowedDelta = event.payload.auditEvent.decision === "allow" ? 1 : 0;
      this.setSnapshot({
        privacyAuditEvents: [event.payload.auditEvent, ...existing].slice(0, 20),
        privacyStatus: this.snapshot.privacyStatus
          ? {
              ...this.snapshot.privacyStatus,
              auditEvents: this.snapshot.privacyStatus.auditEvents + 1,
              blockedEvents: this.snapshot.privacyStatus.blockedEvents + blockedDelta,
              allowedEvents: this.snapshot.privacyStatus.allowedEvents + allowedDelta,
              lastBlocked:
                event.payload.auditEvent.decision === "block"
                  ? event.payload.auditEvent
                  : this.snapshot.privacyStatus.lastBlocked
            }
          : this.snapshot.privacyStatus,
        lastBackendEventType: event.type
      });
      return;
    }

    if (event.type === "privacy.audit.deleted") {
      this.setSnapshot({
        privacyAuditEvents: [],
        privacyStatus: this.snapshot.privacyStatus
          ? {
              ...this.snapshot.privacyStatus,
              auditEvents: 0,
              blockedEvents: 0,
              allowedEvents: 0,
              lastBlocked: null
            }
          : this.snapshot.privacyStatus,
        lastBackendEventType: event.type
      });
      return;
    }

    if (
      event.type === "tool.completed" ||
      event.type === "tool.failed" ||
      event.type === "tool.permission.required"
    ) {
      this.setSnapshot({
        toolResult: event.payload,
        lastBackendEventType: event.type
      });
      return;
    }

    if (event.type === "voice.settings.updated") {
      this.setSnapshot({
        voiceSettings: event.payload,
        lastBackendEventType: event.type
      });
      void this.loadVoice();
      return;
    }

    if (event.type === "voice.recording.started") {
      this.setSnapshot({
        assistantState: "LISTENING",
        lastBackendEventType: event.type
      });
      return;
    }

    if (event.type === "voice.recording.stopped" || event.type === "voice.transcription.started") {
      this.setSnapshot({
        assistantState: "TRANSCRIBING",
        lastBackendEventType: event.type
      });
      return;
    }

    if (event.type === "voice.transcription.completed") {
      this.setSnapshot({
        voiceTranscript: event.payload,
        lastBackendEventType: event.type
      });
      return;
    }

    if (event.type === "voice.transcription.failed") {
      this.setSnapshot({
        voiceBusy: false,
        assistantState: this.restingAssistantState(),
        error: event.payload.reason,
        lastBackendEventType: event.type
      });
      return;
    }

    if (event.type === "tts.started") {
      this.setSnapshot({
        assistantState: "SPEAKING",
        lastBackendEventType: event.type
      });
      return;
    }

    if (event.type === "tts.completed") {
      this.setSnapshot({
        assistantState: this.restingAssistantState(),
        lastBackendEventType: event.type
      });
      return;
    }

    if (event.type === "tts.failed") {
      this.setSnapshot({
        voiceBusy: false,
        assistantState: this.restingAssistantState(),
        error: event.payload.reason,
        lastBackendEventType: event.type
      });
      return;
    }

    if (event.type === "release.local_data.deleted") {
      this.setSnapshot({
        memoryItems: [],
        memoryEntities: [],
        memoryActionItems: [],
        memoryDecisions: [],
        chatMessages: [],
        privacyAuditEvents: [],
        connectors: defaultConnectors(),
        connectorSyncRuns: [],
        onboarding: DEFAULT_ONBOARDING_STATE,
        coreSettings: DEFAULT_CORE_APP_SETTINGS,
        onboardingStep: "welcome",
        releaseDeletePhrase: "",
        releaseDeleteIncludeVault: false,
        lastBackendEventType: event.type
      });
      return;
    }

    if (event.type === "connector.oauth.started") {
      this.setSnapshot({ lastBackendEventType: event.type });
      return;
    }

    if (event.type === "connector.status.changed" || event.type === "connector.oauth.completed") {
      const connectors = mergeConnector(this.snapshot.connectors, event.payload.connector);
      this.setSnapshot({
        connectors,
        syncStatus: deriveSyncStatus(connectors),
        lastBackendEventType: event.type
      });
      return;
    }

    if (
      event.type === "connector.sync.started" ||
      event.type === "connector.sync.completed" ||
      event.type === "connector.sync.failed" ||
      event.type === "connector.sync.skipped"
    ) {
      const connectors = mergeConnector(this.snapshot.connectors, event.payload.connector);
      const connectorSyncRuns = mergeSyncRun(this.snapshot.connectorSyncRuns, event.payload.run);
      this.setSnapshot({
        connectors,
        connectorSyncRuns,
        syncStatus: deriveSyncStatus(connectors),
        lastBackendEventType: event.type
      });
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

  private setConnectorBusy = (connectorId: string, busy: boolean) => {
    this.setSnapshot({
      connectorBusy: {
        ...this.snapshot.connectorBusy,
        [connectorId]: busy
      }
    });
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

const mergeChatResponse = (
  current: ChatMessageItem[],
  response: ChatMessageResponse
): ChatMessageItem[] => {
  const byId = new Map(current.map((message) => [message.id, message]));
  byId.set(response.userMessage.id, response.userMessage);
  byId.set(response.assistantMessage.id, response.assistantMessage);
  return [...byId.values()].sort(
    (left, right) => Date.parse(left.createdAt) - Date.parse(right.createdAt)
  );
};

const mergeMemoryItem = (current: MemoryItem[], item: MemoryItem): MemoryItem[] => {
  const existing = current.filter((memory) => memory.id !== item.id);
  return [item, ...existing].slice(0, 20);
};

const approvedRootFromPath = (path: string): string => {
  const normalized = path.trim();
  const index = Math.max(normalized.lastIndexOf("\\"), normalized.lastIndexOf("/"));
  return index > 0 ? normalized.slice(0, index) : ".";
};

const mergeConnector = (current: ConnectorItem[], connector: ConnectorItem): ConnectorItem[] => {
  const byId = new Map(current.map((item) => [item.id, item]));
  byId.set(connector.id, connector);
  const preferredOrder = ["gmail", "calendar", "github", "drive", "slack", "notion", "jira", "linear", "stripe"];
  return [...byId.values()].sort((left, right) => {
    const leftIndex = preferredOrder.indexOf(left.id);
    const rightIndex = preferredOrder.indexOf(right.id);
    return (leftIndex === -1 ? 99 : leftIndex) - (rightIndex === -1 ? 99 : rightIndex);
  });
};

const mergeSyncRun = (current: ConnectorSyncRun[], run: ConnectorSyncRun): ConnectorSyncRun[] => {
  const byId = new Map(current.map((item) => [item.id, item]));
  byId.set(run.id, run);
  return [...byId.values()]
    .sort((left, right) => Date.parse(right.startedAt) - Date.parse(left.startedAt))
    .slice(0, 12);
};

const deriveSyncStatus = (connectors: ConnectorItem[]): SyncStatus => {
  if (connectors.some((connector) => connector.status === "syncing")) {
    return "syncing";
  }
  if (connectors.some((connector) => connector.status === "error")) {
    return "error";
  }
  if (connectors.some((connector) => connector.status === "paused")) {
    return "paused";
  }
  return "idle";
};
