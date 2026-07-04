import { GripVertical } from "lucide-react";
import { startWindowDrag } from "../../utils/windowDrag";

export function FloatingDockHandle() {
  return (
    <div
      className="dock-handle"
      data-tauri-drag-region
      title="Drag window"
      onPointerDown={startWindowDrag}
    >
      <GripVertical size={16} aria-hidden="true" data-tauri-drag-region />
    </div>
  );
}
