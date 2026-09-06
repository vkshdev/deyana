use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

use crate::cognition::affective::DiscreteEmotion;
use crate::cognition::CognitionState;
use crate::db::{self, DbPool, ReflectionRecord};
use crate::models::{LlmGenerateRequest, LlmRouter};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReflectionOutcome {
    #[serde(alias = "accuracy_score")]
    pub accuracy_score: f32,
    #[serde(alias = "completeness_score")]
    pub completeness_score: f32,
    #[serde(alias = "confidence_score")]
    pub confidence_score: f32,
    #[serde(default, alias = "missed_opportunities")]
    pub missed_opportunities: String,
    #[serde(default, alias = "correction_guidance")]
    pub correction_guidance: String,
}

pub struct MetacognitiveMonitor;

pub fn parse_reflection_json(raw: &str) -> Option<ReflectionOutcome> {
    let cleaned = raw.trim();

    if let Ok(outcome) = serde_json::from_str::<ReflectionOutcome>(cleaned) {
        return Some(outcome);
    }

    // Strip markdown code fence if present
    let content = if let Some(start_idx) = cleaned.find("```") {
        let after_fence = &cleaned[start_idx + 3..];
        let json_start = after_fence.find('\n').map(|i| &after_fence[i + 1..]).unwrap_or(after_fence);
        if let Some(end_idx) = json_start.rfind("```") {
            &json_start[..end_idx]
        } else {
            json_start
        }
    } else {
        cleaned
    };

    if let Ok(outcome) = serde_json::from_str::<ReflectionOutcome>(content.trim()) {
        return Some(outcome);
    }

    // Slice between first '{' and last '}'
    if let (Some(first_brace), Some(last_brace)) = (content.find('{'), content.rfind('}')) {
        if first_brace < last_brace {
            let slice = &content[first_brace..=last_brace];
            if let Ok(outcome) = serde_json::from_str::<ReflectionOutcome>(slice) {
                return Some(outcome);
            }
        }
    }

    None
}

impl MetacognitiveMonitor {
    pub async fn evaluate_turn(
        app: AppHandle,
        pool: DbPool,
        router: Arc<LlmRouter>,
        cognition_state: CognitionState,
        user_prompt: String,
        assistant_response: String,
        session_id: Option<String>,
    ) -> Option<ReflectionRecord> {
        let system_prompt = r#"You are Deyana's Metacognitive Critic.
Evaluate the assistant response against the user prompt.
Evaluate:
1. accuracy_score (0.0-1.0): Are assertions factual, properly qualified, and logically grounded?
2. completeness_score (0.0-1.0): Did it address all aspects of the user's request?
3. confidence_score (0.0-1.0): Overall confidence in the generated response.
4. missed_opportunities: What was omitted, glossed over, or could be improved?
5. correction_guidance: Specific advice for future self-correction.
Output ONLY valid JSON matching this schema:
{
  "accuracyScore": 0.9,
  "completenessScore": 0.85,
  "confidenceScore": 0.88,
  "missedOpportunities": "...",
  "correctionGuidance": "..."
}"#;

        let eval_prompt = format!(
            "User Prompt: {}\nAssistant Response: {}\nEvaluation JSON:",
            user_prompt.trim(),
            assistant_response.trim()
        );

        let req = LlmGenerateRequest {
            prompt: eval_prompt,
            model: None,
            system: Some(system_prompt.to_string()),
            temperature: Some(0.1),
            stream: Some(false),
            grammar: None,
        };

        if let Ok(raw_json) = router.generate_response(&req, |_| {}).await {
            if let Some(outcome) = parse_reflection_json(&raw_json) {
                let conn = pool.get().ok()?;
                let flagged = outcome.confidence_score < 0.6;

                let record = db::save_reflection(
                    &conn,
                    session_id.as_deref(),
                    &user_prompt,
                    &assistant_response,
                    outcome.accuracy_score,
                    outcome.completeness_score,
                    outcome.confidence_score,
                    &outcome.missed_opportunities,
                    &outcome.correction_guidance,
                    flagged,
                ).ok()?;

                if flagged {
                    // Buffer metacognitive correction note for subsequent turn
                    let note = format!(
                        "In the preceding turn, confidence was evaluated low ({:.2}). Correction guidance: {}. Missed opportunities: {}. Ensure rigorous verification of assertions.",
                        outcome.confidence_score, outcome.correction_guidance, outcome.missed_opportunities
                    );
                    *cognition_state.pending_correction.write().unwrap() = Some(note);

                    // Shift affective state to UNCERTAIN
                    let new_state = cognition_state.affective.write().unwrap().transition(
                        DiscreteEmotion::Uncertain,
                        -0.3,
                        0.3,
                        format!("Metacognitive evaluation flagged low confidence ({:.2})", outcome.confidence_score),
                    );
                    let _ = app.emit("cognition:affective_state_changed", &new_state);
                }

                let _ = app.emit("cognition:reflection_completed", &record);
                return Some(record);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_reflection_json() {
        let raw = r#"```json
        {
          "accuracyScore": 0.85,
          "completenessScore": 0.90,
          "confidenceScore": 0.88,
          "missedOpportunities": "Could have linked to RFC 8259",
          "correctionGuidance": "Provide official reference"
        }
        ```"#;

        let outcome = parse_reflection_json(raw);
        assert!(outcome.is_some());
        let o = outcome.unwrap();
        assert!((o.accuracy_score - 0.85).abs() < 1e-4);
        assert!((o.completeness_score - 0.90).abs() < 1e-4);
        assert!((o.confidence_score - 0.88).abs() < 1e-4);
        assert_eq!(o.missed_opportunities, "Could have linked to RFC 8259");
    }

    #[test]
    fn test_metacognitive_low_confidence_buffering() {
        let cognition_state = CognitionState::default();
        let outcome = ReflectionOutcome {
            accuracy_score: 0.5,
            completeness_score: 0.4,
            confidence_score: 0.45,
            missed_opportunities: "Did not check error handling".to_string(),
            correction_guidance: "Review error paths".to_string(),
        };

        if outcome.confidence_score < 0.6 {
            let note = format!(
                "In the preceding turn, confidence was evaluated low ({:.2}). Correction guidance: {}. Missed opportunities: {}. Ensure rigorous verification of assertions.",
                outcome.confidence_score, outcome.correction_guidance, outcome.missed_opportunities
            );
            *cognition_state.pending_correction.write().unwrap() = Some(note);

            let new_state = cognition_state.affective.write().unwrap().transition(
                DiscreteEmotion::Uncertain,
                -0.3,
                0.3,
                "Low confidence reflection",
            );
            assert_eq!(new_state.discrete, DiscreteEmotion::Uncertain);
        }

        let buffered = cognition_state.pending_correction.read().unwrap().clone();
        assert!(buffered.is_some());
        assert!(buffered.unwrap().contains("confidence was evaluated low (0.45)"));
    }
}
