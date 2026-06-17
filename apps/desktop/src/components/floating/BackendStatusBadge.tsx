import type { BackendProcessStatus } from "@deyana/schemas";
import { RotateCw, Server, ServerCrash } from "lucide-react";
import { assistantStore } from "../../stores/assistantStore";

interface BackendStatusBadgeProps {
  backend: BackendProcessStatus;
  eventStreamConnected: boolean;
}

const labelForLifecycle = (backend: BackendProcessStatus, eventStreamConnected: boolean) => {
  if (backend.lifecycle === "running" && eventStreamConnected) {
    return "Core connected";
  }

  if (backend.lifecycle === "running") {
    return "Core online";
  }

  if (backend.lifecycle === "starting") {
    return "Core starting";
  }

  if (backend.lifecycle === "crashed") {
    return "Core crashed";
  }

  if (backend.lifecycle === "stopping" || backend.lifecycle === "stopped") {
    return "Core stopped";
  }

  return "Core offline";
};

export function BackendStatusBadge({ backend, eventStreamConnected }: BackendStatusBadgeProps) {
  const failed = backend.lifecycle === "crashed" || backend.lifecycle === "unavailable";
  const pending = backend.lifecycle === "starting" || backend.lifecycle === "stopping";
  const label = labelForLifecycle(backend, eventStreamConnected);

  return (
    <div className={`status-badge backend-badge ${failed ? "backend-failed" : ""} ${pending ? "backend-pending" : ""}`}>
      {failed ? <ServerCrash size={13} aria-hidden="true" /> : <Server size={13} aria-hidden="true" />}
      <span>{label}</span>
      {failed ? (
        <button
          className="inline-icon-button"
          type="button"
          title="Restart backend"
          aria-label="Restart backend"
          onClick={() => void assistantStore.restartBackend()}
        >
          <RotateCw size={12} aria-hidden="true" />
        </button>
      ) : null}
    </div>
  );
}
