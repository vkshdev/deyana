pub mod audit_log;
pub mod commands;
pub mod crypto;
pub mod firewall;
pub mod vault_key;

use std::path::Path;
use std::sync::RwLock;
use crate::db::DbPool;
pub use firewall::PrivacyRules;
pub use vault_key::VaultKeyState;


pub struct PrivacyState {
    pub db_pool: DbPool,
    pub rules: RwLock<PrivacyRules>,
    pub vault_key: VaultKeyState,
}

impl PrivacyState {
    pub fn new(db_pool: DbPool, app_dir: &Path) -> Self {
        if let Ok(conn) = db_pool.get() {
            let _ = audit_log::init_privacy_audit_logs(&conn);
        }

        let vault_key = VaultKeyState::load_or_init(app_dir).unwrap_or_default();

        Self {
            db_pool,
            rules: RwLock::new(PrivacyRules::default()),
            vault_key,
        }
    }
}

