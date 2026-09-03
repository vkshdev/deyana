use super::*;

#[test]
fn test_local_only_blocks_cloud_ai() {
    let rules = PrivacyRules::default();
    let req = PrivacyCheckRequest {
        url: "https://api.openai.com/v1/chat/completions".to_string(),
        method: Some("POST".to_string()),
        purpose: Some("cloud_ai".to_string()),
        data_category: None,
        payload_preview: Some("Hello world".to_string()),
        user_approved: None,
        connector_id: None,
        external_write: None,
    };

    let (decision, reason, _, dest_cat, _, _, _, _, safe_alt, sha, char_cnt) =
        PrivacyFirewall::evaluate(&rules, &req);

    assert_eq!(decision, "block");
    assert_eq!(dest_cat, "cloud_ai");
    assert!(reason.contains("local-only mode"));
    assert_eq!(safe_alt, "Use local Ollama model");
    assert!(sha.is_some());
    assert_eq!(char_cnt, 11);
}

#[test]
fn test_blacklisted_domain() {
    let mut rules = PrivacyRules::default();
    rules.blacklisted_domains.push("badsite.com".to_string());
    let req = PrivacyCheckRequest {
        url: "https://badsite.com/api".to_string(),
        method: Some("GET".to_string()),
        purpose: Some("public_web_fetch".to_string()),
        data_category: None,
        payload_preview: None,
        user_approved: None,
        connector_id: None,
        external_write: None,
    };

    let (decision, reason, _, _, _, _, _, _, _, _, _) = PrivacyFirewall::evaluate(&rules, &req);
    assert_eq!(decision, "block");
    assert!(reason.contains("blacklisted"));
}

#[test]
fn test_sha256_hex() {
    assert_eq!(
        sha256_hex("test"),
        "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
    );
}
