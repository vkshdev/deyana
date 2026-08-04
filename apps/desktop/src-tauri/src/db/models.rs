use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryEntity {
    pub id: String,
    pub memory_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_uri: Option<String>,
    pub name: String,
    pub entity_type: String,
    pub source_text: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInsight {
    pub id: String,
    pub memory_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_uri: Option<String>,
    pub r#type: String, // "action_item" | "decision"
    pub title: String,
    pub detail: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryItem {
    pub id: String,
    pub r#type: String,
    pub title: String,
    pub summary: String,
    pub content_markdown: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown_path: Option<String>,
    pub source_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_uri: Option<String>,
    pub importance: i32,
    pub tags: Vec<String>,
    pub entities: Vec<MemoryEntity>,
    pub action_items: Vec<MemoryInsight>,
    pub decisions: Vec<MemoryInsight>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TriageMessageResponse {
    pub id: String,
    pub platform: String,
    pub sender: String,
    pub content: String,
    #[serde(rename = "urgency_score", alias = "urgencyScore")]
    pub urgency_score: String,
    #[serde(rename = "auto_draft", alias = "autoDraft")]
    pub auto_draft: String,
    pub status: String,
    #[serde(rename = "created_at", alias = "createdAt")]
    pub created_at: String,
}

pub type TriageItem = TriageMessageResponse;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessageItem {
    pub id: String,
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default)]
    pub source_references: Vec<serde_json::Value>,
    #[serde(default)]
    pub web_source_references: Vec<serde_json::Value>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}
