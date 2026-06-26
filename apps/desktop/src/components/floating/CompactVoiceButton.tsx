import { Mic } from "lucide-react";
import { assistantStore } from "../../stores/assistantStore";

export function CompactVoiceButton() {
  return (
    <button
      className="icon-button"
      type="button"
      title="Voice"
      aria-label="Voice"
      onClick={() => void assistantStore.runPushToTalk()}
    >
      <Mic size={18} aria-hidden="true" />
    </button>
  );
}
