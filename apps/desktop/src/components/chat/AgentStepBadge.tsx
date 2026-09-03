import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import type { LucideIcon } from "lucide-react";
import {
  Activity,
  AlertCircle,
  CheckCircle2,
  ChevronRight,
  FileText,
  GitBranch,
  Globe2,
  Inbox,
  ListTodo,
  Loader2,
  Search,
  Sparkles,
  StickyNote,
  TerminalSquare
} from "lucide-react";
import type { AgentStep, AgentStepStatus } from "@deyana/schemas";

const TOOL_ICON_MAP: Record<string, LucideIcon> = {
  search_memory: Search,
  save_note: StickyNote,
  get_system_status: Activity,
  get_triage_inbox: Inbox,
  web_search: Globe2,
  fetch_page: Globe2,
  read_file: FileText,
  git_status: GitBranch,
  git_diff: GitBranch,
  commit_message: GitBranch,
  code_task: TerminalSquare,
  day_planner: ListTodo
};

function getToolIcon(action: string): LucideIcon {
  return TOOL_ICON_MAP[action.toLowerCase()] ?? Sparkles;
}

function formatToolName(action: string): string {
  if (!action) return "Tool";
  return action
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function formatActionInput(input: Record<string, unknown> | string | undefined): string {
  if (input === undefined || input === null) return "";
  if (typeof input === "string") {
    try {
      const parsed = JSON.parse(input);
      return JSON.stringify(parsed, null, 2);
    } catch {
      return input;
    }
  }
  return JSON.stringify(input, null, 2);
}

function formatObservation(obs: string | undefined): string {
  if (!obs) return "";
  try {
    const parsed = JSON.parse(obs);
    return JSON.stringify(parsed, null, 2);
  } catch {
    return obs;
  }
}

interface AgentStepBadgeProps {
  step: AgentStep;
}

export function AgentStepBadge({ step }: AgentStepBadgeProps) {
  const [expanded, setExpanded] = useState(false);
  const Icon = getToolIcon(step.action);
  const isRunning = step.status === "running";
  const isFailed = step.status === "failed";
  const formattedInput = formatActionInput(step.actionInput);
  const formattedObs = formatObservation(step.observation);
  const hasDetails = Boolean(step.thought || formattedInput || formattedObs);

  return (
    <div
      className={`agent-step-badge-wrapper ${isRunning ? "status-running" : ""} ${isFailed ? "status-failed" : ""}`}
      data-step={step.step}
    >
      <button
        type="button"
        className={`agent-step-pill ${isRunning ? "agent-step-pill-running" : ""} ${isFailed ? "agent-step-pill-failed" : "agent-step-pill-completed"}`}
        onClick={() => setExpanded((prev) => !prev)}
        aria-expanded={expanded}
        aria-label={`Step ${step.step}: ${formatToolName(step.action)} (${step.status})`}
      >
        <span className="step-tool-icon" aria-hidden="true">
          <Icon size={12} />
        </span>

        <span className="step-label">
          <span className="step-index">Step {step.step}:</span>
          <span className="step-action-name">{formatToolName(step.action)}</span>
        </span>

        <span className="step-status" aria-label={`Status: ${step.status}`}>
          {isRunning ? (
            <span className="step-running-indicator">
              <span className="step-pulsing-dot" aria-hidden="true" />
              <Loader2 size={11} className="step-spinner animate-spin" aria-hidden="true" />
            </span>
          ) : isFailed ? (
            <AlertCircle size={11} className="text-rose" aria-hidden="true" />
          ) : (
            <CheckCircle2 size={11} className="text-gold" aria-hidden="true" />
          )}
        </span>

        {step.durationMs !== undefined && step.durationMs !== null ? (
          <span className="step-duration-badge">{step.durationMs}ms</span>
        ) : null}

        {hasDetails ? (
          <ChevronRight
            size={11}
            className={`step-expand-chevron ${expanded ? "step-expand-chevron-open" : ""}`}
            aria-hidden="true"
          />
        ) : null}
      </button>

      <AnimatePresence initial={false}>
        {expanded && hasDetails && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
            transition={{ duration: 0.18, ease: "easeOut" }}
            className="agent-step-drawer"
          >
            {step.thought ? (
              <div className="step-drawer-section">
                <span className="step-section-heading">Thought</span>
                <p className="step-thought-text">{step.thought}</p>
              </div>
            ) : null}

            {formattedInput ? (
              <div className="step-drawer-section">
                <span className="step-section-heading">Action Input</span>
                <pre className="step-code-block">
                  <code>{formattedInput}</code>
                </pre>
              </div>
            ) : null}

            {formattedObs ? (
              <div className="step-drawer-section">
                <span className="step-section-heading">Observation</span>
                <pre className="step-code-block step-observation-block">
                  <code>{formattedObs}</code>
                </pre>
              </div>
            ) : null}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
