use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use futures_util::StreamExt;
use reqwest_eventsource::{Event, EventSource};
use serde::{Deserialize, Serialize};

use super::hardware::HardwareProfileDetector;
use super::types::{
    LlamaCppServerStatus, LlmGenerateRequest, LlmStreamChunk, LocalModelInfo, LocalModelRole,
    ModelProfile,
};

pub const DEFAULT_LLAMACPP_PORT: u16 = 8080;

#[derive(Debug, Clone)]
pub struct LlamaCppEngine {
    model_path: Arc<RwLock<Option<String>>>,
    port: u16,
    endpoint: String,
    child_pid: Arc<RwLock<Option<u32>>>,
    n_ctx: Arc<AtomicU32>,
    last_error: Arc<RwLock<Option<String>>>,
    http_client: reqwest::Client,
}

impl Default for LlamaCppEngine {
    fn default() -> Self {
        Self::new(None)
    }
}

#[derive(Debug, Serialize)]
struct LlamaCppCompletionPayload<'a> {
    prompt: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
    n_predict: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    grammar: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_prompt: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
struct LlamaCppChunk {
    content: Option<String>,
    stop: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct LlamaCppHealthResponse {
    status: Option<String>,
}

impl LlamaCppEngine {
    pub fn new(model_path: Option<String>) -> Self {
        let port = DEFAULT_LLAMACPP_PORT;
        let endpoint = format!("http://127.0.0.1:{port}");
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap_or_default();

        Self {
            model_path: Arc::new(RwLock::new(model_path)),
            port,
            endpoint,
            child_pid: Arc::new(RwLock::new(None)),
            n_ctx: Arc::new(AtomicU32::new(4096)),
            last_error: Arc::new(RwLock::new(None)),
            http_client: client,
        }
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn current_model_path(&self) -> Option<String> {
        self.model_path.read().ok().and_then(|p| p.clone())
    }

    pub fn active_n_ctx(&self) -> u32 {
        self.n_ctx.load(Ordering::Relaxed)
    }

    /// Check if the engine has an active model file or running server
    pub fn is_available(&self) -> bool {
        if self.child_pid.read().ok().map_or(false, |pid| pid.is_some()) {
            return true;
        }
        self.model_path
            .read()
            .ok()
            .and_then(|p| p.clone())
            .map_or(false, |p| Path::new(&p).exists())
    }

    /// Query the llama-server `/health` endpoint
    pub async fn check_health(&self) -> bool {
        let url = format!("{}/health", self.endpoint);
        match self.http_client.get(&url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    let health: Result<LlamaCppHealthResponse, _> = resp.json().await;
                    health
                        .map(|h| h.status.map_or(true, |s| s == "ok" || s == "healthy"))
                        .unwrap_or(true)
                } else {
                    false
                }
            }
            Err(_) => false,
        }
    }

    /// Get current server status struct
    pub fn get_status(&self) -> LlamaCppServerStatus {
        let pid = self.child_pid.read().ok().and_then(|p| *p);
        let model_path = self.current_model_path();
        let running = pid.is_some();
        let last_error = self.last_error.read().ok().and_then(|e| e.clone());

        LlamaCppServerStatus {
            running,
            endpoint: self.endpoint.clone(),
            model_path,
            n_ctx: self.active_n_ctx(),
            pid,
            last_error,
        }
    }

    /// Start llama-server as a managed background child process
    pub async fn start_server(
        &self,
        model_path: &str,
        n_ctx: Option<u32>,
        port: Option<u16>,
    ) -> Result<LlamaCppServerStatus, String> {
        if !Path::new(model_path).exists() {
            let err = format!("Model GGUF file not found at path: {model_path}");
            if let Ok(mut e) = self.last_error.write() {
                *e = Some(err.clone());
            }
            return Err(err);
        }

        // Stop any running instance first
        let _ = self.stop_server().await;

        let selected_port = port.unwrap_or(self.port);
        let ctx = n_ctx.unwrap_or(self.active_n_ctx());
        self.n_ctx.store(ctx, Ordering::Relaxed);

        // Find llama-server executable
        let server_binary = find_llama_server_binary()
            .unwrap_or_else(|| PathBuf::from("llama-server"));

        let mut cmd = tokio::process::Command::new(server_binary);
        cmd.arg("-m")
            .arg(model_path)
            .arg("-c")
            .arg(ctx.to_string())
            .arg("--port")
            .arg(selected_port.to_string())
            .arg("--host")
            .arg("127.0.0.1")
            .arg("-np")
            .arg("1")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        #[cfg(windows)]
        {
            // CREATE_NO_WINDOW = 0x08000000
            cmd.creation_flags(0x08000000);
        }

        match cmd.spawn() {
            Ok(child) => {
                let pid = child.id();
                if let Ok(mut lock) = self.child_pid.write() {
                    *lock = pid;
                }
                if let Ok(mut p) = self.model_path.write() {
                    *p = Some(model_path.to_string());
                }
                if let Ok(mut e) = self.last_error.write() {
                    *e = None;
                }

                // Poll health for up to 5 seconds
                let mut is_healthy = false;
                for _ in 0..10 {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    if self.check_health().await {
                        is_healthy = true;
                        break;
                    }
                }

                if !is_healthy {
                    // Server spawned but hasn't responded yet or returned non-200, still keep pid if running
                }

                Ok(self.get_status())
            }
            Err(spawn_err) => {
                let err = format!(
                    "Failed to launch llama-server subprocess: {spawn_err}. Please ensure `llama-server` is in your PATH or in ~/.deyana/bin/"
                );
                if let Ok(mut e) = self.last_error.write() {
                    *e = Some(err.clone());
                }
                Err(err)
            }
        }
    }

