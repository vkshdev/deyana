import type { MemoryPreviewItem } from "@deyana/schemas";
import { FileText } from "lucide-react";

interface MemoryPreviewProps {
  items: MemoryPreviewItem[];
}

export function MemoryPreview({ items }: MemoryPreviewProps) {
  return (
    <section className="panel-section" aria-label="Recent memory">
      <div className="section-heading">
        <FileText size={15} aria-hidden="true" />
        <span>Memory</span>
      </div>
      <div className="memory-list">
        {items.map((item) => (
          <article className="memory-item" key={item.id}>
            <div>
              <strong>{item.title}</strong>
              <span>{item.source}</span>
            </div>
            <time>{item.updatedLabel}</time>
          </article>
        ))}
      </div>
    </section>
  );
}

