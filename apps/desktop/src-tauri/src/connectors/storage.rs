use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const MASTER_KEY_SEED: &[u8] = b"deyana_connector_storage_secret_v1";

fn get_cipher() -> Aes256Gcm {
    let mut hasher = Sha256::new();
    hasher.update(MASTER_KEY_SEED);
    let key_bytes = hasher.finalize();
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    Aes256Gcm::new(key)
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err("invalid hex length".to_string());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|e| format!("invalid hex character: {e}"))
        })
        .collect()
}

pub fn encrypt_data(plaintext: &str) -> Result<(String, String), String> {
    let cipher = get_cipher();
    let uuid_bytes = Uuid::new_v4();
    let nonce_bytes = &uuid_bytes.as_bytes()[..12];
    let nonce = Nonce::from_slice(nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| format!("Encryption error: {e}"))?;

    Ok((bytes_to_hex(&ciphertext), bytes_to_hex(nonce_bytes)))
}

pub fn decrypt_data(encrypted_hex: &str, nonce_hex: &str) -> Result<String, String> {
    let cipher = get_cipher();
    let ciphertext = hex_to_bytes(encrypted_hex)?;
    let nonce_bytes = hex_to_bytes(nonce_hex)?;
    if nonce_bytes.len() != 12 {
        return Err("Invalid nonce length".to_string());
    }
    let nonce = Nonce::from_slice(&nonce_bytes);

    let decrypted_bytes = cipher
        .decrypt(nonce, ciphertext.as_slice())
        .map_err(|e| format!("Decryption error: {e}"))?;

    String::from_utf8(decrypted_bytes).map_err(|e| format!("UTF-8 conversion error: {e}"))
}

pub fn store_connector_token(
    conn: &Connection,
    connector_id: &str,
    token_json: &str,
    updated_at: &str,
) -> Result<(), String> {
    let (encrypted, nonce) = encrypt_data(token_json)?;
    conn.execute(
        "INSERT INTO connector_tokens (connector_id, encrypted_token, nonce, updated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(connector_id) DO UPDATE SET
           encrypted_token = excluded.encrypted_token,
           nonce = excluded.nonce,
           updated_at = excluded.updated_at",
        params![connector_id, encrypted, nonce, updated_at],
    )
    .map_err(|e| format!("DB store error: {e}"))?;
    Ok(())
}

pub fn get_connector_token(
    conn: &Connection,
    connector_id: &str,
) -> Result<Option<String>, String> {
    let mut stmt = conn
        .prepare("SELECT encrypted_token, nonce FROM connector_tokens WHERE connector_id = ?1")
        .map_err(|e| format!("DB prepare error: {e}"))?;

    let mut rows = stmt
        .query(params![connector_id])
        .map_err(|e| format!("DB query error: {e}"))?;

    if let Some(row) = rows.next().map_err(|e| format!("DB fetch error: {e}"))? {
        let encrypted_hex: String = row.get(0).map_err(|e| format!("DB get error: {e}"))?;
        let nonce_hex: String = row.get(1).map_err(|e| format!("DB get error: {e}"))?;
        let plaintext = decrypt_data(&encrypted_hex, &nonce_hex)?;
        Ok(Some(plaintext))
    } else {
        Ok(None)
    }
}

pub fn delete_connector_token(conn: &Connection, connector_id: &str) -> Result<bool, String> {
    let count = conn
        .execute(
            "DELETE FROM connector_tokens WHERE connector_id = ?1",
            params![connector_id],
        )
        .map_err(|e| format!("DB delete error: {e}"))?;
    Ok(count > 0)
}

#[cfg(test)]
#[path = "tests/storage_tests.rs"]
mod tests;


