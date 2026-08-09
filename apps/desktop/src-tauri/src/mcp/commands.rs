use tauri::State;
use serde_json::Value;

use super::config::{load_configs, parse_claude_desktop_config, save_configs};
use super::types::{
    McpServerConfig, McpServerStatus, McpStatusResponse, McpToolItem,
};
use super::McpState;

#[tauri::command]
pub async fn mcp_get_status(
    state: State<'_, McpState>,
) -> Result<McpStatusResponse, String> {
    let server_guard = state.server.lock().await;
    let mut client_guard = state.client_manager.lock().await;
    let tool_guard = state.tool_registry.read().await;

    let active_servers = client_guard.get_statuses();

    Ok(McpStatusResponse {
        server_running: server_guard.running,
        server_port: server_guard.port,
        connected_clients: client_guard.active_count(),
        active_servers,
        total_tools: tool_guard.list_tools().len() + 3, // 3 exposed local tools
    })
}

#[tauri::command]
pub async fn mcp_start_server(
    state: State<'_, McpState>,
    port: Option<u16>,
) -> Result<McpServerStatus, String> {
    let target_port = port.unwrap_or(3030);
    let mut server_guard = state.server.lock().await;

    let _actual_port = server_guard
        .start(target_port, state.app_dir.clone(), state.db_pool.clone())
        .await?;

    Ok(McpServerStatus {
        id: "deyana-local-server".to_string(),
        name: "Deyana Memory & Vault SSE Server".to_string(),
        connected: true,
        transport_type: "sse".to_string(),
        tool_count: 3,
        error: None,
    })
}

#[tauri::command]
pub async fn mcp_stop_server(
    state: State<'_, McpState>,
) -> Result<McpServerStatus, String> {
    let mut server_guard = state.server.lock().await;
    server_guard.stop();

    Ok(McpServerStatus {
        id: "deyana-local-server".to_string(),
        name: "Deyana Memory & Vault SSE Server".to_string(),
        connected: false,
        transport_type: "sse".to_string(),
        tool_count: 0,
        error: None,
    })
}

#[tauri::command]
pub async fn mcp_list_servers(
    state: State<'_, McpState>,
) -> Result<Vec<McpServerStatus>, String> {
    let configs = load_configs(&state.config_path)?;
    let mut client_guard = state.client_manager.lock().await;

    let mut result = Vec::new();
    for config in configs {
        let is_connected = client_guard.is_connected(&config.id);
        result.push(McpServerStatus {
            id: config.id.clone(),
            name: config.name.clone(),
            connected: is_connected,
            transport_type: format!("{:?}", config.transport_type).to_lowercase(),
            tool_count: if is_connected {
                client_guard
                    .get_statuses()
                    .iter()
                    .find(|s| s.id == config.id)
                    .map(|s| s.tool_count)
                    .unwrap_or(0)
            } else {
                0
            },
            error: None,
        });
    }

    Ok(result)
}

#[tauri::command]
pub async fn mcp_save_server(
    state: State<'_, McpState>,
    config: McpServerConfig,
) -> Result<(), String> {
    let mut configs = load_configs(&state.config_path).unwrap_or_default();
    configs.retain(|c| c.id != config.id);
    configs.push(config);

    save_configs(&state.config_path, &configs)
}

#[tauri::command]
pub async fn mcp_delete_server(
    state: State<'_, McpState>,
    id: String,
) -> Result<(), String> {
    let mut client_guard = state.client_manager.lock().await;
    let _ = client_guard
        .disconnect_server(&id, state.tool_registry.clone())
        .await;

    let mut configs = load_configs(&state.config_path).unwrap_or_default();
    configs.retain(|c| c.id != id);

    save_configs(&state.config_path, &configs)
}

#[tauri::command]
pub async fn mcp_connect_server(
    state: State<'_, McpState>,
    id: String,
) -> Result<McpServerStatus, String> {
    let configs = load_configs(&state.config_path)?;
    let config = configs
        .into_iter()
        .find(|c| c.id == id)
        .ok_or_else(|| format!("Server configuration for '{}' not found", id))?;

    let mut client_guard = state.client_manager.lock().await;
    let tool_count = client_guard
        .connect_server(config.clone(), state.tool_registry.clone())
        .await?;

    Ok(McpServerStatus {
        id: config.id,
        name: config.name,
        connected: true,
        transport_type: format!("{:?}", config.transport_type).to_lowercase(),
        tool_count,
        error: None,
    })
}

