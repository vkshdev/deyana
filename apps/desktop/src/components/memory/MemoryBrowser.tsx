import type { AssistantSnapshot } from "../../stores/assistantStore";
import { Download, FolderOpen, Plus, RefreshCw, Search, Trash2 } from "lucide-react";
import { assistantStore } from "../../stores/assistantStore";

interface MemoryBrowserProps {
  snapshot: AssistantSnapshot;
}

export function MemoryBrowser({ snapshot }: MemoryBrowserProps) {
  return (
    <section className="memory-browser" aria-label="Memory browser">
      <header className="memory-browser-header">
        <div className="section-heading">
          <Search size={15} aria-hidden="true" />
          <span>Memory</span>
        </div>
        <div className="memory-tools">
          <button
            className="icon-button"
            type="button"
            title="Reindex memory"
            aria-label="Reindex memory"
            onClick={() => void assistantStore.reindexMemory()}
          >
            <RefreshCw size={15} aria-hidden="true" />
          </button>
          <button
            className="icon-button"
            type="button"
            title="Export memory"
            aria-label="Export memory"
            onClick={() => void assistantStore.exportMemory()}
          >
            <Download size={15} aria-hidden="true" />
          </button>
          <button
            className="icon-button"
            type="button"
            title="Open vault"
            aria-label="Open vault"
            onClick={() => void assistantStore.openVault()}
          >
            <FolderOpen size={15} aria-hidden="true" />
          </button>
        </div>
      </header>

      <div className="memory-search">
        <input
          type="search"
          value={snapshot.memoryQuery}
          placeholder="Search local memory"
          aria-label="Search local memory"
          onChange={(event) => assistantStore.setMemoryQuery(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              void assistantStore.loadMemory();
            }
          }}
        />
        <button
          className="icon-button"
          type="button"
          title="Search memory"
          aria-label="Search memory"
          onClick={() => void assistantStore.loadMemory()}
        >
          <Search size={15} aria-hidden="true" />
        </button>
      </div>

      <div className="memory-create">
        <input
          value={snapshot.memoryDraft.title}
          placeholder="Title"
          aria-label="Memory title"
          onChange={(event) => assistantStore.setMemoryDraft({ title: event.target.value })}
        />
        <input
          value={snapshot.memoryDraft.summary}
          placeholder="Summary"
          aria-label="Memory summary"
          onChange={(event) => assistantStore.setMemoryDraft({ summary: event.target.value })}
        />
        <textarea
          value={snapshot.memoryDraft.contentMarkdown}
          placeholder="Markdown note"
          aria-label="Memory markdown"
          onChange={(event) => assistantStore.setMemoryDraft({ contentMarkdown: event.target.value })}
        />
        <button
          className="primary-action memory-add"
          type="button"
          disabled={snapshot.memoryBusy}
          onClick={() => void assistantStore.createMemory()}
        >
          <Plus size={15} aria-hidden="true" />
          <span>Add memory</span>
        </button>
      </div>

      <div className="memory-browser-list">
        {snapshot.memoryItems.length ? (
          snapshot.memoryItems.map((item) => (
            <article className="memory-browser-item" key={item.id}>
              <div>
                <strong>{item.title}</strong>
                <span>{item.summary}</span>
                <small>{item.markdownPath ?? item.sourceType}</small>
              </div>
              <button
                className="icon-button"
                type="button"
                title="Delete memory"
                aria-label={`Delete ${item.title}`}
                disabled={snapshot.memoryBusy}
                onClick={() => void assistantStore.deleteMemory(item.id)}
              >
                <Trash2 size={15} aria-hidden="true" />
              </button>
            </article>
          ))
        ) : (
          <div className="memory-empty">No local memory yet</div>
        )}
      </div>

      {snapshot.memoryExportedAt ? (
        <div className="memory-export-state">Exported {new Date(snapshot.memoryExportedAt).toLocaleTimeString()}</div>
      ) : null}
    </section>
  );
}
