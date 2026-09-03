use std::time::Duration;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use super::types::{
    LlmGenerateRequest, LlmStreamChunk, LocalModelInfo, LocalModelRole, LocalModelStatusResponse,
    ModelProfile, ModelStatus,
};

const DEFAULT_OLLAMA_ENDPOINT: &str = "http://localhost:11434";
const DEFAULT_CHAT_MODEL: &str = "qwen3:1.7b";
const DEFAULT_EMBEDDING_MODEL: &str = "all-minilm:latest";

#[derive(Debug, Clone)]
pub struct OllamaClient {
    client: reqwest::Client,
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Option<Vec<OllamaModelTag>>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelTag {
    name: String,
    size: Option<u64>,
}

#[derive(Debug, Serialize)]
struct OllamaGeneratePayload {
    model: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct OllamaStreamPayload {
    model: Option<String>,
    response: Option<String>,
    #[serde(default)]
    done: bool,
}

#[derive(Debug, Serialize)]
struct OllamaEmbeddingPayload {
    model: String,
    prompt: String,
}

#[derive(Debug, Deserialize)]
struct OllamaEmbeddingResponse {
    embedding: Vec<f32>,
}

impl OllamaClient {
    pub fn new(base_url: Option<String>) -> Self {
        let endpoint = base_url
            .unwrap_or_else(|| DEFAULT_OLLAMA_ENDPOINT.to_string())
            .trim_end_matches('/')
            .to_string();
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self {
            client,
            base_url: endpoint,
        }
    }

    pub fn endpoint(&self) -> &str {
        &self.base_url
    }

    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, String> {
        let url = format!("{}/api/embeddings", self.base_url);
        let payload = OllamaEmbeddingPayload {
            model: DEFAULT_EMBEDDING_MODEL.to_string(),
            prompt: text.to_string(),
        };

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| format!("Failed to send embedding request to Ollama ({url}): {e}"))?;

        if !response.status().is_success() {
            return Err(format!("Ollama embedding request failed HTTP {}", response.status()));
        }

        let data: OllamaEmbeddingResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Ollama embedding response: {e}"))?;

        Ok(data.embedding)
    }

    pub async fn list_models(&self) -> Result<Vec<LocalModelInfo>, String> {
        let url = format!("{}/api/tags", self.base_url);
        let res = self
            .client
            .get(&url)
            .timeout(Duration::from_secs(3))
            .send()
            .await
            .map_err(|e| format!("Failed to connect to Ollama at {url}: {e}"))?;

        if !res.status().is_success() {
            return Err(format!("Ollama API returned HTTP status {}", res.status()));
        }

        let tags: OllamaTagsResponse = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse Ollama model tags: {e}"))?;

        let models = tags
            .models
            .unwrap_or_default()
            .into_iter()
            .map(|tag| {
                let is_embed = tag.name.contains("embed") || tag.name.contains("minilm");
                let role = if is_embed {
                    LocalModelRole::Embedding
                } else {
                    LocalModelRole::Chat
                };
                let recommended = tag.name == DEFAULT_CHAT_MODEL || tag.name == DEFAULT_EMBEDDING_MODEL;
                LocalModelInfo {
                    name: tag.name.clone(),
                    role,
                    installed: true,
                    recommended,
                    profile: Some(ModelProfile::Balanced),
                    size_bytes: tag.size,
                    detail: format!("Ollama local model ({})", tag.name),
                }
            })
            .collect();

        Ok(models)
    }

