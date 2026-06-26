import type { AssistantSnapshot } from "../../stores/assistantStore";
import { AlertTriangle, Download, FileText, Gauge, HeartPulse, RotateCw, ShieldCheck, Trash2 } from "lucide-react";
import type { ReactNode } from "react";
import { useEffect } from "react";
import { assistantStore } from "../../stores/assistantStore";

interface ReleaseQualityPanelProps {
  snapshot: AssistantSnapshot;
}

export function ReleaseQualityPanel({ snapshot }: ReleaseQualityPanelProps) {
  useEffect(() => {
    if (!snapshot.releaseReadiness && !snapshot.releaseBusy) {
      void assistantStore.loadReleaseQuality();
    }
  }, [snapshot.releaseBusy, snapshot.releaseReadiness]);

  const readiness = snapshot.releaseReadiness;
  const health = snapshot.releaseConnectorHealth;
  const exportCounts = snapshot.releasePrivacyExport?.counts ?? {};
  const profileMetrics = snapshot.releasePerformance?.metrics.slice(0, 5) ?? [];
  const logs = snapshot.releaseLogs?.files.slice(0, 4) ?? [];

  return (
    <section className="panel-section release-panel" aria-label="Release quality">
      <div className="section-heading release-heading">
        <span>
          <ShieldCheck size={15} aria-hidden="true" />
          <span>Release</span>
        </span>
        <button
          className="inline-icon-button"
          type="button"
          title="Refresh release status"
          aria-label="Refresh release status"
          disabled={snapshot.releaseBusy}
          onClick={() => void assistantStore.loadReleaseQuality()}
        >
          <RotateCw size={13} aria-hidden="true" />
        </button>
      </div>

      <div className="release-grid">
        <MetricTile
          icon={<Download size={14} aria-hidden="true" />}
          label="Installer"
          value={readiness?.installerReady ? "ready" : "check"}
        />
        <MetricTile
          icon={<ShieldCheck size={14} aria-hidden="true" />}
          label="Privacy"
          value={`${exportCounts.privacyAuditEvents ?? 0}`}
        />
        <MetricTile
          icon={<HeartPulse size={14} aria-hidden="true" />}
          label="Connectors"
          value={`${health?.healthy ?? 0}/${health?.items.length ?? 0}`}
        />
      </div>

      <div className="release-readiness-list">
        {(readiness?.items ?? []).slice(0, 5).map((item) => (
          <div className={`release-row release-${item.status}`} key={item.id}>
            <span>{item.label}</span>
            <strong>{item.status}</strong>
          </div>
        ))}
      </div>

      <div className="release-tools">
        <div className="release-tool-block">
          <div className="release-mini-heading">
            <FileText size={14} aria-hidden="true" />
            <span>Logs</span>
          </div>
          <div className="release-log-list">
            {logs.length ? (
              logs.map((log) => (
                <button
                  className="release-log-button"
                  type="button"
                  key={log.path}
                  onClick={() => void assistantStore.readReleaseLog(log.path)}
                >
                  <span>{log.name}</span>
                  <small>{formatBytes(log.sizeBytes)}</small>
                </button>
              ))
            ) : (
              <span className="release-empty">No logs</span>
            )}
          </div>
        </div>

        <div className="release-tool-block">
          <div className="release-mini-heading">
            <Gauge size={14} aria-hidden="true" />
            <span>Profile</span>
          </div>
          <div className="release-profile-list">
            {profileMetrics.map((metric) => (
              <div className="release-row" key={metric.name}>
                <span>{metric.name}</span>
                <strong>{formatMetric(metric.value, metric.unit)}</strong>
              </div>
            ))}
          </div>
        </div>
      </div>

      {snapshot.releaseSelectedLog ? (
        <pre className="release-log-output">{snapshot.releaseSelectedLog.content}</pre>
      ) : null}

      <div className="release-row release-crash-row">
        <span>Crash recovery</span>
        <strong>{snapshot.releaseCrashRecovery?.previousCrashDetected ? "review" : "clean"}</strong>
      </div>

      <div className="release-delete-box">
        <div className="release-mini-heading">
          <AlertTriangle size={14} aria-hidden="true" />
          <span>Delete local data</span>
        </div>
        <input
          value={snapshot.releaseDeletePhrase}
          placeholder="DELETE LOCAL DATA"
          aria-label="Delete local data confirmation"
          onChange={(event) => assistantStore.setReleaseDeletePhrase(event.currentTarget.value)}
        />
        <label className="release-checkbox">
          <input
            type="checkbox"
            checked={snapshot.releaseDeleteIncludeVault}
            onChange={(event) => assistantStore.setReleaseDeleteIncludeVault(event.currentTarget.checked)}
          />
          <span>Include vault</span>
        </label>
        <button
          className="release-delete-button"
          type="button"
          disabled={snapshot.releaseBusy || snapshot.releaseDeletePhrase !== "DELETE LOCAL DATA"}
          onClick={() => void assistantStore.deleteLocalData()}
        >
          <Trash2 size={14} aria-hidden="true" />
          <span>Delete</span>
        </button>
      </div>
    </section>
  );
}

function MetricTile({
  icon,
  label,
  value
}: {
  icon: ReactNode;
  label: string;
  value: string;
}) {
  return (
    <div className="release-metric">
      {icon}
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function formatBytes(value: number) {
  if (value < 1024) {
    return `${value} B`;
  }
  if (value < 1024 * 1024) {
    return `${Math.round(value / 1024)} KB`;
  }
  return `${(value / 1024 / 1024).toFixed(1)} MB`;
}

function formatMetric(value: number, unit: string) {
  if (unit === "bytes") {
    return formatBytes(value);
  }
  if (unit === "ms") {
    return `${value.toFixed(1)} ms`;
  }
  return `${Math.round(value)} ${unit}`;
}
