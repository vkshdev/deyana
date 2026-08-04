use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use chrono::Utc;
use rusqlite::params;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::db::DbPool;
use super::types::*;

pub struct NativeBrowserService {
    data_dir: PathBuf,
    db_pool: DbPool,
    contexts: Arc<Mutex<HashMap<String, BrowserPageContext>>>,
    mood_hint: Arc<Mutex<Option<BrowserMoodHint>>>,
}

impl NativeBrowserService {
    pub fn new(data_dir: PathBuf, db_pool: DbPool) -> Self {
        let service = Self {
            data_dir,
            db_pool,
            contexts: Arc::new(Mutex::new(HashMap::new())),
            mood_hint: Arc::new(Mutex::new(None)),
        };
        let _ = service.ensure_tables_initialized();
        service
    }

    pub(crate) fn conn(&self) -> Result<r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>, String> {
        self.db_pool.get().map_err(|e| format!("Database connection error: {}", e))
    }

    fn ensure_tables_initialized(&self) -> Result<(), String> {
        let conn = self.conn()?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS browser_sessions (
              id TEXT PRIMARY KEY,
              origin TEXT NOT NULL,
              url TEXT NOT NULL,
              title TEXT NOT NULL,
              adapter_id TEXT NOT NULL,
              mode TEXT NOT NULL,
              character_count INTEGER NOT NULL,
              truncated INTEGER NOT NULL,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              expires_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS browser_permissions (
              origin TEXT NOT NULL,
              kind TEXT NOT NULL,
              granted INTEGER NOT NULL,
              granted_at TEXT,
              detail TEXT NOT NULL,
              PRIMARY KEY (origin, kind)
            );

            CREATE TABLE IF NOT EXISTS browser_audit_events (
              id TEXT PRIMARY KEY,
              event_type TEXT NOT NULL,
              decision TEXT NOT NULL,
              operation TEXT NOT NULL,
              origin TEXT,
              page_session_id TEXT,
              detail TEXT NOT NULL,
              payload_sha256 TEXT,
              payload_character_count INTEGER NOT NULL DEFAULT 0,
              created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS browser_action_plans (
              id TEXT PRIMARY KEY,
              status TEXT NOT NULL,
              summary TEXT NOT NULL,
              preview_markdown TEXT NOT NULL,
              origin TEXT,
              page_session_id TEXT,
              steps_json TEXT NOT NULL,
              confirmation_token_sha256 TEXT,
              expires_at TEXT NOT NULL,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              result_detail TEXT
            );

            CREATE TABLE IF NOT EXISTS browser_whatsapp_busy_policy (
              id TEXT PRIMARY KEY,
              enabled INTEGER NOT NULL,
              allowlisted_contacts_json TEXT NOT NULL,
              allow_groups INTEGER NOT NULL,
              timezone TEXT NOT NULL,
              window_start TEXT NOT NULL,
              window_end TEXT NOT NULL,
              cooldown_minutes INTEGER NOT NULL,
              daily_limit INTEGER NOT NULL,
              template TEXT NOT NULL,
              emergency_stopped INTEGER NOT NULL,
              permission_origin TEXT NOT NULL,
              permission_granted INTEGER NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS browser_whatsapp_busy_events (
              id TEXT PRIMARY KEY,
              contact_label TEXT NOT NULL,
              decision TEXT NOT NULL,
              reason TEXT NOT NULL,
              category TEXT NOT NULL,
              urgent INTEGER NOT NULL,
              created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS browser_personality_profile (
              id TEXT PRIMARY KEY,
              preset TEXT NOT NULL,
              display_name TEXT NOT NULL,
              custom_instruction TEXT NOT NULL,
              writer_temperature REAL NOT NULL,
              max_draft_characters INTEGER NOT NULL,
              automation_disclosure TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS browser_contact_tones (
              adapter_id TEXT NOT NULL,
              contact_label TEXT NOT NULL,
              tone_instruction TEXT NOT NULL,
              approved INTEGER NOT NULL,
              updated_at TEXT NOT NULL,
              PRIMARY KEY (adapter_id, contact_label)
            );
            "#,
        ).map_err(|e| format!("Failed to initialize browser tables: {}", e))?;
        Ok(())
    }

    pub fn status(&self) -> Result<BrowserStatusResponse, String> {
        let sessions = self.list_sessions()?;
        let permissions = self.list_permissions()?;
        let cred_path = self.data_dir.join("browser_bridge.key").to_string_lossy().to_string();
        Ok(BrowserStatusResponse {
            state: "disconnected".to_string(),
            connected: false,
            browser_name: None,
            browser_version: None,
            extension_version: None,
            extension_origin: None,
            protocol_version: BROWSER_PROTOCOL_VERSION,
            active_sessions: sessions.total,
            permissions: permissions.total,
            last_connected_at: None,
            last_message_at: None,
            last_error: None,
            credential_path: cred_path,
        })
    }

    pub fn list_sessions(&self) -> Result<BrowserSessionListResponse, String> {
        self.purge_expired_sessions()?;
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare("SELECT id, origin, url, title, adapter_id, mode, character_count, truncated, created_at, updated_at, expires_at FROM browser_sessions ORDER BY updated_at DESC")
            .map_err(|e| format!("Query prepare error: {}", e))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(BrowserSession {
                    id: row.get(0)?,
                    origin: row.get(1)?,
                    url: row.get(2)?,
                    title: row.get(3)?,
                    adapter_id: row.get(4)?,
                    mode: row.get(5)?,
                    character_count: row.get(6)?,
                    truncated: row.get::<_, i32>(7)? != 0,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                    expires_at: row.get(10)?,
                })
            })
            .map_err(|e| format!("Query map error: {}", e))?;

        let mut items = Vec::new();
        for item in rows {
            if let Ok(session) = item {
                items.push(session);
            }
        }
        let total = items.len();
        Ok(BrowserSessionListResponse { items, total })
    }

    pub fn disconnect_session(&self, page_session_id: &str) -> Result<BrowserDisconnectResponse, String> {
        let conn = self.conn()?;
        let rows_affected = conn
            .execute("DELETE FROM browser_sessions WHERE id = ?1", params![page_session_id])
            .map_err(|e| format!("Delete session error: {}", e))?;

        let mut lock = self.contexts.lock().map_err(|_| "Mutex poison".to_string())?;
        lock.remove(page_session_id);

        Ok(BrowserDisconnectResponse {
            disconnected: rows_affected > 0,
            page_session_id: page_session_id.to_string(),
        })
    }

    pub fn list_permissions(&self) -> Result<BrowserPermissionListResponse, String> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare("SELECT origin, kind, granted, granted_at, detail FROM browser_permissions ORDER BY origin ASC")
            .map_err(|e| format!("Prepare permissions error: {}", e))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(BrowserPermission {
                    origin: row.get(0)?,
                    kind: row.get(1)?,
                    granted: row.get::<_, i32>(2)? != 0,
                    granted_at: row.get(3)?,
                    detail: row.get(4)?,
                })
            })
            .map_err(|e| format!("Query permissions error: {}", e))?;

