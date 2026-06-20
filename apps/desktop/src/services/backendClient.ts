import { coreService } from "@deyana/config";
import type {
  AppCoreEvent,
  BackendHealthResponse,
  BackendStatusResponse,
  ChatHistoryDeleteResponse,
  ChatHistoryResponse,
  ChatMessageRequest,
  ChatMessageResponse,
  ConnectorDisconnectResponse,
  ConnectorItem,
  ConnectorListResponse,
  ConnectorOAuthCompleteRequest,
  ConnectorOAuthStartRequest,
  ConnectorOAuthStartResponse,
  ConnectorSettingsPatch,
  ConnectorSyncRequest,
  ConnectorSyncResponse,
  ConnectorSyncRunsResponse,
  CoreAppSettings,
  DailySummaryRequest,
  LocalModelStatusResponse,
  MemoryCreateRequest,
  MemoryDeleteResponse,
  MemoryEntityListResponse,
  MemoryExportResponse,
  MemoryInsightListResponse,
  MemoryItem,
  MemoryListResponse,
  MemoryReindexResponse,
  MemoryUpdateRequest,
  ModelSelectionRequest,
  ModelSelectionResponse,
  ModelTestRequest,
  ModelTestResponse,
  OnboardingCompleteRequest,
  OnboardingCompleteResponse,
  OnboardingState,
  PrivacyAuditDeleteResponse,
  PrivacyAuditListResponse,
  PrivacyCheckRequest,
  PrivacyCheckResponse,
  PrivacyStatusResponse,
  ProjectSummaryRequest,
  SettingsPatch,
  VaultSelectRequest,
  VaultSelectResponse
} from "@deyana/schemas";

export interface BackendEventConnection {
  disconnect: () => void;
}

type MemoryInsightFilterType = "action_item" | "decision";

export interface MemoryEntityListOptions {
  query?: string;
  sourceType?: string;
  sourceId?: string;
  date?: string;
  limit?: number;
}

export interface MemoryInsightListOptions {
  query?: string;
  type?: MemoryInsightFilterType;
  status?: string;
  sourceType?: string;
  sourceId?: string;
  date?: string;
  limit?: number;
}

function appendFilterParam(params: URLSearchParams, key: string, value?: string | null): void {
  const trimmed = value?.trim();
  if (trimmed) {
    params.set(key, trimmed);
  }
}

function appendListLimit(params: URLSearchParams, limit = 100): void {
  const boundedLimit = Number.isFinite(limit) ? Math.min(200, Math.max(1, Math.floor(limit))) : 100;
  params.set("limit", String(boundedLimit));
}

