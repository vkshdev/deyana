import type { PointerEvent as ReactPointerEvent } from "react";
import { tauriClient } from "../services/tauriClient";

const interactiveSelector = "button, a, input, select, textarea, [role='button'], [data-no-window-drag]";

export function startWindowDrag(event: ReactPointerEvent<HTMLElement>): void {
  if (event.defaultPrevented || event.button !== 0) {
    return;
  }

  const target = event.target;
  if (target instanceof Element && target.closest(interactiveSelector)) {
    return;
  }

  event.preventDefault();
  void tauriClient.startDragging().catch(() => undefined);
}
