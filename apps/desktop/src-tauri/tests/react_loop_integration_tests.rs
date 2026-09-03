use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use deyana_desktop_lib::agent::dispatcher::ToolDispatcher;
use deyana_desktop_lib::agent::react::{AgentStepPayload, ReActEngine, MAX_REACT_STEPS};
use deyana_desktop_lib::db::schema::INIT_SQL;
use deyana_desktop_lib::models::LlmGenerateRequest;
use rusqlite::Connection;

/// Helper to set up an in-memory SQLite database initialized with Deyana schema.
fn setup_test_db() -> Arc<std::sync::Mutex<Connection>> {
    let conn = Connection::open_in_memory().expect("Failed to create in-memory SQLite db");
    conn.execute_batch(INIT_SQL)
        .expect("Failed to initialize database schema");
    Arc::new(std::sync::Mutex::new(conn))
}

// ============================================================================
// 1. MULTI-TURN TOOL CALLING TEST
// Flow: Thought -> Action -> Observation -> Thought -> Action -> Observation -> Final Answer
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_multiturn_tool_calling_flow() {
    let conn = setup_test_db();
    let engine = ReActEngine::new();

    let turn_counter = Arc::new(AtomicUsize::new(0));
    let turn_counter_clone = turn_counter.clone();

    let streamed_steps = Arc::new(std::sync::Mutex::new(Vec::<AgentStepPayload>::new()));
    let streamed_steps_clone = streamed_steps.clone();

    // Simulated LLM generator representing a multi-turn conversation
    let generate_fn = move |req: LlmGenerateRequest| {
        let current_turn = turn_counter_clone.fetch_add(1, Ordering::SeqCst);
        let prompt_text = req.prompt.clone();
        async move {
            match current_turn {
                0 => {
                    // Turn 1: LLM decides to search memory for "Project Titan"
                    assert!(
                        prompt_text.contains("Check notes for Project Titan"),
                        "Expected initial user prompt in turn 1"
                    );
                    Ok(r#"{
                        "thought": "I should first search memory to check for existing notes on Project Titan.",
                        "tool": "search_memory",
                        "arguments": {"query": "Titan", "limit": 3}
                    }"#.to_string())
                }
                1 => {
                    // Turn 2: LLM inspects observation in prompt (empty search) and decides to save a note
                    assert!(
                        prompt_text.contains("PREVIOUS ACTIONS AND OBSERVATIONS"),
                        "Scratchpad must be present in turn 2 prompt"
                    );
                    assert!(
                        prompt_text.contains("search_memory"),
                        "Scratchpad must show search_memory action"
                    );
                    Ok(r#"{
                        "thought": "No notes found for Titan. I will save a new note with the project kickoff details.",
                        "tool": "save_note",
                        "arguments": {
                            "content": "Project Titan kickoff: initial architecture verified and milestones scheduled.",
                            "tags": ["titan", "work", "planning"]
                        }
                    }"#.to_string())
                }
                2 => {
                    // Turn 3: LLM sees the successful save note confirmation and formulates final answer
                    assert!(
                        prompt_text.contains("Successfully saved note"),
                        "Scratchpad must show save_note observation"
                    );
                    Ok(r#"{
                        "thought": "The note has been successfully saved. I can now provide the final answer to the user.",
                        "final_answer": "I searched memory for Project Titan, found no existing entries, and saved a new kickoff note with tags: titan, work, planning."
                    }"#.to_string())
                }
                _ => panic!("Unexpected extra turn {current_turn}"),
            }
        }
    };

    let step_sink = move |step: &AgentStepPayload| {
        streamed_steps_clone.lock().unwrap().push(step.clone());
    };

    let output = engine
        .execute_with_llm(
            &conn,
            "You are a helpful desktop assistant.",
            "Check notes for Project Titan and record kickoff notes.",
            None,
            generate_fn,
            step_sink,
        )
        .await
        .expect("ReAct engine execution must succeed");

    // Verify engine final output
    assert!(output.completed, "Output must be marked completed");
    assert_eq!(output.step_count, 3, "Must execute exactly 3 steps");
    assert!(
        output.final_answer.contains("Project Titan"),
        "Final answer must reference Project Titan"
    );
    assert_eq!(output.steps.len(), 3, "Output steps history must record 3 steps");

    // Verify Step 1 in output history
    assert_eq!(output.steps[0].step, 1);
    assert_eq!(output.steps[0].action, "search_memory");
    assert_eq!(output.steps[0].status, "completed");
    assert!(output.steps[0].duration_ms.is_some());

    // Verify Step 2 in output history
    assert_eq!(output.steps[1].step, 2);
    assert_eq!(output.steps[1].action, "save_note");
    assert_eq!(output.steps[1].status, "completed");
    assert!(output.steps[1].observation.contains("Successfully saved note"));

    // Verify Step 3 in output history
    assert_eq!(output.steps[2].step, 3);
    assert_eq!(output.steps[2].action, "final_answer");
    assert_eq!(output.steps[2].status, "completed");

    // Verify streamed events:
    // Step 1: running, then completed
    // Step 2: running, then completed
    // Step 3: completed (final answer)
    let events = streamed_steps.lock().unwrap().clone();
    assert_eq!(events.len(), 5, "Must stream 5 total step events (2 running + 3 completed)");
    assert_eq!(events[0].status, "running");
    assert_eq!(events[0].action, "search_memory");
    assert!(events[0].duration_ms.is_none());

    assert_eq!(events[1].status, "completed");
    assert_eq!(events[1].action, "search_memory");
    assert!(events[1].duration_ms.is_some());

    assert_eq!(events[2].status, "running");
    assert_eq!(events[2].action, "save_note");
    assert!(events[2].duration_ms.is_none());

    assert_eq!(events[3].status, "completed");
    assert_eq!(events[3].action, "save_note");
    assert!(events[3].duration_ms.is_some());

    assert_eq!(events[4].status, "completed");
    assert_eq!(events[4].action, "final_answer");

    // Genuine verification: Verify SQLite DB state directly!
    let direct_conn = conn.lock().unwrap();
    let note_content: String = direct_conn
        .query_row(
            "SELECT content_markdown FROM memory_items WHERE content_markdown LIKE '%Project Titan kickoff%'",
            [],
            |row| row.get(0),
        )
        .expect("Database must genuinely contain the saved note");
    assert!(note_content.contains("initial architecture verified"));
}

