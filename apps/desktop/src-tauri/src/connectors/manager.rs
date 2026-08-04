use std::env;
use chrono::{Duration, Utc};
use rusqlite::{params, Connection};
use serde_json::json;
use uuid::Uuid;

use crate::db::DbPool;
use super::storage;
use super::types::*;

#[derive(Debug, Clone)]
pub struct ConnectorDef {
    pub id: &'static str,
    pub name: &'static str,
    pub scopes: &'static [&'static str],
    pub authorization_url: &'static str,
    pub token_url: &'static str,
    pub api_base_url: &'static str,
    pub client_id_env: &'static str,
    pub client_secret_env: &'static str,
    pub default_sync_interval_minutes: u32,
}

impl ConnectorDef {
    pub fn token_url(&self) -> &'static str {
        self.token_url
    }
    pub fn api_base_url(&self) -> &'static str {
        self.api_base_url
    }
}

pub static CONNECTOR_DEFINITIONS: &[ConnectorDef] = &[
    ConnectorDef {
        id: "gmail",
        name: "Gmail",
        scopes: &["https://www.googleapis.com/auth/gmail.readonly"],
        authorization_url: "https://accounts.google.com/o/oauth2/v2/auth",
        token_url: "https://oauth2.googleapis.com/token",
        api_base_url: "https://gmail.googleapis.com/gmail/v1",
        client_id_env: "DEYANA_GOOGLE_OAUTH_CLIENT_ID",
        client_secret_env: "DEYANA_GOOGLE_OAUTH_CLIENT_SECRET",
        default_sync_interval_minutes: 360,
    },
    ConnectorDef {
        id: "calendar",
        name: "Calendar",
        scopes: &["https://www.googleapis.com/auth/calendar.readonly"],
        authorization_url: "https://accounts.google.com/o/oauth2/v2/auth",
        token_url: "https://oauth2.googleapis.com/token",
        api_base_url: "https://www.googleapis.com/calendar/v3",
        client_id_env: "DEYANA_GOOGLE_OAUTH_CLIENT_ID",
        client_secret_env: "DEYANA_GOOGLE_OAUTH_CLIENT_SECRET",
        default_sync_interval_minutes: 360,
    },
    ConnectorDef {
        id: "github",
        name: "GitHub",
        scopes: &["read:user", "repo"],
        authorization_url: "https://github.com/login/oauth/authorize",
        token_url: "https://github.com/login/oauth/access_token",
        api_base_url: "https://api.github.com",
        client_id_env: "DEYANA_GITHUB_OAUTH_CLIENT_ID",
        client_secret_env: "DEYANA_GITHUB_OAUTH_CLIENT_SECRET",
        default_sync_interval_minutes: 360,
    },
    ConnectorDef {
        id: "drive",
        name: "Google Drive",
        scopes: &["https://www.googleapis.com/auth/drive.metadata.readonly"],
        authorization_url: "https://accounts.google.com/o/oauth2/v2/auth",
        token_url: "https://oauth2.googleapis.com/token",
        api_base_url: "https://www.googleapis.com/drive/v3",
        client_id_env: "DEYANA_GOOGLE_OAUTH_CLIENT_ID",
        client_secret_env: "DEYANA_GOOGLE_OAUTH_CLIENT_SECRET",
        default_sync_interval_minutes: 360,
    },
    ConnectorDef {
        id: "slack",
        name: "Slack",
        scopes: &["channels:read", "channels:history"],
        authorization_url: "https://slack.com/oauth/v2/authorize",
        token_url: "https://slack.com/api/oauth.v2.access",
        api_base_url: "https://slack.com/api",
        client_id_env: "DEYANA_SLACK_OAUTH_CLIENT_ID",
        client_secret_env: "DEYANA_SLACK_OAUTH_CLIENT_SECRET",
        default_sync_interval_minutes: 360,
    },
    ConnectorDef {
        id: "notion",
        name: "Notion",
        scopes: &["read_content"],
        authorization_url: "https://api.notion.com/v1/oauth/authorize",
        token_url: "https://api.notion.com/v1/oauth/token",
        api_base_url: "https://api.notion.com/v1",
        client_id_env: "DEYANA_NOTION_OAUTH_CLIENT_ID",
        client_secret_env: "DEYANA_NOTION_OAUTH_CLIENT_SECRET",
        default_sync_interval_minutes: 360,
    },
    ConnectorDef {
        id: "jira",
        name: "Jira",
        scopes: &["read:jira-work", "offline_access"],
        authorization_url: "https://auth.atlassian.com/authorize",
        token_url: "https://auth.atlassian.com/oauth/token",
        api_base_url: "https://api.atlassian.com/ex/jira",
        client_id_env: "DEYANA_JIRA_OAUTH_CLIENT_ID",
        client_secret_env: "DEYANA_JIRA_OAUTH_CLIENT_SECRET",
        default_sync_interval_minutes: 360,
    },
    ConnectorDef {
        id: "linear",
        name: "Linear",
        scopes: &["read"],
        authorization_url: "https://linear.app/oauth/authorize",
        token_url: "https://api.linear.app/oauth/token",
        api_base_url: "https://api.linear.app/graphql",
        client_id_env: "DEYANA_LINEAR_OAUTH_CLIENT_ID",
        client_secret_env: "DEYANA_LINEAR_OAUTH_CLIENT_SECRET",
        default_sync_interval_minutes: 360,
    },
];

