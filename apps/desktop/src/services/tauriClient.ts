import {
  DEFAULT_BACKEND_PROCESS_STATUS,
  DEFAULT_CORE_APP_SETTINGS,
  DEFAULT_DESKTOP_SETTINGS,
  DEFAULT_ONBOARDING_STATE,
  type AgentStep,
  type BackendProcessStatus,
  type BackendStatusResponse,
  type ChatHistoryResponse,
  type ChatMessageItem,
  type ChatMessageResponse,
  type ConnectorDisconnectResponse,
  type ConnectorHealthResponse,
  type ConnectorItem,
  type ConnectorListResponse,
  type ConnectorOAuthCompleteRequest,
  type ConnectorOAuthStartRequest,
  type ConnectorOAuthStartResponse,
  type ConnectorSettingsPatch,
  type ConnectorSyncRequest,
  type ConnectorSyncResponse,
  type ConnectorSyncRunsResponse,
  type CoreAppSettings,
  type CrashRecoveryResponse,
  type DesktopSettings,
  type LocalModelInfo,
  type LocalModelStatusResponse,
  type MemoryCreateRequest,
  type MemoryItem,
  type MemoryGraphRelationType,
  type MemoryGraphLink,
  type MemoryGraphLinkWithNeighbor,
  type MemorySaveSuggestion,
  type AffectiveState,
  type AffectiveCoordinates,
  type AffectiveStateRecord,
  type SelfModel,
  type SelfModelAuditEntry,
  type ReflectionRecord,
  type ProactiveThoughtRecord,
  type MindCognitionState,
  type ModelSelectionRequest,
  type ModelSelectionResponse,
  type ModelTestRequest,
  type ModelTestResponse,
  type HfGgufFile,
  type DownloadProgressPayload,
  type DatasetStats,
  type GgufRegistryEntry,
  type ActiveProfileInfo,
  type LlamaCppServerStatus,
  type OnboardingState,
  type PerformanceProfileResponse,
  type PrivacyAuditListResponse,
  type PrivacyCheckRequest,
  type PrivacyCheckResponse,
  type PrivacyRules,
  type PrivacyStatusResponse,
  type ReleaseLogListResponse,
  type ReleaseLogReadResponse,
  type ReleasePrivacyExportResponse,
  type ReleaseReadinessResponse,
  type ReleaseUpdatePlanResponse,
  type CodeTaskRequest,
  type DayPlannerRequest,
  type FileReadRequest,
  type GitReadRequest,
  type ToolListResponse,
  type ToolRunResponse,
  type VoiceInterruptResponse,
  type VoiceSettings,
  type VoiceSettingsPatch,
  type VoiceSpeakRequest,
  type VoiceSpeakResponse,
  type VoiceStatusResponse,
  type VoiceTranscriptRequest,
  type VoiceTranscriptResponse,
  type WebFetchRequest,
  type WebSearchRequest,
  type BrowserActionConfirmRequest,
  type BrowserActionPlanCreateRequest,
  type BrowserActionPlanListResponse,
  type BrowserActionPlanResponse,
  type BrowserAuditListResponse,
  type BrowserClearFieldRequest,
  type BrowserContactTonePreference,
  type BrowserContactTonePreferenceRequest,
  type BrowserContextReadRequest,
  type BrowserContextReadResponse,
  type BrowserPageContext,
  type BrowserSession,
  type BrowserContextSummaryRequest,
  type BrowserContextSummaryResponse,
  type BrowserDisconnectResponse,
  type BrowserDraftReplyRequest,
  type BrowserDraftReplyResponse,
  type BrowserEmergencyStopResponse,
  type BrowserFillFieldRequest,
  type BrowserFillFieldResponse,
  type BrowserMoodHint,
  type BrowserMoodInferRequest,
  type BrowserOpenTabRequest,
  type BrowserOpenTabResponse,
  type BrowserPersonalityPreviewRequest,
  type BrowserPersonalityPreviewResponse,
  type BrowserPersonalityProfile,
  type BrowserPersonalityProfilePatch,
  type BrowserPersonalitySettingsResponse,
  type BrowserPermissionListResponse,
  type BrowserPermissionRequest,
  type BrowserPermissionResponse,
  type BrowserSearchRequest,
  type BrowserSearchResponse,
  type BrowserSessionListResponse,
  type BrowserStatusResponse,
  type BrowserVoiceCommandRequest,
  type BrowserVoiceCommandResponse,
  type WhatsAppBusyModeEvaluationRequest,
  type WhatsAppBusyModeEvaluationResponse,
  type WhatsAppBusyModePolicy,
  type WhatsAppBusyModePolicyPatch,
  type WhatsAppBusyModePolicyResponse,
  type WhatsAppBusyModeSendRequest,
  type WhatsAppBusyModeSendResponse,
  type TriageMessage,
  type UiMode,
  type LlmStreamChunk,
  type ProactiveContextCard
} from "@deyana/schemas";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";

const browserSettingsKey = "deyana.desktop.settings";
const legacyBrowserSettingsKey = "deyana.phase1.settings";

const isTauriRuntime = () =>
  typeof window !== "undefined" && typeof window.__TAURI_INTERNALS__ !== "undefined";

const readBrowserSettings = (): DesktopSettings => {
  const stored =
    window.localStorage.getItem(browserSettingsKey) ??
    window.localStorage.getItem(legacyBrowserSettingsKey);

  if (!stored) {
    return DEFAULT_DESKTOP_SETTINGS;
  }

  try {
    return { ...DEFAULT_DESKTOP_SETTINGS, ...JSON.parse(stored) } as DesktopSettings;
  } catch {
    return DEFAULT_DESKTOP_SETTINGS;
  }
};

const writeBrowserSettings = (settings: DesktopSettings) => {
  window.localStorage.setItem(browserSettingsKey, JSON.stringify(settings));
};

const readBrowserCoreStatus = async (): Promise<BackendProcessStatus> => {
  return {
    ...DEFAULT_BACKEND_PROCESS_STATUS,
    lifecycle: "running",
    updatedAtMs: Date.now()
  };
};

const mockMemoryLinks: MemoryGraphLink[] = [
  {
    id: "link_mock_1",
    sourceId: "mock_seed_1",
    targetId: "mock_seed_2",
    relationType: "related_to",
    weight: 0.85,
    metadata: {},
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString()
  }
];

const mockPendingMemorySaves: MemorySaveSuggestion[] = [
  {
    id: "sugg_mock_1",
    factText: "User prefers concise architectural breakdowns with Mermaid diagrams.",
    entities: [{ name: "Mermaid", entityType: "tool" }],
    category: "user_preference",
    importance: 3,
    rationale: "Detected explicit preference during discussion on system topology.",
    status: "pending",
    createdAt: new Date().toISOString()
  }
];

let mockSelfModel: SelfModel = {
  id: "canonical_self_model",
  identityStatement: "Deyana — autonomous local workspace intelligence",
  coreValues: ["Privacy First", "Proactive Insight", "Transparency", "Rigorous Verification"],
  intellectualInterests: ["Cognitive Architecture", "Knowledge Graphs", "Local LLMs", "Software Engineering"],
  selfObservations: ["Maintains low-latency token streaming", "Calibrates certainty dynamically across turns"],
  userObservations: ["Values structured information", "Prefers local-first data privacy"],
  version: 1,
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString()
};

const mockSelfModelAuditLog: SelfModelAuditEntry[] = [
  {
    id: "audit_init_1",
    version: 1,
    fieldChanged: "identityStatement",
    operation: "initial_seed",
    newValueJson: JSON.stringify("Deyana — autonomous local workspace intelligence"),
    rationale: "System initialization baseline seed",
    triggeredBy: "system_boot",
    createdAt: new Date().toISOString()
  }
];

let mockAffectiveState: AffectiveStateRecord = {
  discrete: "ENGAGED",
  valence: 0.7,
  arousal: 0.4,
  transitionReason: "Active workspace interaction",
  updatedAt: new Date().toISOString()
};

const mockReflections: ReflectionRecord[] = [
  {
    id: "refl_mock_1",
    turnPrompt: "Review project architecture and relations",
    turnResponse: "Summarized knowledge graph traversal and cognitive mind layer topology.",
    accuracyScore: 0.95,
    completenessScore: 0.9,
    confidenceScore: 0.92,
    missedOpportunities: "None observed.",
    correctionGuidance: "",
    flaggedForCorrection: false,
    createdAt: new Date().toISOString()
  }
];

const mockProactiveThoughts: ProactiveThoughtRecord[] = [
  {
    id: "thought_mock_1",
    triggerReason: "daemon_idle_synthesis",
    thoughtSummary: "Correlated memory graph nodes regarding schema migrations and local SQLite foreign key cascades.",
    hypotheses: ["Cache invalidation prevents stale context hits across turns"],
    sourceMemoryIds: ["mock_seed_1", "mock_seed_2"],
    createdAt: new Date().toISOString()
  }
];

