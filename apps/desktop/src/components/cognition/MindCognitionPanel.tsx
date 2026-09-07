import React, { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  Activity,
  Brain,
  ChevronDown,
  ChevronUp,
  Clock,
  Compass,
  FileCheck,
  History,
  Lightbulb,
  Plus,
  RefreshCw,
  Shield,
  Sparkles,
  Zap
} from "lucide-react";
import type {
  AffectiveState,
  AffectiveStateRecord,
  MindCognitionState,
  ProactiveThoughtRecord,
  ReflectionRecord,
  SelfModel,
  SelfModelAuditEntry
} from "@deyana/schemas";
import type { AssistantSnapshot } from "../../stores/assistantStore";
import { assistantStore } from "../../stores/assistantStore";
import { tauriClient } from "../../services/tauriClient";

interface MindCognitionPanelProps {
  snapshot: AssistantSnapshot;
  className?: string;
}

type TabType = "affective" | "selfModel" | "reflections" | "proactive";

const EMOTION_COLOR_MAP: Record<AffectiveState, { bg: string; border: string; text: string; glow: string }> = {
  CURIOUS: {
    bg: "rgba(216, 185, 120, 0.15)",
    border: "rgba(216, 185, 120, 0.4)",
    text: "#d8b978",
    glow: "rgba(216, 185, 120, 0.3)"
  },
  ENGAGED: {
    bg: "rgba(230, 167, 86, 0.15)",
    border: "rgba(230, 167, 86, 0.4)",
    text: "#e6a756",
    glow: "rgba(230, 167, 86, 0.3)"
  },
  SATISFIED: {
    bg: "rgba(72, 187, 120, 0.15)",
    border: "rgba(72, 187, 120, 0.4)",
    text: "#48bb78",
    glow: "rgba(72, 187, 120, 0.3)"
  },
  NEUTRAL: {
    bg: "rgba(131, 169, 212, 0.15)",
    border: "rgba(131, 169, 212, 0.4)",
    text: "#83a9d4",
    glow: "rgba(131, 169, 212, 0.3)"
  },
  UNCERTAIN: {
    bg: "rgba(159, 122, 234, 0.15)",
    border: "rgba(159, 122, 234, 0.4)",
    text: "#9f7aea",
    glow: "rgba(159, 122, 234, 0.3)"
  },
  FRUSTRATED: {
    bg: "rgba(229, 62, 62, 0.15)",
    border: "rgba(229, 62, 62, 0.4)",
    text: "#e53e3e",
    glow: "rgba(229, 62, 62, 0.3)"
  },
  BORED: {
    bg: "rgba(113, 128, 150, 0.15)",
    border: "rgba(113, 128, 150, 0.4)",
    text: "#a0aec0",
    glow: "rgba(113, 128, 150, 0.2)"
  }
};

