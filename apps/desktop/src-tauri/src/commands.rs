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
    vault_key: State<'_, crate::privacy::VaultKeyState>,
    limit: Option<usize>,
) -> Result<Vec<db::MemoryItem>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    db::get_memory_items_decrypted(&conn, limit, vault_key.key()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_memory_item(
    pool: State<'_, db::DbPool>,
    vault_key: State<'_, crate::privacy::VaultKeyState>,
    content: String,
    source: String,
    tags: Option<Vec<String>>,
) -> Result<db::MemoryItem, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    db::save_memory_item_encrypted(&conn, &content, &source, tags, vault_key.key()).map_err(|e| e.to_string())
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
    vault_key: State<'_, crate::privacy::VaultKeyState>,
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
    db::save_chat_message_encrypted(&conn, sess, &role, &content, None, vault_key.key()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_chat_history(
    pool: State<'_, db::DbPool>,
    vault_key: State<'_, crate::privacy::VaultKeyState>,
    session_id: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<db::ChatMessageItem>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    let sess = session_id.as_deref().filter(|s| !s.is_empty());
    db::get_chat_history_decrypted(&conn, sess, limit, vault_key.key()).map_err(|e| e.to_string())
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
        grammar: None,
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

#[tauri::command]
pub fn get_active_profile(
    state: State<'_, models::LlmState>,
) -> models::ActiveProfileInfo {
    state.router.get_active_profile_info()
}

#[tauri::command]
pub fn set_active_profile(
    state: State<'_, models::LlmState>,
    profile: models::ModelProfile,
) -> models::ActiveProfileInfo {
    state.router.set_active_profile(profile);
    state.router.get_active_profile_info()
}

#[tauri::command]
pub async fn start_llamacpp_server(
    state: State<'_, models::LlmState>,
    registry_state: State<'_, models::GgufRegistryState>,
    model_path: String,
    n_ctx: Option<u32>,
    port: Option<u16>,
    chat_template: Option<String>,
) -> Result<models::LlamaCppServerStatus, String> {
    // Resolve chat template: use the explicit override, or look it up from the registry
    // based on the model_path matching a registered entry.
    let resolved_template = chat_template.or_else(|| {
        registry_state
            .inner
            .read()
            .ok()
            .and_then(|reg| {
                reg.list()
                    .into_iter()
                    .find(|e| e.file_path == model_path)
                    .and_then(|e| e.chat_template)
            })
    });

    state
        .router
        .llamacpp_engine()
        .start_server(&model_path, n_ctx, port, resolved_template.as_deref())
        .await
}

#[tauri::command]
pub async fn stop_llamacpp_server(
    state: State<'_, models::LlmState>,
) -> Result<(), String> {
    state.router.llamacpp_engine().stop_server().await
}

#[tauri::command]
pub fn get_llamacpp_status(
    state: State<'_, models::LlmState>,
) -> models::LlamaCppServerStatus {
    state.router.llamacpp_engine().get_status()
}

#[tauri::command]
pub async fn list_hf_gguf_files(
    repo_id: String,
) -> Result<Vec<models::HfGgufFile>, String> {
    models::HfDownloader::list_repo_ggufs(&repo_id).await
}

#[tauri::command]
pub async fn download_hf_gguf(
    app: AppHandle,
    registry_state: State<'_, models::GgufRegistryState>,
    repo_id: String,
    filename: String,
) -> Result<models::GgufRegistryEntry, String> {
    let models_dir = models::HardwareProfileDetector::get_default_models_dir();
    let entry = models::HfDownloader::download_gguf(
        &repo_id,
        &filename,
        &models_dir,
        Some(&app),
    )
    .await?;

    let mut reg = registry_state
        .inner
        .write()
        .map_err(|e| format!("Registry lock poisoned: {e}"))?;
    reg.register(entry)
}

#[tauri::command]
pub fn list_registered_models(
    registry_state: State<'_, models::GgufRegistryState>,
) -> Result<Vec<models::GgufRegistryEntry>, String> {
    let reg = registry_state
        .inner
        .read()
        .map_err(|e| format!("Registry lock poisoned: {e}"))?;
    Ok(reg.list())
}

#[tauri::command]
pub fn register_local_gguf(
    registry_state: State<'_, models::GgufRegistryState>,
    file_path: String,
    display_name: Option<String>,
    n_ctx: Option<u32>,
) -> Result<models::GgufRegistryEntry, String> {
    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(format!("File does not exist: {file_path}"));
    }

    let default_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "custom-model.gguf".to_string());
    let name = display_name.unwrap_or(default_name);
    let size_bytes = path.metadata().map(|m| m.len()).unwrap_or(0);
    let id = models::slugify(&name);

    let detected = models::HardwareProfileDetector::scan_models_dir(
        path.parent().unwrap_or_else(|| std::path::Path::new(".")),
    );
    let profile_tier = detected
        .into_iter()
        .find(|m| m.path == file_path)
        .map(|m| m.detected_profile)
        .unwrap_or(models::ModelProfile::Balanced);

    let entry = models::GgufRegistryEntry {
        id,
        display_name: name,
        file_path,
        size_bytes,
        profile_tier,
        n_ctx: n_ctx.unwrap_or(4096),
        chat_template: None,
        source: "local_import".to_string(),
        registered_at: chrono::Utc::now().to_rfc3339(),
    };

    let mut reg = registry_state
        .inner
        .write()
        .map_err(|e| format!("Registry lock poisoned: {e}"))?;
    reg.register(entry)
}

#[tauri::command]
pub fn delete_registered_model(
    registry_state: State<'_, models::GgufRegistryState>,
    model_id: String,
    delete_file: Option<bool>,
) -> Result<(), String> {
    let mut reg = registry_state
        .inner
        .write()
        .map_err(|e| format!("Registry lock poisoned: {e}"))?;
    reg.unregister(&model_id, delete_file.unwrap_or(false))
}

#[tauri::command]
pub fn submit_response_feedback(
    pool: State<'_, db::DbPool>,
    id: String,
    feedback: i32,
    user_edit: Option<String>,
) -> Result<(), String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    models::dataset::record_feedback(&conn, &id, feedback, user_edit.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_dataset_stats(
    pool: State<'_, db::DbPool>,
) -> Result<models::DatasetStats, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    models::dataset::get_dataset_stats(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_training_dataset(
    pool: State<'_, db::DbPool>,
    format: String,
    output_path: Option<String>,
) -> Result<String, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    let path = if let Some(p) = output_path {
        std::path::PathBuf::from(p)
    } else {
        let base = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("."));
        let ext = if format.to_lowercase() == "dpo" {
            "deyana_dpo_dataset.jsonl"
        } else {
            "deyana_sharegpt_dataset.jsonl"
        };
        base.join(".deyana").join(ext)
    };

    let count = match format.to_lowercase().as_str() {
        "dpo" => models::dataset::export_dpo(&conn, &path)?,
        _ => models::dataset::export_sharegpt(&conn, &path)?,
    };

    Ok(format!(
        "Successfully exported {count} samples to {}",
        path.to_string_lossy()
    ))
}

#[tauri::command]
pub fn clear_training_dataset(
    pool: State<'_, db::DbPool>,
) -> Result<usize, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    models::dataset::clear_training_dataset(&conn).map_err(|e| e.to_string())
}