// ============================================================================
// 2. TOOL EXECUTION VIA ToolDispatcher (VALID AND INVALID INPUTS)
// ============================================================================
#[test]
fn test_tool_dispatcher_direct_valid_and_invalid_inputs() {
    let conn = setup_test_db();
    let locked_conn = conn.lock().unwrap();

    // 2.1 search_memory - valid query
    let res = ToolDispatcher::dispatch(
        &locked_conn,
        "search_memory",
        &serde_json::json!({"query": "anything", "limit": 5}),
    );
    assert!(res.is_ok(), "search_memory must succeed with valid query");
    let val: serde_json::Value = serde_json::from_str(&res.unwrap()).unwrap();
    assert!(val.is_array(), "search_memory output must be a JSON array");

    // 2.2 search_memory - missing query (graceful fallback)
    let res = ToolDispatcher::dispatch(&locked_conn, "search_memory", &serde_json::json!({}));
    assert!(res.is_ok(), "search_memory must default query when missing");

    // 2.3 save_note - valid parameters
    let res = ToolDispatcher::dispatch(
        &locked_conn,
        "save_note",
        &serde_json::json!({
            "content": "Dispatcher direct test note content",
            "tags": ["unit_test", "dispatcher"]
        }),
    );
    assert!(res.is_ok(), "save_note must succeed with valid content and tags");
    let msg = res.unwrap();
    assert!(msg.contains("Successfully saved note"));

    // 2.4 save_note - invalid: missing required "content" parameter
    let res = ToolDispatcher::dispatch(
        &locked_conn,
        "save_note",
        &serde_json::json!({"tags": ["no_content"]}),
    );
    assert!(res.is_err(), "save_note must return Err when 'content' is missing");
    assert!(
        res.unwrap_err().contains("Missing required parameter 'content'"),
        "Error message must specify missing content parameter"
    );

    // 2.5 get_system_status - valid execution
    let res = ToolDispatcher::dispatch(&locked_conn, "get_system_status", &serde_json::json!({}));
    assert!(res.is_ok(), "get_system_status must succeed");
    let sys_status: serde_json::Value = serde_json::from_str(&res.unwrap()).unwrap();
    assert!(sys_status.get("total_ram_gb").is_some());
    assert!(sys_status.get("active_profile").is_some());
    assert!(sys_status.get("recommended_chat_model").is_some());

    // 2.6 get_triage_inbox - valid execution
    let res = ToolDispatcher::dispatch(&locked_conn, "get_triage_inbox", &serde_json::json!({}));
    assert!(res.is_ok(), "get_triage_inbox must succeed");
    let inbox: serde_json::Value = serde_json::from_str(&res.unwrap()).unwrap();
    assert!(inbox.is_array());

    // 2.7 Unknown / unsupported tool invocation
    let res = ToolDispatcher::dispatch(&locked_conn, "unknown_calculator_tool", &serde_json::json!({}));
    assert!(res.is_err(), "Unknown tool must return Err");
    assert!(
        res.unwrap_err().contains("Unknown or unsupported tool"),
        "Error message must indicate unknown tool"
    );
}

