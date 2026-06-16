import { coreService } from "@deyana/config";
import type { BackendHealthResponse, BackendStatusResponse, CoreWebSocketEvent } from "@deyana/schemas";

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
    onEvent: (event: CoreWebSocketEvent) => void,
    onClose: (reason: string) => void
  ): BackendEventConnection {
    const socket = new WebSocket(coreService.websocketUrl);

    socket.onmessage = (message) => {
      try {
        onEvent(JSON.parse(message.data as string) as CoreWebSocketEvent);
      } catch {
        onClose("Received an invalid backend event");
      }
    };

    socket.onerror = () => onClose("Backend event stream failed");
    socket.onclose = () => onClose("Backend event stream closed");

    return {
      disconnect: () => socket.close()
    };
  }
};
