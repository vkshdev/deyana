import type { AssistantSnapshot } from "../../stores/assistantStore";
import { productIdentity } from "@deyana/config";
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
import { BrowserAgentPanel } from "../browser/BrowserAgentPanel";
import { ModelSetupPanel } from "./ModelSetupPanel";
import { PrivacyAuditPanel } from "./PrivacyAuditPanel";
import { QuickActions } from "./QuickActions";
import { ReleaseQualityPanel } from "./ReleaseQualityPanel";
import { ToolPanel } from "./ToolPanel";
import { VoicePanel } from "./VoicePanel";
import { ProactiveAlertCards } from "./ProactiveAlertCards";
import { startWindowDrag } from "../../utils/windowDrag";

interface FloatingPanelProps {
  snapshot: AssistantSnapshot;
}

export function FloatingPanel({ snapshot }: FloatingPanelProps) {
  return (
    <section
      className="floating-panel"
      aria-label={`${productIdentity.name} assistant panel`}
      style={{
        background:
          "linear-gradient(150deg, rgba(131, 169, 212, 0.12), transparent 36%), linear-gradient(330deg, rgba(216, 185, 120, 0.08), transparent 38%), rgba(4, 8, 14, 0.82)",
        border: "1px solid rgba(131, 169, 212, 0.25)",
        boxShadow: "0 24px 74px rgba(0, 0, 0, 0.6), 0 0 32px rgba(131, 169, 212, 0.18)",
        backdropFilter: "blur(24px) saturate(118%)",
        WebkitBackdropFilter: "blur(24px) saturate(118%)"
      }}
    >
      <header className="panel-header" data-tauri-drag-region onPointerDown={startWindowDrag}>
        <FloatingDockHandle />
        <div className="brand-mark" data-tauri-drag-region>
          <FloatingStatusRing state={snapshot.assistantState} />
          <div data-tauri-drag-region>
            <strong style={{ color: "#d8b978" }}>{productIdentity.brand}</strong>
            <span style={{ color: "#83a9d4" }}>{snapshot.assistantState.replaceAll("_", " ")}</span>
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
              <Eye size={17} aria-hidden="true" style={{ color: "#d8b978" }} />
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
        {/* Proactive Context Alert Cards */}
        <ProactiveAlertCards snapshot={snapshot} />

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
        <BrowserAgentPanel snapshot={snapshot} />
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
          style={{
            background: "linear-gradient(135deg, rgba(216, 185, 120, 0.2), rgba(131, 169, 212, 0.15))",
            border: "1px solid rgba(216, 185, 120, 0.4)",
            color: "#d8b978"
          }}
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
          <Bell size={17} aria-hidden="true" style={{ color: snapshot.proactiveCards?.length ? "#d8b978" : undefined }} />
        </button>
      </footer>

      {snapshot.error ? <div className="panel-error">{snapshot.error}</div> : null}
    </section>
  );
}
