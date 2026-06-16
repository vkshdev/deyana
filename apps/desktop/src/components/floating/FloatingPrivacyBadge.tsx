import { ShieldCheck } from "lucide-react";

export function FloatingPrivacyBadge() {
  return (
    <div className="status-badge privacy-badge" title="Local privacy mode">
      <ShieldCheck size={14} aria-hidden="true" />
      <span>Local</span>
    </div>
  );
}