    /// Stop the managed llama-server child process
    pub async fn stop_server(&self) -> Result<(), String> {
        let pid_opt = {
            let mut lock = self.child_pid.write().map_err(|e| e.to_string())?;
            lock.take()
        };

        if let Some(pid) = pid_opt {
            #[cfg(windows)]
            {
                let _ = std::process::Command::new("taskkill")
                    .args(["/F", "/PID", &pid.to_string()])
                    .output();
            }
            #[cfg(not(windows))]
            {
                let _ = std::process::Command::new("kill")
                    .args(["-9", &pid.to_string()])
                    .output();
            }
        }

        Ok(())
    }

    /// List models detected in ~/.deyana/models and any currently configured model
    pub async fn list_models(&self) -> Vec<LocalModelInfo> {
        let mut models = Vec::new();

        // Scan local GGUFs
        let ggufs = HardwareProfileDetector::scan_default_models_dir();
        for gguf in ggufs {
            models.push(LocalModelInfo {
                name: gguf.file_name.clone(),
                role: LocalModelRole::Chat,
                installed: true,
                recommended: false,
                profile: Some(gguf.detected_profile),
                size_bytes: Some(gguf.size_bytes),
                detail: format!("Local GGUF model at {}", gguf.path),
            });
        }

        // Add currently configured path if not already in list
        if let Some(ref path) = self.current_model_path() {
            let path_obj = Path::new(path);
            let file_name = path_obj
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "custom-model.gguf".to_string());

            if !models.iter().any(|m| m.name == file_name) && path_obj.exists() {
                models.push(LocalModelInfo {
                    name: file_name,
                    role: LocalModelRole::Chat,
                    installed: true,
                    recommended: false,
                    profile: Some(ModelProfile::Balanced),
                    size_bytes: path_obj.metadata().ok().map(|m| m.len()),
                    detail: format!("Configured llama.cpp model at {path}"),
                });
            }
        }

        models
    }

    /// Stream completion from llama-server `/completion` endpoint
    pub async fn generate_stream<F>(
        &self,
        req: &LlmGenerateRequest,
        mut callback: F,
    ) -> Result<String, String>
    where
        F: FnMut(LlmStreamChunk) + Send,
    {
        let url = format!("{}/completion", self.endpoint);

        // Build prompt with system instructions if provided
        let full_prompt = if let Some(ref sys) = req.system {
            format!("<|im_start|>system\n{sys}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n", req.prompt)
        } else {
            req.prompt.clone()
        };

        let payload = LlamaCppCompletionPayload {
            prompt: &full_prompt,
            temperature: req.temperature,
            stream: true,
            n_predict: 2048,
            grammar: req.grammar.as_deref(),
            system_prompt: req.system.as_deref(),
        };

        let request = self.http_client.post(&url).json(&payload);

        let mut event_source = EventSource::new(request)
            .map_err(|e| format!("Failed to create SSE stream connection to llama-server: {e}"))?;

        let mut full_response = String::new();

        while let Some(event_result) = event_source.next().await {
            match event_result {
                Ok(Event::Message(message)) => {
                    if let Ok(chunk) = serde_json::from_str::<LlamaCppChunk>(&message.data) {
                        if let Some(content) = chunk.content {
                            full_response.push_str(&content);
                            callback(LlmStreamChunk {
                                response: content,
                                done: false,
                                model: Some("llama.cpp".to_string()),
                            });
                        }
                        if chunk.stop == Some(true) {
                            break;
                        }
                    }
                }
                Ok(Event::Open) => {}
                Err(err) => {
                    // Check if we already received content (stream completed normally)
                    if !full_response.is_empty() {
                        break;
                    }
                    return Err(format!("llama-server SSE stream error: {err}"));
                }
            }
        }

        callback(LlmStreamChunk {
            response: String::new(),
            done: true,
            model: Some("llama.cpp".to_string()),
        });

        Ok(full_response)
    }
}

fn find_llama_server_binary() -> Option<PathBuf> {
    let base = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    
    #[cfg(windows)]
    let binary_name = "llama-server.exe";
    #[cfg(not(windows))]
    let binary_name = "llama-server";

    let deyana_bin = base.join(".deyana").join("bin").join(binary_name);
    if deyana_bin.exists() {
        return Some(deyana_bin);
    }

    let local_bin = PathBuf::from(binary_name);
    if local_bin.exists() {
        return Some(local_bin);
    }

    None
}


#[cfg(test)]
#[path = "tests/llamacpp_tests.rs"]
mod tests;


