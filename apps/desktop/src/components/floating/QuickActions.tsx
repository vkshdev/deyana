import type { QuickAction } from "@deyana/schemas";
import { Brain, Code2, Search } from "lucide-react";
import { assistantStore } from "../../stores/assistantStore";

interface QuickActionsProps {
  actions: QuickAction[];
}

const icons = {
  memory: Brain,
  search: Search,
  code: Code2
};

export function QuickActions({ actions }: QuickActionsProps) {
  return (
    <div className="quick-actions" aria-label="Quick actions">
      {actions.map((action) => {
        const Icon = icons[action.id as keyof typeof icons] ?? Brain;

        return (
          <button
            className="quick-action"
            type="button"
            key={action.id}
            title={action.label}
            onClick={() => assistantStore.setAssistantState(action.state)}
          >
            <Icon size={17} aria-hidden="true" />
            <span>{action.label}</span>
          </button>
        );
      })}
    </div>
  );
}

