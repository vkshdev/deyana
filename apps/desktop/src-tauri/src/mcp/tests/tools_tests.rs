use super::*;

fn create_test_dir() -> PathBuf {
    let path = std::env::temp_dir().join(format!("deyana_test_mcp_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn test_resolve_vault_path_valid() {
    let root = Path::new("C:\\vault");
    let path = resolve_vault_path(root, "notes/idea.md").unwrap();
    assert_eq!(path, Path::new("C:\\vault\\notes/idea.md"));
}

#[test]
fn test_resolve_vault_path_traversal_blocked() {
    let root = Path::new("C:\\vault");

    let traversal_payloads = vec![
        "../secret.txt",
        "../../secret.txt",
        "folder/../../secret.txt",
        "folder/../secret.txt",
        "..\\secret.txt",
        "..\\..\\secret.txt",
        "folder\\..\\secret.txt",
        "C:\\Windows\\System32\\drivers\\etc\\hosts",
        "/etc/passwd",
        "\\\\server\\share\\file.txt",
        "C:relative.txt",
    ];

    for payload in traversal_payloads {
        let res = resolve_vault_path(root, payload);
        assert!(
            res.is_err(),
            "Expected path traversal payload '{}' to be rejected",
            payload
        );
    }
}

#[test]
fn test_execute_read_vault_note_traversal_blocked() {
    let app_dir = create_test_dir();

    let args = serde_json::json!({
        "path": "../../../Windows/System32/drivers/etc/hosts"
    });

    let res = execute_read_vault_note(&app_dir, &args);
    let _ = fs::remove_dir_all(&app_dir);
    assert!(res.is_err());
    assert!(res.as_ref().unwrap_err().contains("strictly prohibited") || res.as_ref().unwrap_err().contains("not allowed"));
}

#[test]
fn test_execute_create_vault_note_traversal_blocked() {
    let app_dir = create_test_dir();

    let db_dir = app_dir.join("db");
    fs::create_dir_all(&db_dir).unwrap();
    let db_path = db_dir.join("test.db");
    let pool = crate::db::init_db(&db_path).unwrap();

    let args_folder_traversal = serde_json::json!({
        "title": "valid_note",
        "content": "some content",
        "folder": "../../../tmp"
    });
    let res1 = execute_create_vault_note(&app_dir, &pool, &args_folder_traversal);
    assert!(res1.is_err());

    let args_title_traversal = serde_json::json!({
        "title": "../../hacked_note",
        "content": "some content"
    });
    let res2 = execute_create_vault_note(&app_dir, &pool, &args_title_traversal);
    assert!(res2.is_err());

    let _ = fs::remove_dir_all(&app_dir);
}
