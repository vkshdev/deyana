use std::path::{Path, PathBuf};
use sysinfo::System;

use super::types::{ActiveProfileInfo, GgufModelEntry, ModelProfile, ProfileDefaults};

pub struct HardwareProfileDetector;

impl HardwareProfileDetector {
    /// Probe total system RAM in GB.
    pub fn get_total_ram_gb() -> u64 {
        let mut sys = System::new_all();
        sys.refresh_memory();
        let total_bytes = sys.total_memory();
        total_bytes / (1024 * 1024 * 1024)
    }

    pub fn detect_profile_from_ram(ram_gb: u64) -> ModelProfile {
        if ram_gb < 16 {
            ModelProfile::Balanced
        } else if ram_gb < 32 {
            ModelProfile::Standard
        } else if ram_gb < 64 {
            ModelProfile::Power
        } else {
            ModelProfile::Ultra
        }
    }


    /// Full hardware profile detection at startup.
    pub fn detect() -> ActiveProfileInfo {
        let ram_gb = Self::get_total_ram_gb();
        let profile = Self::detect_profile_from_ram(ram_gb);
        let defaults = Self::profile_defaults(&profile);
        let local_ggufs = Self::scan_default_models_dir();

        ActiveProfileInfo {
            profile,
            ram_gb,
            recommended_chat_model: defaults.recommended_chat_model,
            recommended_embedding_model: defaults.recommended_embedding_model,
            n_ctx_tokens: defaults.n_ctx_tokens,
            max_parallel_jobs: defaults.max_parallel_jobs,
            always_on_voice: defaults.always_on_voice,
            local_ggufs,
        }
    }

    /// Return sensible configuration defaults for any given profile tier.
    pub fn profile_defaults(profile: &ModelProfile) -> ProfileDefaults {
        match profile {
            ModelProfile::Balanced => ProfileDefaults {
                recommended_chat_model: "qwen3:1.7b".to_string(),
                recommended_embedding_model: "all-minilm:latest".to_string(),
                n_ctx_tokens: 4096,
                max_parallel_jobs: 1,
                always_on_voice: false,
            },
            ModelProfile::Standard => ProfileDefaults {
                recommended_chat_model: "llama3.3:8b".to_string(),
                recommended_embedding_model: "nomic-embed-text:latest".to_string(),
                n_ctx_tokens: 8192,
                max_parallel_jobs: 2,
                always_on_voice: false,
            },
            ModelProfile::Power => ProfileDefaults {
                recommended_chat_model: "qwen2.5:14b".to_string(),
                recommended_embedding_model: "nomic-embed-text:latest".to_string(),
                n_ctx_tokens: 16384,
                max_parallel_jobs: 2,
                always_on_voice: true,
            },
            ModelProfile::Ultra => ProfileDefaults {
                recommended_chat_model: "llama3.3:70b".to_string(),
                recommended_embedding_model: "bge-large:latest".to_string(),
                n_ctx_tokens: 65536,
                max_parallel_jobs: 4,
                always_on_voice: true,
            },
        }
    }

    /// Get default path to Deyana GGUF storage directory: `~/.deyana/models`
    pub fn get_default_models_dir() -> PathBuf {
        let base = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        base.join(".deyana").join("models")
    }

    /// Scan `~/.deyana/models/` for .gguf files
    pub fn scan_default_models_dir() -> Vec<GgufModelEntry> {
        let dir = Self::get_default_models_dir();
        Self::scan_models_dir(&dir)
    }

    /// Scan any directory for GGUF model files and calculate RAM estimates.
    pub fn scan_models_dir(dir: &Path) -> Vec<GgufModelEntry> {
        let mut entries = Vec::new();
        if !dir.exists() || !dir.is_dir() {
            return entries;
        }

        if let Ok(read_dir) = std::fs::read_dir(dir) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext.to_string_lossy().eq_ignore_ascii_case("gguf") {
                            let file_name = path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();
                            let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
                            let size_gb = size_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                            // Working rule of thumb: RAM needed is ~1.25x model size (model weight + KV cache overhead)
                            let estimated_ram_gb = (size_gb * 1.25 * 10.0).round() / 10.0;

                            let detected_profile = if estimated_ram_gb < 14.0 {
                                ModelProfile::Balanced
                            } else if estimated_ram_gb < 28.0 {
                                ModelProfile::Standard
                            } else if estimated_ram_gb < 58.0 {
                                ModelProfile::Power
                            } else {
                                ModelProfile::Ultra
                            };

                            entries.push(GgufModelEntry {
                                path: path.to_string_lossy().to_string(),
                                file_name,
                                size_bytes,
                                estimated_ram_gb,
                                detected_profile,
                            });
                        }
                    }
                }
            }
        }
        entries
    }
}

#[cfg(test)]
#[path = "tests/hardware_tests.rs"]
mod tests;

