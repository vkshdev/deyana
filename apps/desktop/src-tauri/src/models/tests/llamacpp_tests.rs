use super::*;

#[test]
fn test_llamacpp_engine_defaults() {
    let engine = LlamaCppEngine::default();
    assert_eq!(engine.port(), 8080);
    assert_eq!(engine.endpoint(), "http://127.0.0.1:8080");
    assert_eq!(engine.active_n_ctx(), 4096);
    let status = engine.get_status();
    assert!(!status.running);
    assert_eq!(status.pid, None);
}

#[test]
fn test_llamacpp_status_serialization() {
    let engine = LlamaCppEngine::new(Some("models/test.gguf".to_string()));
    let status = engine.get_status();
    let json = serde_json::to_string(&status).expect("Failed to serialize status");
    assert!(json.contains("models/test.gguf"));
    assert!(json.contains("8080"));
}
