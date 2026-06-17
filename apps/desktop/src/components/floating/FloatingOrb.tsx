import type { AssistantSnapshot } from "../../stores/assistantStore";
import { ChevronLeft, MessageCircle, Power, Server, ServerCrash } from "lucide-react";
import { assistantStore } from "../../stores/assistantStore";
import { CompactVoiceButton } from "./CompactVoiceButton";
import { FloatingDockHandle } from "./FloatingDockHandle";
import { FloatingStatusRing } from "./FloatingStatusRing";

interface FloatingOrbProps {
  snapshot: AssistantSnapshot;
}

export function FloatingOrb({ snapshot }: FloatingOrbProps) {
  return (
    <section className="floating-orb" aria-label="DE'YANA compact assistant">
      <FloatingDockHandle />
      <FloatingStatusRing state={snapshot.assistantState} compact />
      <div className="compact-status-stack">
        {snapshot.backend.lifecycle === "running" ? (
          <Server size={14} aria-hidden="true" />
        ) : (
          <ServerCrash size={14} aria-hidden="true" />
        )}
        <span>{snapshot.backend.lifecycle === "running" ? "OK" : "!"}</span>
      </div>
      <div className="compact-actions">
        <button
          className="icon-button"
          type="button"
          title="Open chat"
          aria-label="Open chat"
          onClick={() => assistantStore.setFloatingMode("expanded")}
        >
          <MessageCircle size={18} aria-hidden="true" />
        </button>
        <CompactVoiceButton />
        <button
          className="icon-button"
          type="button"
          title="Hide"
          aria-label="Hide"
          onClick={() => void assistantStore.hideWindow()}
        >
          <Power size={17} aria-hidden="true" />
        </button>
      </div>
      <button
        className="edge-toggle"
        type="button"
        title="Expand"
        aria-label="Expand"
        onClick={() => assistantStore.setFloatingMode("expanded")}
      >
        <ChevronLeft size={16} aria-hidden="true" />
      </button>
    </section>
  );
}
