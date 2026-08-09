pub mod client;
pub mod commands;
pub mod config;
pub mod server;
pub mod tools;
pub mod types;
pub mod stress_tests;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use crate::db::DbPool;
pub use client::{McpClientManager, McpToolRegistry};
pub use server::McpServer;

pub struct McpState {
    pub client_manager: Arc<Mutex<McpClientManager>>,
    pub tool_registry: Arc<RwLock<McpToolRegistry>>,
    pub server: Arc<Mutex<McpServer>>,
    pub config_path: PathBuf,
    pub app_dir: PathBuf,
    pub db_pool: DbPool,
}

impl McpState {
    pub fn new(app_dir: PathBuf, db_pool: DbPool) -> Self {
        let config_path = app_dir.join("mcp_config.json");
        Self {
            client_manager: Arc::new(Mutex::new(McpClientManager::new())),
            tool_registry: Arc::new(RwLock::new(McpToolRegistry::new())),
            server: Arc::new(Mutex::new(McpServer::new())),
            config_path,
            app_dir,
            db_pool,
        }
    }
}
