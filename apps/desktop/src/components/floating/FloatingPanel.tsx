import type { AssistantSnapshot } from "../../stores/assistantStore";
import { Bell, ChevronRight, Eye, EyeOff, Mic, Settings } from "lucide-react";
import { assistantStore } from "../../stores/assistantStore";
import { FloatingDockHandle } from "./FloatingDockHandle";
import { FloatingModelBadge } from "./FloatingModelBadge";
import { FloatingPrivacyBadge } from "./FloatingPrivacyBadge";
import { FloatingStatusRing } from "./FloatingStatusRing";
import { FloatingSyncIndicator } from "./FloatingSyncIndicator";
import { ConnectorStatusList } from "./ConnectorStatusList";
import { MemoryPreview } from "./MemoryPreview";
import { QuickActions } from "./QuickActions";

interface FloatingPanelProps {
  snapshot: AssistantSnapshot;
}

export function FloatingPanel({ snapshot }: FloatingPanelProps) {
  return (
    <section className="floating-panel" aria-label="DE'YANA assistant panel">
      <header className="panel-header">
        <FloatingDockHandle />
        <div className="brand-mark" data-tauri-drag-region>
          <FloatingStatusRing state={snapshot.assistantState} />
          <div data-tauri-drag-region>
            <strong>DE'YANA</strong>
            <span>{snapshot.assistantState.replaceAll("_", " ")}</span>
          </div>
        </div>
        <div className="header-actions">
          <button
            className="icon-button"
            type="button"
            title={snapshot.settings.alwaysOnTop ? "Disable always on top" : "Enable always on top"}
            aria-label={snapshot.settings.alwaysOnTop ? "Disable always on top" : "Enable always on top"}
            onClick={() => assistantStore.setAlwaysOnTop(!snapshot.settings.alwaysOnTop)}
          >
            {snapshot.settings.alwaysOnTop ? (
              <Eye size={17} aria-hidden="true" />
            ) : (
              <EyeOff size={17} aria-hidden="true" />
            )}
          </button>
          <button className="icon-button" type="button" title="Settings" aria-label="Settings">
            <Settings size={17} aria-hidden="true" />
          </button>
          <button
            className="icon-button"
            type="button"
            title="Collapse"
            aria-label="Collapse"
            onClick={() => assistantStore.setFloatingMode("compact")}
          >
            <ChevronRight size={17} aria-hidden="true" />
          </button>
        </div>
      </header>

      <div className="status-row">
        <FloatingPrivacyBadge />
        <FloatingModelBadge status={snapshot.modelStatus} />
        <FloatingSyncIndicator status={snapshot.syncStatus} />
      </div>

      <section className="chat-surface" aria-label="Chat">
        <article className="message message-assistant">
          <span>Local shell ready. Backend core starts in Phase 2.</span>
        </article>
        <article className="message message-user">
          <span>Keep private data local.</span>
        </article>
        <article className="message message-assistant">
          <span>Low-spec mode is active with qwen3:1.7b.</span>
        </article>
      </section>

      <QuickActions actions={snapshot.quickActions} />
      <MemoryPreview items={snapshot.memoryPreview} />
      <ConnectorStatusList connectors={snapshot.connectors} />

      <footer className="panel-footer">
        <button
          className="voice-control"
          type="button"
          title="Voice"
          onClick={() => assistantStore.setAssistantState("LISTENING")}
        >
          <Mic size={17} aria-hidden="true" />
          <span>Push to talk</span>
        </button>
        <button
          className="icon-button"
          type="button"
          title="Notifications"
          aria-label="Notifications"
          onClick={() => assistantStore.setAssistantState("SYNCING")}
        >
          <Bell size={17} aria-hidden="true" />
        </button>
      </footer>

      {snapshot.error ? <div className="panel-error">{snapshot.error}</div> : null}
    </section>
  );
}

