import type { AssistantSnapshot } from "../../stores/assistantStore";
import { productIdentity } from "@deyana/config";
import { ChevronLeft } from "lucide-react";
import { assistantStore } from "../../stores/assistantStore";
import { FloatingDockHandle } from "./FloatingDockHandle";
import { FloatingStatusRing } from "./FloatingStatusRing";
import { ChatInput } from "../chat/ChatInput";
import { startWindowDrag } from "../../utils/windowDrag";

interface CompactCommandBarProps {
  snapshot: AssistantSnapshot;
}

export function CompactCommandBar({ snapshot }: CompactCommandBarProps) {
  return (
    <section className="compact-command-bar" aria-label={`${productIdentity.name} command bar`}>
      <div className="command-bar-drag-region" data-tauri-drag-region onPointerDown={startWindowDrag}>
        <FloatingDockHandle />
        <FloatingStatusRing state={snapshot.assistantState} compact />
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
      >
        <ChevronLeft size={16} aria-hidden="true" />
      </button>
    </section>
  );
}
