use super::*;

#[test]
fn test_chat_message_encryption_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(INIT_SQL).unwrap();

    let key = [0x77u8; 32];
    let secret = "My secret password is: SuperSecretPassword123!";

    let saved = save_chat_message_encrypted(&conn, Some("s1"), "user", secret, None, &key).unwrap();
    assert_eq!(saved.content, secret);

    let raw_content: String = conn
        .query_row("SELECT content FROM chat_messages WHERE id = ?1", params![&saved.id], |r| r.get(0))
        .unwrap();
    assert!(raw_content.starts_with("ENC::"));
    assert!(!raw_content.contains("SuperSecretPassword"));

    let history = get_chat_history_decrypted(&conn, Some("s1"), None, &key).unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].content, secret);
}

#[test]
fn test_memory_item_encryption_roundtrip() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(INIT_SQL).unwrap();

    let key = [0x88u8; 32];
    let note = "Confidential client meeting notes regarding Project Phoenix.";

    let saved = save_memory_item_encrypted(&conn, note, "notes", Some(vec!["phoenix".to_string()]), &key).unwrap();
    assert_eq!(saved.content_markdown, note);

    let raw_content: String = conn
        .query_row("SELECT content_markdown FROM memory_items WHERE id = ?1", params![&saved.id], |r| r.get(0))
        .unwrap();
    assert!(raw_content.starts_with("ENC::"));

    let items = get_memory_items_decrypted(&conn, None, &key).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].content_markdown, note);
}

#[test]
fn test_memory_links_crud_and_cascade_delete() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(INIT_SQL).unwrap();

    let m1 = save_memory_item(&conn, "First core concept note", "notes", None).unwrap();
    let m2 = save_memory_item(&conn, "Second dependency requirement note", "notes", None).unwrap();
    let m3 = save_memory_item(&conn, "Third referencing note", "notes", None).unwrap();

    let meta = serde_json::json!({"context": "build pipeline"});
    let link1 = create_memory_link(
        &conn,
        &m1.id,
        &m2.id,
        MemoryRelationType::DependsOn,
        0.9,
        Some(meta),
    )
    .unwrap();

    assert_eq!(link1.source_id, m1.id);
    assert_eq!(link1.target_id, m2.id);
    assert_eq!(link1.relation_type, MemoryRelationType::DependsOn);
    assert!((link1.weight - 0.9).abs() < 1e-5);
    assert_eq!(link1.metadata["context"], "build pipeline");

    let link2 = create_memory_link(
        &conn,
        &m3.id,
        &m1.id,
        MemoryRelationType::RelatedTo,
        0.8,
        None,
    )
    .unwrap();

    let m1_links = get_memory_links_for_item(&conn, &m1.id).unwrap();
    assert_eq!(m1_links.len(), 2);

    let batch_links = get_links_for_memory_ids(&conn, &[m1.id.clone()]).unwrap();
    assert_eq!(batch_links.len(), 2);

    // Verify cascading deletion when m1 is deleted
    conn.execute("DELETE FROM memory_items WHERE id = ?1", params![&m1.id]).unwrap();

    let m2_links = get_memory_links_for_item(&conn, &m2.id).unwrap();
    assert_eq!(m2_links.len(), 0, "Outgoing link should be cascade deleted");

    let m3_links = get_memory_links_for_item(&conn, &m3.id).unwrap();
    assert_eq!(m3_links.len(), 0, "Incoming link should be cascade deleted");
}

