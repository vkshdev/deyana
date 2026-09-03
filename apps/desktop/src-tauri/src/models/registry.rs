use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use super::hardware::HardwareProfileDetector;
use super::types::{GgufRegistryEntry, ModelProfile};

pub const REGISTRY_FILENAME: &str = "registry.json";

#[derive(Debug, Clone)]
pub struct GgufRegistry {
    entries: HashMap<String, GgufRegistryEntry>,
    registry_file: PathBuf,
}

impl Default for GgufRegistry {
    fn default() -> Self {
        let models_dir = HardwareProfileDetector::get_default_models_dir();
        Self::load_or_create(&models_dir)
    }
}

impl GgufRegistry {
    pub fn load_or_create(models_dir: &Path) -> Self {
        let _ = fs::create_dir_all(models_dir);
        let registry_file = models_dir.join(REGISTRY_FILENAME);

        let mut entries = HashMap::new();

        if registry_file.exists() {
            if let Ok(content) = fs::read_to_string(&registry_file) {
                if let Ok(saved_entries) = serde_json::from_str::<Vec<GgufRegistryEntry>>(&content) {
                    for entry in saved_entries {
                        entries.insert(entry.id.clone(), entry);
                    }
                }
            }
        }

        // Auto-scan models dir and register any unregistered .gguf files
        let scanned = HardwareProfileDetector::scan_models_dir(models_dir);
        for gguf in scanned {
            let id = slugify(&gguf.file_name);
            if !entries.contains_key(&id) {
                let entry = GgufRegistryEntry {
                    id: id.clone(),
                    display_name: gguf.file_name.clone(),
                    file_path: gguf.path,
                    size_bytes: gguf.size_bytes,
                    profile_tier: gguf.detected_profile,
                    n_ctx: 4096,
                    chat_template: None,
                    source: "local_scan".to_string(),
                    registered_at: chrono::Utc::now().to_rfc3339(),
                };
                entries.insert(id, entry);
            }
        }

        Self {
            entries,
            registry_file,
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let list: Vec<&GgufRegistryEntry> = self.entries.values().collect();
        let json = serde_json::to_string_pretty(&list).map_err(|e| e.to_string())?;
        fs::write(&self.registry_file, json).map_err(|e| e.to_string())
    }

    pub fn register(&mut self, entry: GgufRegistryEntry) -> Result<GgufRegistryEntry, String> {
        let id = entry.id.clone();
        self.entries.insert(id.clone(), entry.clone());
        let _ = self.save();
        Ok(entry)
    }

    pub fn unregister(&mut self, id: &str, delete_file: bool) -> Result<(), String> {
        if let Some(removed) = self.entries.remove(id) {
            if delete_file {
                let path = Path::new(&removed.file_path);
                if path.exists() {
                    let _ = fs::remove_file(path);
                }
            }
            let _ = self.save();
            Ok(())
        } else {
            Err(format!("Model with ID '{id}' not found in registry"))
        }
    }

    pub fn list(&self) -> Vec<GgufRegistryEntry> {
        let mut list: Vec<GgufRegistryEntry> = self.entries.values().cloned().collect();
        list.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        list
    }

    pub fn get(&self, id: &str) -> Option<GgufRegistryEntry> {
        self.entries.get(id).cloned()
    }
}

#[derive(Clone)]
pub struct GgufRegistryState {
    pub inner: Arc<RwLock<GgufRegistry>>,
}

impl Default for GgufRegistryState {
    fn default() -> Self {
        Self {
            inner: Arc::new(RwLock::new(GgufRegistry::default())),
        }
    }
}

pub fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
#[path = "tests/registry_tests.rs"]
mod tests;

