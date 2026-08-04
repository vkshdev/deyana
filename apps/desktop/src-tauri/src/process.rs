use serde::Serialize;
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreProcessSnapshot {
    pub lifecycle: String,
    pub endpoint: String,
    pub websocket_url: String,
    pub pid: Option<u32>,
    pub started_at_ms: Option<u128>,
    pub updated_at_ms: u128,
    pub restart_count: u32,
    pub last_error: Option<String>,
}

impl Default for CoreProcessSnapshot {
    fn default() -> Self {
        Self {
            lifecycle: "running".to_string(),
            endpoint: "native_rust".to_string(),
            websocket_url: "native_rust".to_string(),
            pid: Some(std::process::id()),
            started_at_ms: Some(now_ms()),
            updated_at_ms: now_ms(),
            restart_count: 0,
            last_error: None,
        }
    }
}

#[derive(Clone, Default)]
pub struct CoreProcessManager {
    inner: Arc<CoreProcessInner>,
}

#[derive(Default)]
struct CoreProcessInner {
    snapshot: Mutex<CoreProcessSnapshot>,
}

impl CoreProcessManager {
    pub fn start(&self, app: &AppHandle) -> Result<CoreProcessSnapshot, String> {
        self.set_snapshot(app, |snapshot| {
            snapshot.lifecycle = "running".to_string();
            snapshot.pid = Some(std::process::id());
            snapshot.started_at_ms.get_or_insert_with(now_ms);
            snapshot.updated_at_ms = now_ms();
            snapshot.last_error = None;
        });
        Ok(self.snapshot())
    }

    pub fn stop(&self, app: &AppHandle, _reason: &str) -> Result<CoreProcessSnapshot, String> {
        self.set_snapshot(app, |snapshot| {
            snapshot.lifecycle = "stopped".to_string();
            snapshot.updated_at_ms = now_ms();
        });
        Ok(self.snapshot())
    }

    pub fn restart(&self, app: &AppHandle) -> Result<CoreProcessSnapshot, String> {
        {
            let mut snapshot = self.inner.snapshot.lock().map_err(|error| error.to_string())?;
            snapshot.restart_count += 1;
        }
        self.start(app)
    }

    pub fn snapshot(&self) -> CoreProcessSnapshot {
        self.inner
            .snapshot
            .lock()
            .map(|snapshot| snapshot.clone())
            .unwrap_or_default()
    }

    fn set_snapshot(&self, app: &AppHandle, update: impl FnOnce(&mut CoreProcessSnapshot)) {
        let snapshot = {
            let mut snapshot = self.inner.snapshot.lock().expect("core snapshot lock poisoned");
            update(&mut snapshot);
            snapshot.clone()
        };

        let _ = write_status_snapshot(app, &snapshot);
        let _ = app.emit("core:status", snapshot);
    }
}

pub fn record_desktop_ready(app: &AppHandle) -> Result<(), String> {
    app.emit("desktop:ready", "desktop_shell")
        .map_err(|error| error.to_string())
}

fn write_status_snapshot(app: &AppHandle, snapshot: &CoreProcessSnapshot) -> Result<(), String> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&config_dir).map_err(|error| error.to_string())?;

    let path = config_dir.join("core-process-status.json");
    let temp_path = path.with_extension("json.tmp");
    let content = serde_json::to_string_pretty(snapshot).map_err(|error| error.to_string())?;

    fs::write(&temp_path, content).map_err(|error| error.to_string())?;
    fs::rename(temp_path, path).map_err(|error| error.to_string())
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}
