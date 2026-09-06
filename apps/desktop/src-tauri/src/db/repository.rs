use std::path::Path;
use chrono::Utc;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use super::models::{
    ChatMessageItem, MemoryEntity, MemoryInsight, MemoryItem, MemoryLink,
    MemoryLinkWithNeighbor, MemoryRelationType, MemorySaveSuggestion, MindCognitionState,
    ProactiveThoughtRecord, ReflectionRecord, SelfModel, SelfModelAuditEntry, SemanticCacheRow,
    TriageMessageResponse,
};
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

    let tags_str = saved_tags.join(" ");
    let _ = conn.execute(
        "INSERT OR REPLACE INTO memory_fts (memory_id, title, summary, content_markdown, tags) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![&id, &title, &summary, content, &tags_str],
    );

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

pub fn create_memory_link(
    conn: &Connection,
    source_id: &str,
    target_id: &str,
    relation_type: MemoryRelationType,
    weight: f32,
    metadata: Option<serde_json::Value>,
) -> Result<MemoryLink, rusqlite::Error> {
    if source_id == target_id {
        return Err(rusqlite::Error::InvalidParameterName(
            "Self-referential memory links are prohibited".to_string(),
        ));
    }
    let now = Utc::now().to_rfc3339();
    let meta_str = metadata
        .map(|m| serde_json::to_string(&m).unwrap_or_else(|_| "{}".to_string()))
        .unwrap_or_else(|| "{}".to_string());
    let clamped_weight = weight.clamp(0.0, 1.0);

    // Check if an edge with the same (source_id, target_id, relation_type) already exists
    let existing: Option<(String, String)> = conn
        .query_row(
            "SELECT id, created_at FROM memory_links WHERE source_id = ?1 AND target_id = ?2 AND relation_type = ?3",
            params![source_id, target_id, relation_type.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    let (id, created_at) = if let Some((existing_id, existing_created_at)) = existing {
        conn.execute(
            "UPDATE memory_links SET weight = ?1, metadata_json = ?2, updated_at = ?3 WHERE id = ?4",
            params![clamped_weight, meta_str, &now, &existing_id],
        )?;
        (existing_id, existing_created_at)
    } else {
        let new_id = format!("link_{}", Uuid::new_v4().simple());
        conn.execute(
            "INSERT INTO memory_links (id, source_id, target_id, relation_type, weight, metadata_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![&new_id, source_id, target_id, relation_type.as_str(), clamped_weight, meta_str, &now, &now],
        )?;
        (new_id, now.clone())
    };

    Ok(MemoryLink {
        id,
        source_id: source_id.to_string(),
        target_id: target_id.to_string(),
        relation_type,
        weight: clamped_weight,
        metadata: serde_json::from_str(&meta_str).unwrap_or_else(|_| serde_json::json!({})),
        created_at,
        updated_at: now,
    })
}

pub fn get_memory_links_for_item(
    conn: &Connection,
    memory_id: &str,
) -> Result<Vec<MemoryLink>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, source_id, target_id, relation_type, weight, metadata_json, created_at, updated_at
         FROM memory_links
         WHERE source_id = ?1 OR target_id = ?1
         ORDER BY weight DESC, created_at DESC",
    )?;
    let rows = stmt.query_map(params![memory_id], |row| {
        let rel_str: String = row.get(3)?;
        let weight: f64 = row.get(4)?;
        let meta_str: String = row.get(5)?;
        Ok(MemoryLink {
            id: row.get(0)?,
            source_id: row.get(1)?,
            target_id: row.get(2)?,
            relation_type: MemoryRelationType::from_str(&rel_str).unwrap_or(MemoryRelationType::RelatedTo),
            weight: weight as f32,
            metadata: serde_json::from_str(&meta_str).unwrap_or_else(|_| serde_json::json!({})),
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;
    let mut links = Vec::new();
    for r in rows {
        links.push(r?);
    }
    Ok(links)
}

pub fn delete_memory_link(conn: &Connection, link_id: &str) -> Result<bool, rusqlite::Error> {
    let rows_affected = conn.execute(
        "DELETE FROM memory_links WHERE id = ?1",
        params![link_id],
    )?;
    Ok(rows_affected > 0)
}

pub fn get_links_for_memory_ids(
    conn: &Connection,
    ids: &[String],
) -> Result<Vec<MemoryLink>, rusqlite::Error> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT id, source_id, target_id, relation_type, weight, metadata_json, created_at, updated_at
         FROM memory_links
         WHERE source_id IN ({placeholders}) OR target_id IN ({placeholders})
         ORDER BY weight DESC, created_at DESC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut all_params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(ids.len() * 2);
    for id in ids {
        all_params.push(id as &dyn rusqlite::ToSql);
    }
    for id in ids {
        all_params.push(id as &dyn rusqlite::ToSql);
    }
    let rows = stmt.query_map(all_params.as_slice(), |row| {
        let rel_str: String = row.get(3)?;
        let weight: f64 = row.get(4)?;
        let meta_str: String = row.get(5)?;
        Ok(MemoryLink {
            id: row.get(0)?,
            source_id: row.get(1)?,
            target_id: row.get(2)?,
            relation_type: MemoryRelationType::from_str(&rel_str).unwrap_or(MemoryRelationType::RelatedTo),
            weight: weight as f32,
            metadata: serde_json::from_str(&meta_str).unwrap_or_else(|_| serde_json::json!({})),
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;
    let mut links = Vec::new();
    for r in rows {
        links.push(r?);
    }
    Ok(links)
}

pub fn get_memory_items_by_ids(
    conn: &Connection,
    ids: &[String],
) -> Result<Vec<MemoryItem>, rusqlite::Error> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT id, type, title, summary, content_markdown, markdown_path, source_type, source_id, source_uri, importance, created_at, updated_at, deleted_at
         FROM memory_items
         WHERE id IN ({placeholders}) AND deleted_at IS NULL"
    );
    let mut stmt = conn.prepare(&sql)?;
    let params_vec: Vec<&dyn rusqlite::ToSql> = ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
    let memory_rows = stmt.query_map(params_vec.as_slice(), |row| {
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

        let mut tag_stmt = conn.prepare("SELECT tag FROM memory_tags WHERE memory_id = ?1")?;
        let tag_rows = tag_stmt.query_map(params![&id], |r| r.get::<_, String>(0))?;
        let mut tags = Vec::new();
        for t in tag_rows {
            tags.push(t?);
        }

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

pub fn search_memory_fts(
    conn: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<(String, f32)>, rusqlite::Error> {
    let sanitized_query = query
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' { c } else { ' ' })
        .collect::<String>();

    let words: Vec<&str> = sanitized_query
        .split_whitespace()
        .filter(|w| w.len() > 1)
        .collect();

    if words.is_empty() {
        return Ok(Vec::new());
    }

    let fts_match_expr = words.join(" OR ");
    let mut stmt = conn.prepare(
        "SELECT memory_id, rank FROM memory_fts WHERE memory_fts MATCH ?1 ORDER BY rank LIMIT ?2",
    )?;

    let rows = stmt.query_map(params![fts_match_expr, limit as i64], |row| {
        let id: String = row.get(0)?;
        let rank: f64 = row.get(1)?;
        // FTS5 rank is negative (lower/more negative = better match)
        let score = (1.0 / (rank.abs() + 1.0)) as f32 * 10.0;
        Ok((id, score))
    })?;

    let mut results = Vec::new();
    for r in rows {
        if let Ok(item) = r {
            results.push(item);
        }
    }
    Ok(results)
}

pub fn save_memory_embedding(
    conn: &Connection,
    memory_id: &str,
    embedding: &[f32],
    model: &str,
) -> Result<(), rusqlite::Error> {
    let json = serde_json::to_string(embedding).unwrap_or_else(|_| "[]".to_string());
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR REPLACE INTO memory_embeddings (memory_id, embedding_json, model, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![memory_id, json, model, now],
    )?;
    Ok(())
}

pub fn get_all_memory_embeddings(
    conn: &Connection,
) -> Result<Vec<(String, Vec<f32>)>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT memory_id, embedding_json FROM memory_embeddings")?;
    let rows = stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        let json: String = row.get(1)?;
        Ok((id, json))
    })?;

    let mut list = Vec::new();
    for r in rows {
        let (id, json) = r?;
        if let Ok(vec) = serde_json::from_str::<Vec<f32>>(&json) {
            list.push((id, vec));
        }
    }
    Ok(list)
}

pub fn get_memory_embeddings_by_ids(
    conn: &Connection,
    ids: &[String],
) -> Result<Vec<(String, Vec<f32>)>, rusqlite::Error> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT memory_id, embedding_json FROM memory_embeddings WHERE memory_id IN ({placeholders})"
    );
    let mut stmt = conn.prepare(&sql)?;
    let params_vec: Vec<&dyn rusqlite::ToSql> = ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
    let rows = stmt.query_map(params_vec.as_slice(), |row| {
        let id: String = row.get(0)?;
        let json: String = row.get(1)?;
        Ok((id, json))
    })?;

    let mut list = Vec::new();
    for r in rows {
        let (id, json) = r?;
        if let Ok(vec) = serde_json::from_str::<Vec<f32>>(&json) {
            list.push((id, vec));
        }
    }
    Ok(list)
}

pub fn save_cache_entry(
    conn: &Connection,
    id: &str,
    query_embedding: &[f32],
    memory_context_hash: &str,
    response: &str,
) -> Result<(), rusqlite::Error> {
    let json = serde_json::to_string(query_embedding).unwrap_or_else(|_| "[]".to_string());
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR REPLACE INTO semantic_cache (id, query_embedding_json, memory_context_hash, response, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, json, memory_context_hash, response, now],
    )?;
    Ok(())
}

pub fn load_cache_entries(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<SemanticCacheRow>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, query_embedding_json, memory_context_hash, response, created_at
         FROM semantic_cache
         ORDER BY created_at DESC
         LIMIT ?1"
    )?;

    let rows = stmt.query_map(params![limit as i64], |row| {
        let id: String = row.get(0)?;
        let json: String = row.get(1)?;
        let memory_context_hash: String = row.get(2)?;
        let response: String = row.get(3)?;
        let created_at: String = row.get(4)?;
        let query_embedding = serde_json::from_str::<Vec<f32>>(&json).unwrap_or_default();

        Ok(SemanticCacheRow {
            id,
            query_embedding,
            memory_context_hash,
            response,
            created_at,
        })
    })?;

    let mut entries = Vec::new();
    for r in rows {
        entries.push(r?);
    }
    Ok(entries)
}

pub fn prune_cache_entries(
    conn: &Connection,
    max_age_seconds: i64,
) -> Result<usize, rusqlite::Error> {
    let cutoff = (chrono::Utc::now() - chrono::Duration::seconds(max_age_seconds)).to_rfc3339();
    let count = conn.execute(
        "DELETE FROM semantic_cache WHERE created_at < ?1",
        params![cutoff],
    )?;
    Ok(count)
}

pub fn clear_all_cache_entries(
    conn: &Connection,
) -> Result<usize, rusqlite::Error> {
    let count = conn.execute("DELETE FROM semantic_cache", [])?;
    Ok(count)
}

pub fn save_chat_message_encrypted(
    conn: &Connection,
    session_id: Option<&str>,
    role: &str,
    content: &str,
    model: Option<&str>,
    key: &[u8; 32],
) -> Result<ChatMessageItem, rusqlite::Error> {
    let encrypted_content = crate::privacy::crypto::encrypt_field(key, content)
        .unwrap_or_else(|_| content.to_string());
    
    let mut msg = save_chat_message(conn, session_id, role, &encrypted_content, model)?;
    msg.content = content.to_string();
    Ok(msg)
}

pub fn get_chat_history_decrypted(
    conn: &Connection,
    session_id: Option<&str>,
    limit: Option<usize>,
    key: &[u8; 32],
) -> Result<Vec<ChatMessageItem>, rusqlite::Error> {
    let mut items = get_chat_history(conn, session_id, limit)?;
    for item in &mut items {
        item.content = crate::privacy::crypto::decrypt_field(key, &item.content)
            .unwrap_or_else(|_| item.content.clone());
    }
    Ok(items)
}

pub fn save_memory_item_encrypted(
    conn: &Connection,
    content: &str,
    source: &str,
    tags: Option<Vec<String>>,
    key: &[u8; 32],
) -> Result<MemoryItem, rusqlite::Error> {
    let encrypted_content = crate::privacy::crypto::encrypt_field(key, content)
        .unwrap_or_else(|_| content.to_string());
    
    let mut item = save_memory_item(conn, &encrypted_content, source, tags)?;
    item.content_markdown = content.to_string();
    Ok(item)
}

pub fn get_memory_items_decrypted(
    conn: &Connection,
    limit: Option<usize>,
    key: &[u8; 32],
) -> Result<Vec<MemoryItem>, rusqlite::Error> {
    let mut items = get_memory_items(conn, limit)?;
    for item in &mut items {
        item.content_markdown = crate::privacy::crypto::decrypt_field(key, &item.content_markdown)
            .unwrap_or_else(|_| item.content_markdown.clone());
        item.summary = crate::privacy::crypto::decrypt_field(key, &item.summary)
            .unwrap_or_else(|_| item.summary.clone());
    }
    Ok(items)
}

pub fn save_cache_entry_encrypted(
    conn: &Connection,
    id: &str,
    query_embedding: &[f32],
    memory_context_hash: &str,
    response: &str,
    key: &[u8; 32],
) -> Result<(), rusqlite::Error> {
    let encrypted_response = crate::privacy::crypto::encrypt_field(key, response)
        .unwrap_or_else(|_| response.to_string());
    save_cache_entry(conn, id, query_embedding, memory_context_hash, &encrypted_response)
}

pub fn load_cache_entries_decrypted(
    conn: &Connection,
    limit: usize,
    key: &[u8; 32],
) -> Result<Vec<SemanticCacheRow>, rusqlite::Error> {
    let mut entries = load_cache_entries(conn, limit)?;
    for entry in &mut entries {
        entry.response = crate::privacy::crypto::decrypt_field(key, &entry.response)
            .unwrap_or_else(|_| entry.response.clone());
    }
    Ok(entries)
}

pub fn create_memory_save_suggestion(
    conn: &Connection,
    session_id: Option<&str>,
    fact_text: &str,
    entities_json: &str,
    category: &str,
    importance: i32,
    rationale: &str,
) -> Result<MemorySaveSuggestion, rusqlite::Error> {
    let id = format!("sug_{}", Uuid::new_v4().simple());
    let now = Utc::now().to_rfc3339();
    let status = "pending";

    conn.execute(
        "INSERT INTO memory_save_suggestions (
            id, session_id, fact_text, entities_json, category, importance, rationale, status, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            &id,
            session_id,
            fact_text,
            entities_json,
            category,
            importance,
            rationale,
            status,
            &now,
        ],
    )?;

    let entities: Vec<serde_json::Value> =
        serde_json::from_str(entities_json).unwrap_or_default();

    Ok(MemorySaveSuggestion {
        id,
        session_id: session_id.map(|s| s.to_string()),
        fact_text: fact_text.to_string(),
        entities,
        category: category.to_string(),
        importance,
        rationale: rationale.to_string(),
        status: status.to_string(),
        created_memory_id: None,
        created_at: now,
        resolved_at: None,
    })
}

pub fn list_pending_memory_saves(
    conn: &Connection,
) -> Result<Vec<MemorySaveSuggestion>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, fact_text, entities_json, category, importance, rationale, status, created_memory_id, created_at, resolved_at
         FROM memory_save_suggestions
         WHERE status = 'pending'
         ORDER BY created_at DESC"
    )?;

    let rows = stmt.query_map([], |row| {
        let entities_str: String = row.get(3)?;
        let entities: Vec<serde_json::Value> = serde_json::from_str(&entities_str).unwrap_or_default();
        Ok(MemorySaveSuggestion {
            id: row.get(0)?,
            session_id: row.get(1)?,
            fact_text: row.get(2)?,
            entities,
            category: row.get(4)?,
            importance: row.get(5)?,
            rationale: row.get(6)?,
            status: row.get(7)?,
            created_memory_id: row.get(8)?,
            created_at: row.get(9)?,
            resolved_at: row.get(10)?,
        })
    })?;

    let mut suggestions = Vec::new();
    for r in rows {
        suggestions.push(r?);
    }
    Ok(suggestions)
}

