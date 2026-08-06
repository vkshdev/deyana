use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

use crate::db::DbPool;

#[derive(Debug, Clone)]
pub enum DaemonEvent {
    ProcessEvents,
    HealthCheck,
    Shutdown,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProactiveContextCard {
    pub id: String,
    pub card_type: String,
    pub title: String,
    pub summary: String,
    pub action_label: Option<String>,
    pub action_command: Option<String>,
    pub priority: String,
    pub timestamp: String,
    pub context: serde_json::Value,
}

#[cfg(target_os = "windows")]
pub mod win32 {
    use std::ffi::c_int;

    pub type HWND = *mut std::ffi::c_void;

    #[link(name = "user32")]
    extern "system" {
        pub fn GetForegroundWindow() -> HWND;
        pub fn GetWindowTextW(hwnd: HWND, lpString: *mut u16, nMaxCount: c_int) -> c_int;
    }

    pub fn get_active_window_title() -> Option<String> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                return None;
            }
            let mut buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), 512);
            if len > 0 {
                let title = String::from_utf16_lossy(&buf[..len as usize]);
                let trimmed = title.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            } else {
                None
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub mod win32 {
    pub fn get_active_window_title() -> Option<String> {
        None
    }
}

pub struct DaemonManager {
    sender: mpsc::Sender<DaemonEvent>,
    is_running: Arc<AtomicBool>,
    app_handle: tauri::AppHandle,
}

impl DaemonManager {
    pub fn spawn(app_handle: tauri::AppHandle, db_pool: DbPool) -> Self {
        let (tx, mut rx) = mpsc::channel::<DaemonEvent>(100);
        let is_running = Arc::new(AtomicBool::new(true));
        let is_running_clone = is_running.clone();
        let app_handle_loop = app_handle.clone();
        let daemon_app_handle = app_handle.clone();

        tokio::spawn(async move {
            let mut health_ticker = interval(Duration::from_secs(30));
            let mut window_ticker = interval(Duration::from_secs(5));
            let mut context_ticker = interval(Duration::from_secs(60));
            let mut last_window_title: Option<String> = None;

            println!("[Daemon] Background daemon loop started.");

            while is_running_clone.load(Ordering::Relaxed) {
                tokio::select! {
                    _ = health_ticker.tick() => {
                        let pool = db_pool.clone();
                        tokio::task::spawn_blocking(move || {
                            Self::perform_health_check(&pool);
                        });
                    }
                    _ = window_ticker.tick() => {
                        if let Some(current_title) = win32::get_active_window_title() {
                            let title_changed = match &last_window_title {
                                Some(prev) => prev != &current_title,
                                None => true,
                            };

                            if title_changed {
                                last_window_title = Some(current_title.clone());
                                let card = ProactiveContextCard {
                                    id: format!("card_window_{}", chrono::Utc::now().timestamp_millis()),
                                    card_type: "active_window_insight".to_string(),
                                    title: format!("Active Window: {}", current_title),
                                    summary: format!("Detected focus switch to: {}", current_title),
                                    action_label: Some("View Context".to_string()),
                                    action_command: Some("show_main_window".to_string()),
                                    priority: "normal".to_string(),
                                    timestamp: chrono::Utc::now().to_rfc3339(),
                                    context: serde_json::json!({
                                        "windowTitle": current_title,
                                        "event": "window_focus_change"
                                    }),
                                };
                                let _ = app_handle_loop.emit("proactive:context_card", &card);
                            }
                        }
                    }
                    _ = context_ticker.tick() => {
                        let pool = db_pool.clone();
                        let app_emitter = app_handle_loop.clone();
                        tokio::task::spawn_blocking(move || {
                            let active_title = win32::get_active_window_title();
                            let now_str = chrono::Utc::now().to_rfc3339();
                            let (insights, triage_items) = Self::poll_db_context(&pool);

                            let card = ProactiveContextCard {
                                id: format!("card_context_{}", chrono::Utc::now().timestamp_millis()),
                                card_type: "proactive_context".to_string(),
                                title: "Proactive Sentinel Scan".to_string(),
                                summary: format!(
                                    "Active window: {}. Open action items: {}. Urgent pending items: {}.",
                                    active_title.as_deref().unwrap_or("Desktop / Unknown"),
                                    insights.len(),
                                    triage_items.len()
                                ),
                                action_label: Some("Open Assistant".to_string()),
                                action_command: Some("show_main_window".to_string()),
                                priority: if triage_items.iter().any(|t| t.urgency_score == "URGENT") {
                                    "urgent".to_string()
                                } else {
                                    "normal".to_string()
                                },
                                timestamp: now_str.clone(),
                                context: serde_json::json!({
                                    "activeWindow": active_title,
                                    "openActionItemsCount": insights.len(),
                                    "pendingTriageCount": triage_items.len(),
                                    "actionItems": insights.iter().map(|i| serde_json::json!({
                                        "id": i.id,
                                        "title": i.title,
                                        "dueAt": i.due_at
                                    })).collect::<Vec<_>>(),
                                    "triageItems": triage_items.iter().map(|t| serde_json::json!({
                                        "id": t.id,
                                        "platform": t.platform,
                                        "sender": t.sender,
                                        "urgency": t.urgency_score
                                    })).collect::<Vec<_>>()
                                }),
                            };
                            let _ = app_emitter.emit("proactive:context_card", &card);

                            for item in &triage_items {
                                if item.urgency_score == "URGENT" {
                                    let triage_card = ProactiveContextCard {
                                        id: format!("card_triage_{}", item.id),
                                        card_type: "urgent_triage".to_string(),
                                        title: format!("Urgent Message from {}", item.sender),
                                        summary: item.content.clone(),
                                        action_label: Some("Review Message".to_string()),
                                        action_command: Some("show_main_window".to_string()),
                                        priority: "urgent".to_string(),
                                        timestamp: item.created_at.clone(),
                                        context: serde_json::json!({
                                            "triageId": item.id,
                                            "platform": item.platform,
                                            "sender": item.sender,
                                        }),
                                    };
                                    let _ = app_emitter.emit("proactive:context_card", &triage_card);
                                }
                            }

                            for insight in &insights {
                                if insight.r#type == "action_item" {
                                    let insight_card = ProactiveContextCard {
                                        id: format!("card_insight_{}", insight.id),
                                        card_type: "calendar_reminder".to_string(),
                                        title: format!("Reminder: {}", insight.title),
                                        summary: insight.detail.clone(),
                                        action_label: Some("View Action Item".to_string()),
                                        action_command: Some("show_main_window".to_string()),
                                        priority: "normal".to_string(),
                                        timestamp: insight.created_at.clone(),
                                        context: serde_json::json!({
                                            "insightId": insight.id,
                                            "dueAt": insight.due_at,
                                        }),
                                    };
                                    let _ = app_emitter.emit("proactive:context_card", &insight_card);
                                }
                            }
                        });
                    }
                    event_opt = rx.recv() => {
                        match event_opt {
                            Some(DaemonEvent::ProcessEvents) => {
                                println!("[Daemon] Processing periodic background events...");
                            }
                            Some(DaemonEvent::HealthCheck) => {
                                let pool = db_pool.clone();
                                tokio::task::spawn_blocking(move || {
                                    Self::perform_health_check(&pool);
                                });
                            }
                            Some(DaemonEvent::Shutdown) => {
                                println!("[Daemon] Shutting down daemon...");
                                is_running_clone.store(false, Ordering::Relaxed);
                                break;
                            }
                            None => {
                                println!("[Daemon] Channel closed, exiting daemon loop...");
                                is_running_clone.store(false, Ordering::Relaxed);
                                break;
                            }
                        }
                    }
                }
            }
            println!("[Daemon] Background daemon loop stopped.");
        });

        Self {
            sender: tx,
            is_running,
            app_handle: daemon_app_handle,
        }
    }

    fn poll_db_context(
        db_pool: &DbPool,
    ) -> (
        Vec<crate::db::models::MemoryInsight>,
        Vec<crate::db::models::TriageMessageResponse>,
    ) {
        let mut insights = Vec::new();
        let mut triage_items = Vec::new();

        if let Ok(conn) = db_pool.get() {
            if let Ok(mut stmt) = conn.prepare(
                "SELECT id, memory_id, type, title, detail, status, due_at, created_at FROM memory_insights WHERE status = 'open' ORDER BY created_at DESC LIMIT 10",
            ) {
                if let Ok(rows) = stmt.query_map([], |r| {
                    Ok(crate::db::models::MemoryInsight {
                        id: r.get(0)?,
                        memory_id: r.get(1)?,
                        memory_title: None,
                        source_type: None,
                        source_id: None,
                        source_uri: None,
                        r#type: r.get(2)?,
                        title: r.get(3)?,
                        detail: r.get(4)?,
                        status: r.get(5)?,
                        due_at: r.get(6)?,
                        created_at: r.get(7)?,
                    })
                }) {
                    for row in rows.flatten() {
                        insights.push(row);
                    }
                }
            }

            if let Ok(mut stmt) = conn.prepare(
                "SELECT id, platform, sender, content, urgency_score, auto_draft, status, created_at FROM triage_inbox WHERE status = 'pending' ORDER BY created_at DESC LIMIT 10",
            ) {
                if let Ok(rows) = stmt.query_map([], |r| {
                    Ok(crate::db::models::TriageMessageResponse {
                        id: r.get(0)?,
                        platform: r.get(1)?,
                        sender: r.get(2)?,
                        content: r.get(3)?,
                        urgency_score: r.get(4)?,
                        auto_draft: r.get(5)?,
                        status: r.get(6)?,
                        created_at: r.get(7)?,
                    })
                }) {
                    for row in rows.flatten() {
                        triage_items.push(row);
                    }
                }
            }
        }

        (insights, triage_items)
    }

    fn perform_health_check(db_pool: &DbPool) {
        match db_pool.get() {
            Ok(conn) => {
                if let Err(e) = crate::db::check_health(&conn) {
                    eprintln!("[Daemon] DB health check failed: {}", e);
                }
            }
            Err(e) => {
                eprintln!("[Daemon] Failed to get connection for health check: {}", e);
            }
        }
    }

    pub fn app_handle(&self) -> &tauri::AppHandle {
        &self.app_handle
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    pub async fn trigger_event(&self, event: DaemonEvent) -> Result<(), String> {
        self.sender
            .send(event)
            .await
            .map_err(|e| format!("Failed to send daemon event: {}", e))
    }
}

