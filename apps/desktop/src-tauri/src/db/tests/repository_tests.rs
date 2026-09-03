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
