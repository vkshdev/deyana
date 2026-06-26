use std::{path::PathBuf, process::Command};
use tauri::{AppHandle, Manager, State};

use crate::{process, settings, window};

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
