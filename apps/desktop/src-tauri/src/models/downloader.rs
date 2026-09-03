use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::Duration;


use futures_util::StreamExt;
use serde::Deserialize;
use tauri::{AppHandle, Emitter};

use super::hardware::HardwareProfileDetector;
use super::registry::slugify;
use super::types::{DownloadProgressPayload, GgufRegistryEntry, HfGgufFile, ModelProfile};

#[derive(Debug, Deserialize)]
struct HfModelInfoResponse {
    siblings: Option<Vec<HfSibling>>,
}

#[derive(Debug, Deserialize)]
struct HfSibling {
    rfilename: String,
}

pub struct HfDownloader;

impl HfDownloader {
    /// Fetch list of available GGUF files in a HuggingFace repository
    pub async fn list_repo_ggufs(repo_id: &str) -> Result<Vec<HfGgufFile>, String> {
        let clean_repo = repo_id.trim();
        if clean_repo.is_empty() {
            return Err("HuggingFace repository ID cannot be empty".to_string());
        }

        let api_url = format!("https://huggingface.co/api/models/{clean_repo}");
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("Deyana-Desktop/0.1.0")
            .build()
            .map_err(|e| e.to_string())?;

        let resp = client
            .get(&api_url)
            .send()
            .await
            .map_err(|e| format!("Failed to reach HuggingFace Hub API: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!(
                "HuggingFace API error: HTTP {} for repository '{clean_repo}'",
                resp.status()
            ));
        }

        let info: HfModelInfoResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse HuggingFace model metadata: {e}"))?;

        let mut gguf_files = Vec::new();

        if let Some(siblings) = info.siblings {
            for sibling in siblings {
                let filename = sibling.rfilename;
                if filename.to_lowercase().ends_with(".gguf") {
                    let quantization = extract_quantization(&filename);
                    let download_url = format!(
                        "https://huggingface.co/{clean_repo}/resolve/main/{filename}"
                    );

                    // Estimate size/RAM from quantization tag if known
                    let estimated_ram_gb = estimate_ram_from_quant(&filename, &quantization);
                    let recommended_profile = if estimated_ram_gb < 14.0 {
                        ModelProfile::Balanced
                    } else if estimated_ram_gb < 28.0 {
                        ModelProfile::Standard
                    } else if estimated_ram_gb < 58.0 {
                        ModelProfile::Power
                    } else {
                        ModelProfile::Ultra
                    };

                    gguf_files.push(HfGgufFile {
                        filename,
                        size_bytes: (estimated_ram_gb * 1024.0 * 1024.0 * 1024.0 * 0.8) as u64,
                        quantization,
                        estimated_ram_gb,
                        recommended_profile,
                        download_url,
                    });
                }
            }
        }

        if gguf_files.is_empty() {
            return Err(format!(
                "No .gguf files found in HuggingFace repository '{clean_repo}'"
            ));
        }

