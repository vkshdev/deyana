use tauri::{AppHandle, Emitter};

pub fn record_phase1_ready(app: &AppHandle) -> Result<(), String> {
    app.emit("app.ready", "phase1.desktop_shell")
        .map_err(|error| error.to_string())
}