// ============================================================================
// 3. TOOL FAILURE AND AGENT RECOVERY LOOP
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_tool_failure_and_agent_recovery_loop() {
    let conn = setup_test_db();
    let engine = ReActEngine::new();

    let turn_counter = Arc::new(AtomicUsize::new(0));
    let turn_counter_clone = turn_counter.clone();

    let streamed_steps = Arc::new(std::sync::Mutex::new(Vec::<AgentStepPayload>::new()));
    let streamed_steps_clone = streamed_steps.clone();

    let generate_fn = move |req: LlmGenerateRequest| {
        let turn = turn_counter_clone.fetch_add(1, Ordering::SeqCst);
        let prompt_text = req.prompt.clone();
        async move {
            match turn {
                0 => {
                    // Turn 1: LLM attempts to call save_note with missing required "content"
                    Ok(r#"{
                        "thought": "Saving note without required content field to trigger validation error.",
                        "tool": "save_note",
                        "arguments": {"invalid_arg": 123}
                    }"#.to_string())
                }
                1 => {
                    // Turn 2: LLM inspects scratchpad, notices the failure message, and recovers
                    assert!(
                        prompt_text.contains("Error executing tool 'save_note'"),
                        "Scratchpad must contain the tool failure message"
                    );
                    assert!(
                        prompt_text.contains("Missing required parameter 'content'"),
                        "Scratchpad must contain specific missing parameter error"
                    );
                    Ok(r#"{
                        "thought": "I see the parameter 'content' was missing. Correcting now.",
                        "tool": "save_note",
                        "arguments": {"content": "Recovered note after validation error", "tags": ["recovered"]}
                    }"#.to_string())
                }
                2 => {
                    // Turn 3: Conclude
                    Ok(r#"{
                        "thought": "The note has now been saved successfully.",
                        "final_answer": "Encountered a missing parameter error on first attempt, then recovered and saved the note successfully."
                    }"#.to_string())
                }
                _ => panic!("Unexpected turn {turn}"),
            }
        }
    };

    let step_sink = move |step: &AgentStepPayload| {
        streamed_steps_clone.lock().unwrap().push(step.clone());
    };

    let output = engine
        .execute_with_llm(
            &conn,
            "System prompt",
            "Save a recovered note",
            None,
            generate_fn,
            step_sink,
        )
        .await
        .expect("Execution must succeed despite intermediate tool failure");

    assert!(output.completed);
    assert_eq!(output.step_count, 3);
    assert_eq!(output.steps[0].status, "failed");
    assert!(output.steps[0].observation.contains("Missing required parameter 'content'"));
    assert_eq!(output.steps[1].status, "completed");
    assert_eq!(output.steps[2].status, "completed");

    // Verify genuine DB insertion for the recovered turn
    let direct_conn = conn.lock().unwrap();
    let count: i64 = direct_conn
        .query_row(
            "SELECT COUNT(*) FROM memory_items WHERE content_markdown = 'Recovered note after validation error'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "Recovered note must exist in database");
}

// ============================================================================
// 4. STEP SAFETY LIMIT: STRICT TERMINATION AT MAX_REACT_STEPS = 5
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_step_safety_limit_strictly_terminates_at_max_steps() {
    let conn = setup_test_db();
    let engine = ReActEngine::new();

    let calls_count = Arc::new(AtomicUsize::new(0));
    let calls_count_clone = calls_count.clone();

    let streamed_steps = Arc::new(std::sync::Mutex::new(Vec::<AgentStepPayload>::new()));
    let streamed_steps_clone = streamed_steps.clone();

    let generate_fn = move |_req: LlmGenerateRequest| {
        let count = calls_count_clone.fetch_add(1, Ordering::SeqCst);
        async move {
            Ok(format!(
                r#"{{"thought": "Keep looping turn {count}", "tool": "get_system_status", "arguments": {{}}}}"#
            ))
        }
    };

    let step_sink = move |step: &AgentStepPayload| {
        streamed_steps_clone.lock().unwrap().push(step.clone());
    };

    let output = engine
        .execute_with_llm(
            &conn,
            "System prompt",
            "This query triggers an infinite tool loop",
            None,
            generate_fn,
            step_sink,
        )
        .await
        .expect("Execution must complete gracefully at safety limit");

    assert_eq!(
        calls_count.load(Ordering::SeqCst),
        MAX_REACT_STEPS,
        "LLM generator must be invoked exactly MAX_REACT_STEPS times"
    );

    assert!(output.completed);
    assert!(
        output.final_answer.contains(&format!("Reached maximum safety limit of {} steps", MAX_REACT_STEPS)),
        "Final answer must synthesize safety limit termination message"
    );

    let last_step = output.steps.last().expect("Must have steps");
    assert_eq!(last_step.step, MAX_REACT_STEPS);
    assert_eq!(last_step.action, "final_answer");
    assert_eq!(last_step.status, "completed");
    assert!(last_step.thought.contains("Reached maximum limit"));

    let events = streamed_steps.lock().unwrap().clone();
    assert_eq!(events.len(), 11);
    let final_event = events.last().unwrap();
    assert_eq!(final_event.status, "completed");
    assert_eq!(final_event.action, "final_answer");
}

// ============================================================================
// 5. STEP EVENT PAYLOAD STREAMING CONTRACT (AgentStepPayload)
// ============================================================================
#[test]
fn test_step_event_payload_streaming_contract() {
    let payload = AgentStepPayload {
        step: 2,
        thought: "Analyzing database records".to_string(),
        action: "search_memory".to_string(),
        action_input: serde_json::json!({"query": "inbox", "limit": 10}),
        observation: r#"[{"id":1,"title":"Sample"}]"#.to_string(),
        status: "completed".to_string(),
        duration_ms: Some(42),
    };

    let serialized = serde_json::to_string(&payload).expect("Serialization failed");
    let val: serde_json::Value = serde_json::from_str(&serialized).expect("Deserialization failed");

    assert!(val.get("step").is_some());
    assert!(val.get("thought").is_some());
    assert!(val.get("action").is_some());
    assert!(val.get("actionInput").is_some(), "Must serialize as actionInput (camelCase)");
    assert!(val.get("observation").is_some());
    assert!(val.get("status").is_some());
    assert!(val.get("durationMs").is_some(), "Must serialize as durationMs (camelCase)");

    assert!(val.get("action_input").is_none());
    assert!(val.get("duration_ms").is_none());

    assert_eq!(val["step"].as_u64(), Some(2));
    assert_eq!(val["action"].as_str(), Some("search_memory"));
    assert_eq!(val["status"].as_str(), Some("completed"));
    assert_eq!(val["durationMs"].as_u64(), Some(42));
    assert_eq!(val["actionInput"]["query"].as_str(), Some("inbox"));

    let roundtripped: AgentStepPayload = serde_json::from_value(val).expect("Roundtrip deserialization failed");
    assert_eq!(roundtripped, payload);
}

// ============================================================================
// 6. MARKDOWN CODEBLOCK PARSING AND ROBUST EXTRACTION
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_markdown_codeblock_and_conversational_wrapping() {
    let conn = setup_test_db();
    let engine = ReActEngine::new();

    let generate_fn = move |_req: LlmGenerateRequest| async move {
        Ok(r#"Certainly, here is the tool execution request:
```json
{
    "thought": "Checking system status with wrapped response",
    "tool": "get_system_status",
    "arguments": {}
}
```
Hope that helps!"#.to_string())
    };

    let streamed_steps = Arc::new(std::sync::Mutex::new(Vec::<AgentStepPayload>::new()));
    let streamed_steps_clone = streamed_steps.clone();

    let engine_single = engine.with_max_steps(1);
    let output = engine_single
        .execute_with_llm(
            &conn,
            "System",
            "Status",
            None,
            generate_fn,
            move |step| {
                streamed_steps_clone.lock().unwrap().push(step.clone());
            },
        )
        .await
        .expect("Execution must parse markdown codeblocks successfully");

    assert!(output.completed);
    assert_eq!(output.steps[0].action, "get_system_status");
    assert_eq!(output.steps[0].thought, "Checking system status with wrapped response");
}

// ============================================================================
// 7. DIRECT CONVERSATIONAL RESPONSE WITHOUT TOOLS (0 Tool Calls)
// ============================================================================
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_direct_conversational_response_without_tools() {
    let conn = setup_test_db();
    let engine = ReActEngine::new();

    let generate_fn = move |_req: LlmGenerateRequest| async move {
        Ok(r#"{
            "thought": "This is a simple greeting and does not require tool execution.",
            "final_answer": "Hello! I am Deyana, your personal AI assistant. How may I help you today?"
        }"#.to_string())
    };

    let streamed_steps = Arc::new(std::sync::Mutex::new(Vec::<AgentStepPayload>::new()));
    let streamed_steps_clone = streamed_steps.clone();

    let output = engine
        .execute_with_llm(
            &conn,
            "System",
            "Hello Deyana",
            None,
            generate_fn,
            move |step| {
                streamed_steps_clone.lock().unwrap().push(step.clone());
            },
        )
        .await
        .expect("Execution must succeed for direct conversational answer");

    assert!(output.completed);
    assert_eq!(output.step_count, 1, "Must complete in exactly 1 turn");
    assert_eq!(output.steps[0].action, "final_answer");
    assert_eq!(output.steps[0].status, "completed");
    assert!(output.final_answer.contains("Hello! I am Deyana"));
}
