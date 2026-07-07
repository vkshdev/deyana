use serde::Serialize;
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager};

const CORE_HOST: &str = "127.0.0.1";
const CORE_PORT: u16 = 8765;
const CORE_ENDPOINT: &str = "http://127.0.0.1:8765";
const CORE_WS_ENDPOINT: &str = "ws://127.0.0.1:8765/ws";

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
            lifecycle: "unavailable".to_string(),
            endpoint: CORE_ENDPOINT.to_string(),
            websocket_url: CORE_WS_ENDPOINT.to_string(),
            pid: None,
            started_at_ms: None,
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
    child: Mutex<Option<Child>>,
    snapshot: Mutex<CoreProcessSnapshot>,
}

impl CoreProcessManager {
    pub fn start(&self, app: &AppHandle) -> Result<CoreProcessSnapshot, String> {
        if self.has_running_child() {
            return Ok(self.snapshot());
        }

        if core_is_healthy() {
            self.set_snapshot(app, |snapshot| {
                snapshot.lifecycle = "running".to_string();
                snapshot.pid = None;
                snapshot.started_at_ms.get_or_insert_with(now_ms);
                snapshot.updated_at_ms = now_ms();
                snapshot.last_error = None;
            });
            return Ok(self.snapshot());
        }

        if port_is_open() {
            self.set_snapshot(app, |snapshot| {
                snapshot.lifecycle = "unavailable".to_string();
                snapshot.pid = None;
                snapshot.updated_at_ms = now_ms();
                snapshot.last_error = Some(
                    "port 8765 is occupied by a service that is not a healthy Deyana core"
                        .to_string(),
                );
            });
            return Ok(self.snapshot());
        }

        let service_dir = service_dir(app)?;
        let python = python_executable(&service_dir);
        let app_config_dir = app
            .path()
            .app_config_dir()
            .map_err(|error| error.to_string())?;
        let data_dir = std::env::var("DEYANA_CORE_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| app_config_dir.clone());
        let log_dir = std::env::var("DEYANA_CORE_LOG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| app_config_dir.join("logs"));
        fs::create_dir_all(&log_dir).map_err(|error| error.to_string())?;

        let stdout = File::create(log_dir.join("core.stdout.log")).map_err(|error| error.to_string())?;
        let stderr = File::create(log_dir.join("core.stderr.log")).map_err(|error| error.to_string())?;

        let mut command = Command::new(&python);
        command
            .arg("-m")
            .arg("deyana_core")
            .current_dir(&service_dir)
            .env("DEYANA_CORE_HOST", CORE_HOST)
            .env("DEYANA_CORE_PORT", CORE_PORT.to_string())
            .env("DEYANA_CORE_DATA_DIR", data_dir.to_string_lossy().to_string())
            .env("DEYANA_CORE_LOG_DIR", log_dir.join("core").to_string_lossy().to_string())
            .env("DEYANA_CORE_HEARTBEAT_SECONDS", "5")
            .env("PYTHONPATH", service_dir.join("src").to_string_lossy().to_string())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }

        self.set_snapshot(app, |snapshot| {
            snapshot.lifecycle = "starting".to_string();
            snapshot.pid = None;
            snapshot.started_at_ms = Some(now_ms());
            snapshot.updated_at_ms = now_ms();
            snapshot.last_error = None;
        });

        let child = command.spawn().map_err(|error| {
            let message = format!(
                "unable to start core service with {}: {error}",
                python.display()
            );
            self.set_snapshot(app, |snapshot| {
                snapshot.lifecycle = "unavailable".to_string();
                snapshot.last_error = Some(message.clone());
                snapshot.updated_at_ms = now_ms();
            });
            message
        })?;

        let pid = child.id();
        {
            let mut child_lock = self.inner.child.lock().map_err(|error| error.to_string())?;
            *child_lock = Some(child);
        }

        self.set_snapshot(app, |snapshot| {
            snapshot.lifecycle = "starting".to_string();
            snapshot.pid = Some(pid);
            snapshot.updated_at_ms = now_ms();
        });

        self.spawn_monitor(app.clone());
        Ok(self.snapshot())
    }

    pub fn stop(&self, app: &AppHandle, reason: &str) -> Result<CoreProcessSnapshot, String> {
        let has_child = {
            let child_lock = self.inner.child.lock().map_err(|error| error.to_string())?;
            child_lock.is_some()
        };

        if !has_child {
            if core_is_healthy() {
                self.set_snapshot(app, |snapshot| {
                    snapshot.lifecycle = "running".to_string();
                    snapshot.pid = None;
                    snapshot.updated_at_ms = now_ms();
                    snapshot.last_error = Some(
                        "the connected Deyana core is externally managed and was not stopped"
                            .to_string(),
                    );
                });
                return Ok(self.snapshot());
            }
            self.set_snapshot(app, |snapshot| {
                snapshot.lifecycle = "stopped".to_string();
                snapshot.pid = None;
                snapshot.updated_at_ms = now_ms();
                snapshot.last_error = None;
            });
            return Ok(self.snapshot());
        }

        self.set_snapshot(app, |snapshot| {
            snapshot.lifecycle = "stopping".to_string();
            snapshot.updated_at_ms = now_ms();
            snapshot.last_error = Some(reason.to_string());
        });

        request_clean_shutdown();

        for _ in 0..20 {
            if self.collect_exited_child(app, "stopped")? {
                self.set_snapshot(app, |snapshot| {
                    snapshot.last_error = None;
                });
                return Ok(self.snapshot());
            }
            thread::sleep(Duration::from_millis(100));
        }

        let mut child_lock = self.inner.child.lock().map_err(|error| error.to_string())?;
        if let Some(child) = child_lock.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
        *child_lock = None;
        drop(child_lock);

        self.set_snapshot(app, |snapshot| {
            snapshot.lifecycle = "stopped".to_string();
            snapshot.pid = None;
            snapshot.updated_at_ms = now_ms();
            snapshot.last_error = None;
        });
        Ok(self.snapshot())
    }

    pub fn restart(&self, app: &AppHandle) -> Result<CoreProcessSnapshot, String> {
        {
            let mut snapshot = self.inner.snapshot.lock().map_err(|error| error.to_string())?;
            snapshot.restart_count += 1;
        }
        let _ = self.stop(app, "restart_requested")?;
        self.start(app)
    }

    pub fn snapshot(&self) -> CoreProcessSnapshot {
        self.inner
            .snapshot
            .lock()
            .map(|snapshot| snapshot.clone())
            .unwrap_or_default()
    }

    fn spawn_monitor(&self, app: AppHandle) {
        let manager = self.clone();
        thread::spawn(move || loop {
            thread::sleep(Duration::from_millis(700));

            match manager.collect_exited_child(&app, "crashed") {
                Ok(true) => break,
                Ok(false) => {}
                Err(error) => {
                    manager.set_snapshot(&app, |snapshot| {
                        snapshot.lifecycle = "crashed".to_string();
                        snapshot.last_error = Some(error);
                        snapshot.updated_at_ms = now_ms();
                    });
                    break;
                }
            }

            let lifecycle = manager.snapshot().lifecycle;
            if lifecycle == "starting" && core_is_healthy() {
                manager.set_snapshot(&app, |snapshot| {
                    snapshot.lifecycle = "running".to_string();
                    snapshot.updated_at_ms = now_ms();
                    snapshot.last_error = None;
                });
            }

            if !manager.has_running_child() {
                break;
            }
        });
    }

    fn collect_exited_child(&self, app: &AppHandle, lifecycle: &str) -> Result<bool, String> {
        let mut child_lock = self.inner.child.lock().map_err(|error| error.to_string())?;
        let Some(child) = child_lock.as_mut() else {
            return Ok(true);
        };

        let Some(status) = child.try_wait().map_err(|error| error.to_string())? else {
            return Ok(false);
        };

        *child_lock = None;
        drop(child_lock);

        let healthy_core_available = core_is_healthy();

        self.set_snapshot(app, |snapshot| {
            snapshot.lifecycle = if healthy_core_available {
                "running".to_string()
            } else {
                lifecycle.to_string()
            };
            snapshot.pid = None;
            snapshot.updated_at_ms = now_ms();
            snapshot.last_error = if status.success() || healthy_core_available {
                None
            } else {
                Some(format!("core exited with status {status}"))
            };
        });
        Ok(true)
    }

    fn has_running_child(&self) -> bool {
        self.inner
            .child
            .lock()
            .map(|child| child.is_some())
            .unwrap_or(false)
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

fn python_executable(service_dir: &Path) -> PathBuf {
    let windows_venv = service_dir.join(".venv").join("Scripts").join("python.exe");
    if windows_venv.exists() {
        return windows_venv;
    }

    let unix_venv = service_dir.join(".venv").join("bin").join("python");
    if unix_venv.exists() {
        return unix_venv;
    }

    if cfg!(windows) {
        PathBuf::from("python")
    } else {
        PathBuf::from("python3")
    }
}

fn service_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let tauri_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let desktop_dir = tauri_dir
        .parent()
        .ok_or_else(|| "unable to resolve desktop app directory".to_string())?;
    let apps_dir = desktop_dir
        .parent()
        .ok_or_else(|| "unable to resolve apps directory".to_string())?;
    let repo_dir = apps_dir
        .parent()
        .ok_or_else(|| "unable to resolve repository directory".to_string())?;
    let development_service_dir = repo_dir.join("services").join("core");
    let bundled_service_dir = app
        .path()
        .resource_dir()
        .ok()
        .map(|resource_dir| resource_dir.join("services").join("core"));

    // Tauri exposes bundled resources during development too. Prefer the
    // repository service so the managed virtualenv (including WebSocket
    // support) is used when launching from VS Code or `tauri dev`.
    if cfg!(debug_assertions) && has_core_source(&development_service_dir) {
        return Ok(development_service_dir);
    }

    // Release bundles currently include the Python source but not a full
    // virtualenv. For prototype MSI builds created from this checkout, prefer
    // the repository service when its managed venv exists; otherwise the
    // installed app may launch system Python without FastAPI/Uvicorn installed.
    if has_core_source(&development_service_dir) && has_managed_python(&development_service_dir) {
        return Ok(development_service_dir);
    }

    if let Some(bundled_service_dir) = bundled_service_dir.as_ref() {
        if has_core_source(bundled_service_dir) && has_managed_python(bundled_service_dir) {
            return Ok(bundled_service_dir.clone());
        }
    }

    if let Some(bundled_service_dir) = bundled_service_dir {
        if has_core_source(&bundled_service_dir) {
            return Ok(bundled_service_dir);
        }
    }

    if has_core_source(&development_service_dir) {
        Ok(development_service_dir)
    } else {
        Err(format!(
            "core service directory not found: {}",
            development_service_dir.display()
        ))
    }
}

fn has_core_source(service_dir: &Path) -> bool {
    service_dir.join("src").join("deyana_core").exists()
}

fn has_managed_python(service_dir: &Path) -> bool {
    service_dir
        .join(".venv")
        .join("Scripts")
        .join("python.exe")
        .exists()
        || service_dir.join(".venv").join("bin").join("python").exists()
}

fn port_is_open() -> bool {
    let address = SocketAddr::from(([127, 0, 0, 1], CORE_PORT));
    TcpStream::connect_timeout(&address, Duration::from_millis(200)).is_ok()
}

fn core_is_healthy() -> bool {
    let address = SocketAddr::from(([127, 0, 0, 1], CORE_PORT));
    let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(300)) else {
        return false;
    };
    let timeout = Some(Duration::from_millis(500));
    if stream.set_read_timeout(timeout).is_err() || stream.set_write_timeout(timeout).is_err() {
        return false;
    }

    let request = format!(
        "GET /status HTTP/1.1\r\nHost: {CORE_HOST}:{CORE_PORT}\r\nConnection: close\r\nAccept: application/json\r\n\r\n"
    );
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }

