import type { AssistantSnapshot } from "../../stores/assistantStore";
import { Activity, Gauge, PanelLeft, PanelRight } from "lucide-react";
import { assistantStore } from "../../stores/assistantStore";

interface FloatingPreferencesProps {
  snapshot: AssistantSnapshot;
}

export function FloatingPreferences({ snapshot }: FloatingPreferencesProps) {
  return (
    <section className="floating-preferences" aria-label="Floating UI preferences">
      <button
        className={snapshot.settings.lowPowerMode ? "preference-toggle preference-toggle-active" : "preference-toggle"}
        type="button"
        title={snapshot.settings.lowPowerMode ? "Disable low-power mode" : "Enable low-power mode"}
        aria-label={snapshot.settings.lowPowerMode ? "Disable low-power mode" : "Enable low-power mode"}
        onClick={() => void assistantStore.setLowPowerMode(!snapshot.settings.lowPowerMode)}
      >
        <Gauge size={15} aria-hidden="true" />
        <span>Power</span>
      </button>
      <button
        className={snapshot.settings.reduceMotion ? "preference-toggle preference-toggle-active" : "preference-toggle"}
        type="button"
        title={snapshot.settings.reduceMotion ? "Enable motion" : "Reduce motion"}
        aria-label={snapshot.settings.reduceMotion ? "Enable motion" : "Reduce motion"}
        onClick={() => void assistantStore.setReduceMotion(!snapshot.settings.reduceMotion)}
      >
        <Activity size={15} aria-hidden="true" />
        <span>Motion</span>
      </button>
      <div className="dock-controls">
        <button
          className="icon-button"
          type="button"
          title="Dock left"
          aria-label="Dock left"
          onClick={() => void assistantStore.dockFloatingWindow("left")}
        >
          <PanelLeft size={16} aria-hidden="true" />
        </button>
        <button
          className="icon-button"
          type="button"
          title="Dock right"
          aria-label="Dock right"
          onClick={() => void assistantStore.dockFloatingWindow("right")}
        >
          <PanelRight size={16} aria-hidden="true" />
        </button>
      </div>
    </section>
  );
}
