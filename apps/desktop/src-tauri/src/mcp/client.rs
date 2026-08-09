use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio::process::{Child, Command};
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};
use futures_util::{SinkExt, StreamExt};
use reqwest::Client as HttpClient;
use reqwest_eventsource::{Event, EventSource};
use serde_json::{json, Value};

use super::types::{
    JsonRpcRequest, JsonRpcResponse, McpServerConfig, McpServerStatus, McpTool,
    McpToolItem, McpTransportType,
};

#[derive(Default)]
pub struct McpToolRegistry {
    tools: HashMap<String, McpToolItem>,
}

impl McpToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register_tools(&mut self, server_id: &str, tools: Vec<McpTool>) {
        // First remove existing tools for this server
        self.unregister_tools(server_id);

        for tool in tools {
            let tool_id = format!("mcp::{}:{}", server_id, tool.name);
            let item = McpToolItem {
                id: tool_id.clone(),
                server_id: server_id.to_string(),
                name: tool.name,
                description: tool.description.unwrap_or_default(),
                input_schema: tool.input_schema,
            };
            self.tools.insert(tool_id, item);
        }
    }

    pub fn unregister_tools(&mut self, server_id: &str) {
        self.tools.retain(|_, v| v.server_id != server_id);
    }

    pub fn list_tools(&self) -> Vec<McpToolItem> {
        self.tools.values().cloned().collect()
    }

    pub fn get_tool(&self, tool_id: &str) -> Option<McpToolItem> {
        self.tools.get(tool_id).cloned()
    }
}

pub enum ClientTransport {
    Stdio {
        tx: mpsc::Sender<JsonRpcRequest>,
        pending: Arc<StdMutex<HashMap<String, oneshot::Sender<JsonRpcResponse>>>>,
        child: Arc<StdMutex<Child>>,
    },
    Sse {
        post_url: String,
        http_client: HttpClient,
    },
}

pub struct ActiveClient {
    pub config: McpServerConfig,
    pub transport: ClientTransport,
    pub tools: Vec<McpTool>,
}

impl ActiveClient {
    pub fn is_alive(&self) -> bool {
        match &self.transport {
            ClientTransport::Stdio { child, .. } => {
                let mut guard = match child.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                match guard.try_wait() {
                    Ok(None) => true,
                    Ok(Some(_)) => false,
                    Err(_) => false,
                }
            }
            ClientTransport::Sse { .. } => true,
        }
    }

