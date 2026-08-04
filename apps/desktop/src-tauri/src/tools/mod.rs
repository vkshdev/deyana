pub mod commands;
pub mod service;
pub mod types;

use crate::db::DbPool;
pub use service::ToolService;

pub struct ToolState {
    pub service: ToolService,
}

impl ToolState {
    pub fn new(db_pool: DbPool) -> Self {
        Self {
            service: ToolService::new(db_pool),
        }
    }
}
