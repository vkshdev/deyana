import type { AssistantSnapshot } from "../../stores/assistantStore";
import { Bell, ChevronRight, Eye, EyeOff, Mic, RotateCw, Settings } from "lucide-react";
import { assistantStore } from "../../stores/assistantStore";
import { BackendStatusBadge } from "./BackendStatusBadge";
import { FloatingDockHandle } from "./FloatingDockHandle";
import { FloatingModelBadge } from "./FloatingModelBadge";
import { FloatingPreferences } from "./FloatingPreferences";
import { FloatingPrivacyBadge } from "./FloatingPrivacyBadge";
import { FloatingStatusRing } from "./FloatingStatusRing";
import { FloatingSyncIndicator } from "./FloatingSyncIndicator";
import { ConnectorStatusList } from "./ConnectorStatusList";
import { LocalChat } from "./LocalChat";
import { MemoryBrowser } from "../memory/MemoryBrowser";
import { ModelSetupPanel } from "./ModelSetupPanel";
import { PrivacyAuditPanel } from "./PrivacyAuditPanel";
import { QuickActions } from "./QuickActions";
import { ReleaseQualityPanel } from "./ReleaseQualityPanel";
import { ToolPanel } from "./ToolPanel";
import { VoicePanel } from "./VoicePanel";

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

      <div className="panel-body">
        <div className="status-row">
          <FloatingPrivacyBadge />
          <BackendStatusBadge
            backend={snapshot.backend}
            eventStreamConnected={snapshot.backendEventStreamConnected}
          />
          <FloatingModelBadge
            status={snapshot.modelStatus}
            modelName={snapshot.coreSettings.selectedChatModel}
          />
          <FloatingSyncIndicator status={snapshot.syncStatus} />
        </div>

        <FloatingPreferences snapshot={snapshot} />
        <ModelSetupPanel snapshot={snapshot} />
        <VoicePanel snapshot={snapshot} />
        <LocalChat snapshot={snapshot} />
        <PrivacyAuditPanel snapshot={snapshot} />
        <ReleaseQualityPanel snapshot={snapshot} />

        <QuickActions actions={snapshot.quickActions} />
        <ToolPanel snapshot={snapshot} />
        <MemoryBrowser snapshot={snapshot} />
        <ConnectorStatusList snapshot={snapshot} />
      </div>

      <footer className="panel-footer">
        <button
          className="voice-control"
          type="button"
          title="Voice"
          disabled={snapshot.voiceBusy}
          onClick={() => void assistantStore.runPushToTalk()}
        >
          <Mic size={17} aria-hidden="true" />
          <span>{snapshot.voiceBusy ? snapshot.assistantState.replaceAll("_", " ") : "Push to talk"}</span>
        </button>
        <button
          className="icon-button"
          type="button"
          title="Restart backend"
          aria-label="Restart backend"
          onClick={() => void assistantStore.restartBackend()}
        >
          <RotateCw size={17} aria-hidden="true" />
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
