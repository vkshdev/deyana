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
    assert!(val.get("nextSyncAt").is_none());
    assert!(val.get("tokenStored").is_some());
    assert!(val.get("tokenUpdatedAt").is_some());
    assert!(val.get("lastError").is_none());
    assert!(val.get("updatedAt").is_some());

    let patch_json = json!({
        "enabled": false,
        "syncIntervalMinutes": 120
    });
    let patch: ConnectorSettingsPatch = serde_json::from_value(patch_json).unwrap();
    assert_eq!(patch.enabled, Some(false));
    assert_eq!(patch.sync_interval_minutes, Some(120));

    let oauth_req_json = json!({
        "state": "state123",
        "code": "code456",
        "userApproved": true
    });
    let oauth_req: ConnectorOAuthCompleteRequest = serde_json::from_value(oauth_req_json).unwrap();
    assert_eq!(oauth_req.state, "state123");
    assert_eq!(oauth_req.code, "code456");
    assert_eq!(oauth_req.user_approved, Some(true));

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
