use chrono::Utc;
use uuid::Uuid;

use super::audit_log;
use super::firewall::{
    PrivacyAuditEvent, PrivacyAuditListResponse, PrivacyCheckRequest, PrivacyCheckResponse,
    PrivacyFirewall, PrivacyRules, PrivacyStatusResponse,
};
use super::PrivacyState;

#[tauri::command]
pub async fn evaluate_privacy_policy(
    state: tauri::State<'_, PrivacyState>,
    request: PrivacyCheckRequest,
) -> Result<PrivacyCheckResponse, String> {
    let rules = state.rules.read().map_err(|e| e.to_string())?.clone();

    let (
        decision,
        reason,
        destination,
        destination_category,
        data_category,
        purpose,
        method,
        risk_level,
        safe_alternative,
        payload_sha256,
        payload_character_count,
    ) = PrivacyFirewall::evaluate(&rules, &request);

    let event_type = if decision == "block" {
        "privacy.request.blocked".to_string()
    } else {
        "privacy.request.allowed".to_string()
    };

    let audit_event = PrivacyAuditEvent {
        id: format!("privacy_{}", Uuid::new_v4().simple()),
        event_type,
        decision: decision.clone(),
        reason: reason.clone(),
        destination: destination.clone(),
        destination_category: destination_category.clone(),
        data_category: data_category.clone(),
        purpose: purpose.clone(),
        method,
        risk_level,
        user_approved: request.user_approved.unwrap_or(false),
        connector_id: request.connector_id.clone(),
        safe_alternative: safe_alternative.clone(),
        payload_sha256,
        payload_character_count,
        created_at: Utc::now().to_rfc3339(),
    };

    let conn = state.db_pool.get().map_err(|e| e.to_string())?;
    let _ = audit_log::record_audit_event(&conn, &audit_event);

    Ok(PrivacyCheckResponse {
        allowed: decision == "allow",
        decision,
        reason,
        destination,
        destination_category,
        data_category,
        purpose,
        safe_alternative,
        audit_event,
    })
}

#[tauri::command]
pub async fn get_privacy_audit_logs(
    state: tauri::State<'_, PrivacyState>,
    limit: Option<usize>,
) -> Result<PrivacyAuditListResponse, String> {
    let conn = state.db_pool.get().map_err(|e| e.to_string())?;
    let limit_val = limit.unwrap_or(50);
    let events = audit_log::get_privacy_audit_logs(&conn, limit_val).map_err(|e| e.to_string())?;
    let total = audit_log::get_total_audit_events_count(&conn).unwrap_or(events.len());

    Ok(PrivacyAuditListResponse { events, total })
}

#[tauri::command]
pub async fn get_privacy_status(
    state: tauri::State<'_, PrivacyState>,
) -> Result<PrivacyStatusResponse, String> {
    let rules = state.rules.read().map_err(|e| e.to_string())?;
    let mode = if rules.local_only { "local_only" } else { "custom" };

    let conn = state.db_pool.get().map_err(|e| e.to_string())?;
    audit_log::get_privacy_status(&conn, mode).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_privacy_rules(
    state: tauri::State<'_, PrivacyState>,
    rules: PrivacyRules,
) -> Result<PrivacyRules, String> {
    let mut current_rules = state.rules.write().map_err(|e| e.to_string())?;
    *current_rules = rules.clone();
    Ok(rules)
}
