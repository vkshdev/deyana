import type { AssistantState } from "@deyana/schemas";

interface FloatingStatusRingProps {
  state: AssistantState;
  compact?: boolean;
}

const stateClass: Partial<Record<AssistantState, string>> = {
  LISTENING: "status-listening",
  THINKING: "status-thinking",
  RETRIEVING_MEMORY: "status-thinking",
  SEARCHING_WEB: "status-thinking",
  CODING: "status-coding",
  SYNCING: "status-syncing",
  SPEAKING: "status-speaking",
  BLOCKED_BY_PRIVACY: "status-blocked",
  CONNECTOR_ERROR: "status-error",
  ERROR: "status-error",
  MODEL_MISSING: "status-error"
};

export function FloatingStatusRing({ state, compact = false }: FloatingStatusRingProps) {
  return (
    <div className={compact ? "status-ring status-ring-compact" : "status-ring"}>
      <div className={`status-ring-inner ${stateClass[state] ?? "status-idle"}`} />
      <div className="status-core" />
    </div>
  );
}

