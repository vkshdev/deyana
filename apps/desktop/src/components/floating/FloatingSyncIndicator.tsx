import type { SyncStatus } from "@deyana/schemas";
import { RefreshCcw } from "lucide-react";

interface FloatingSyncIndicatorProps {
  status: SyncStatus;
}

export function FloatingSyncIndicator({ status }: FloatingSyncIndicatorProps) {
  return (
    <div className={`sync-indicator sync-${status}`} title="Connector sync status">
      <RefreshCcw size={14} aria-hidden="true" />
      <span>{status}</span>
    </div>
  );
}

