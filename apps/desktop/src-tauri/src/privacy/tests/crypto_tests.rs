use super::*;

#[test]
fn test_aes_gcm_encrypt_decrypt_roundtrip() {
    let key = [42u8; 32];
    let original_text = "This is a highly confidential Deyana memory note with sensitive data.";

    let encrypted = encrypt_field(&key, original_text).unwrap();
    assert!(is_encrypted(&encrypted));
    assert!(!encrypted.contains("confidential"));

    let decrypted = decrypt_field(&key, &encrypted).unwrap();
    assert_eq!(decrypted, original_text);
}

#[test]
fn test_plaintext_passthrough_on_non_encrypted() {
    let key = [42u8; 32];
    let plaintext = "Legacy unencrypted message from old database";
    assert!(!is_encrypted(plaintext));

    let decrypted = decrypt_field(&key, plaintext).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_hex_encode_decode() {
    let original = vec![0x00, 0x01, 0x0f, 0xff, 0x42, 0xaa];
    let encoded = hex_encode(&original);
    assert_eq!(encoded, "00010fff42aa");
    let decoded = hex_decode(&encoded).unwrap();
    assert_eq!(decoded, original);
}
