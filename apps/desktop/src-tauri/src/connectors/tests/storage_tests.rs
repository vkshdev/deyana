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
        assert_eq!(nonce.len(), 24);

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

    let err1 = decrypt_data("abc", &nonce);
    assert!(err1.is_err() && err1.unwrap_err().contains("invalid hex length"));

    let err2 = decrypt_data(&encrypted, "ZZZZZZZZZZZZZZZZZZZZZZZZ");
    assert!(err2.is_err() && err2.unwrap_err().contains("invalid hex character"));

    let short_nonce = "001122334455";
    let err3 = decrypt_data(&encrypted, short_nonce);
    assert!(err3.is_err() && err3.unwrap_err().contains("Invalid nonce length"));

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

    assert_eq!(get_connector_token(&conn, "gmail").unwrap(), None);

    let token_v1 = "{\"accessToken\":\"access_v1\"}";
    store_connector_token(&conn, "gmail", token_v1, "2026-08-03T10:00:00Z").unwrap();
    assert_eq!(get_connector_token(&conn, "gmail").unwrap(), Some(token_v1.to_string()));

    let token_v2 = "{\"accessToken\":\"access_v2\"}";
    store_connector_token(&conn, "gmail", token_v2, "2026-08-03T11:00:00Z").unwrap();
    assert_eq!(get_connector_token(&conn, "gmail").unwrap(), Some(token_v2.to_string()));

    let mut stmt = conn.prepare("SELECT encrypted_token, nonce FROM connector_tokens WHERE connector_id = 'gmail'").unwrap();
    let row = stmt.query_row([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))).unwrap();
    assert_ne!(row.0, token_v2);
    assert!(!row.0.contains("access_v2"));

    assert!(delete_connector_token(&conn, "gmail").unwrap());
    assert_eq!(get_connector_token(&conn, "gmail").unwrap(), None);
    assert!(!delete_connector_token(&conn, "gmail").unwrap());
}
