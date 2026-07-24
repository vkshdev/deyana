use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CredentialFile {
    endpoint: String,
    encrypted_credential: String,
}

#[derive(Debug, Deserialize)]
struct EncryptedEnvelope {
    provider: String,
    payload: String,
}

#[derive(Debug, Deserialize)]
struct BridgeSecret {
    token: String,
}

#[derive(Debug)]
pub struct BrowserCredential {
    pub endpoint: String,
    pub token: String,
}

pub fn load() -> Result<BrowserCredential, String> {
    let path = credential_path()?;
    let content = fs::read_to_string(&path)
        .map_err(|error| format!("unable to read {}: {error}", path.display()))?;
    let credential: CredentialFile = serde_json::from_str(&content)
        .map_err(|error| format!("browser credential file is invalid: {error}"))?;
    let encrypted: EncryptedEnvelope = serde_json::from_str(&credential.encrypted_credential)
        .map_err(|error| format!("encrypted browser credential is invalid: {error}"))?;
    if encrypted.provider != "windows-dpapi" {
        return Err(format!(
            "unsupported browser credential provider: {}",
            encrypted.provider
        ));
    }

    let ciphertext = STANDARD
        .decode(encrypted.payload)
        .map_err(|error| format!("browser credential payload is invalid: {error}"))?;
    let plaintext = unprotect(&ciphertext)?;
    let secret: BridgeSecret = serde_json::from_slice(&plaintext)
        .map_err(|error| format!("decrypted browser credential is invalid: {error}"))?;
    if secret.token.len() < 32 {
        return Err("decrypted browser credential token is too short".to_string());
    }
    Ok(BrowserCredential {
        endpoint: credential.endpoint,
        token: secret.token,
    })
}

fn credential_path() -> Result<PathBuf, String> {
    let app_data = env::var_os("APPDATA")
        .ok_or_else(|| "APPDATA is unavailable; browser bridge requires Windows user data".to_string())?;
    Ok(PathBuf::from(app_data)
        .join("app.deyana.desktop")
        .join("browser-bridge-credential.json"))
}

#[cfg(windows)]
fn unprotect(ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPT_INTEGER_BLOB,
    };
    use windows_sys::Win32::Foundation::LocalFree;

    let mut input = CRYPT_INTEGER_BLOB {
        cbData: ciphertext.len() as u32,
        pbData: ciphertext.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: null_mut(),
    };
    let result = unsafe {
        CryptUnprotectData(
            &mut input,
            null_mut(),
            null(),
            null_mut(),
            null(),
            0,
            &mut output,
        )
    };
    if result == 0 {
        return Err("Windows DPAPI could not decrypt the browser bridge credential".to_string());
    }

    let plaintext = unsafe {
        let bytes = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        LocalFree(output.pbData as *mut _);
        bytes
    };
    Ok(plaintext)
}

#[cfg(not(windows))]
fn unprotect(_ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    Err("The Phase 16 native browser host currently supports Windows DPAPI only.".to_string())
}
