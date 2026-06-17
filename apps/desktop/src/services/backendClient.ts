import { coreService } from "@deyana/config";
import type {
  AppCoreEvent,
  BackendHealthResponse,
  BackendStatusResponse,
  CoreAppSettings,
  MemoryCreateRequest,
  MemoryDeleteResponse,
  MemoryExportResponse,
  MemoryItem,
  MemoryListResponse,
  MemoryReindexResponse,
  MemoryUpdateRequest,
  OnboardingCompleteRequest,
  OnboardingCompleteResponse,
  OnboardingState,
  SettingsPatch,
  VaultSelectRequest,
  VaultSelectResponse
} from "@deyana/schemas";

export interface BackendEventConnection {
  disconnect: () => void;
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

  async exportMemory(): Promise<MemoryExportResponse> {
    const response = await fetch(`${coreService.endpoint}/memory/export`);
    if (!response.ok) {
      throw new Error(`memory export returned ${response.status}`);
    }
    return response.json() as Promise<MemoryExportResponse>;
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
