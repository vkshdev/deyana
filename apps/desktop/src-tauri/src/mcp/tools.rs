use std::fs;
use std::path::{Path, PathBuf};
use serde_json::Value;

use crate::db::DbPool;
use crate::agent::retrieval::retrieve_memory_context;
use super::types::McpToolCallResult;

pub fn execute_search_deyana_memory(
    db_pool: &DbPool,
    args: &Value,
) -> Result<McpToolCallResult, String> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter 'query'".to_string())?;

    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(5) as usize;

    let conn = db_pool
        .get()
        .map_err(|e| format!("Failed to acquire database connection: {}", e))?;

    let items = retrieve_memory_context(&conn, None, query, limit)
        .map_err(|e| format!("Memory search query failed: {}", e))?;

    if items.is_empty() {
        return Ok(McpToolCallResult::text(format!(
            "No memory items found matching query: '{}'",
            query
        )));
    }

    let mut markdown = format!("# Search Results for '{}'\n\n", query);
    for (idx, item) in items.iter().enumerate() {
        markdown.push_str(&format!(
            "### {}. {}\n- **Source**: {}\n- **Summary**: {}\n- **Content**:\n{}\n\n",
            idx + 1,
            item.title,
            item.source_type,
            item.summary,
            item.content_markdown
        ));
    }

    Ok(McpToolCallResult::text(markdown))
}

pub fn execute_read_vault_note(
    app_dir: &Path,
    args: &Value,
) -> Result<McpToolCallResult, String> {
    let note_path_str = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter 'path'".to_string())?;

    let vault_root = app_dir.join("vault");
    if !vault_root.exists() {
        fs::create_dir_all(&vault_root)
            .map_err(|e| format!("Failed to create vault directory: {}", e))?;
    }

    let target_path = resolve_vault_path(&vault_root, note_path_str)?;

    if !target_path.exists() || !target_path.is_file() {
        return Ok(McpToolCallResult::error(format!(
            "Vault note not found: '{}'",
            note_path_str
        )));
    }

    let content = fs::read_to_string(&target_path)
        .map_err(|e| format!("Failed to read vault note: {}", e))?;

    Ok(McpToolCallResult::text(content))
}

pub fn execute_create_vault_note(
    app_dir: &Path,
    db_pool: &DbPool,
    args: &Value,
) -> Result<McpToolCallResult, String> {
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter 'title'".to_string())?;

    let content = args
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required parameter 'content'".to_string())?;

    let folder = args.get("folder").and_then(|v| v.as_str()).unwrap_or("");
    let tags: Option<Vec<String>> = args.get("tags").and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|t| t.as_str().map(|s| s.to_string()))
            .collect()
    });

    let vault_root = app_dir.join("vault");
    let target_dir = if folder.trim().is_empty() {
        vault_root.clone()
    } else {
        resolve_vault_path(&vault_root, folder)?
    };

    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Failed to create folder in vault: {}", e))?;

    let filename = if title.to_lowercase().ends_with(".md") {
        title.to_string()
    } else {
        format!("{}.md", title.trim())
    };

    let target_file = resolve_vault_path(&target_dir, &filename)?;

    fs::write(&target_file, content)
        .map_err(|e| format!("Failed to write vault note file: {}", e))?;

    // Index created note in Deyana memory database
    if let Ok(conn) = db_pool.get() {
        let memory_text = format!("# {}\n\n{}", title, content);
        let _ = crate::db::save_memory_item(&conn, &memory_text, "vault", tags);
    }

    Ok(McpToolCallResult::text(format!(
        "Successfully created vault note at '{}/{}'",
        folder, filename
    )))
}

fn resolve_vault_path(vault_root: &Path, relative: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative);

    // Prevent path traversal attacks
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                return Err("Path traversal (..) is strictly prohibited".to_string());
            }
            std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                return Err("Absolute paths are not allowed for vault notes".to_string());
            }
            _ => {}
        }
    }

    Ok(vault_root.join(relative))
}

#[cfg(test)]
mod tests {
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

        // 1. Traversal in folder
        let args_folder_traversal = serde_json::json!({
            "title": "valid_note",
            "content": "some content",
            "folder": "../../../tmp"
        });
        let res1 = execute_create_vault_note(&app_dir, &pool, &args_folder_traversal);
        assert!(res1.is_err());

        // 2. Traversal in title
        let args_title_traversal = serde_json::json!({
            "title": "../../hacked_note",
            "content": "some content"
        });
        let res2 = execute_create_vault_note(&app_dir, &pool, &args_title_traversal);
        assert!(res2.is_err());

        let _ = fs::remove_dir_all(&app_dir);
    }
}



