use super::*;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn create_test_pool() -> DbPool {
    let manager = SqliteConnectionManager::memory();
    Pool::builder().max_size(1).build(manager).unwrap()
}

#[test]
fn test_voice_transcript_payload() {
    let payload = VoiceTranscriptPayload {
        transcript: "Hello Deyana".to_string(),
        engine: "windows_speech".to_string(),
        language: "en-US".to_string(),
        duration_seconds: 1.5,
        created_at: Utc::now().to_rfc3339(),
    };

    assert_eq!(payload.transcript, "Hello Deyana");
    assert_eq!(payload.engine, "windows_speech");
    assert_eq!(payload.duration_seconds, 1.5);
}

#[test]
fn test_interrupt_speech_when_no_process() {
    let temp_dir = std::env::temp_dir().join(format!("deyana_test_{}", uuid::Uuid::new_v4()));
    let service = LocalVoiceService::new(temp_dir.clone(), create_test_pool());

    let res = service.interrupt_speech();
    assert!(!res.interrupted);
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn test_interrupt_speech_kills_running_process() {
    let temp_dir = std::env::temp_dir().join(format!("deyana_test_{}", uuid::Uuid::new_v4()));
    let service = LocalVoiceService::new(temp_dir.clone(), create_test_pool());

    let child = if cfg!(target_os = "windows") {
        Command::new("powershell.exe")
            .args(["-NoProfile", "-Command", "Start-Sleep -Seconds 10"])
            .spawn()
            .unwrap()
    } else {
        Command::new("sleep").arg("10").spawn().unwrap()
    };

    {
        let mut lock = service.active_tts_process.lock().unwrap();
        *lock = Some(child);
    }

    let res = service.interrupt_speech();
    assert!(res.interrupted);

    let lock = service.active_tts_process.lock().unwrap();
    assert!(lock.is_none());

    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn test_speak_loop_unlocks_during_wait_and_allows_interruption() {
    let temp_dir = std::env::temp_dir().join(format!("deyana_test_{}", uuid::Uuid::new_v4()));
    let service = Arc::new(LocalVoiceService::new(temp_dir.clone(), create_create_pool_dummy(temp_dir.as_path())));

    fn create_create_pool_dummy(_path: &std::path::Path) -> DbPool {
        create_test_pool()
    }

    let child = if cfg!(target_os = "windows") {
        Command::new("powershell.exe")
            .args(["-NoProfile", "-Command", "Start-Sleep -Seconds 10"])
            .spawn()
            .unwrap()
    } else {
        Command::new("sleep").arg("10").spawn().unwrap()
    };

    {
        let mut lock = service.active_tts_process.lock().unwrap();
        *lock = Some(child);
    }

    let service_clone = service.clone();
    let handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(100));
        service_clone.interrupt_speech()
    });

    let wait_result = loop {
        let status_opt = {
            let mut lock = service.active_tts_process.lock();
            if let Ok(ref mut guard) = lock {
                if let Some(ref mut c) = **guard {
                    match c.try_wait() {
                        Ok(Some(status)) => Ok(Some(status)),
                        Ok(None) => Ok(None),
                        Err(e) => Err(format!("Text-to-speech execution error: {}", e)),
                    }
                } else {
                    Err("Speech was cancelled.".to_string())
                }
            } else {
                Err("Speech synchronization error.".to_string())
            }
        };

        match status_opt {
            Ok(Some(status)) => {
                if let Ok(mut guard) = service.active_tts_process.lock() {
                    *guard = None;
                }
                break Ok(status);
            }
            Ok(None) => {
                thread::sleep(Duration::from_millis(20));
            }
            Err(err) => {
                if let Ok(mut guard) = service.active_tts_process.lock() {
                    *guard = None;
                }
                break Err(err);
            }
        }
    };

    let interrupt_res = handle.join().unwrap();
    assert!(interrupt_res.interrupted);
    assert_eq!(wait_result.unwrap_err(), "Speech was cancelled.");

    let _ = std::fs::remove_dir_all(temp_dir);
}
