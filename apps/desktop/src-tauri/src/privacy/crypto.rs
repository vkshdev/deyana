use std::fs;
use std::path::Path;
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};

pub const ENCRYPTED_PREFIX: &str = "ENC::";
pub const KEY_FILENAME: &str = "vault.key.enc";


/// Derive or load the 256-bit vault master key from disk using DPAPI.
pub fn load_or_create_vault_key(app_dir: &Path) -> Result<[u8; 32], String> {
    let _ = fs::create_dir_all(app_dir);
    let key_file = app_dir.join(KEY_FILENAME);

    if key_file.exists() {
        let encrypted_bytes = fs::read(&key_file)
            .map_err(|e| format!("Failed to read vault key file: {e}"))?;
        
        let decrypted = unprotect_key_bytes(&encrypted_bytes)?;
        if decrypted.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&decrypted);
            return Ok(key);
        } else {
            return Err("Decrypted vault key has invalid length (expected 32 bytes)".to_string());
        }
    }

    // Generate fresh 32-byte master key
    let raw_key = Aes256Gcm::generate_key(OsRng);
    let raw_slice = raw_key.as_slice();

    let protected = protect_key_bytes(raw_slice)?;
    fs::write(&key_file, &protected)
        .map_err(|e| format!("Failed to persist encrypted vault key: {e}"))?;

    let mut key = [0u8; 32];
    key.copy_from_slice(raw_slice);
    Ok(key)
}

pub fn protect_key_bytes(raw_key: &[u8]) -> Result<Vec<u8>, String> {
    #[cfg(windows)]
    {
        dpapi_protect(raw_key)
    }
    #[cfg(not(windows))]
    {
        use sha2::{Digest, Sha256};
        let salt = b"DEYANA_VAULT_FALLBACK_SALT";
        let mut hasher = Sha256::new();
        hasher.update(salt);
        hasher.update(raw_key);
        let hash = hasher.finalize();
        let mut out = raw_key.to_vec();
        for (i, b) in out.iter_mut().enumerate() {
            *b ^= hash[i % hash.len()];
        }
        Ok(out)
    }
}

pub fn unprotect_key_bytes(protected_data: &[u8]) -> Result<Vec<u8>, String> {
    #[cfg(windows)]
    {
        dpapi_unprotect(protected_data)
    }
    #[cfg(not(windows))]
    {
        use sha2::{Digest, Sha256};
        let salt = b"DEYANA_VAULT_FALLBACK_SALT";
        let mut hasher = Sha256::new();
        hasher.update(salt);
        hasher.update(protected_data);
        let hash = hasher.finalize();
        let mut out = protected_data.to_vec();
        for (i, b) in out.iter_mut().enumerate() {
            *b ^= hash[i % hash.len()];
        }
        Ok(out)
    }
}


#[cfg(windows)]
fn dpapi_protect(data: &[u8]) -> Result<Vec<u8>, String> {
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };
    use windows::Win32::Foundation::LocalFree;

    let mut blob_in = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut blob_out = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };

    unsafe {
        CryptProtectData(
            &mut blob_in,
            windows::core::PCWSTR::null(),
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut blob_out,
        )
        .map_err(|e| format!("DPAPI CryptProtectData failed: {e}"))?;

        let slice = std::slice::from_raw_parts(blob_out.pbData, blob_out.cbData as usize).to_vec();
        let _ = LocalFree(windows::Win32::Foundation::HLOCAL(blob_out.pbData as _));
        Ok(slice)
    }
}

#[cfg(windows)]
fn dpapi_unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
    use windows::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };
    use windows::Win32::Foundation::LocalFree;

    let mut blob_in = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut blob_out = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };

    unsafe {
        CryptUnprotectData(
            &mut blob_in,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut blob_out,
        )
        .map_err(|e| format!("DPAPI CryptUnprotectData failed: {e}"))?;

        let slice = std::slice::from_raw_parts(blob_out.pbData, blob_out.cbData as usize).to_vec();
        let _ = LocalFree(windows::Win32::Foundation::HLOCAL(blob_out.pbData as _));
        Ok(slice)
    }
}


/// Check if a string contains our encrypted ciphertext prefix
pub fn is_encrypted(value: &str) -> bool {
    value.starts_with(ENCRYPTED_PREFIX)
}

/// AES-256-GCM field encryption.
/// Formats output as: `ENC::<hex_nonce>::<hex_ciphertext_and_tag>`
pub fn encrypt_field(key_bytes: &[u8; 32], plaintext: &str) -> Result<String, String> {
    let key = Key::<Aes256Gcm>::from_slice(key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| format!("AES-256-GCM encryption failed: {e}"))?;

    let nonce_hex = hex_encode(nonce.as_slice());
    let ct_hex = hex_encode(&ciphertext);

    Ok(format!("{ENCRYPTED_PREFIX}{nonce_hex}::{ct_hex}"))
}

/// AES-256-GCM field decryption.
/// If string does not start with `ENC::`, returns plaintext string transparently!
pub fn decrypt_field(key_bytes: &[u8; 32], stored_value: &str) -> Result<String, String> {
    if !is_encrypted(stored_value) {
        // Plaintext row (backward compatibility)
        return Ok(stored_value.to_string());
    }

    let payload = &stored_value[ENCRYPTED_PREFIX.len()..];
    let parts: Vec<&str> = payload.split("::").collect();
    if parts.len() != 2 {
        return Err("Malformed encrypted field format (expected nonce::ciphertext)".to_string());
    }

    let nonce_bytes = hex_decode(parts[0])
        .map_err(|e| format!("Invalid hex nonce: {e}"))?;
    let ct_bytes = hex_decode(parts[1])
        .map_err(|e| format!("Invalid hex ciphertext: {e}"))?;

    if nonce_bytes.len() != 12 {
        return Err(format!("Invalid nonce length (expected 12 bytes, got {})", nonce_bytes.len()));
    }

    let key = Key::<Aes256Gcm>::from_slice(key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let decrypted = cipher
        .decrypt(nonce, ct_bytes.as_slice())
        .map_err(|e| format!("AES-256-GCM decryption failed: {e}"))?;

    String::from_utf8(decrypted).map_err(|e| format!("Decrypted bytes are not valid UTF-8: {e}"))
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 {
        return Err("Hex string has odd length".to_string());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

#[cfg(test)]
#[path = "tests/crypto_tests.rs"]
mod tests;

