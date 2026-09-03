import { useEffect, useRef } from "react";
import type { AssistantSnapshot } from "../../stores/assistantStore";
import type { MemorySourceReference, WebSourceReference } from "@deyana/schemas";
import { MessageSquare, Trash2 } from "lucide-react";
import { assistantStore } from "../../stores/assistantStore";
import { ChatInput } from "../chat/ChatInput";
import { AgentStepBadge } from "../chat/AgentStepBadge";

interface LocalChatProps {
  snapshot: AssistantSnapshot;
}

export function LocalChat({ snapshot }: LocalChatProps) {
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const emptyMessage =
    snapshot.modelStatus === "available"
      ? "Local model ready."
      : snapshot.modelStatusDetail?.message ?? "Local model unavailable.";

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [
    snapshot.chatMessages,
    snapshot.activeAgentSteps,
    snapshot.streamingResponse,
    snapshot.chatBusy
  ]);

  return (
    <section className="chat-surface" aria-label="Local chat">
      <header className="chat-header">
        <div className="section-heading">
          <MessageSquare size={15} aria-hidden="true" />
          <span>Chat</span>
        </div>
        <button
          className="icon-button"
          type="button"
          title="Clear chat"
          aria-label="Clear chat"
          disabled={snapshot.chatBusy || !snapshot.chatMessages.length}
          onClick={() => void assistantStore.clearChatHistory()}
        >
          <Trash2 size={15} aria-hidden="true" />
        </button>
      </header>

      <div className="chat-log" ref={scrollRef}>
        {snapshot.chatMessages.length ? (
          snapshot.chatMessages.map((message) => (
            <article
              className={message.role === "user" ? "message message-user" : "message message-assistant"}
              key={message.id}
            >
              {message.steps && message.steps.length ? (
                <div className="agent-steps-container" aria-label="Agent execution steps">
                  {message.steps.map((step) => (
                    <AgentStepBadge key={step.step} step={step} />
                  ))}
                </div>
              ) : null}
              {message.content ? <span>{message.content}</span> : null}
              {message.sourceReferences.length ? (
                <div className="source-stack" aria-label="Local memory sources">
                  {message.sourceReferences.map((source) => (
                    <SourceReference source={source} key={`${message.id}-${source.id}`} />
                  ))}
                </div>
              ) : null}
              {message.webSourceReferences.length ? (
                <div className="source-stack" aria-label="Public web sources">
                  {message.webSourceReferences.map((source, index) => (
                    <WebSourceReferenceView
                      source={source}
                      label={`W${index + 1}`}
                      key={`${message.id}-${source.url}`}
                    />
                  ))}
                </div>
              ) : null}
              {message.model ? <small>{message.model}</small> : null}
            </article>
          ))
        ) : !snapshot.chatBusy ? (
          <article className="message message-assistant">
            <span>{emptyMessage}</span>
          </article>
        ) : null}

        {snapshot.chatBusy ? (
          <article className="message message-assistant message-in-flight" aria-live="polite">
            {snapshot.activeAgentSteps && snapshot.activeAgentSteps.length ? (
              <div className="agent-steps-container" aria-label="Real-time agent steps">
                {snapshot.activeAgentSteps.map((step) => (
                  <AgentStepBadge key={step.step} step={step} />
                ))}
              </div>
            ) : null}
            {snapshot.streamingResponse ? (
              <span className="streaming-response-text">{snapshot.streamingResponse}</span>
            ) : !snapshot.activeAgentSteps || !snapshot.activeAgentSteps.length ? (
              <div className="agent-step-thinking">
                <span className="step-pulse-dot" aria-hidden="true" />
                <span>Thinking...</span>
              </div>
            ) : null}
          </article>
        ) : null}
      </div>

      <ChatInput snapshot={snapshot} />
    </section>
  );
}

function SourceReference({ source }: { source: MemorySourceReference }) {
  const path = source.markdownPath ?? source.sourceUri ?? source.sourceType;

  return (
    <details className="source-reference">
      <summary>
        <span>[{source.label}]</span>
        <strong>{source.title}</strong>
      </summary>
      <p>{source.snippet}</p>
      <small>{path}</small>
    </details>
  );
}

function WebSourceReferenceView({
  source,
  label
}: {
  source: WebSourceReference;
  label: string;
}) {
  return (
    <details className="source-reference source-reference-web">
      <summary>
        <span>[{label}]</span>
        <strong>{source.title}</strong>
      </summary>
      <p>{source.snippet}</p>
      <a href={source.url} target="_blank" rel="noreferrer">
        {source.url}
      </a>
    </details>
  );
}
