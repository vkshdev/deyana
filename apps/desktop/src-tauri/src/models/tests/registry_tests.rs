use super::*;

#[test]
fn test_slugify() {
    assert_eq!(slugify("Qwen2.5-7B-Instruct.Q4_K_M.gguf"), "qwen2-5-7b-instruct-q4-k-m-gguf");
    assert_eq!(slugify("Llama-3.3-70B.gguf"), "llama-3-3-70b-gguf");
}

#[test]
fn test_registry_in_memory() {
    let temp_dir = std::env::temp_dir().join("deyana_test_registry");
    let mut registry = GgufRegistry {
        entries: HashMap::new(),
        registry_file: temp_dir.join(REGISTRY_FILENAME),
    };

    let entry = GgufRegistryEntry {
        id: "test-model".to_string(),
        display_name: "Test Model".to_string(),
        file_path: "d:/models/test.gguf".to_string(),
        size_bytes: 1024 * 1024 * 1024,
        profile_tier: ModelProfile::Balanced,
        n_ctx: 4096,
        chat_template: Some("chatml".to_string()),
        source: "test".to_string(),
        registered_at: "2026-09-02T00:00:00Z".to_string(),
    };

    let registered = registry.register(entry.clone()).unwrap();
    assert_eq!(registered.id, "test-model");
    assert_eq!(registry.list().len(), 1);
    assert_eq!(registry.get("test-model").unwrap().display_name, "Test Model");

    registry.unregister("test-model", false).unwrap();
    assert_eq!(registry.list().len(), 0);
}