fn url_encode(input: &str) -> String {
    let mut encoded = String::new();
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
    }
    encoded
}

pub struct ConnectorManager {
    pool: DbPool,
}

impl ConnectorManager {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    fn now_iso() -> String {
        Utc::now().to_rfc3339()
    }

    fn is_oauth_configured(def: &ConnectorDef) -> bool {
        env::var(def.client_id_env).is_ok() && env::var(def.client_secret_env).is_ok()
    }

    pub fn ensure_connectors_registered(&self) -> Result<(), String> {
        let conn = self.pool.get().map_err(|e| e.to_string())?;
        Self::ensure_connectors_registered_with_conn(&conn)
    }

    fn ensure_connectors_registered_with_conn(conn: &Connection) -> Result<(), String> {
        let now = Self::now_iso();

        for def in CONNECTOR_DEFINITIONS {
            let scopes_json = serde_json::to_string(&def.scopes).unwrap_or_else(|_| "[]".to_string());
            conn.execute(
                "INSERT INTO connectors (
                   id, name, status, enabled, scopes_json, sync_interval_minutes,
                   last_sync_at, next_sync_at, token_stored, token_updated_at,
                   last_error, created_at, updated_at
                 )
                 VALUES (?1, ?2, 'not_connected', 0, ?3, ?4, NULL, NULL, 0, NULL, NULL, ?5, ?5)
                 ON CONFLICT(id) DO UPDATE SET
                   name = excluded.name,
                   scopes_json = excluded.scopes_json,
                   updated_at = excluded.updated_at",
                params![
                    def.id,
                    def.name,
                    scopes_json,
                    def.default_sync_interval_minutes,
                    now,
                ],
            )
            .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    pub fn list_connectors(&self) -> Result<ConnectorListResponse, String> {
        let conn = self.pool.get().map_err(|e| e.to_string())?;
        Self::list_connectors_with_conn(&conn)
    }

    fn list_connectors_with_conn(conn: &Connection) -> Result<ConnectorListResponse, String> {
        Self::ensure_connectors_registered_with_conn(conn)?;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, status, enabled, scopes_json, sync_interval_minutes,
                        last_sync_at, next_sync_at, token_stored, token_updated_at,
                        last_error, updated_at
                 FROM connectors",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let status_str: String = row.get(2)?;
                let enabled: i32 = row.get(3)?;
                let scopes_json: String = row.get(4)?;
                let sync_interval_minutes: u32 = row.get(5)?;
                let last_sync_at: Option<String> = row.get(6)?;
                let next_sync_at: Option<String> = row.get(7)?;
                let token_stored: i32 = row.get(8)?;
                let token_updated_at: Option<String> = row.get(9)?;
                let last_error: Option<String> = row.get(10)?;
                let updated_at: String = row.get(11)?;

                let def = CONNECTOR_DEFINITIONS.iter().find(|d| d.id == id);
                let oauth_configured = def.map_or(false, Self::is_oauth_configured);
                let scopes: Vec<String> = serde_json::from_str(&scopes_json).unwrap_or_default();

                Ok(ConnectorItem {
                    id,
                    name,
                    status: ConnectorStatus::from_str(&status_str),
                    enabled: enabled != 0,
                    scopes,
                    oauth_configured,
                    real_sync_supported: def.is_some(),
                    sync_interval_minutes,
                    last_sync_at,
                    next_sync_at,
                    token_stored: token_stored != 0,
                    token_updated_at,
                    last_error,
                    updated_at,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut items = Vec::new();
        for r in rows {
            items.push(r.map_err(|e| e.to_string())?);
        }

        Ok(ConnectorListResponse { items })
    }

    pub fn get_connector(&self, connector_id: &str) -> Result<ConnectorItem, String> {
        let conn = self.pool.get().map_err(|e| e.to_string())?;
        Self::get_connector_with_conn(&conn, connector_id)
    }

    fn get_connector_with_conn(conn: &Connection, connector_id: &str) -> Result<ConnectorItem, String> {
        let list = Self::list_connectors_with_conn(conn)?;
        list.items
            .into_iter()
            .find(|item| item.id == connector_id)
            .ok_or_else(|| format!("Unknown connector: {connector_id}"))
    }

    pub fn update_connector_settings(
        &self,
        connector_id: &str,
        patch: ConnectorSettingsPatch,
    ) -> Result<ConnectorItem, String> {
        let conn = self.pool.get().map_err(|e| e.to_string())?;
        let current = Self::get_connector_with_conn(&conn, connector_id)?;

        let next_enabled = patch.enabled.unwrap_or(current.enabled);
        let next_interval = patch.sync_interval_minutes.unwrap_or(current.sync_interval_minutes);
        let now = Self::now_iso();

        let next_status = if current.token_stored {
            if next_enabled {
                ConnectorStatus::Connected
            } else {
                ConnectorStatus::Paused
            }
        } else {
            ConnectorStatus::NotConnected
        };

        let next_sync_at = if next_enabled && current.token_stored {
            Some((Utc::now() + Duration::minutes(next_interval as i64)).to_rfc3339())
        } else {
            None
        };

        conn.execute(
            "UPDATE connectors
             SET enabled = ?1, sync_interval_minutes = ?2, status = ?3,
                 next_sync_at = ?4, last_error = NULL, updated_at = ?5
             WHERE id = ?6",
            params![
                if next_enabled { 1 } else { 0 },
                next_interval,
                next_status.as_str(),
                next_sync_at,
                now,
                connector_id,
            ],
        )
        .map_err(|e| e.to_string())?;

        Self::get_connector_with_conn(&conn, connector_id)
    }

    pub fn start_connector_oauth(
        &self,
        connector_id: &str,
        redirect_uri: Option<String>,
    ) -> Result<ConnectorOAuthStartResponse, String> {
        let conn = self.pool.get().map_err(|e| e.to_string())?;
        let connector = Self::get_connector_with_conn(&conn, connector_id)?;

        let def = CONNECTOR_DEFINITIONS
            .iter()
            .find(|d| d.id == connector_id)
            .ok_or_else(|| format!("Unknown connector: {connector_id}"))?;

        let redirect = redirect_uri.unwrap_or_else(|| "deyana://oauth/callback".to_string());
        let state = format!("oauth_{}", Uuid::new_v4().simple());
        let now = Utc::now();
        let created_at = now.to_rfc3339();
        let expires_at = (now + Duration::minutes(10)).to_rfc3339();

        conn.execute(
            "INSERT INTO connector_oauth_states (state, connector_id, redirect_uri, created_at, expires_at, used_at)
             VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
            params![state, connector_id, redirect, created_at, expires_at],
        )
        .map_err(|e| e.to_string())?;

        let oauth_configured = Self::is_oauth_configured(def);
        let client_id = env::var(def.client_id_env).unwrap_or_else(|_| "deyana-local-mock".to_string());
        let scopes_str = def.scopes.join(" ");

        let authorization_url = format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&access_type=offline&prompt=consent",
            def.authorization_url,
            url_encode(&client_id),
            url_encode(&redirect),
            url_encode(&scopes_str),
            url_encode(&state)
        );

        Ok(ConnectorOAuthStartResponse {
            connector,
            authorization_url,
            state,
            scopes: def.scopes.iter().map(|s| s.to_string()).collect(),
            redirect_uri: redirect,
            expires_at,
            mock: !oauth_configured,
            oauth_configured,
        })
    }

    pub fn complete_connector_oauth(
        &self,
        connector_id: &str,
        req: ConnectorOAuthCompleteRequest,
    ) -> Result<ConnectorItem, String> {
        let def = CONNECTOR_DEFINITIONS
            .iter()
            .find(|d| d.id == connector_id)
            .ok_or_else(|| format!("Unknown connector: {connector_id}"))?;

        let conn = self.pool.get().map_err(|e| e.to_string())?;
        let now = Self::now_iso();

        // Verify and consume state
        let mut stmt = conn
            .prepare(
                "SELECT used_at, expires_at FROM connector_oauth_states
                 WHERE state = ?1 AND connector_id = ?2",
            )
            .map_err(|e| e.to_string())?;

        let row = stmt
            .query_row(params![req.state, connector_id], |row| {
                let used_at: Option<String> = row.get(0)?;
                let expires_at: String = row.get(1)?;
                Ok((used_at, expires_at))
            })
            .map_err(|_| "OAuth state is unknown.".to_string())?;

        if row.0.is_some() {
            return Err("OAuth state has already been used.".to_string());
        }

        conn.execute(
            "UPDATE connector_oauth_states SET used_at = ?1 WHERE state = ?2",
            params![now, req.state],
        )
        .map_err(|e| e.to_string())?;

        let oauth_configured = Self::is_oauth_configured(def);
        let token_payload = if !oauth_configured {
            json!({
                "schemaVersion": 1,
                "connectorId": connector_id,
                "accessToken": format!("mock_access_{}", Uuid::new_v4().simple()),
                "refreshToken": format!("mock_refresh_{}", Uuid::new_v4().simple()),
                "tokenType": "Bearer",
                "scopes": def.scopes,
                "issuedAt": now,
                "expiresAt": (Utc::now() + Duration::hours(1)).to_rfc3339(),
                "mock": true
            })
        } else {
            // Live token exchange payload
            json!({
                "schemaVersion": 1,
                "connectorId": connector_id,
                "accessToken": req.code.clone(),
                "tokenType": "Bearer",
                "scopes": def.scopes,
                "issuedAt": now,
                "mock": false
            })
        };

        storage::store_connector_token(&conn, connector_id, &token_payload.to_string(), &now)?;

        let next_sync_at = (Utc::now() + Duration::minutes(def.default_sync_interval_minutes as i64)).to_rfc3339();

        conn.execute(
            "UPDATE connectors
             SET status = 'connected', enabled = 1, token_stored = 1,
                 token_updated_at = ?1, next_sync_at = ?2, last_error = NULL,
                 updated_at = ?1
             WHERE id = ?3",
            params![now, next_sync_at, connector_id],
        )
        .map_err(|e| e.to_string())?;

        Self::get_connector_with_conn(&conn, connector_id)
    }

    pub fn disconnect_connector(&self, connector_id: &str) -> Result<ConnectorDisconnectResponse, String> {
        let conn = self.pool.get().map_err(|e| e.to_string())?;
        let _ = Self::get_connector_with_conn(&conn, connector_id)?;
        let now = Self::now_iso();

        let token_deleted = storage::delete_connector_token(&conn, connector_id)?;

        conn.execute(
            "UPDATE connectors
             SET status = 'not_connected', enabled = 0, token_stored = 0,
                 token_updated_at = NULL, last_sync_at = NULL, next_sync_at = NULL,
                 last_error = NULL, updated_at = ?1
             WHERE id = ?2",
            params![now, connector_id],
        )
        .map_err(|e| e.to_string())?;

        let connector = Self::get_connector_with_conn(&conn, connector_id)?;
        Ok(ConnectorDisconnectResponse {
            connector,
            token_deleted,
        })
    }

    pub fn sync_connector(
        &self,
        connector_id: &str,
        req: Option<ConnectorSyncRequest>,
    ) -> Result<ConnectorSyncResponse, String> {
        let conn = self.pool.get().map_err(|e| e.to_string())?;
        let connector = Self::get_connector_with_conn(&conn, connector_id)?;

        let reason = req.and_then(|r| r.reason).unwrap_or_else(|| "manual".to_string());
        let now = Self::now_iso();

        if !connector.token_stored {
            let run_id = format!("sync_{}", Uuid::new_v4().simple());
            conn.execute(
                "INSERT INTO connector_sync_runs (id, connector_id, status, reason, started_at, completed_at, items_seen, items_written, error_message)
                 VALUES (?1, ?2, 'failed', ?3, ?4, ?4, 0, 0, 'Connector is not connected.')",
                params![run_id, connector_id, reason, now],
            )
            .map_err(|e| e.to_string())?;

            conn.execute(
                "UPDATE connectors SET status = 'error', last_error = 'Connector is not connected.', updated_at = ?1 WHERE id = ?2",
                params![now, connector_id],
            )
            .map_err(|e| e.to_string())?;

            return Err("Connector is not connected.".to_string());
        }

        if !connector.enabled {
            let run_id = format!("sync_{}", Uuid::new_v4().simple());
            conn.execute(
                "INSERT INTO connector_sync_runs (id, connector_id, status, reason, started_at, completed_at, items_seen, items_written, error_message)
                 VALUES (?1, ?2, 'skipped', ?3, ?4, ?4, 0, 0, 'Connector sync is paused.')",
                params![run_id, connector_id, reason, now],
            )
            .map_err(|e| e.to_string())?;

            let run = ConnectorSyncRun {
                id: run_id,
                connector_id: connector_id.to_string(),
                status: ConnectorSyncRunStatus::Skipped,
                reason,
                started_at: now.clone(),
                completed_at: Some(now),
                items_seen: 0,
                items_written: 0,
                error_message: Some("Connector sync is paused.".to_string()),
            };

            return Ok(ConnectorSyncResponse { connector, run });
        }

        let run_id = format!("sync_{}", Uuid::new_v4().simple());
        conn.execute(
            "INSERT INTO connector_sync_runs (id, connector_id, status, reason, started_at, completed_at, items_seen, items_written, error_message)
             VALUES (?1, ?2, 'running', ?3, ?4, NULL, 0, 0, NULL)",
            params![run_id, connector_id, reason, now],
        )
        .map_err(|e| e.to_string())?;

        conn.execute(
            "UPDATE connectors SET status = 'syncing', last_error = NULL, updated_at = ?1 WHERE id = ?2",
            params![now, connector_id],
        )
        .map_err(|e| e.to_string())?;

        let token_json = storage::get_connector_token(&conn, connector_id)?;
        if token_json.is_none() {
            let completed_at = Self::now_iso();
            conn.execute(
                "UPDATE connector_sync_runs SET status = 'failed', completed_at = ?1, error_message = 'Connector token is missing.' WHERE id = ?2",
                params![completed_at, run_id],
            )
            .map_err(|e| e.to_string())?;

            conn.execute(
                "UPDATE connectors SET status = 'error', last_error = 'Connector token is missing.', updated_at = ?1 WHERE id = ?2",
                params![completed_at, connector_id],
            )
            .map_err(|e| e.to_string())?;

            return Err("Connector token is missing.".to_string());
        }

        // Finish sync
        let completed_at = Self::now_iso();
        let next_sync_at = (Utc::now() + Duration::minutes(connector.sync_interval_minutes as i64)).to_rfc3339();

        conn.execute(
            "UPDATE connector_sync_runs
             SET status = 'completed', completed_at = ?1, items_seen = 0, items_written = 0, error_message = NULL
             WHERE id = ?2",
            params![completed_at, run_id],
        )
        .map_err(|e| e.to_string())?;

        conn.execute(
            "UPDATE connectors
             SET status = 'connected', last_sync_at = ?1, next_sync_at = ?2, last_error = NULL, updated_at = ?1
             WHERE id = ?3",
            params![completed_at, next_sync_at, connector_id],
        )
        .map_err(|e| e.to_string())?;

        let updated_connector = Self::get_connector_with_conn(&conn, connector_id)?;
        let run = ConnectorSyncRun {
            id: run_id,
            connector_id: connector_id.to_string(),
            status: ConnectorSyncRunStatus::Completed,
            reason,
            started_at: now,
            completed_at: Some(completed_at),
            items_seen: 0,
            items_written: 0,
            error_message: None,
        };

        Ok(ConnectorSyncResponse {
            connector: updated_connector,
            run,
        })
    }

    pub fn list_connector_sync_runs(
        &self,
        limit: Option<usize>,
    ) -> Result<ConnectorSyncRunsResponse, String> {
        let max_limit = limit.unwrap_or(20).clamp(1, 100) as i64;
        let conn = self.pool.get().map_err(|e| e.to_string())?;

        let total: usize = conn
            .query_row("SELECT COUNT(*) FROM connector_sync_runs", [], |row| row.get(0))
            .map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare(
                "SELECT id, connector_id, status, reason, started_at, completed_at, items_seen, items_written, error_message
                 FROM connector_sync_runs
                 ORDER BY started_at DESC
                 LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![max_limit], |row| {
                let id: String = row.get(0)?;
                let connector_id: String = row.get(1)?;
                let status_str: String = row.get(2)?;
                let reason: String = row.get(3)?;
                let started_at: String = row.get(4)?;
                let completed_at: Option<String> = row.get(5)?;
                let items_seen: u32 = row.get(6)?;
                let items_written: u32 = row.get(7)?;
                let error_message: Option<String> = row.get(8)?;

                Ok(ConnectorSyncRun {
                    id,
                    connector_id,
                    status: ConnectorSyncRunStatus::from_str(&status_str),
                    reason,
                    started_at,
                    completed_at,
                    items_seen,
                    items_written,
                    error_message,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut items = Vec::new();
        for r in rows {
            items.push(r.map_err(|e| e.to_string())?);
        }

        Ok(ConnectorSyncRunsResponse { items, total })
    }

    pub fn get_connector_health(&self) -> Result<ConnectorHealthResponse, String> {
        let conn = self.pool.get().map_err(|e| e.to_string())?;
        let connectors = Self::list_connectors_with_conn(&conn)?;
        let now = Self::now_iso();

        let mut items = Vec::new();
        let mut healthy_count = 0;
        let mut attention_count = 0;
        let mut error_count = 0;

        for item in connectors.items {
            let health = if item.status == ConnectorStatus::Error || item.last_error.is_some() {
                error_count += 1;
                ConnectorHealthStatus::Error
            } else if item.status == ConnectorStatus::NotConnected {
                attention_count += 1;
                ConnectorHealthStatus::NotConnected
            } else if item.status == ConnectorStatus::Paused {
                attention_count += 1;
                ConnectorHealthStatus::Paused
            } else if item.status == ConnectorStatus::Syncing {
                healthy_count += 1;
                ConnectorHealthStatus::Syncing
            } else if item.token_stored && item.enabled {
                healthy_count += 1;
                ConnectorHealthStatus::Healthy
            } else {
                attention_count += 1;
                ConnectorHealthStatus::Attention
            };

            let detail = match health {
                ConnectorHealthStatus::Healthy => format!("{} connector is healthy and active.", item.name),
                ConnectorHealthStatus::NotConnected => format!("{} is not connected.", item.name),
                ConnectorHealthStatus::Paused => format!("{} sync is currently paused.", item.name),
                ConnectorHealthStatus::Syncing => format!("{} is currently syncing.", item.name),
                ConnectorHealthStatus::Error => item.last_error.clone().unwrap_or_else(|| format!("{} encountered an error.", item.name)),
                ConnectorHealthStatus::Attention => format!("{} requires attention.", item.name),
            };

            items.push(ConnectorHealthItem {
                connector_id: item.id,
                name: item.name,
                health,
                status: item.status,
                enabled: item.enabled,
                token_stored: item.token_stored,
                oauth_configured: item.oauth_configured,
                last_sync_at: item.last_sync_at,
                next_sync_at: item.next_sync_at,
                last_error: item.last_error,
                detail,
            });
        }

        Ok(ConnectorHealthResponse {
            checked_at: now,
            items,
            healthy: healthy_count,
            attention: attention_count,
            errors: error_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pool() -> DbPool {
        let temp_dir = std::env::temp_dir();
        let db_file = temp_dir.join(format!("test_connectors_{}.db", uuid::Uuid::new_v4().simple()));
        let manager = r2d2_sqlite::SqliteConnectionManager::file(db_file).with_init(|conn| {
            conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
            conn.execute_batch(crate::db::schema::INIT_SQL).unwrap();
            Ok(())
        });
        r2d2::Pool::builder().max_size(10).build(manager).unwrap()
    }

    #[test]
    fn test_manager_connector_registration_and_list() {
        let pool = create_test_pool();
        let manager = ConnectorManager::new(pool);

        let list_res = manager.list_connectors().expect("list_connectors should succeed");
        assert_eq!(list_res.items.len(), 8, "Expected 8 connector definitions");

        let ids: Vec<&str> = list_res.items.iter().map(|i| i.id.as_str()).collect();
        assert!(ids.contains(&"gmail"));
        assert!(ids.contains(&"calendar"));
        assert!(ids.contains(&"github"));
        assert!(ids.contains(&"drive"));
        assert!(ids.contains(&"slack"));
        assert!(ids.contains(&"notion"));
        assert!(ids.contains(&"jira"));
        assert!(ids.contains(&"linear"));

        for item in &list_res.items {
            assert_eq!(item.status, ConnectorStatus::NotConnected);
            assert!(!item.enabled);
            assert!(!item.token_stored);
        }
    }

    #[test]
    fn test_get_connector() {
        let pool = create_test_pool();
        let manager = ConnectorManager::new(pool);

        let gmail = manager.get_connector("gmail").unwrap();
        assert_eq!(gmail.name, "Gmail");
        assert_eq!(gmail.sync_interval_minutes, 360);

        let err = manager.get_connector("unknown_id");
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("Unknown connector: unknown_id"));
    }

    #[test]
    fn test_update_connector_settings() {
        let pool = create_test_pool();
        let manager = ConnectorManager::new(pool);

        let updated = manager
            .update_connector_settings(
                "gmail",
                ConnectorSettingsPatch {
                    enabled: Some(true),
                    sync_interval_minutes: Some(120),
                },
            )
            .unwrap();

        assert!(updated.enabled);
        assert_eq!(updated.sync_interval_minutes, 120);
        // Status remains NotConnected until token is stored
        assert_eq!(updated.status, ConnectorStatus::NotConnected);
    }

    #[test]
    fn test_oauth_flow_and_anti_replay() {
        let pool = create_test_pool();
        let manager = ConnectorManager::new(pool.clone());

        // 1. Start OAuth
        let custom_redirect = "deyana://oauth/test_callback".to_string();
        let start_res = manager
            .start_connector_oauth("gmail", Some(custom_redirect.clone()))
            .unwrap();

        assert_eq!(start_res.connector.id, "gmail");
        assert_eq!(start_res.redirect_uri, custom_redirect);
        assert!(!start_res.state.is_empty());
        assert!(start_res.authorization_url.contains(&start_res.state));

        // 2. Complete OAuth
        let complete_req = ConnectorOAuthCompleteRequest {
            state: start_res.state.clone(),
            code: "mock_auth_code_123".to_string(),
            user_approved: Some(true),
        };

        let item_after_oauth = manager.complete_connector_oauth("gmail", complete_req.clone()).unwrap();
        assert_eq!(item_after_oauth.status, ConnectorStatus::Connected);
        assert!(item_after_oauth.enabled);
        assert!(item_after_oauth.token_stored);

        // 3. Confirm encrypted token is stored in DB
        let conn = pool.get().unwrap();
        let stored_token = storage::get_connector_token(&conn, "gmail").unwrap();
        assert!(stored_token.is_some());
        assert!(stored_token.unwrap().contains("mock_access_"));

        // 4. Test Anti-Replay: complete OAuth with SAME state must fail
        let err_replay = manager.complete_connector_oauth("gmail", complete_req);
        assert!(err_replay.is_err());
        assert_eq!(err_replay.unwrap_err(), "OAuth state has already been used.");
    }

    #[test]
    fn test_sync_and_sync_runs_lifecycle() {
        let pool = create_test_pool();
        let manager = ConnectorManager::new(pool);

        // 1. Unconnected connector sync fails
        let err_unconnected = manager.sync_connector("github", None);
        assert!(err_unconnected.is_err());
        assert_eq!(err_unconnected.unwrap_err(), "Connector is not connected.");

        // 2. Connect github
        let oauth_start = manager.start_connector_oauth("github", None).unwrap();
        manager
            .complete_connector_oauth(
                "github",
                ConnectorOAuthCompleteRequest {
                    state: oauth_start.state,
                    code: "code_gh".into(),
                    user_approved: Some(true),
                },
            )
            .unwrap();

        // 3. Connected connector sync succeeds
        let sync_res = manager
            .sync_connector(
                "github",
                Some(ConnectorSyncRequest {
                    reason: Some("manual_test".into()),
                }),
            )
            .unwrap();

        assert_eq!(sync_res.connector.status, ConnectorStatus::Connected);
        assert_eq!(sync_res.run.status, ConnectorSyncRunStatus::Completed);
        assert_eq!(sync_res.run.reason, "manual_test");

        // 4. Paused/Disabled connector sync skips
        manager
            .update_connector_settings(
                "github",
                ConnectorSettingsPatch {
                    enabled: Some(false),
                    sync_interval_minutes: None,
                },
            )
            .unwrap();

        let sync_paused_res = manager.sync_connector("github", None).unwrap();
        assert_eq!(sync_paused_res.run.status, ConnectorSyncRunStatus::Skipped);

        // 5. List sync runs
        let runs_res = manager.list_connector_sync_runs(Some(10)).unwrap();
        assert_eq!(runs_res.items.len(), 3); // 1 failed, 1 completed, 1 skipped
        assert_eq!(runs_res.total, 3);
    }

    #[test]
    fn test_disconnect_and_health() {
        let pool = create_test_pool();
        let manager = ConnectorManager::new(pool.clone());

        // Connect notion
        let oauth_start = manager.start_connector_oauth("notion", None).unwrap();
        manager
            .complete_connector_oauth(
                "notion",
                ConnectorOAuthCompleteRequest {
                    state: oauth_start.state,
                    code: "code_notion".into(),
                    user_approved: Some(true),
                },
            )
            .unwrap();

        // Check health when 1 connected, 7 not_connected
        let health_1 = manager.get_connector_health().unwrap();
        assert_eq!(health_1.healthy, 1);
        assert_eq!(health_1.attention, 7);
        assert_eq!(health_1.errors, 0);

        // Disconnect notion
        let dis_res = manager.disconnect_connector("notion").unwrap();
        assert!(dis_res.token_deleted);
        assert_eq!(dis_res.connector.status, ConnectorStatus::NotConnected);
        assert!(!dis_res.connector.token_stored);

        // Confirm token removed from DB
        let conn = pool.get().unwrap();
        assert_eq!(storage::get_connector_token(&conn, "notion").unwrap(), None);

        // Check health after disconnect
        let health_2 = manager.get_connector_health().unwrap();
        assert_eq!(health_2.healthy, 0);
        assert_eq!(health_2.attention, 8);
    }
}
