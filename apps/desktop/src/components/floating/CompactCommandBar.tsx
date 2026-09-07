import type { AssistantSnapshot } from "../../stores/assistantStore";
import { productIdentity } from "@deyana/config";
import { ChevronLeft } from "lucide-react";
import { assistantStore } from "../../stores/assistantStore";
import { FloatingDockHandle } from "./FloatingDockHandle";
import { FloatingStatusRing } from "./FloatingStatusRing";
import { ChatInput } from "../chat/ChatInput";
import { ProactiveAlertCards } from "./ProactiveAlertCards";
import { startWindowDrag } from "../../utils/windowDrag";

interface CompactCommandBarProps {
  snapshot: AssistantSnapshot;
}

export function CompactCommandBar({ snapshot }: CompactCommandBarProps) {
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "6px",
        width: "400px",
        position: "relative"
      }}
    >
      {/* Proactive Context Alert Banner for Compact Mode */}
      <ProactiveAlertCards snapshot={snapshot} compact />

      <section
        className="compact-command-bar"
        aria-label={`${productIdentity.name} command bar`}
        style={{
          width: "400px",
          height: "60px",
          background:
            "linear-gradient(135deg, rgba(131, 169, 212, 0.1), transparent 40%), linear-gradient(315deg, rgba(216, 185, 120, 0.06), transparent 40%), rgba(4, 8, 14, 0.82)",
          border: "1px solid rgba(131, 169, 212, 0.25)",
          borderRadius: "12px",
          boxShadow: "0 16px 48px rgba(0, 0, 0, 0.55), 0 0 24px rgba(131, 169, 212, 0.16)",
          backdropFilter: "blur(24px) saturate(118%)",
          WebkitBackdropFilter: "blur(24px) saturate(118%)"
        }}
      >
        <div className="command-bar-drag-region" data-tauri-drag-region onPointerDown={startWindowDrag}>
          <FloatingDockHandle />
          <FloatingStatusRing
            state={snapshot.assistantState}
            affectiveState={snapshot.affectiveState}
            compact
          />
        </div>
        
        <div className="command-bar-input">
          <ChatInput snapshot={snapshot} />
        </div>

        <button
          className="icon-button expand-button"
          type="button"
          title="Expand"
          aria-label="Expand"
          onClick={() => assistantStore.setFloatingMode("expanded")}
          style={{
            borderColor: "rgba(131, 169, 212, 0.3)",
            color: "#83a9d4"
          }}
        >
          <ChevronLeft size={16} aria-hidden="true" />
        </button>
      </section>
    </div>
  );
}
