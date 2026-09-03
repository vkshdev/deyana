use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use deyana_desktop_lib::agent::react::{AgentStepPayload, ConnectionProvider, ReActEngine, MAX_REACT_STEPS};
use deyana_desktop_lib::db::schema::INIT_SQL;
use deyana_desktop_lib::models::LlmGenerateRequest;
use rusqlite::Connection;

fn setup_test_db() -> Arc<std::sync::Mutex<Connection>> {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(INIT_SQL).unwrap();
    Arc::new(std::sync::Mutex::new(conn))
}

fn assert_send<T: Send>(_t: &T) {}

/// 1. EMPIRICAL IPC SERIALIZATION CONTRACT TEST
/// Verifies that AgentStepPayload serializes exactly to the frontend camelCase contract
/// specified in PROJECT.md: step, thought, action, actionInput, observation, status, durationMs.
#[test]
fn test_agent_step_payload_camelcase_serialization_contract() {
    let payload = AgentStepPayload {
        step: 1,
        thought: "I need to check system resources".to_string(),
        action: "get_system_status".to_string(),
        action_input: serde_json::json!({"filter": "memory"}),
        observation: "RAM: 16GB".to_string(),
        status: "completed".to_string(),
        duration_ms: Some(125),
    };

    let json_str = serde_json::to_string(&payload).expect("Failed to serialize AgentStepPayload");
    let v: serde_json::Value = serde_json::from_str(&json_str).expect("Failed to parse serialized JSON");

    // Must be a JSON object
    assert!(v.is_object(), "Serialized payload must be a JSON object");
    let obj = v.as_object().unwrap();

    // Verify exact camelCase field names exist
    assert!(obj.contains_key("step"), "Missing 'step'");
    assert!(obj.contains_key("thought"), "Missing 'thought'");
    assert!(obj.contains_key("action"), "Missing 'action'");
    assert!(obj.contains_key("actionInput"), "Missing 'actionInput'");
    assert!(obj.contains_key("observation"), "Missing 'observation'");
    assert!(obj.contains_key("status"), "Missing 'status'");
    assert!(obj.contains_key("durationMs"), "Missing 'durationMs'");

    // Verify snake_case keys are NOT present
    assert!(!obj.contains_key("action_input"), "Found snake_case 'action_input'");
    assert!(!obj.contains_key("duration_ms"), "Found snake_case 'duration_ms'");

    // Verify types and values
    assert_eq!(obj.get("step").unwrap().as_u64(), Some(1));
    assert_eq!(obj.get("thought").unwrap().as_str(), Some("I need to check system resources"));
    assert_eq!(obj.get("action").unwrap().as_str(), Some("get_system_status"));
    assert_eq!(obj.get("actionInput").unwrap().get("filter").unwrap().as_str(), Some("memory"));
    assert_eq!(obj.get("observation").unwrap().as_str(), Some("RAM: 16GB"));
    assert_eq!(obj.get("status").unwrap().as_str(), Some("completed"));
    assert_eq!(obj.get("durationMs").unwrap().as_u64(), Some(125));

    // Test with duration_ms: None
    let running_payload = AgentStepPayload {
        step: 2,
        thought: "Running next step".to_string(),
        action: "search_memory".to_string(),
        action_input: serde_json::json!({}),
        observation: String::new(),
        status: "running".to_string(),
        duration_ms: None,
    };
    let running_json = serde_json::to_string(&running_payload).unwrap();
    let running_v: serde_json::Value = serde_json::from_str(&running_json).unwrap();
    let running_obj = running_v.as_object().unwrap();

    assert!(running_obj.contains_key("durationMs"));
    assert!(running_obj.get("durationMs").unwrap().is_null());
    assert_eq!(running_obj.get("status").unwrap().as_str(), Some("running"));

    // Verify deserialization from camelCase frontend JSON works cleanly
    let deserialized: AgentStepPayload = serde_json::from_value(running_v).unwrap();
    assert_eq!(deserialized, running_payload);
}

/// 2. EMPIRICAL ASYNC SEND BOUNDS TEST
/// Verifies that ReActEngine::execute_with_llm produces a Future that implements Send
/// and can be spawned on a multi-threaded Tokio runtime.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_react_engine_future_is_send() {
    let conn = setup_test_db();
    let engine = Arc::new(ReActEngine::new());

    let handle = tokio::spawn({
        let conn = conn.clone();
        let engine = engine.clone();
        async move {
            let fut = engine.execute_with_llm(
                &conn,
                "System prompt",
                "Hello",
                None,
                |_req| async {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    Ok(r#"{"thought": "done", "final_answer": "Success"}"#.to_string())
                },
                |_step| {},
            );

            // Assert compile-time Send bound on the future itself
            assert_send(&fut);

            fut.await
        }
    });

    let res = handle.await.expect("Task join failed").expect("Engine execution failed");
    assert_eq!(res.final_answer, "Success");
}

