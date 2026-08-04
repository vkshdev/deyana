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
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_enums_serde_snake_case() {
        assert_eq!(serde_json::to_string(&ConnectorStatus::NotConnected).unwrap(), "\"not_connected\"");
        assert_eq!(serde_json::to_string(&ConnectorStatus::Connected).unwrap(), "\"connected\"");
        assert_eq!(serde_json::to_string(&ConnectorStatus::Syncing).unwrap(), "\"syncing\"");
        assert_eq!(serde_json::to_string(&ConnectorStatus::Paused).unwrap(), "\"paused\"");
        assert_eq!(serde_json::to_string(&ConnectorStatus::Error).unwrap(), "\"error\"");

        assert_eq!(serde_json::to_string(&ConnectorSyncRunStatus::Queued).unwrap(), "\"queued\"");
        assert_eq!(serde_json::to_string(&ConnectorSyncRunStatus::Running).unwrap(), "\"running\"");
        assert_eq!(serde_json::to_string(&ConnectorSyncRunStatus::Completed).unwrap(), "\"completed\"");
        assert_eq!(serde_json::to_string(&ConnectorSyncRunStatus::Failed).unwrap(), "\"failed\"");
        assert_eq!(serde_json::to_string(&ConnectorSyncRunStatus::Skipped).unwrap(), "\"skipped\"");

        assert_eq!(serde_json::to_string(&ConnectorHealthStatus::Healthy).unwrap(), "\"healthy\"");
        assert_eq!(serde_json::to_string(&ConnectorHealthStatus::NotConnected).unwrap(), "\"not_connected\"");
        assert_eq!(serde_json::to_string(&ConnectorHealthStatus::Paused).unwrap(), "\"paused\"");
        assert_eq!(serde_json::to_string(&ConnectorHealthStatus::Syncing).unwrap(), "\"syncing\"");
        assert_eq!(serde_json::to_string(&ConnectorHealthStatus::Error).unwrap(), "\"error\"");
        assert_eq!(serde_json::to_string(&ConnectorHealthStatus::Attention).unwrap(), "\"attention\"");
    }

    #[test]
    fn test_structs_serde_camel_case() {
        let item = ConnectorItem {
            id: "gmail".into(),
            name: "Gmail".into(),
            status: ConnectorStatus::Connected,
            enabled: true,
            scopes: vec!["scope1".into()],
            oauth_configured: false,
            real_sync_supported: true,
            sync_interval_minutes: 360,
            last_sync_at: Some("2026-08-03T10:00:00Z".into()),
            next_sync_at: None,
            token_stored: true,
            token_updated_at: Some("2026-08-03T09:00:00Z".into()),
            last_error: None,
            updated_at: "2026-08-03T10:00:00Z".into(),
        };

        let val = serde_json::to_value(&item).unwrap();
        assert!(val.get("oauthConfigured").is_some());
        assert!(val.get("realSyncSupported").is_some());
        assert!(val.get("syncIntervalMinutes").is_some());
        assert!(val.get("lastSyncAt").is_some());
        assert!(val.get("nextSyncAt").is_none()); // skip_serializing_if Option::is_none
        assert!(val.get("tokenStored").is_some());
        assert!(val.get("tokenUpdatedAt").is_some());
        assert!(val.get("lastError").is_none());
        assert!(val.get("updatedAt").is_some());

        // Test ConnectorSettingsPatch deserialization from camelCase JSON
        let patch_json = json!({
            "enabled": false,
            "syncIntervalMinutes": 120
        });
        let patch: ConnectorSettingsPatch = serde_json::from_value(patch_json).unwrap();
        assert_eq!(patch.enabled, Some(false));
        assert_eq!(patch.sync_interval_minutes, Some(120));

        // Test ConnectorOAuthCompleteRequest deserialization
        let oauth_req_json = json!({
            "state": "state123",
            "code": "code456",
            "userApproved": true
        });
        let oauth_req: ConnectorOAuthCompleteRequest = serde_json::from_value(oauth_req_json).unwrap();
        assert_eq!(oauth_req.state, "state123");
        assert_eq!(oauth_req.code, "code456");
        assert_eq!(oauth_req.user_approved, Some(true));

        // Test ConnectorSyncRun serialization
        let sync_run = ConnectorSyncRun {
            id: "run1".into(),
            connector_id: "gmail".into(),
            status: ConnectorSyncRunStatus::Completed,
            reason: "manual".into(),
            started_at: "2026-08-03T10:00:00Z".into(),
            completed_at: Some("2026-08-03T10:01:00Z".into()),
            items_seen: 5,
            items_written: 2,
            error_message: None,
        };
        let run_val = serde_json::to_value(&sync_run).unwrap();
        assert_eq!(run_val["connectorId"], "gmail");
        assert_eq!(run_val["startedAt"], "2026-08-03T10:00:00Z");
        assert_eq!(run_val["completedAt"], "2026-08-03T10:01:00Z");
        assert_eq!(run_val["itemsSeen"], 5);
        assert_eq!(run_val["itemsWritten"], 2);
        assert!(run_val.get("errorMessage").is_none());
    }
}

