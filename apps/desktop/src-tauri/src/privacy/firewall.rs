use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyRules {
    pub local_only: bool,
    pub whitelisted_domains: Vec<String>,
    pub blacklisted_domains: Vec<String>,
    pub pii_redaction_enabled: bool,
    pub pii_rules: Vec<String>,
}

impl Default for PrivacyRules {
    fn default() -> Self {
        Self {
            local_only: true,
            whitelisted_domains: Vec::new(),
            blacklisted_domains: Vec::new(),
            pii_redaction_enabled: true,
            pii_rules: vec![
                "email".to_string(),
                "phone".to_string(),
                "ssn".to_string(),
                "credit_card".to_string(),
                "api_key".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyCheckRequest {
    pub url: String,
    pub method: Option<String>,
    pub purpose: Option<String>,
    pub data_category: Option<String>,
    pub payload_preview: Option<String>,
    pub user_approved: Option<bool>,
    pub connector_id: Option<String>,
    pub external_write: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyAuditEvent {
    pub id: String,
    pub event_type: String,
    pub decision: String,
    pub reason: String,
    pub destination: String,
    pub destination_category: String,
    pub data_category: String,
    pub purpose: String,
    pub method: String,
    pub risk_level: String,
    pub user_approved: bool,
    pub connector_id: Option<String>,
    pub safe_alternative: String,
    pub payload_sha256: Option<String>,
    pub payload_character_count: usize,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyCheckResponse {
    pub allowed: bool,
    pub decision: String,
    pub reason: String,
    pub destination: String,
    pub destination_category: String,
    pub data_category: String,
    pub purpose: String,
    pub safe_alternative: String,
    pub audit_event: PrivacyAuditEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyAuditListResponse {
    pub events: Vec<PrivacyAuditEvent>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyStatusResponse {
    pub mode: String,
    pub enforced: bool,
    pub audit_events: usize,
    pub blocked_events: usize,
    pub allowed_events: usize,
    pub last_blocked: Option<PrivacyAuditEvent>,
    pub blocked_categories: Vec<String>,
}

pub struct PrivacyFirewall;

impl PrivacyFirewall {
    pub fn evaluate(
        rules: &PrivacyRules,
        request: &PrivacyCheckRequest,
    ) -> (
        String, // decision ("allow" | "block")
        String, // reason
        String, // destination
        String, // destination_category
        String, // data_category
        String, // purpose
        String, // method
        String, // risk_level
        String, // safe_alternative
        Option<String>, // payload_sha256
        usize,  // payload_character_count
    ) {
        let method = request
            .method
            .as_deref()
            .unwrap_or("GET")
            .trim()
            .to_uppercase();
        let purpose = request
            .purpose
            .as_deref()
            .unwrap_or("unknown")
            .trim()
            .to_string();

        let destination = normalize_destination(&request.url);
        let destination_category = classify_destination(&destination, &purpose);
        let data_category = request
            .data_category
            .clone()
            .unwrap_or_else(|| classify_payload(request, &purpose));

        let payload_preview = request.payload_preview.as_deref().unwrap_or("");
        let payload_char_count = payload_preview.len();
        let payload_sha256 = if payload_preview.is_empty() {
            None
        } else {
            Some(sha256_hex(payload_preview))
        };

        let pii_detected = if rules.pii_redaction_enabled && !payload_preview.is_empty() {
            detect_pii(payload_preview, &rules.pii_rules)
        } else {
            false
        };

        let (decision, reason) = decide(
            rules,
            &destination,
            &destination_category,
            &data_category,
            &purpose,
            &method,
            request.user_approved.unwrap_or(false),
            request.external_write.unwrap_or(false),
        );

        let risk_level = calculate_risk_level(&decision, &destination_category, &data_category, pii_detected);
        let safe_alt = safe_alternative_for(&destination_category, &data_category);

        (
            decision,
            reason,
            destination,
            destination_category,
            data_category,
            purpose,
            method,
            risk_level,
            safe_alt,
            payload_sha256,
            payload_char_count,
        )
    }
}

pub fn normalize_destination(url: &str) -> String {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return "unknown".to_string();
    }
    trimmed.to_string()
}

pub fn classify_destination(destination: &str, purpose: &str) -> String {
    let lower = destination.to_lowercase();
    let host = extract_host(&lower);
    let path = extract_path(&lower);

    if lower.starts_with("file://") || is_local_host(&host) {
        return "local".to_string();
    }

    match purpose {
        "embedding" => return "hosted_embedding".to_string(),
        "reranking" => return "hosted_reranker".to_string(),
        "speech_to_text" => return "cloud_stt".to_string(),
        "text_to_speech" => return "cloud_tts".to_string(),
        "cloud_ai" => return "cloud_ai".to_string(),
        _ => {}
    }

    if is_cloud_ai_host(&host) {
        if path.contains("embedding") || path.contains("embeddings") {
            return "hosted_embedding".to_string();
        }
        if path.contains("rerank") {
            return "hosted_reranker".to_string();
        }
        if path.contains("audio/transcription") || path.contains("audio/translations") {
            return "cloud_stt".to_string();
        }
        if path.contains("audio/speech") {
            return "cloud_tts".to_string();
        }
        return "cloud_ai".to_string();
    }

    if is_embedding_host(&host) || path.contains("embedding") || path.contains("embeddings") {
        return "hosted_embedding".to_string();
    }
    if path.contains("rerank") {
        return "hosted_reranker".to_string();
    }
    if is_stt_host(&host) {
        return "cloud_stt".to_string();
    }
    if is_tts_host(&host) {
        return "cloud_tts".to_string();
    }
    if is_oauth_host(&host) {
        return "oauth_connector".to_string();
    }
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return "public_web".to_string();
    }

    "unknown_external".to_string()
}

fn extract_host(url: &str) -> String {
    let after_scheme = if let Some(pos) = url.find("://") {
        &url[pos + 3..]
    } else {
        url
    };
    let host_part = after_scheme
        .split('/')
        .next()
        .unwrap_or(after_scheme)
        .split('?')
        .next()
        .unwrap_or(after_scheme)
        .split('#')
        .next()
        .unwrap_or(after_scheme);
    
    // strip port if any
    let host_only = if host_part.starts_with('[') {
        // IPv6 bracket
        host_part.split(']').next().unwrap_or(host_part)
    } else {
        host_part.split(':').next().unwrap_or(host_part)
    };
    host_only.to_string()
}

fn extract_path(url: &str) -> String {
    if let Some(pos) = url.find("://") {
        let after_scheme = &url[pos + 3..];
        if let Some(slash_pos) = after_scheme.find('/') {
            return after_scheme[slash_pos..].to_string();
        }
    }
    String::new()
}

fn is_local_host(host: &str) -> bool {
    let local_hosts: HashSet<&str> = ["localhost", "127.0.0.1", "::1", "0.0.0.0"].into_iter().collect();
    local_hosts.contains(host) || host.ends_with(".localhost")
}

fn is_cloud_ai_host(host: &str) -> bool {
    let hosts = [
        "api.openai.com",
        "chatgpt.com",
        "api.anthropic.com",
        "api.groq.com",
        "api.mistral.ai",
        "api.cohere.ai",
        "api.together.xyz",
        "api.perplexity.ai",
        "api.deepseek.com",
        "openrouter.ai",
        "api.openrouter.ai",
        "generativelanguage.googleapis.com",
        "aiplatform.googleapis.com",
        "openai.azure.com",
    ];
    hosts.iter().any(|&known| host == known || host.ends_with(&format!(".{}", known)))
}

fn is_embedding_host(host: &str) -> bool {
    let hosts = ["api.jina.ai", "api.voyageai.com"];
    hosts.iter().any(|&known| host == known || host.ends_with(&format!(".{}", known)))
}

fn is_stt_host(host: &str) -> bool {
    let hosts = ["api.deepgram.com", "api.assemblyai.com", "api.rev.ai", "speech.googleapis.com"];
    hosts.iter().any(|&known| host == known || host.ends_with(&format!(".{}", known)))
}

fn is_tts_host(host: &str) -> bool {
    let hosts = ["api.elevenlabs.io", "api.play.ht", "texttospeech.googleapis.com"];
    hosts.iter().any(|&known| host == known || host.ends_with(&format!(".{}", known)))
}

fn is_oauth_host(host: &str) -> bool {
    let hosts = [
        "accounts.google.com",
        "oauth2.googleapis.com",
        "www.googleapis.com",
        "gmail.googleapis.com",
        "calendar-json.googleapis.com",
        "api.github.com",
        "github.com",
        "slack.com",
        "api.slack.com",
        "api.notion.com",
        "auth.atlassian.com",
        "api.atlassian.com",
        "linear.app",
        "api.linear.app",
    ];
    hosts.iter().any(|&known| host == known || host.ends_with(&format!(".{}", known)))
}

fn classify_payload(request: &PrivacyCheckRequest, purpose: &str) -> String {
    match purpose {
        "embedding" => return "embedding_text".to_string(),
        "speech_to_text" => return "audio".to_string(),
        "text_to_speech" => return "transcript".to_string(),
        "cloud_ai" => return "private_memory".to_string(),
        "oauth_api_fetch" => return "oauth_token".to_string(),
        "connector_api_fetch" => return "connector_metadata".to_string(),
        "public_web_fetch" => return "public_query".to_string(),
        _ => {}
    }

    let preview = request.payload_preview.as_deref().unwrap_or("").to_lowercase();
    if preview.is_empty() {
        return "unknown".to_string();
    }

    if ["source code", "private repo", "stack trace", "api_key"].iter().any(|k| preview.contains(k)) {
        return "source_code".to_string();
    }
    if ["gmail", "calendar", "slack", "notion", "connector"].iter().any(|k| preview.contains(k)) {
        return "connector_metadata".to_string();
    }
    if ["voice recording", "audio", "microphone"].iter().any(|k| preview.contains(k)) {
        return "audio".to_string();
    }
    if ["transcript", "dictation"].iter().any(|k| preview.contains(k)) {
        return "transcript".to_string();
    }
    if ["vault", "memory", "chat history", "summary", "private note"].iter().any(|k| preview.contains(k)) {
        return "memory_summary".to_string();
    }

    "public_query".to_string()
}

fn decide(
    rules: &PrivacyRules,
    destination: &str,
    destination_category: &str,
    data_category: &str,
    purpose: &str,
    method: &str,
    user_approved: bool,
    external_write: bool,
) -> (String, String) {
    let host = extract_host(&destination.to_lowercase());

    // 1. Blacklist check
    if rules.blacklisted_domains.iter().any(|domain| {
        let domain_lower = domain.to_lowercase();
        host == domain_lower || host.ends_with(&format!(".{}", domain_lower))
    }) {
        return ("block".to_string(), "Domain is blacklisted by privacy policy.".to_string());
    }

    // 2. Whitelist check (if whitelisted_domains non-empty and target is not local)
    if destination_category != "local" && !rules.whitelisted_domains.is_empty() {
        let is_whitelisted = rules.whitelisted_domains.iter().any(|domain| {
            let domain_lower = domain.to_lowercase();
            host == domain_lower || host.ends_with(&format!(".{}", domain_lower))
        });
        if !is_whitelisted {
            return ("block".to_string(), "Domain is not in whitelisted domains.".to_string());
        }
    }

    // 3. Local check
    if destination_category == "local" {
        return ("allow".to_string(), "Local destination is allowed.".to_string());
    }

    // 4. Blocked external categories in local-only mode
    if rules.local_only {
        let blocked_categories = [
            "cloud_ai",
            "hosted_embedding",
            "hosted_reranker",
            "cloud_stt",
            "cloud_tts",
        ];
        if blocked_categories.contains(&destination_category) {
            let reason = match destination_category {
                "cloud_ai" => "External AI model endpoints are blocked in local-only mode.",
                "hosted_embedding" => "Hosted embedding endpoints are blocked; embeddings must run locally.",
                "hosted_reranker" => "Hosted reranker endpoints are blocked; reranking must run locally.",
                "cloud_stt" => "Cloud speech-to-text endpoints are blocked; voice processing must run locally.",
                "cloud_tts" => "Cloud text-to-speech endpoints are blocked; speech output must run locally.",
                _ => "External destination is blocked in local-only mode.",
            };
            return ("block".to_string(), reason.to_string());
        }
    }

    let sensitive_data_categories: HashSet<&str> = [
        "private_memory",
        "memory_summary",
        "embedding_text",
        "audio",
        "transcript",
        "source_code",
        "local_file",
        "chat_history",
    ]
    .into_iter()
    .collect();

    // 5. OAuth Connector checks
    if destination_category == "oauth_connector" {
        if purpose != "oauth_api_fetch" && purpose != "connector_api_fetch" {
            return (
                "block".to_string(),
                "Connector endpoints are allowed only for approved OAuth or connector fetches.".to_string(),
            );
        }
        if !user_approved {
            return (
                "block".to_string(),
                "Connector/OAuth request requires explicit user approval.".to_string(),
            );
        }
        if external_write && method != "GET" && method != "HEAD" {
            return (
                "block".to_string(),
                "External writes require a later confirmation flow before they can run.".to_string(),
            );
        }
        let allowed_oauth_data = ["oauth_token", "connector_metadata", "public_query", "public_content", "unknown"];
        if !allowed_oauth_data.contains(&data_category) {
            return (
                "block".to_string(),
                "Sensitive private payload cannot be sent to connector/OAuth endpoints.".to_string(),
            );
        }
        return ("allow".to_string(), "Approved connector/OAuth request is allowed.".to_string());
    }

    // 6. Public Web checks
    if destination_category == "public_web" {
        if purpose != "public_web_fetch" {
            return (
                "block".to_string(),
                "Public web access is limited to explicit public web fetch requests.".to_string(),
            );
        }
        if method != "GET" && method != "HEAD" {
            return (
                "block".to_string(),
                "Public web fetch is read-only in this phase.".to_string(),
            );
        }
        let allowed_web_data = ["public_query", "public_content", "unknown"];
        if !allowed_web_data.contains(&data_category) {
            return (
                "block".to_string(),
                "Sensitive private payload cannot be sent to public web endpoints.".to_string(),
            );
        }
        return ("allow".to_string(), "Public web fetch is allowed.".to_string());
    }

    // 7. Sensitive data check
    if sensitive_data_categories.contains(data_category) {
        return (
            "block".to_string(),
            "Sensitive local data cannot leave the device.".to_string(),
        );
    }

    ("block".to_string(), "Unknown external destination is blocked until a policy explicitly allows it.".to_string())
}

fn detect_pii(payload: &str, pii_rules: &[String]) -> bool {
    let lower = payload.to_lowercase();
    for rule in pii_rules {
        match rule.to_lowercase().as_str() {
            "email" => {
                if lower.contains('@') && lower.contains('.') {
                    return true;
                }
            }
            "phone" => {
                if payload.chars().filter(|c| c.is_ascii_digit()).count() >= 10 {
                    return true;
                }
            }
            "ssn" => {
                if lower.contains("ssn") || lower.contains("social security") {
                    return true;
                }
            }
            "credit_card" => {
                if lower.contains("card") || lower.contains("cvv") || lower.contains("credit") {
                    return true;
                }
            }
            "api_key" => {
                if lower.contains("api_key") || lower.contains("secret") || lower.contains("bearer ") || lower.contains("sk-") {
                    return true;
                }
            }
            custom => {
                if lower.contains(custom) {
                    return true;
                }
            }
        }
    }
    false
}

fn calculate_risk_level(
    decision: &str,
    destination_category: &str,
    data_category: &str,
    pii_detected: bool,
) -> String {
    if pii_detected {
        return "critical".to_string();
    }
    if decision == "block" && (destination_category == "cloud_ai" || data_category == "private_memory" || data_category == "source_code") {
        return "high".to_string();
    }
    if decision == "block" || destination_category == "public_web" || destination_category == "oauth_connector" {
        return "medium".to_string();
    }
    "low".to_string()
}

pub fn safe_alternative_for(destination_category: &str, data_category: &str) -> String {
    match destination_category {
        "cloud_ai" => "Use local Ollama model".to_string(),
        "hosted_embedding" => "Use local embedding model".to_string(),
        "hosted_reranker" => "Use local retrieval and scoring".to_string(),
        "cloud_stt" => "Use local STT".to_string(),
        "cloud_tts" => "Use local TTS".to_string(),
        _ => {
            if ["private_memory", "memory_summary", "source_code", "local_file", "chat_history"].contains(&data_category) {
                "Keep private data local".to_string()
            } else {
                "Request explicit approval or add a specific policy".to_string()
            }
        }
    }
}

fn sha256_hex(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
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
}


