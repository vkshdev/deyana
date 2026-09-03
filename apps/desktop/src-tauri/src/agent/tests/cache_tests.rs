use super::*;
use rusqlite::Connection;
use crate::db::schema::INIT_SQL;

#[test]
fn test_cache_lookup_hit_and_miss() {
    let cache = SemanticCache::new(10, 30, 0.92);
    let emb1 = vec![1.0, 0.0, 0.0];
    let emb_similar = vec![0.98, 0.1, 0.0];
    let emb_different = vec![0.0, 1.0, 0.0];

    let context_hash = "test_hash_1".to_string();

    cache.insert(emb1, context_hash.clone(), "Cached answer to prompt 1".to_string(), None);

    let hit = cache.lookup(&emb_similar, &context_hash);
    assert_eq!(hit, Some("Cached answer to prompt 1".to_string()));

    let miss_emb = cache.lookup(&emb_different, &context_hash);
    assert_eq!(miss_emb, None);

    let miss_ctx = cache.lookup(&emb_similar, "different_hash");
    assert_eq!(miss_ctx, None);
}

#[test]
fn test_context_hash_stability_and_changes() {
    let item1 = MemoryItem {
        id: "mem_1".to_string(),
        r#type: "note".to_string(),
        title: "Project Notes".to_string(),
        summary: "summary".to_string(),
        content_markdown: "content".to_string(),
        markdown_path: None,
        source_type: "user".to_string(),
        source_id: None,
        source_uri: None,
        importance: 3,
        tags: vec![],
        entities: vec![],
        action_items: vec![],
        decisions: vec![],
        created_at: "2026-08-21T10:00:00Z".to_string(),
        updated_at: "2026-08-21T10:00:00Z".to_string(),
        deleted_at: None,
    };

    let hash1 = SemanticCache::compute_context_hash(&[item1.clone()]);
    let hash2 = SemanticCache::compute_context_hash(&[item1.clone()]);
    assert_eq!(hash1, hash2);

    let mut item1_updated = item1.clone();
    item1_updated.updated_at = "2026-08-21T10:05:00Z".to_string();
    let hash3 = SemanticCache::compute_context_hash(&[item1_updated]);
    assert_ne!(hash1, hash3);

    let empty_hash = SemanticCache::compute_context_hash(&[]);
    assert_eq!(empty_hash, "empty_context");
}

#[test]
fn test_sqlite_hydration_and_persistence() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(INIT_SQL).unwrap();

    let cache = SemanticCache::new(10, 30, 0.92);
    let emb = vec![0.5, 0.5, 0.5];
    let ctx_hash = "ctx_abc".to_string();
    let resp = "Persisted cached reply".to_string();

    cache.insert(emb.clone(), ctx_hash.clone(), resp.clone(), Some(&conn));

    let hydrated = SemanticCache::load_from_db(&conn);
    let hit = hydrated.lookup(&emb, &ctx_hash);
    assert_eq!(hit, Some(resp));
}
