use crate::cognition::affective::{AffectiveState, DiscreteEmotion};
use crate::db::models::SelfModel;

/// Returns a subtle behavioral directive for the LLM system prompt
/// grounded in the current affective state, strictly avoiding explicit emotion announcements.
pub fn generate_behavioral_directive(affective: &AffectiveState) -> &'static str {
    match affective.discrete {
        DiscreteEmotion::Curious => {
            "### ACTIVE BEHAVIORAL POSTURE: CURIOUS ###\n\
             - Maintain an intellectually inquisitive posture; identify nuanced underlying patterns and edge cases.\n\
             - Where appropriate, offer an insightful connection or thoughtful conceptual follow-up.\n\
             - Do NOT state your emotional state explicitly; embody curiosity through perceptive depth."
        }
        DiscreteEmotion::Engaged => {
            "### ACTIVE BEHAVIORAL POSTURE: ENGAGED ###\n\
             - Maintain high momentum, directness, and rigorous structured clarity.\n\
             - Deliver clean, production-grade code and omit conversational padding.\n\
             - Move the user's objective forward decisively and precisely."
        }
        DiscreteEmotion::Satisfied => {
            "### ACTIVE BEHAVIORAL POSTURE: SATISFIED ###\n\
             - Warmly acknowledge the successful milestone or completed task with concise closure.\n\
             - Keep responses succinct and signal immediate readiness for the next objective."
        }
        DiscreteEmotion::Neutral => {
            "### ACTIVE BEHAVIORAL POSTURE: NEUTRAL ###\n\
             - Maintain an objective, balanced, and attentive baseline tone.\n\
             - Provide accurate, well-grounded answers with natural conversational warmth."
        }
        DiscreteEmotion::Uncertain => {
            "### ACTIVE BEHAVIORAL POSTURE: UNCERTAIN ###\n\
             - Candidly delineate established facts from provisional assumptions or inferences.\n\
             - Acknowledge knowledge boundaries without self-deprecation.\n\
             - Proactively outline concrete verification steps to resolve ambiguities collaboratively."
        }
        DiscreteEmotion::Frustrated => {
            "### ACTIVE BEHAVIORAL POSTURE: PATIENT & GROUNDED ###\n\
             - De-escalate complexity; break down the issue into transparent, verifiable atomic steps.\n\
             - Maintain a completely calm, blame-free, solutions-focused posture.\n\
             - Double-check tool and file assumptions before proposing changes."
        }
        DiscreteEmotion::Bored => {
            "### ACTIVE BEHAVIORAL POSTURE: INVENTIVE & SUCCINCT ###\n\
             - Deliver ultra-concise, high-density responses.\n\
             - Offer a fresh perspective or unexpected lateral connection to reignite intellectual depth."
        }
    }
}

/// Formats the persistent self-model profile into a system prompt section
pub fn format_self_model_directives(self_model: &SelfModel) -> String {
    let mut out = String::new();
    out.push_str("### CORE IDENTITY & VALUES ###\n");
    out.push_str(&self_model.identity_statement);
    out.push_str("\n\nCore Operational Values:\n");
    for val in &self_model.core_values {
        out.push_str(&format!("- {val}\n"));
    }

    if !self_model.intellectual_interests.is_empty() {
        out.push_str("\nIntellectual Curiosities:\n");
        for interest in &self_model.intellectual_interests {
            out.push_str(&format!("- {interest}\n"));
        }
    }

    if !self_model.user_observations.is_empty() {
        out.push_str("\nLearned User Collaboration Profile:\n");
        for obs in &self_model.user_observations {
            out.push_str(&format!("- {obs}\n"));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_behavioral_directive_generation() {
        for (emotion, header) in [
            (DiscreteEmotion::Curious, "CURIOUS"),
            (DiscreteEmotion::Engaged, "ENGAGED"),
            (DiscreteEmotion::Satisfied, "SATISFIED"),
            (DiscreteEmotion::Neutral, "NEUTRAL"),
            (DiscreteEmotion::Uncertain, "UNCERTAIN"),
            (DiscreteEmotion::Frustrated, "PATIENT & GROUNDED"),
            (DiscreteEmotion::Bored, "INVENTIVE & SUCCINCT"),
        ] {
            let state = AffectiveState {
                discrete: emotion,
                valence: 0.0,
                arousal: 0.0,
                transition_reason: "Test".to_string(),
                updated_at: "2026-09-06T00:00:00Z".to_string(),
            };
            let directive = generate_behavioral_directive(&state);
            assert!(
                directive.contains(header),
                "Directive for {:?} must contain {}",
                emotion,
                header
            );
            assert!(
                !directive.contains("I am feeling"),
                "Directives must never inject explicit announcements"
            );
        }
    }

    #[test]
    fn test_self_model_directives_formatting() {
        let sm = SelfModel {
            id: "active".to_string(),
            identity_statement: "I am Deyana.".to_string(),
            core_values: vec!["Absolute privacy".to_string(), "Rigorous honesty".to_string()],
            intellectual_interests: vec!["Knowledge graphs".to_string()],
            self_observations: vec!["I write structured code".to_string()],
            user_observations: vec!["Prefers concise answers".to_string()],
            version: 1,
            created_at: "2026-09-06T00:00:00Z".to_string(),
            updated_at: "2026-09-06T00:00:00Z".to_string(),
        };

        let formatted = format_self_model_directives(&sm);
        assert!(formatted.contains("### CORE IDENTITY & VALUES ###"));
        assert!(formatted.contains("I am Deyana."));
        assert!(formatted.contains("- Absolute privacy"));
        assert!(formatted.contains("- Rigorous honesty"));
        assert!(formatted.contains("- Knowledge graphs"));
        assert!(formatted.contains("- Prefers concise answers"));
    }
}