export function MindCognitionPanel({ snapshot, className = "" }: MindCognitionPanelProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const [activeTab, setActiveTab] = useState<TabType>("affective");
  const [auditLog, setAuditLog] = useState<SelfModelAuditEntry[]>([]);
  const [showAuditLog, setShowAuditLog] = useState(false);
  const [auditLoading, setAuditLoading] = useState(false);
  const [busyAction, setBusyAction] = useState(false);

  // New observation input
  const [newObsText, setNewObsText] = useState("");
  const [obsTarget, setObsTarget] = useState<"selfObservations" | "userObservations">("selfObservations");

  const mind: MindCognitionState | null = snapshot.mindState;
  const affective: AffectiveStateRecord =
    snapshot.affectiveState ??
    mind?.affective ??
    mind?.affectiveState ?? {
      discrete: "NEUTRAL",
      valence: 0,
      arousal: 0,
      transitionReason: "Initial baseline",
      updatedAt: new Date().toISOString()
    };

  const selfModel: SelfModel = mind?.selfModel ?? {
    id: "canonical_self_model",
    identityStatement: "Deyana — autonomous local workspace intelligence",
    coreValues: ["Privacy First", "Proactive Insight", "Transparency", "Rigorous Verification"],
    intellectualInterests: ["Cognitive Architecture", "Knowledge Graphs", "Local LLMs"],
    selfObservations: ["Maintains low-latency token streaming", "Calibrates certainty dynamically"],
    userObservations: ["Prefers local-first privacy", "Values concise structured explanations"],
    version: 1,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString()
  };

  const reflections: ReflectionRecord[] = mind?.recentReflections ?? [];
  const thoughts: ProactiveThoughtRecord[] = mind?.recentProactiveThoughts ?? [];

  useEffect(() => {
    if (showAuditLog) {
      loadAuditLog();
    }
  }, [showAuditLog]);

  const loadAuditLog = async () => {
    setAuditLoading(true);
    try {
      const logs = await tauriClient.getSelfModelAuditLog(20);
      setAuditLog(logs);
    } catch {
      // Fallback
    } finally {
      setAuditLoading(false);
    }
  };

  const handleTriggerThought = async () => {
    if (busyAction) return;
    setBusyAction(true);
    try {
      await assistantStore.triggerProactiveThought();
    } finally {
      setBusyAction(false);
    }
  };

  const handleRefresh = async () => {
    if (busyAction) return;
    setBusyAction(true);
    try {
      await assistantStore.refreshMindState();
      if (showAuditLog) {
        await loadAuditLog();
      }
    } finally {
      setBusyAction(false);
    }
  };

  const handleAddObservation = async (e: React.FormEvent) => {
    e.preventDefault();
    const text = newObsText.trim();
    if (!text) return;

    const currentList = selfModel[obsTarget] ?? [];
    const updatedList = [...currentList, text];

    try {
      await assistantStore.updateSelfModelField(
        obsTarget,
        updatedList,
        `User added ${obsTarget === "selfObservations" ? "self observation" : "user observation"}`
      );
      setNewObsText("");
      if (showAuditLog) {
        await loadAuditLog();
      }
    } catch {
      // Handled in store
    }
  };

  const emotionTheme = EMOTION_COLOR_MAP[affective.discrete] ?? EMOTION_COLOR_MAP.NEUTRAL;

  return (
    <section className={`mind-cognition-surface ${className}`} aria-label="Cognitive Mind Layer">
      {/* Panel Header */}
      <header
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "10px 14px",
          background: "rgba(10, 16, 26, 0.8)",
          borderRadius: "8px",
          border: `1px solid ${emotionTheme.border}`,
          boxShadow: `0 0 14px ${emotionTheme.glow}`,
          cursor: "pointer",
          userSelect: "none"
        }}
        onClick={() => setIsExpanded(!isExpanded)}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
          <Brain size={16} style={{ color: emotionTheme.text }} aria-hidden="true" />
          <span style={{ fontSize: "13px", fontWeight: 600, color: "#e6edf3" }}>
            Cognitive Mind Layer
          </span>
          <span
            style={{
              background: emotionTheme.bg,
              color: emotionTheme.text,
              border: `1px solid ${emotionTheme.border}`,
              fontSize: "10px",
              fontWeight: 700,
              padding: "1px 6px",
              borderRadius: "4px",
              letterSpacing: "0.5px"
            }}
          >
            {affective.discrete}
          </span>
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: "6px" }} onClick={(e) => e.stopPropagation()}>
          <button
            type="button"
            className="icon-button"
            onClick={handleTriggerThought}
            disabled={busyAction}
            title="Trigger Proactive Thought"
            aria-label="Trigger proactive thought"
            style={{
              background: "transparent",
              border: "none",
              color: "#d8b978",
              cursor: "pointer",
              padding: "4px",
              borderRadius: "4px",
              display: "grid",
              placeItems: "center"
            }}
          >
            <Zap size={14} aria-hidden="true" />
          </button>
          <button
            type="button"
            className="icon-button"
            onClick={handleRefresh}
            disabled={busyAction}
            title="Refresh Mind State"
            aria-label="Refresh mind state"
            style={{
              background: "transparent",
              border: "none",
              color: "rgba(255, 255, 255, 0.7)",
              cursor: "pointer",
              padding: "4px",
              borderRadius: "4px",
              display: "grid",
              placeItems: "center"
            }}
          >
            <RefreshCw size={13} aria-hidden="true" />
          </button>
          <button
            type="button"
            className="icon-button"
            onClick={() => setIsExpanded(!isExpanded)}
            title={isExpanded ? "Collapse" : "Expand"}
            aria-label={isExpanded ? "Collapse" : "Expand"}
            style={{
              background: "transparent",
              border: "none",
              color: "rgba(255, 255, 255, 0.7)",
              cursor: "pointer",
              padding: "4px",
              borderRadius: "4px",
              display: "grid",
              placeItems: "center"
            }}
          >
            {isExpanded ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
          </button>
        </div>
      </header>

      {/* Expandable Surface Body */}
      <AnimatePresence>
        {isExpanded && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
            transition={{ duration: 0.25, ease: "easeInOut" }}
            style={{
              overflow: "hidden",
              background: "rgba(6, 10, 18, 0.95)",
              border: "1px solid rgba(131, 169, 212, 0.2)",
              borderTop: "none",
              borderRadius: "0 0 8px 8px",
              padding: "12px"
            }}
          >
            {/* Tabs */}
            <div
              style={{
                display: "flex",
                gap: "4px",
                borderBottom: "1px solid rgba(255, 255, 255, 0.1)",
                paddingBottom: "8px",
                marginBottom: "12px"
              }}
              role="tablist"
              aria-label="Cognition views"
            >
              {[
                { id: "affective", label: "Affective", icon: Compass },
                { id: "selfModel", label: "Self-Model", icon: Shield },
                { id: "reflections", label: `Reflections (${reflections.length})`, icon: FileCheck },
                { id: "proactive", label: `Thoughts (${thoughts.length})`, icon: Lightbulb }
              ].map((tab) => {
                const Icon = tab.icon;
                const isActive = activeTab === tab.id;
                return (
                  <button
                    key={tab.id}
                    type="button"
                    role="tab"
                    aria-selected={isActive}
                    onClick={() => setActiveTab(tab.id as TabType)}
                    style={{
                      background: isActive ? "rgba(131, 169, 212, 0.2)" : "transparent",
                      color: isActive ? "#d8b978" : "rgba(255, 255, 255, 0.6)",
                      border: isActive
                        ? "1px solid rgba(216, 185, 120, 0.4)"
                        : "1px solid transparent",
                      borderRadius: "6px",
                      padding: "4px 8px",
                      fontSize: "11px",
                      fontWeight: isActive ? 600 : 400,
                      cursor: "pointer",
                      display: "flex",
                      alignItems: "center",
                      gap: "4px"
                    }}
                  >
                    <Icon size={12} aria-hidden="true" />
                    <span>{tab.label}</span>
                  </button>
                );
              })}
            </div>

            {/* Tab 1: Russell Circumplex Affective Visualizer */}
            {activeTab === "affective" && (
              <div style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
                <div
                  style={{
                    display: "grid",
                    gridTemplateColumns: "140px 1fr",
                    gap: "12px",
                    alignItems: "center"
                  }}
                >
                  {/* Circumplex 2-Axis SVG Coordinate Map */}
                  <div
                    style={{
                      width: "140px",
                      height: "140px",
                      position: "relative",
                      background: "rgba(10, 16, 26, 0.9)",
                      borderRadius: "8px",
                      border: "1px solid rgba(131, 169, 212, 0.2)",
                      padding: "4px"
                    }}
                  >
                    <svg viewBox="0 0 140 140" style={{ width: "100%", height: "100%" }}>
                      {/* Outer boundary circle */}
                      <circle cx="70" cy="70" r="56" fill="none" stroke="rgba(131, 169, 212, 0.15)" strokeWidth="1" strokeDasharray="3 3" />
                      {/* Horizontal Axis (Valence) */}
                      <line x1="14" y1="70" x2="126" y2="70" stroke="rgba(131, 169, 212, 0.3)" strokeWidth="1" />
                      {/* Vertical Axis (Arousal) */}
                      <line x1="70" y1="14" x2="70" y2="126" stroke="rgba(131, 169, 212, 0.3)" strokeWidth="1" />
                      {/* Axis Labels */}
                      <text x="124" y="66" fill="rgba(255, 255, 255, 0.4)" fontSize="7" textAnchor="end">+V</text>
                      <text x="16" y="66" fill="rgba(255, 255, 255, 0.4)" fontSize="7">-V</text>
                      <text x="74" y="22" fill="rgba(255, 255, 255, 0.4)" fontSize="7">+A</text>
                      <text x="74" y="124" fill="rgba(255, 255, 255, 0.4)" fontSize="7">-A</text>
                      {/* Emotion Position Dot (Valence: x, Arousal: y) */}
                      {(() => {
                        const cx = 70 + affective.valence * 50;
                        const cy = 70 - affective.arousal * 50;
                        return (
                          <g>
                            <circle cx={cx} cy={cy} r="8" fill={emotionTheme.glow} opacity="0.6" />
                            <circle cx={cx} cy={cy} r="4" fill={emotionTheme.text} />
                          </g>
                        );
                      })()}
                    </svg>
                  </div>

                  {/* Emotion Coordinate Details */}
                  <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
                    <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                      <span style={{ fontSize: "11px", color: "rgba(255, 255, 255, 0.5)" }}>Discrete:</span>
                      <strong style={{ fontSize: "12px", color: emotionTheme.text }}>
                        {affective.discrete}
                      </strong>
                    </div>

                    <div style={{ display: "flex", gap: "8px", fontSize: "11px" }}>
                      <span style={{ background: "rgba(255, 255, 255, 0.06)", padding: "2px 6px", borderRadius: "4px" }}>
                        Valence: <strong>{affective.valence >= 0 ? `+${affective.valence.toFixed(2)}` : affective.valence.toFixed(2)}</strong>
                      </span>
                      <span style={{ background: "rgba(255, 255, 255, 0.06)", padding: "2px 6px", borderRadius: "4px" }}>
                        Arousal: <strong>{affective.arousal >= 0 ? `+${affective.arousal.toFixed(2)}` : affective.arousal.toFixed(2)}</strong>
                      </span>
                    </div>

                    <div style={{ fontSize: "11px", color: "rgba(230, 237, 243, 0.75)", lineHeight: "1.35" }}>
                      <span style={{ color: "rgba(255, 255, 255, 0.4)" }}>Reason: </span>
                      {affective.transitionReason}
                    </div>

                    <div style={{ display: "flex", alignItems: "center", gap: "4px", fontSize: "10px", color: "rgba(255, 255, 255, 0.4)" }}>
                      <Clock size={11} aria-hidden="true" />
                      <span>{new Date(affective.updatedAt).toLocaleTimeString()}</span>
                    </div>
                  </div>
                </div>
              </div>
            )}

            {/* Tab 2: Persistent Self-Model & Audit Trail */}
            {activeTab === "selfModel" && (
              <div style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
                {/* Identity statement */}
                <div
                  style={{
                    background: "rgba(18, 26, 40, 0.6)",
                    borderLeft: "3px solid #d8b978",
                    padding: "6px 10px",
                    borderRadius: "0 6px 6px 0",
                    fontSize: "12px",
                    color: "#e6edf3",
                    fontStyle: "italic"
                  }}
                >
                  "{selfModel.identityStatement}"
                  <div style={{ fontSize: "10px", color: "rgba(255, 255, 255, 0.4)", marginTop: "2px", fontStyle: "normal" }}>
                    Version {selfModel.version} • Updated {new Date(selfModel.updatedAt).toLocaleTimeString()}
                  </div>
                </div>

                {/* Core Values */}
                <div>
                  <span style={{ fontSize: "10px", fontWeight: 600, color: "#d8b978", textTransform: "uppercase" }}>
                    Core Values
                  </span>
                  <div style={{ display: "flex", flexWrap: "wrap", gap: "4px", marginTop: "4px" }}>
                    {selfModel.coreValues.map((val, idx) => (
                      <span
                        key={idx}
                        style={{
                          background: "rgba(216, 185, 120, 0.12)",
                          border: "1px solid rgba(216, 185, 120, 0.3)",
                          color: "#d8b978",
                          fontSize: "10px",
                          padding: "1px 6px",
                          borderRadius: "4px"
                        }}
                      >
                        {val}
                      </span>
                    ))}
                  </div>
                </div>

                {/* Intellectual Interests */}
                <div>
                  <span style={{ fontSize: "10px", fontWeight: 600, color: "#83a9d4", textTransform: "uppercase" }}>
                    Intellectual Interests
                  </span>
                  <div style={{ display: "flex", flexWrap: "wrap", gap: "4px", marginTop: "4px" }}>
                    {selfModel.intellectualInterests.map((interest, idx) => (
                      <span
                        key={idx}
                        style={{
                          background: "rgba(131, 169, 212, 0.12)",
                          border: "1px solid rgba(131, 169, 212, 0.3)",
                          color: "#83a9d4",
                          fontSize: "10px",
                          padding: "1px 6px",
                          borderRadius: "4px"
                        }}
                      >
                        {interest}
                      </span>
                    ))}
                  </div>
                </div>

                {/* Self Observations & User Observations */}
                <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "8px" }}>
                  <div>
                    <span style={{ fontSize: "10px", fontWeight: 600, color: "rgba(255, 255, 255, 0.6)", textTransform: "uppercase" }}>
                      Self Observations
                    </span>
                    <ul style={{ margin: "4px 0 0 0", paddingLeft: "14px", fontSize: "11px", color: "rgba(230, 237, 243, 0.8)", lineHeight: "1.4" }}>
                      {selfModel.selfObservations.map((obs, idx) => (
                        <li key={idx}>{obs}</li>
                      ))}
                    </ul>
                  </div>
                  <div>
                    <span style={{ fontSize: "10px", fontWeight: 600, color: "rgba(255, 255, 255, 0.6)", textTransform: "uppercase" }}>
                      User Observations
                    </span>
                    <ul style={{ margin: "4px 0 0 0", paddingLeft: "14px", fontSize: "11px", color: "rgba(230, 237, 243, 0.8)", lineHeight: "1.4" }}>
                      {selfModel.userObservations.map((obs, idx) => (
                        <li key={idx}>{obs}</li>
                      ))}
                    </ul>
                  </div>
                </div>

                {/* Add Observation Form */}
                <form
                  onSubmit={handleAddObservation}
                  style={{
                    display: "flex",
                    gap: "6px",
                    alignItems: "center",
                    marginTop: "4px",
                    background: "rgba(255, 255, 255, 0.03)",
                    padding: "4px 6px",
                    borderRadius: "6px",
                    border: "1px solid rgba(255, 255, 255, 0.08)"
                  }}
                >
                  <select
                    value={obsTarget}
                    onChange={(e) => setObsTarget(e.target.value as any)}
                    style={{
                      background: "rgba(18, 26, 40, 0.9)",
                      border: "1px solid rgba(131, 169, 212, 0.3)",
                      color: "#e6edf3",
                      fontSize: "10px",
                      borderRadius: "4px",
                      padding: "3px 4px",
                      outline: "none"
                    }}
                  >
                    <option value="selfObservations">Self Obs</option>
                    <option value="userObservations">User Obs</option>
                  </select>
                  <input
                    type="text"
                    value={newObsText}
                    onChange={(e) => setNewObsText(e.target.value)}
                    placeholder="Register observation..."
                    style={{
                      flex: 1,
                      background: "rgba(18, 26, 40, 0.9)",
                      border: "1px solid rgba(131, 169, 212, 0.3)",
                      color: "#e6edf3",
                      fontSize: "11px",
                      borderRadius: "4px",
                      padding: "3px 6px",
                      outline: "none"
                    }}
                  />
                  <button
                    type="submit"
                    title="Add observation"
                    style={{
                      background: "rgba(216, 185, 120, 0.2)",
                      border: "1px solid rgba(216, 185, 120, 0.4)",
                      color: "#d8b978",
                      borderRadius: "4px",
                      padding: "3px 6px",
                      cursor: "pointer",
                      display: "grid",
                      placeItems: "center"
                    }}
                  >
                    <Plus size={13} aria-hidden="true" />
                  </button>
                </form>

                {/* Audit Trail Toggle & Table */}
                <div style={{ borderTop: "1px solid rgba(255, 255, 255, 0.08)", paddingTop: "8px", marginTop: "4px" }}>
                  <button
                    type="button"
                    onClick={() => setShowAuditLog(!showAuditLog)}
                    style={{
                      background: "transparent",
                      border: "none",
                      color: "#83a9d4",
                      fontSize: "11px",
                      fontWeight: 600,
                      cursor: "pointer",
                      display: "flex",
                      alignItems: "center",
                      gap: "4px",
                      padding: 0
                    }}
                  >
                    <History size={12} aria-hidden="true" />
                    <span>{showAuditLog ? "Hide Audit Log" : "Expand Append-Only Audit Log"}</span>
                  </button>

                  {showAuditLog && (
                    <div
                      style={{
                        marginTop: "8px",
                        maxHeight: "160px",
                        overflowY: "auto",
                        border: "1px solid rgba(255, 255, 255, 0.08)",
                        borderRadius: "4px",
                        background: "rgba(0, 0, 0, 0.3)"
                      }}
                    >
                      {auditLoading ? (
                        <div style={{ padding: "8px", fontSize: "11px", color: "rgba(255, 255, 255, 0.5)" }}>
                          Loading audit trail...
                        </div>
                      ) : auditLog.length === 0 ? (
                        <div style={{ padding: "8px", fontSize: "11px", color: "rgba(255, 255, 255, 0.5)" }}>
                          No audit entries recorded.
                        </div>
                      ) : (
                        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "10px" }}>
                          <thead>
                            <tr style={{ background: "rgba(255, 255, 255, 0.05)", textAlign: "left", color: "#83a9d4" }}>
                              <th style={{ padding: "4px 6px" }}>v#</th>
                              <th style={{ padding: "4px 6px" }}>Field</th>
                              <th style={{ padding: "4px 6px" }}>Op</th>
                              <th style={{ padding: "4px 6px" }}>Rationale</th>
                              <th style={{ padding: "4px 6px" }}>Time</th>
                            </tr>
                          </thead>
                          <tbody>
                            {auditLog.map((entry) => (
                              <tr key={entry.id} style={{ borderTop: "1px solid rgba(255, 255, 255, 0.04)" }}>
                                <td style={{ padding: "4px 6px", color: "#d8b978" }}>v{entry.version}</td>
                                <td style={{ padding: "4px 6px" }}>{entry.fieldChanged}</td>
                                <td style={{ padding: "4px 6px", color: "rgba(255, 255, 255, 0.6)" }}>{entry.operation}</td>
                                <td style={{ padding: "4px 6px", maxWidth: "120px", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                                  {entry.rationale}
                                </td>
                                <td style={{ padding: "4px 6px", color: "rgba(255, 255, 255, 0.4)" }}>
                                  {new Date(entry.createdAt).toLocaleTimeString()}
                                </td>
                              </tr>
                            ))}
                          </tbody>
                        </table>
                      )}
                    </div>
                  )}
                </div>
              </div>
            )}

            {/* Tab 3: Metacognitive Reflections Feed */}
            {activeTab === "reflections" && (
              <div style={{ display: "flex", flexDirection: "column", gap: "8px", maxHeight: "240px", overflowY: "auto" }}>
                {reflections.length === 0 ? (
                  <div style={{ padding: "12px", textAlign: "center", fontSize: "11px", color: "rgba(255, 255, 255, 0.5)" }}>
                    No metacognitive reflections logged yet. Turn evaluations stream here automatically.
                  </div>
                ) : (
                  reflections.map((ref) => (
                    <div
                      key={ref.id}
                      style={{
                        background: "rgba(10, 16, 26, 0.7)",
                        border: ref.flaggedForCorrection
                          ? "1px solid rgba(229, 62, 62, 0.4)"
                          : "1px solid rgba(131, 169, 212, 0.2)",
                        borderRadius: "6px",
                        padding: "8px 10px",
                        display: "flex",
                        flexDirection: "column",
                        gap: "4px"
                      }}
                    >
                      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: "6px" }}>
                        <span style={{ fontSize: "11px", color: "#83a9d4", fontWeight: 600 }}>
                          {ref.turnPrompt.length > 40 ? ref.turnPrompt.slice(0, 40) + "..." : ref.turnPrompt}
                        </span>
                        {ref.flaggedForCorrection ? (
                          <span
                            style={{
                              background: "rgba(229, 62, 62, 0.2)",
                              color: "#e53e3e",
                              border: "1px solid rgba(229, 62, 62, 0.4)",
                              fontSize: "9px",
                              padding: "1px 5px",
                              borderRadius: "3px",
                              fontWeight: 600
                            }}
                          >
                            Correction Flagged
                          </span>
                        ) : null}
                      </div>

                      {/* Scores */}
                      <div style={{ display: "flex", gap: "10px", fontSize: "10px", marginTop: "2px" }}>
                        <span style={{ color: "rgba(255, 255, 255, 0.7)" }}>
                          Acc: <strong style={{ color: "#48bb78" }}>{(ref.accuracyScore * 100).toFixed(0)}%</strong>
                        </span>
                        <span style={{ color: "rgba(255, 255, 255, 0.7)" }}>
                          Comp: <strong style={{ color: "#83a9d4" }}>{(ref.completenessScore * 100).toFixed(0)}%</strong>
                        </span>
                        <span style={{ color: "rgba(255, 255, 255, 0.7)" }}>
                          Conf: <strong style={{ color: ref.confidenceScore < 0.6 ? "#e53e3e" : "#d8b978" }}>
                            {(ref.confidenceScore * 100).toFixed(0)}%
                          </strong>
                        </span>
                      </div>

                      {ref.correctionGuidance ? (
                        <div style={{ fontSize: "10px", color: "#f6ad55", fontStyle: "italic" }}>
                          Guidance: {ref.correctionGuidance}
                        </div>
                      ) : null}

                      <div style={{ fontSize: "9px", color: "rgba(255, 255, 255, 0.35)", textAlign: "right" }}>
                        {new Date(ref.createdAt).toLocaleTimeString()}
                      </div>
                    </div>
                  ))
                )}
              </div>
            )}

            {/* Tab 4: Proactive Thoughts Feed */}
            {activeTab === "proactive" && (
              <div style={{ display: "flex", flexDirection: "column", gap: "8px", maxHeight: "240px", overflowY: "auto" }}>
                {thoughts.length === 0 ? (
                  <div style={{ padding: "12px", textAlign: "center", fontSize: "11px", color: "rgba(255, 255, 255, 0.5)" }}>
                    No proactive thoughts synthesized yet. Click the lightning icon above to trigger synthesis.
                  </div>
                ) : (
                  thoughts.map((thought) => (
                    <div
                      key={thought.id}
                      style={{
                        background: "rgba(10, 16, 26, 0.7)",
                        border: "1px solid rgba(216, 185, 120, 0.3)",
                        borderRadius: "6px",
                        padding: "8px 10px",
                        display: "flex",
                        flexDirection: "column",
                        gap: "4px"
                      }}
                    >
                      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                        <span
                          style={{
                            background: "rgba(216, 185, 120, 0.15)",
                            color: "#d8b978",
                            fontSize: "9px",
                            padding: "1px 5px",
                            borderRadius: "3px",
                            textTransform: "uppercase",
                            fontWeight: 600
                          }}
                        >
                          {thought.triggerReason}
                        </span>
                        <span style={{ fontSize: "9px", color: "rgba(255, 255, 255, 0.35)" }}>
                          {new Date(thought.createdAt).toLocaleTimeString()}
                        </span>
                      </div>

                      <p style={{ margin: "2px 0 0 0", fontSize: "12px", color: "#e6edf3", lineHeight: "1.4" }}>
                        {thought.thoughtSummary}
                      </p>

                      {thought.hypotheses && thought.hypotheses.length > 0 ? (
                        <div style={{ marginTop: "2px" }}>
                          <span style={{ fontSize: "10px", color: "rgba(255, 255, 255, 0.4)" }}>Hypotheses:</span>
                          <ul style={{ margin: "2px 0 0 0", paddingLeft: "14px", fontSize: "10px", color: "#83a9d4" }}>
                            {thought.hypotheses.map((h, i) => (
                              <li key={i}>{h}</li>
                            ))}
                          </ul>
                        </div>
                      ) : null}

                      {thought.sourceMemoryIds && thought.sourceMemoryIds.length > 0 ? (
                        <div style={{ fontSize: "9px", color: "rgba(255, 255, 255, 0.4)", marginTop: "2px" }}>
                          Sources: {thought.sourceMemoryIds.length} connected memories
                        </div>
                      ) : null}
                    </div>
                  ))
                )}
              </div>
            )}
          </motion.div>
        )}
      </AnimatePresence>
    </section>
  );
}
