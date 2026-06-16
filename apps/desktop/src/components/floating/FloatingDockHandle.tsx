import { GripVertical } from "lucide-react";

export function FloatingDockHandle() {
  return (
    <div className="dock-handle" data-tauri-drag-region title="Drag">
      <GripVertical size={16} aria-hidden="true" data-tauri-drag-region />
    </div>
  );
}

