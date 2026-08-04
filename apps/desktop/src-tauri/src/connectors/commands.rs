use tauri::State;

use super::ConnectorState;
use super::types::*;

#[tauri::command]
pub fn list_connectors(
    state: State<'_, ConnectorState>,
) -> Result<ConnectorListResponse, String> {
    state.manager.list_connectors()
}

#[tauri::command]
pub fn get_connector(
    state: State<'_, ConnectorState>,
    connector_id: String,
) -> Result<ConnectorItem, String> {
    state.manager.get_connector(&connector_id)
}

#[tauri::command]
pub fn update_connector_settings(
    state: State<'_, ConnectorState>,
    connector_id: String,
    patch: ConnectorSettingsPatch,
) -> Result<ConnectorItem, String> {
    state.manager.update_connector_settings(&connector_id, patch)
}

#[tauri::command]
pub fn start_connector_oauth(
    state: State<'_, ConnectorState>,
    connector_id: String,
    redirect_uri: Option<String>,
) -> Result<ConnectorOAuthStartResponse, String> {
    state.manager.start_connector_oauth(&connector_id, redirect_uri)
}

#[tauri::command]
pub fn complete_connector_oauth(
    state: State<'_, ConnectorState>,
    connector_id: String,
    request: ConnectorOAuthCompleteRequest,
) -> Result<ConnectorItem, String> {
    state.manager.complete_connector_oauth(&connector_id, request)
}

#[tauri::command]
pub fn disconnect_connector(
    state: State<'_, ConnectorState>,
    connector_id: String,
) -> Result<ConnectorDisconnectResponse, String> {
    state.manager.disconnect_connector(&connector_id)
}

#[tauri::command]
pub fn sync_connector(
    state: State<'_, ConnectorState>,
    connector_id: String,
    request: Option<ConnectorSyncRequest>,
) -> Result<ConnectorSyncResponse, String> {
    state.manager.sync_connector(&connector_id, request)
}

#[tauri::command]
pub fn list_connector_sync_runs(
    state: State<'_, ConnectorState>,
    limit: Option<usize>,
) -> Result<ConnectorSyncRunsResponse, String> {
    state.manager.list_connector_sync_runs(limit)
}

#[tauri::command]
pub fn get_connector_health(
    state: State<'_, ConnectorState>,
) -> Result<ConnectorHealthResponse, String> {
    state.manager.get_connector_health()
}