    pub async fn check_status(&self) -> LocalModelStatusResponse {
        let checked_at = chrono::Utc::now().to_rfc3339();
        match self.list_models().await {
            Ok(models) => {
                let chat_available = models.iter().any(|m| m.role == LocalModelRole::Chat);
                let embed_available = models.iter().any(|m| m.role == LocalModelRole::Embedding);
                let selected_chat = models
                    .iter()
                    .find(|m| m.role == LocalModelRole::Chat)
                    .map(|m| m.name.clone())
                    .unwrap_or_else(|| DEFAULT_CHAT_MODEL.to_string());
                let selected_embed = models
                    .iter()
                    .find(|m| m.role == LocalModelRole::Embedding)
                    .map(|m| m.name.clone())
                    .unwrap_or_else(|| DEFAULT_EMBEDDING_MODEL.to_string());

                LocalModelStatusResponse {
                    provider: "ollama".to_string(),
                    status: if chat_available {
                        ModelStatus::Available
                    } else {
                        ModelStatus::Missing
                    },
                    endpoint: self.base_url.clone(),
                    selected_chat_model: selected_chat,
                    selected_embedding_model: selected_embed,
                    recommended_chat_model: DEFAULT_CHAT_MODEL.to_string(),
                    recommended_embedding_model: DEFAULT_EMBEDDING_MODEL.to_string(),
                    chat_model_available: chat_available,
                    embedding_model_available: embed_available,
                    available_models: models,
                    setup_models: vec![],
                    max_parallel_model_jobs: 1,
                    think: false,
                    message: "Ollama service is active and responsive.".to_string(),
                    checked_at,
                    profile: Some(ModelProfile::Balanced),
                    n_ctx_tokens: 4096,
                }
            }
            Err(err) => LocalModelStatusResponse {
                provider: "ollama".to_string(),
                status: ModelStatus::Offline,
                endpoint: self.base_url.clone(),
                selected_chat_model: DEFAULT_CHAT_MODEL.to_string(),
                selected_embedding_model: DEFAULT_EMBEDDING_MODEL.to_string(),
                recommended_chat_model: DEFAULT_CHAT_MODEL.to_string(),
                recommended_embedding_model: DEFAULT_EMBEDDING_MODEL.to_string(),
                chat_model_available: false,
                embedding_model_available: false,
                available_models: vec![],
                setup_models: vec![],
                max_parallel_model_jobs: 1,
                think: false,
                message: format!("Ollama service is offline or unreachable: {err}"),
                checked_at,
                profile: Some(ModelProfile::Balanced),
                n_ctx_tokens: 4096,
            },
        }
    }


    pub async fn generate_stream<F>(
        &self,
        req: &LlmGenerateRequest,
        mut callback: F,
    ) -> Result<String, String>
    where
        F: FnMut(LlmStreamChunk) + Send,
    {
        let url = format!("{}/api/generate", self.base_url);
        let model = req
            .model
            .clone()
            .unwrap_or_else(|| DEFAULT_CHAT_MODEL.to_string());

        let payload = OllamaGeneratePayload {
            model: model.clone(),
            prompt: req.prompt.clone(),
            system: req.system.clone(),
            stream: true,
            options: req.temperature.map(|t| OllamaOptions {
                temperature: Some(t),
            }),
        };

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Failed to send generation request to Ollama ({url}): {e}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let err_text = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".to_string());
            return Err(format!("Ollama generation failed [HTTP {status}]: {err_text}"));
        }

        let mut stream = response.bytes_stream();
        let mut buffer = Vec::new();
        let mut full_text = String::new();

        while let Some(chunk_res) = stream.next().await {
            let chunk_bytes = chunk_res.map_err(|e| format!("Stream read error: {e}"))?;
            buffer.extend_from_slice(&chunk_bytes);

            while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
                let line_bytes: Vec<u8> = buffer.drain(..=pos).collect();
                let line_str = String::from_utf8_lossy(&line_bytes);
                let trimmed = line_str.trim();
                if trimmed.is_empty() {
                    continue;
                }

                if let Ok(payload) = serde_json::from_str::<OllamaStreamPayload>(trimmed) {
                    let chunk_text = payload.response.unwrap_or_default();
                    full_text.push_str(&chunk_text);

                    let stream_chunk = LlmStreamChunk {
                        response: chunk_text,
                        done: payload.done,
                        model: payload.model.or_else(|| Some(model.clone())),
                    };
                    callback(stream_chunk);
                }
            }
        }

        if !buffer.is_empty() {
            let line_str = String::from_utf8_lossy(&buffer);
            let trimmed = line_str.trim();
            if !trimmed.is_empty() {
                if let Ok(payload) = serde_json::from_str::<OllamaStreamPayload>(trimmed) {
                    let chunk_text = payload.response.unwrap_or_default();
                    full_text.push_str(&chunk_text);

                    let stream_chunk = LlmStreamChunk {
                        response: chunk_text,
                        done: payload.done,
                        model: payload.model.or_else(|| Some(model.clone())),
                    };
                    callback(stream_chunk);
                }
            }
        }

        Ok(full_text)
    }
}