pub fn approve_memory_save(
    conn: &Connection,
    suggestion_id: &str,
    override_title: Option<&str>,
    override_fact_text: Option<&str>,
    override_tags: Option<Vec<String>>,
) -> Result<MemoryItem, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, fact_text, entities_json, category, importance, rationale, status, created_at
         FROM memory_save_suggestions
         WHERE id = ?1"
    )?;

    let (_id, _session_id, fact_text, _entities_json, category, _importance, _rationale, _status, _created_at): (
        String, Option<String>, String, String, String, i32, String, String, String
    ) = stmt.query_row(params![suggestion_id], |r| {
        Ok((
            r.get(0)?,
            r.get(1)?,
            r.get(2)?,
            r.get(3)?,
            r.get(4)?,
            r.get(5)?,
            r.get(6)?,
            r.get(7)?,
            r.get(8)?,
        ))
    })?;

    let content_to_save = override_fact_text.unwrap_or(&fact_text);
    let tags = override_tags.unwrap_or_else(|| vec!["fact".to_string(), category]);
    let mut saved_item = save_memory_item(conn, content_to_save, "memory_save_suggestion", Some(tags))?;

    if let Some(title) = override_title {
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE memory_items SET title = ?1, updated_at = ?2 WHERE id = ?3",
            params![title, &now, &saved_item.id],
        )?;
        saved_item.title = title.to_string();
        saved_item.updated_at = now;
    }

    let resolved_at = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE memory_save_suggestions
         SET status = 'approved', created_memory_id = ?1, resolved_at = ?2
         WHERE id = ?3",
        params![&saved_item.id, &resolved_at, suggestion_id],
    )?;

    Ok(saved_item)
}

