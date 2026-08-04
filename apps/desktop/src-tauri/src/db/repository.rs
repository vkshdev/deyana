use std::path::Path;
use chrono::Utc;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection};
use uuid::Uuid;

use super::models::{ChatMessageItem, MemoryEntity, MemoryInsight, MemoryItem, TriageMessageResponse};
use super::schema::INIT_SQL;

pub type DbPool = Pool<SqliteConnectionManager>;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Database pool error: {0}")]
    Pool(#[from] r2d2::Error),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn init_db(db_path: &Path) -> Result<DbPool, DbError> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let manager = SqliteConnectionManager::file(db_path).with_init(|conn| {
        conn.execute_batch(
            "PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 5000;",
        )
    });
    let pool = Pool::new(manager)?;

    let conn = pool.get()?;
    conn.execute_batch(INIT_SQL)?;

    Ok(pool)
}

pub fn check_health(conn: &Connection) -> Result<bool, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT 1")?;
    let val: i32 = stmt.query_row([], |row| row.get(0))?;
    Ok(val == 1)
}

pub fn get_memory_items(
    conn: &Connection,
    limit: Option<usize>,
) -> Result<Vec<MemoryItem>, rusqlite::Error> {
    let max_limit = limit.unwrap_or(100) as i64;

    let mut stmt = conn.prepare(
        "SELECT id, type, title, summary, content_markdown, markdown_path, source_type, source_id, source_uri, importance, created_at, updated_at, deleted_at
         FROM memory_items
         WHERE deleted_at IS NULL
         ORDER BY created_at DESC
         LIMIT ?1"
    )?;

    let memory_rows = stmt.query_map(params![max_limit], |row| {
        let id: String = row.get(0)?;
        let memory_type: String = row.get(1)?;
        let title: String = row.get(2)?;
        let summary: String = row.get(3)?;
        let content_markdown: String = row.get(4)?;
        let markdown_path: Option<String> = row.get(5)?;
        let source_type: String = row.get(6)?;
        let source_id: Option<String> = row.get(7)?;
        let source_uri: Option<String> = row.get(8)?;
        let importance: i32 = row.get(9)?;
        let created_at: String = row.get(10)?;
        let updated_at: String = row.get(11)?;
        let deleted_at: Option<String> = row.get(12)?;

        Ok((
            id,
            memory_type,
            title,
            summary,
            content_markdown,
            markdown_path,
            source_type,
            source_id,
            source_uri,
            importance,
            created_at,
            updated_at,
            deleted_at,
        ))
    })?;

    let mut items = Vec::new();

    for row_res in memory_rows {
        let (
            id,
            memory_type,
            title,
            summary,
            content_markdown,
            markdown_path,
            source_type,
            source_id,
            source_uri,
            importance,
            created_at,
            updated_at,
            deleted_at,
        ) = row_res?;

        // Query tags for this item
        let mut tag_stmt = conn.prepare("SELECT tag FROM memory_tags WHERE memory_id = ?1")?;
        let tag_rows = tag_stmt.query_map(params![&id], |r| r.get::<_, String>(0))?;
        let mut tags = Vec::new();
        for t in tag_rows {
            tags.push(t?);
        }

        // Query entities for this item
        let mut entity_stmt = conn.prepare(
            "SELECT id, memory_id, name, entity_type, source_text, created_at FROM memory_entities WHERE memory_id = ?1"
        )?;
        let entity_rows = entity_stmt.query_map(params![&id], |r| {
            Ok(MemoryEntity {
                id: r.get(0)?,
                memory_id: r.get(1)?,
                memory_title: None,
                source_type: None,
                source_id: None,
                source_uri: None,
                name: r.get(2)?,
                entity_type: r.get(3)?,
                source_text: r.get(4)?,
                created_at: r.get(5)?,
            })
        })?;
        let mut entities = Vec::new();
        for e in entity_rows {
            entities.push(e?);
        }

        // Query insights for this item
        let mut insight_stmt = conn.prepare(
            "SELECT id, memory_id, type, title, detail, status, due_at, created_at FROM memory_insights WHERE memory_id = ?1"
        )?;
        let insight_rows = insight_stmt.query_map(params![&id], |r| {
            Ok(MemoryInsight {
                id: r.get(0)?,
                memory_id: r.get(1)?,
                memory_title: None,
                source_type: None,
                source_id: None,
                source_uri: None,
                r#type: r.get(2)?,
                title: r.get(3)?,
                detail: r.get(4)?,
                status: r.get(5)?,
                due_at: r.get(6)?,
                created_at: r.get(7)?,
            })
        })?;

        let mut action_items = Vec::new();
        let mut decisions = Vec::new();

        for ins in insight_rows {
            let insight = ins?;
            if insight.r#type == "action_item" {
                action_items.push(insight);
            } else if insight.r#type == "decision" {
                decisions.push(insight);
            }
        }

        items.push(MemoryItem {
            id,
            r#type: memory_type,
            title,
            summary,
            content_markdown,
            markdown_path,
            source_type,
            source_id,
            source_uri,
            importance,
            tags,
            entities,
            action_items,
            decisions,
            created_at,
            updated_at,
            deleted_at,
        });
    }

    Ok(items)
}

