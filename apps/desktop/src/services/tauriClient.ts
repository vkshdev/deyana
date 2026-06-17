import {
  DEFAULT_BACKEND_PROCESS_STATUS,
  DEFAULT_DESKTOP_SETTINGS,
  type BackendProcessStatus,
  type DesktopSettings,
  type UiMode
} from "@deyana/schemas";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
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
  const controller = new AbortController();
  const timeout = window.setTimeout(() => controller.abort(), 700);

  try {
    const response = await fetch("http://127.0.0.1:8765/health", {
      signal: controller.signal
    });
    if (!response.ok) {
      throw new Error(`core health returned ${response.status}`);
    }

    return {
      ...DEFAULT_BACKEND_PROCESS_STATUS,
      lifecycle: "running",
      updatedAtMs: Date.now()
    };
  } catch {
    return {
      ...DEFAULT_BACKEND_PROCESS_STATUS,
      lifecycle: "unavailable",
      updatedAtMs: Date.now(),
      lastError: "Core service is not running"
    };
  } finally {
    window.clearTimeout(timeout);
  }
};

export const tauriClient = {
  isTauriRuntime,

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

  async hideMainWindow(): Promise<void> {
    if (!isTauriRuntime()) {
      return;
    }

    await invoke("hide_main_window");
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
      title: "Choose DE'YANA vault folder"
    });

    return typeof selected === "string" ? selected : null;
  },

  async openVaultFolder(path: string): Promise<void> {
    if (!isTauriRuntime()) {
      return;
    }

    await invoke("open_vault_folder", { path });
  }
};
