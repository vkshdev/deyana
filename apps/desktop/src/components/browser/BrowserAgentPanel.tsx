import type { AssistantSnapshot } from "../../stores/assistantStore";
import type { BrowserContextMode, BrowserPersonalityPreset, BrowserWritableField } from "@deyana/schemas";
import {
  BookOpenText,
  Eraser,
  ExternalLink,
  Globe2,
  Link2,
  PenLine,
  RefreshCw,
  Search,
  ShieldCheck,
  ShieldX,
  Unplug,
  Undo2,
  Wand2,
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

      {snapshot.browserContext ? (
        <section className="browser-draft-panel" aria-label="Inline browser draft assistance">
          <div className="browser-adapter-health">
            <strong>{snapshot.browserContext.adapterHealth.adapterId}</strong>
            <span>{snapshot.browserContext.adapterHealth.detail}</span>
          </div>
          <div className="browser-field-row">
            <PenLine size={13} aria-hidden="true" />
            <select
              aria-label="Draft target text field"
              disabled={snapshot.browserBusy || !snapshot.browserContext.writableFields.length}
              value={
                snapshot.browserDraftTarget?.handle ??
                snapshot.browserDraft?.field?.handle ??
                snapshot.browserContext.writableFields[0]?.handle ??
                ""
              }
              onChange={(event) => assistantStore.setBrowserDraftTarget(event.target.value)}
            >
              {!snapshot.browserContext.writableFields.length ? (
                <option value="">No safe visible text field</option>
              ) : null}
              {snapshot.browserContext.writableFields.map((field) => (
                <option value={field.handle} key={field.handle}>
                  {fieldLabel(field)}
                </option>
              ))}
            </select>
          </div>
          <div className="browser-draft-row">
            <Wand2 size={13} aria-hidden="true" />
            <input
              value={snapshot.browserDraftInstruction}
              placeholder="Draft instruction, e.g. reply politely that I am busy"
              aria-label="Draft instruction"
              disabled={snapshot.browserBusy}
              onChange={(event) => assistantStore.setBrowserDraftInstruction(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  void assistantStore.draftBrowserReply("reply");
                }
              }}
            />
            <button
              type="button"
              disabled={snapshot.browserBusy || !connected || !snapshot.browserContext.writableFields.length}
              onClick={() => void assistantStore.draftBrowserReply("reply")}
            >
              Draft
            </button>
          </div>
          <div className="browser-draft-actions">
            <button
              type="button"
              disabled={snapshot.browserBusy || !connected || !snapshot.browserContext.writableFields.length}
              onClick={() => void assistantStore.draftBrowserReply("regenerate")}
            >
              Regenerate
            </button>
            <button
              type="button"
              disabled={snapshot.browserBusy || !connected || !snapshot.browserContext.writableFields.length}
              onClick={() => void assistantStore.draftBrowserReply("shorten")}
            >
              Shorten
            </button>
            <button
              type="button"
              disabled={snapshot.browserBusy || !connected || !snapshot.browserContext.writableFields.length}
              onClick={() => void assistantStore.draftBrowserReply("formalize")}
            >
              Formalize
            </button>
          </div>

          {snapshot.browserDraft?.draft ? (
            <article className="browser-draft-preview">
              <strong>Review before insertion</strong>
              <p>{snapshot.browserDraft.draft}</p>
              <small>
                Target: {snapshot.browserDraft.field ? fieldLabel(snapshot.browserDraft.field) : "selected field"}
                {snapshot.browserDraft.model ? ` · ${snapshot.browserDraft.model}` : " · local fallback"}
              </small>
              <div className="browser-draft-actions">
                <button
                  type="button"
                  disabled={snapshot.browserBusy}
                  onClick={() => void assistantStore.insertBrowserDraft()}
                >
                  Preview insert
                </button>
                {["whatsapp_web", "messenger", "instagram", "discord", "telegram", "gmail", "linkedin", "slack"].includes(snapshot.browserDraft.context?.adapterId ?? "") ? (
                  <button
                    type="button"
                    disabled={snapshot.browserBusy}
                    onClick={() => void assistantStore.previewWhatsAppSend()}
                  >
                    Preview message send
                  </button>
                ) : null}
                <button
                  type="button"
                  disabled={snapshot.browserBusy}
                  onClick={() => void assistantStore.restoreBrowserDraftField(true)}
                >
                  <Undo2 size={12} aria-hidden="true" />
                  Restore
                </button>
                <button
                  type="button"
                  disabled={snapshot.browserBusy}
                  onClick={() => void assistantStore.restoreBrowserDraftField(false)}
                >
                  <Eraser size={12} aria-hidden="true" />
                  Clear field
                </button>
                <button
                  type="button"
                  disabled={snapshot.browserBusy}
                  onClick={() => assistantStore.clearBrowserDraft()}
                >
                  Dismiss
                </button>
              </div>
            </article>
          ) : null}

          {snapshot.browserActionPlan ? (
            <article className="browser-action-preview">
              <strong>Confirmed action preview</strong>
              <p>{snapshot.browserActionPlan.previewMarkdown}</p>
              <small>
                Status: {snapshot.browserActionPlan.status} · expires {new Date(snapshot.browserActionPlan.expiresAt).toLocaleTimeString()}
              </small>
              <div className="browser-draft-actions">
                <button
                  type="button"
                  disabled={snapshot.browserBusy || snapshot.browserActionPlan.status !== "pending_confirmation"}
                  onClick={() => void assistantStore.confirmAndExecuteBrowserAction()}
                >
                  Confirm and execute
                </button>
                <button
                  type="button"
                  disabled={snapshot.browserBusy}
                  onClick={() => void assistantStore.cancelBrowserActionPlan()}
                >
                  Cancel
                </button>
                <button
                  type="button"
                  disabled={snapshot.browserBusy}
                  onClick={() => void assistantStore.emergencyStopBrowserActions()}
                >
                  <ShieldX size={12} aria-hidden="true" />
                  Emergency stop
                </button>
              </div>
            </article>
          ) : null}

          <WhatsAppBusyModePanel snapshot={snapshot} connected={connected} />
          <BrowserPersonalityPanel snapshot={snapshot} />
        </section>
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

function WhatsAppBusyModePanel({
  snapshot,
  connected
}: {
  snapshot: AssistantSnapshot;
  connected: boolean;
}) {
  const policy = snapshot.whatsappBusyModePolicy;
  const isWhatsApp = snapshot.browserContext?.adapterId === "whatsapp_web";
  const allowlistDraft =
    snapshot.whatsappBusyModeAllowlistDraft || policy?.allowlistedContacts.join("\n") || "";

  if (!policy) {
    return null;
  }

  return (
    <article className="browser-busy-mode-panel">
      <div className="browser-busy-heading">
        <strong>WhatsApp busy mode</strong>
        <span>{policy.enabled ? "restricted automation on" : "off by default"}</span>
      </div>
      <label className="browser-toggle-row">
        <input
          type="checkbox"
          checked={policy.enabled}
          disabled={snapshot.browserBusy}
          onChange={(event) =>
            void assistantStore.patchWhatsAppBusyModePolicy({ enabled: event.currentTarget.checked })
          }
        />
        <span>Enable allowlisted automatic busy replies</span>
      </label>
      <textarea
        value={allowlistDraft}
        placeholder="One allowed WhatsApp contact per line"
        disabled={snapshot.browserBusy}
        onChange={(event) => assistantStore.setWhatsAppBusyModeAllowlistDraft(event.currentTarget.value)}
      />
      <div className="browser-busy-controls">
        <input
          value={policy.windowStart}
          aria-label="Busy mode start time"
          disabled={snapshot.browserBusy}
          onChange={(event) =>
            void assistantStore.patchWhatsAppBusyModePolicy({ windowStart: event.currentTarget.value })
          }
        />
        <input
          value={policy.windowEnd}
          aria-label="Busy mode end time"
          disabled={snapshot.browserBusy}
          onChange={(event) =>
            void assistantStore.patchWhatsAppBusyModePolicy({ windowEnd: event.currentTarget.value })
          }
        />
        <button type="button" disabled={snapshot.browserBusy} onClick={() => void assistantStore.saveWhatsAppBusyAllowlist()}>
          Save allowlist
        </button>
        {policy.emergencyStopped ? (
          <button
            type="button"
            disabled={snapshot.browserBusy}
            onClick={() => void assistantStore.patchWhatsAppBusyModePolicy({ resetEmergencyStop: true })}
          >
            Reset stop
          </button>
        ) : null}
      </div>
      <small>
        Permission: {policy.permissionGranted ? "granted" : "not granted"} · groups{" "}
        {policy.allowGroups ? "allowed" : "blocked"} · cooldown {policy.cooldownMinutes}m · daily limit{" "}
        {policy.dailyLimit}
      </small>
      <p className="browser-busy-template">{policy.template}</p>
      <div className="browser-draft-actions">
        <button
          type="button"
          disabled={snapshot.browserBusy || !connected || !isWhatsApp}
          onClick={() => void assistantStore.evaluateWhatsAppBusyMode()}
        >
          Evaluate visible chat
        </button>
        <button
          type="button"
          disabled={snapshot.browserBusy || !connected || !isWhatsApp || !policy.enabled}
          onClick={() => void assistantStore.sendWhatsAppBusyReply()}
        >
          Send policy reply
        </button>
      </div>
      {snapshot.whatsappBusyModeEvaluation ? (
        <small>
          Decision: {snapshot.whatsappBusyModeEvaluation.decision} ·{" "}
          {snapshot.whatsappBusyModeEvaluation.reason}
        </small>
      ) : null}
    </article>
  );
}

function BrowserPersonalityPanel({ snapshot }: { snapshot: AssistantSnapshot }) {
  const profile = snapshot.browserPersonalityProfile;
  if (!profile) {
    return null;
  }

  return (
    <article className="browser-personality-panel">
      <div className="browser-busy-heading">
        <strong>Draft personality</strong>
        <span>{snapshot.browserMoodHint ? `mood: ${snapshot.browserMoodHint.label}` : "planner stays deterministic"}</span>
      </div>
      <div className="browser-busy-controls">
        <select
          value={profile.preset}
          disabled={snapshot.browserBusy}
          aria-label="Draft personality preset"
          onChange={(event) =>
            void assistantStore.patchBrowserPersonalityProfile({
              preset: event.currentTarget.value as BrowserPersonalityPreset
            })
          }
        >
          <option value="supportive">Supportive</option>
          <option value="concise">Concise</option>
          <option value="professional">Professional</option>
          <option value="playful">Playful</option>
          <option value="custom">Custom</option>
        </select>
        <button type="button" disabled={snapshot.browserBusy} onClick={() => void assistantStore.inferBrowserMoodFromDraftInstruction()}>
          Infer mood
        </button>
        <button type="button" disabled={snapshot.browserBusy} onClick={() => void assistantStore.previewBrowserPersonality()}>
          Preview style
        </button>
      </div>
      <textarea
        defaultValue={profile.customInstruction}
        placeholder="Optional custom writer style. This cannot change browser permissions."
        disabled={snapshot.browserBusy}
        onBlur={(event) =>
          void assistantStore.patchBrowserPersonalityProfile({
            preset: "custom",
            customInstruction: event.currentTarget.value
          })
        }
      />
      <small>
        Writer temperature {profile.writerTemperature.toFixed(2)} · max {profile.maxDraftCharacters} characters · automation disclosure stays required
      </small>
      {snapshot.browserPersonalityPreview ? (
        <p className="browser-busy-template">{snapshot.browserPersonalityPreview.preview}</p>
      ) : null}
    </article>
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

function fieldLabel(field: BrowserWritableField): string {
  const suffix = field.valueCharacterCount ? ` · ${field.valueCharacterCount} chars` : "";
  return `${field.label || field.placeholder || field.kind}${suffix}`;
}