    let mut response = Vec::new();
    match stream.read_to_end(&mut response) {
        Ok(_) => {}
        Err(error)
            if !response.is_empty()
                && matches!(error.kind(), io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock) => {}
        Err(_) => return false,
    }

    let Ok(response) = String::from_utf8(response) else {
        return false;
    };
    let Some((headers, body)) = response.split_once("\r\n\r\n") else {
        return false;
    };
    if !headers.lines().next().is_some_and(|line| line.contains(" 200 ")) {
        return false;
    }

    let Ok(payload) = serde_json::from_str::<serde_json::Value>(body) else {
        return false;
    };
    payload.get("service").and_then(|value| value.as_str()) == Some("deyana-core")
        && payload.get("lifecycle").and_then(|value| value.as_str()) == Some("running")
        && payload
            .get("featureFlags")
            .and_then(|value| value.get("installedDesktopCors"))
            .and_then(|value| value.as_bool())
            == Some(true)
}

fn request_clean_shutdown() {
    let address = SocketAddr::from(([127, 0, 0, 1], CORE_PORT));
    let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(250)) else {
        return;
    };

    let _ = stream.set_write_timeout(Some(Duration::from_millis(250)));
    let request = format!(
        "POST /shutdown HTTP/1.1\r\nHost: {CORE_HOST}:{CORE_PORT}\r\nConnection: close\r\nContent-Length: 0\r\n\r\n"
    );
    let _ = stream.write_all(request.as_bytes());
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
