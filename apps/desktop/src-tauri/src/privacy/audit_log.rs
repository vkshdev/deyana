use rusqlite::{params, Connection, Result};

use super::firewall::{PrivacyAuditEvent, PrivacyStatusResponse};

pub fn init_privacy_audit_logs(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS privacy_audit_logs (
            id TEXT PRIMARY KEY,
            event_type TEXT NOT NULL,
            decision TEXT NOT NULL,
            reason TEXT NOT NULL,
            target_domain TEXT NOT NULL,
            destination_category TEXT NOT NULL,
            data_category TEXT NOT NULL,
            purpose TEXT NOT NULL,
            method TEXT NOT NULL,
            risk_level TEXT NOT NULL DEFAULT 'low',
            user_approved INTEGER NOT NULL DEFAULT 0,
            connector_id TEXT,
            safe_alternative TEXT NOT NULL,
            payload_sha256 TEXT,
            payload_character_count INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_privacy_audit_logs_created_at ON privacy_audit_logs(created_at);
        CREATE INDEX IF NOT EXISTS idx_privacy_audit_logs_decision ON privacy_audit_logs(decision);",
    )?;
    Ok(())
}

pub fn record_audit_event(conn: &Connection, event: &PrivacyAuditEvent) -> Result<()> {
    conn.execute(
        "INSERT INTO privacy_audit_logs (
            id, event_type, decision, reason, target_domain,
            destination_category, data_category, purpose, method, risk_level,
            user_approved, connector_id, safe_alternative, payload_sha256,
            payload_character_count, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            &event.id,
            &event.event_type,
            &event.decision,
            &event.reason,
            &event.destination,
            &event.destination_category,
            &event.data_category,
            &event.purpose,
            &event.method,
            &event.risk_level,
            if event.user_approved { 1 } else { 0 },
            &event.connector_id,
            &event.safe_alternative,
            &event.payload_sha256,
            event.payload_character_count as i64,
            &event.created_at,
        ],
    )?;
    Ok(())
}

pub fn get_privacy_audit_logs(conn: &Connection, limit: usize) -> Result<Vec<PrivacyAuditEvent>> {
    let max_limit = limit.clamp(1, 200) as i64;
    let mut stmt = conn.prepare(
        "SELECT id, event_type, decision, reason, target_domain,
                destination_category, data_category, purpose, method, risk_level,
                user_approved, connector_id, safe_alternative, payload_sha256,
                payload_character_count, created_at
         FROM privacy_audit_logs
         ORDER BY created_at DESC
         LIMIT ?1",
    )?;

    let rows = stmt.query_map(params![max_limit], |row| {
        let id: String = row.get(0)?;
        let event_type: String = row.get(1)?;
        let decision: String = row.get(2)?;
        let reason: String = row.get(3)?;
        let destination: String = row.get(4)?;
        let destination_category: String = row.get(5)?;
        let data_category: String = row.get(6)?;
        let purpose: String = row.get(7)?;
        let method: String = row.get(8)?;
        let risk_level: String = row.get(9)?;
        let user_approved_int: i32 = row.get(10)?;
        let connector_id: Option<String> = row.get(11)?;
        let safe_alternative: String = row.get(12)?;
        let payload_sha256: Option<String> = row.get(13)?;
        let payload_char_count_i64: i64 = row.get(14)?;
        let created_at: String = row.get(15)?;

        Ok(PrivacyAuditEvent {
            id,
            event_type,
            decision,
            reason,
            destination,
            destination_category,
            data_category,
            purpose,
            method,
            risk_level,
            user_approved: user_approved_int != 0,
            connector_id,
            safe_alternative,
            payload_sha256,
            payload_character_count: payload_char_count_i64 as usize,
            created_at,
        })
    })?;

    let mut events = Vec::new();
    for r in rows {
        events.push(r?);
    }
    Ok(events)
}

pub fn get_total_audit_events_count(conn: &Connection) -> Result<usize> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM privacy_audit_logs", [], |row| row.get(0))?;
    Ok(count as usize)
}

pub fn get_privacy_status(conn: &Connection, mode: &str) -> Result<PrivacyStatusResponse> {
    let total = get_total_audit_events_count(conn).unwrap_or(0);

    let blocked_events: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM privacy_audit_logs WHERE decision = 'block'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let allowed_events: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM privacy_audit_logs WHERE decision = 'allow'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let last_blocked = conn
        .query_row(
            "SELECT id, event_type, decision, reason, target_domain,
                    destination_category, data_category, purpose, method, risk_level,
                    user_approved, connector_id, safe_alternative, payload_sha256,
                    payload_character_count, created_at
             FROM privacy_audit_logs
             WHERE decision = 'block'
             ORDER BY created_at DESC
             LIMIT 1",
            [],
            |row| {
                let id: String = row.get(0)?;
                let event_type: String = row.get(1)?;
                let decision: String = row.get(2)?;
                let reason: String = row.get(3)?;
                let destination: String = row.get(4)?;
                let destination_category: String = row.get(5)?;
                let data_category: String = row.get(6)?;
                let purpose: String = row.get(7)?;
                let method: String = row.get(8)?;
                let risk_level: String = row.get(9)?;
                let user_approved_int: i32 = row.get(10)?;
                let connector_id: Option<String> = row.get(11)?;
                let safe_alternative: String = row.get(12)?;
                let payload_sha256: Option<String> = row.get(13)?;
                let payload_char_count_i64: i64 = row.get(14)?;
                let created_at: String = row.get(15)?;

                Ok(PrivacyAuditEvent {
                    id,
                    event_type,
                    decision,
                    reason,
                    destination,
                    destination_category,
                    data_category,
                    purpose,
                    method,
                    risk_level,
                    user_approved: user_approved_int != 0,
                    connector_id,
                    safe_alternative,
                    payload_sha256,
                    payload_character_count: payload_char_count_i64 as usize,
                    created_at,
                })
            },
        )
        .ok();

    let blocked_categories = vec![
        "cloud_ai".to_string(),
        "hosted_embedding".to_string(),
        "hosted_reranker".to_string(),
        "cloud_stt".to_string(),
        "cloud_tts".to_string(),
    ];

    Ok(PrivacyStatusResponse {
        mode: mode.to_string(),
        enforced: true,
        audit_events: total,
        blocked_events: blocked_events as usize,
        allowed_events: allowed_events as usize,
        last_blocked,
        blocked_categories,
    })
}

pub fn clear_privacy_audit_logs(conn: &Connection) -> Result<usize> {
    let count = conn.execute("DELETE FROM privacy_audit_logs", [])?;
    Ok(count)
}