pub fn save_memory_item(
    conn: &Connection,
    content: &str,
    source: &str,
    tags: Option<Vec<String>>,
) -> Result<MemoryItem, rusqlite::Error> {
    let id = format!("memory_{}", Uuid::new_v4().simple());
    let now = Utc::now().to_rfc3339();

    // Derive simple title & summary from content safely
    let first_line = content.lines().next().unwrap_or(content);
    let title = if first_line.chars().count() > 60 {
        format!("{}...", first_line.chars().take(60).collect::<String>())
    } else if first_line.is_empty() {
        "Untitled Memory".to_string()
    } else {
        first_line.to_string()
    };

    let summary = if content.chars().count() > 200 {
        format!("{}...", content.chars().take(200).collect::<String>())
    } else {
        content.to_string()
    };

    let memory_type = "note".to_string();
    let importance = 3;

    conn.execute(
        "INSERT INTO memory_items (id, type, title, summary, content_markdown, source_type, importance, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            &id,
            &memory_type,
            &title,
            &summary,
            content,
            source,
            importance,
            &now,
            &now
        ],
    )?;

    let mut saved_tags = Vec::new();
    if let Some(tag_list) = tags {
        let mut tag_stmt = conn.prepare(
            "INSERT OR IGNORE INTO memory_tags (memory_id, tag) VALUES (?1, ?2)"
        )?;
        for tag in tag_list {
            tag_stmt.execute(params![&id, &tag])?;
            saved_tags.push(tag);
        }
    }

    Ok(MemoryItem {
        id,
        r#type: memory_type,
        title,
        summary,
        content_markdown: content.to_string(),
        markdown_path: None,
        source_type: source.to_string(),
        source_id: None,
        source_uri: None,
        importance,
        tags: saved_tags,
        entities: Vec::new(),
        action_items: Vec::new(),
        decisions: Vec::new(),
        created_at: now.clone(),
        updated_at: now,
        deleted_at: None,
    })
}

pub fn get_triage_inbox(
    conn: &Connection,
) -> Result<Vec<TriageMessageResponse>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, platform, sender, content, urgency_score, auto_draft, status, created_at
         FROM triage_inbox
         WHERE status = 'pending'
         ORDER BY created_at DESC"
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(TriageMessageResponse {
            id: row.get(0)?,
            platform: row.get(1)?,
            sender: row.get(2)?,
            content: row.get(3)?,
            urgency_score: row.get(4)?,
            auto_draft: row.get(5)?,
            status: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?;

    let mut items = Vec::new();
    for r in rows {
        items.push(r?);
    }
    Ok(items)
}

pub fn resolve_triage_item(
    conn: &Connection,
    id: &str,
    resolution: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE triage_inbox SET status = ?1 WHERE id = ?2",
        params![resolution, id],
    )?;
    Ok(())
}

pub fn save_chat_message(
    conn: &Connection,
    session_id: Option<&str>,
    role: &str,
    content: &str,
    model: Option<&str>,
) -> Result<ChatMessageItem, rusqlite::Error> {
    let id = format!("chat_{}", Uuid::new_v4().simple());
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO chat_messages (id, session_id, role, content, model, source_context_json, web_source_context_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, '[]', '[]', ?6)",
        params![
            &id,
            session_id,
            role,
            content,
            model,
            &now
        ],
    )?;

    Ok(ChatMessageItem {
        id,
        role: role.to_string(),
        content: content.to_string(),
        model: model.map(|s| s.to_string()),
        source_references: Vec::new(),
        web_source_references: Vec::new(),
        created_at: now,
        session_id: session_id.map(|s| s.to_string()),
    })
}

pub fn get_chat_history(
    conn: &Connection,
    session_id: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<ChatMessageItem>, rusqlite::Error> {
    let max_limit = limit.unwrap_or(100) as i64;
    let mut items = Vec::new();

    if let Some(sid) = session_id {
        let mut stmt = conn.prepare(
            "SELECT id, role, content, model, source_context_json, web_source_context_json, created_at, session_id
             FROM chat_messages
             WHERE session_id = ?1
             ORDER BY created_at ASC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![sid, max_limit], |row| {
            let id: String = row.get(0)?;
            let role: String = row.get(1)?;
            let content: String = row.get(2)?;
            let model: Option<String> = row.get(3)?;
            let source_json: Option<String> = row.get(4)?;
            let web_json: Option<String> = row.get(5)?;
            let created_at: String = row.get(6)?;
            let session_id: Option<String> = row.get(7)?;

            let source_references = source_json
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();
            let web_source_references = web_json
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();

            Ok(ChatMessageItem {
                id,
                role,
                content,
                model,
                source_references,
                web_source_references,
                created_at,
                session_id,
            })
        })?;

        for r in rows {
            items.push(r?);
        }
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, role, content, model, source_context_json, web_source_context_json, created_at, session_id
             FROM chat_messages
             ORDER BY created_at ASC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![max_limit], |row| {
            let id: String = row.get(0)?;
            let role: String = row.get(1)?;
            let content: String = row.get(2)?;
            let model: Option<String> = row.get(3)?;
            let source_json: Option<String> = row.get(4)?;
            let web_json: Option<String> = row.get(5)?;
            let created_at: String = row.get(6)?;
            let session_id: Option<String> = row.get(7)?;

            let source_references = source_json
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();
            let web_source_references = web_json
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();

            Ok(ChatMessageItem {
                id,
                role,
                content,
                model,
                source_references,
                web_source_references,
                created_at,
                session_id,
            })
        })?;

        for r in rows {
            items.push(r?);
        }
    }

    Ok(items)
}

