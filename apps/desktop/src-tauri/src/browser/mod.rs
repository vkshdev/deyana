pub mod commands;
pub mod service;
pub mod types;

use std::path::PathBuf;
use crate::db::DbPool;
pub use service::NativeBrowserService;

pub struct BrowserState {
    pub service: NativeBrowserService,
}

impl BrowserState {
    pub fn new(data_dir: PathBuf, db_pool: DbPool) -> Self {
        Self {
            service: NativeBrowserService::new(data_dir, db_pool),
        }
    }
}

#[cfg(test)]
#[path = "tests/browser_tests.rs"]
mod tests;

