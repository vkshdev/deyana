use serde::Deserialize;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AllowedOrigins {
    allowed_origins: Vec<String>,
}

pub fn caller_origin() -> Result<String, String> {
    let origin = env::args()
        .skip(1)
        .find(|argument| argument.starts_with("chrome-extension://"))
        .ok_or_else(|| "browser did not provide an extension origin".to_string())?;
    validate_format(&origin)?;
    let allowed = load_allowed_origins()?;
    if !allowed.iter().any(|candidate| candidate == &origin) {
        return Err(format!("extension origin is not allowed: {origin}"));
    }
    Ok(origin)
}

fn load_allowed_origins() -> Result<Vec<String>, String> {
    let executable = env::current_exe()
        .map_err(|error| format!("unable to locate native host executable: {error}"))?;
    let path = executable
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| "native host executable has no parent directory".to_string())?
        .join("browser-native-origins.json");
    let content = fs::read_to_string(&path)
        .map_err(|error| format!("unable to read {}: {error}", path.display()))?;
    let config: AllowedOrigins = serde_json::from_str(&content)
        .map_err(|error| format!("allowed origins file is invalid: {error}"))?;
    if config.allowed_origins.is_empty() {
        return Err("allowed origins file contains no extension origins".to_string());
    }
    for origin in &config.allowed_origins {
        validate_format(origin)?;
    }
    Ok(config.allowed_origins)
}

fn validate_format(origin: &str) -> Result<(), String> {
    let id = origin
        .strip_prefix("chrome-extension://")
        .and_then(|value| value.strip_suffix('/'))
        .ok_or_else(|| "extension origin has an invalid scheme or suffix".to_string())?;
    if id.len() != 32 || !id.bytes().all(|value| matches!(value, b'a'..=b'p')) {
        return Err("extension ID must contain exactly 32 lowercase a-p characters".to_string());
    }
    Ok(())
}
