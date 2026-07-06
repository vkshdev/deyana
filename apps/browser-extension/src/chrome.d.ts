declare namespace chrome {
  namespace runtime {
    interface Port {
      postMessage(message: unknown): void;
      disconnect(): void;
      onMessage: Event<(message: unknown) => void>;
      onDisconnect: Event<() => void>;
    }

    interface MessageSender {
      tab?: tabs.Tab;
    }

    interface Event<TListener extends (...args: any[]) => unknown> {
      addListener(callback: TListener): void;
      removeListener(callback: TListener): void;
    }

    const id: string;
    const lastError: { message?: string } | undefined;
    const onMessage: Event<(
      message: unknown,
      sender: MessageSender,
      sendResponse: (response: unknown) => void
    ) => boolean | void>;
    const onStartup: Event<() => void>;
    const onInstalled: Event<() => void>;
    function connectNative(application: string): Port;
    function getManifest(): { version: string };
    function sendMessage<T = unknown>(message: unknown): Promise<T>;
  }

  namespace tabs {
    interface Tab {
      id?: number;
      url?: string;
      title?: string;
      active?: boolean;
    }

    const onUpdated: runtime.Event<(
      tabId: number,
      changeInfo: { status?: string; url?: string },
      tab: Tab
    ) => void>;
    const onRemoved: runtime.Event<(tabId: number) => void>;
    function query(queryInfo: { active?: boolean; currentWindow?: boolean }): Promise<Tab[]>;
    function sendMessage<T = unknown>(tabId: number, message: unknown): Promise<T>;
    function create(createProperties: { url: string; active: boolean }): Promise<Tab>;
  }

  namespace scripting {
    function executeScript(details: {
      target: { tabId: number };
      files: string[];
    }): Promise<unknown>;
  }

  namespace permissions {
    interface Permissions {
      permissions?: string[];
      origins?: string[];
    }

    function getAll(): Promise<Permissions>;
    function request(permissions: Permissions): Promise<boolean>;
    function remove(permissions: Permissions): Promise<boolean>;
  }

  namespace commands {
    const onCommand: runtime.Event<(command: string) => void>;
  }

  namespace contextMenus {
    interface OnClickData {
      menuItemId: string | number;
      selectionText?: string;
    }

    const onClicked: runtime.Event<(info: OnClickData, tab?: tabs.Tab) => void>;
    function create(properties: {
      id: string;
      title: string;
      contexts: Array<"page" | "selection">;
    }): void;
    function removeAll(): Promise<void>;
  }
}
