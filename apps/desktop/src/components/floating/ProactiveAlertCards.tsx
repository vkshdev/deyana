import type { AssistantSnapshot } from "../../stores/assistantStore";
import { AnimatePresence, motion } from "framer-motion";
import { AlertTriangle, Bell, Sparkles, X, Zap } from "lucide-react";
import { assistantStore } from "../../stores/assistantStore";

interface ProactiveAlertCardsProps {
  snapshot: AssistantSnapshot;
  compact?: boolean;
  className?: string;
}

export function ProactiveAlertCards({
  snapshot,
  compact = false,
  className = ""
}: ProactiveAlertCardsProps) {
  const cards = snapshot.proactiveCards ?? [];

  if (!cards || cards.length === 0) {
    return null;
  }

  // Cap visible card stack to avoid UI overflow
  const visibleCards = cards.slice(0, compact ? 1 : 3);
  const remainingCount = cards.length - visibleCards.length;

  return (
    <div
      className={`proactive-alert-cards-wrapper ${className}`}
      style={{
        display: "flex",
        flexDirection: "column",
        gap: compact ? "6px" : "8px",
        width: "100%",
        zIndex: 20
      }}
    >
      <AnimatePresence initial={false}>
        {visibleCards.map((card) => {
          const isUrgent = card.priority === "urgent" || card.priority === "high";
          const priorityColor = isUrgent ? "#d8b978" : "#83a9d4";
          const borderColor = isUrgent ? "rgba(216, 185, 120, 0.45)" : "rgba(131, 169, 212, 0.35)";

          return (
            <motion.div
              key={card.id}
              initial={{ opacity: 0, y: compact ? 10 : -10, scale: 0.95 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, scale: 0.9, height: 0, marginBottom: 0, padding: 0 }}
              transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1] }}
              style={{
                background: "rgba(4, 8, 14, 0.88)",
                border: `1px solid ${borderColor}`,
                boxShadow: isUrgent
                  ? "0 8px 24px rgba(0,0,0,0.5), 0 0 16px rgba(216, 185, 120, 0.2)"
                  : "0 8px 24px rgba(0,0,0,0.5), 0 0 16px rgba(131, 169, 212, 0.15)",
                backdropFilter: "blur(20px)",
                borderRadius: "8px",
                padding: compact ? "6px 10px" : "10px 12px",
                display: "flex",
                flexDirection: "column",
                gap: compact ? "4px" : "6px",
                position: "relative",
                overflow: "hidden"
              }}
            >
              {/* Top ambient highlight line */}
              <div
                style={{
                  position: "absolute",
                  top: 0,
                  left: 0,
                  right: 0,
                  height: "2px",
                  background: isUrgent
                    ? "linear-gradient(90deg, transparent, #d8b978, transparent)"
                    : "linear-gradient(90deg, transparent, #83a9d4, transparent)"
                }}
              />

              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  gap: "8px"
                }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                  {isUrgent ? (
                    <AlertTriangle size={13} style={{ color: "#d8b978" }} aria-hidden="true" />
                  ) : (
                    <Sparkles size={13} style={{ color: "#83a9d4" }} aria-hidden="true" />
                  )}
                  <span
                    style={{
                      fontSize: "10px",
                      fontWeight: 700,
                      letterSpacing: "0.6px",
                      color: priorityColor,
                      textTransform: "uppercase"
                    }}
                  >
                    {card.priority}
                  </span>
                  <span
                    style={{
                      fontSize: "10px",
                      color: "#a8b2c0",
                      fontWeight: 500
                    }}
                  >
                    • {card.cardType || "Alert"}
                  </span>
                </div>

                <button
                  type="button"
                  className="icon-button"
                  title="Dismiss alert"
                  aria-label="Dismiss alert"
                  onClick={() => assistantStore.dismissProactiveCard(card.id)}
                  style={{
                    background: "transparent",
                    border: "none",
                    color: "#a8b2c0",
                    cursor: "pointer",
                    padding: "2px",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    borderRadius: "4px"
                  }}
                >
                  <X size={13} aria-hidden="true" />
                </button>
              </div>

              <div>
                <strong
                  style={{
                    fontSize: compact ? "11px" : "12px",
                    fontWeight: 600,
                    color: "#f3f1ec",
                    display: "block",
                    marginBottom: "2px"
                  }}
                >
                  {card.title}
                </strong>
                <p
                  style={{
                    fontSize: "11px",
                    color: "#a8b2c0",
                    margin: 0,
                    lineHeight: "1.35",
                    display: "-webkit-box",
                    WebkitLineClamp: compact ? 1 : 2,
                    WebkitBoxOrient: "vertical",
                    overflow: "hidden"
                  }}
                >
                  {card.summary}
                </p>
              </div>

              {(card.actionLabel || card.actionCommand) && (
                <div style={{ display: "flex", justifyContent: "flex-end", marginTop: "2px" }}>
                  <button
                    type="button"
                    onClick={() => {
                      const cmd = card.actionCommand || card.actionLabel;
                      if (cmd) {
                        void assistantStore.agentChat(cmd);
                      }
                      assistantStore.dismissProactiveCard(card.id);
                    }}
                    style={{
                      background: isUrgent
                        ? "linear-gradient(135deg, rgba(216,185,120,0.25), rgba(216,185,120,0.1))"
                        : "linear-gradient(135deg, rgba(131,169,212,0.25), rgba(131,169,212,0.1))",
                      border: `1px solid ${priorityColor}`,
                      color: priorityColor,
                      fontSize: "10px",
                      fontWeight: 600,
                      padding: "3px 8px",
                      borderRadius: "4px",
                      cursor: "pointer",
                      display: "inline-flex",
                      alignItems: "center",
                      gap: "4px"
                    }}
                  >
                    <Zap size={11} aria-hidden="true" />
                    <span>{card.actionLabel || "Execute Action"}</span>
                  </button>
                </div>
              )}
            </motion.div>
          );
        })}
      </AnimatePresence>

      {remainingCount > 0 && !compact && (
        <div
          style={{
            fontSize: "10px",
            color: "#a8b2c0",
            display: "flex",
            alignItems: "center",
            gap: "4px",
            paddingLeft: "4px"
          }}
        >
          <Bell size={11} style={{ color: "#83a9d4" }} aria-hidden="true" />
          <span>+{remainingCount} pending proactive card{remainingCount > 1 ? "s" : ""}</span>
        </div>
      )}
    </div>
  );
}
