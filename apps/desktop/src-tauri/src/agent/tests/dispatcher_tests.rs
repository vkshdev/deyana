use super::*;
use crate::db::schema::INIT_SQL;

#[test]
fn test_dispatch_get_system_status() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(INIT_SQL).unwrap();

    let res = ToolDispatcher::dispatch(&conn, "get_system_status", &serde_json::json!({})).unwrap();
    assert!(res.contains("total_ram_gb"));
    assert!(res.contains("active_profile"));
}

#[test]
fn test_dispatch_save_and_search_memory() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(INIT_SQL).unwrap();

    let save_args = serde_json::json!({
        "content": "Deploy Kubernetes clusters with Terraform and Helm",
        "tags": ["devops", "k8s"]
    });
    let save_res = ToolDispatcher::dispatch(&conn, "save_note", &save_args).unwrap();
    assert!(save_res.contains("Successfully saved note"));

    let search_args = serde_json::json!({
        "query": "Kubernetes",
        "limit": 5
    });
    let search_res = ToolDispatcher::dispatch(&conn, "search_memory", &search_args).unwrap();
    assert!(!search_res.is_empty());
}

#[test]
fn test_dispatch_unknown_tool() {
    let conn = Connection::open_in_memory().unwrap();
    let res = ToolDispatcher::dispatch(&conn, "non_existent_tool", &serde_json::json!({}));
    assert!(res.is_err());
}