    pub async fn send_rpc(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, String> {
        if !self.is_alive() {
            return Err("MCP client process has exited or is disconnected".to_string());
        }

        match &self.transport {
            ClientTransport::Stdio { tx, pending, .. } => {
                let req_id = match &request.id {
                    Some(id_val) => serde_json::to_string(id_val).map_err(|e| e.to_string())?,
                    None => {
                        tx.send(request)
                            .await
                            .map_err(|e| format!("Failed to send JSON-RPC notification: {}", e))?;
                        return Ok(JsonRpcResponse::success(None, json!({})));
                    }
                };

                let (resp_tx, resp_rx) = oneshot::channel::<JsonRpcResponse>();
                {
                    let mut map = pending.lock().unwrap();
                    map.insert(req_id.clone(), resp_tx);
                }

                if let Err(e) = tx.send(request).await {
                    let mut map = pending.lock().unwrap();
                    map.remove(&req_id);
                    return Err(format!("Failed to send JSON-RPC to stdio process: {}", e));
                }

                match resp_rx.await {
                    Ok(resp) => Ok(resp),
                    Err(_) => {
                        let mut map = pending.lock().unwrap();
                        map.remove(&req_id);
                        Err("Stdio channel closed before response was received".to_string())
                    }
                }
            }
            ClientTransport::Sse { post_url, http_client } => {
                let resp = http_client
                    .post(post_url)
                    .json(&request)
                    .send()
                    .await
                    .map_err(|e| format!("SSE POST request failed: {}", e))?;

                if !resp.status().is_success() {
                    return Err(format!("SSE POST returned error status: {}", resp.status()));
                }

                let json_resp: JsonRpcResponse = resp
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse SSE JSON-RPC response: {}", e))?;

                Ok(json_resp)
            }
        }
    }
}

#[derive(Default)]
pub struct McpClientManager {
    clients: HashMap<String, ActiveClient>,
}

impl McpClientManager {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    pub async fn connect_server(
        &mut self,
        config: McpServerConfig,
        registry: Arc<RwLock<McpToolRegistry>>,
    ) -> Result<usize, String> {
        let server_id = config.id.clone();

        // Disconnect first if already active
        let _ = self.disconnect_server(&server_id, registry.clone()).await;

        let transport = match config.transport_type {
            McpTransportType::Stdio => {
                let cmd_str = config
                    .command
                    .as_deref()
                    .ok_or_else(|| "Missing command for stdio server".to_string())?;

                let mut cmd = Command::new(cmd_str);
                if let Some(ref args) = config.args {
                    cmd.args(args);
                }
                if let Some(ref env) = config.env {
                    cmd.envs(env);
                }

                cmd.stdin(Stdio::piped());
                cmd.stdout(Stdio::piped());
                cmd.stderr(Stdio::null());
                cmd.kill_on_drop(true);

                let mut child = cmd
                    .spawn()
                    .map_err(|e| format!("Failed to spawn process '{}': {}", cmd_str, e))?;

                let stdin = child
                    .stdin
                    .take()
                    .ok_or_else(|| "Failed to capture child process stdin".to_string())?;
                let stdout = child
                    .stdout
                    .take()
                    .ok_or_else(|| "Failed to capture child process stdout".to_string())?;

                let mut framed_writer = FramedWrite::new(stdin, LinesCodec::new());
                let mut framed_reader = FramedRead::new(stdout, LinesCodec::new());

                let (tx_req, mut rx_req) = mpsc::channel::<JsonRpcRequest>(32);
                let pending: Arc<StdMutex<HashMap<String, oneshot::Sender<JsonRpcResponse>>>> =
                    Arc::new(StdMutex::new(HashMap::new()));
                let pending_reader = pending.clone();

                // Spawn writer loop
                tokio::spawn(async move {
                    while let Some(req) = rx_req.recv().await {
                        if let Ok(line) = serde_json::to_string(&req) {
                            if framed_writer.send(line).await.is_err() {
                                break;
                            }
                        }
                    }
                });

                // Spawn reader loop
                tokio::spawn(async move {
                    while let Some(Ok(line)) = framed_reader.next().await {
                        if let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(&line) {
                            if let Some(ref id_val) = resp.id {
                                if let Ok(key) = serde_json::to_string(id_val) {
                                    let sender = {
                                        let mut map = pending_reader.lock().unwrap();
                                        map.remove(&key)
                                    };
                                    if let Some(tx) = sender {
                                        let _ = tx.send(resp);
                                    }
                                }
                            }
                        }
                    }
                });

                ClientTransport::Stdio {
                    tx: tx_req,
                    pending,
                    child: Arc::new(StdMutex::new(child)),
                }
            }
            McpTransportType::Sse => {
                let url_str = config
                    .url
                    .as_deref()
                    .ok_or_else(|| "Missing URL for SSE server".to_string())?;

                let http_client = HttpClient::builder()
                    .timeout(std::time::Duration::from_secs(30))
                    .build()
                    .map_err(|e| e.to_string())?;

                let mut event_source = EventSource::get(url_str);
                let default_post_url = format!("{}/message", url_str.trim_end_matches("/sse").trim_end_matches('/'));

                // Try to read initial SSE event to get endpoint URL
                let default_url_clone = default_post_url.clone();
                let timeout_res = tokio::time::timeout(std::time::Duration::from_secs(5), async move {
                    while let Some(event_res) = event_source.next().await {
                        if let Ok(Event::Message(msg)) = event_res {
                            if msg.event == "endpoint" {
                                let rel = msg.data.trim();
                                if rel.starts_with("http://") || rel.starts_with("https://") {
                                    return rel.to_string();
                                } else {
                                    let base = url_str.trim_end_matches("/sse").trim_end_matches('/');
                                    let path = if rel.starts_with('/') { rel } else { &format!("/{}", rel) };
                                    return format!("{}{}", base, path);
                                }
                            }
                        }
                    }
                    default_url_clone
                }).await;

                let final_post_url = timeout_res.unwrap_or(default_post_url);

                ClientTransport::Sse {
                    post_url: final_post_url,
                    http_client,
                }
            }
        };

        let client = ActiveClient {
            config: config.clone(),
            transport,
            tools: Vec::new(),
        };

        // Handshake: 1. initialize
        let init_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "roots": { "listChanged": true }
                },
                "clientInfo": {
                    "name": "DeyanaDesktop",
                    "version": "0.1.0"
                }
            })),
        };

        let _init_resp = client.send_rpc(init_req).await?;

        // 2. notifications/initialized
        let notif_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: "notifications/initialized".to_string(),
            params: None,
        };
        let _ = client.send_rpc(notif_req).await;

        // 3. tools/list
        let tools_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/list".to_string(),
            params: Some(json!({})),
        };

        let tools_resp = client.send_rpc(tools_req).await?;
        let mut discovered_tools = Vec::new();

        if let Some(result) = tools_resp.result {
            if let Some(tools_arr) = result.get("tools").and_then(|v| v.as_array()) {
                for t in tools_arr {
                    if let Ok(tool) = serde_json::from_value::<McpTool>(t.clone()) {
                        discovered_tools.push(tool);
                    }
                }
            }
        }

        let tool_count = discovered_tools.len();

        // Update registry
        {
            let mut reg = registry.write().await;
            reg.register_tools(&server_id, discovered_tools.clone());
        }

        let mut active_client = client;
        active_client.tools = discovered_tools;

        self.clients.insert(server_id, active_client);

        Ok(tool_count)
    }

    pub async fn disconnect_server(
        &mut self,
        server_id: &str,
        registry: Arc<RwLock<McpToolRegistry>>,
    ) -> Result<(), String> {
        if self.clients.remove(server_id).is_some() {
            let mut reg = registry.write().await;
            reg.unregister_tools(server_id);
        }
        Ok(())
    }

    pub async fn call_tool(
        &self,
        server_id: &str,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value, String> {
        let client = self
            .clients
            .get(server_id)
            .ok_or_else(|| format!("Server '{}' is not connected", server_id))?;

        let call_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(3)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": tool_name,
                "arguments": arguments
            })),
        };

        let resp = client.send_rpc(call_req).await?;

        if let Some(err) = resp.error {
            return Err(format!("MCP Error {}: {}", err.code, err.message));
        }

        resp.result
            .ok_or_else(|| "Empty result from tool call".to_string())
    }

    pub fn get_statuses(&mut self) -> Vec<McpServerStatus> {
        self.clients.retain(|_, c| c.is_alive());
        self.clients
            .values()
            .map(|c| McpServerStatus {
                id: c.config.id.clone(),
                name: c.config.name.clone(),
                connected: true,
                transport_type: format!("{:?}", c.config.transport_type).to_lowercase(),
                tool_count: c.tools.len(),
                error: None,
            })
            .collect()
    }

    pub fn is_connected(&mut self, server_id: &str) -> bool {
        if let Some(client) = self.clients.get(server_id) {
            if !client.is_alive() {
                self.clients.remove(server_id);
                false
            } else {
                true
            }
        } else {
            false
        }
    }

    pub fn active_count(&mut self) -> usize {
        self.clients.retain(|_, c| c.is_alive());
        self.clients.len()
    }
}
