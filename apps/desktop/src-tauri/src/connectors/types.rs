use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorStatus {
    NotConnected,
    Connected,
    Syncing,
    Paused,
    Error,
}

impl ConnectorStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectorStatus::NotConnected => "not_connected",
            ConnectorStatus::Connected => "connected",
            ConnectorStatus::Syncing => "syncing",
            ConnectorStatus::Paused => "paused",
            ConnectorStatus::Error => "error",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "connected" => ConnectorStatus::Connected,
            "syncing" => ConnectorStatus::Syncing,
            "paused" => ConnectorStatus::Paused,
            "error" => ConnectorStatus::Error,
            _ => ConnectorStatus::NotConnected,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorSyncRunStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Skipped,
}

impl ConnectorSyncRunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectorSyncRunStatus::Queued => "queued",
            ConnectorSyncRunStatus::Running => "running",
            ConnectorSyncRunStatus::Completed => "completed",
            ConnectorSyncRunStatus::Failed => "failed",
            ConnectorSyncRunStatus::Skipped => "skipped",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "running" => ConnectorSyncRunStatus::Running,
            "completed" => ConnectorSyncRunStatus::Completed,
            "failed" => ConnectorSyncRunStatus::Failed,
            "skipped" => ConnectorSyncRunStatus::Skipped,
            _ => ConnectorSyncRunStatus::Queued,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorHealthStatus {
    Healthy,
    NotConnected,
    Paused,
    Syncing,
    Error,
    Attention,
}

impl ConnectorHealthStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectorHealthStatus::Healthy => "healthy",
            ConnectorHealthStatus::NotConnected => "not_connected",
            ConnectorHealthStatus::Paused => "paused",
            ConnectorHealthStatus::Syncing => "syncing",
            ConnectorHealthStatus::Error => "error",
            ConnectorHealthStatus::Attention => "attention",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorItem {
    pub id: String,
    pub name: String,
    pub status: ConnectorStatus,
    pub enabled: bool,
    pub scopes: Vec<String>,
    pub oauth_configured: bool,
    pub real_sync_supported: bool,
    pub sync_interval_minutes: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_sync_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_sync_at: Option<String>,
    pub token_stored: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorListResponse {
    pub items: Vec<ConnectorItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorSettingsPatch {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub sync_interval_minutes: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorOAuthStartRequest {
    #[serde(default)]
    pub redirect_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorOAuthStartResponse {
    pub connector: ConnectorItem,
    pub authorization_url: String,
    pub state: String,
    pub scopes: Vec<String>,
    pub redirect_uri: String,
    pub expires_at: String,
    pub mock: bool,
    pub oauth_configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorOAuthCompleteRequest {
    pub state: String,
    pub code: String,
    #[serde(default)]
    pub user_approved: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorDisconnectResponse {
    pub connector: ConnectorItem,
    pub token_deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorSyncRequest {
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorSyncRun {
    pub id: String,
    pub connector_id: String,
    pub status: ConnectorSyncRunStatus,
    pub reason: String,
    pub started_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    pub items_seen: u32,
    pub items_written: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorSyncResponse {
    pub connector: ConnectorItem,
    pub run: ConnectorSyncRun,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorSyncRunsResponse {
    pub items: Vec<ConnectorSyncRun>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorHealthItem {
    pub connector_id: String,
    pub name: String,
    pub health: ConnectorHealthStatus,
    pub status: ConnectorStatus,
    pub enabled: bool,
    pub token_stored: bool,
    pub oauth_configured: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_sync_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_sync_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorHealthResponse {
    pub checked_at: String,
    pub items: Vec<ConnectorHealthItem>,
    pub healthy: u32,
    pub attention: u32,
    pub errors: u32,
}

#[cfg(test)]
#[path = "tests/types_tests.rs"]
mod tests;


