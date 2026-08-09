use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde_json::Value;
use super::types::{McpServerConfig, McpTransportType};

pub fn load_configs(path: &Path) -> Result<Vec<McpServerConfig>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read MCP config file: {}", e))?;
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse MCP config JSON: {}", e))
}

pub fn save_configs(path: &Path, configs: &[McpServerConfig]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    let json = serde_json::to_string_pretty(configs)
        .map_err(|e| format!("Failed to serialize MCP configs: {}", e))?;
    fs::write(path, json)
        .map_err(|e| format!("Failed to write MCP config file: {}", e))
}

pub fn parse_claude_desktop_config(json_str: &str) -> Result<Vec<McpServerConfig>, String> {
    let root: Value = serde_json::from_str(json_str)
        .map_err(|e| format!("Invalid JSON format: {}", e))?;

    let servers_map = if let Some(mcp_servers) = root.get("mcpServers") {
        mcp_servers.as_object().ok_or_else(|| "'mcpServers' must be a JSON object".to_string())?
    } else if let Some(obj) = root.as_object() {
        obj
    } else {
        return Err("JSON root must be an object".to_string());
    };

    let mut result = Vec::new();

    for (server_key, server_val) in servers_map {
        let obj = match server_val.as_object() {
            Some(o) => o,
            None => continue,
        };

        if let Some(cmd_val) = obj.get("command").and_then(|v| v.as_str()) {
            let args = obj.get("args").and_then(|v| v.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect()
            });

            let env = obj.get("env").and_then(|v| v.as_object()).map(|env_map| {
                let mut map = HashMap::new();
                for (k, v) in env_map {
                    if let Some(val_str) = v.as_str() {
                        map.insert(k.clone(), val_str.to_string());
                    }
                }
                map
            });

            result.push(McpServerConfig {
                id: server_key.clone(),
                name: server_key.clone(),
                transport_type: McpTransportType::Stdio,
                command: Some(cmd_val.to_string()),
                args,
                env,
                url: None,
                enabled: true,
                auto_connect: true,
            });
        } else if let Some(url_val) = obj.get("url").and_then(|v| v.as_str()) {
            result.push(McpServerConfig {
                id: server_key.clone(),
                name: server_key.clone(),
                transport_type: McpTransportType::Sse,
                command: None,
                args: None,
                env: None,
                url: Some(url_val.to_string()),
                enabled: true,
                auto_connect: true,
            });
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_claude_desktop_config_stdio_and_sse() {
        let json = r#"{
            "mcpServers": {
                "filesystem": {
                    "command": "npx",
                    "args": ["-y", "@modelcontextprotocol/server-filesystem", "C:\\docs"],
                    "env": { "NODE_ENV": "production" }
                },
                "remote_sse": {
                    "url": "http://localhost:8080/sse"
                }
            }
        }"#;

        let configs = parse_claude_desktop_config(json).expect("Failed to parse config");
        assert_eq!(configs.len(), 2);

        let fs_config = configs.iter().find(|c| c.id == "filesystem").unwrap();
        assert_eq!(fs_config.transport_type, McpTransportType::Stdio);
        assert_eq!(fs_config.command.as_deref(), Some("npx"));
        assert_eq!(
            fs_config.args.as_ref().unwrap(),
            &vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem".to_string(),
                "C:\\docs".to_string()
            ]
        );

        let sse_config = configs.iter().find(|c| c.id == "remote_sse").unwrap();
        assert_eq!(sse_config.transport_type, McpTransportType::Sse);
        assert_eq!(sse_config.url.as_deref(), Some("http://localhost:8080/sse"));
    }
}

