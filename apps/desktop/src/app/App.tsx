import { MotionConfig, motion } from "framer-motion";
import { useEffect } from "react";
import { FloatingOrb } from "../components/floating/FloatingOrb";
import { FloatingPanel } from "../components/floating/FloatingPanel";
import { assistantStore, useAssistantSnapshot } from "../stores/assistantStore";

export function App() {
  const snapshot = useAssistantSnapshot();
  const reducedMotion = snapshot.settings.lowPowerMode || snapshot.settings.reduceMotion;
  const isExpanded = snapshot.settings.uiMode === "expanded";

  useEffect(() => {
    void assistantStore.hydrate();
  }, []);

  return (
    <MotionConfig reducedMotion={reducedMotion ? "always" : "user"}>
      <motion.main
        className={isExpanded ? "app-shell app-shell-expanded" : "app-shell app-shell-compact"}
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.18, ease: "easeOut" }}
      >
        {isExpanded ? <FloatingPanel snapshot={snapshot} /> : <FloatingOrb snapshot={snapshot} />}
      </motion.main>
    </MotionConfig>
  );
}