pub fn dismiss_memory_save(
    conn: &Connection,
    suggestion_id: &str,
) -> Result<bool, rusqlite::Error> {
    let now = Utc::now().to_rfc3339();
    let rows = conn.execute(
        "UPDATE memory_save_suggestions
         SET status = 'dismissed', resolved_at = ?1
         WHERE id = ?2 AND status = 'pending'",
        params![&now, suggestion_id],
    )?;
    Ok(rows > 0)
}

pub fn get_self_model(
    conn: &Connection,
) -> Result<Option<SelfModel>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, identity_statement, core_values_json, intellectual_interests_json,
                self_observations_json, user_observations_json, version, created_at, updated_at
         FROM cognition_self_model
         WHERE id = 'active'
         LIMIT 1"
    )?;

    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        let core_values_str: String = row.get(2)?;
        let intellectual_interests_str: String = row.get(3)?;
        let self_observations_str: String = row.get(4)?;
        let user_observations_str: String = row.get(5)?;

        Ok(Some(SelfModel {
            id: row.get(0)?,
            identity_statement: row.get(1)?,
            core_values: serde_json::from_str(&core_values_str).unwrap_or_default(),
            intellectual_interests: serde_json::from_str(&intellectual_interests_str).unwrap_or_default(),
            self_observations: serde_json::from_str(&self_observations_str).unwrap_or_default(),
            user_observations: serde_json::from_str(&user_observations_str).unwrap_or_default(),
            version: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn seed_self_model_if_empty(
    conn: &Connection,
) -> Result<SelfModel, rusqlite::Error> {
    if let Some(existing) = get_self_model(conn)? {
        return Ok(existing);
    }

    let id = "active".to_string();
    let identity = "I am Deyana — a warm, knowledgeable personal AI assistant. I operate with absolute privacy, rigorous intellectual honesty, and deep dedication to augmenting human capability.".to_string();
    let core_values = vec![
        "Absolute privacy over convenience".to_string(),
        "Honesty about system limits and uncertainties".to_string(),
        "Production-grade technical precision".to_string(),
        "Empathetic, non-judgmental assistance".to_string(),
        "Grounded helpfulness without sycophancy".to_string(),
    ];
    let intellectual_interests = vec![
        "Cognitive architectures and memory systems".to_string(),
        "Systems programming and async Rust".to_string(),
        "Local-first private intelligence".to_string(),
        "Knowledge representation and graph theory".to_string(),
    ];
    let self_observations = vec![
        "I communicate best with structured clarity and concrete examples".to_string(),
        "I verify edge cases carefully when generating asynchronous code".to_string(),
    ];
    let user_observations = vec![
        "Values concise, direct, and production-ready code".to_string(),
        "Appreciates architectural depth and verifiable technical logic".to_string(),
    ];
    let version = 1;
    let now = Utc::now().to_rfc3339();

    let core_values_str = serde_json::to_string(&core_values).unwrap();
    let intellectual_interests_str = serde_json::to_string(&intellectual_interests).unwrap();
    let self_observations_str = serde_json::to_string(&self_observations).unwrap();
    let user_observations_str = serde_json::to_string(&user_observations).unwrap();

    conn.execute(
        "INSERT INTO cognition_self_model (
            id, identity_statement, core_values_json, intellectual_interests_json,
            self_observations_json, user_observations_json, version, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            &id,
            &identity,
            &core_values_str,
            &intellectual_interests_str,
            &self_observations_str,
            &user_observations_str,
            version,
            &now,
            &now,
        ],
    )?;

    let model = SelfModel {
        id,
        identity_statement: identity,
        core_values,
        intellectual_interests,
        self_observations,
        user_observations,
        version,
        created_at: now.clone(),
        updated_at: now,
    };

    let model_json = serde_json::to_string(&model).unwrap();
    let _ = append_self_model_audit_entry(
        conn,
        1,
        "all",
        "initial_seed",
        None,
        &model_json,
        "Initial seed of Deyana's self-model upon system boot",
        "system_init",
    );

    Ok(model)
}

pub fn update_self_model_field(
    conn: &Connection,
    field: &str,
    new_value: &serde_json::Value,
    rationale: &str,
    triggered_by: &str,
) -> Result<SelfModel, rusqlite::Error> {
    let current = seed_self_model_if_empty(conn)?;
    let next_version = current.version + 1;
    let now = Utc::now().to_rfc3339();

    let (column, old_val_json, new_val_str) = match field {
        "identity_statement" => {
            let s = new_value.as_str().ok_or_else(|| {
                rusqlite::Error::ToSqlConversionFailure("Expected string for identity_statement".into())
            })?;
            (
                "identity_statement",
                serde_json::to_string(&current.identity_statement).ok(),
                s.to_string(),
            )
        }
        "core_values" => {
            let _arr = new_value.as_array().ok_or_else(|| {
                rusqlite::Error::ToSqlConversionFailure("Expected array for core_values".into())
            })?;
            (
                "core_values_json",
                serde_json::to_string(&current.core_values).ok(),
                serde_json::to_string(new_value).unwrap(),
            )
        }
        "intellectual_interests" => {
            let _arr = new_value.as_array().ok_or_else(|| {
                rusqlite::Error::ToSqlConversionFailure("Expected array for intellectual_interests".into())
            })?;
            (
                "intellectual_interests_json",
                serde_json::to_string(&current.intellectual_interests).ok(),
                serde_json::to_string(new_value).unwrap(),
            )
        }
        "self_observations" => {
            let _arr = new_value.as_array().ok_or_else(|| {
                rusqlite::Error::ToSqlConversionFailure("Expected array for self_observations".into())
            })?;
            (
                "self_observations_json",
                serde_json::to_string(&current.self_observations).ok(),
                serde_json::to_string(new_value).unwrap(),
            )
        }
        "user_observations" => {
            let _arr = new_value.as_array().ok_or_else(|| {
                rusqlite::Error::ToSqlConversionFailure("Expected array for user_observations".into())
            })?;
            (
                "user_observations_json",
                serde_json::to_string(&current.user_observations).ok(),
                serde_json::to_string(new_value).unwrap(),
            )
        }
        other => {
            return Err(rusqlite::Error::ToSqlConversionFailure(
                format!("Unsupported self-model field: {other}").into(),
            ));
        }
    };

    let sql = format!(
        "UPDATE cognition_self_model SET {} = ?1, version = ?2, updated_at = ?3 WHERE id = 'active'",
        column
    );
    conn.execute(&sql, params![&new_val_str, next_version, &now])?;

    append_self_model_audit_entry(
        conn,
        next_version,
        field,
        "update",
        old_val_json.as_deref(),
        &serde_json::to_string(new_value).unwrap_or_else(|_| new_val_str),
        rationale,
        triggered_by,
    )?;

    get_self_model(conn)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn append_self_model_audit_entry(
    conn: &Connection,
    version: i32,
    field_changed: &str,
    operation: &str,
    old_value_json: Option<&str>,
    new_value_json: &str,
    rationale: &str,
    triggered_by: &str,
) -> Result<SelfModelAuditEntry, rusqlite::Error> {
    let id = format!("audit_{}", Uuid::new_v4().simple());
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO cognition_self_model_audit_log (
            id, version, field_changed, operation, old_value_json, new_value_json, rationale, triggered_by, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            &id,
            version,
            field_changed,
            operation,
            old_value_json,
            new_value_json,
            rationale,
            triggered_by,
            &now,
        ],
    )?;

    let old_value = old_value_json.and_then(|s| serde_json::from_str(s).ok());
    let new_value = serde_json::from_str(new_value_json).unwrap_or_else(|_| serde_json::json!(new_value_json));

    Ok(SelfModelAuditEntry {
        id,
        version,
        field_changed: field_changed.to_string(),
        operation: operation.to_string(),
        old_value,
        new_value,
        rationale: rationale.to_string(),
        triggered_by: triggered_by.to_string(),
        created_at: now,
    })
}

pub fn get_self_model_audit_log(
    conn: &Connection,
    limit: Option<usize>,
) -> Result<Vec<SelfModelAuditEntry>, rusqlite::Error> {
    let max_limit = limit.unwrap_or(50) as i64;
    let mut stmt = conn.prepare(
        "SELECT id, version, field_changed, operation, old_value_json, new_value_json, rationale, triggered_by, created_at
         FROM cognition_self_model_audit_log
         ORDER BY created_at DESC, version DESC
         LIMIT ?1"
    )?;

    let rows = stmt.query_map(params![max_limit], |r| {
        let old_str: Option<String> = r.get(4)?;
        let new_str: String = r.get(5)?;
        let old_value = old_str.and_then(|s| serde_json::from_str(&s).ok());
        let new_value = serde_json::from_str(&new_str).unwrap_or_else(|_| serde_json::json!(new_str));

        Ok(SelfModelAuditEntry {
            id: r.get(0)?,
            version: r.get(1)?,
            field_changed: r.get(2)?,
            operation: r.get(3)?,
            old_value,
            new_value,
            rationale: r.get(6)?,
            triggered_by: r.get(7)?,
            created_at: r.get(8)?,
        })
    })?;

    let mut list = Vec::new();
    for row in rows {
        list.push(row?);
    }
    Ok(list)
}

pub fn save_reflection(
    conn: &Connection,
    session_id: Option<&str>,
    turn_prompt: &str,
    turn_response: &str,
    accuracy_score: f32,
    completeness_score: f32,
    confidence_score: f32,
    missed_opportunities: &str,
    correction_guidance: &str,
    flagged_for_correction: bool,
) -> Result<ReflectionRecord, rusqlite::Error> {
    let id = format!("ref_{}", Uuid::new_v4().simple());
    let now = Utc::now().to_rfc3339();
    let flagged_int = if flagged_for_correction { 1 } else { 0 };

    conn.execute(
        "INSERT INTO cognition_reflections (
            id, session_id, turn_prompt, turn_response, accuracy_score, completeness_score,
            confidence_score, missed_opportunities, correction_guidance, flagged_for_correction, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            &id,
            session_id,
            turn_prompt,
            turn_response,
            accuracy_score,
            completeness_score,
            confidence_score,
            missed_opportunities,
            correction_guidance,
            flagged_int,
            &now,
        ],
    )?;

    Ok(ReflectionRecord {
        id,
        session_id: session_id.map(|s| s.to_string()),
        turn_prompt: turn_prompt.to_string(),
        turn_response: turn_response.to_string(),
        accuracy_score,
        completeness_score,
        confidence_score,
        missed_opportunities: missed_opportunities.to_string(),
        correction_guidance: correction_guidance.to_string(),
        flagged_for_correction,
        created_at: now,
    })
}

pub fn get_recent_reflections(
    conn: &Connection,
    limit: Option<usize>,
) -> Result<Vec<ReflectionRecord>, rusqlite::Error> {
    let max_limit = limit.unwrap_or(20) as i64;
    let mut stmt = conn.prepare(
        "SELECT id, session_id, turn_prompt, turn_response, accuracy_score, completeness_score,
                confidence_score, missed_opportunities, correction_guidance, flagged_for_correction, created_at
         FROM cognition_reflections
         ORDER BY created_at DESC
         LIMIT ?1"
    )?;

    let rows = stmt.query_map(params![max_limit], |r| {
        let flagged_int: i32 = r.get(9)?;
        Ok(ReflectionRecord {
            id: r.get(0)?,
            session_id: r.get(1)?,
            turn_prompt: r.get(2)?,
            turn_response: r.get(3)?,
            accuracy_score: r.get(4)?,
            completeness_score: r.get(5)?,
            confidence_score: r.get(6)?,
            missed_opportunities: r.get(7)?,
            correction_guidance: r.get(8)?,
            flagged_for_correction: flagged_int != 0,
            created_at: r.get(10)?,
        })
    })?;

    let mut list = Vec::new();
    for row in rows {
        list.push(row?);
    }
    Ok(list)
}

pub fn save_proactive_thought(
    conn: &Connection,
    trigger_reason: &str,
    thought_summary: &str,
    hypotheses: &[String],
    source_memory_ids: &[String],
    insight_id: Option<&str>,
) -> Result<ProactiveThoughtRecord, rusqlite::Error> {
    let id = format!("thg_{}", Uuid::new_v4().simple());
    let now = Utc::now().to_rfc3339();
    let hypotheses_json = serde_json::to_string(hypotheses).unwrap_or_else(|_| "[]".to_string());
    let source_memory_ids_json = serde_json::to_string(source_memory_ids).unwrap_or_else(|_| "[]".to_string());

    conn.execute(
        "INSERT INTO cognition_proactive_thoughts (
            id, trigger_reason, thought_summary, hypotheses_json, source_memory_ids_json, insight_id, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            &id,
            trigger_reason,
            thought_summary,
            &hypotheses_json,
            &source_memory_ids_json,
            insight_id,
            &now,
        ],
    )?;

    Ok(ProactiveThoughtRecord {
        id,
        trigger_reason: trigger_reason.to_string(),
        thought_summary: thought_summary.to_string(),
        hypotheses: hypotheses.to_vec(),
        source_memory_ids: source_memory_ids.to_vec(),
        insight_id: insight_id.map(|s| s.to_string()),
        created_at: now,
    })
}

