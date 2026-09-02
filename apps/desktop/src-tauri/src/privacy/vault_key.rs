use std::path::Path;
use std::sync::Arc;

use super::crypto::{decrypt_field, encrypt_field, load_or_create_vault_key};

#[derive(Clone)]
pub struct VaultKeyState {
    key: Arc<[u8; 32]>,
}

impl Default for VaultKeyState {
    fn default() -> Self {
        Self {
            key: Arc::new([0x5Du8; 32]),
        }
    }
}


impl VaultKeyState {
    pub fn new(key: [u8; 32]) -> Self {
        Self {
            key: Arc::new(key),
        }
    }

    pub fn load_or_init(app_dir: &Path) -> Result<Self, String> {
        let key = load_or_create_vault_key(app_dir)?;
        Ok(Self::new(key))
    }

    pub fn key(&self) -> &[u8; 32] {
        &self.key
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String, String> {
        encrypt_field(&self.key, plaintext)
    }

    pub fn decrypt(&self, stored_value: &str) -> Result<String, String> {
        decrypt_field(&self.key, stored_value)
    }
}