#[test]
fn test_memory_links_unique_and_self_referential_constraints() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(INIT_SQL).unwrap();

    let m1 = save_memory_item(&conn, "Memory A", "notes", None).unwrap();
    let m2 = save_memory_item(&conn, "Memory B", "notes", None).unwrap();

    // 1. Self-referential links must be rejected
    let self_link_res = create_memory_link(
        &conn,
        &m1.id,
        &m1.id,
        MemoryRelationType::RelatedTo,
        0.5,
        None,
    );
    assert!(self_link_res.is_err(), "Self-referential link must fail");

    // 2. Compound unique constraint: inserting existing (source, target, relation_type) updates in place
    let first = create_memory_link(
        &conn,
        &m1.id,
        &m2.id,
        MemoryRelationType::RelatedTo,
        0.4,
        None,
    )
    .unwrap();
    assert!((first.weight - 0.4).abs() < 1e-5);

    let updated = create_memory_link(
        &conn,
        &m1.id,
        &m2.id,
        MemoryRelationType::RelatedTo,
        0.95,
        Some(serde_json::json!({"updated": true})),
    )
    .unwrap();
    assert_eq!(updated.id, first.id, "Link ID should be preserved across updates");
    assert!((updated.weight - 0.95).abs() < 1e-5);

    let links = get_memory_links_for_item(&conn, &m1.id).unwrap();
    assert_eq!(links.len(), 1, "Should not duplicate compound unique link");
    assert!((links[0].weight - 0.95).abs() < 1e-5);
    assert_eq!(links[0].metadata["updated"], true);
}

#[test]
fn test_delete_memory_link() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(INIT_SQL).unwrap();

    let m1 = save_memory_item(&conn, "Source memory", "notes", None).unwrap();
    let m2 = save_memory_item(&conn, "Target memory", "notes", None).unwrap();

    let link = create_memory_link(
        &conn,
        &m1.id,
        &m2.id,
        MemoryRelationType::DependsOn,
        0.75,
        None,
    )
    .unwrap();

    let deleted = delete_memory_link(&conn, &link.id).unwrap();
    assert!(deleted, "Existing link deletion must return true");

    let links_after = get_memory_links_for_item(&conn, &m1.id).unwrap();
    assert!(links_after.is_empty(), "Link must no longer exist");

    let delete_again = delete_memory_link(&conn, &link.id).unwrap();
    assert!(!delete_again, "Deleting non-existent link must return false");
}

#[test]
fn test_self_model_persistence_and_audit_log() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(INIT_SQL).unwrap();

    // 1. Initial seed
    let seeded = seed_self_model_if_empty(&conn).unwrap();
    assert_eq!(seeded.id, "active");
    assert_eq!(seeded.version, 1);
    assert!(!seeded.core_values.is_empty());
    assert!(!seeded.intellectual_interests.is_empty());

    // Audit log should contain initial_seed entry
    let audit1 = get_self_model_audit_log(&conn, None).unwrap();
    assert_eq!(audit1.len(), 1);
    assert_eq!(audit1[0].operation, "initial_seed");
    assert_eq!(audit1[0].version, 1);

    // 2. Update core_values field
    let new_values = serde_json::json!([
        "Absolute privacy over convenience",
        "Empathetic, rigorous technical precision",
        "Autonomous cognitive reflection"
    ]);
    let updated1 = update_self_model_field(
        &conn,
        "core_values",
        &new_values,
        "Refining operational ethics",
        "user_action",
    )
    .unwrap();

    assert_eq!(updated1.version, 2);
    assert_eq!(updated1.core_values.len(), 3);
    assert_eq!(updated1.core_values[2], "Autonomous cognitive reflection");

    // 3. Update identity_statement field
    let updated2 = update_self_model_field(
        &conn,
        "identity_statement",
        &serde_json::json!("I am Deyana — your private cognitive copilot."),
        "Identity evolution",
        "user_action",
    )
    .unwrap();

    assert_eq!(updated2.version, 3);
    assert_eq!(updated2.identity_statement, "I am Deyana — your private cognitive copilot.");

    // Audit log should have 3 entries (version 3, version 2, version 1)
    let audit_log = get_self_model_audit_log(&conn, None).unwrap();
    assert_eq!(audit_log.len(), 3);
    assert_eq!(audit_log[0].version, 3);
    assert_eq!(audit_log[0].field_changed, "identity_statement");
    assert_eq!(audit_log[0].rationale, "Identity evolution");
    assert_eq!(audit_log[1].version, 2);
    assert_eq!(audit_log[1].field_changed, "core_values");
}

