use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

use crate::db::DbPool;

#[derive(Debug, Clone)]
pub enum DaemonEvent {
    ProcessEvents,
    HealthCheck,
    Shutdown,
}

pub struct DaemonManager {
    sender: mpsc::Sender<DaemonEvent>,
    is_running: Arc<AtomicBool>,
}

impl DaemonManager {
    pub fn spawn(db_pool: DbPool) -> Self {
        let (tx, mut rx) = mpsc::channel::<DaemonEvent>(100);
        let is_running = Arc::new(AtomicBool::new(true));
        let is_running_clone = is_running.clone();

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(30));
            println!("[Daemon] Background daemon loop started.");

            while is_running_clone.load(Ordering::Relaxed) {
                tokio::select! {
                    _ = ticker.tick() => {
                        let pool = db_pool.clone();
                        tokio::task::spawn_blocking(move || {
                            Self::perform_health_check(&pool);
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
        }
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
