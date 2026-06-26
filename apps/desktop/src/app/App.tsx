import { MotionConfig, motion } from "framer-motion";
import { useEffect } from "react";
import { FloatingOrb } from "../components/floating/FloatingOrb";
import { FloatingPanel } from "../components/floating/FloatingPanel";
import { OnboardingFlow } from "../components/onboarding/OnboardingFlow";
import { assistantStore, useAssistantSnapshot } from "../stores/assistantStore";

export function App() {
  const snapshot = useAssistantSnapshot();
  const reducedMotion = snapshot.settings.lowPowerMode || snapshot.settings.reduceMotion;
  const isOnboarding = !snapshot.onboarding.completed;
  const isExpanded = isOnboarding || snapshot.settings.uiMode === "expanded";

  useEffect(() => {
    void assistantStore.hydrate();
  }, []);

  return (
    <MotionConfig reducedMotion={reducedMotion ? "always" : "user"}>
      <motion.main
        className={isExpanded ? "app-shell app-shell-expanded" : "app-shell app-shell-compact"}
        data-low-power={snapshot.settings.lowPowerMode ? "true" : "false"}
        data-reduce-motion={snapshot.settings.reduceMotion ? "true" : "false"}
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.18, ease: "easeOut" }}
      >
        {isOnboarding ? (
          <OnboardingFlow snapshot={snapshot} />
        ) : isExpanded ? (
          <FloatingPanel snapshot={snapshot} />
        ) : (
          <FloatingOrb snapshot={snapshot} />
        )}
      </motion.main>
    </MotionConfig>
  );
}
