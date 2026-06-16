import { DEFAULT_PHASE1_SETTINGS, type Phase1Settings, type UiMode } from "@deyana/schemas";
import { invoke } from "@tauri-apps/api/core";

const browserSettingsKey = "deyana.phase1.settings";

const isTauriRuntime = () =>
  typeof window !== "undefined" && typeof window.__TAURI_INTERNALS__ !== "undefined";

const readBrowserSettings = (): Phase1Settings => {
  const stored = window.localStorage.getItem(browserSettingsKey);

  if (!stored) {
    return DEFAULT_PHASE1_SETTINGS;
  }

  try {
    return { ...DEFAULT_PHASE1_SETTINGS, ...JSON.parse(stored) } as Phase1Settings;
  } catch {
    return DEFAULT_PHASE1_SETTINGS;
  }
};

const writeBrowserSettings = (settings: Phase1Settings) => {
  window.localStorage.setItem(browserSettingsKey, JSON.stringify(settings));
};

export const tauriClient = {
  async getPhase1Settings(): Promise<Phase1Settings> {
    if (!isTauriRuntime()) {
      return readBrowserSettings();
    }

    return invoke<Phase1Settings>("get_phase1_settings");
  },

  async setFloatingMode(mode: UiMode): Promise<Phase1Settings> {
    if (!isTauriRuntime()) {
      const next = { ...readBrowserSettings(), uiMode: mode };
      writeBrowserSettings(next);
      return next;
    }

    return invoke<Phase1Settings>("set_floating_mode", { mode });
  },

  async setAlwaysOnTop(alwaysOnTop: boolean): Promise<Phase1Settings> {
    if (!isTauriRuntime()) {
      const next = { ...readBrowserSettings(), alwaysOnTop };
      writeBrowserSettings(next);
      return next;
    }

    return invoke<Phase1Settings>("set_always_on_top", { alwaysOnTop });
  },

  async hideMainWindow(): Promise<void> {
    if (!isTauriRuntime()) {
      return;
    }

    await invoke("hide_main_window");
  }
};