        gguf_files.sort_by(|a, b| a.filename.cmp(&b.filename));
        Ok(gguf_files)
    }

    /// Stream download a GGUF file from HuggingFace directly to the models directory
    pub async fn download_gguf(
        repo_id: &str,
        filename: &str,
        dest_dir: &Path,
        app: Option<&AppHandle>,
    ) -> Result<GgufRegistryEntry, String> {
        let _ = std::fs::create_dir_all(dest_dir);
        let final_path = dest_dir.join(filename);
        let temp_path = dest_dir.join(format!("{filename}.downloading"));

        let download_url = format!(
            "https://huggingface.co/{repo_id}/resolve/main/{filename}"
        );

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3600))
            .user_agent("Deyana-Desktop/0.1.0")
            .build()
            .map_err(|e| e.to_string())?;

        let response = client
            .get(&download_url)
            .send()
            .await
            .map_err(|e| format!("Download request failed: {e}"))?;

        if !response.status().is_success() {
            return Err(format!(
                "Failed to download file from HuggingFace: HTTP {}",
                response.status()
            ));
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut file = File::create(&temp_path)
            .map_err(|e| format!("Failed to create temporary file: {e}"))?;

        let mut stream = response.bytes_stream();
        let mut downloaded_bytes: u64 = 0;
        let mut last_progress_emit = std::time::Instant::now();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result
                .map_err(|e| format!("Error while downloading stream chunk: {e}"))?;

            file.write_all(&chunk)
                .map_err(|e| format!("Failed to write chunk to disk: {e}"))?;

            downloaded_bytes += chunk.len() as u64;

            if let Some(app_handle) = app {
                if last_progress_emit.elapsed() >= Duration::from_millis(250) {
                    last_progress_emit = std::time::Instant::now();
                    let percent = if total_size > 0 {
                        (downloaded_bytes as f32 / total_size as f32) * 100.0
                    } else {
                        0.0
                    };

                    let payload = DownloadProgressPayload {
                        repo_id: repo_id.to_string(),
                        filename: filename.to_string(),
                        bytes_downloaded: downloaded_bytes,
                        total_bytes: total_size,
                        percent,
                    };
                    let _ = app_handle.emit("models:download_progress", &payload);
                }
            }
        }

        file.flush().map_err(|e| format!("Failed to flush file: {e}"))?;
        drop(file);

        std::fs::rename(&temp_path, &final_path)
            .map_err(|e| format!("Failed to finalize downloaded file: {e}"))?;


        let final_size = std::fs::metadata(&final_path)
            .map(|m| m.len())
            .unwrap_or(downloaded_bytes);

        let detected = HardwareProfileDetector::scan_models_dir(dest_dir);
        let matched = detected.into_iter().find(|m| m.file_name == filename);

        let profile_tier = matched
            .map(|m| m.detected_profile)
            .unwrap_or(ModelProfile::Balanced);

        let id = slugify(filename);
        let entry = GgufRegistryEntry {
            id,
            display_name: filename.to_string(),
            file_path: final_path.to_string_lossy().to_string(),
            size_bytes: final_size,
            profile_tier,
            n_ctx: 4096,
            chat_template: None,
            source: format!("huggingface:{repo_id}"),
            registered_at: chrono::Utc::now().to_rfc3339(),
        };

        Ok(entry)
    }
}

pub fn extract_quantization(filename: &str) -> String {
    let upper = filename.to_uppercase();
    let quants = [
        "Q4_K_M", "Q4_K_S", "Q4_0", "Q4_1",
        "Q5_K_M", "Q5_K_S", "Q5_0", "Q5_1",
        "Q8_0", "Q6_K", "Q3_K_L", "Q3_K_M", "Q3_K_S",
        "Q2_K", "BF16", "F16", "FP16", "IQ4_XS", "IQ3_XXS",
    ];

    for q in quants {
        if upper.contains(q) {
            return q.to_string();
        }
    }

    "Q4_K_M".to_string()
}

pub fn estimate_ram_from_quant(filename: &str, quant: &str) -> f32 {
    let lower = filename.to_lowercase();
    // Rough parameter size heuristic from model name
    let param_b: f32 = if lower.contains("70b") || lower.contains("72b") {
        70.0
    } else if lower.contains("32b") || lower.contains("34b") {
        32.0
    } else if lower.contains("14b") || lower.contains("13b") {
        14.0
    } else if lower.contains("7b") || lower.contains("8b") {
        8.0
    } else if lower.contains("3b") {
        3.0
    } else if lower.contains("1.5b") || lower.contains("1.7b") || lower.contains("2b") {
        2.0
    } else {
        7.0
    };

    let bits_per_weight: f32 = match quant {
        "Q2_K" => 2.5,
        "Q3_K_S" | "Q3_K_M" | "Q3_K_L" => 3.5,
        "Q4_0" | "Q4_1" | "Q4_K_S" | "Q4_K_M" => 4.5,
        "Q5_0" | "Q5_1" | "Q5_K_S" | "Q5_K_M" => 5.5,
        "Q6_K" => 6.5,
        "Q8_0" => 8.5,
        "F16" | "FP16" | "BF16" => 16.0,
        _ => 4.5,
    };

    let model_gb: f32 = (param_b * bits_per_weight) / 8.0;
    (model_gb * 1.25 * 10.0).round() / 10.0
}


#[cfg(test)]
#[path = "tests/downloader_tests.rs"]
mod tests;

