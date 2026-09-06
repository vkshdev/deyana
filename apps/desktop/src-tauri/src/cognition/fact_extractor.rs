use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

use crate::agent::cache::SemanticCache;
use crate::db::{self, DbPool};
use crate::models::{LlmGenerateRequest, LlmRouter};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedEntity {
    pub name: String,
    #[serde(alias = "entity_type")]
    pub entity_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedFactItem {
    pub fact: String,
    pub category: String,
    #[serde(default)]
    pub entities: Vec<ExtractedEntity>,
    pub importance: i32,
    pub rationale: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExtractedFactsPayload {
    pub facts: Vec<ExtractedFactItem>,
}

pub struct FactExtractor;

pub fn is_conversational_filler(text: &str) -> bool {
    let lower = text.trim().to_lowercase();
    let normalized = lower.trim_end_matches(['.', '!', '?', ',', ' ']);

    matches!(
        normalized,
        "hi" | "hello" | "hey" | "ok" | "okay" | "k" | "sure" | "yep" | "yes" | "no"
            | "nope" | "thanks" | "thank you" | "ty" | "thx" | "got it" | "cool"
            | "great" | "awesome" | "perfect" | "bye" | "good morning" | "good night"
            | "see ya" | "sounds good" | "understood"
    )
}

pub fn parse_extracted_facts_json(raw: &str) -> Option<ExtractedFactsPayload> {
    let cleaned = raw.trim();

    // 1. Direct parse attempt
    if let Ok(payload) = serde_json::from_str::<ExtractedFactsPayload>(cleaned) {
        return Some(payload);
    }

    // 2. Strip markdown code fence if present
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

    if let Ok(payload) = serde_json::from_str::<ExtractedFactsPayload>(content.trim()) {
        return Some(payload);
    }

    // 3. Slice between first '{' and last '}'
    if let (Some(first_brace), Some(last_brace)) = (content.find('{'), content.rfind('}')) {
        if first_brace < last_brace {
            let slice = &content[first_brace..=last_brace];
            if let Ok(payload) = serde_json::from_str::<ExtractedFactsPayload>(slice) {
                return Some(payload);
            }
        }
    }

    None
}

impl FactExtractor {
    pub async fn process_turn_async(
        app: AppHandle,
        pool: DbPool,
        router: Arc<LlmRouter>,
        user_prompt: String,
        assistant_response: String,
        session_id: Option<String>,
    ) {
        let trimmed_prompt = user_prompt.trim();
        if trimmed_prompt.len() < 12 || is_conversational_filler(trimmed_prompt) {
            return;
        }

        let extraction_system = r#"You are Deyana's memory distillation engine.
Analyze the user and assistant turn. Extract permanent facts, preferences, project architectures, or key technical decisions.
Respond ONLY with a JSON object matching this schema:
{
  "facts": [
    {
      "fact": "statement of long-term fact or preference",
      "category": "project|preference|personal|workflow",
      "entities": [{"name": "name", "entityType": "type"}],
      "importance": 1-5,
      "rationale": "reason for importance"
    }
  ]
}"#;

        let extraction_prompt = format!(
            "User: {}\nAssistant: {}\nExtract any noteworthy long-term facts or preferences as JSON:",
            trimmed_prompt, assistant_response.trim()
        );

        let req = LlmGenerateRequest {
            prompt: extraction_prompt,
            model: None,
            system: Some(extraction_system.to_string()),
            temperature: Some(0.1),
            stream: Some(false),
            grammar: None,
        };

        if let Ok(raw_json) = router.generate_response(&req, |_| {}).await {
            if let Some(parsed) = parse_extracted_facts_json(&raw_json) {
                let conn = match pool.get() {
                    Ok(c) => c,
                    Err(_) => return,
                };

                for item in parsed.facts {
                    if item.importance >= 4 {
                        // High Importance (4-5): Auto-commit to memory_items
                        let tags = vec!["auto_extracted".to_string(), item.category.clone()];
                        if let Ok(saved) = db::save_memory_item(&conn, &item.fact, "fact_extractor", Some(tags)) {
                            // Invalidate semantic cache on memory creation
                            if let Some(cache) = app.try_state::<SemanticCache>() {
                                cache.clear(Some(&conn));
                            }
                            let _ = app.emit("memory:fact_auto_committed", &saved);
                        }
                    } else if item.importance == 3 {
                        // Medium Importance (3): Stage in memory_save_suggestions
                        let entities_json = serde_json::to_string(&item.entities).unwrap_or_else(|_| "[]".to_string());
                        if let Ok(suggestion) = db::create_memory_save_suggestion(
                            &conn,
                            session_id.as_deref(),
                            &item.fact,
                            &entities_json,
                            &item.category,
                            item.importance,
                            &item.rationale,
                        ) {
                            let card_payload = serde_json::json!({
                                "id": suggestion.id,
                                "fact": suggestion.fact_text,
                                "category": suggestion.category,
                                "importance": suggestion.importance,
                                "rationale": suggestion.rationale,
                                "entities": suggestion.entities,
                            });
                            let _ = app.emit("memory:save_card", &card_payload);
                        }
                    }
                    // Low Importance (1-2) is discarded
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_conversational_filler() {
        assert!(is_conversational_filler("hi"));
        assert!(is_conversational_filler("Hello!"));
        assert!(is_conversational_filler("thanks."));
        assert!(is_conversational_filler("ok"));
        assert!(is_conversational_filler("Got it!"));
        assert!(is_conversational_filler("cool"));
        assert!(!is_conversational_filler("My name is Alex and I live in Seattle"));
        assert!(!is_conversational_filler("We use PostgreSQL for storage"));
    }

    #[test]
    fn test_parse_extracted_facts_json() {
        let raw = r#"```json
        {
          "facts": [
            {
              "fact": "User prefers TypeScript over JavaScript",
              "category": "preference",
              "entities": [{"name": "TypeScript", "entityType": "language"}],
              "importance": 4,
              "rationale": "Explicit technical preference"
            }
          ]
        }
        ```"#;

        let parsed = parse_extracted_facts_json(raw);
        assert!(parsed.is_some());
        let payload = parsed.unwrap();
        assert_eq!(payload.facts.len(), 1);
        assert_eq!(payload.facts[0].fact, "User prefers TypeScript over JavaScript");
        assert_eq!(payload.facts[0].importance, 4);
        assert_eq!(payload.facts[0].entities[0].name, "TypeScript");
    }
}