#[tauri::command]
pub async fn mcp_disconnect_server(
    state: State<'_, McpState>,
    id: String,
) -> Result<McpServerStatus, String> {
    let mut client_guard = state.client_manager.lock().await;
    let _ = client_guard
        .disconnect_server(&id, state.tool_registry.clone())
        .await;

    Ok(McpServerStatus {
        id: id.clone(),
        name: id,
        connected: false,
        transport_type: "unknown".to_string(),
        tool_count: 0,
        error: None,
    })
}

#[tauri::command]
pub async fn mcp_list_tools(
    state: State<'_, McpState>,
) -> Result<Vec<McpToolItem>, String> {
    let tool_guard = state.tool_registry.read().await;
    let mut tools = tool_guard.list_tools();

    // Include local server tools
    tools.push(McpToolItem {
        id: "mcp::deyana::search_deyana_memory".to_string(),
        server_id: "deyana-local-server".to_string(),
        name: "search_deyana_memory".to_string(),
        description: "Search Deyana's internal SQLite memory database using hybrid full-text and semantic retrieval.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query keywords or concept" },
                "limit": { "type": "integer", "description": "Maximum memory items to return" }
            },
            "required": ["query"]
        }),
    });

    tools.push(McpToolItem {
        id: "mcp::deyana::read_vault_note".to_string(),
        server_id: "deyana-local-server".to_string(),
        name: "read_vault_note".to_string(),
        description: "Read a markdown note from Deyana's local vault by relative path or note title.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Relative file path or note title" }
            },
            "required": ["path"]
        }),
    });

    tools.push(McpToolItem {
        id: "mcp::deyana::create_vault_note".to_string(),
        server_id: "deyana-local-server".to_string(),
        name: "create_vault_note".to_string(),
        description: "Create or overwrite a markdown note in Deyana's vault and index it into Deyana's memory database.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "Title of the vault note" },
                "content": { "type": "string", "description": "Markdown content for the note" },
                "folder": { "type": "string", "description": "Optional subfolder inside vault" },
                "tags": { "type": "array", "items": { "type": "string" }, "description": "Optional list of tags" }
            },
            "required": ["title", "content"]
        }),
    });

    Ok(tools)
}

#[tauri::command]
pub async fn mcp_call_tool(
    state: State<'_, McpState>,
    server_id: String,
    tool_name: String,
    arguments: Value,
) -> Result<Value, String> {
    if server_id == "deyana-local-server" || server_id == "deyana" {
        match tool_name.as_str() {
            "search_deyana_memory" => {
                let res = super::tools::execute_search_deyana_memory(&state.db_pool, &arguments)?;
                return serde_json::to_value(res).map_err(|e| e.to_string());
            }
            "read_vault_note" => {
                let res = super::tools::execute_read_vault_note(&state.app_dir, &arguments)?;
                return serde_json::to_value(res).map_err(|e| e.to_string());
            }
            "create_vault_note" => {
                let res = super::tools::execute_create_vault_note(&state.app_dir, &state.db_pool, &arguments)?;
                return serde_json::to_value(res).map_err(|e| e.to_string());
            }
            _ => return Err(format!("Unknown local Deyana MCP tool: {}", tool_name)),
        }
    }

    let client_guard = state.client_manager.lock().await;
    client_guard
        .call_tool(&server_id, &tool_name, arguments)
        .await
}

#[tauri::command]
pub async fn mcp_import_config(
    state: State<'_, McpState>,
    config_json: String,
) -> Result<usize, String> {
    let new_configs = parse_claude_desktop_config(&config_json)?;
    let imported_count = new_configs.len();

    let mut existing_configs = load_configs(&state.config_path).unwrap_or_default();

    for cfg in new_configs {
        existing_configs.retain(|c| c.id != cfg.id);
        existing_configs.push(cfg);
    }

    save_configs(&state.config_path, &existing_configs)?;

    Ok(imported_count)
}
