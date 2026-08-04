use serde::{Deserialize, Serialize};
use crate::privacy::firewall::PrivacyCheckResponse;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolManifest {
    pub tool_id: String,
    pub name: String,
    pub description: String,
    pub risk: String,
    pub requires_approval: bool,
    pub dangerous: bool,
    pub applies_changes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolListResponse {
    pub tools: Vec<ToolManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResultItem {
    pub title: String,
    pub summary: String,
    pub url: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolRunResponse {
    pub tool_id: String,
    pub status: String,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub items: Vec<ToolResultItem>,
    pub permission_required: bool,
    pub confirmation_required: bool,
    pub applies_changes: bool,
    pub privacy: Option<PrivacyCheckResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSearchRequest {
    pub query: String,
    pub limit: Option<usize>,
    pub user_approved: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebFetchRequest {
    pub url: String,
    pub user_approved: Option<bool>,
    pub max_characters: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileReadRequest {
    pub path: String,
    pub allowed_root: String,
    pub user_approved: Option<bool>,
    pub max_characters: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitReadRequest {
    pub repo_path: String,
    pub user_approved: Option<bool>,
    pub max_characters: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeTaskRequest {
    pub goal: String,
    pub context: Option<String>,
    pub user_approved: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DayPlannerRequest {
    pub date: Option<String>,
    pub commitments: Option<Vec<String>>,
    pub focus: Option<Vec<String>>,
    pub notes: Option<String>,
}
