use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocalModelRole {
    Chat,
    Embedding,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelStatus {
    Available,
    Missing,
    Checking,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelProfile {
    #[serde(alias = "low_spec")]
    Balanced,
    Standard,
    Power,
    Ultra,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDefaults {
    pub recommended_chat_model: String,
    pub recommended_embedding_model: String,
    pub n_ctx_tokens: u32,
    pub max_parallel_jobs: u32,
    pub always_on_voice: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GgufRegistryEntry {
    pub id: String,
    pub display_name: String,
    pub file_path: String,
    pub size_bytes: u64,
    pub profile_tier: ModelProfile,
    pub n_ctx: u32,
    pub chat_template: Option<String>,
    pub source: String,
    pub registered_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HfGgufFile {
    pub filename: String,
    pub size_bytes: u64,
    pub quantization: String,
    pub estimated_ram_gb: f32,
    pub recommended_profile: ModelProfile,
    pub download_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgressPayload {
    pub repo_id: String,
    pub filename: String,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetStats {
    pub total_samples: usize,
    pub thumbs_up_count: usize,
    pub thumbs_down_count: usize,
    pub edited_samples_count: usize,
    pub models_breakdown: std::collections::HashMap<String, usize>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GgufModelEntry {
    pub path: String,
    pub file_name: String,
    pub size_bytes: u64,
    pub estimated_ram_gb: f32,
    pub detected_profile: ModelProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveProfileInfo {
    pub profile: ModelProfile,
    pub ram_gb: u64,
    pub recommended_chat_model: String,
    pub recommended_embedding_model: String,
    pub n_ctx_tokens: u32,
    pub max_parallel_jobs: u32,
    pub always_on_voice: bool,
    pub local_ggufs: Vec<GgufModelEntry>,

}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalModelInfo {
    pub name: String,
    pub role: LocalModelRole,
    pub installed: bool,
    pub recommended: bool,
    pub profile: Option<ModelProfile>,
    pub size_bytes: Option<u64>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalModelStatusResponse {
    pub provider: String,
    pub status: ModelStatus,
    pub endpoint: String,
    pub selected_chat_model: String,
    pub selected_embedding_model: String,
    pub recommended_chat_model: String,
    pub recommended_embedding_model: String,
    pub chat_model_available: bool,
    pub embedding_model_available: bool,
    pub available_models: Vec<LocalModelInfo>,
    pub setup_models: Vec<LocalModelInfo>,
    pub max_parallel_model_jobs: u32,
    pub think: bool,
    pub message: String,
    pub checked_at: String,
    pub profile: Option<ModelProfile>,
    pub n_ctx_tokens: u32,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlamaCppServerStatus {
    pub running: bool,
    pub endpoint: String,
    pub model_path: Option<String>,
    pub n_ctx: u32,
    pub pid: Option<u32>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmGenerateRequest {
    pub prompt: String,
    pub model: Option<String>,
    pub system: Option<String>,
    pub temperature: Option<f32>,
    pub stream: Option<bool>,
    pub grammar: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmGenerateResponse {
    pub response: String,
    pub model: String,
    pub done: bool,
    pub total_duration_ns: Option<u64>,
    pub load_duration_ns: Option<u64>,
    pub prompt_eval_count: Option<u32>,
    pub eval_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmStreamChunk {
    pub response: String,
    pub done: bool,
    pub model: Option<String>,
}

