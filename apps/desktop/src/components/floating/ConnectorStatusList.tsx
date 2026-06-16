import type { ConnectorPreview } from "@deyana/schemas";
import { Link2 } from "lucide-react";

interface ConnectorStatusListProps {
  connectors: ConnectorPreview[];
}

export function ConnectorStatusList({ connectors }: ConnectorStatusListProps) {
  return (
    <section className="panel-section" aria-label="Connectors">
      <div className="section-heading">
        <Link2 size={15} aria-hidden="true" />
        <span>Connectors</span>
      </div>
      <div className="connector-list">
        {connectors.map((connector) => (
          <div className="connector-row" key={connector.id}>
            <span className={`connector-dot connector-${connector.status}`} />
            <div>
              <strong>{connector.name}</strong>
              <span>{connector.lastSyncLabel}</span>
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}

