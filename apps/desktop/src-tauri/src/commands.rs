use std::{path::PathBuf, process::Command};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::{db, models, process, settings, window};

#[tauri::command]
pub fn get_desktop_settings(app: AppHandle) -> settings::DesktopSettings {
    settings::read_settings(&app)
}

#[tauri::command]
pub fn set_floating_mode(app: AppHandle, mode: String) -> Result<settings::DesktopSettings, String> {
    if mode != "compact" && mode != "expanded" {
        return Err(format!("unsupported floating mode: {mode}"));
    }

    window::set_mode(&app, &mode)?;
    settings::update_settings(&app, |settings| {
        settings.ui_mode = mode;
    })
}

#[tauri::command]
pub fn set_always_on_top(
    app: AppHandle,
    always_on_top: bool,
) -> Result<settings::DesktopSettings, String> {
    if let Some(window) = app.get_webview_window("main") {
        window
            .set_always_on_top(always_on_top)
            .map_err(|error| error.to_string())?;
    }

    settings::update_settings(&app, |settings| {
        settings.always_on_top = always_on_top;
    })
}

#[tauri::command]
pub fn set_low_power_mode(
    app: AppHandle,
    low_power_mode: bool,
) -> Result<settings::DesktopSettings, String> {
    settings::update_settings(&app, |settings| {
        settings.low_power_mode = low_power_mode;
    })
}

#[tauri::command]
pub fn set_reduce_motion(
    app: AppHandle,
    reduce_motion: bool,
) -> Result<settings::DesktopSettings, String> {
    settings::update_settings(&app, |settings| {
        settings.reduce_motion = reduce_motion;
    })
}

#[tauri::command]
pub fn dock_floating_window(
    app: AppHandle,
    edge: String,
) -> Result<settings::DesktopSettings, String> {
    if edge != "left" && edge != "right" {
        return Err(format!("unsupported dock edge: {edge}"));
    }

    window::dock_to_edge(&app, &edge)?;
    Ok(settings::read_settings(&app))
}

#[tauri::command]
pub fn show_main_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window is not registered".to_string())?;

    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hide_main_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window is not registered".to_string())?;

    window.hide().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_core_status(
    manager: State<'_, process::CoreProcessManager>,
) -> process::CoreProcessSnapshot {
    manager.snapshot()
}

#[tauri::command]
pub fn restart_core(
    app: AppHandle,
    manager: State<'_, process::CoreProcessManager>,
) -> Result<process::CoreProcessSnapshot, String> {
    manager.restart(&app)
}

#[tauri::command]
pub fn stop_core(
    app: AppHandle,
    manager: State<'_, process::CoreProcessManager>,
) -> Result<process::CoreProcessSnapshot, String> {
    manager.stop(&app, "user_requested_stop")
}

#[tauri::command]
pub fn open_vault_folder(path: String) -> Result<(), String> {
    let vault_path = PathBuf::from(path);
    if !vault_path.is_dir() {
        return Err("vault folder does not exist".to_string());
    }

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("explorer.exe");
        command.arg(vault_path);
        command
    };

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(vault_path);
        command
    };

    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(vault_path);
        command
    };

    command.spawn().map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_memory_items(
    pool: State<'_, db::DbPool>,
    limit: Option<usize>,
) -> Result<Vec<db::MemoryItem>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    db::get_memory_items(&conn, limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_memory_item(
    pool: State<'_, db::DbPool>,
    content: String,
    source: String,
    tags: Option<Vec<String>>,
) -> Result<db::MemoryItem, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    db::save_memory_item(&conn, &content, &source, tags).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_triage_inbox(
    pool: State<'_, db::DbPool>,
) -> Result<Vec<db::TriageMessageResponse>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    db::get_triage_inbox(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn resolve_triage_item(
    pool: State<'_, db::DbPool>,
    id: String,
    resolution: String,
) -> Result<(), String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    db::resolve_triage_item(&conn, &id, &resolution).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_chat_message(
    pool: State<'_, db::DbPool>,
    session_id: String,
    role: String,
    content: String,
) -> Result<db::ChatMessageItem, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    let sess = if session_id.is_empty() {
        None
    } else {
        Some(session_id.as_str())
    };
    db::save_chat_message(&conn, sess, &role, &content, None).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_chat_history(
    pool: State<'_, db::DbPool>,
    session_id: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<db::ChatMessageItem>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    let sess = session_id.as_deref().filter(|s| !s.is_empty());
    db::get_chat_history(&conn, sess, limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn generate_response(
    app: AppHandle,
    state: State<'_, models::LlmState>,
    prompt: String,
    model: Option<String>,
    system: Option<String>,
    temperature: Option<f32>,
    stream: Option<bool>,
) -> Result<String, String> {
    let req = models::LlmGenerateRequest {
        prompt,
        model,
        system,
        temperature,
        stream,
    };

    let should_stream = stream.unwrap_or(true);
    let app_handle = app.clone();

    state
        .router
        .generate_response(&req, move |chunk| {
            if should_stream {
                let _ = app_handle.emit("llm:stream_chunk", &chunk);
            }
        })
        .await
}

#[tauri::command]
pub async fn list_models(
    state: State<'_, models::LlmState>,
) -> Result<Vec<models::LocalModelInfo>, String> {
    state.router.list_models().await
}

#[tauri::command]
pub async fn get_model_status(
    state: State<'_, models::LlmState>,
) -> Result<models::LocalModelStatusResponse, String> {
    state.router.get_model_status().await
}



