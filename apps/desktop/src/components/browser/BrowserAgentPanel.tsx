import type { AssistantSnapshot } from "../../stores/assistantStore";
import type { BrowserContextMode } from "@deyana/schemas";
import {
  BookOpenText,
  ExternalLink,
  Globe2,
  Link2,
  RefreshCw,
  Search,
  ShieldCheck,
  Unplug,
  X
} from "lucide-react";
import { assistantStore } from "../../stores/assistantStore";

interface BrowserAgentPanelProps {
  snapshot: AssistantSnapshot;
}

const contextModes: Array<{ value: BrowserContextMode; label: string }> = [
  { value: "main", label: "Main content" },
  { value: "selection", label: "Selected text" },
  { value: "visible", label: "Visible page" }
];

export function BrowserAgentPanel({ snapshot }: BrowserAgentPanelProps) {
  const connected = snapshot.browserStatus.connected;
  const connectionLabel = connected
    ? `${snapshot.browserStatus.browserName ?? "Browser"} connected`
    : "Browser extension disconnected";

  return (
    <section className="browser-panel" aria-label="Browser agent">
      <header className="browser-heading">
        <span className="section-heading">
          <Globe2 size={15} aria-hidden="true" />
          <span>Browser Agent</span>
        </span>
        <button
          className="icon-button"
          type="button"
          title="Refresh browser status"
          aria-label="Refresh browser status"
          disabled={snapshot.browserBusy}
          onClick={() => void assistantStore.loadBrowser()}
        >
          <RefreshCw size={14} aria-hidden="true" />
        </button>
      </header>

      <div className={`browser-status ${connected ? "browser-connected" : "browser-disconnected"}`}>
        {connected ? <ShieldCheck size={14} aria-hidden="true" /> : <Unplug size={14} aria-hidden="true" />}
        <span>
          <strong>{connectionLabel}</strong>
          <small>
            {connected
              ? `Extension ${snapshot.browserStatus.extensionVersion ?? "unknown"} · protocol ${snapshot.browserStatus.protocolVersion}`
              : snapshot.browserStatus.lastError ?? "Open the Deyana extension in the target tab."}
          </small>
        </span>
      </div>

      {!connected ? (
        <button
          className="browser-instruction"
          type="button"
          onClick={() => void assistantStore.requestActiveTabPermission()}
        >
          Browser access requires a toolbar click, context-menu action, or Alt+Shift+D in the target tab.
        </button>
      ) : null}

      <div className="browser-context-controls">
        <select
          aria-label="Browser page context mode"
          value={snapshot.browserContextMode}
          disabled={snapshot.browserBusy}
          onChange={(event) =>
            assistantStore.setBrowserContextMode(event.target.value as BrowserContextMode)
          }
        >
          {contextModes.map((mode) => (
            <option value={mode.value} key={mode.value}>
              {mode.label}
            </option>
          ))}
        </select>
        <button
          type="button"
          disabled={snapshot.browserBusy || !connected}
          onClick={() => void assistantStore.readBrowserPage()}
        >
          <BookOpenText size={13} aria-hidden="true" />
          Read
        </button>
        <button
          type="button"
          disabled={snapshot.browserBusy || !connected || snapshot.modelStatus !== "available"}
          onClick={() => void assistantStore.summarizeBrowserPage()}
        >
          Summarize
        </button>
      </div>

      {snapshot.browserContext ? (
        <article className="browser-context-preview">
          <div>
            <strong>{snapshot.browserContext.title}</strong>
            <small>{snapshot.browserContext.origin}</small>
          </div>
          <span>
            {snapshot.browserContext.characterCount.toLocaleString()} characters
            {snapshot.browserContext.truncated ? " · truncated" : ""}
          </span>
          <p>{contextPreview(snapshot)}</p>
        </article>
      ) : null}

      {snapshot.browserSummary?.summary ? (
        <article className="browser-summary">
          <strong>Local summary</strong>
          <p>{snapshot.browserSummary.summary}</p>
          <small>
            {snapshot.browserSummary.model ?? "Local model"} · {snapshot.browserSummary.latencyMs} ms
          </small>
        </article>
      ) : null}

      <div className="browser-search-row">
        <Search size={13} aria-hidden="true" />
        <input
          value={snapshot.browserSearchQuery}
          placeholder="Public web search"
          aria-label="Public web search query"
          disabled={snapshot.browserBusy}
          onChange={(event) => assistantStore.setBrowserSearchQuery(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              void assistantStore.searchBrowser();
            }
          }}
        />
        <button
          type="button"
          disabled={snapshot.browserBusy || !snapshot.browserSearchQuery.trim()}
          onClick={() => void assistantStore.searchBrowser()}
        >
          Search
        </button>
      </div>

      {snapshot.browserSearchResult?.items.length ? (
        <div className="browser-search-results">
          {snapshot.browserSearchResult.items.map((item) => (
            <article key={`${item.url}-${item.title}`}>
              <div>
                <strong>{item.title}</strong>
                <small>{item.summary}</small>
              </div>
              {item.url ? (
                <button
                  className="icon-button"
                  type="button"
                  title="Open search result"
                  aria-label={`Open ${item.title}`}
                  disabled={snapshot.browserBusy}
                  onClick={() => void assistantStore.openBrowserUrl(item.url ?? "")}
                >
                  <ExternalLink size={13} aria-hidden="true" />
                </button>
              ) : null}
            </article>
          ))}
        </div>
      ) : null}

      <div className="browser-open-row">
        <Link2 size={13} aria-hidden="true" />
        <input
          value={snapshot.browserOpenUrl}
          placeholder="https://example.com"
          aria-label="URL to open"
          disabled={snapshot.browserBusy}
          onChange={(event) => assistantStore.setBrowserOpenUrl(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              void assistantStore.openBrowserUrl();
            }
          }}
        />
        <button
          type="button"
          disabled={snapshot.browserBusy || !connected || !snapshot.browserOpenUrl.trim()}
          onClick={() => void assistantStore.openBrowserUrl()}
        >
          Open
        </button>
      </div>

      {snapshot.browserSessions.length ? (
        <details className="browser-details">
          <summary>Active page sessions ({snapshot.browserSessions.length})</summary>
          <div className="browser-detail-list">
            {snapshot.browserSessions.map((session) => (
              <article key={session.id}>
                <span>
                  <strong>{session.title}</strong>
                  <small>{session.origin}</small>
                </span>
                <button
                  className="icon-button"
                  type="button"
                  title="Disconnect page session"
                  aria-label={`Disconnect ${session.title}`}
                  disabled={snapshot.browserBusy}
                  onClick={() => void assistantStore.disconnectBrowserSession(session.id)}
                >
                  <X size={13} aria-hidden="true" />
                </button>
              </article>
            ))}
          </div>
        </details>
      ) : null}

      {snapshot.browserPermissions.length ? (
        <details className="browser-details">
          <summary>Optional permissions ({snapshot.browserPermissions.length})</summary>
          <div className="browser-detail-list">
            {snapshot.browserPermissions.map((permission) => (
              <article key={`${permission.origin}-${permission.kind}`}>
                <span>
                  <strong>{permission.origin}</strong>
                  <small>{permission.detail}</small>
                </span>
                {permission.kind === "optional_origin" && permission.granted ? (
                  <button
                    className="icon-button"
                    type="button"
                    title="Revoke optional permission"
                    aria-label={`Revoke ${permission.origin}`}
                    disabled={snapshot.browserBusy}
                    onClick={() => void assistantStore.revokeBrowserPermission(permission.origin)}
                  >
                    <X size={13} aria-hidden="true" />
                  </button>
                ) : null}
              </article>
            ))}
          </div>
        </details>
      ) : null}

      {snapshot.browserAuditEvents.length ? (
        <details className="browser-details">
          <summary>Local browser audit</summary>
          <div className="browser-audit-list">
            {snapshot.browserAuditEvents.slice(0, 5).map((event) => (
              <article key={event.id}>
                <strong>{event.operation}</strong>
                <span>{event.detail}</span>
                <small>{event.decision}</small>
              </article>
            ))}
          </div>
        </details>
      ) : null}
    </section>
  );
}

function contextPreview(snapshot: AssistantSnapshot): string {
  const context = snapshot.browserContext;
  if (!context) {
    return "";
  }
  const content =
    context.mode === "selection" && context.selectionText
      ? context.selectionText
      : context.mode === "main" && context.mainText
        ? context.mainText
        : context.visibleText;
  return content.slice(0, 900);
}
