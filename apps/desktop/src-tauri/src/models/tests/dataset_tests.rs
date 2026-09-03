use super::*;
use crate::db::schema::INIT_SQL;

#[test]
fn test_pii_masking() {
    let text_with_key = "My API key is sk-1234567890abcdef1234567890 and email is user@example.com.";
    let masked = mask_pii(text_with_key);
    assert!(!masked.contains("sk-1234567890"));
    assert!(masked.contains("[API_KEY]"));
    assert!(!masked.contains("user@example.com"));
    assert!(masked.contains("[EMAIL]"));
}

#[test]
fn test_dataset_logging_and_sharegpt_export() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(INIT_SQL).unwrap();

    let id = log_interaction(
        &conn,
        "What is Rust?",
        "Memory context about Rust",
        "Rust is a fast and memory-safe language.",
        "qwen3:1.7b",
        Some("VS Code"),
        Some("session-1"),
    )
    .unwrap();

    assert!(!id.is_empty());

    let stats = get_dataset_stats(&conn).unwrap();
    assert_eq!(stats.total_samples, 1);
    assert_eq!(stats.models_breakdown.get("qwen3:1.7b"), Some(&1));

    record_feedback(&conn, &id, 1, Some("Rust is a systems programming language focused on safety.")).unwrap();
    let stats_after = get_dataset_stats(&conn).unwrap();
    assert_eq!(stats_after.thumbs_up_count, 1);
    assert_eq!(stats_after.edited_samples_count, 1);

    let temp_file = std::env::temp_dir().join("test_sharegpt.jsonl");
    let exported = export_sharegpt(&conn, &temp_file).unwrap();
    assert_eq!(exported, 1);

    let content = std::fs::read_to_string(&temp_file).unwrap();
    assert!(content.contains("conversations"));
    assert!(content.contains("human"));
    assert!(content.contains("gpt"));
    assert!(content.contains("systems programming language"));
}
