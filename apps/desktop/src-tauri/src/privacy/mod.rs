pub mod audit_log;
pub mod commands;
pub mod firewall;

use std::sync::RwLock;
use crate::db::DbPool;
pub use firewall::{PrivacyFirewall, PrivacyRules};

pub struct PrivacyState {
    pub db_pool: DbPool,
    pub rules: RwLock<PrivacyRules>,
}

impl PrivacyState {
    pub fn new(db_pool: DbPool) -> Self {
        if let Ok(conn) = db_pool.get() {
            let _ = audit_log::init_privacy_audit_logs(&conn);
        }

        Self {
            db_pool,
            rules: RwLock::new(PrivacyRules::default()),
        }
    }
}
