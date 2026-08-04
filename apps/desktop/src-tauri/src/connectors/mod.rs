pub mod commands;
pub mod manager;
pub mod storage;
pub mod types;

pub use manager::ConnectorManager;

use crate::db::DbPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct ConnectorState {
    pub manager: Arc<ConnectorManager>,
}

impl ConnectorState {
    pub fn new(pool: DbPool) -> Self {
        let manager = ConnectorManager::new(pool);
        let _ = manager.ensure_connectors_registered();
        Self {
            manager: Arc::new(manager),
        }
    }
}