/// 3. EMPIRICAL CONCURRENCY SAFETY TEST: NO SQLITE LOCK HELD ACROSS AWAIT
/// Demonstrates empirically that while the LLM generator is suspended at an `.await` point,
/// the database connection is NOT locked by ConnectionProvider.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_no_sqlite_connection_lock_held_across_await() {
    let conn = setup_test_db();
    let engine = Arc::new(ReActEngine::new());
    let step_count = Arc::new(AtomicUsize::new(0));

    // A barrier or flag to signal when the LLM is inside .await
    let inside_llm_await = Arc::new(tokio::sync::Notify::new());
    let allow_llm_finish = Arc::new(tokio::sync::Notify::new());

    let step_count_clone = step_count.clone();
    let inside_notify = inside_llm_await.clone();
    let allow_notify = allow_llm_finish.clone();

    // Spawn the ReAct engine run in task 1
    let task_conn = conn.clone();
    let task_engine = engine.clone();
    let engine_task = tokio::spawn(async move {
        task_engine
            .execute_with_llm(
                &task_conn,
                "System prompt",
                "Perform action",
                None,
                move |_req| {
                    let inside = inside_notify.clone();
                    let allow = allow_notify.clone();
                    let sc = step_count_clone.clone();
                    async move {
                        let c = sc.fetch_add(1, Ordering::SeqCst);
                        if c == 0 {
                            // Turn 1: request get_system_status tool
                            Ok(r#"{"thought": "check", "tool": "get_system_status", "arguments": {}}"#.to_string())
                        } else {
                            // Turn 2: LLM pauses inside await!
                            inside.notify_one();
                            allow.notified().await;
                            Ok(r#"{"thought": "done", "final_answer": "Complete"}"#.to_string())
                        }
                    }
                },
                |_step| {},
            )
            .await
    });

    // Wait until task 1 is suspended inside LLM await
    inside_llm_await.notified().await;

    // EMPIRICAL PROOF: While the engine is suspended awaiting LLM response,
    // we MUST be able to immediately acquire the SQLite lock without blocking!
    let lock_attempt = conn.try_lock();
    assert!(
        lock_attempt.is_ok(),
        "FAIL: SQLite mutex lock was held across an .await point!"
    );

    // Can even perform DB operations while engine is awaiting LLM
    {
        let direct_conn = lock_attempt.unwrap();
        let val: i32 = direct_conn.query_row("SELECT 42", [], |r| r.get(0)).unwrap();
        assert_eq!(val, 42);
    } // Lock released

    // Now signal the LLM generator to complete
    allow_llm_finish.notify_one();

    let output = engine_task.await.unwrap().unwrap();
    assert_eq!(output.final_answer, "Complete");
}

/// 4. EMPIRICAL MULTI-THREADED STRESS TEST
/// Spawns 10 concurrent ReAct engine tasks sharing the same Arc<Mutex<Connection>>
/// executing multi-step tool calls to stress concurrency and ensure no deadlocks or race conditions.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_react_stress_execution() {
    let conn = setup_test_db();
    let engine = Arc::new(ReActEngine::new());
    let num_tasks = 10;
    let mut handles = Vec::new();

    for i in 0..num_tasks {
        let task_conn = conn.clone();
        let task_engine = engine.clone();

        let handle = tokio::spawn(async move {
            let turn_tracker = Arc::new(AtomicUsize::new(0));
            let tracker_clone = turn_tracker.clone();

            task_engine
                .execute_with_llm(
                    &task_conn,
                    "System prompt",
                    &format!("User query {i}"),
                    None,
                    move |_req| {
                        let turn = tracker_clone.fetch_add(1, Ordering::SeqCst);
                        async move {
                            tokio::time::sleep(Duration::from_millis(5)).await;
                            if turn == 0 {
                                Ok(r#"{"thought": "checking status", "tool": "get_system_status", "arguments": {}}"#.to_string())
                            } else {
                                Ok(format!(r#"{{"thought": "done", "final_answer": "Task {i} done"}}"#))
                            }
                        }
                    },
                    |_step| {},
                )
                .await
        });

        handles.push(handle);
    }

    // Await all concurrent tasks with a timeout to detect any deadlock
    let timeout_duration = Duration::from_secs(15);
    let results = tokio::time::timeout(timeout_duration, async {
        let mut completed = Vec::new();
        for h in handles {
            let res = h.await.expect("Task panicked").expect("Engine execution failed");
            completed.push(res);
        }
        completed
    })
    .await
    .expect("DEADLOCK DETECTED: Concurrent ReAct tasks did not complete within timeout!");

    assert_eq!(results.len(), num_tasks);
    for (i, res) in results.iter().enumerate() {
        assert_eq!(res.final_answer, format!("Task {i} done"));
        assert_eq!(res.step_count, 2);
    }
}
