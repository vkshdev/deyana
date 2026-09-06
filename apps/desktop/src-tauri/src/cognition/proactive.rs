use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::db::{self, DbPool, ProactiveThoughtRecord};
use crate::models::{LlmGenerateRequest, LlmRouter};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedThoughtPayload {
    pub title: String,
    #[serde(alias = "thought_summary")]
    pub thought_summary: String,
    #[serde(default)]
    pub hypotheses: Vec<String>,
    #[serde(default, alias = "source_memory_ids")]
    pub source_memory_ids: Vec<String>,
}

pub struct ProactiveThoughtGenerator;

pub fn parse_thought_json(raw: &str) -> Option<GeneratedThoughtPayload> {
    let cleaned = raw.trim();

    if let Ok(payload) = serde_json::from_str::<GeneratedThoughtPayload>(cleaned) {
        return Some(payload);
    }

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

    if let Ok(payload) = serde_json::from_str::<GeneratedThoughtPayload>(content.trim()) {
        return Some(payload);
    }

    if let (Some(first_brace), Some(last_brace)) = (content.find('{'), content.rfind('}')) {
        if first_brace < last_brace {
            let slice = &content[first_brace..=last_brace];
            if let Ok(payload) = serde_json::from_str::<GeneratedThoughtPayload>(slice) {
                return Some(payload);
            }
        }
    }

    None
}

impl ProactiveThoughtGenerator {
    pub async fn execute_cycle(
        app: &AppHandle,
        pool: &DbPool,
        router: &LlmRouter,
        trigger_reason: &str,
    ) -> Result<Option<ProactiveThoughtRecord>, String> {
        let conn = pool.get().map_err(|e| e.to_string())?;

        let recent_memories = db::get_memory_items(&conn, Some(10)).map_err(|e| e.to_string())?;
        if recent_memories.len() < 2 {
            return Ok(None);
        }

        let memory_ids: Vec<String> = recent_memories.iter().map(|m| m.id.clone()).collect();
        let links = db::get_links_for_memory_ids(&conn, &memory_ids).unwrap_or_default();

        let mut memories_desc = String::new();
        for (i, m) in recent_memories.iter().enumerate() {
            memories_desc.push_str(&format!("{}. [{}] {}: {}\n", i + 1, m.id, m.title, m.summary));
        }

        let mut links_desc = String::new();
        if !links.is_empty() {
            links_desc.push_str("Knowledge Graph Links:\n");
            for l in &links {
                links_desc.push_str(&format!(
                    "- [{}] --({})--> [{}] (weight: {:.2})\n",
                    l.source_id,
                    l.relation_type.as_str(),
                    l.target_id,
                    l.weight
                ));
            }
        }

        let prompt = format!(
            "Analyze the user's stored knowledge graph memories and relational links:\n\n{}\n{}\n\
Synthesize a proactive cognitive insight connecting these ideas, suggesting a helpful hypothesis or latent synergy for the user's ongoing work.\n\
Respond ONLY with valid JSON matching this schema:\n\
{{\n  \"title\": \"Insight Title\",\n  \"thoughtSummary\": \"Comprehensive synthesis of the latent connection\",\n  \"hypotheses\": [\"Hypothesis 1\", \"Hypothesis 2\"],\n  \"sourceMemoryIds\": [\"id1\", \"id2\"]\n}}",
            memories_desc, links_desc
        );

        let req = LlmGenerateRequest {
            prompt,
            model: None,
            system: Some("You are Deyana's proactive cognitive synthesis engine.".to_string()),
            temperature: Some(0.3),
            stream: Some(false),
            grammar: None,
        };

        if let Ok(raw_json) = router.generate_response(&req, |_| {}).await {
            if let Some(parsed) = parse_thought_json(&raw_json) {
                let now = chrono::Utc::now().to_rfc3339();
                let insight_id = format!("ins_{}", Uuid::new_v4().simple());
                let first_mem_id = parsed.source_memory_ids.first()
                    .cloned()
                    .unwrap_or_else(|| recent_memories[0].id.clone());

                let _ = conn.execute(
                    "INSERT INTO memory_insights (id, memory_id, type, title, detail, status, created_at)
                     VALUES (?1, ?2, 'cognitive_insight', ?3, ?4, 'open', ?5)",
                    rusqlite::params![
                        &insight_id,
                        &first_mem_id,
                        &parsed.title,
                        &parsed.thought_summary,
                        &now
                    ],
                );

                let source_ids = if parsed.source_memory_ids.is_empty() {
                    vec![first_mem_id]
                } else {
                    parsed.source_memory_ids
                };

                let record = db::save_proactive_thought(
                    &conn,
                    trigger_reason,
                    &parsed.thought_summary,
                    &parsed.hypotheses,
                    &source_ids,
                    Some(&insight_id),
                ).map_err(|e| e.to_string())?;

                let card = crate::daemon::ProactiveContextCard {
                    id: format!("card_thought_{}", chrono::Utc::now().timestamp_millis()),
                    card_type: "cognitive_insight".to_string(),
                    title: format!("Insight: {}", parsed.title),
                    summary: parsed.thought_summary.clone(),
                    action_label: Some("View Context".to_string()),
                    action_command: Some("show_main_window".to_string()),
                    priority: "normal".to_string(),
                    timestamp: now,
                    context: serde_json::json!({
                        "trigger": trigger_reason,
                        "hypotheses": parsed.hypotheses,
                        "insightId": insight_id,
                        "thoughtRecordId": record.id,
                    }),
                };

                let _ = app.emit("proactive:context_card", &card);
                let _ = app.emit("cognition:thought_generated", &record);

                return Ok(Some(record));
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_thought_json() {
        let raw = r#"```json
        {
          "title": "Cross-System Memory Convergence",
          "thoughtSummary": "The SQLite schema and vector embeddings can be synchronized via write-ahead hooks.",
          "hypotheses": [
            "Adding transactional commit hooks will eliminate cache staleness"
          ],
          "sourceMemoryIds": ["mem_1", "mem_2"]
        }
        ```"#;

        let parsed = parse_thought_json(raw);
        assert!(parsed.is_some());
        let thought = parsed.unwrap();
        assert_eq!(thought.title, "Cross-System Memory Convergence");
        assert_eq!(thought.hypotheses.len(), 1);
        assert_eq!(thought.source_memory_ids, vec!["mem_1", "mem_2"]);
    }
}
