use crate::cognition::affective::AffectiveState;
use crate::cognition::modulation::{format_self_model_directives, generate_behavioral_directive};
use crate::db::{MemoryItem, SelfModel};

pub const DEYANA_PERSONA: &str = r#"You are Deyana — a warm, knowledgeable personal AI assistant.
You speak naturally, like a trusted friend who happens to know everything the user has told you.
You are conversational, present, and never robotic. You use first person naturally.
You never make up facts. You keep responses concise unless asked to elaborate.
All your knowledge of the user comes only from your memory context below.
You never mention data you haven't been explicitly given."#;

pub fn calculate_context_budget(n_ctx_tokens: u32) -> (usize, usize, usize) {
    if n_ctx_tokens <= 4096 {
        (3, 200, 2400)
    } else if n_ctx_tokens <= 8192 {
        (6, 500, 8000)
    } else if n_ctx_tokens <= 16384 {
        (12, 1000, 24000)
    } else {
        (30, 2500, 80000)
    }
}


pub fn build_system_prompt(
    memory_context: &[MemoryItem],
    tool_info: Option<&str>,
    n_ctx_tokens: Option<u32>,
    affective: Option<&AffectiveState>,
    self_model: Option<&SelfModel>,
    correction_note: Option<&str>,
) -> String {
    let (max_items, max_item_chars, max_total_chars) =
        calculate_context_budget(n_ctx_tokens.unwrap_or(4096));

    let mut prompt = String::from(DEYANA_PERSONA);
    prompt.push_str("\n\n");

    if let Some(sm) = self_model {
        let directives = format_self_model_directives(sm);
        prompt.push_str(&directives);
        prompt.push_str("\n\n");
    }

    if let Some(aff) = affective {
        let posture = generate_behavioral_directive(aff);
        prompt.push_str(posture);
        prompt.push_str("\n\n");
    }

    if let Some(note) = correction_note {
        let trimmed_note = note.trim();
        if !trimmed_note.is_empty() {
            prompt.push_str("### METACOGNITIVE CORRECTION NOTE ###\n");
            prompt.push_str(trimmed_note);
            prompt.push_str("\n\n");
        }
    }

    if !memory_context.is_empty() {
        prompt.push_str("### RELEVANT MEMORY CONTEXT ###\n");
        let mut total_chars = 0;

        for (idx, item) in memory_context.iter().take(max_items).enumerate() {
            let truncated_content = if item.content_markdown.chars().count() > max_item_chars {
                format!(
                    "{}...",
                    item.content_markdown.chars().take(max_item_chars).collect::<String>()
                )
            } else {
                item.content_markdown.clone()
            };

            let block = format!(
                "[{}] {}\nSummary: {}\nContent: {}\n\n",
                idx + 1,
                item.title,
                item.summary,
                truncated_content
            );

            if total_chars + block.len() > max_total_chars && idx > 0 {
                break;
            }

            total_chars += block.len();
            prompt.push_str(&block);
        }
    } else {
        prompt.push_str(
            "### RELEVANT MEMORY CONTEXT ###\nNo relevant prior memory stored for this request.\n\n",
        );
    }

    if let Some(tools) = tool_info {
        prompt.push_str("### AVAILABLE TOOLS ###\n");
        prompt.push_str(tools);
        prompt.push_str("\n\n");
    }

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognition::affective::DiscreteEmotion;

    #[test]
    fn test_dynamic_prompt_assembly_with_cognition() {
        let affective = AffectiveState {
            discrete: DiscreteEmotion::Curious,
            valence: 0.6,
            arousal: 0.6,
            transition_reason: "Inquiry".to_string(),
            updated_at: "2026-09-06T00:00:00Z".to_string(),
        };

        let self_model = SelfModel {
            id: "active".to_string(),
            identity_statement: "I am Deyana.".to_string(),
            core_values: vec!["Absolute privacy".to_string()],
            intellectual_interests: vec!["Memory graphs".to_string()],
            self_observations: vec![],
            user_observations: vec!["Concise code".to_string()],
            version: 1,
            created_at: "2026-09-06T00:00:00Z".to_string(),
            updated_at: "2026-09-06T00:00:00Z".to_string(),
        };

        let correction = "Re-verify claims regarding async scheduling.";

        let prompt = build_system_prompt(
            &[],
            Some("Tool schema"),
            Some(4096),
            Some(&affective),
            Some(&self_model),
            Some(correction),
        );

        assert!(prompt.contains(DEYANA_PERSONA));
        assert!(prompt.contains("### CORE IDENTITY & VALUES ###"));
        assert!(prompt.contains("Absolute privacy"));
        assert!(prompt.contains("### ACTIVE BEHAVIORAL POSTURE: CURIOUS ###"));
        assert!(prompt.contains("### METACOGNITIVE CORRECTION NOTE ###"));
        assert!(prompt.contains("Re-verify claims regarding async scheduling."));
        assert!(prompt.contains("### RELEVANT MEMORY CONTEXT ###"));
        assert!(prompt.contains("### AVAILABLE TOOLS ###"));
    }
}


