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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryRelationType {
    RelatedTo,
    DependsOn,
    Contradicts,
    SameEntity,
}

impl MemoryRelationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RelatedTo => "related_to",
            Self::DependsOn => "depends_on",
            Self::Contradicts => "contradicts",
            Self::SameEntity => "same_entity",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "related_to" => Ok(Self::RelatedTo),
            "depends_on" => Ok(Self::DependsOn),
            "contradicts" => Ok(Self::Contradicts),
            "same_entity" => Ok(Self::SameEntity),
            other => Err(format!("Unknown memory relation type: {other}")),
        }
    }

    pub fn relevance_multiplier(&self) -> f32 {
        match self {
            Self::DependsOn => 1.0,
            Self::SameEntity => 0.9,
            Self::RelatedTo => 0.8,
            Self::Contradicts => 0.7,
        }
    }
}

impl std::fmt::Display for MemoryRelationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for MemoryRelationType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str(s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryLink {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub relation_type: MemoryRelationType,
    pub weight: f32,
    pub metadata: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryLinkWithNeighbor {
    #[serde(flatten)]
    pub link: MemoryLink,
    pub neighbor_item: Option<MemoryItem>,
    pub direction: String, // "outgoing" | "incoming"
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticCacheRow {
    pub id: String,
    pub query_embedding: Vec<f32>,
    pub memory_context_hash: String,
    pub response: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySaveSuggestion {
    pub id: String,
    pub session_id: Option<String>,
    pub fact_text: String,
    pub entities: Vec<serde_json::Value>,
    pub category: String,
    pub importance: i32,
    pub rationale: String,
    pub status: String,
    pub created_memory_id: Option<String>,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfModel {
    pub id: String,
    pub identity_statement: String,
    pub core_values: Vec<String>,
    pub intellectual_interests: Vec<String>,
    pub self_observations: Vec<String>,
    pub user_observations: Vec<String>,
    pub version: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfModelAuditEntry {
    pub id: String,
    pub version: i32,
    pub field_changed: String,
    pub operation: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: serde_json::Value,
    pub rationale: String,
    pub triggered_by: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReflectionRecord {
    pub id: String,
    pub session_id: Option<String>,
    pub turn_prompt: String,
    pub turn_response: String,
    pub accuracy_score: f32,
    pub completeness_score: f32,
    pub confidence_score: f32,
    pub missed_opportunities: String,
    pub correction_guidance: String,
    pub flagged_for_correction: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProactiveThoughtRecord {
    pub id: String,
    pub trigger_reason: String,
    pub thought_summary: String,
    pub hypotheses: Vec<String>,
    pub source_memory_ids: Vec<String>,
    pub insight_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MindCognitionState {
    pub affective_state: crate::cognition::affective::AffectiveState,
    pub self_model: SelfModel,
    pub pending_correction_note: Option<String>,
    pub recent_reflections: Vec<ReflectionRecord>,
    pub recent_proactive_thoughts: Vec<ProactiveThoughtRecord>,
}


