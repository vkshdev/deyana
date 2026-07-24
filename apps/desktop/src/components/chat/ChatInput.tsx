import { Send } from "lucide-react";
import type { AssistantSnapshot } from "../../stores/assistantStore";
import { assistantStore } from "../../stores/assistantStore";

interface ChatInputProps {
  snapshot: AssistantSnapshot;
  disabled?: boolean;
}

export function ChatInput({ snapshot, disabled = false }: ChatInputProps) {
  const isSendDisabled = disabled || snapshot.chatBusy || snapshot.modelStatus !== "available" || !snapshot.chatDraft.trim();
  const isInputDisabled = disabled || snapshot.chatBusy || snapshot.modelStatus !== "available";

  return (
    <form
      className="chat-form"
      onSubmit={(event) => {
        event.preventDefault();
        if (!isSendDisabled) {
          void assistantStore.sendChatMessage();
        }
      }}
    >
      <input
        value={snapshot.chatDraft}
        placeholder="Message local model"
        aria-label="Message local model"
        disabled={isInputDisabled}
        onChange={(event) => assistantStore.setChatDraft(event.target.value)}
      />
      <button
        className="icon-button"
        type="submit"
        title="Send"
        aria-label="Send"
        disabled={isSendDisabled}
      >
        <Send size={15} aria-hidden="true" />
      </button>
    </form>
  );
}