export const backendClient = {
  async getHealth(timeoutMs = 800): Promise<BackendHealthResponse> {
    const controller = new AbortController();
    const timeout = window.setTimeout(() => controller.abort(), timeoutMs);

    try {
      const response = await fetch(`${coreService.endpoint}/health`, {
        signal: controller.signal
      });
      if (!response.ok) {
        throw new Error(`core health returned ${response.status}`);
      }

      return response.json() as Promise<BackendHealthResponse>;
    } finally {
      window.clearTimeout(timeout);
    }
  },

  async getStatus(timeoutMs = 1000): Promise<BackendStatusResponse> {
    const controller = new AbortController();
    const timeout = window.setTimeout(() => controller.abort(), timeoutMs);

    try {
      const response = await fetch(`${coreService.endpoint}/status`, {
        signal: controller.signal
      });
      if (!response.ok) {
        throw new Error(`core status returned ${response.status}`);
      }

      return response.json() as Promise<BackendStatusResponse>;
    } finally {
      window.clearTimeout(timeout);
    }
  },

  connectEvents(
    onEvent: (event: AppCoreEvent) => void,
    onClose: (reason: string) => void
  ): BackendEventConnection {
    const socket = new WebSocket(coreService.websocketUrl);

    socket.onmessage = (message) => {
      try {
        onEvent(JSON.parse(message.data as string) as AppCoreEvent);
      } catch {
        onClose("Received an invalid backend event");
      }
    };

    socket.onerror = () => onClose("Backend event stream failed");
    socket.onclose = () => onClose("Backend event stream closed");

    return {
      disconnect: () => socket.close()
    };
  },

  async getSettings(): Promise<CoreAppSettings> {
    const response = await fetch(`${coreService.endpoint}/settings`);
    if (!response.ok) {
      throw new Error(`settings returned ${response.status}`);
    }
    return response.json() as Promise<CoreAppSettings>;
  },

  async patchSettings(patch: SettingsPatch): Promise<CoreAppSettings> {
    const response = await fetch(`${coreService.endpoint}/settings`, {
      method: "PATCH",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(patch)
    });
    if (!response.ok) {
      throw new Error(`settings update returned ${response.status}`);
    }
    return response.json() as Promise<CoreAppSettings>;
  },

  async getOnboardingState(): Promise<OnboardingState> {
    const response = await fetch(`${coreService.endpoint}/onboarding/state`);
    if (!response.ok) {
      throw new Error(`onboarding state returned ${response.status}`);
    }
    return response.json() as Promise<OnboardingState>;
  },

  async selectVault(request: VaultSelectRequest): Promise<VaultSelectResponse> {
    const response = await fetch(`${coreService.endpoint}/vault/select`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(request)
    });
    if (!response.ok) {
      const detail = await readErrorDetail(response);
      throw new Error(detail || `vault selection returned ${response.status}`);
    }
    return response.json() as Promise<VaultSelectResponse>;
  },

  async completeOnboarding(
    request: OnboardingCompleteRequest
  ): Promise<OnboardingCompleteResponse> {
    const response = await fetch(`${coreService.endpoint}/onboarding/complete`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(request)
    });
    if (!response.ok) {
      const detail = await readErrorDetail(response);
      throw new Error(detail || `onboarding completion returned ${response.status}`);
    }
    return response.json() as Promise<OnboardingCompleteResponse>;
  },

  async listMemory(query?: string): Promise<MemoryListResponse> {
    const params = new URLSearchParams();
    if (query?.trim()) {
      params.set("query", query.trim());
    }
    params.set("limit", "20");
    const suffix = params.toString() ? `?${params.toString()}` : "";
    const response = await fetch(`${coreService.endpoint}/memory${suffix}`);
    if (!response.ok) {
      throw new Error(`memory list returned ${response.status}`);
    }
    return response.json() as Promise<MemoryListResponse>;
  },

  async createMemory(request: MemoryCreateRequest): Promise<MemoryItem> {
    const response = await fetch(`${coreService.endpoint}/memory`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(request)
    });
    if (!response.ok) {
      const detail = await readErrorDetail(response);
      throw new Error(detail || `memory create returned ${response.status}`);
    }
    return response.json() as Promise<MemoryItem>;
  },

  async updateMemory(id: string, request: MemoryUpdateRequest): Promise<MemoryItem> {
    const response = await fetch(`${coreService.endpoint}/memory/${id}`, {
      method: "PATCH",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(request)
    });
    if (!response.ok) {
      const detail = await readErrorDetail(response);
      throw new Error(detail || `memory update returned ${response.status}`);
    }
    return response.json() as Promise<MemoryItem>;
  },

  async deleteMemory(id: string): Promise<MemoryDeleteResponse> {
    const response = await fetch(`${coreService.endpoint}/memory/${id}`, {
      method: "DELETE"
    });
    if (!response.ok) {
      const detail = await readErrorDetail(response);
      throw new Error(detail || `memory delete returned ${response.status}`);
    }
    return response.json() as Promise<MemoryDeleteResponse>;
  },

  async reindexMemory(): Promise<MemoryReindexResponse> {
    const response = await fetch(`${coreService.endpoint}/memory/reindex`, {
      method: "POST"
    });
    if (!response.ok) {
      throw new Error(`memory reindex returned ${response.status}`);
    }
    return response.json() as Promise<MemoryReindexResponse>;
  },

  async generateDailySummary(request: DailySummaryRequest = {}): Promise<MemoryItem> {
    const response = await fetch(`${coreService.endpoint}/memory/summaries/daily`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(request)
    });
    if (!response.ok) {
      const detail = await readErrorDetail(response);
      throw new Error(detail || `daily summary returned ${response.status}`);
    }
    return response.json() as Promise<MemoryItem>;
  },

  async generateProjectSummary(request: ProjectSummaryRequest): Promise<MemoryItem> {
    const response = await fetch(`${coreService.endpoint}/memory/summaries/project`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(request)
    });
    if (!response.ok) {
      const detail = await readErrorDetail(response);
      throw new Error(detail || `project summary returned ${response.status}`);
    }
    return response.json() as Promise<MemoryItem>;
  },

  async exportMemory(): Promise<MemoryExportResponse> {
    const response = await fetch(`${coreService.endpoint}/memory/export`);
    if (!response.ok) {
      throw new Error(`memory export returned ${response.status}`);
    }
    return response.json() as Promise<MemoryExportResponse>;
  },

  async listMemoryEntities(options?: string | MemoryEntityListOptions): Promise<MemoryEntityListResponse> {
    const normalizedOptions = typeof options === "string" ? { query: options } : options ?? {};
    const params = new URLSearchParams();
    appendListLimit(params, normalizedOptions.limit);
    appendFilterParam(params, "query", normalizedOptions.query);
    appendFilterParam(params, "sourceType", normalizedOptions.sourceType);
    appendFilterParam(params, "sourceId", normalizedOptions.sourceId);
    appendFilterParam(params, "date", normalizedOptions.date);
    const response = await fetch(`${coreService.endpoint}/memory/entities?${params.toString()}`);
    if (!response.ok) {
      throw new Error(`memory entities returned ${response.status}`);
    }
    return response.json() as Promise<MemoryEntityListResponse>;
  },

  async listMemoryInsights(
    typeOrOptions?: MemoryInsightFilterType | MemoryInsightListOptions,
    status?: string,
    options?: MemoryInsightListOptions
  ): Promise<MemoryInsightListResponse> {
    const fallbackOptions = options ?? {};
    const normalizedOptions =
      typeof typeOrOptions === "object"
        ? typeOrOptions
        : {
            ...fallbackOptions,
            type: typeOrOptions ?? fallbackOptions.type,
            status: status ?? fallbackOptions.status
          };
    const params = new URLSearchParams();
    appendListLimit(params, normalizedOptions.limit);
    appendFilterParam(params, "query", normalizedOptions.query);
    appendFilterParam(params, "type", normalizedOptions.type);
    appendFilterParam(params, "status", normalizedOptions.status);
    appendFilterParam(params, "sourceType", normalizedOptions.sourceType);
    appendFilterParam(params, "sourceId", normalizedOptions.sourceId);
    appendFilterParam(params, "date", normalizedOptions.date);
    const response = await fetch(`${coreService.endpoint}/memory/insights?${params.toString()}`);
    if (!response.ok) {
      throw new Error(`memory insights returned ${response.status}`);
    }
    return response.json() as Promise<MemoryInsightListResponse>;
  },

  async getModelStatus(): Promise<LocalModelStatusResponse> {
    const response = await fetch(`${coreService.endpoint}/models/status`);
    if (!response.ok) {
      throw new Error(`model status returned ${response.status}`);
    }
    return response.json() as Promise<LocalModelStatusResponse>;
  },

  async selectModel(request: ModelSelectionRequest): Promise<ModelSelectionResponse> {
    const response = await fetch(`${coreService.endpoint}/models/selection`, {
      method: "PATCH",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(request)
    });
    if (!response.ok) {
      const detail = await readErrorDetail(response);
      throw new Error(detail || `model selection returned ${response.status}`);
    }
    return response.json() as Promise<ModelSelectionResponse>;
  },

  async testModel(request: ModelTestRequest = {}): Promise<ModelTestResponse> {
    const response = await fetch(`${coreService.endpoint}/model/test`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(request)
    });
    if (!response.ok) {
      const detail = await readErrorDetail(response);
      throw new Error(detail || `model test returned ${response.status}`);
    }
    return response.json() as Promise<ModelTestResponse>;
  },

  async getChatHistory(): Promise<ChatHistoryResponse> {
    const response = await fetch(`${coreService.endpoint}/chat/history`);
    if (!response.ok) {
      throw new Error(`chat history returned ${response.status}`);
    }
    return response.json() as Promise<ChatHistoryResponse>;
  },

  async sendChatMessage(request: ChatMessageRequest): Promise<ChatMessageResponse> {
    const response = await fetch(`${coreService.endpoint}/chat/message`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(request)
    });
    if (!response.ok) {
      const detail = await readErrorDetail(response);
      throw new Error(detail || `chat message returned ${response.status}`);
    }
    return response.json() as Promise<ChatMessageResponse>;
  },

  async clearChatHistory(): Promise<ChatHistoryDeleteResponse> {
    const response = await fetch(`${coreService.endpoint}/chat/history`, {
      method: "DELETE"
    });
    if (!response.ok) {
      const detail = await readErrorDetail(response);
      throw new Error(detail || `chat clear returned ${response.status}`);
    }
    return response.json() as Promise<ChatHistoryDeleteResponse>;
  },

  async getPrivacyStatus(): Promise<PrivacyStatusResponse> {
    const response = await fetch(`${coreService.endpoint}/privacy/status`);
    if (!response.ok) {
      throw new Error(`privacy status returned ${response.status}`);
    }
    return response.json() as Promise<PrivacyStatusResponse>;
  },

  async listPrivacyAudit(): Promise<PrivacyAuditListResponse> {
    const response = await fetch(`${coreService.endpoint}/privacy/audit?limit=20`);
    if (!response.ok) {
      throw new Error(`privacy audit returned ${response.status}`);
    }
    return response.json() as Promise<PrivacyAuditListResponse>;
  },

  async checkPrivacyRequest(request: PrivacyCheckRequest): Promise<PrivacyCheckResponse> {
    const response = await fetch(`${coreService.endpoint}/privacy/check`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(request)
    });
    if (!response.ok) {
      const detail = await readErrorDetail(response);
      throw new Error(detail || `privacy check returned ${response.status}`);
    }
    return response.json() as Promise<PrivacyCheckResponse>;
  },

  async clearPrivacyAudit(): Promise<PrivacyAuditDeleteResponse> {
    const response = await fetch(`${coreService.endpoint}/privacy/audit`, {
      method: "DELETE"
    });
    if (!response.ok) {
      const detail = await readErrorDetail(response);
      throw new Error(detail || `privacy audit clear returned ${response.status}`);
    }
    return response.json() as Promise<PrivacyAuditDeleteResponse>;
  },

  async listConnectors(): Promise<ConnectorListResponse> {
    const response = await fetch(`${coreService.endpoint}/connectors`);
    if (!response.ok) {
      throw new Error(`connector list returned ${response.status}`);
    }
    return response.json() as Promise<ConnectorListResponse>;
  },

  async getConnector(connectorId: string): Promise<ConnectorItem> {
    const response = await fetch(`${coreService.endpoint}/connectors/${connectorId}`);
    if (!response.ok) {
      const detail = await readErrorDetail(response);
      throw new Error(detail || `connector status returned ${response.status}`);
    }
    return response.json() as Promise<ConnectorItem>;
  },

  async updateConnectorSettings(
    connectorId: string,
    request: ConnectorSettingsPatch
  ): Promise<ConnectorItem> {
    const response = await fetch(`${coreService.endpoint}/connectors/${connectorId}/settings`, {
      method: "PATCH",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(request)
    });
    if (!response.ok) {
      const detail = await readErrorDetail(response);
      throw new Error(detail || `connector settings returned ${response.status}`);
    }
    return response.json() as Promise<ConnectorItem>;
  },

  async startConnectorOAuth(
    connectorId: string,
    request: ConnectorOAuthStartRequest = {}
  ): Promise<ConnectorOAuthStartResponse> {
    const response = await fetch(`${coreService.endpoint}/connectors/${connectorId}/oauth/start`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(request)
    });
    if (!response.ok) {
      const detail = await readErrorDetail(response);
      throw new Error(detail || `connector OAuth start returned ${response.status}`);
    }
    return response.json() as Promise<ConnectorOAuthStartResponse>;
  },

  async completeConnectorOAuth(
    connectorId: string,
    request: ConnectorOAuthCompleteRequest
  ): Promise<ConnectorItem> {
    const response = await fetch(`${coreService.endpoint}/connectors/${connectorId}/oauth/complete`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(request)
    });
    if (!response.ok) {
      const detail = await readErrorDetail(response);
      throw new Error(detail || `connector OAuth completion returned ${response.status}`);
    }
    return response.json() as Promise<ConnectorItem>;
  },

  async disconnectConnector(connectorId: string): Promise<ConnectorDisconnectResponse> {
    const response = await fetch(`${coreService.endpoint}/connectors/${connectorId}/disconnect`, {
      method: "POST"
    });
    if (!response.ok) {
      const detail = await readErrorDetail(response);
      throw new Error(detail || `connector disconnect returned ${response.status}`);
    }
    return response.json() as Promise<ConnectorDisconnectResponse>;
  },

  async syncConnector(
    connectorId: string,
    request: ConnectorSyncRequest = { reason: "manual" }
  ): Promise<ConnectorSyncResponse> {
    const response = await fetch(`${coreService.endpoint}/connectors/${connectorId}/sync`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(request)
    });
    if (!response.ok) {
      const detail = await readErrorDetail(response);
      throw new Error(detail || `connector sync returned ${response.status}`);
    }
    return response.json() as Promise<ConnectorSyncResponse>;
  },

  async listConnectorSyncRuns(): Promise<ConnectorSyncRunsResponse> {
    const response = await fetch(`${coreService.endpoint}/connectors/sync-runs?limit=12`);
    if (!response.ok) {
      throw new Error(`connector sync runs returned ${response.status}`);
    }
    return response.json() as Promise<ConnectorSyncRunsResponse>;
  }
};

const readErrorDetail = async (response: Response) => {
  try {
    const body = (await response.json()) as { detail?: string };
    return body.detail;
  } catch {
    return undefined;
  }
};