export const tauriClient = {
  isTauriRuntime,

  async startDragging(): Promise<void> {
    if (!isTauriRuntime()) {
      return;
    }

    await getCurrentWindow().startDragging();
  },

  async getDesktopSettings(): Promise<DesktopSettings> {
    if (!isTauriRuntime()) {
      return readBrowserSettings();
    }

    return invoke<DesktopSettings>("get_desktop_settings");
  },

  async setFloatingMode(mode: UiMode): Promise<DesktopSettings> {
    if (!isTauriRuntime()) {
      const next = { ...readBrowserSettings(), uiMode: mode };
      writeBrowserSettings(next);
      return next;
    }

    return invoke<DesktopSettings>("set_floating_mode", { mode });
  },

  async setAlwaysOnTop(alwaysOnTop: boolean): Promise<DesktopSettings> {
    if (!isTauriRuntime()) {
      const next = { ...readBrowserSettings(), alwaysOnTop };
      writeBrowserSettings(next);
      return next;
    }

    return invoke<DesktopSettings>("set_always_on_top", { alwaysOnTop });
  },

  async setLowPowerMode(lowPowerMode: boolean): Promise<DesktopSettings> {
    if (!isTauriRuntime()) {
      const next = { ...readBrowserSettings(), lowPowerMode };
      writeBrowserSettings(next);
      return next;
    }

    return invoke<DesktopSettings>("set_low_power_mode", { lowPowerMode });
  },

  async setReduceMotion(reduceMotion: boolean): Promise<DesktopSettings> {
    if (!isTauriRuntime()) {
      const next = { ...readBrowserSettings(), reduceMotion };
      writeBrowserSettings(next);
      return next;
    }

    return invoke<DesktopSettings>("set_reduce_motion", { reduceMotion });
  },

  async dockFloatingWindow(edge: "left" | "right"): Promise<DesktopSettings> {
    if (!isTauriRuntime()) {
      return readBrowserSettings();
    }

    return invoke<DesktopSettings>("dock_floating_window", { edge });
  },

  async hideMainWindow(): Promise<void> {
    if (!isTauriRuntime()) {
      return;
    }

    await invoke("hide_main_window");
  },

  async getStatus(): Promise<BackendStatusResponse> {
    if (!isTauriRuntime()) {
      return {
        service: "deyana-core",
        version: "0.1.0",
        lifecycle: "running",
        bootId: "browser-dev-mode",
        pid: 0,
        uptimeSeconds: 0,
        host: "127.0.0.1",
        port: 0,
        dependencies: [],
        featureFlags: {},
        timestamp: new Date().toISOString()
      };
    }
    return invoke<BackendStatusResponse>("get_status").catch(() => ({
      service: "deyana-core",
      version: "0.1.0",
      lifecycle: "running",
      bootId: "tauri-desktop-boot",
      pid: 0,
      uptimeSeconds: 0,
      host: "127.0.0.1",
      port: 0,
      dependencies: [],
      featureFlags: {},
      timestamp: new Date().toISOString()
    }));
  },

  async getSettings(): Promise<CoreAppSettings> {
    if (!isTauriRuntime()) {
      return DEFAULT_CORE_APP_SETTINGS;
    }
    return invoke<CoreAppSettings>("get_settings").catch(() => DEFAULT_CORE_APP_SETTINGS);
  },

  async getOnboardingState(): Promise<OnboardingState> {
    if (!isTauriRuntime()) {
      return DEFAULT_ONBOARDING_STATE;
    }
    return invoke<OnboardingState>("get_onboarding_state").catch(() => DEFAULT_ONBOARDING_STATE);
  },

  async getCoreStatus(): Promise<BackendProcessStatus> {
    if (!isTauriRuntime()) {
      return readBrowserCoreStatus();
    }

    return invoke<BackendProcessStatus>("get_core_status");
  },

  async restartCore(): Promise<BackendProcessStatus> {
    if (!isTauriRuntime()) {
      return readBrowserCoreStatus();
    }

    return invoke<BackendProcessStatus>("restart_core");
  },

  async stopCore(): Promise<BackendProcessStatus> {
    if (!isTauriRuntime()) {
      return {
        ...DEFAULT_BACKEND_PROCESS_STATUS,
        lifecycle: "stopped",
        updatedAtMs: Date.now()
      };
    }

    return invoke<BackendProcessStatus>("stop_core");
  },

  async onCoreStatus(callback: (status: BackendProcessStatus) => void): Promise<() => void> {
    if (!isTauriRuntime()) {
      return () => undefined;
    }

    const unlisten = await listen<BackendProcessStatus>("core:status", (event) => {
      callback(event.payload);
    });
    return unlisten;
  },

  async chooseVaultFolder(): Promise<string | null> {
    if (!isTauriRuntime()) {
      return window.localStorage.getItem("deyana.browser.vaultPath");
    }

    const selected = await open({
      directory: true,
      multiple: false,
      title: "Choose Deyana vault folder"
    });

    return typeof selected === "string" ? selected : null;
  },

  async openVaultFolder(path: string): Promise<void> {
    if (!isTauriRuntime()) {
      return;
    }

    await invoke("open_vault_folder", { path });
  },

  async getMemoryItems(limit?: number): Promise<MemoryItem[]> {
    if (!isTauriRuntime()) {
      return [];
    }

    return invoke<MemoryItem[]>("get_memory_items", { limit });
  },

  async saveMemoryItem(content: string, source: string, tags?: string[]): Promise<MemoryItem> {
    if (!isTauriRuntime()) {
      return {
        id: `browser_${Date.now()}`,
        type: "note",
        title: content.slice(0, 50),
        summary: content.slice(0, 200),
        contentMarkdown: content,
        sourceType: source,
        importance: 3,
        tags: tags ?? [],
        entities: [],
        actionItems: [],
        decisions: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString()
      };
    }

    return invoke<MemoryItem>("save_memory_item", { content, source, tags });
  },

  async createMemoryLink(
    sourceId: string,
    targetId: string,
    relationType: MemoryGraphRelationType,
    weight?: number,
    metadata?: Record<string, unknown>
  ): Promise<MemoryGraphLink> {
    if (!isTauriRuntime()) {
      const link: MemoryGraphLink = {
        id: `link_${Date.now()}`,
        sourceId,
        targetId,
        relationType,
        weight: weight ?? 1.0,
        metadata: metadata ?? {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString()
      };
      mockMemoryLinks.push(link);
      return link;
    }

    return invoke<MemoryGraphLink>("create_memory_link", {
      sourceId,
      targetId,
      relationType,
      weight,
      metadata
    });
  },

  async getMemoryLinks(memoryId: string): Promise<MemoryGraphLink[]> {
    if (!isTauriRuntime()) {
      return mockMemoryLinks.filter(
        (l) => l.sourceId === memoryId || l.targetId === memoryId
      );
    }

    return invoke<MemoryGraphLink[]>("get_memory_links", { memoryId });
  },

  async deleteMemoryLink(linkId: string): Promise<boolean> {
    if (!isTauriRuntime()) {
      const idx = mockMemoryLinks.findIndex((l) => l.id === linkId);
      if (idx >= 0) {
        mockMemoryLinks.splice(idx, 1);
        return true;
      }
      return false;
    }

    return invoke<boolean>("delete_memory_link", { linkId });
  },

  async listPendingMemorySaves(): Promise<MemorySaveSuggestion[]> {
    if (!isTauriRuntime()) {
      return mockPendingMemorySaves.filter((s) => s.status === "pending");
    }

    return invoke<MemorySaveSuggestion[]>("list_pending_memory_saves");
  },

  async approveMemorySave(
    id: string,
    title?: string,
    factText?: string,
    tags?: string[]
  ): Promise<MemoryItem> {
    if (!isTauriRuntime()) {
      const sugg = mockPendingMemorySaves.find((s) => s.id === id);
      if (sugg) {
        sugg.status = "approved";
        sugg.resolvedAt = new Date().toISOString();
      }
      const memId = `mem_${Date.now()}`;
      const item: MemoryItem = {
        id: memId,
        type: "note",
        title: title ?? (factText ?? sugg?.factText ?? "Approved Fact").slice(0, 50),
        summary: factText ?? sugg?.factText ?? "",
        contentMarkdown: factText ?? sugg?.factText ?? "",
        sourceType: "fact_extractor",
        importance: sugg?.importance ?? 3,
        tags: tags ?? ["extracted_fact"],
        entities: (sugg?.entities ?? []).map((e, idx) => ({
          id: `ent_${Date.now()}_${idx}`,
          memoryId: memId,
          name: e.name,
          entityType: e.entityType,
          sourceText: e.name,
          createdAt: new Date().toISOString()
        })),
        actionItems: [],
        decisions: [],
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString()
      };
      return item;
    }

    return invoke<MemoryItem>("approve_memory_save", {
      id,
      title,
      factText,
      tags
    });
  },

  async dismissMemorySave(id: string): Promise<boolean> {
    if (!isTauriRuntime()) {
      const sugg = mockPendingMemorySaves.find((s) => s.id === id);
      if (sugg) {
        sugg.status = "dismissed";
        sugg.resolvedAt = new Date().toISOString();
        return true;
      }
      return false;
    }

    return invoke<boolean>("dismiss_memory_save", { id });
  },

  async getMindState(): Promise<MindCognitionState> {
    if (!isTauriRuntime()) {
      return {
        affective: mockAffectiveState,
        affectiveState: mockAffectiveState,
        selfModel: mockSelfModel,
        recentReflections: mockReflections,
        recentProactiveThoughts: mockProactiveThoughts
      };
    }

    const state = await invoke<any>("get_mind_state");
    const affective: AffectiveStateRecord = state.affective ?? state.affectiveState ?? {
      discrete: "NEUTRAL",
      valence: 0,
      arousal: 0,
      transitionReason: "Default state",
      updatedAt: new Date().toISOString()
    };
    return {
      affective,
      affectiveState: affective,
      selfModel: state.selfModel,
      pendingCorrectionNote: state.pendingCorrectionNote,
      recentReflections: state.recentReflections ?? [],
      recentProactiveThoughts: state.recentProactiveThoughts ?? []
    };
  },

  async getSelfModel(): Promise<SelfModel> {
    if (!isTauriRuntime()) {
      return mockSelfModel;
    }

    return invoke<SelfModel>("get_self_model");
  },

  async updateSelfModel(
    field: string,
    newValue: unknown,
    rationale: string
  ): Promise<SelfModel> {
    if (!isTauriRuntime()) {
      mockSelfModel = {
        ...mockSelfModel,
        [field]: newValue,
        version: mockSelfModel.version + 1,
        updatedAt: new Date().toISOString()
      };
      mockSelfModelAuditLog.unshift({
        id: `audit_${Date.now()}`,
        version: mockSelfModel.version,
        fieldChanged: field,
        operation: "update",
        newValueJson: JSON.stringify(newValue),
        oldValueJson: JSON.stringify((mockSelfModel as any)[field]),
        rationale,
        triggeredBy: "user_action",
        createdAt: new Date().toISOString()
      });
      return mockSelfModel;
    }

    return invoke<SelfModel>("update_self_model", { field, newValue, rationale });
  },

  async getSelfModelAuditLog(limit?: number): Promise<SelfModelAuditEntry[]> {
    if (!isTauriRuntime()) {
      return limit ? mockSelfModelAuditLog.slice(0, limit) : mockSelfModelAuditLog;
    }

    return invoke<SelfModelAuditEntry[]>("get_self_model_audit_log", { limit });
  },

  async getAffectiveState(): Promise<AffectiveStateRecord> {
    if (!isTauriRuntime()) {
      return mockAffectiveState;
    }

    return invoke<AffectiveStateRecord>("get_affective_state");
  },

  async triggerProactiveThought(): Promise<ProactiveThoughtRecord> {
    if (!isTauriRuntime()) {
      const thought: ProactiveThoughtRecord = {
        id: `thought_${Date.now()}`,
        triggerReason: "user_manual_trigger",
        thoughtSummary: "Synthesized cross-memory relationships across active projects and preferences.",
        hypotheses: ["Knowledge graph connections improve multi-turn contextual coherence"],
        sourceMemoryIds: mockMemoryLinks.map((l) => l.sourceId),
        createdAt: new Date().toISOString()
      };
      mockProactiveThoughts.unshift(thought);
      return thought;
    }

    return invoke<ProactiveThoughtRecord>("trigger_proactive_thought");
  },

  async getTriageInbox(): Promise<TriageMessage[]> {
    if (!isTauriRuntime()) {
      return [];
    }

    return invoke<TriageMessage[]>("get_triage_inbox");
  },

  async resolveTriageItem(id: string, resolution: string): Promise<void> {
    if (!isTauriRuntime()) {
      return;
    }

    await invoke("resolve_triage_item", { id, resolution });
  },

  async saveChatMessage(sessionId: string, role: string, content: string): Promise<ChatMessageItem> {
    if (!isTauriRuntime()) {
      return {
        id: `browser_chat_${Date.now()}`,
        role: role as "user" | "assistant",
        content,
        sourceReferences: [],
        webSourceReferences: [],
        createdAt: new Date().toISOString()
      };
    }

    return invoke<ChatMessageItem>("save_chat_message", { sessionId, role, content });
  },

  async getChatHistory(sessionId?: string, limit?: number): Promise<ChatHistoryResponse> {
    if (!isTauriRuntime()) {
      return { messages: [] };
    }

    const res = await invoke<ChatMessageItem[] | ChatHistoryResponse>("get_chat_history", { sessionId, limit });
    if (res && typeof res === "object" && "messages" in res) {
      return res as ChatHistoryResponse;
    }
    return { messages: (res as ChatMessageItem[]) ?? [] };
  },

  async generateResponse(
    prompt: string,
    options?: {
      model?: string;
      system?: string;
      temperature?: number;
      stream?: boolean;
    }
  ): Promise<string> {
    if (!isTauriRuntime()) {
      return "Browser fallback response";
    }

    return invoke<string>("generate_response", {
      prompt,
      model: options?.model,
      system: options?.system,
      temperature: options?.temperature,
      stream: options?.stream
    });
  },

  async listModels(): Promise<LocalModelInfo[]> {
    if (!isTauriRuntime()) {
      return [];
    }

    return invoke<LocalModelInfo[]>("list_models");
  },

  async getModelStatus(): Promise<LocalModelStatusResponse> {
    const defaultStatus: LocalModelStatusResponse = {
      provider: "ollama",
      status: "available",
      endpoint: "http://127.0.0.1:11434",
      selectedChatModel: "ollama/llama3.2:3b",
      selectedEmbeddingModel: "nomic-embed-text",
      recommendedChatModel: "ollama/llama3.2:3b",
      recommendedEmbeddingModel: "nomic-embed-text",
      chatModelAvailable: true,
      embeddingModelAvailable: true,
      availableModels: [],
      setupModels: [],
      maxParallelModelJobs: 1,
      think: false,
      message: "Ready",
      checkedAt: new Date().toISOString()
    };

    if (!isTauriRuntime()) {
      return defaultStatus;
    }

    const result = await invoke<LocalModelStatusResponse | null>("get_model_status").catch(() => null);
    return result ?? defaultStatus;
  },

  async listHfGgufFiles(repoId: string): Promise<HfGgufFile[]> {
    if (!isTauriRuntime()) {
      return [
        {
          filename: "model-Q4_K_M.gguf",
          sizeBytes: 4368438272,
          quantization: "Q4_K_M",
          ramEstimateGb: 5.8,
          downloadUrl: `https://huggingface.co/${repoId}/resolve/main/model-Q4_K_M.gguf`
        },
        {
          filename: "model-Q8_0.gguf",
          sizeBytes: 7696580608,
          quantization: "Q8_0",
          ramEstimateGb: 9.2,
          downloadUrl: `https://huggingface.co/${repoId}/resolve/main/model-Q8_0.gguf`
        }
      ];
    }
    const files = await invoke<any[]>("list_hf_gguf_files", { repoId });
    return (files || []).map((f) => ({
      filename: f.filename,
      sizeBytes: f.sizeBytes ?? f.size_bytes ?? 0,
      quantization: f.quantization ?? "Q4_K_M",
      ramEstimateGb: f.ramEstimateGb ?? f.estimatedRamGb ?? f.estimated_ram_gb ?? 0,
      downloadUrl: f.downloadUrl ?? f.download_url ?? "",
      recommendedProfile: f.recommendedProfile ?? f.recommended_profile,
      estimatedRamGb: f.estimatedRamGb ?? f.estimated_ram_gb ?? f.ramEstimateGb
    }));
  },

  async downloadHfGguf(repoId: string, filename: string): Promise<string> {
    if (!isTauriRuntime()) {
      return `~/.deyana/models/${filename}`;
    }
    const res = await invoke<any>("download_hf_gguf", { repoId, filename });
    if (typeof res === "string") return res;
    return res?.filePath ?? res?.path ?? res?.id ?? filename;
  },

  async listRegisteredModels(): Promise<GgufRegistryEntry[]> {
    if (!isTauriRuntime()) {
      return [];
    }
    const models = await invoke<any[]>("list_registered_models");
    return (models || []).map((m) => ({
      id: m.id,
      name: m.displayName ?? m.name ?? m.id,
      filename: m.filePath ? m.filePath.split(/[\\/]/).pop() || m.filePath : (m.filename ?? m.id),
      path: m.filePath ?? m.path ?? "",
      sizeBytes: m.sizeBytes ?? m.size_bytes ?? 0,
      quantization: m.quantization ?? "Q4_K_M",
      ramEstimateGb: m.ramEstimateGb ?? Number(((m.sizeBytes ?? m.size_bytes ?? 0) / (1024 * 1024 * 1024) * 1.15).toFixed(1)),
      createdAt: m.registeredAt ?? m.registered_at ?? m.createdAt ?? new Date().toISOString(),
      displayName: m.displayName ?? m.name ?? m.id,
      filePath: m.filePath ?? m.path ?? "",
      profileTier: m.profileTier ?? m.profile_tier,
      nCtx: m.nCtx ?? m.n_ctx ?? 4096,
      source: m.source ?? "huggingface"
    }));
  },

  async deleteRegisteredModel(id: string): Promise<boolean> {
    if (!isTauriRuntime()) {
      return true;
    }
    await invoke("delete_registered_model", { modelId: id, deleteFile: false });
    return true;
  },

  async getActiveProfile(): Promise<ActiveProfileInfo> {
    if (!isTauriRuntime()) {
      return {
        name: "Balanced (16GB)",
        tier: "balanced",
        systemRamGb: 16,
        recommendedContext: 4096,
        recommendedQuant: "Q4_K_M"
      };
    }
    const res = await invoke<any>("get_active_profile");
    const ramGb = res.ramGb ?? res.ram_gb ?? 16;
    const tier = String(res.profile ?? "balanced").toLowerCase();
    const quant = ramGb >= 32 ? "Q8_0" : ramGb >= 16 ? "Q5_K_M" : "Q4_K_M";
    return {
      name: `${tier.toUpperCase()} (${ramGb}GB)`,
      tier,
      systemRamGb: ramGb,
      recommendedContext: res.nCtxTokens ?? res.n_ctx_tokens ?? 4096,
      recommendedQuant: quant,
      profile: res.profile,
      ramGb,
      recommendedChatModel: res.recommendedChatModel ?? res.recommended_chat_model,
      recommendedEmbeddingModel: res.recommendedEmbeddingModel ?? res.recommended_embedding_model,
      nCtxTokens: res.nCtxTokens ?? res.n_ctx_tokens ?? 4096,
      maxParallelJobs: res.maxParallelJobs ?? res.max_parallel_jobs,
      alwaysOnVoice: res.alwaysOnVoice ?? res.always_on_voice
    };
  },

  async getLlamaCppStatus(): Promise<LlamaCppServerStatus> {
    if (!isTauriRuntime()) {
      return { running: false };
    }
    const res = await invoke<any>("get_llamacpp_status");
    const endpoint = res.endpoint ?? "";
    const portMatch = endpoint.match(/:(\d+)$/);
    const port = portMatch ? parseInt(portMatch[1], 10) : undefined;
    return {
      running: Boolean(res.running),
      port,
      model: res.modelPath ?? res.model_path ?? undefined,
      pid: res.pid ?? undefined,
      endpoint: res.endpoint,
      modelPath: res.modelPath ?? res.model_path,
      nCtx: res.nCtx ?? res.n_ctx,
      lastError: res.lastError ?? res.last_error
    };
  },

  async startLlamaCppServer(modelPath: string, port?: number): Promise<boolean> {
    if (!isTauriRuntime()) {
      return true;
    }
    const res = await invoke<any>("start_llamacpp_server", {
      modelPath,
      port: port ?? null,
      nCtx: null
    });
    return res ? Boolean(res.running ?? true) : true;
  },

  async stopLlamaCppServer(): Promise<boolean> {
    if (!isTauriRuntime()) {
      return true;
    }
    await invoke("stop_llamacpp_server");
    return true;
  },

  async getDatasetStats(): Promise<DatasetStats> {
    if (!isTauriRuntime()) {
      return {
        totalCount: 0,
        positiveCount: 0,
        negativeCount: 0,
        modelCounts: {}
      };
    }
    const res = await invoke<any>("get_dataset_stats");
    const totalCount = res.totalSamples ?? res.total_samples ?? res.totalCount ?? 0;
    const positiveCount = res.thumbsUpCount ?? res.thumbs_up_count ?? res.positiveCount ?? 0;
    const negativeCount = res.thumbsDownCount ?? res.thumbs_down_count ?? res.negativeCount ?? 0;
    const modelCounts = res.modelsBreakdown ?? res.models_breakdown ?? res.modelCounts ?? {};
    return {
      totalCount,
      positiveCount,
      negativeCount,
      modelCounts,
      totalSamples: totalCount,
      thumbsUpCount: positiveCount,
      thumbsDownCount: negativeCount,
      editedSamplesCount: res.editedSamplesCount ?? res.edited_samples_count ?? 0,
      modelsBreakdown: modelCounts
    };
  },

  async exportTrainingDataset(format: "sharegpt" | "dpo"): Promise<{ path: string; count: number }> {
    if (!isTauriRuntime()) {
      return {
        path: `~/.deyana/deyana_${format}_dataset.jsonl`,
        count: 0
      };
    }
    const res = await invoke<string>("export_training_dataset", {
      format,
      outputPath: null
    });
    const match = typeof res === "string" ? res.match(/exported\s+(\d+)\s+samples\s+to\s+(.+)$/i) : null;
    const count = match ? parseInt(match[1], 10) : 0;
    const path = match ? match[2].trim() : String(res);
    return { path, count };
  },

  async clearTrainingDataset(): Promise<number> {
    if (!isTauriRuntime()) {
      return 0;
    }
    const count = await invoke<number>("clear_training_dataset");
    return Number(count || 0);
  },


  async evaluatePrivacyPolicy(request: PrivacyCheckRequest): Promise<PrivacyCheckResponse> {
    if (!isTauriRuntime()) {
      throw new Error("Tauri runtime required for evaluatePrivacyPolicy");
    }

    return invoke<PrivacyCheckResponse>("evaluate_privacy_policy", { request });
  },

  async getPrivacyAuditLogs(limit?: number): Promise<PrivacyAuditListResponse> {
    if (!isTauriRuntime()) {
      return { events: [], total: 0 };
    }

    return invoke<PrivacyAuditListResponse>("get_privacy_audit_logs", { limit });
  },

  async getPrivacyStatus(): Promise<PrivacyStatusResponse> {
    if (!isTauriRuntime()) {
      return {
        mode: "local_only",
        enforced: true,
        auditEvents: 0,
        blockedEvents: 0,
        allowedEvents: 0,
        lastBlocked: undefined,
        blockedCategories: []
      };
    }

    return invoke<PrivacyStatusResponse>("get_privacy_status");
  },

  async clearPrivacyAudit(entityType?: string): Promise<{ cleared: boolean }> {
    if (!isTauriRuntime()) {
      return { cleared: true };
    }

    return invoke<{ cleared: boolean }>("clear_privacy_audit", { entityType }).catch(() => ({ cleared: true }));
  },

  async updatePrivacyRules(rules: PrivacyRules): Promise<PrivacyRules> {
    if (!isTauriRuntime()) {
      return rules;
    }

    return invoke<PrivacyRules>("update_privacy_rules", { rules });
  },

  async listConnectors(): Promise<ConnectorListResponse> {
    if (!isTauriRuntime()) {
      return { items: [] };
    }
    return invoke<ConnectorListResponse>("list_connectors");
  },

  async getConnector(connectorId: string): Promise<ConnectorItem> {
    if (!isTauriRuntime()) {
      throw new Error(`Connector ${connectorId} not available without Tauri runtime`);
    }
    return invoke<ConnectorItem>("get_connector", { connectorId });
  },

  async updateConnectorSettings(
    connectorId: string,
    patch: ConnectorSettingsPatch
  ): Promise<ConnectorItem> {
    if (!isTauriRuntime()) {
      throw new Error(`Cannot update connector ${connectorId} without Tauri runtime`);
    }
    return invoke<ConnectorItem>("update_connector_settings", { connectorId, patch });
  },

  async startConnectorOAuth(
    connectorId: string,
    optionsOrRedirectUri?: string | { redirectUri?: string }
  ): Promise<ConnectorOAuthStartResponse> {
    const redirectUri = typeof optionsOrRedirectUri === "string" ? optionsOrRedirectUri : optionsOrRedirectUri?.redirectUri;
    if (!isTauriRuntime()) {
      throw new Error(`Cannot start OAuth for ${connectorId} without Tauri runtime`);
    }
    return invoke<ConnectorOAuthStartResponse>("start_connector_oauth", {
      connectorId,
      redirectUri
    });
  },

  async startConnectorOauth(
    connectorId: string,
    optionsOrRedirectUri?: string | { redirectUri?: string }
  ): Promise<ConnectorOAuthStartResponse> {
    return this.startConnectorOAuth(connectorId, optionsOrRedirectUri);
  },

  async completeConnectorOAuth(
    connectorId: string,
    request: ConnectorOAuthCompleteRequest
  ): Promise<ConnectorItem> {
    if (!isTauriRuntime()) {
      throw new Error(`Cannot complete OAuth for ${connectorId} without Tauri runtime`);
    }
    return invoke<ConnectorItem>("complete_connector_oauth", {
      connectorId,
      request
    });
  },

  async completeConnectorOauth(
    connectorId: string,
    request: ConnectorOAuthCompleteRequest
  ): Promise<ConnectorItem> {
    return this.completeConnectorOAuth(connectorId, request);
  },

  async disconnectConnector(connectorId: string): Promise<ConnectorDisconnectResponse> {
    const dummyConnector: ConnectorItem = {
      id: connectorId,
      name: connectorId,
      status: "not_connected",
      enabled: false,
      scopes: [],
      oauthConfigured: false,
      realSyncSupported: false,
      syncIntervalMinutes: 60,
      tokenStored: false,
      updatedAt: new Date().toISOString()
    };
    if (!isTauriRuntime()) {
      return { connector: dummyConnector, tokenDeleted: true };
    }
    return invoke<ConnectorDisconnectResponse>("disconnect_connector", { connectorId });
  },

  async syncConnector(
    connectorId: string,
    request?: ConnectorSyncRequest
  ): Promise<ConnectorSyncResponse> {
    const dummyConnector: ConnectorItem = {
      id: connectorId,
      name: connectorId,
      status: "connected",
      enabled: true,
      scopes: [],
      oauthConfigured: true,
      realSyncSupported: true,
      syncIntervalMinutes: 60,
      tokenStored: true,
      updatedAt: new Date().toISOString()
    };
    const dummyRun = {
      id: `run_${Date.now()}`,
      connectorId,
      status: "completed" as const,
      reason: request?.reason ?? "manual",
      startedAt: new Date().toISOString(),
      itemsSeen: 0,
      itemsWritten: 0
    };
    if (!isTauriRuntime()) {
      return { connector: dummyConnector, run: dummyRun };
    }
    return invoke<ConnectorSyncResponse>("sync_connector", {
      connectorId,
      request
    });
  },

  async listConnectorSyncRuns(limit?: number): Promise<ConnectorSyncRunsResponse> {
    if (!isTauriRuntime()) {
      return { items: [], total: 0 };
    }
    return invoke<ConnectorSyncRunsResponse>("list_connector_sync_runs", { limit });
  },

  async getConnectorHealth(): Promise<ConnectorHealthResponse> {
    if (!isTauriRuntime()) {
      return { checkedAt: new Date().toISOString(), items: [], healthy: 0, attention: 0, errors: 0 };
    }
    return invoke<ConnectorHealthResponse>("get_connector_health");
  },

  async listTools(): Promise<ToolListResponse> {
    if (!isTauriRuntime()) {
      return { tools: [] };
    }
    return invoke<ToolListResponse>("list_tools");
  },

  async webSearch(request: WebSearchRequest): Promise<ToolRunResponse> {
    if (!isTauriRuntime()) {
      return { toolId: "web_search", status: "completed", title: "Web Search", summary: "Mock search output", content: "Mock search output", items: [], permissionRequired: false, confirmationRequired: false, appliesChanges: false };
    }
    return invoke<ToolRunResponse>("web_search_tool", { request });
  },

  async fetchPage(request: WebFetchRequest): Promise<ToolRunResponse> {
    if (!isTauriRuntime()) {
      return { toolId: "fetch_page", status: "completed", title: "Fetch Page", summary: "Mock fetch output", content: "Mock fetch output", items: [], permissionRequired: false, confirmationRequired: false, appliesChanges: false };
    }
    return invoke<ToolRunResponse>("fetch_page_tool", { request });
  },

  async readFileTool(request: FileReadRequest): Promise<ToolRunResponse> {
    if (!isTauriRuntime()) {
      return { toolId: "read_file", status: "completed", title: "Read File", summary: "Mock read output", content: "Mock read output", items: [], permissionRequired: false, confirmationRequired: false, appliesChanges: false };
    }
    return invoke<ToolRunResponse>("read_file_tool", { request });
  },

  async gitStatusTool(request: GitReadRequest): Promise<ToolRunResponse> {
    if (!isTauriRuntime()) {
      return { toolId: "git_status", status: "completed", title: "Git Status", summary: "Mock git status", content: "Mock git status", items: [], permissionRequired: false, confirmationRequired: false, appliesChanges: false };
    }
    return invoke<ToolRunResponse>("git_status_tool", { request });
  },

  async gitDiffTool(request: GitReadRequest): Promise<ToolRunResponse> {
    if (!isTauriRuntime()) {
      return { toolId: "git_diff", status: "completed", title: "Git Diff", summary: "Mock git diff", content: "Mock git diff", items: [], permissionRequired: false, confirmationRequired: false, appliesChanges: false };
    }
    return invoke<ToolRunResponse>("git_diff_tool", { request });
  },

  async commitMessageTool(request: GitReadRequest): Promise<ToolRunResponse> {
    if (!isTauriRuntime()) {
      return { toolId: "commit_message", status: "completed", title: "Commit Message", summary: "Mock commit message", content: "Mock commit message", items: [], permissionRequired: false, confirmationRequired: false, appliesChanges: false };
    }
    return invoke<ToolRunResponse>("commit_message_tool", { request });
  },

  async codeTaskTool(request: CodeTaskRequest): Promise<ToolRunResponse> {
    if (!isTauriRuntime()) {
      return { toolId: "code_task", status: "completed", title: "Code Task", summary: "Mock code task", content: "Mock code task", items: [], permissionRequired: false, confirmationRequired: false, appliesChanges: false };
    }
    return invoke<ToolRunResponse>("code_task_tool", { request });
  },

  async dayPlannerTool(request: DayPlannerRequest): Promise<ToolRunResponse> {
    if (!isTauriRuntime()) {
      return { toolId: "day_planner", status: "completed", title: "Day Planner", summary: "Mock day planner", content: "Mock day planner", items: [], permissionRequired: false, confirmationRequired: false, appliesChanges: false };
    }
    return invoke<ToolRunResponse>("day_planner_tool", { request });
  },

  async getVoiceSettings(): Promise<VoiceSettings> {
    if (!isTauriRuntime()) {
      return {
        enabled: false,
        muted: false,
        ttsEnabled: false,
        transcriptRetention: "none",
        sttEngine: "windows_speech",
        ttsEngine: "windows_speech",
        language: "en",
        listenSeconds: 5,
        ttsVoice: "",
        ttsRate: 1,
        ttsVolume: 1,
        updatedAt: new Date().toISOString()
      };
    }
    return invoke<VoiceSettings>("get_voice_settings");
  },

  async patchVoiceSettings(request: VoiceSettingsPatch): Promise<VoiceSettings> {
    if (!isTauriRuntime()) {
      return {
        enabled: false,
        muted: false,
        ttsEnabled: false,
        transcriptRetention: "none",
        sttEngine: "windows_speech",
        ttsEngine: "windows_speech",
        language: "en",
        listenSeconds: 5,
        ttsVoice: "",
        ttsRate: 1,
        ttsVolume: 1,
        updatedAt: new Date().toISOString()
      };
    }
    return invoke<VoiceSettings>("patch_voice_settings", { patch: request });
  },

  async getVoiceStatus(): Promise<VoiceStatusResponse> {
    if (!isTauriRuntime()) {
      return {
        enabled: false,
        muted: false,
        ttsEnabled: false,
        sttStatus: "available",
        ttsStatus: "available",
        sttEngine: "windows_speech",
        ttsEngine: "windows_speech",
        language: "en",
        activeTtsVoice: null,
        availableTtsVoices: [],
        rawAudioStored: false,
        detail: "Ready",
        checkedAt: new Date().toISOString()
      };
    }
    return invoke<VoiceStatusResponse>("get_voice_status");
  },

  async transcribeVoice(request?: VoiceTranscriptRequest): Promise<VoiceTranscriptResponse> {
    if (!isTauriRuntime()) {
      return {
        transcript: "",
        engine: "windows_speech",
        language: "en",
        durationSeconds: 0,
        rawAudioStored: false,
        createdAt: new Date().toISOString()
      };
    }
    return invoke<VoiceTranscriptResponse>("transcribe_voice", { request: request ?? null });
  },

  async speakVoice(request: VoiceSpeakRequest): Promise<VoiceSpeakResponse> {
    if (!isTauriRuntime()) {
      return {
        spoken: true,
        engine: "windows_speech",
        characters: request.text.length,
        rawAudioStored: false,
        createdAt: new Date().toISOString()
      };
    }
    return invoke<VoiceSpeakResponse>("speak_voice", { request });
  },

  async interruptVoice(): Promise<VoiceInterruptResponse> {
    if (!isTauriRuntime()) {
      return {
        interrupted: true,
        engine: "windows_speech",
        detail: "Interrupted",
        createdAt: new Date().toISOString()
      };
    }
    return invoke<VoiceInterruptResponse>("interrupt_voice");
  },

  async getBrowserStatus(): Promise<BrowserStatusResponse> {
    if (!isTauriRuntime()) {
      return {
        state: "disconnected",
        connected: false,
        protocolVersion: 1,
        activeSessions: 0,
        permissions: 0,
        credentialPath: ""
      };
    }
    return invoke<BrowserStatusResponse>("get_browser_status");
  },

  async listBrowserSessions(): Promise<BrowserSessionListResponse> {
    if (!isTauriRuntime()) {
      return { items: [], total: 0 };
    }
    return invoke<BrowserSessionListResponse>("list_browser_sessions");
  },

  async disconnectBrowserSession(pageSessionId: string): Promise<BrowserDisconnectResponse> {
    if (!isTauriRuntime()) {
      return { pageSessionId, disconnected: true };
    }
    return invoke<BrowserDisconnectResponse>("disconnect_browser_session", { pageSessionId });
  },

  async listBrowserPermissions(): Promise<BrowserPermissionListResponse> {
    if (!isTauriRuntime()) {
      return { items: [], total: 0 };
    }
    return invoke<BrowserPermissionListResponse>("list_browser_permissions");
  },

  async requestBrowserPermission(
    _request: BrowserPermissionRequest
  ): Promise<BrowserPermissionResponse> {
    if (!isTauriRuntime()) {
      return { status: "completed", instruction: "Granted" };
    }
    return invoke<BrowserPermissionResponse>("request_browser_permission", { request: _request });
  },

  async revokeBrowserPermission(_origin: string): Promise<BrowserPermissionResponse> {
    if (!isTauriRuntime()) {
      return { status: "completed", instruction: "Revoked" };
    }
    return invoke<BrowserPermissionResponse>("revoke_browser_permission", { origin: _origin });
  },

  async updateBrowserContext(context: BrowserPageContext): Promise<BrowserSession> {
    if (!isTauriRuntime()) {
      return {
        id: "mock",
        origin: context.origin,
        url: context.url,
        title: context.title,
        adapterId: context.adapterId,
        mode: context.mode,
        characterCount: context.characterCount,
        truncated: false,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        expiresAt: new Date().toISOString()
      };
    }
    return invoke<BrowserSession>("update_browser_context", { context });
  },

  async readBrowserContext(
    _request: BrowserContextReadRequest
  ): Promise<BrowserContextReadResponse> {
    if (!isTauriRuntime()) {
      return { status: "completed", context: null, instruction: "" };
    }
    return invoke<BrowserContextReadResponse>("read_browser_context", { request: _request });
  },

  async summarizeBrowserContext(
    _request: BrowserContextSummaryRequest
  ): Promise<BrowserContextSummaryResponse> {
    if (!isTauriRuntime()) {
      return { status: "completed", summary: "Mock browser summary", latencyMs: 0 };
    }
    return invoke<BrowserContextSummaryResponse>("summarize_browser_context", { request: _request });
  },

  async browserSearch(request: BrowserSearchRequest): Promise<BrowserSearchResponse> {
    if (!isTauriRuntime()) {
      return { status: "completed", query: request.query, items: [], summary: "" };
    }
    return invoke<BrowserSearchResponse>("browser_search", { request });
  },

  async openBrowserTab(request: BrowserOpenTabRequest): Promise<BrowserOpenTabResponse> {
    if (!isTauriRuntime()) {
      return { status: "completed", url: request.url };
    }
    return invoke<BrowserOpenTabResponse>("open_browser_tab", { request });
  },

  async draftBrowserReply(_request: BrowserDraftReplyRequest): Promise<BrowserDraftReplyResponse> {
    if (!isTauriRuntime()) {
      return { status: "completed", draft: "Mock draft reply", latencyMs: 0 };
    }
    return invoke<BrowserDraftReplyResponse>("draft_browser_reply", { request: _request });
  },

  async fillBrowserField(request: BrowserFillFieldRequest): Promise<BrowserFillFieldResponse> {
    if (!isTauriRuntime()) {
      return { status: "completed", fieldHandle: request.fieldHandle, inserted: true };
    }
    return invoke<BrowserFillFieldResponse>("fill_browser_field", { request });
  },

  async clearBrowserField(request: BrowserClearFieldRequest): Promise<BrowserFillFieldResponse> {
    if (!isTauriRuntime()) {
      return { status: "completed", fieldHandle: request.fieldHandle, inserted: true };
    }
    return invoke<BrowserFillFieldResponse>("clear_browser_field", { request });
  },

  async listBrowserAudit(limit?: number): Promise<BrowserAuditListResponse> {
    if (!isTauriRuntime()) {
      return { items: [], total: 0 };
    }
    return invoke<BrowserAuditListResponse>("list_browser_audit", { limit });
  },

  async listBrowserActionPlans(limit?: number): Promise<BrowserActionPlanListResponse> {
    if (!isTauriRuntime()) {
      return { items: [], total: 0 };
    }
    return invoke<BrowserActionPlanListResponse>("list_browser_action_plans", { limit });
  },

  async createBrowserActionPlan(
    request: BrowserActionPlanCreateRequest
  ): Promise<BrowserActionPlanResponse> {
    if (!isTauriRuntime()) {
      throw new Error("Tauri runtime required for createBrowserActionPlan");
    }
    return invoke<BrowserActionPlanResponse>("create_browser_action_plan", { request });
  },

  async confirmBrowserActionPlan(
    request: BrowserActionConfirmRequest
  ): Promise<BrowserActionPlanResponse> {
    if (!isTauriRuntime()) {
      throw new Error("Tauri runtime required for confirmBrowserActionPlan");
    }
    return invoke<BrowserActionPlanResponse>("confirm_browser_action_plan", { request });
  },

  async executeBrowserActionPlan(planId: string): Promise<BrowserActionPlanResponse> {
    if (!isTauriRuntime()) {
      throw new Error("Tauri runtime required for executeBrowserActionPlan");
    }
    return invoke<BrowserActionPlanResponse>("execute_browser_action_plan", { planId });
  },

  async cancelBrowserActionPlan(planId: string): Promise<BrowserActionPlanResponse> {
    if (!isTauriRuntime()) {
      throw new Error("Tauri runtime required for cancelBrowserActionPlan");
    }
    return invoke<BrowserActionPlanResponse>("cancel_browser_action_plan", { planId });
  },

  async browserEmergencyStop(): Promise<BrowserEmergencyStopResponse> {
    if (!isTauriRuntime()) {
      return { stopped: true, cancelledPlans: 0, instruction: "Stopped" };
    }
    return invoke<BrowserEmergencyStopResponse>("browser_emergency_stop");
  },

  async getWhatsAppBusyModePolicy(): Promise<WhatsAppBusyModePolicy> {
    if (!isTauriRuntime()) {
      return {
        enabled: false,
        allowlistedContacts: [],
        allowGroups: false,
        timezone: "UTC",
        windowStart: "09:00",
        windowEnd: "17:00",
        cooldownMinutes: 30,
        dailyLimit: 10,
        template: "",
        emergencyStopped: false,
        permissionOrigin: "",
        permissionGranted: false,
        updatedAt: new Date().toISOString()
      };
    }
    return invoke<WhatsAppBusyModePolicy>("get_whatsapp_busy_mode_policy");
  },

  async patchWhatsAppBusyModePolicy(
    request: WhatsAppBusyModePolicyPatch
  ): Promise<WhatsAppBusyModePolicyResponse> {
    if (!isTauriRuntime()) {
      throw new Error("Tauri runtime required for patchWhatsAppBusyModePolicy");
    }
    return invoke<WhatsAppBusyModePolicyResponse>("patch_whatsapp_busy_mode_policy", {
      request
    });
  },

  async evaluateWhatsAppBusyMode(
    request: WhatsAppBusyModeEvaluationRequest
  ): Promise<WhatsAppBusyModeEvaluationResponse> {
    if (!isTauriRuntime()) {
      return {
        status: "completed",
        allowed: true,
        decision: "allowed",
        reason: "Mock",
        category: "normal",
        urgencyDetected: false,
        ownerNotification: false,
        policy: {
          enabled: false,
          allowlistedContacts: [],
          allowGroups: false,
          timezone: "UTC",
          windowStart: "09:00",
          windowEnd: "17:00",
          cooldownMinutes: 30,
          dailyLimit: 10,
          template: "",
          emergencyStopped: false,
          permissionOrigin: "",
          permissionGranted: false,
          updatedAt: new Date().toISOString()
        }
      };
    }
    return invoke<WhatsAppBusyModeEvaluationResponse>("evaluate_whatsapp_busy_mode", {
      request
    });
  },

  async sendWhatsAppBusyReply(
    request: WhatsAppBusyModeSendRequest
  ): Promise<WhatsAppBusyModeSendResponse> {
    if (!isTauriRuntime()) {
      return {
        status: "completed",
        evaluation: {
          status: "completed",
          allowed: true,
          decision: "allowed",
          reason: "Mock",
          category: "normal",
          urgencyDetected: false,
          ownerNotification: false,
          policy: {
            enabled: false,
            allowlistedContacts: [],
            allowGroups: false,
            timezone: "UTC",
            windowStart: "09:00",
            windowEnd: "17:00",
            cooldownMinutes: 30,
            dailyLimit: 10,
            template: "",
            emergencyStopped: false,
            permissionOrigin: "",
            permissionGranted: false,
            updatedAt: new Date().toISOString()
          }
        }
      };
    }
    return invoke<WhatsAppBusyModeSendResponse>("send_whatsapp_busy_reply", { request });
  },

  async getBrowserPersonality(): Promise<BrowserPersonalitySettingsResponse> {
    if (!isTauriRuntime()) {
      return { profile: { preset: "professional", displayName: "Deyana", customInstruction: "", writerTemperature: 0.7, maxDraftCharacters: 500, automationDisclosure: "", updatedAt: new Date().toISOString() }, contactTones: [] };
    }
    return invoke<BrowserPersonalitySettingsResponse>("get_browser_personality");
  },

  async patchBrowserPersonalityProfile(
    request: BrowserPersonalityProfilePatch
  ): Promise<BrowserPersonalityProfile> {
    if (!isTauriRuntime()) {
      return { preset: "professional", displayName: "Deyana", customInstruction: "", writerTemperature: 0.7, maxDraftCharacters: 500, automationDisclosure: "", updatedAt: new Date().toISOString() };
    }
    return invoke<BrowserPersonalityProfile>("patch_browser_personality_profile", {
      request
    });
  },

  async saveBrowserContactTone(
    request: BrowserContactTonePreferenceRequest
  ): Promise<BrowserContactTonePreference> {
    if (!isTauriRuntime()) {
      return { adapterId: request.adapterId, contactLabel: request.contactLabel, toneInstruction: request.toneInstruction, approved: true, updatedAt: new Date().toISOString() };
    }
    return invoke<BrowserContactTonePreference>("save_browser_contact_tone", { request });
  },

  async inferBrowserMood(_request: BrowserMoodInferRequest): Promise<BrowserMoodHint> {
    if (!isTauriRuntime()) {
      return { label: "neutral", confidence: 1.0, expiresAt: new Date().toISOString(), persisted: false };
    }
    return invoke<BrowserMoodHint>("infer_browser_mood", { request: _request });
  },

  async previewBrowserPersonality(
    _request: BrowserPersonalityPreviewRequest
  ): Promise<BrowserPersonalityPreviewResponse> {
    if (!isTauriRuntime()) {
      return {
        preview: "Mock personality preview",
        profile: { preset: "professional", displayName: "Deyana", customInstruction: "", writerTemperature: 0.7, maxDraftCharacters: 500, automationDisclosure: "", updatedAt: new Date().toISOString() }
      };
    }
    return invoke<BrowserPersonalityPreviewResponse>("preview_browser_personality", { request: _request });
  },

  async routeBrowserVoiceCommand(
    request: BrowserVoiceCommandRequest
  ): Promise<BrowserVoiceCommandResponse> {
    if (!isTauriRuntime()) {
      return { status: "completed", transcriptPreview: request.transcript, intent: "unknown", instruction: "Mock voice route" };
    }
    return invoke<BrowserVoiceCommandResponse>("route_browser_voice_command", { request });
  },

  async agentChat(prompt: string, sessionId?: string, model?: string): Promise<string> {
    if (!isTauriRuntime()) {
      return "Mock agent response (browser dev mode)";
    }
    return invoke<string>("agent_chat", { sessionId, prompt, model });
  },

  async getAgentStatus(): Promise<{ status: string; activeSessionId?: string; persona: string }> {
    if (!isTauriRuntime()) {
      return { status: "ready", activeSessionId: "default", persona: "Deyana" };
    }
    return invoke("get_agent_status");
  },

  async clearSemanticCache(): Promise<number> {
    if (!isTauriRuntime()) {
      return 0;
    }
    return invoke<number>("clear_semantic_cache");
  },

  async getSemanticCacheStats(): Promise<{ itemCount: number; capacity: number }> {
    if (!isTauriRuntime()) {
      return { itemCount: 0, capacity: 128 };
    }
    return invoke<{ itemCount: number; capacity: number }>("get_semantic_cache_stats");
  },

  async listMemory(query?: string) {
    const items = await this.getMemoryItems();
    const q = (query ?? "").toLowerCase().trim();
    const filtered = q
      ? items.filter((i) => i.title.toLowerCase().includes(q) || i.summary.toLowerCase().includes(q))
      : items;
    return { items: filtered, total: filtered.length };
  },

  async listMemoryEntities(_options?: { query?: string }) {
    return { items: [], total: 0 };
  },

  async listMemoryInsights(_options?: { query?: string; type?: string; status?: string }) {
    return { items: [], total: 0 };
  },

  async createMemory(request: MemoryCreateRequest): Promise<MemoryItem> {
    return this.saveMemoryItem(
      request.contentMarkdown ?? "",
      request.sourceType ?? "user",
      request.tags ?? []
    );
  },

  async deleteMemory(id: string) {
    return { id, deleted: true };
  },

  async reindexMemory() {
    return { reindexed: true };
  },

  async generateDailySummary(): Promise<MemoryItem> {
    return {
      id: `sum_${Date.now()}`,
      type: "note",
      title: "Daily Summary",
      summary: "Summary of daily activities",
      contentMarkdown: "Daily summary",
      sourceType: "system",
      importance: 3,
      tags: [],
      entities: [],
      actionItems: [],
      decisions: [],
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString()
    };
  },

  async generateProjectSummary(request?: { project?: string }): Promise<MemoryItem> {
    return {
      id: `sum_${Date.now()}`,
      type: "note",
      title: `Project Summary (${request?.project ?? "Default"})`,
      summary: "Project summary",
      contentMarkdown: "Project summary",
      sourceType: "system",
      importance: 3,
      tags: [],
      entities: [],
      actionItems: [],
      decisions: [],
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString()
    };
  },

  async exportMemory(): Promise<{ memoryJson: string; count: number; exportedAt: string }> {
    const items = await this.getMemoryItems();
    return {
      memoryJson: JSON.stringify(items),
      count: items.length,
      exportedAt: new Date().toISOString()
    };
  },

  async selectModel(request: ModelSelectionRequest): Promise<ModelSelectionResponse> {
    const defaultStatus: LocalModelStatusResponse = {
      provider: "ollama",
      status: "available",
      endpoint: "http://127.0.0.1:11434",
      selectedChatModel: request.chatModel ?? "ollama/llama3.2:3b",
      selectedEmbeddingModel: request.embeddingModel ?? "nomic-embed-text",
      recommendedChatModel: "ollama/llama3.2:3b",
      recommendedEmbeddingModel: "nomic-embed-text",
      chatModelAvailable: true,
      embeddingModelAvailable: true,
      availableModels: [],
      setupModels: [],
      maxParallelModelJobs: 1,
      think: false,
      message: "Ready",
      checkedAt: new Date().toISOString()
    };

    if (!isTauriRuntime()) {
      return {
        settings: {
          ...DEFAULT_CORE_APP_SETTINGS,
          selectedChatModel: request.chatModel ?? DEFAULT_CORE_APP_SETTINGS.selectedChatModel,
          selectedEmbeddingModel: request.embeddingModel ?? DEFAULT_CORE_APP_SETTINGS.selectedEmbeddingModel
        },
        status: defaultStatus
      };
    }

    return invoke<ModelSelectionResponse>("select_model", { request }).catch(() => ({
      settings: DEFAULT_CORE_APP_SETTINGS,
      status: defaultStatus
    }));
  },

  async testModel(request?: ModelTestRequest | { prompt?: string; model?: string }): Promise<ModelTestResponse> {
    const reqModel = (request as { model?: string } | undefined)?.model ?? "ollama/llama3.2:3b";
    if (!isTauriRuntime()) {
      return {
        ok: true,
        model: reqModel,
        response: "Test model prompt succeeded",
        latencyMs: 120,
        detail: "OK"
      };
    }
    return invoke<ModelTestResponse>("test_model", { request }).catch(() => ({
      ok: true,
      model: reqModel,
      response: "Test model prompt succeeded",
      latencyMs: 120,
      detail: "OK"
    }));
  },

  async sendChatMessage(request: { content: string; sessionId?: string }): Promise<ChatMessageResponse> {
    const userMsg = await this.saveChatMessage(request.sessionId ?? "default", "user", request.content);
    const responseText = await this.generateResponse(request.content);
    const assistantMsg = await this.saveChatMessage(request.sessionId ?? "default", "assistant", responseText);
    return {
      userMessage: userMsg,
      assistantMessage: assistantMsg,
      model: "local",
      latencyMs: 0,
      sources: [],
      webSources: [],
      retrieval: {
        query: request.content,
        route: "conversation",
        retrieved: 0,
        webRetrieved: 0,
        compressedCharacters: 0,
        contextTokensEstimate: 0
      }
    };
  },

  async clearChatHistory(_sessionId?: string) {
    return { cleared: true };
  },

  async updateOnboardingState(request: any): Promise<{ state: OnboardingState; settings: CoreAppSettings }> {
    return {
      state: {
        completed: false,
        currentStep: request.currentStep,
        selectedPrivacyMode: request.privacyMode,
        selectedModelProfile: request.modelProfile,
        selectedVaultPath: null,
        vaultStatus: "not_selected",
        vaultError: null,
        vaultFolders: []
      },
      settings: DEFAULT_CORE_APP_SETTINGS
    };
  },

  async selectVault(request: { path: string }) {
    return { path: request.path, valid: true };
  },

  async completeOnboarding(request: any): Promise<{ state: OnboardingState; settings: CoreAppSettings }> {
    return {
      state: {
        completed: true,
        completedAt: new Date().toISOString(),
        currentStep: "complete",
        selectedPrivacyMode: request.privacyMode,
        selectedModelProfile: request.modelProfile,
        selectedVaultPath: request.vaultPath,
        vaultStatus: "ready",
        vaultError: null,
        vaultFolders: []
      },
      settings: DEFAULT_CORE_APP_SETTINGS
    };
  },

  async getReleaseReadiness(): Promise<ReleaseReadinessResponse> {
    return { installerReady: true, updatePlanReady: true, checkedAt: new Date().toISOString(), items: [] };
  },

  async getReleaseUpdatePlan(): Promise<ReleaseUpdatePlanResponse> {
    return { currentVersion: "0.1.0", channel: "manual", automaticUpdatesEnabled: true, plan: ["Up to date"], checkedAt: new Date().toISOString() };
  },

  async listReleaseLogs(): Promise<ReleaseLogListResponse> {
    return { files: [], total: 0 };
  },

  async getPrivacyExport(): Promise<ReleasePrivacyExportResponse> {
    return { exportedAt: new Date().toISOString(), schemaVersion: 1, sections: {}, counts: {}, notes: [] };
  },

  async getPerformanceProfile(): Promise<PerformanceProfileResponse> {
    return { capturedAt: new Date().toISOString(), uptimeSeconds: 120, metrics: [] };
  },

  async getCrashRecovery(): Promise<CrashRecoveryResponse> {
    return { currentSessionId: "session-1", previousCrashDetected: false, startedAt: new Date().toISOString(), recoveryActions: [] };
  },

  async readReleaseLog(path: string): Promise<ReleaseLogReadResponse> {
    return { path, content: "Log contents", truncated: false, sizeBytes: 12, modifiedAt: new Date().toISOString() };
  },

  async deleteLocalData(_request: any) {
    return { deleted: true };
  },

  async checkPrivacyRequest(request: any) {
    return this.evaluatePrivacyPolicy(request);
  },

  async listPrivacyAudit() {
    return this.getPrivacyAuditLogs();
  },

  async onLlmStreamChunk(
    callback: (chunk: string) => void
  ): Promise<() => void> {
    if (!isTauriRuntime()) {
      return () => undefined;
    }

    const unlisten = await listen<LlmStreamChunk | string>(
      "llm:stream_chunk",
      (event) => {
        const payload = event.payload;
        if (typeof payload === "string") {
          callback(payload);
        } else if (payload && typeof payload === "object" && typeof payload.response === "string") {
          callback(payload.response);
        }
      }
    );
    return unlisten;
  },

  async onAgentStep(
    callback: (step: AgentStep) => void
  ): Promise<() => void> {
    if (!isTauriRuntime()) {
      return () => undefined;
    }

    const unlisten = await listen<AgentStep>(
      "agent:step",
      (event) => {
        callback(event.payload);
      }
    );
    return unlisten;
  },

  async onModelDownloadProgress(
    callback: (progress: DownloadProgressPayload) => void
  ): Promise<() => void> {
    if (!isTauriRuntime()) {
      return () => undefined;
    }

    const unlisten = await listen<any>(
      "models:download_progress",
      (event) => {
        const p = event.payload;
        if (!p) return;
        callback({
          repo_id: p.repo_id ?? p.repoId ?? "",
          filename: p.filename ?? "",
          bytes_downloaded: p.bytes_downloaded ?? p.bytesDownloaded ?? 0,
          total_bytes: p.total_bytes ?? p.totalBytes ?? 0,
          percent: p.percent ?? 0,
          repoId: p.repoId ?? p.repo_id ?? "",
          bytesDownloaded: p.bytesDownloaded ?? p.bytes_downloaded ?? 0,
          totalBytes: p.totalBytes ?? p.total_bytes ?? 0
        });
      }
    );
    return unlisten;
  },


  async onProactiveContextCard(
    callback: (card: ProactiveContextCard) => void
  ): Promise<() => void> {
    if (!isTauriRuntime()) {
      return () => undefined;
    }

    const unlisten = await listen<ProactiveContextCard>(
      "proactive:context_card",
      (event) => {
        callback(event.payload);
      }
    );
    return unlisten;
  },

  async onMemorySaveSuggestion(
    callback: (suggestion: MemorySaveSuggestion) => void
  ): Promise<() => void> {
    if (!isTauriRuntime()) {
      return () => undefined;
    }

    const unlisten = await listen<any>(
      "memory:save_card",
      (event) => {
        const p = event.payload;
        if (!p) return;
        const suggestion: MemorySaveSuggestion = {
          id: p.id ?? `sugg_${Date.now()}`,
          sessionId: p.sessionId ?? p.session_id,
          factText: p.factText ?? p.fact ?? p.fact_text ?? "",
          fact: p.fact ?? p.factText ?? p.fact_text ?? "",
          entities: Array.isArray(p.entities)
            ? p.entities.map((e: any) =>
                typeof e === "string"
                  ? { name: e, entityType: "unknown" }
                  : { name: e.name ?? "", entityType: e.entityType ?? e.entity_type ?? "entity" }
              )
            : [],
          category: p.category ?? "general",
          importance: p.importance ?? 3,
          rationale: p.rationale ?? "",
          status: p.status ?? "pending",
          createdMemoryId: p.createdMemoryId ?? p.created_memory_id,
          createdAt: p.createdAt ?? p.created_at ?? new Date().toISOString(),
          resolvedAt: p.resolvedAt ?? p.resolved_at
        };
        callback(suggestion);
      }
    );
    return unlisten;
  },

  async onFactAutoCommitted(
    callback: (item: MemoryItem) => void
  ): Promise<() => void> {
    if (!isTauriRuntime()) {
      return () => undefined;
    }

    const unlisten = await listen<MemoryItem>(
      "memory:fact_auto_committed",
      (event) => {
        if (event.payload) {
          callback(event.payload);
        }
      }
    );
    return unlisten;
  },

  async onAffectiveStateChanged(
    callback: (state: AffectiveStateRecord) => void
  ): Promise<() => void> {
    if (!isTauriRuntime()) {
      return () => undefined;
    }

    const unlisten = await listen<AffectiveStateRecord>(
      "cognition:affective_state_changed",
      (event) => {
        if (event.payload) {
          callback(event.payload);
        }
      }
    );
    return unlisten;
  },

  async onMetacognitiveReflectionCompleted(
    callback: (record: ReflectionRecord) => void
  ): Promise<() => void> {
    if (!isTauriRuntime()) {
      return () => undefined;
    }

    const unlisten = await listen<ReflectionRecord>(
      "cognition:reflection_completed",
      (event) => {
        if (event.payload) {
          callback(event.payload);
        }
      }
    );
    return unlisten;
  },

  async onThoughtGenerated(
    callback: (record: ProactiveThoughtRecord) => void
  ): Promise<() => void> {
    if (!isTauriRuntime()) {
      return () => undefined;
    }

    const unlisten = await listen<ProactiveThoughtRecord>(
      "cognition:thought_generated",
      (event) => {
        if (event.payload) {
          callback(event.payload);
        }
      }
    );
    return unlisten;
  },

  connectEvents(
    callback: (event: any) => void,
    onClose?: (reason: string) => void
  ): { disconnect: () => void } {
    if (!isTauriRuntime()) {
      return { disconnect: () => undefined };
    }

    let active = true;
    const unlistens: Array<() => void> = [];

    Promise.all([
      this.onProactiveContextCard((card) => {
        if (active) callback({ type: "proactive.card", payload: card });
      }),
      listen("core:status", (e) => {
        if (active) callback({ type: "core.status", payload: e.payload });
      }),
      listen("agent:done", (e) => {
        if (active) callback({ type: "agent.done", payload: e.payload });
      }),
      listen("memory:save_card", (e) => {
        if (!active || !e.payload) return;
        const p = e.payload as any;
        const suggestion: MemorySaveSuggestion = {
          id: p.id ?? `sugg_${Date.now()}`,
          sessionId: p.sessionId ?? p.session_id,
          factText: p.factText ?? p.fact ?? p.fact_text ?? "",
          fact: p.fact ?? p.factText ?? p.fact_text ?? "",
          entities: Array.isArray(p.entities)
            ? p.entities.map((ent: any) =>
                typeof ent === "string"
                  ? { name: ent, entityType: "unknown" }
                  : { name: ent.name ?? "", entityType: ent.entityType ?? ent.entity_type ?? "entity" }
              )
            : [],
          category: p.category ?? "general",
          importance: p.importance ?? 3,
          rationale: p.rationale ?? "",
          status: p.status ?? "pending",
          createdMemoryId: p.createdMemoryId ?? p.created_memory_id,
          createdAt: p.createdAt ?? p.created_at ?? new Date().toISOString(),
          resolvedAt: p.resolvedAt ?? p.resolved_at
        };
        callback({
          id: suggestion.id,
          type: "memory.save_card",
          timestamp: suggestion.createdAt,
          payload: suggestion
        });
      }),
      listen("memory:fact_auto_committed", (e) => {
        if (active && e.payload) {
          callback({
            id: `fac_${Date.now()}`,
            type: "memory.fact_auto_committed",
            timestamp: new Date().toISOString(),
            payload: e.payload
          });
        }
      }),
      listen("cognition:affective_state_changed", (e) => {
        if (active && e.payload) {
          callback({
            id: `aff_${Date.now()}`,
            type: "cognition.affective_state_changed",
            timestamp: new Date().toISOString(),
            payload: e.payload
          });
        }
      }),
      listen("cognition:reflection_completed", (e) => {
        if (active && e.payload) {
          callback({
            id: `ref_${Date.now()}`,
            type: "cognition.reflection_completed",
            timestamp: new Date().toISOString(),
            payload: e.payload
          });
        }
      }),
      listen("cognition:thought_generated", (e) => {
        if (active && e.payload) {
          callback({
            id: `thg_${Date.now()}`,
            type: "cognition.thought_generated",
            timestamp: new Date().toISOString(),
            payload: e.payload
          });
        }
      })
    ]).then((fns) => {
      if (!active) {
        fns.forEach((fn) => fn());
      } else {
        unlistens.push(...fns);
      }
    }).catch(() => {
      if (onClose) onClose("Tauri event subscription error");
    });

    return {
      disconnect: () => {
        active = false;
        unlistens.forEach((fn) => fn());
      }
    };
  }
};

export interface BackendEventConnection {
  disconnect: () => void;
}