#[test]
fn test_memory_save_suggestions_lifecycle() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(INIT_SQL).unwrap();

    let entities_json = r#"[{"name": "Deyana", "entityType": "project"}]"#;
    let sug1 = create_memory_save_suggestion(
        &conn,
        Some("session_1"),
        "User prefers Rust and SQLite for low-latency desktop apps",
        entities_json,
        "preference",
        3,
        "Direct architectural preference expressed in chat",
    )
    .unwrap();

    assert_eq!(sug1.status, "pending");
    assert_eq!(sug1.importance, 3);
    assert_eq!(sug1.category, "preference");
    assert_eq!(sug1.entities.len(), 1);

    // List pending suggestions
    let pending = list_pending_memory_saves(&conn).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, sug1.id);

    // Approve suggestion
    let approved_memory = approve_memory_save(
        &conn,
        &sug1.id,
        Some("Core Architecture Preference"),
        None,
        Some(vec!["preference".to_string(), "architecture".to_string()]),
    )
    .unwrap();

    assert_eq!(approved_memory.title, "Core Architecture Preference");
    assert!(approved_memory.content_markdown.contains("User prefers Rust and SQLite"));

    // Verify status updated in DB
    let pending_after_approval = list_pending_memory_saves(&conn).unwrap();
    assert!(pending_after_approval.is_empty());

    // Create a second suggestion and dismiss it
    let sug2 = create_memory_save_suggestion(
        &conn,
        Some("session_1"),
        "User might travel next week",
        "[]",
        "plan",
        3,
        "Tentative schedule mention",
    )
    .unwrap();

    let dismissed = dismiss_memory_save(&conn, &sug2.id).unwrap();
    assert!(dismissed);

    let pending_after_dismissal = list_pending_memory_saves(&conn).unwrap();
    assert!(pending_after_dismissal.is_empty());
}

#[test]
fn test_reflections_and_proactive_thoughts_persistence() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(INIT_SQL).unwrap();

    // Reflections
    let ref1 = save_reflection(
        &conn,
        Some("session_1"),
        "How do I configure Tauri IPC?",
        "Use #[tauri::command] and generate_handler!",
        0.95,
        0.90,
        0.92,
        "Could mention async commands",
        "Keep up the structured code examples",
        false,
    )
    .unwrap();
    assert_eq!(ref1.flagged_for_correction, false);

    let ref2 = save_reflection(
        &conn,
        Some("session_1"),
        "Is C++ faster than Rust here?",
        "It depends on the compiler flags",
        0.50,
        0.45,
        0.48,
        "Failed to provide benchmark references",
        "Acknowledge uncertainty and benchmark before claiming parity",
        true,
    )
    .unwrap();
    assert_eq!(ref2.flagged_for_correction, true);

    let reflections = get_recent_reflections(&conn, Some(10)).unwrap();
    assert_eq!(reflections.len(), 2);
    assert_eq!(reflections[0].id, ref2.id); // Ordered by created_at DESC

    // Proactive thoughts
    let thg = save_proactive_thought(
        &conn,
        "interval_45min",
        "Discovered strong link between memory indexing and semantic caching",
        &["Cache invalidation prevents stale context".to_string()],
        &["mem_1".to_string(), "mem_2".to_string()],
        None,
    )
    .unwrap();

    assert_eq!(thg.trigger_reason, "interval_45min");
    assert_eq!(thg.hypotheses.len(), 1);
    assert_eq!(thg.source_memory_ids.len(), 2);

    let thoughts = get_recent_proactive_thoughts(&conn, Some(10)).unwrap();
    assert_eq!(thoughts.len(), 1);
    assert_eq!(thoughts[0].id, thg.id);
}

