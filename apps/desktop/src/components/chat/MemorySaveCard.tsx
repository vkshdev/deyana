import React, { useState } from "react";
import { motion } from "framer-motion";
import {
  BookmarkCheck,
  Edit2,
  Sparkles,
  Tag,
  X
} from "lucide-react";
import type { MemorySaveSuggestion } from "@deyana/schemas";
import { assistantStore } from "../../stores/assistantStore";

interface MemorySaveCardProps {
  suggestion: MemorySaveSuggestion;
  onApprove?: (id: string, title?: string, factText?: string, tags?: string[]) => void;
  onDismiss?: (id: string) => void;
  className?: string;
}

export function MemorySaveCard({
  suggestion,
  onApprove,
  onDismiss,
  className = ""
}: MemorySaveCardProps) {
  const initialText = suggestion.factText || suggestion.fact || "";
  const [isEditing, setIsEditing] = useState(false);
  const [factText, setFactText] = useState(initialText);
  const [title, setTitle] = useState(
    initialText.length > 50 ? initialText.slice(0, 50) + "..." : initialText
  );
  const [tagsInput, setTagsInput] = useState(
    [suggestion.category, "extracted_fact"].filter(Boolean).join(", ")
  );
  const [busy, setBusy] = useState(false);

  const handleApprove = async () => {
    if (busy) return;
    setBusy(true);
    try {
      const parsedTags = tagsInput
        .split(",")
        .map((t) => t.trim())
        .filter(Boolean);

      if (onApprove) {
        onApprove(suggestion.id, title.trim() || undefined, factText.trim(), parsedTags);
      } else {
        await assistantStore.approveMemorySave(
          suggestion.id,
          title.trim() || undefined,
          factText.trim(),
          parsedTags
        );
      }
    } finally {
      setBusy(false);
    }
  };

  const handleDismiss = async () => {
    if (busy) return;
    setBusy(true);
    try {
      if (onDismiss) {
        onDismiss(suggestion.id);
      } else {
        await assistantStore.dismissMemorySave(suggestion.id);
      }
    } finally {
      setBusy(false);
    }
  };

  const importanceLevel = Math.max(1, Math.min(5, suggestion.importance || 3));

  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: 12, scale: 0.96 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, scale: 0.92, height: 0, marginBottom: 0, overflow: "hidden" }}
      transition={{ duration: 0.24, ease: "easeOut" }}
      className={`memory-save-card ${className}`}
      style={{
        background: "rgba(10, 16, 26, 0.92)",
        border: "1px solid rgba(216, 185, 120, 0.35)",
        boxShadow: "0 6px 20px rgba(0, 0, 0, 0.45), 0 0 12px rgba(216, 185, 120, 0.12)",
        borderRadius: "10px",
        padding: "12px 14px",
        margin: "8px 0",
        backdropFilter: "blur(16px)",
        position: "relative",
        display: "flex",
        flexDirection: "column",
        gap: "10px"
      }}
      role="region"
      aria-label="Fact extraction suggestion"
    >
      {/* Top Header */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          gap: "8px"
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "6px", flexWrap: "wrap" }}>
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: "4px",
              color: "#d8b978",
              fontSize: "11px",
              fontWeight: 600,
              textTransform: "uppercase",
              letterSpacing: "0.5px"
            }}
          >
            <Sparkles size={13} aria-hidden="true" />
            <span>Memory Suggestion</span>
          </div>

          <span
            style={{
              background: "rgba(131, 169, 212, 0.15)",
              color: "#83a9d4",
              border: "1px solid rgba(131, 169, 212, 0.3)",
              fontSize: "10px",
              padding: "1px 6px",
              borderRadius: "4px",
              textTransform: "lowercase"
            }}
          >
            {suggestion.category || "knowledge"}
          </span>

          {/* Importance Indicator (3/5) */}
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: "3px",
              background: "rgba(216, 185, 120, 0.1)",
              border: "1px solid rgba(216, 185, 120, 0.25)",
              borderRadius: "4px",
              padding: "1px 6px",
              fontSize: "10px",
              color: "#d8b978"
            }}
            title={`Importance: ${importanceLevel}/5`}
            aria-label={`Importance ${importanceLevel} out of 5`}
          >
            <span>Importance:</span>
            <strong>{importanceLevel}/5</strong>
            <div style={{ display: "flex", gap: "2px", marginLeft: "2px" }}>
              {[1, 2, 3, 4, 5].map((level) => (
                <span
                  key={level}
                  style={{
                    width: "4px",
                    height: "6px",
                    borderRadius: "1px",
                    background:
                      level <= importanceLevel ? "#d8b978" : "rgba(255, 255, 255, 0.15)"
                  }}
                />
              ))}
            </div>
          </div>
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: "4px" }}>
          <button
            type="button"
            className="icon-button"
            onClick={() => setIsEditing(!isEditing)}
            title={isEditing ? "Close edit" : "Edit fact"}
            aria-label={isEditing ? "Close edit" : "Edit fact"}
            style={{
              background: isEditing ? "rgba(216, 185, 120, 0.2)" : "transparent",
              color: isEditing ? "#d8b978" : "rgba(255, 255, 255, 0.6)",
              padding: "4px",
              border: "none",
              borderRadius: "4px",
              cursor: "pointer",
              display: "grid",
              placeItems: "center"
            }}
          >
            <Edit2 size={13} aria-hidden="true" />
          </button>
          <button
            type="button"
            className="icon-button"
            onClick={handleDismiss}
            disabled={busy}
            title="Dismiss suggestion"
            aria-label="Dismiss suggestion"
            style={{
              background: "transparent",
              color: "rgba(255, 255, 255, 0.5)",
              padding: "4px",
              border: "none",
              borderRadius: "4px",
              cursor: "pointer",
              display: "grid",
              placeItems: "center"
            }}
          >
            <X size={14} aria-hidden="true" />
          </button>
        </div>
      </div>

      {/* Content Area */}
      {isEditing ? (
        <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
          <div>
            <label
              style={{
                display: "block",
                fontSize: "11px",
                color: "rgba(255, 255, 255, 0.6)",
                marginBottom: "2px"
              }}
            >
              Title:
            </label>
            <input
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              style={{
                width: "100%",
                background: "rgba(18, 26, 40, 0.8)",
                border: "1px solid rgba(131, 169, 212, 0.3)",
                color: "#e6edf3",
                borderRadius: "4px",
                padding: "4px 8px",
                fontSize: "12px",
                outline: "none"
              }}
            />
          </div>

          <div>
            <label
              style={{
                display: "block",
                fontSize: "11px",
                color: "rgba(255, 255, 255, 0.6)",
                marginBottom: "2px"
              }}
            >
              Fact Content:
            </label>
            <textarea
              rows={3}
              value={factText}
              onChange={(e) => setFactText(e.target.value)}
              style={{
                width: "100%",
                background: "rgba(18, 26, 40, 0.8)",
                border: "1px solid rgba(131, 169, 212, 0.3)",
                color: "#e6edf3",
                borderRadius: "4px",
                padding: "6px 8px",
                fontSize: "12px",
                lineHeight: "1.4",
                outline: "none",
                resize: "vertical"
              }}
            />
          </div>

          <div>
            <label
              style={{
                display: "block",
                fontSize: "11px",
                color: "rgba(255, 255, 255, 0.6)",
                marginBottom: "2px"
              }}
            >
              Tags (comma separated):
            </label>
            <input
              type="text"
              value={tagsInput}
              onChange={(e) => setTagsInput(e.target.value)}
              style={{
                width: "100%",
                background: "rgba(18, 26, 40, 0.8)",
                border: "1px solid rgba(131, 169, 212, 0.3)",
                color: "#e6edf3",
                borderRadius: "4px",
                padding: "4px 8px",
                fontSize: "12px",
                outline: "none"
              }}
            />
          </div>
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
          <p
            style={{
              margin: 0,
              fontSize: "13px",
              lineHeight: "1.45",
              color: "#e6edf3",
              fontWeight: 500
            }}
          >
            {factText}
          </p>

          {suggestion.rationale ? (
            <p
              style={{
                margin: 0,
                fontSize: "11px",
                color: "rgba(230, 237, 243, 0.6)",
                fontStyle: "italic"
              }}
            >
              {suggestion.rationale}
            </p>
          ) : null}

          {/* Entities badges */}
          {suggestion.entities && suggestion.entities.length > 0 ? (
            <div
              style={{
                display: "flex",
                flexWrap: "wrap",
                gap: "4px",
                marginTop: "2px"
              }}
            >
              {suggestion.entities.map((ent, idx) => (
                <span
                  key={`${ent.name}-${idx}`}
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    gap: "3px",
                    background: "rgba(255, 255, 255, 0.06)",
                    border: "1px solid rgba(255, 255, 255, 0.12)",
                    borderRadius: "3px",
                    padding: "1px 5px",
                    fontSize: "10px",
                    color: "rgba(230, 237, 243, 0.8)"
                  }}
                >
                  <Tag size={9} aria-hidden="true" style={{ color: "#83a9d4" }} />
                  <strong>{ent.name}</strong>
                  <span style={{ color: "rgba(255, 255, 255, 0.4)", fontSize: "9px" }}>
                    ({ent.entityType})
                  </span>
                </span>
              ))}
            </div>
          ) : null}
        </div>
      )}

      {/* Action Footer */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "flex-end",
          gap: "8px",
          marginTop: "2px",
          paddingTop: "6px",
          borderTop: "1px solid rgba(255, 255, 255, 0.07)"
        }}
      >
        <button
          type="button"
          onClick={handleDismiss}
          disabled={busy}
          style={{
            background: "transparent",
            border: "1px solid rgba(255, 255, 255, 0.15)",
            borderRadius: "6px",
            color: "rgba(255, 255, 255, 0.7)",
            padding: "4px 10px",
            fontSize: "11px",
            cursor: "pointer",
            display: "inline-flex",
            alignItems: "center",
            gap: "4px"
          }}
        >
          <X size={12} aria-hidden="true" />
          <span>Dismiss</span>
        </button>

        <button
          type="button"
          onClick={handleApprove}
          disabled={busy}
          style={{
            background:
              "linear-gradient(135deg, rgba(216, 185, 120, 0.35), rgba(131, 169, 212, 0.2))",
            border: "1px solid rgba(216, 185, 120, 0.6)",
            borderRadius: "6px",
            color: "#f4dfb5",
            fontWeight: 600,
            padding: "4px 12px",
            fontSize: "11px",
            cursor: "pointer",
            display: "inline-flex",
            alignItems: "center",
            gap: "5px",
            boxShadow: "0 2px 8px rgba(216, 185, 120, 0.15)"
          }}
        >
          <BookmarkCheck size={13} aria-hidden="true" />
          <span>Approve & Save</span>
        </button>
      </div>
    </motion.div>
  );
}
