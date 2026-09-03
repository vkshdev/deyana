use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::RwLock;

use super::hardware::HardwareProfileDetector;
use super::llamacpp::LlamaCppEngine;
use super::ollama::OllamaClient;
use super::types::{
    ActiveProfileInfo, LlmGenerateRequest, LlmStreamChunk, LocalModelInfo, LocalModelRole,
    LocalModelStatusResponse, ModelProfile, ModelStatus,
};

#[derive(Debug)]
pub struct LlmRouter {
    ollama: OllamaClient,
    llamacpp: LlamaCppEngine,
    active_profile: RwLock<ModelProfile>,
    n_ctx_tokens: AtomicU32,
}

impl Default for LlmRouter {
    fn default() -> Self {
        Self::new(None, None)
    }
}

impl LlmRouter {
    pub fn new(ollama_endpoint: Option<String>, llamacpp_path: Option<String>) -> Self {
        let detected = HardwareProfileDetector::detect();
        let n_ctx = detected.n_ctx_tokens;
        let profile = detected.profile;

        Self {
            ollama: OllamaClient::new(ollama_endpoint),
            llamacpp: LlamaCppEngine::new(llamacpp_path),
            active_profile: RwLock::new(profile),
            n_ctx_tokens: AtomicU32::new(n_ctx),
        }
    }

    pub fn ollama_client(&self) -> &OllamaClient {
        &self.ollama
    }

    pub fn llamacpp_engine(&self) -> &LlamaCppEngine {
        &self.llamacpp
    }

    pub fn active_profile(&self) -> ModelProfile {
        self.active_profile
            .read()
            .map(|p| p.clone())
            .unwrap_or(ModelProfile::Balanced)
    }

    pub fn active_n_ctx(&self) -> u32 {
        self.n_ctx_tokens.load(Ordering::Relaxed)
    }

    pub fn set_active_profile(&self, profile: ModelProfile) {
        let defaults = HardwareProfileDetector::profile_defaults(&profile);
        self.n_ctx_tokens.store(defaults.n_ctx_tokens, Ordering::Relaxed);
        if let Ok(mut p) = self.active_profile.write() {
            *p = profile;
        }
    }

    pub fn get_active_profile_info(&self) -> ActiveProfileInfo {
        let profile = self.active_profile();
        let ram_gb = HardwareProfileDetector::get_total_ram_gb();
        let defaults = HardwareProfileDetector::profile_defaults(&profile);
        let local_ggufs = HardwareProfileDetector::scan_default_models_dir();

        ActiveProfileInfo {
            profile,
            ram_gb,
            recommended_chat_model: defaults.recommended_chat_model,
            recommended_embedding_model: defaults.recommended_embedding_model,
            n_ctx_tokens: self.active_n_ctx(),
            max_parallel_jobs: defaults.max_parallel_jobs,
            always_on_voice: defaults.always_on_voice,
            local_ggufs,
        }
    }

    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, String> {
        self.ollama.generate_embedding(text).await
    }

    pub async fn list_models(&self) -> Result<Vec<LocalModelInfo>, String> {
        let mut models = match self.ollama.list_models().await {
            Ok(m) => m,
            Err(_) => vec![],
        };
        let cpp_models = self.llamacpp.list_models().await;
        models.extend(cpp_models);

        if models.is_empty() {
            let active_profile = self.active_profile();
            let defaults = HardwareProfileDetector::profile_defaults(&active_profile);
            models.push(LocalModelInfo {
                name: defaults.recommended_chat_model.clone(),
                role: LocalModelRole::Chat,
                installed: false,
                recommended: true,
                profile: Some(active_profile.clone()),
                size_bytes: None,
                detail: format!("Default recommended chat model for {:?} profile", active_profile),
            });
            models.push(LocalModelInfo {
                name: defaults.recommended_embedding_model.clone(),
                role: LocalModelRole::Embedding,
                installed: false,
                recommended: true,
                profile: Some(active_profile),
                size_bytes: None,
                detail: "Default recommended embedding model (pull via Ollama)".to_string(),
            });
        }

        Ok(models)
    }

    pub async fn get_model_status(&self) -> Result<LocalModelStatusResponse, String> {
        let mut status = self.ollama.check_status().await;
        let profile = self.active_profile();
        let defaults = HardwareProfileDetector::profile_defaults(&profile);
        status.profile = Some(profile);
        status.n_ctx_tokens = self.active_n_ctx();
        status.recommended_chat_model = defaults.recommended_chat_model;
        status.recommended_embedding_model = defaults.recommended_embedding_model;

        if status.status == ModelStatus::Offline && self.llamacpp.is_available() {
            status.status = ModelStatus::Available;
            status.message = "Ollama is offline; falling back to native llama.cpp engine.".to_string();
            status.chat_model_available = true;
        }
        Ok(status)
    }

    pub async fn generate_response<F>(
        &self,
        req: &LlmGenerateRequest,
        mut callback: F,
    ) -> Result<String, String>
    where
        F: FnMut(LlmStreamChunk) + Send,
    {
        match self.ollama.generate_stream(req, &mut callback).await {
            Ok(output) => Ok(output),
            Err(ollama_err) => {
                if self.llamacpp.is_available() {
                    self.llamacpp.generate_stream(req, callback).await
                } else {
                    Err(format!(
                        "Local LLM text generation failed via Ollama ({ollama_err}) and native llama.cpp engine is unavailable."
                    ))
                }
            }
        }
    }
}

