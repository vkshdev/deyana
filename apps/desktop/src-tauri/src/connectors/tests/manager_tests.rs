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
    assert_eq!(updated.status, ConnectorStatus::NotConnected);
}

#[test]
fn test_oauth_flow_and_anti_replay() {
    let pool = create_test_pool();
    let manager = ConnectorManager::new(pool.clone());

    let custom_redirect = "deyana://oauth/test_callback".to_string();
    let start_res = manager
        .start_connector_oauth("gmail", Some(custom_redirect.clone()))
        .unwrap();

    assert_eq!(start_res.connector.id, "gmail");
    assert_eq!(start_res.redirect_uri, custom_redirect);
    assert!(!start_res.state.is_empty());
    assert!(start_res.authorization_url.contains(&start_res.state));

    let complete_req = ConnectorOAuthCompleteRequest {
        state: start_res.state.clone(),
        code: "mock_auth_code_123".to_string(),
        user_approved: Some(true),
    };

    let item_after_oauth = manager.complete_connector_oauth("gmail", complete_req.clone()).unwrap();
    assert_eq!(item_after_oauth.status, ConnectorStatus::Connected);
    assert!(item_after_oauth.enabled);
    assert!(item_after_oauth.token_stored);

    let conn = pool.get().unwrap();
    let stored_token = storage::get_connector_token(&conn, "gmail").unwrap();
    assert!(stored_token.is_some());
    assert!(stored_token.unwrap().contains("mock_access_"));

    let err_replay = manager.complete_connector_oauth("gmail", complete_req);
    assert!(err_replay.is_err());
    assert_eq!(err_replay.unwrap_err(), "OAuth state has already been used.");
}

#[test]
fn test_sync_and_sync_runs_lifecycle() {
    let pool = create_test_pool();
    let manager = ConnectorManager::new(pool);

    let err_unconnected = manager.sync_connector("github", None);
    assert!(err_unconnected.is_err());
    assert_eq!(err_unconnected.unwrap_err(), "Connector is not connected.");

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

    let runs_res = manager.list_connector_sync_runs(Some(10)).unwrap();
    assert_eq!(runs_res.items.len(), 3);
    assert_eq!(runs_res.total, 3);
}

#[test]
fn test_disconnect_and_health() {
    let pool = create_test_pool();
    let manager = ConnectorManager::new(pool.clone());

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

    let health_1 = manager.get_connector_health().unwrap();
    assert_eq!(health_1.healthy, 1);
    assert_eq!(health_1.attention, 7);
    assert_eq!(health_1.errors, 0);

    let dis_res = manager.disconnect_connector("notion").unwrap();
    assert!(dis_res.token_deleted);
    assert_eq!(dis_res.connector.status, ConnectorStatus::NotConnected);
    assert!(!dis_res.connector.token_stored);

    let conn = pool.get().unwrap();
    assert_eq!(storage::get_connector_token(&conn, "notion").unwrap(), None);

    let health_2 = manager.get_connector_health().unwrap();
    assert_eq!(health_2.healthy, 0);
    assert_eq!(health_2.attention, 8);
}
