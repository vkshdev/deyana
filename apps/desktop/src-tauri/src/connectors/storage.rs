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
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption_roundtrip() {
        let test_cases = vec![
            "simple_token_12345",
            "{\"accessToken\":\"secret_abc\",\"refreshToken\":\"refresh_xyz\",\"expiresIn\":3600}",
            "Unicode: 🔐 Token с кириллицей & emoji 🚀",
            "",
        ];

        for original in test_cases {
            let (encrypted, nonce) = encrypt_data(original).expect("encryption should succeed");
            assert!(!encrypted.is_empty());
            assert_eq!(nonce.len(), 24); // 12 bytes = 24 hex characters

            let decrypted = decrypt_data(&encrypted, &nonce).expect("decryption should succeed");
            assert_eq!(decrypted, original);
        }
    }

    #[test]
    fn test_nonce_uniqueness() {
        let plaintext = "identical_secret_payload";
        let (enc1, nonce1) = encrypt_data(plaintext).unwrap();
        let (enc2, nonce2) = encrypt_data(plaintext).unwrap();

        assert_ne!(nonce1, nonce2, "Nonces must be distinct for each encryption");
        assert_ne!(enc1, enc2, "Ciphertexts must be distinct due to unique nonces");

        assert_eq!(decrypt_data(&enc1, &nonce1).unwrap(), plaintext);
        assert_eq!(decrypt_data(&enc2, &nonce2).unwrap(), plaintext);
    }

    #[test]
    fn test_invalid_inputs_and_tampered_data() {
        let (encrypted, nonce) = encrypt_data("valid_data").unwrap();

        // Invalid hex length
        let err1 = decrypt_data("abc", &nonce);
        assert!(err1.is_err() && err1.unwrap_err().contains("invalid hex length"));

        // Invalid hex characters
        let err2 = decrypt_data(&encrypted, "ZZZZZZZZZZZZZZZZZZZZZZZZ");
        assert!(err2.is_err() && err2.unwrap_err().contains("invalid hex character"));

        // Nonce length != 12 bytes (24 hex chars)
        let short_nonce = "001122334455"; // 6 bytes
        let err3 = decrypt_data(&encrypted, short_nonce);
        assert!(err3.is_err() && err3.unwrap_err().contains("Invalid nonce length"));

        // Tampered ciphertext (flip last character)
        let mut tampered_enc = encrypted.clone();
        let last_char = tampered_enc.pop().unwrap();
        let replacement = if last_char == '0' { '1' } else { '0' };
        tampered_enc.push(replacement);

        let err4 = decrypt_data(&tampered_enc, &nonce);
        assert!(err4.is_err() && err4.unwrap_err().contains("Decryption error"));
    }

    #[test]
    fn test_sqlite_token_storage() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE connector_tokens (
                connector_id TEXT PRIMARY KEY,
                encrypted_token TEXT NOT NULL,
                nonce TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        ).unwrap();

        // 1. Initially empty
        assert_eq!(get_connector_token(&conn, "gmail").unwrap(), None);

        // 2. Store token
        let token_v1 = "{\"accessToken\":\"access_v1\"}";
        store_connector_token(&conn, "gmail", token_v1, "2026-08-03T10:00:00Z").unwrap();
        assert_eq!(get_connector_token(&conn, "gmail").unwrap(), Some(token_v1.to_string()));

        // 3. Update token (ON CONFLICT UPDATE)
        let token_v2 = "{\"accessToken\":\"access_v2\"}";
        store_connector_token(&conn, "gmail", token_v2, "2026-08-03T11:00:00Z").unwrap();
        assert_eq!(get_connector_token(&conn, "gmail").unwrap(), Some(token_v2.to_string()));

        // Verify direct DB values are encrypted hex, not plaintext
        let mut stmt = conn.prepare("SELECT encrypted_token, nonce FROM connector_tokens WHERE connector_id = 'gmail'").unwrap();
        let row = stmt.query_row([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))).unwrap();
        assert_ne!(row.0, token_v2);
        assert!(!row.0.contains("access_v2"));

        // 4. Delete token
        assert!(delete_connector_token(&conn, "gmail").unwrap());
        assert_eq!(get_connector_token(&conn, "gmail").unwrap(), None);
        assert!(!delete_connector_token(&conn, "gmail").unwrap());
    }
}