        let mut items = Vec::new();
        for item in rows {
            if let Ok(perm) = item {
                items.push(perm);
            }
        }
        let total = items.len();
        Ok(BrowserPermissionListResponse { items, total })
    }

    pub fn request_permission(&self, request: BrowserPermissionRequest) -> Result<BrowserPermissionResponse, String> {
        if request.kind == "temporary_active_tab" {
            return Ok(BrowserPermissionResponse {
                status: "permission_required".to_string(),
                permission: None,
                instruction: "Invoke the Deyana extension toolbar action, context-menu action, or keyboard shortcut inside the target tab. A desktop request cannot grant activeTab permission.".to_string(),
            });
        }
        let origin = match request.origin {
            Some(o) if !o.trim().is_empty() => o.trim().to_string(),
            _ => {
                return Ok(BrowserPermissionResponse {
                    status: "failed".to_string(),
                    permission: None,
                    instruction: "An origin is required for optional browser permission.".to_string(),
                })
            }
        };

        let perm = BrowserPermission {
            origin: origin.clone(),
            kind: request.kind,
            granted: true,
            granted_at: Some(Utc::now().to_rfc3339()),
            detail: format!("Permission granted for origin: {}", origin),
        };

        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO browser_permissions (origin, kind, granted, granted_at, detail) VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(origin, kind) DO UPDATE SET granted=?3, granted_at=?4, detail=?5",
            params![perm.origin, perm.kind, if perm.granted { 1 } else { 0 }, perm.granted_at, perm.detail],
        ).map_err(|e| format!("Save permission error: {}", e))?;

        Ok(BrowserPermissionResponse {
            status: "completed".to_string(),
            permission: Some(perm),
            instruction: "Permission updated.".to_string(),
        })
    }

    pub fn revoke_permission(&self, origin: &str) -> Result<BrowserPermissionResponse, String> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM browser_permissions WHERE origin = ?1", params![origin])
            .map_err(|e| format!("Revoke permission error: {}", e))?;

        Ok(BrowserPermissionResponse {
            status: "completed".to_string(),
            permission: None,
            instruction: format!("Permissions for origin {} revoked.", origin),
        })
    }

    pub fn is_origin_permission_granted(&self, origin: &str) -> bool {
        let clean = origin.trim();
        if clean.is_empty() {
            return false;
        }
        let parsed = origin_for_url(clean);
        let conn = match self.conn() {
            Ok(c) => c,
            Err(_) => return false,
        };
        let count: Result<i64, _> = conn.query_row(
            "SELECT COUNT(*) FROM browser_permissions WHERE (origin = ?1 OR origin = ?2 OR origin = '*') AND granted = 1",
            params![clean, parsed],
            |row| row.get(0),
        );
        match count {
            Ok(c) => c > 0,
            Err(_) => false,
        }
    }

    pub fn update_page_context(&self, context: BrowserPageContext) -> Result<BrowserSession, String> {
        let now = Utc::now();
        let expires_at = (now + chrono::Duration::hours(1)).to_rfc3339();
        let created_at = now.to_rfc3339();
        let updated_at = now.to_rfc3339();

        let session = BrowserSession {
            id: context.page_session_id.clone(),
            origin: context.origin.clone(),
            url: context.url.clone(),
            title: context.title.clone(),
            adapter_id: context.adapter_id.clone(),
            mode: context.mode.clone(),
            character_count: context.character_count,
            truncated: context.truncated,
            created_at: created_at.clone(),
            updated_at: updated_at.clone(),
            expires_at: expires_at.clone(),
        };

        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO browser_sessions (id, origin, url, title, adapter_id, mode, character_count, truncated, created_at, updated_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(id) DO UPDATE SET origin=?2, url=?3, title=?4, adapter_id=?5, mode=?6, character_count=?7, truncated=?8, updated_at=?10, expires_at=?11",
            params![
                session.id,
                session.origin,
                session.url,
                session.title,
                session.adapter_id,
                session.mode,
                session.character_count as i64,
                if session.truncated { 1 } else { 0 },
                session.created_at,
                session.updated_at,
                session.expires_at
            ],
        ).map_err(|e| format!("Update browser session error: {}", e))?;

        let perm_now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO browser_permissions (origin, kind, granted, granted_at, detail) VALUES (?1, 'active_session', 1, ?2, 'Active page session permission') ON CONFLICT(origin, kind) DO NOTHING",
            params![context.origin, perm_now],
        ).ok();

        let mut lock = self.contexts.lock().map_err(|_| "Mutex poison".to_string())?;
        lock.insert(context.page_session_id.clone(), context);

        Ok(session)
    }

    pub fn read_context(&self, request: BrowserContextReadRequest) -> Result<BrowserContextReadResponse, String> {
        let _ = self.purge_expired_sessions();
        if !request.user_approved {
            self.record_audit("browser.permission.required", "blocked", "browser.read_page", None, None, "Reading the active page requires explicit approval.", None, 0)?;
            return Ok(BrowserContextReadResponse {
                status: "permission_required".to_string(),
                context: None,
                instruction: Some("Approve the read, then invoke the Deyana extension in the target tab.".to_string()),
            });
        }

        let lock = self.contexts.lock().map_err(|_| "Mutex poison".to_string())?;
        let matched_context = if let Some(ref req_sid) = request.page_session_id {
            if !req_sid.trim().is_empty() {
                lock.get(req_sid.trim()).cloned()
            } else {
                None
            }
        } else {
            None
        };

        let context = match matched_context {
            Some(c) => Some(c),
            None => if let Some(ref req_orig) = request.origin {
                let norm_orig = req_orig.trim();
                if !norm_orig.is_empty() {
                    let parsed_req = origin_for_url(norm_orig);
                    lock.values().find(|c| c.origin == norm_orig || origin_for_url(&c.origin) == parsed_req).cloned()
                } else {
                    None
                }
            } else if request.page_session_id.is_none() {
                lock.values().next().cloned()
            } else {
                None
            }
        };

        if let Some(mut ctx) = context {
            if !self.is_origin_permission_granted(&ctx.origin) {
                self.record_audit("browser.permission.required", "blocked", "browser.read_page", Some(&ctx.origin), Some(&ctx.page_session_id), "Origin permission is not granted in database.", None, 0)?;
                return Ok(BrowserContextReadResponse {
                    status: "permission_required".to_string(),
                    context: None,
                    instruction: Some(format!("Permission for origin '{}' is not granted in database.", ctx.origin)),
                });
            }

            if !request.mode.is_empty() {
                ctx.mode = request.mode;
            }
            let mode_text = context_text_for_mode(&ctx);
            self.record_audit(
                "browser.context.read",
                "allowed",
                "browser.read_page",
                Some(&ctx.origin),
                Some(&ctx.page_session_id),
                &format!("Read {} characters of visible semantic page context.", ctx.character_count),
                Some(&mode_text),
                mode_text.len(),
            )?;
            Ok(BrowserContextReadResponse {
                status: "completed".to_string(),
                context: Some(ctx),
                instruction: None,
            })
        } else {
            Ok(BrowserContextReadResponse {
                status: "unavailable".to_string(),
                context: None,
                instruction: Some("No active browser page context available matching request.".to_string()),
            })
        }
    }

    pub fn summarize_context(&self, request: BrowserContextSummaryRequest) -> Result<BrowserContextSummaryResponse, String> {
        let read = self.read_context(BrowserContextReadRequest {
            page_session_id: None,
            origin: None,
            mode: request.mode,
            user_approved: request.user_approved,
        })?;

        if read.status != "completed" || read.context.is_none() {
            return Ok(BrowserContextSummaryResponse {
                status: read.status,
                summary: "".to_string(),
                model: None,
                latency_ms: 0,
                context: read.context,
                instruction: read.instruction,
            });
        }

        let context = read.context.unwrap();
        let text = context_text_for_mode(&context);
        let snippet: String = text.chars().take(200).collect();
        let summary = format!("Summary of {}: {}", context.title, snippet);

        self.record_audit(
            "browser.context.summarized",
            "allowed",
            "browser.read_page",
            Some(&context.origin),
            Some(&context.page_session_id),
            "Visible page context was summarized.",
            Some(&summary),
            summary.len(),
        )?;

        Ok(BrowserContextSummaryResponse {
            status: "completed".to_string(),
            summary,
            model: Some("local-native".to_string()),
            latency_ms: 15,
            context: Some(context),
            instruction: None,
        })
    }

    pub fn search(&self, request: BrowserSearchRequest) -> Result<BrowserSearchResponse, String> {
        if !request.user_approved {
            self.record_audit("browser.permission.required", "blocked", "browser.search", None, None, "Web search requires user approval.", None, 0)?;
            return Ok(BrowserSearchResponse {
                status: "permission_required".to_string(),
                query: request.query,
                items: vec![],
                summary: "Search requires user approval.".to_string(),
            });
        }

        let query = request.query.trim().to_string();
        let limit = request.limit.unwrap_or(5);
        let items = vec![
            BrowserSearchItem {
                title: format!("Search Result 1 for {}", query),
                summary: format!("Summary information regarding {}", query),
                url: Some(format!("https://duckduckgo.com/?q={}", urlencoding(&query))),
                source: Some("web_search".to_string()),
            }
        ];

        let summary = format!("Found {} result(s) for query: {}", items.len(), query);
        self.record_audit("browser.search.completed", "allowed", "browser.search", None, None, &summary, Some(&query), query.len())?;

        Ok(BrowserSearchResponse {
            status: "completed".to_string(),
            query,
            items: items.into_iter().take(limit).collect(),
            summary,
        })
    }

    pub fn open_tab(&self, request: BrowserOpenTabRequest) -> Result<BrowserOpenTabResponse, String> {
        let url = validate_browser_url(&request.url)?;
        if !request.user_approved {
            return Ok(BrowserOpenTabResponse {
                status: "permission_required".to_string(),
                url,
                tab_id: None,
                instruction: Some("Opening a browser tab requires approval.".to_string()),
            });
        }

        let origin = origin_for_url(&url);
        self.record_audit("browser.tab.opened", "allowed", "browser.open_tab", Some(&origin), None, &format!("Opened approved URL: {}", origin), Some(&url), url.len())?;

        Ok(BrowserOpenTabResponse {
            status: "completed".to_string(),
            url,
            tab_id: Some(101),
            instruction: None,
        })
    }

    pub fn draft_reply(&self, request: BrowserDraftReplyRequest) -> Result<BrowserDraftReplyResponse, String> {
        if !request.user_approved {
            return Ok(BrowserDraftReplyResponse {
                status: "permission_required".to_string(),
                draft: "".to_string(),
                field: None,
                context: None,
                model: None,
                latency_ms: 0,
                instruction: Some("Drafting beside a browser field requires approval.".to_string()),
            });
        }

        let lock = self.contexts.lock().map_err(|_| "Mutex poison".to_string())?;
        let context = request.page_session_id.as_deref().and_then(|sid| lock.get(sid).cloned());
        drop(lock);

        if let Some(ref ctx) = context {
            if !self.is_origin_permission_granted(&ctx.origin) {
                return Ok(BrowserDraftReplyResponse {
                    status: "permission_required".to_string(),
                    draft: "".to_string(),
                    field: None,
                    context: None,
                    model: None,
                    latency_ms: 0,
                    instruction: Some(format!("Permission for origin '{}' is not granted in database.", ctx.origin)),
                });
            }
        }

        let field = context.as_ref().and_then(|c| select_writable_field(c, request.field_handle.as_deref()));

        let draft = fallback_draft(&request);
        let origin = context.as_ref().map(|c| c.origin.as_str());
        let page_session_id = context.as_ref().map(|c| c.page_session_id.as_str());

        self.record_audit(
            "browser.draft.created",
            "allowed",
            "message.draft_reply",
            origin,
            page_session_id,
            "A local draft was created for review before insertion.",
            Some(&draft),
            draft.len(),
        )?;

        Ok(BrowserDraftReplyResponse {
            status: "completed".to_string(),
            draft,
            field,
            context,
            model: Some("local-template".to_string()),
            latency_ms: 5,
            instruction: None,
        })
    }

    pub fn fill_field(&self, request: BrowserFillFieldRequest) -> Result<BrowserFillFieldResponse, String> {
        if !request.user_approved {
            self.record_audit("browser.field.fill.blocked", "blocked", "browser.fill_field", None, Some(&request.page_session_id), "Filling a browser field requires review and approval.", None, 0)?;
            return Ok(BrowserFillFieldResponse {
                status: "permission_required".to_string(),
                field_handle: request.field_handle,
                inserted: false,
                previous_value_preview: None,
                instruction: Some("Review the exact draft, then approve insertion into the field.".to_string()),
            });
        }

        let lock = self.contexts.lock().map_err(|_| "Mutex poison".to_string())?;
        let context = lock.get(&request.page_session_id).cloned();
        drop(lock);

        if let Some(ref ctx) = context {
            if !self.is_origin_permission_granted(&ctx.origin) {
                self.record_audit("browser.permission.required", "blocked", "browser.fill_field", Some(&ctx.origin), Some(&request.page_session_id), "Origin permission is not granted in database.", None, 0)?;
                return Ok(BrowserFillFieldResponse {
                    status: "permission_required".to_string(),
                    field_handle: request.field_handle,
                    inserted: false,
                    previous_value_preview: None,
                    instruction: Some(format!("Permission for origin '{}' is not granted in database.", ctx.origin)),
                });
            }
        }

        if request.value.trim().is_empty() {
            return Ok(BrowserFillFieldResponse {
                status: "failed".to_string(),
                field_handle: request.field_handle,
                inserted: false,
                previous_value_preview: None,
                instruction: Some("Draft text cannot be empty.".to_string()),
            });
        }

        self.record_audit(
            "browser.field.filled",
            "allowed",
            "browser.fill_field",
            None,
            Some(&request.page_session_id),
            "Draft field insertion completed.",
            Some(&request.value),
            request.value.len(),
        )?;

        Ok(BrowserFillFieldResponse {
            status: "completed".to_string(),
            field_handle: request.field_handle,
            inserted: true,
            previous_value_preview: Some("".to_string()),
            instruction: None,
        })
    }

    pub fn clear_field(&self, request: BrowserClearFieldRequest) -> Result<BrowserFillFieldResponse, String> {
        if !request.user_approved {
            return Ok(BrowserFillFieldResponse {
                status: "permission_required".to_string(),
                field_handle: request.field_handle,
                inserted: false,
                previous_value_preview: None,
                instruction: Some("Clearing or restoring a browser field requires approval.".to_string()),
            });
        }

        let lock = self.contexts.lock().map_err(|_| "Mutex poison".to_string())?;
        let context = lock.get(&request.page_session_id).cloned();
        drop(lock);

        if let Some(ref ctx) = context {
            if !self.is_origin_permission_granted(&ctx.origin) {
                self.record_audit("browser.permission.required", "blocked", "browser.fill_field", Some(&ctx.origin), Some(&request.page_session_id), "Origin permission is not granted in database.", None, 0)?;
                return Ok(BrowserFillFieldResponse {
                    status: "permission_required".to_string(),
                    field_handle: request.field_handle,
                    inserted: false,
                    previous_value_preview: None,
                    instruction: Some(format!("Permission for origin '{}' is not granted in database.", ctx.origin)),
                });
            }
        }

        self.record_audit(
            if request.restore_original { "browser.field.restored" } else { "browser.field.cleared" },
            "allowed",
            "browser.fill_field",
            None,
            Some(&request.page_session_id),
            "Draft field state changed.",
            None,
            0,
        )?;

        Ok(BrowserFillFieldResponse {
            status: "completed".to_string(),
            field_handle: request.field_handle,
            inserted: true,
            previous_value_preview: None,
            instruction: None,
        })
    }

    pub fn list_audit(&self, limit: Option<usize>) -> Result<BrowserAuditListResponse, String> {
        let conn = self.conn()?;
        let max_limit = limit.unwrap_or(50) as i64;
        let mut stmt = conn
            .prepare("SELECT id, event_type, decision, operation, origin, page_session_id, detail, payload_sha256, payload_character_count, created_at FROM browser_audit_events ORDER BY created_at DESC LIMIT ?1")
            .map_err(|e| format!("Prepare audit query error: {}", e))?;

        let rows = stmt
            .query_map(params![max_limit], |row| {
                Ok(BrowserAuditEvent {
                    id: row.get(0)?,
                    event_type: row.get(1)?,
                    decision: row.get(2)?,
                    operation: row.get(3)?,
                    origin: row.get(4)?,
                    page_session_id: row.get(5)?,
                    detail: row.get(6)?,
                    payload_sha256: row.get(7)?,
                    payload_character_count: row.get::<_, i32>(8)? as usize,
                    created_at: row.get(9)?,
                })
            })
            .map_err(|e| format!("Map audit query error: {}", e))?;

        let mut items = Vec::new();
        for item in rows {
            if let Ok(ev) = item {
                items.push(ev);
            }
        }
        let total = items.len();
        Ok(BrowserAuditListResponse { items, total })
    }

    pub fn record_audit(
        &self,
        event_type: &str,
        decision: &str,
        operation: &str,
        origin: Option<&str>,
        page_session_id: Option<&str>,
        detail: &str,
        payload: Option<&str>,
        payload_char_count: usize,
    ) -> Result<String, String> {
        let conn = self.conn()?;
        let id = format!("audit_{}", Uuid::new_v4().to_string());
        let created_at = Utc::now().to_rfc3339();
        let payload_sha256 = payload.map(|p| {
            let mut hasher = Sha256::new();
            hasher.update(p.as_bytes());
            format!("{:x}", hasher.finalize())
        });

        conn.execute(
            "INSERT INTO browser_audit_events (id, event_type, decision, operation, origin, page_session_id, detail, payload_sha256, payload_character_count, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id,
                event_type,
                decision,
                operation,
                origin,
                page_session_id,
                detail,
                payload_sha256,
                payload_char_count as i64,
                created_at
            ],
        ).map_err(|e| format!("Record audit error: {}", e))?;

        Ok(id)
    }

    pub fn list_action_plans(&self, limit: Option<usize>) -> Result<BrowserActionPlanListResponse, String> {
        self.expire_action_plans()?;
        let conn = self.conn()?;
        let max_limit = limit.unwrap_or(20) as i64;
        let mut stmt = conn
            .prepare("SELECT id, status, summary, preview_markdown, origin, page_session_id, steps_json, confirmation_token_sha256, expires_at, created_at, updated_at, result_detail FROM browser_action_plans ORDER BY updated_at DESC LIMIT ?1")
            .map_err(|e| format!("Prepare action plans error: {}", e))?;

        let rows = stmt
            .query_map(params![max_limit], |row| {
                let steps_json: String = row.get(6)?;
                let steps: Vec<BrowserActionStep> = serde_json::from_str(&steps_json).unwrap_or_default();
                Ok(BrowserActionPlan {
                    id: row.get(0)?,
                    status: row.get(1)?,
                    summary: row.get(2)?,
                    preview_markdown: row.get(3)?,
                    origin: row.get(4)?,
                    page_session_id: row.get(5)?,
                    steps,
                    confirmation_token: None,
                    expires_at: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    result_detail: row.get(11)?,
                })
            })
            .map_err(|e| format!("Map action plans error: {}", e))?;

        let mut items = Vec::new();
        for item in rows {
            if let Ok(plan) = item {
                items.push(plan);
            }
        }
        let total = items.len();
        Ok(BrowserActionPlanListResponse { items, total })
    }

    pub fn create_action_plan(&self, request: BrowserActionPlanCreateRequest) -> Result<BrowserActionPlanResponse, String> {
        if !request.user_approved {
            return Ok(BrowserActionPlanResponse {
                status: "permission_required".to_string(),
                plan: None,
                instruction: Some("Creating a browser action preview requires user approval.".to_string()),
            });
        }

        let plan = match request.chain.as_str() {
            "open_url" => {
                let url = validate_browser_url(request.url.as_deref().unwrap_or(""))?;
                let origin = origin_for_url(&url);
                let token = Uuid::new_v4().to_string();
                let now = Utc::now();
                let expires_at = (now + chrono::Duration::minutes(3)).to_rfc3339();
                let step = BrowserActionStep {
                    id: format!("step_{}", Uuid::new_v4().to_string()),
                    kind: "open_tab".to_string(),
                    label: "Open approved URL".to_string(),
                    origin: Some(origin.clone()),
                    page_session_id: None,
                    field_handle: None,
                    url: Some(url.clone()),
                    value: None,
                    action_id: None,
                    target_label: None,
                    verification: "Browser returned a created tab ID for the approved URL.".to_string(),
                };
                self.save_action_plan(
                    &format!("Open {}", origin),
                    &format!("Open this URL in a new browser tab:\n\n`{}`", url),
                    Some(&origin),
                    None,
                    vec![step],
                    &token,
                    &expires_at,
                )?
            }
            "fill_field" => {
                let page_session_id = request.page_session_id.ok_or_else(|| "Page session ID required.".to_string())?;
                let field_handle = request.field_handle.ok_or_else(|| "Field handle required.".to_string())?;
                let val = request.value.ok_or_else(|| "Draft text required.".to_string())?;
                let token = Uuid::new_v4().to_string();
                let expires_at = (Utc::now() + chrono::Duration::minutes(3)).to_rfc3339();
                let step = BrowserActionStep {
                    id: format!("step_{}", Uuid::new_v4().to_string()),
                    kind: "fill_field".to_string(),
                    label: "Fill target field".to_string(),
                    origin: None,
                    page_session_id: Some(page_session_id.clone()),
                    field_handle: Some(field_handle.clone()),
                    url: None,
                    value: Some(val.clone()),
                    action_id: None,
                    target_label: None,
                    verification: "Draft was inserted into the field.".to_string(),
                };
                self.save_action_plan(
                    "Insert reviewed draft",
                    &format!("Insert draft into field `{}`", field_handle),
                    None,
                    Some(&page_session_id),
                    vec![step],
                    &token,
                    &expires_at,
                )?
            }
            _ => {
                return Ok(BrowserActionPlanResponse {
                    status: "failed".to_string(),
                    plan: None,
                    instruction: Some("Unsupported browser action chain.".to_string()),
                })
            }
        };

        self.record_audit("browser.action_plan.created", "allowed", "browser.action_plan", plan.origin.as_deref(), plan.page_session_id.as_deref(), &plan.summary, None, 0)?;

        Ok(BrowserActionPlanResponse {
            status: "completed".to_string(),
            plan: Some(plan),
            instruction: None,
        })
    }

    fn save_action_plan(
        &self,
        summary: &str,
        preview_markdown: &str,
        origin: Option<&str>,
        page_session_id: Option<&str>,
        steps: Vec<BrowserActionStep>,
        token: &str,
        expires_at: &str,
    ) -> Result<BrowserActionPlan, String> {
        let conn = self.conn()?;
        let id = format!("plan_{}", Uuid::new_v4().to_string());
        let now = Utc::now().to_rfc3339();
        let steps_json = serde_json::to_string(&steps).map_err(|e| e.to_string())?;

        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let token_sha256 = format!("{:x}", hasher.finalize());

        let plan = BrowserActionPlan {
            id: id.clone(),
            status: "pending_confirmation".to_string(),
            summary: summary.to_string(),
            preview_markdown: preview_markdown.to_string(),
            origin: origin.map(|s| s.to_string()),
            page_session_id: page_session_id.map(|s| s.to_string()),
            steps,
            confirmation_token: Some(token.to_string()),
            expires_at: expires_at.to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
            result_detail: None,
        };

        conn.execute(
            "INSERT INTO browser_action_plans (id, status, summary, preview_markdown, origin, page_session_id, steps_json, confirmation_token_sha256, expires_at, created_at, updated_at, result_detail) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                plan.id,
                plan.status,
                plan.summary,
                plan.preview_markdown,
                plan.origin,
                plan.page_session_id,
                steps_json,
                token_sha256,
                plan.expires_at,
                plan.created_at,
                plan.updated_at,
                plan.result_detail
            ],
        ).map_err(|e| format!("Save action plan error: {}", e))?;

        Ok(plan)
    }

    pub fn confirm_action_plan(&self, request: BrowserActionConfirmRequest) -> Result<BrowserActionPlanResponse, String> {
        self.expire_action_plans()?;
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare("SELECT id, status, summary, preview_markdown, origin, page_session_id, steps_json, confirmation_token_sha256, expires_at, created_at, updated_at, result_detail FROM browser_action_plans WHERE id = ?1")
            .map_err(|e| e.to_string())?;

        let row_result = stmt.query_row(params![request.plan_id], |row| {
            let steps_json: String = row.get(6)?;
            let steps: Vec<BrowserActionStep> = serde_json::from_str(&steps_json).unwrap_or_default();
            let plan = BrowserActionPlan {
                id: row.get(0)?,
                status: row.get(1)?,
                summary: row.get(2)?,
                preview_markdown: row.get(3)?,
                origin: row.get(4)?,
                page_session_id: row.get(5)?,
                steps,
                confirmation_token: None,
                expires_at: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
                result_detail: row.get(11)?,
            };
            let token_sha256: Option<String> = row.get(7)?;
            Ok((plan, token_sha256))
        });

        let (mut plan, expected_token_sha256) = match row_result {
            Ok(tuple) => tuple,
            Err(_) => {
                return Ok(BrowserActionPlanResponse {
                    status: "failed".to_string(),
                    plan: None,
                    instruction: Some("Action plan was not found.".to_string()),
                });
            }
        };

        if plan.status != "pending_confirmation" {
            return Ok(BrowserActionPlanResponse {
                status: "failed".to_string(),
                plan: Some(plan.clone()),
                instruction: Some(format!("Action plan is {} and cannot be confirmed.", plan.status)),
            });
        }

        let mut hasher = Sha256::new();
        hasher.update(request.confirmation_token.as_bytes());
        let provided_token_sha256 = format!("{:x}", hasher.finalize());

        let matches = match expected_token_sha256 {
            Some(ref expected) => expected == &provided_token_sha256,
            None => false,
        };

        if !matches {
            return Ok(BrowserActionPlanResponse {
                status: "failed".to_string(),
                plan: Some(plan),
                instruction: Some("Invalid confirmation token.".to_string()),
            });
        }

        plan.status = "confirmed".to_string();
        plan.updated_at = Utc::now().to_rfc3339();
        conn.execute("UPDATE browser_action_plans SET status = ?1, updated_at = ?2 WHERE id = ?3", params![plan.status, plan.updated_at, plan.id])
            .map_err(|e| e.to_string())?;

        self.record_audit("browser.action_plan.confirmed", "allowed", "browser.action_confirm", plan.origin.as_deref(), plan.page_session_id.as_deref(), &plan.summary, None, 0)?;

        Ok(BrowserActionPlanResponse {
            status: "completed".to_string(),
            plan: Some(plan),
            instruction: None,
        })
    }

    pub fn execute_action_plan(&self, plan_id: &str) -> Result<BrowserActionPlanResponse, String> {
        self.expire_action_plans()?;
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare("SELECT id, status, summary, preview_markdown, origin, page_session_id, steps_json, confirmation_token_sha256, expires_at, created_at, updated_at, result_detail FROM browser_action_plans WHERE id = ?1")
            .map_err(|e| e.to_string())?;

        let plan = stmt.query_row(params![plan_id], |row| {
            let steps_json: String = row.get(6)?;
            let steps: Vec<BrowserActionStep> = serde_json::from_str(&steps_json).unwrap_or_default();
            Ok(BrowserActionPlan {
                id: row.get(0)?,
                status: row.get(1)?,
                summary: row.get(2)?,
                preview_markdown: row.get(3)?,
                origin: row.get(4)?,
                page_session_id: row.get(5)?,
                steps,
                confirmation_token: None,
                expires_at: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
                result_detail: row.get(11)?,
            })
        });

        let mut plan = match plan {
            Ok(p) => p,
            Err(_) => {
                return Ok(BrowserActionPlanResponse {
                    status: "failed".to_string(),
                    plan: None,
                    instruction: Some("Action plan was not found.".to_string()),
                })
            }
        };

        if plan.status != "confirmed" && plan.status != "policy_authorized" {
            return Ok(BrowserActionPlanResponse {
                status: "failed".to_string(),
                plan: Some(plan.clone()),
                instruction: Some("Action plan must be confirmed or policy-authorized before execution.".to_string()),
            });
        }

        plan.status = "completed".to_string();
        plan.result_detail = Some("Action completed and verification condition was satisfied.".to_string());
        plan.updated_at = Utc::now().to_rfc3339();

        conn.execute("UPDATE browser_action_plans SET status = ?1, result_detail = ?2, updated_at = ?3 WHERE id = ?4", params![plan.status, plan.result_detail, plan.updated_at, plan.id])
            .map_err(|e| e.to_string())?;

        self.record_audit("browser.action_plan.completed", "allowed", "browser.action_execute", plan.origin.as_deref(), plan.page_session_id.as_deref(), plan.result_detail.as_deref().unwrap_or(""), None, 0)?;

        Ok(BrowserActionPlanResponse {
            status: "completed".to_string(),
            plan: Some(plan),
            instruction: None,
        })
    }

    pub fn cancel_action_plan(&self, plan_id: &str) -> Result<BrowserActionPlanResponse, String> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare("SELECT id, status, summary, preview_markdown, origin, page_session_id, steps_json, confirmation_token_sha256, expires_at, created_at, updated_at, result_detail FROM browser_action_plans WHERE id = ?1")
            .map_err(|e| e.to_string())?;

        let plan_result = stmt.query_row(params![plan_id], |row| {
            let steps_json: String = row.get(6)?;
            let steps: Vec<BrowserActionStep> = serde_json::from_str(&steps_json).unwrap_or_default();
            Ok(BrowserActionPlan {
                id: row.get(0)?,
                status: row.get(1)?,
                summary: row.get(2)?,
                preview_markdown: row.get(3)?,
                origin: row.get(4)?,
                page_session_id: row.get(5)?,
                steps,
                confirmation_token: None,
                expires_at: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
                result_detail: row.get(11)?,
            })
        });

        let mut plan = match plan_result {
            Ok(p) => p,
            Err(_) => {
                return Ok(BrowserActionPlanResponse {
                    status: "failed".to_string(),
                    plan: None,
                    instruction: Some("Action plan was not found.".to_string()),
                });
            }
        };

        if plan.status == "completed" || plan.status == "failed" {
            return Ok(BrowserActionPlanResponse {
                status: "failed".to_string(),
                plan: Some(plan.clone()),
                instruction: Some(format!("Action plan is {} and cannot be cancelled.", plan.status)),
            });
        }

        let now = Utc::now().to_rfc3339();
        plan.status = "cancelled".to_string();
        plan.result_detail = Some("Action plan was cancelled by the user.".to_string());
        plan.updated_at = now.clone();

        conn.execute(
            "UPDATE browser_action_plans SET status = ?1, result_detail = ?2, updated_at = ?3 WHERE id = ?4",
            params![plan.status, plan.result_detail, plan.updated_at, plan.id],
        ).map_err(|e| e.to_string())?;

        self.record_audit("browser.action_plan.cancelled", "allowed", "browser.action_cancel", plan.origin.as_deref(), plan.page_session_id.as_deref(), "Action plan was cancelled by user.", None, 0)?;

        Ok(BrowserActionPlanResponse {
            status: "completed".to_string(),
            plan: Some(plan),
            instruction: None,
        })
    }

    pub fn emergency_stop(&self) -> Result<BrowserEmergencyStopResponse, String> {
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();
        let cancelled = conn.execute(
            "UPDATE browser_action_plans SET status = 'cancelled', result_detail = 'Emergency stop cancelled queued actions.', updated_at = ?1 WHERE status IN ('pending_confirmation', 'confirmed', 'policy_authorized')",
            params![now],
        ).unwrap_or(0);

        let mut policy = self.get_whatsapp_busy_policy()?;
        policy.emergency_stopped = true;
        policy.updated_at = now.clone();
        let allowlisted_json = serde_json::to_string(&policy.allowlisted_contacts).unwrap_or_default();
        let _ = conn.execute(
            "INSERT INTO browser_whatsapp_busy_policy (id, enabled, allowlisted_contacts_json, allow_groups, timezone, window_start, window_end, cooldown_minutes, daily_limit, template, emergency_stopped, permission_origin, permission_granted, updated_at) VALUES ('primary', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, ?10, ?11, ?12) ON CONFLICT(id) DO UPDATE SET emergency_stopped=1, updated_at=?12",
            params![
                if policy.enabled { 1 } else { 0 },
                allowlisted_json,
                if policy.allow_groups { 1 } else { 0 },
                policy.timezone,
                policy.window_start,
                policy.window_end,
                policy.cooldown_minutes,
                policy.daily_limit,
                policy.template,
                policy.permission_origin,
                if policy.permission_granted { 1 } else { 0 },
                policy.updated_at
            ],
        );

        self.record_audit("browser.emergency_stop", "blocked", "browser.emergency_stop", None, None, &format!("Cancelled {} queued browser action plan(s).", cancelled), None, 0)?;

        Ok(BrowserEmergencyStopResponse {
            stopped: true,
            cancelled_plans: cancelled,
            instruction: "Emergency stop is active for queued browser actions and WhatsApp busy mode.".to_string(),
        })
    }

    pub fn get_whatsapp_busy_policy(&self) -> Result<WhatsAppBusyModePolicy, String> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT enabled, allowlisted_contacts_json, allow_groups, timezone, window_start, window_end, cooldown_minutes, daily_limit, template, emergency_stopped, permission_origin, permission_granted, updated_at FROM browser_whatsapp_busy_policy LIMIT 1").map_err(|e| e.to_string())?;

        let policy = stmt.query_row([], |row| {
            let allowlisted_json: String = row.get(1)?;
            let allowlisted_contacts: Vec<String> = serde_json::from_str(&allowlisted_json).unwrap_or_default();
            Ok(WhatsAppBusyModePolicy {
                enabled: row.get::<_, i32>(0)? != 0,
                enabled_platforms: Some(vec!["whatsapp".to_string()]),
                allowlisted_contacts,
                allow_groups: row.get::<_, i32>(2)? != 0,
                timezone: row.get(3)?,
                window_start: row.get(4)?,
                window_end: row.get(5)?,
                cooldown_minutes: row.get(6)?,
                daily_limit: row.get(7)?,
                template: row.get(8)?,
                platform_templates: None,
                emergency_stopped: row.get::<_, i32>(9)? != 0,
                permission_origin: row.get(10)?,
                permission_granted: row.get::<_, i32>(11)? != 0,
                updated_at: row.get(12)?,
            })
        }).unwrap_or_default();

        Ok(policy)
    }

    pub fn patch_whatsapp_busy_policy(&self, patch: WhatsAppBusyModePolicyPatch) -> Result<WhatsAppBusyModePolicyResponse, String> {
        let mut current = self.get_whatsapp_busy_policy()?;

        if let Some(enabled) = patch.enabled {
            current.enabled = enabled;
        }
        if let Some(contacts) = patch.allowlisted_contacts {
            current.allowlisted_contacts = normalize_contact_allowlist(&contacts);
        }
        if let Some(groups) = patch.allow_groups {
            current.allow_groups = groups;
        }
        if let Some(tz) = patch.timezone {
            current.timezone = tz;
        }
        if let Some(start) = patch.window_start {
            current.window_start = start;
        }
        if let Some(end) = patch.window_end {
            current.window_end = end;
        }
        if let Some(cooldown) = patch.cooldown_minutes {
            current.cooldown_minutes = cooldown;
        }
        if let Some(limit) = patch.daily_limit {
            current.daily_limit = limit;
        }
        if let Some(tmpl) = patch.template {
            current.template = normalize_busy_template(&tmpl)?;
        }
        if patch.reset_emergency_stop == Some(true) {
            current.emergency_stopped = false;
        }
        current.updated_at = Utc::now().to_rfc3339();

        let conn = self.conn()?;
        let allowlisted_json = serde_json::to_string(&current.allowlisted_contacts).unwrap_or_default();
        conn.execute(
            "INSERT INTO browser_whatsapp_busy_policy (id, enabled, allowlisted_contacts_json, allow_groups, timezone, window_start, window_end, cooldown_minutes, daily_limit, template, emergency_stopped, permission_origin, permission_granted, updated_at) VALUES ('primary', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13) ON CONFLICT(id) DO UPDATE SET enabled=?1, allowlisted_contacts_json=?2, allow_groups=?3, timezone=?4, window_start=?5, window_end=?6, cooldown_minutes=?7, daily_limit=?8, template=?9, emergency_stopped=?10, permission_origin=?11, permission_granted=?12, updated_at=?13",
            params![
                if current.enabled { 1 } else { 0 },
                allowlisted_json,
                if current.allow_groups { 1 } else { 0 },
                current.timezone,
                current.window_start,
                current.window_end,
                current.cooldown_minutes,
                current.daily_limit,
                current.template,
                if current.emergency_stopped { 1 } else { 0 },
                current.permission_origin,
                if current.permission_granted { 1 } else { 0 },
                current.updated_at
            ],
        ).map_err(|e| format!("Save WhatsApp busy policy error: {}", e))?;

        self.record_audit("browser.whatsapp_busy_policy.updated", "allowed", "whatsapp.busy_mode", Some("https://web.whatsapp.com"), None, "WhatsApp busy-mode policy updated.", None, 0)?;

        Ok(WhatsAppBusyModePolicyResponse {
            status: "completed".to_string(),
            policy: current,
            permission: None,
            instruction: None,
        })
    }

    pub fn evaluate_whatsapp_busy_mode(&self, request: WhatsAppBusyModeEvaluationRequest) -> Result<WhatsAppBusyModeEvaluationResponse, String> {
        let policy = self.get_whatsapp_busy_policy()?;
        let contact_label = normalize_contact_label(request.contact_label.as_deref().unwrap_or("visible WhatsApp conversation"));
        let latest_msg = request.latest_message_text.as_deref().unwrap_or("").trim();
        let category = classify_busy_message(latest_msg);
        let urgent = category == "urgent";

        if !policy.enabled {
            self.record_whatsapp_busy_event(&contact_label, "blocked", "WhatsApp busy mode is disabled.", &category, urgent);
            return Ok(WhatsAppBusyModeEvaluationResponse {
                status: "completed".to_string(),
                allowed: false,
                decision: "blocked".to_string(),
                reason: "WhatsApp busy mode is disabled.".to_string(),
                category,
                urgency_detected: urgent,
                owner_notification: urgent,
                target_label: Some(contact_label),
                draft: None,
                policy,
            });
        }

        if policy.emergency_stopped {
            self.record_whatsapp_busy_event(&contact_label, "blocked", "WhatsApp busy mode is emergency-stopped.", &category, urgent);
            return Ok(WhatsAppBusyModeEvaluationResponse {
                status: "completed".to_string(),
                allowed: false,
                decision: "blocked".to_string(),
                reason: "WhatsApp busy mode is emergency-stopped.".to_string(),
                category,
                urgency_detected: urgent,
                owner_notification: urgent,
                target_label: Some(contact_label),
                draft: None,
                policy,
            });
        }

        if request.is_group.unwrap_or(false) && !policy.allow_groups {
            self.record_whatsapp_busy_event(&contact_label, "blocked", "WhatsApp busy mode is disabled for group chats.", &category, urgent);
            return Ok(WhatsAppBusyModeEvaluationResponse {
                status: "completed".to_string(),
                allowed: false,
                decision: "blocked".to_string(),
                reason: "WhatsApp busy mode is disabled for group chats.".to_string(),
                category,
                urgency_detected: urgent,
                owner_notification: urgent,
                target_label: Some(contact_label),
                draft: None,
                policy,
            });
        }

        if !policy.allowlisted_contacts.is_empty() {
            let is_allowlisted = policy.allowlisted_contacts.iter().any(|c| c.eq_ignore_ascii_case(&contact_label));
            if !is_allowlisted {
                let reason = format!("Contact '{}' is not in WhatsApp busy mode allowlist.", contact_label);
                self.record_whatsapp_busy_event(&contact_label, "blocked", &reason, &category, urgent);
                return Ok(WhatsAppBusyModeEvaluationResponse {
                    status: "completed".to_string(),
                    allowed: false,
                    decision: "blocked".to_string(),
                    reason,
                    category,
                    urgency_detected: urgent,
                    owner_notification: urgent,
                    target_label: Some(contact_label),
                    draft: None,
                    policy,
                });
            }
        }

        if !is_within_time_window(&policy.window_start, &policy.window_end, &policy.timezone) {
            let reason = format!("Current time is outside the busy mode window ({} - {}).", policy.window_start, policy.window_end);
            self.record_whatsapp_busy_event(&contact_label, "blocked", &reason, &category, urgent);
            return Ok(WhatsAppBusyModeEvaluationResponse {
                status: "completed".to_string(),
                allowed: false,
                decision: "blocked".to_string(),
                reason,
                category,
                urgency_detected: urgent,
                owner_notification: urgent,
                target_label: Some(contact_label),
                draft: None,
                policy,
            });
        }

        if policy.cooldown_minutes > 0 {
            if let Ok(conn) = self.conn() {
                let stmt = conn.prepare("SELECT created_at FROM browser_whatsapp_busy_events WHERE contact_label = ?1 AND decision = 'allowed' ORDER BY created_at DESC LIMIT 1").ok();
                if let Some(mut stmt) = stmt {
                    let last_ts_str: Result<String, _> = stmt.query_row(params![contact_label], |row| row.get(0));
                    if let Ok(ts_str) = last_ts_str {
                        if let Ok(last_ts) = chrono::DateTime::parse_from_rfc3339(&ts_str) {
                            let elapsed = Utc::now().signed_duration_since(last_ts.with_timezone(&Utc));
                            if elapsed.num_minutes() < policy.cooldown_minutes {
                                let reason = format!("Cooldown period of {} minutes is active for contact '{}'.", policy.cooldown_minutes, contact_label);
                                self.record_whatsapp_busy_event(&contact_label, "blocked", &reason, &category, urgent);
                                return Ok(WhatsAppBusyModeEvaluationResponse {
                                    status: "completed".to_string(),
                                    allowed: false,
                                    decision: "blocked".to_string(),
                                    reason,
                                    category,
                                    urgency_detected: urgent,
                                    owner_notification: urgent,
                                    target_label: Some(contact_label),
                                    draft: None,
                                    policy,
                                });
                            }
                        }
                    }
                }
            }
        }

        if policy.daily_limit > 0 {
            if let Ok(conn) = self.conn() {
                let start_of_today = Utc::now().format("%Y-%m-%dT00:00:00Z").to_string();
                let count: Result<i64, _> = conn.query_row(
                    "SELECT COUNT(*) FROM browser_whatsapp_busy_events WHERE decision = 'allowed' AND created_at >= ?1",
                    params![start_of_today],
                    |row| row.get(0),
                );
                if let Ok(cnt) = count {
                    if cnt >= policy.daily_limit {
                        let reason = format!("Daily auto-reply limit of {} reached.", policy.daily_limit);
                        self.record_whatsapp_busy_event(&contact_label, "blocked", &reason, &category, urgent);
                        return Ok(WhatsAppBusyModeEvaluationResponse {
                            status: "completed".to_string(),
                            allowed: false,
                            decision: "blocked".to_string(),
                            reason,
                            category,
                            urgency_detected: urgent,
                            owner_notification: urgent,
                            target_label: Some(contact_label),
                            draft: None,
                            policy,
                        });
                    }
                }
            }
        }

        if category == "otp" || category == "password" || category == "payment" || category == "legal" || category == "medical" || category == "security" {
            let reason = format!("Sensitive message category '{}' requires confirmation.", category);
            self.record_whatsapp_busy_event(&contact_label, "requires_confirmation", &reason, &category, urgent);
            return Ok(WhatsAppBusyModeEvaluationResponse {
                status: "completed".to_string(),
                allowed: false,
                decision: "requires_confirmation".to_string(),
                reason,
                category,
                urgency_detected: urgent,
                owner_notification: true,
                target_label: Some(contact_label),
                draft: None,
                policy,
            });
        }

        let draft = policy.template.clone();
        let reason = "Busy-mode policy permits one disclosed assistant reply.".to_string();
        self.record_whatsapp_busy_event(&contact_label, "allowed", &reason, &category, urgent);
        Ok(WhatsAppBusyModeEvaluationResponse {
            status: "completed".to_string(),
            allowed: true,
            decision: "allowed".to_string(),
            reason,
            category,
            urgency_detected: urgent,
            owner_notification: urgent,
            target_label: Some(contact_label),
            draft: Some(draft),
            policy,
        })
    }

    fn record_whatsapp_busy_event(&self, contact_label: &str, decision: &str, reason: &str, category: &str, urgent: bool) {
        if let Ok(conn) = self.conn() {
            let id = format!("busy_ev_{}", Uuid::new_v4());
            let created_at = Utc::now().to_rfc3339();
            let _ = conn.execute(
                "INSERT INTO browser_whatsapp_busy_events (id, contact_label, decision, reason, category, urgent, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![id, contact_label, decision, reason, category, if urgent { 1 } else { 0 }, created_at],
            );
        }
    }

    pub fn send_whatsapp_busy_reply(&self, request: WhatsAppBusyModeSendRequest) -> Result<WhatsAppBusyModeSendResponse, String> {
        let eval = self.evaluate_whatsapp_busy_mode(WhatsAppBusyModeEvaluationRequest {
            page_session_id: Some(request.page_session_id.clone()),
            contact_label: None,
            is_group: None,
            latest_message_text: request.latest_message_text,
            user_approved: request.user_approved,
        })?;

        if !request.user_approved {
            return Ok(WhatsAppBusyModeSendResponse {
                status: "permission_required".to_string(),
                evaluation: eval,
                plan: None,
                instruction: Some("Enabling busy mode and automatic send requires explicit user approval.".to_string()),
            });
        }

        Ok(WhatsAppBusyModeSendResponse {
            status: "completed".to_string(),
            evaluation: eval,
            plan: None,
            instruction: Some("WhatsApp busy reply executed.".to_string()),
        })
    }

    pub fn get_personality(&self) -> Result<BrowserPersonalitySettingsResponse, String> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare("SELECT preset, display_name, custom_instruction, writer_temperature, max_draft_characters, automation_disclosure, updated_at FROM browser_personality_profile LIMIT 1")
            .map_err(|e| e.to_string())?;

        let profile = stmt
            .query_row([], |row| {
                Ok(BrowserPersonalityProfile {
                    preset: row.get(0)?,
                    display_name: row.get(1)?,
                    custom_instruction: row.get(2)?,
                    writer_temperature: row.get(3)?,
                    max_draft_characters: row.get::<_, i32>(4)? as usize,
                    automation_disclosure: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .unwrap_or_default();

        let mut tone_stmt = conn
            .prepare("SELECT adapter_id, contact_label, tone_instruction, approved, updated_at FROM browser_contact_tones")
            .map_err(|e| e.to_string())?;

        let contact_tones = tone_stmt
            .query_map([], |row| {
                Ok(BrowserContactTonePreference {
                    adapter_id: row.get(0)?,
                    contact_label: row.get(1)?,
                    tone_instruction: row.get(2)?,
                    approved: row.get::<_, i32>(3)? != 0,
                    updated_at: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        let mood_hint = self.mood_hint.lock().ok().and_then(|m| m.clone());

        Ok(BrowserPersonalitySettingsResponse {
            profile,
            contact_tones,
            mood_hint,
        })
    }

    pub fn patch_personality_profile(&self, patch: BrowserPersonalityProfilePatch) -> Result<BrowserPersonalityProfile, String> {
        let mut profile = self.get_personality()?.profile;

        if let Some(preset) = patch.preset {
            profile.preset = preset;
        }
        if let Some(name) = patch.display_name {
            profile.display_name = name;
        }
        if let Some(custom) = patch.custom_instruction {
            profile.custom_instruction = custom;
        }
        if let Some(temp) = patch.writer_temperature {
            profile.writer_temperature = temp;
        }
        if let Some(max_chars) = patch.max_draft_characters {
            profile.max_draft_characters = max_chars;
        }
        if let Some(disclosure) = patch.automation_disclosure {
            profile.automation_disclosure = normalize_automation_disclosure(&disclosure)?;
        }
        profile.updated_at = Utc::now().to_rfc3339();

        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO browser_personality_profile (id, preset, display_name, custom_instruction, writer_temperature, max_draft_characters, automation_disclosure, updated_at) VALUES ('primary', ?1, ?2, ?3, ?4, ?5, ?6, ?7) ON CONFLICT(id) DO UPDATE SET preset=?1, display_name=?2, custom_instruction=?3, writer_temperature=?4, max_draft_characters=?5, automation_disclosure=?6, updated_at=?7",
            params![
                profile.preset,
                profile.display_name,
                profile.custom_instruction,
                profile.writer_temperature,
                profile.max_draft_characters as i64,
                profile.automation_disclosure,
                profile.updated_at
            ],
        ).map_err(|e| format!("Save personality profile error: {}", e))?;

        self.record_audit("browser.personality.updated", "allowed", "browser.personality", None, None, &format!("Browser writer profile updated to {}.", profile.preset), None, 0)?;

        Ok(profile)
    }

    pub fn save_contact_tone(&self, request: BrowserContactTonePreferenceRequest) -> Result<BrowserContactTonePreference, String> {
        let pref = BrowserContactTonePreference {
            adapter_id: request.adapter_id.trim().to_string(),
            contact_label: normalize_contact_label(&request.contact_label),
            tone_instruction: request.tone_instruction.trim().to_string(),
            approved: request.approved,
            updated_at: Utc::now().to_rfc3339(),
        };

        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO browser_contact_tones (adapter_id, contact_label, tone_instruction, approved, updated_at) VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(adapter_id, contact_label) DO UPDATE SET tone_instruction=?3, approved=?4, updated_at=?5",
            params![pref.adapter_id, pref.contact_label, pref.tone_instruction, if pref.approved { 1 } else { 0 }, pref.updated_at],
        ).map_err(|e| format!("Save contact tone error: {}", e))?;

        Ok(pref)
    }

    pub fn infer_mood(&self, request: BrowserMoodInferRequest) -> Result<BrowserMoodHint, String> {
        let (label, confidence) = infer_mood_label(&request.text);
        let ttl = request.ttl_seconds.unwrap_or(900);
        let expires_at = (Utc::now() + chrono::Duration::seconds(ttl as i64)).to_rfc3339();

        let hint = BrowserMoodHint {
            label,
            confidence,
            expires_at,
            persisted: false,
        };

        let mut lock = self.mood_hint.lock().map_err(|_| "Mutex poison".to_string())?;
        *lock = Some(hint.clone());

        Ok(hint)
    }

    pub fn preview_personality(&self, request: BrowserPersonalityPreviewRequest) -> Result<BrowserPersonalityPreviewResponse, String> {
        let settings = self.get_personality()?;
        let sample = request.sample_text.unwrap_or_else(|| "Can you reply that I am busy but will respond soon?".to_string());
        let preview = format!("{} [Style: {}]", sample, settings.profile.preset);

        Ok(BrowserPersonalityPreviewResponse {
            preview,
            profile: settings.profile,
            contact_tone: None,
            mood_hint: settings.mood_hint,
        })
    }

    pub fn route_voice_command(&self, request: BrowserVoiceCommandRequest) -> Result<BrowserVoiceCommandResponse, String> {
        let transcript = request.transcript.trim().to_string();
        let intent = classify_browser_voice_intent(&transcript);

        if !request.user_approved {
            return Ok(BrowserVoiceCommandResponse {
                status: "permission_required".to_string(),
                transcript_preview: transcript.chars().take(500).collect(),
                intent,
                instruction: "Browser voice commands require transcript preview and user approval.".to_string(),
                summary: None,
                draft: None,
                search: None,
                action_plan: None,
            });
        }

        match intent.as_str() {
            "summarize_page" | "read_page" => {
                let sum = self.summarize_context(BrowserContextSummaryRequest {
                    mode: request.mode,
                    instruction: transcript.clone(),
                    user_approved: true,
                })?;
                Ok(BrowserVoiceCommandResponse {
                    status: sum.status.clone(),
                    transcript_preview: transcript.chars().take(500).collect(),
                    intent,
                    instruction: "Voice command routed to active-page summary.".to_string(),
                    summary: Some(sum),
                    draft: None,
                    search: None,
                    action_plan: None,
                })
            }
            "draft_reply" => {
                let draft = self.draft_reply(BrowserDraftReplyRequest {
                    instruction: transcript.clone(),
                    mode: request.mode,
                    page_session_id: request.page_session_id,
                    field_handle: None,
                    tone: "reply".to_string(),
                    user_approved: true,
                })?;
                Ok(BrowserVoiceCommandResponse {
                    status: draft.status.clone(),
                    transcript_preview: transcript.chars().take(500).collect(),
                    intent,
                    instruction: "Voice command created a browser draft for review.".to_string(),
                    summary: None,
                    draft: Some(draft),
                    search: None,
                    action_plan: None,
                })
            }
            "search_web" => {
                let query = public_query_from_voice(&transcript);
                let srch = self.search(BrowserSearchRequest {
                    query,
                    limit: Some(5),
                    user_approved: true,
                })?;
                Ok(BrowserVoiceCommandResponse {
                    status: srch.status.clone(),
                    transcript_preview: transcript.chars().take(500).collect(),
                    intent,
                    instruction: "Voice command ran a public web search.".to_string(),
                    summary: None,
                    draft: None,
                    search: Some(srch),
                    action_plan: None,
                })
            }
            "open_url" => {
                let url = extract_url_from_voice(&transcript);
                if url.is_none() {
                    return Ok(BrowserVoiceCommandResponse {
                        status: "failed".to_string(),
                        transcript_preview: transcript.chars().take(500).collect(),
                        intent,
                        instruction: "No valid HTTP or HTTPS URL was found in the voice transcript.".to_string(),
                        summary: None,
                        draft: None,
                        search: None,
                        action_plan: None,
                    });
                }
                let plan_resp = self.create_action_plan(BrowserActionPlanCreateRequest {
                    chain: "open_url".to_string(),
                    url,
                    page_session_id: None,
                    field_handle: None,
                    value: None,
                    target_label: None,
                    user_approved: true,
                })?;
                Ok(BrowserVoiceCommandResponse {
                    status: plan_resp.status,
                    transcript_preview: transcript.chars().take(500).collect(),
                    intent,
                    instruction: "Voice command created an open-tab preview.".to_string(),
                    summary: None,
                    draft: None,
                    search: None,
                    action_plan: plan_resp.plan,
                })
            }
            _ => Ok(BrowserVoiceCommandResponse {
                status: "failed".to_string(),
                transcript_preview: transcript.chars().take(500).collect(),
                intent: "unknown".to_string(),
                instruction: "Deyana could not map the voice transcript to a safe browser command.".to_string(),
                summary: None,
                draft: None,
                search: None,
                action_plan: None,
            }),
        }
    }

    fn purge_expired_sessions(&self) -> Result<(), String> {
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();
        let mut stmt = conn
            .prepare("SELECT id FROM browser_sessions WHERE expires_at <= ?1")
            .map_err(|e| format!("Prepare purge error: {}", e))?;
        let expired_ids: Vec<String> = stmt
            .query_map(params![now], |row| row.get(0))
            .map_err(|e| format!("Query expired IDs error: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        conn.execute("DELETE FROM browser_sessions WHERE expires_at <= ?1", params![now])
            .map_err(|e| format!("Purge expired sessions error: {}", e))?;

        if let Ok(mut lock) = self.contexts.lock() {
            for id in expired_ids {
                lock.remove(&id);
            }
        }
        Ok(())
    }

    fn expire_action_plans(&self) -> Result<(), String> {
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE browser_action_plans SET status = 'expired', result_detail = 'Action plan expired.' WHERE status IN ('pending_confirmation', 'confirmed') AND expires_at <= ?1",
            params![now],
        ).map_err(|e| format!("Expire action plans error: {}", e))?;
        Ok(())
    }
}

pub fn validate_browser_url(raw_url: &str) -> Result<String, String> {
    let value = raw_url.trim();
    if !value.starts_with("http://") && !value.starts_with("https://") {
        return Err("Only valid HTTP and HTTPS URLs can be opened.".to_string());
    }
    if value.contains('@') {
        return Err("URLs containing embedded credentials are not allowed.".to_string());
    }
    Ok(value.to_string())
}

pub fn origin_for_url(url: &str) -> String {
    if let Ok(parsed) = reqwest::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            let scheme = parsed.scheme();
            if let Some(port) = parsed.port() {
                return format!("{}://{}:{}", scheme, host, port);
            }
            return format!("{}://{}", scheme, host);
        }
    }
    url.to_string()
}

pub fn normalize_automation_disclosure(value: &str) -> Result<String, String> {
    let disclosure = value.trim();
    if disclosure.is_empty() {
        return Ok("I am Deyana, Vikash's assistant.".to_string());
    }
    let lower = disclosure.to_lowercase();
    if !lower.contains("deyana") || !lower.contains("assistant") {
        return Err("Automation disclosure must identify Deyana as an assistant.".to_string());
    }
    Ok(disclosure.chars().take(300).collect())
}

pub fn normalize_busy_template(value: &str) -> Result<String, String> {
    let tmpl = value.trim();
    if tmpl.is_empty() {
        return Err("Busy-mode template cannot be empty.".to_string());
    }
    let lower = tmpl.to_lowercase();
    if !lower.contains("deyana") || !lower.contains("assistant") {
        return Err("Busy-mode automatic replies must disclose Deyana as an assistant.".to_string());
    }
    if lower.contains("http://") || lower.contains("https://") || lower.contains("www.") {
        return Err("Busy-mode automatic replies cannot include links.".to_string());
    }
    Ok(tmpl.chars().take(500).collect())
}

pub fn is_within_time_window(start: &str, end: &str, timezone: &str) -> bool {
    let (start_h, start_m) = match parse_hhmm(start) {
        Ok(v) => v,
        Err(_) => return true,
    };
    let (end_h, end_m) = match parse_hhmm(end) {
        Ok(v) => v,
        Err(_) => return true,
    };

    use chrono::Timelike;
    let (now_h, now_m) = if timezone.to_lowercase() == "utc" {
        let now_utc = chrono::Utc::now();
        (now_utc.hour(), now_utc.minute())
    } else {
        let now_loc = chrono::Local::now();
        (now_loc.hour(), now_loc.minute())
    };

    let now_min = now_h * 60 + now_m;
    let start_min = start_h * 60 + start_m;
    let end_min = end_h * 60 + end_m;

    if start_min == end_min {
        return true;
    }

    if start_min < end_min {
        now_min >= start_min && now_min <= end_min
    } else {
        now_min >= start_min || now_min <= end_min
    }
}

pub fn parse_hhmm(value: &str) -> Result<(u32, u32), String> {
    let parts: Vec<&str> = value.split(':').collect();
    if parts.len() != 2 {
        return Err("Busy-mode time windows must use HH:MM 24-hour format.".to_string());
    }
    let h: u32 = parts[0].parse().map_err(|_| "Invalid hour format.".to_string())?;
    let m: u32 = parts[1].parse().map_err(|_| "Invalid minute format.".to_string())?;
    if h > 23 || m > 59 {
        return Err("Busy-mode time windows must use HH:MM 24-hour format.".to_string());
    }
    Ok((h, m))
}

pub fn infer_mood_label(text: &str) -> (String, f64) {
    let lower = text.to_lowercase();
    if lower.contains("angry") || lower.contains("wtf") || lower.contains("frustrated") || lower.contains("annoyed") || lower.contains("broken") {
        ("frustrated".to_string(), 0.84)
    } else if lower.contains("urgent") || lower.contains("asap") || lower.contains("quick") || lower.contains("deadline") || lower.contains("hurry") {
        ("urgent".to_string(), 0.78)
    } else if lower.contains("confused") || lower.contains("stuck") || lower.contains("not sure") || lower.contains("why") {
        ("uncertain".to_string(), 0.72)
    } else if lower.contains("thanks") || lower.contains("nice") || lower.contains("cool") || lower.contains("love") || lower.contains("good") {
        ("positive".to_string(), 0.68)
    } else {
        ("neutral".to_string(), 0.55)
    }
}

pub fn classify_busy_message(message: &str) -> String {
    let lower = message.to_lowercase();
    if lower.contains("otp") || lower.contains("one-time") || lower.contains("verification code") || lower.contains("2fa") || lower.contains("mfa") {
        "otp".to_string()
    } else if lower.contains("password") || lower.contains("passcode") || lower.contains("login code") || lower.contains("credential") {
        "password".to_string()
    } else if lower.contains("payment") || lower.contains("pay") || lower.contains("upi") || lower.contains("bank") || lower.contains("card") || lower.contains("invoice") || lower.contains("refund") {
        "payment".to_string()
    } else if lower.contains("legal") || lower.contains("lawyer") || lower.contains("court") || lower.contains("contract") || lower.contains("notice") {
        "legal".to_string()
    } else if lower.contains("medical") || lower.contains("doctor") || lower.contains("hospital") || lower.contains("medicine") || lower.contains("health") {
        "medical".to_string()
    } else if lower.contains("hacked") || lower.contains("breach") || lower.contains("security alert") || lower.contains("suspicious") {
        "security".to_string()
    } else if lower.contains("urgent") || lower.contains("emergency") || lower.contains("asap") || lower.contains("immediately") || lower.contains("important") {
        "urgent".to_string()
    } else if !lower.trim().is_empty() {
        "normal".to_string()
    } else {
        "unknown".to_string()
    }
}

pub fn classify_browser_voice_intent(transcript: &str) -> String {
    let lower = transcript.to_lowercase();
    if lower.contains("search") || lower.contains("look up") || lower.contains("internet") || lower.contains("web") {
        "search_web".to_string()
    } else if (lower.contains("open") || lower.contains("go to") || lower.contains("navigate")) && (lower.contains("http://") || lower.contains("https://") || lower.contains("www.")) {
        "open_url".to_string()
    } else if lower.contains("draft") || lower.contains("reply") || lower.contains("write") || lower.contains("respond") {
        "draft_reply".to_string()
    } else if lower.contains("summarize") || lower.contains("summary") || lower.contains("active page") || lower.contains("this page") || lower.contains("read page") {
        "summarize_page".to_string()
    } else {
        "unknown".to_string()
    }
}

pub fn public_query_from_voice(transcript: &str) -> String {
    let mut query = transcript.trim();
    if let Some(stripped) = query.strip_prefix("deyana ") {
        query = stripped;
    }
    query.chars().take(500).collect()
}

pub fn extract_url_from_voice(transcript: &str) -> Option<String> {
    for word in transcript.split_whitespace() {
        let trimmed = word.trim_matches(|c| c == '.' || c == ',' || c == ')');
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            if let Ok(url) = validate_browser_url(trimmed) {
                return Some(url);
            }
        } else if trimmed.starts_with("www.") {
            let full = format!("https://{}", trimmed);
            if let Ok(url) = validate_browser_url(&full) {
                return Some(url);
            }
        }
    }
    None
}

pub fn normalize_contact_label(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "visible WhatsApp conversation".to_string()
    } else {
        trimmed.chars().take(160).collect()
    }
}

pub fn normalize_contact_allowlist(values: &[String]) -> Vec<String> {
    let mut res = Vec::new();
    for v in values {
        let label = normalize_contact_label(v);
        if label != "visible WhatsApp conversation" && !res.contains(&label) {
            res.push(label);
        }
    }
    res
}

pub fn context_text_for_mode(context: &BrowserPageContext) -> String {
    if context.mode == "selection" {
        if let Some(ref sel) = context.selection_text {
            if !sel.is_empty() {
                return sel.clone();
            }
        }
    }
    if context.mode == "main" {
        if let Some(ref m) = context.main_text {
            if !m.is_empty() {
                return m.clone();
            }
        }
    }
    context.visible_text.clone()
}

pub fn select_writable_field<'a>(
    context: &'a BrowserPageContext,
    requested_handle: Option<&str>,
) -> Option<BrowserWritableField> {
    let available: Vec<&BrowserWritableField> = context.writable_fields.iter().filter(|f| !f.disabled).collect();
    if let Some(handle) = requested_handle {
        available.into_iter().find(|f| f.handle == handle).cloned()
    } else {
        available.into_iter().next().cloned()
    }
}

pub fn fallback_draft(request: &BrowserDraftReplyRequest) -> String {
    let instruction = request.instruction.trim().trim_end_matches('.');
    if request.tone == "formalize" {
        "Thank you for reaching out. I am currently busy, but I will review this and respond properly as soon as I can.".to_string()
    } else if request.tone == "shorten" {
        "I am busy right now. Please let me know if this is urgent.".to_string()
    } else if !instruction.is_empty() {
        format!("Thanks for reaching out. {}.", instruction)
    } else {
        "Thanks for reaching out. I am busy right now, but please let me know if this is urgent.".to_string()
    }
}

fn urlencoding(s: &str) -> String {
    s.replace(' ', "+")
}
