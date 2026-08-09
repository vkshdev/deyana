use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::{Query, State},
    response::sse::{Event, Sse},
    routing::{get, post},
    Json, Router,
};
use futures_util::stream::{self, Stream};
use tokio::sync::{mpsc, oneshot, RwLock};
use serde_json::json;
use uuid::Uuid;

use crate::db::DbPool;
use super::types::{JsonRpcRequest, JsonRpcResponse, McpTool};
use super::tools;

#[derive(Clone)]
pub struct ServerAppState {
    pub app_dir: PathBuf,
    pub db_pool: DbPool,
    pub sessions: Arc<RwLock<HashMap<String, mpsc::Sender<Event>>>>,
}

pub struct McpServer {
    pub running: bool,
    pub port: u16,
    shutdown_tx: Option<oneshot::Sender<()>>,
    pub sessions: Arc<RwLock<HashMap<String, mpsc::Sender<Event>>>>,
}

impl McpServer {
    pub fn new() -> Self {
        Self {
            running: false,
            port: 3030,
            shutdown_tx: None,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn session_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    pub async fn start(
        &mut self,
        port: u16,
        app_dir: PathBuf,
        db_pool: DbPool,
    ) -> Result<u16, String> {
        if self.running {
            return Ok(self.port);
        }

        let state = ServerAppState {
            app_dir,
            db_pool,
            sessions: self.sessions.clone(),
        };

        let router = Router::new()
            .route("/sse", get(sse_handler))
            .route("/message", post(message_handler))
            .with_state(state);

        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| format!("Failed to bind MCP server to port {}: {}", port, e))?;

        let actual_port = listener
            .local_addr()
            .map_err(|e| e.to_string())?
            .port();

        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        tokio::spawn(async move {
            let server = axum::serve(listener, router).with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            });
            let _ = server.await;
        });

        self.running = true;
        self.port = actual_port;
        self.shutdown_tx = Some(shutdown_tx);

        Ok(actual_port)
    }

    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        self.running = false;
        let sessions = self.sessions.clone();
        tokio::spawn(async move {
            let mut map = sessions.write().await;
            map.clear();
        });
    }
}

#[derive(serde::Deserialize)]
pub struct MessageQuery {
    pub session_id: Option<String>,
}

struct SessionDropGuard {
    session_id: String,
    sessions: Arc<RwLock<HashMap<String, mpsc::Sender<Event>>>>,
}

impl Drop for SessionDropGuard {
    fn drop(&mut self) {
        let session_id = self.session_id.clone();
        let sessions = self.sessions.clone();
        tokio::spawn(async move {
            let mut map = sessions.write().await;
            map.remove(&session_id);
        });
    }
}

async fn sse_handler(
    State(state): State<ServerAppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let session_id = Uuid::new_v4().to_string();
    let (tx, rx) = mpsc::channel::<Event>(32);

    {
        let mut sessions = state.sessions.write().await;
        sessions.insert(session_id.clone(), tx.clone());
    }

    // Send initial endpoint discovery event
    let endpoint_event = Event::default()
        .event("endpoint")
        .data(format!("/message?session_id={}", session_id));
    let _ = tx.send(endpoint_event).await;

    let guard = SessionDropGuard {
        session_id,
        sessions: state.sessions.clone(),
    };

    let sse_stream = stream::unfold((rx, guard), |(mut rx, guard)| async move {
        let ev = rx.recv().await?;
        Some((Ok::<Event, Infallible>(ev), (rx, guard)))
    });

    Sse::new(sse_stream).keep_alive(axum::response::sse::KeepAlive::new())
}

async fn message_handler(
    State(state): State<ServerAppState>,
    Query(_query): Query<MessageQuery>,
    Json(req): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    let response = process_json_rpc(&state, req).await;
    Json(response)
}

async fn process_json_rpc(state: &ServerAppState, req: JsonRpcRequest) -> JsonRpcResponse {
    match req.method.as_str() {
        "initialize" => JsonRpcResponse::success(
            req.id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "deyana-mcp-server",
                    "version": "0.1.0"
                }
            }),
        ),
        "notifications/initialized" => {
            JsonRpcResponse::success(req.id, json!({}))
        }
        "tools/list" => {
            let tools_list = vec![
                McpTool {
                    name: "search_deyana_memory".to_string(),
                    description: Some("Search Deyana's internal SQLite memory database using hybrid full-text and semantic retrieval.".to_string()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "query": { "type": "string", "description": "Search query keywords or concept" },
                            "limit": { "type": "integer", "description": "Maximum memory items to return (default 5)" }
                        },
                        "required": ["query"]
                    }),
                },
                McpTool {
                    name: "read_vault_note".to_string(),
                    description: Some("Read a markdown note from Deyana's local vault by relative path or note title.".to_string()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Relative file path (e.g. 'project/plan.md') or note title" }
                        },
                        "required": ["path"]
                    }),
                },
                McpTool {
                    name: "create_vault_note".to_string(),
                    description: Some("Create or overwrite a markdown note in Deyana's vault and index it into Deyana's memory database.".to_string()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "title": { "type": "string", "description": "Title of the vault note" },
                            "content": { "type": "string", "description": "Markdown content for the note" },
                            "folder": { "type": "string", "description": "Optional subfolder inside vault (e.g. 'notes')" },
                            "tags": { "type": "array", "items": { "type": "string" }, "description": "Optional list of tags" }
                        },
                        "required": ["title", "content"]
                    }),
                },
            ];

            JsonRpcResponse::success(req.id, json!({ "tools": tools_list }))
        }
        "tools/call" => {
            let params = match req.params {
                Some(ref p) => p,
                None => return JsonRpcResponse::error(req.id, -32602, "Missing parameters"),
            };

            let tool_name = match params.get("name").and_then(|v| v.as_str()) {
                Some(n) => n,
                None => return JsonRpcResponse::error(req.id, -32602, "Missing parameter 'name'"),
            };

            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

            let result = match tool_name {
                "search_deyana_memory" => {
                    tools::execute_search_deyana_memory(&state.db_pool, &arguments)
                }
                "read_vault_note" => {
                    tools::execute_read_vault_note(&state.app_dir, &arguments)
                }
                "create_vault_note" => {
                    tools::execute_create_vault_note(&state.app_dir, &state.db_pool, &arguments)
                }
                _ => Err(format!("Unknown tool: {}", tool_name)),
            };

            match result {
                Ok(res) => match serde_json::to_value(res) {
                    Ok(val) => JsonRpcResponse::success(req.id, val),
                    Err(e) => JsonRpcResponse::error(req.id, -32603, e.to_string()),
                },
                Err(e) => JsonRpcResponse::error(req.id, -32603, e),
            }
        }
        _ => JsonRpcResponse::error(req.id, -32601, format!("Method not found: {}", req.method)),
    }
}
