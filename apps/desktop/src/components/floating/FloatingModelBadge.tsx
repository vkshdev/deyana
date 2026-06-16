import type { ModelStatus } from "@deyana/schemas";
import { Cpu } from "lucide-react";

interface FloatingModelBadgeProps {
  status: ModelStatus;
}

export function FloatingModelBadge({ status }: FloatingModelBadgeProps) {
  const label = status === "available" ? "qwen3 1.7B" : "Model";

  return (
    <div className={`status-badge model-badge model-${status}`} title="Local model status">
      <Cpu size={14} aria-hidden="true" />
      <span>{label}</span>
    </div>
  );
}

