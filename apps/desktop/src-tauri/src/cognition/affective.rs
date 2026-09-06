use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscreteEmotion {
    Curious,
    Engaged,
    Satisfied,
    Neutral,
    Uncertain,
    Frustrated,
    Bored,
}

impl DiscreteEmotion {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Curious => "CURIOUS",
            Self::Engaged => "ENGAGED",
            Self::Satisfied => "SATISFIED",
            Self::Neutral => "NEUTRAL",
            Self::Uncertain => "UNCERTAIN",
            Self::Frustrated => "FRUSTRATED",
            Self::Bored => "BORED",
        }
    }

    pub fn baseline_coordinates(&self) -> (f32, f32) {
        match self {
            Self::Curious => (0.6, 0.6),
            Self::Engaged => (0.7, 0.4),
            Self::Satisfied => (0.7, -0.4),
            Self::Neutral => (0.0, 0.0),
            Self::Uncertain => (-0.3, 0.3),
            Self::Frustrated => (-0.6, 0.6),
            Self::Bored => (-0.1, -0.6),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AffectiveState {
    pub discrete: DiscreteEmotion,
    pub valence: f32, // -1.0 to +1.0
    pub arousal: f32, // -1.0 to +1.0
    pub transition_reason: String,
    pub updated_at: String,
}

impl Default for AffectiveState {
    fn default() -> Self {
        let (val, aro) = DiscreteEmotion::Neutral.baseline_coordinates();
        Self {
            discrete: DiscreteEmotion::Neutral,
            valence: val,
            arousal: aro,
            transition_reason: "Initial baseline state".to_string(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

pub struct AffectiveStateMachine {
    current: AffectiveState,
    last_interaction_time: Instant,
}

impl Default for AffectiveStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl AffectiveStateMachine {
    pub fn new() -> Self {
        Self {
            current: AffectiveState::default(),
            last_interaction_time: Instant::now(),
        }
    }

    pub fn current(&self) -> AffectiveState {
        self.current.clone()
    }

    pub fn last_interaction_time(&self) -> Instant {
        self.last_interaction_time
    }

    pub fn set_last_interaction_time(&mut self, time: Instant) {
        self.last_interaction_time = time;
    }

    pub fn transition(
        &mut self,
        state: DiscreteEmotion,
        valence: f32,
        arousal: f32,
        reason: impl Into<String>,
    ) -> AffectiveState {
        self.current.discrete = state;
        self.current.valence = valence.clamp(-1.0, 1.0);
        self.current.arousal = arousal.clamp(-1.0, 1.0);
        self.current.transition_reason = reason.into();
        self.current.updated_at = chrono::Utc::now().to_rfc3339();
        self.current.clone()
    }

    /// Process signals from a completed conversation turn
    pub fn evaluate_turn_signals(
        &mut self,
        user_prompt: &str,
        assistant_response: &str,
        tool_failures: usize,
        reflection_confidence: Option<f32>,
    ) -> AffectiveState {
        self.last_interaction_time = Instant::now();
        let prompt_lower = user_prompt.to_lowercase();
        let resp_lower = assistant_response.to_lowercase();

        if tool_failures >= 2 {
            self.transition(
                DiscreteEmotion::Frustrated,
                -0.6,
                0.6,
                format!("Encountered {tool_failures} tool failures during turn execution"),
            );
        } else if let Some(conf) = reflection_confidence.filter(|&c| c < 0.6) {
            self.transition(
                DiscreteEmotion::Uncertain,
                -0.3,
                0.3,
                format!("Metacognitive evaluation flagged low confidence ({conf:.2}) in turn response"),
            );
        } else if prompt_lower.contains("thank")
            || prompt_lower.contains("great job")
            || prompt_lower.contains("perfect")
            || prompt_lower.contains("awesome")
            || resp_lower.contains("successfully completed")
        {
            self.transition(
                DiscreteEmotion::Satisfied,
                0.7,
                -0.4,
                "Task completed successfully with positive validation",
            );
        } else if prompt_lower.contains("how does")
            || prompt_lower.contains("why")
            || prompt_lower.contains("what if")
            || prompt_lower.contains("explain")
            || prompt_lower.contains("explore")
            || prompt_lower.contains("can you tell me about")
        {
            self.transition(
                DiscreteEmotion::Curious,
                0.6,
                0.6,
                "Inquisitive conceptual inquiry prompting exploration",
            );
        } else {
            self.transition(
                DiscreteEmotion::Engaged,
                0.7,
                0.4,
                "Active focused collaborative problem solving",
            );
        }

        self.current.clone()
    }

    /// Periodic idle decay check (e.g. called each minute by daemon)
    /// Decays to NEUTRAL after 15 minutes, and to BORED after 45 minutes
    pub fn apply_idle_decay(&mut self) -> Option<AffectiveState> {
        let idle_secs = self.last_interaction_time.elapsed().as_secs();
        let idle_minutes = idle_secs / 60;

        if idle_minutes >= 45 {
            if self.current.discrete != DiscreteEmotion::Bored {
                let state = self.transition(
                    DiscreteEmotion::Bored,
                    -0.1,
                    -0.6,
                    format!("Prolonged system idle of {idle_minutes} minutes without interaction"),
                );
                Some(state)
            } else {
                None
            }
        } else if idle_minutes >= 15 {
            if self.current.discrete != DiscreteEmotion::Neutral {
                let state = self.transition(
                    DiscreteEmotion::Neutral,
                    0.0,
                    0.0,
                    format!("Decayed to baseline neutral after {idle_minutes} minutes of inactivity"),
                );
                Some(state)
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_discrete_emotion_serialization() {
        assert_eq!(
            serde_json::to_string(&DiscreteEmotion::Curious).unwrap(),
            "\"CURIOUS\""
        );
        assert_eq!(
            serde_json::to_string(&DiscreteEmotion::Engaged).unwrap(),
            "\"ENGAGED\""
        );
        assert_eq!(
            serde_json::to_string(&DiscreteEmotion::Satisfied).unwrap(),
            "\"SATISFIED\""
        );
        assert_eq!(
            serde_json::to_string(&DiscreteEmotion::Neutral).unwrap(),
            "\"NEUTRAL\""
        );
        assert_eq!(
            serde_json::to_string(&DiscreteEmotion::Uncertain).unwrap(),
            "\"UNCERTAIN\""
        );
        assert_eq!(
            serde_json::to_string(&DiscreteEmotion::Frustrated).unwrap(),
            "\"FRUSTRATED\""
        );
        assert_eq!(
            serde_json::to_string(&DiscreteEmotion::Bored).unwrap(),
            "\"BORED\""
        );
    }

    #[test]
    fn test_affective_state_transitions_all_discrete_states() {
        let mut fsm = AffectiveStateMachine::new();
        assert_eq!(fsm.current().discrete, DiscreteEmotion::Neutral);

        fsm.transition(DiscreteEmotion::Curious, 0.6, 0.6, "Test curiosity");
        assert_eq!(fsm.current().discrete, DiscreteEmotion::Curious);
        assert_eq!(fsm.current().valence, 0.6);
        assert_eq!(fsm.current().arousal, 0.6);

        fsm.transition(DiscreteEmotion::Engaged, 0.7, 0.4, "Test engaged");
        assert_eq!(fsm.current().discrete, DiscreteEmotion::Engaged);

        fsm.transition(DiscreteEmotion::Satisfied, 0.7, -0.4, "Test satisfied");
        assert_eq!(fsm.current().discrete, DiscreteEmotion::Satisfied);

        fsm.transition(DiscreteEmotion::Neutral, 0.0, 0.0, "Test neutral");
        assert_eq!(fsm.current().discrete, DiscreteEmotion::Neutral);

        fsm.transition(DiscreteEmotion::Uncertain, -0.3, 0.3, "Test uncertain");
        assert_eq!(fsm.current().discrete, DiscreteEmotion::Uncertain);

        fsm.transition(DiscreteEmotion::Frustrated, -0.6, 0.6, "Test frustrated");
        assert_eq!(fsm.current().discrete, DiscreteEmotion::Frustrated);

        fsm.transition(DiscreteEmotion::Bored, -0.1, -0.6, "Test bored");
        assert_eq!(fsm.current().discrete, DiscreteEmotion::Bored);
    }

    #[test]
    fn test_turn_signal_evaluations() {
        let mut fsm = AffectiveStateMachine::new();

        // Repeated tool failures trigger Frustrated
        let state = fsm.evaluate_turn_signals("Fix this", "failed", 2, None);
        assert_eq!(state.discrete, DiscreteEmotion::Frustrated);

        // Low confidence triggers Uncertain
        let state = fsm.evaluate_turn_signals("What is X?", "X might be Y", 0, Some(0.45));
        assert_eq!(state.discrete, DiscreteEmotion::Uncertain);

        // Inquisitive inquiry triggers Curious
        let state = fsm.evaluate_turn_signals("Can you explain how async Rust futures work?", "Futures are poll-based", 0, Some(0.9));
        assert_eq!(state.discrete, DiscreteEmotion::Curious);

        // Validation triggers Satisfied
        let state = fsm.evaluate_turn_signals("Thank you so much, that's perfect!", "You're welcome!", 0, Some(0.95));
        assert_eq!(state.discrete, DiscreteEmotion::Satisfied);

        // Standard task execution triggers Engaged
        let state = fsm.evaluate_turn_signals("Implement the user authentication endpoint", "Here is the code", 0, Some(0.85));
        assert_eq!(state.discrete, DiscreteEmotion::Engaged);
    }

    #[test]
    fn test_affective_idle_decay() {
        let mut fsm = AffectiveStateMachine::new();
        fsm.transition(DiscreteEmotion::Engaged, 0.7, 0.4, "Active work");

        // Under 15m: no decay
        fsm.set_last_interaction_time(Instant::now() - Duration::from_secs(10 * 60));
        assert!(fsm.apply_idle_decay().is_none());
        assert_eq!(fsm.current().discrete, DiscreteEmotion::Engaged);

        // At 16m: decay to Neutral
        fsm.set_last_interaction_time(Instant::now() - Duration::from_secs(16 * 60));
        let decayed = fsm.apply_idle_decay();
        assert!(decayed.is_some());
        assert_eq!(decayed.unwrap().discrete, DiscreteEmotion::Neutral);

        // Second check within 15-44m when already Neutral: returns None
        assert!(fsm.apply_idle_decay().is_none());

        // At 50m: decay to Bored
        fsm.set_last_interaction_time(Instant::now() - Duration::from_secs(50 * 60));
        let bored_state = fsm.apply_idle_decay();
        assert!(bored_state.is_some());
        assert_eq!(bored_state.unwrap().discrete, DiscreteEmotion::Bored);

        // Subsequent checks when already Bored: returns None
        assert!(fsm.apply_idle_decay().is_none());
    }
}
