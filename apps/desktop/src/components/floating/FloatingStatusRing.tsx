import type { AffectiveState, AffectiveStateRecord, AssistantState } from "@deyana/schemas";
import { DeyanaMark } from "../brand/DeyanaMark";

interface FloatingStatusRingProps {
  state: AssistantState;
  affectiveState?: AffectiveStateRecord | AffectiveState | null;
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

export function FloatingStatusRing({
  state,
  affectiveState,
  compact = false
}: FloatingStatusRingProps) {
  const discrete: AffectiveState =
    typeof affectiveState === "string"
      ? affectiveState
      : affectiveState?.discrete ?? "NEUTRAL";

  const auraClass = `affective-aura aura-${discrete.toLowerCase()}`;

  return (
    <div
      className={compact ? "status-ring status-ring-compact" : "status-ring"}
      title={`State: ${state} • Mood: ${discrete}`}
    >
      <div className={`status-ring-inner ${stateClass[state] ?? "status-idle"}`} />
      <div className={auraClass} aria-hidden="true" />
      <DeyanaMark className="status-core-mark" />
    </div>
  );
}
