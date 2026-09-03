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