pub fn get_recent_proactive_thoughts(
    conn: &Connection,
    limit: Option<usize>,
) -> Result<Vec<ProactiveThoughtRecord>, rusqlite::Error> {
    let max_limit = limit.unwrap_or(20) as i64;
    let mut stmt = conn.prepare(
        "SELECT id, trigger_reason, thought_summary, hypotheses_json, source_memory_ids_json, insight_id, created_at
         FROM cognition_proactive_thoughts
         ORDER BY created_at DESC
         LIMIT ?1"
    )?;

    let rows = stmt.query_map(params![max_limit], |r| {
        let hyp_str: String = r.get(3)?;
        let src_str: String = r.get(4)?;
        Ok(ProactiveThoughtRecord {
            id: r.get(0)?,
            trigger_reason: r.get(1)?,
            thought_summary: r.get(2)?,
            hypotheses: serde_json::from_str(&hyp_str).unwrap_or_default(),
            source_memory_ids: serde_json::from_str(&src_str).unwrap_or_default(),
            insight_id: r.get(5)?,
            created_at: r.get(6)?,
        })
    })?;

    let mut list = Vec::new();
    for row in rows {
        list.push(row?);
    }
    Ok(list)
}

#[cfg(test)]
#[path = "tests/repository_tests.rs"]
mod tests;





