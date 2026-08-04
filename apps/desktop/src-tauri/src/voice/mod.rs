pub mod commands;
pub mod service;
pub mod types;

use std::path::PathBuf;
use crate::db::DbPool;
pub use service::LocalVoiceService;

pub struct VoiceState {
    pub service: LocalVoiceService,
}

impl VoiceState {
    pub fn new(data_dir: PathBuf, db_pool: DbPool) -> Self {
        Self {
            service: LocalVoiceService::new(data_dir, db_pool),
        }
    }
}
