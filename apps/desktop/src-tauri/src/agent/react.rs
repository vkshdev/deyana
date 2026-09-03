use std::sync::Arc;
use std::time::Instant;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::agent::dispatcher::ToolDispatcher;
use crate::agent::grammar::{generate_tool_whitelist_grammar, parse_tool_call, ToolCallInvocation};
use crate::models::{LlmGenerateRequest, LlmRouter, LlmStreamChunk};

/// Hard safety step limit for the autonomous ReAct execution loop
pub const MAX_REACT_STEPS: usize = 5;

/// IPC payload streamed over Tauri event `agent:step` matching the frontend contract
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentStepPayload {
    pub step: usize,
    pub thought: String,
    pub action: String,
    pub action_input: serde_json::Value,
    pub observation: String,
    pub status: String, // "running" | "completed" | "failed"
    pub duration_ms: Option<u64>,
}

/// Result produced upon completing the ReAct engine execution
#[derive(Debug, Clone)]
pub struct ReActOutput {
    pub final_answer: String,
    pub steps: Vec<AgentStepPayload>,
    pub step_count: usize,
    pub completed: bool,
}

/// Thread-safe connection provider ensuring no un-synced Connection reference is held across await points
#[derive(Clone)]
pub enum ConnectionProvider<'a> {
    Pool(&'a crate::db::DbPool),
    Direct(Arc<std::sync::Mutex<Connection>>),
}

impl<'a> ConnectionProvider<'a> {
    pub fn execute_tool(&self, tool_name: &str, args: &serde_json::Value) -> Result<String, String> {
        match self {
            ConnectionProvider::Pool(pool) => {
                let conn = pool.get().map_err(|e| format!("DB pool checkout failed: {e}"))?;
                ToolDispatcher::dispatch(&conn, tool_name, args)
            }
            ConnectionProvider::Direct(arc) => {
                let conn = arc.lock().map_err(|e| format!("DB mutex lock poisoned: {e}"))?;
                ToolDispatcher::dispatch(&conn, tool_name, args)
            }
        }
    }
}

impl<'a> From<&'a crate::db::DbPool> for ConnectionProvider<'a> {
    fn from(pool: &'a crate::db::DbPool) -> Self {
        ConnectionProvider::Pool(pool)
    }
}

impl<'a> From<Arc<std::sync::Mutex<Connection>>> for ConnectionProvider<'a> {
    fn from(arc: Arc<std::sync::Mutex<Connection>>) -> Self {
        ConnectionProvider::Direct(arc)
    }
}

impl<'a> From<&'a Arc<std::sync::Mutex<Connection>>> for ConnectionProvider<'a> {
    fn from(arc: &'a Arc<std::sync::Mutex<Connection>>) -> Self {
        ConnectionProvider::Direct(arc.clone())
    }
}

/// Parse LLM output into ToolCallInvocation, handling clean JSON or markdown codeblocks
pub fn parse_react_response(output: &str) -> Option<ToolCallInvocation> {
    let trimmed = output.trim();
    if let Some(inv) = parse_tool_call(trimmed) {
        return Some(inv);
    }

    // Fallback: extract inner JSON object if output contains markdown code fence or outer chatter
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            if end > start {
                let slice = &trimmed[start..=end];
                if let Ok(inv) = serde_json::from_str::<ToolCallInvocation>(slice) {
                    if inv.tool.is_some() || inv.final_answer.is_some() {
                        return Some(inv);
                    }
                }
            }
        }
    }

    None
}

/// Autonomous multi-turn ReAct agent loop
pub struct ReActEngine {
    pub max_steps: usize,
    pub allowed_tools: Vec<&'static str>,
}

impl Default for ReActEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ReActEngine {
    pub fn new() -> Self {
        Self {
            max_steps: MAX_REACT_STEPS,
            allowed_tools: ToolDispatcher::available_tools(),
        }
    }

    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.max_steps = max_steps;
        self
    }

    pub fn with_tools(mut self, tools: Vec<&'static str>) -> Self {
        self.allowed_tools = tools;
        self
    }

    /// Build human-readable descriptions of available native tools for prompting
    pub fn build_tool_descriptions() -> String {
        r#"- search_memory: {"query": "<string>", "limit": <optional number>} — Search user's memory using full-text search.
- save_note: {"content": "<string>", "tags": ["<string>"]} — Save a new note or important fact into memory.
- get_system_status: {} — Query local system specifications, physical RAM, profile tier, and context token limit.
- get_triage_inbox: {} — Retrieve pending unread inbox triage items and notifications.

Response Instructions:
You MUST respond with valid JSON matching one of these two structures:

1. To run a tool:
{"thought": "<reasoning for calling this tool>", "tool": "<tool_name>", "arguments": {<parameters>}}

2. When you have enough information to answer the user's request:
{"thought": "<reasoning summarizing your conclusion>", "final_answer": "<your complete, helpful answer to the user>"}"#.to_string()
    }

    /// Execute the ReAct loop using the live LlmRouter
    pub async fn execute<'a, P, S, C>(
        &self,
        provider: P,
        router: &LlmRouter,
        model: Option<&str>,
        system_prompt: &str,
        user_prompt: &str,
        mut step_sink: S,
        mut stream_chunk_sink: C,
    ) -> Result<ReActOutput, String>
    where
        P: Into<ConnectionProvider<'a>>,
        S: FnMut(&AgentStepPayload),
        C: FnMut(LlmStreamChunk) + Send + 'static,
    {
        let conn_provider: ConnectionProvider<'a> = provider.into();
        let grammar_str = generate_tool_whitelist_grammar(&self.allowed_tools);
        let mut steps_history: Vec<AgentStepPayload> = Vec::new();
        let mut scratchpad = String::new();
        let mut final_answer_opt: Option<String> = None;
        let loop_start = Instant::now();

        for step in 1..=self.max_steps {
            let step_prompt = if scratchpad.is_empty() {
                user_prompt.to_string()
            } else {
                format!(
                    "{}\n\n### PREVIOUS ACTIONS AND OBSERVATIONS ###\n{}\nContinue reasoning to answer the user's request. Provide either a tool call or your final_answer in JSON format.",
                    user_prompt,
                    scratchpad.trim()
                )
            };

            let req = LlmGenerateRequest {
                prompt: step_prompt,
                model: model.map(|s| s.to_string()),
                system: Some(system_prompt.to_string()),
                temperature: Some(0.2),
                stream: Some(true),
                grammar: Some(grammar_str.clone()),
            };

            let step_start = Instant::now();
            let raw_response = router
                .generate_response(&req, |chunk| {
                    stream_chunk_sink(chunk);
                })
                .await?;

            let parsed = parse_react_response(&raw_response);

            match parsed {
                Some(ToolCallInvocation {
                    thought,
                    tool: Some(tool_name),
                    arguments,
                    ..
                }) => {
                    let thought_str = thought.unwrap_or_else(|| format!("Executing tool '{tool_name}'"));
                    let args_val = arguments.unwrap_or_else(|| serde_json::json!({}));

                    // Emit status: "running"
                    let mut step_payload = AgentStepPayload {
                        step,
                        thought: thought_str.clone(),
                        action: tool_name.clone(),
                        action_input: args_val.clone(),
                        observation: String::new(),
                        status: "running".to_string(),
                        duration_ms: None,
                    };
                    step_sink(&step_payload);

                    // Execute tool via connection provider
                    let tool_start = Instant::now();
                    let dispatch_result = conn_provider.execute_tool(&tool_name, &args_val);
                    let tool_duration = tool_start.elapsed().as_millis() as u64;

                    let (obs_str, status_str) = match dispatch_result {
                        Ok(obs) => (obs, "completed".to_string()),
                        Err(err) => (format!("Error executing tool '{tool_name}': {err}"), "failed".to_string()),
                    };

                    step_payload.observation = obs_str.clone();
                    step_payload.status = status_str;
                    step_payload.duration_ms = Some(tool_duration);

                    step_sink(&step_payload);
                    steps_history.push(step_payload);

                    scratchpad.push_str(&format!(
                        "Step {step}:\nThought: {thought_str}\nAction: {tool_name}({args_val})\nObservation: {obs_str}\n\n"
                    ));

                    // Check safety limit termination
                    if step == self.max_steps {
                        let fallback = format!(
                            "Reached maximum safety limit of {} steps while processing your request.\nSummary of findings:\n{}",
                            self.max_steps,
                            obs_str
                        );
                        let final_step = AgentStepPayload {
                            step: self.max_steps,
                            thought: format!("Reached maximum limit of {} steps. Concluding gracefully.", self.max_steps),
                            action: "final_answer".to_string(),
                            action_input: serde_json::json!({}),
                            observation: fallback.clone(),
                            status: "completed".to_string(),
                            duration_ms: Some(loop_start.elapsed().as_millis() as u64),
                        };
                        step_sink(&final_step);
                        steps_history.push(final_step);

                        final_answer_opt = Some(fallback);
                        break;
                    }
                }

                Some(ToolCallInvocation {
                    thought,
                    final_answer: Some(answer),
                    ..
                }) => {
                    let thought_str = thought.unwrap_or_else(|| "Formulated final answer".to_string());
                    let step_duration = step_start.elapsed().as_millis() as u64;

                    let step_payload = AgentStepPayload {
                        step,
                        thought: thought_str,
                        action: "final_answer".to_string(),
                        action_input: serde_json::json!({}),
                        observation: answer.clone(),
                        status: "completed".to_string(),
                        duration_ms: Some(step_duration),
                    };
                    step_sink(&step_payload);
                    steps_history.push(step_payload);

                    final_answer_opt = Some(answer);
                    break;
                }

                _ => {
                    // Direct unstructured response or non-tool output
                    let answer = raw_response.trim().to_string();
                    let step_duration = step_start.elapsed().as_millis() as u64;

                    let step_payload = AgentStepPayload {
                        step,
                        thought: "Provided direct response".to_string(),
                        action: "final_answer".to_string(),
                        action_input: serde_json::json!({}),
                        observation: answer.clone(),
                        status: "completed".to_string(),
                        duration_ms: Some(step_duration),
                    };
                    step_sink(&step_payload);
                    steps_history.push(step_payload);

                    final_answer_opt = Some(answer);
                    break;
                }
            }
        }

        let final_answer = final_answer_opt.unwrap_or_else(|| {
            format!(
                "Reached maximum limit of {} execution steps.",
                self.max_steps
            )
        });

        Ok(ReActOutput {
            final_answer,
            step_count: steps_history.len(),
            completed: true,
            steps: steps_history,
        })
    }

    /// Execute the ReAct loop using an arbitrary async generator closure (for deterministic testing)
    pub async fn execute_with_llm<'a, P, F, Fut, S>(
        &self,
        provider: P,
        system_prompt: &str,
        user_prompt: &str,
        model: Option<&str>,
        mut generate_fn: F,
        mut step_sink: S,
    ) -> Result<ReActOutput, String>
    where
        P: Into<ConnectionProvider<'a>>,
        F: FnMut(LlmGenerateRequest) -> Fut,
        Fut: std::future::Future<Output = Result<String, String>>,
        S: FnMut(&AgentStepPayload),
    {
        let conn_provider: ConnectionProvider<'a> = provider.into();
        let grammar_str = generate_tool_whitelist_grammar(&self.allowed_tools);
        let mut steps_history: Vec<AgentStepPayload> = Vec::new();
        let mut scratchpad = String::new();
        let mut final_answer_opt: Option<String> = None;
        let loop_start = Instant::now();

        for step in 1..=self.max_steps {
            let step_prompt = if scratchpad.is_empty() {
                user_prompt.to_string()
            } else {
                format!(
                    "{}\n\n### PREVIOUS ACTIONS AND OBSERVATIONS ###\n{}\nContinue reasoning to answer the user's request. Provide either a tool call or your final_answer in JSON format.",
                    user_prompt,
                    scratchpad.trim()
                )
            };

            let req = LlmGenerateRequest {
                prompt: step_prompt,
                model: model.map(|s| s.to_string()),
                system: Some(system_prompt.to_string()),
                temperature: Some(0.2),
                stream: Some(false),
                grammar: Some(grammar_str.clone()),
            };

            let step_start = Instant::now();
            let raw_response = generate_fn(req).await?;
            let parsed = parse_react_response(&raw_response);

            match parsed {
                Some(ToolCallInvocation {
                    thought,
                    tool: Some(tool_name),
                    arguments,
                    ..
                }) => {
                    let thought_str = thought.unwrap_or_else(|| format!("Executing tool '{tool_name}'"));
                    let args_val = arguments.unwrap_or_else(|| serde_json::json!({}));

                    // Emit status: "running"
                    let mut step_payload = AgentStepPayload {
                        step,
                        thought: thought_str.clone(),
                        action: tool_name.clone(),
                        action_input: args_val.clone(),
                        observation: String::new(),
                        status: "running".to_string(),
                        duration_ms: None,
                    };
                    step_sink(&step_payload);

                    // Execute tool via connection provider
                    let tool_start = Instant::now();
                    let dispatch_result = conn_provider.execute_tool(&tool_name, &args_val);
                    let tool_duration = tool_start.elapsed().as_millis() as u64;

                    let (obs_str, status_str) = match dispatch_result {
                        Ok(obs) => (obs, "completed".to_string()),
                        Err(err) => (format!("Error executing tool '{tool_name}': {err}"), "failed".to_string()),
                    };

                    step_payload.observation = obs_str.clone();
                    step_payload.status = status_str;
                    step_payload.duration_ms = Some(tool_duration);

                    step_sink(&step_payload);
                    steps_history.push(step_payload);

                    scratchpad.push_str(&format!(
                        "Step {step}:\nThought: {thought_str}\nAction: {tool_name}({args_val})\nObservation: {obs_str}\n\n"
                    ));

                    // Check safety limit termination
                    if step == self.max_steps {
                        let fallback = format!(
                            "Reached maximum safety limit of {} steps while processing your request.\nSummary of findings:\n{}",
                            self.max_steps,
                            obs_str
                        );
                        let final_step = AgentStepPayload {
                            step: self.max_steps,
                            thought: format!("Reached maximum limit of {} steps. Concluding gracefully.", self.max_steps),
                            action: "final_answer".to_string(),
                            action_input: serde_json::json!({}),
                            observation: fallback.clone(),
                            status: "completed".to_string(),
                            duration_ms: Some(loop_start.elapsed().as_millis() as u64),
                        };
                        step_sink(&final_step);
                        steps_history.push(final_step);

                        final_answer_opt = Some(fallback);
                        break;
                    }
                }

                Some(ToolCallInvocation {
                    thought,
                    final_answer: Some(answer),
                    ..
                }) => {
                    let thought_str = thought.unwrap_or_else(|| "Formulated final answer".to_string());
                    let step_duration = step_start.elapsed().as_millis() as u64;

                    let step_payload = AgentStepPayload {
                        step,
                        thought: thought_str,
                        action: "final_answer".to_string(),
                        action_input: serde_json::json!({}),
                        observation: answer.clone(),
                        status: "completed".to_string(),
                        duration_ms: Some(step_duration),
                    };
                    step_sink(&step_payload);
                    steps_history.push(step_payload);

                    final_answer_opt = Some(answer);
                    break;
                }

                _ => {
                    // Direct unstructured response or non-tool output
                    let answer = raw_response.trim().to_string();
                    let step_duration = step_start.elapsed().as_millis() as u64;

                    let step_payload = AgentStepPayload {
                        step,
                        thought: "Provided direct response".to_string(),
                        action: "final_answer".to_string(),
                        action_input: serde_json::json!({}),
                        observation: answer.clone(),
                        status: "completed".to_string(),
                        duration_ms: Some(step_duration),
                    };
                    step_sink(&step_payload);
                    steps_history.push(step_payload);

                    final_answer_opt = Some(answer);
                    break;
                }
            }
        }

        let final_answer = final_answer_opt.unwrap_or_else(|| {
            format!(
                "Reached maximum limit of {} execution steps.",
                self.max_steps
            )
        });

        Ok(ReActOutput {
            final_answer,
            step_count: steps_history.len(),
            completed: true,
            steps: steps_history,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::INIT_SQL;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn setup_test_db() -> Arc<std::sync::Mutex<Connection>> {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(INIT_SQL).unwrap();
        Arc::new(std::sync::Mutex::new(conn))
    }

    #[tokio::test]
    async fn test_react_direct_final_answer() {
        let conn = setup_test_db();
        let engine = ReActEngine::new();
        let mut emitted_steps = Vec::new();

        let generator = |_req: LlmGenerateRequest| async {
            Ok(r#"{"thought": "I know this immediately", "final_answer": "Hello! I am Deyana."}"#.to_string())
        };

        let result = engine
            .execute_with_llm(
                &conn,
                "System prompt",
                "Hello",
                None,
                generator,
                |step| emitted_steps.push(step.clone()),
            )
            .await
            .unwrap();

        assert_eq!(result.final_answer, "Hello! I am Deyana.");
        assert_eq!(emitted_steps.len(), 1);
        assert_eq!(emitted_steps[0].action, "final_answer");
        assert_eq!(emitted_steps[0].status, "completed");
        assert_eq!(emitted_steps[0].step, 1);
    }

    #[tokio::test]
    async fn test_react_multi_turn_tool_then_final_answer() {
        let conn = setup_test_db();
        let engine = ReActEngine::new();
        let mut emitted_steps = Vec::new();
        let call_count = Arc::new(AtomicUsize::new(0));

        let count_clone = call_count.clone();
        let generator = move |_req: LlmGenerateRequest| {
            let count = count_clone.fetch_add(1, Ordering::SeqCst);
            async move {
                if count == 0 {
                    // Turn 1: Call get_system_status tool
                    Ok(r#"{"thought": "Check system status", "tool": "get_system_status", "arguments": {}}"#.to_string())
                } else {
                    // Turn 2: Synthesize final answer based on tool observation
                    Ok(r#"{"thought": "System status retrieved", "final_answer": "System is healthy and profile is active."}"#.to_string())
                }
            }
        };

        let result = engine
            .execute_with_llm(
                &conn,
                "System prompt",
                "What is system status?",
                None,
                generator,
                |step| emitted_steps.push(step.clone()),
            )
            .await
            .unwrap();

        assert_eq!(result.final_answer, "System is healthy and profile is active.");
        assert_eq!(call_count.load(Ordering::SeqCst), 2);

        // Turn 1 emits "running" then "completed", Turn 2 emits "completed" final_answer
        assert_eq!(emitted_steps.len(), 3);
        assert_eq!(emitted_steps[0].action, "get_system_status");
        assert_eq!(emitted_steps[0].status, "running");
        assert_eq!(emitted_steps[1].action, "get_system_status");
        assert_eq!(emitted_steps[1].status, "completed");
        assert!(emitted_steps[1].observation.contains("total_ram_gb"));

        assert_eq!(emitted_steps[2].action, "final_answer");
        assert_eq!(emitted_steps[2].status, "completed");
        assert_eq!(emitted_steps[2].step, 2);
    }

    #[tokio::test]
    async fn test_react_safety_limit_termination() {
        let conn = setup_test_db();
        let engine = ReActEngine::new();
        let mut emitted_steps = Vec::new();
        let call_count = Arc::new(AtomicUsize::new(0));

        let count_clone = call_count.clone();
        let generator = move |_req: LlmGenerateRequest| {
            count_clone.fetch_add(1, Ordering::SeqCst);
            async move {
                // Model keeps asking to run get_system_status repeatedly
                Ok(r#"{"thought": "Need to check again", "tool": "get_system_status", "arguments": {}}"#.to_string())
            }
        };

        let result = engine
            .execute_with_llm(
                &conn,
                "System prompt",
                "Check status indefinitely",
                None,
                generator,
                |step| emitted_steps.push(step.clone()),
            )
            .await
            .unwrap();

        // Must terminate after MAX_REACT_STEPS (5)
        assert_eq!(call_count.load(Ordering::SeqCst), MAX_REACT_STEPS);
        assert!(result.final_answer.contains("Reached maximum safety limit of 5 steps"));
        assert!(emitted_steps.iter().any(|s| s.action == "final_answer" && s.step == 5));
    }

    #[tokio::test]
    async fn test_react_tool_failure_handling() {
        let conn = setup_test_db();
        let engine = ReActEngine::new();
        let mut emitted_steps = Vec::new();
        let call_count = Arc::new(AtomicUsize::new(0));

        let count_clone = call_count.clone();
        let generator = move |_req: LlmGenerateRequest| {
            let count = count_clone.fetch_add(1, Ordering::SeqCst);
            async move {
                if count == 0 {
                    // Call unknown tool
                    Ok(r#"{"thought": "Try running non-existent tool", "tool": "invalid_tool", "arguments": {}}"#.to_string())
                } else {
                    Ok(r#"{"thought": "Handled tool error", "final_answer": "Tool was unsupported, but I recovered."}"#.to_string())
                }
            }
        };

        let result = engine
            .execute_with_llm(
                &conn,
                "System prompt",
                "Run something invalid",
                None,
                generator,
                |step| emitted_steps.push(step.clone()),
            )
            .await
            .unwrap();

        assert_eq!(result.final_answer, "Tool was unsupported, but I recovered.");
        assert_eq!(emitted_steps[1].status, "failed");
        assert!(emitted_steps[1].observation.contains("Unknown or unsupported tool"));
    }

    #[tokio::test]
    async fn test_react_unstructured_fallback() {
        let conn = setup_test_db();
        let engine = ReActEngine::new();
        let mut emitted_steps = Vec::new();

        let generator = |_req: LlmGenerateRequest| async {
            // Direct plain text response without JSON wrapper
            Ok("I am Deyana, your personal AI assistant. How can I help you?".to_string())
        };

        let result = engine
            .execute_with_llm(
                &conn,
                "System prompt",
                "Who are you?",
                None,
                generator,
                |step| emitted_steps.push(step.clone()),
            )
            .await
            .unwrap();

        assert_eq!(result.final_answer, "I am Deyana, your personal AI assistant. How can I help you?");
        assert_eq!(emitted_steps.len(), 1);
        assert_eq!(emitted_steps[0].action, "final_answer");
        assert_eq!(emitted_steps[0].status, "completed");
    }

    #[tokio::test]
    async fn test_react_markdown_codeblock_parsing() {
        let conn = setup_test_db();
        let engine = ReActEngine::new();
        let mut emitted_steps = Vec::new();

        let generator = |_req: LlmGenerateRequest| async {
            // Markdown code fence wrapped JSON
            Ok("```json\n{\"thought\": \"Parsed from code block\", \"final_answer\": \"Success!\"}\n```".to_string())
        };

        let result = engine
            .execute_with_llm(
                &conn,
                "System prompt",
                "Test code fence",
                None,
                generator,
                |step| emitted_steps.push(step.clone()),
            )
            .await
            .unwrap();

        assert_eq!(result.final_answer, "Success!");
        assert_eq!(emitted_steps[0].thought, "Parsed from code block");
    }

    #[tokio::test]
    async fn test_stress_malformed_json_unclosed_brace() {
        let conn = setup_test_db();
        let engine = ReActEngine::new();
        let mut emitted = Vec::new();

        let malformed_output = r#"{"thought": "Searching memory", "tool": "search_memory", "arguments": {"query": "test""#;
        let generator = move |_req: LlmGenerateRequest| {
            let out = malformed_output.to_string();
            async move { Ok(out) }
        };

        let result = engine
            .execute_with_llm(
                &conn,
                "system prompt",
                "search something",
                None,
                generator,
                |s| emitted.push(s.clone()),
            )
            .await
            .expect("Engine should handle malformed JSON without error");

        assert_eq!(result.final_answer, malformed_output);
        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0].action, "final_answer");
        assert_eq!(emitted[0].status, "completed");
    }

    #[tokio::test]
    async fn test_stress_malformed_json_random_gibberish() {
        let conn = setup_test_db();
        let engine = ReActEngine::new();
        let mut emitted = Vec::new();

        let gibberish = "Some non-JSON text with random symbols @#$%^&*() and {unmatched braces";
        let generator = move |_req: LlmGenerateRequest| {
            let out = gibberish.to_string();
            async move { Ok(out) }
        };

        let result = engine
            .execute_with_llm(
                &conn,
                "system prompt",
                "tell me something",
                None,
                generator,
                |s| emitted.push(s.clone()),
            )
            .await
            .expect("Engine should handle gibberish without error");

        assert_eq!(result.final_answer, gibberish);
        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0].action, "final_answer");
    }

    #[tokio::test]
    async fn test_stress_markdown_code_fences_variations() {
        let conn = setup_test_db();
        let engine = ReActEngine::new();

        let chatter_with_fence = r#"
Sure! Here is the tool call you need:
```json
{
  "thought": "I will get the system status",
  "tool": "get_system_status",
  "arguments": {}
}
```
Let me know if you need anything else!
"#;
        let parsed = parse_react_response(chatter_with_fence).expect("Should parse fence with chatter");
        assert_eq!(parsed.thought.as_deref(), Some("I will get the system status"));
        assert_eq!(parsed.tool.as_deref(), Some("get_system_status"));

        let fence_no_tag = "```\n{\"thought\": \"Direct answer\", \"final_answer\": \"42\"}\n```";
        let parsed2 = parse_react_response(fence_no_tag).expect("Should parse fence with no tag");
        assert_eq!(parsed2.final_answer.as_deref(), Some("42"));

        let call_count = Arc::new(AtomicUsize::new(0));
        let count_clone = call_count.clone();
        let mut emitted = Vec::new();

        let generator = move |_req: LlmGenerateRequest| {
            let count = count_clone.fetch_add(1, Ordering::SeqCst);
            async move {
                if count == 0 {
                    Ok("```json\n{\"thought\": \"Check hardware\", \"tool\": \"get_system_status\", \"arguments\": {}}\n```".to_string())
                } else {
                    Ok("```json\n{\"thought\": \"Got it\", \"final_answer\": \"Status OK\"}\n```".to_string())
                }
            }
        };

        let result = engine
            .execute_with_llm(
                &conn,
                "sys",
                "check status",
                None,
                generator,
                |s| emitted.push(s.clone()),
            )
            .await
            .expect("Multi-turn with code fences should succeed");

        assert_eq!(result.final_answer, "Status OK");
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
        assert_eq!(emitted.len(), 3);
    }

    #[tokio::test]
    async fn test_stress_unknown_tool_recovery() {
        let conn = setup_test_db();
        let engine = ReActEngine::new();
        let mut emitted = Vec::new();
        let call_count = Arc::new(AtomicUsize::new(0));

        let count_clone = call_count.clone();
        let generator = move |req: LlmGenerateRequest| {
            let count = count_clone.fetch_add(1, Ordering::SeqCst);
            async move {
                if count == 0 {
                    Ok(r#"{"thought": "Attempting exploit", "tool": "exec_bash", "arguments": {"cmd": "rm -rf /"}}"#.to_string())
                } else {
                    assert!(req.prompt.contains("Unknown or unsupported tool: 'exec_bash'"));
                    Ok(r#"{"thought": "Bash is not supported", "final_answer": "I cannot run bash commands."}"#.to_string())
                }
            }
        };

        let result = engine
            .execute_with_llm(
                &conn,
                "system",
                "run rm",
                None,
                generator,
                |s| emitted.push(s.clone()),
            )
            .await
            .expect("Should handle unknown tool gracefully");

        assert_eq!(result.final_answer, "I cannot run bash commands.");
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
        assert_eq!(emitted[0].action, "exec_bash");
        assert_eq!(emitted[0].status, "running");
        assert_eq!(emitted[1].action, "exec_bash");
        assert_eq!(emitted[1].status, "failed");
        assert!(emitted[1].observation.contains("Unknown or unsupported tool"));
    }

    #[tokio::test]
    async fn test_stress_missing_tool_parameters() {
        let conn = setup_test_db();
        let engine = ReActEngine::new();
        let mut emitted = Vec::new();
        let call_count = Arc::new(AtomicUsize::new(0));

        let count_clone = call_count.clone();
        let generator = move |req: LlmGenerateRequest| {
            let count = count_clone.fetch_add(1, Ordering::SeqCst);
            async move {
                if count == 0 {
                    Ok(r#"{"thought": "Saving note with no content", "tool": "save_note", "arguments": {}}"#.to_string())
                } else {
                    assert!(req.prompt.contains("Missing required parameter 'content'"));
                    Ok(r#"{"thought": "Fixed", "final_answer": "Note could not be saved without content."}"#.to_string())
                }
            }
        };

        let result = engine
            .execute_with_llm(
                &conn,
                "system",
                "save empty note",
                None,
                generator,
                |s| emitted.push(s.clone()),
            )
            .await
            .expect("Should handle missing tool parameters gracefully");

        assert_eq!(result.final_answer, "Note could not be saved without content.");
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
        assert_eq!(emitted[1].status, "failed");
        assert!(emitted[1].observation.contains("Missing required parameter 'content'"));
    }

    #[tokio::test]
    async fn test_stress_infinite_loop_strict_5_step_bound() {
        let conn = setup_test_db();
        let engine = ReActEngine::new();
        let mut emitted = Vec::new();
        let call_count = Arc::new(AtomicUsize::new(0));

        let count_clone = call_count.clone();
        let generator = move |_req: LlmGenerateRequest| {
            count_clone.fetch_add(1, Ordering::SeqCst);
            async move {
                Ok(r#"{"thought": "Infinite loop check", "tool": "get_system_status", "arguments": {}}"#.to_string())
            }
        };

        let result = engine
            .execute_with_llm(
                &conn,
                "system",
                "loop forever",
                None,
                generator,
                |s| emitted.push(s.clone()),
            )
            .await
            .expect("Infinite loop must terminate without crashing");

        assert_eq!(call_count.load(Ordering::SeqCst), MAX_REACT_STEPS);
        assert_eq!(MAX_REACT_STEPS, 5);
        assert!(result.final_answer.contains("Reached maximum safety limit of 5 steps"));
        assert!(result.completed);
        assert_eq!(emitted.len(), 11);

        let last_event = emitted.last().expect("must have final event");
        assert_eq!(last_event.action, "final_answer");
        assert_eq!(last_event.status, "completed");
        assert_eq!(last_event.step, 5);
    }

    #[tokio::test]
    async fn test_stress_step_duration_tracking() {
        let conn = setup_test_db();
        let engine = ReActEngine::new();
        let mut emitted = Vec::new();

        let generator = |_req: LlmGenerateRequest| async {
            Ok(r#"{"thought": "Direct answer", "final_answer": "Finished quickly"}"#.to_string())
        };

        let result = engine
            .execute_with_llm(
                &conn,
                "system",
                "quick question",
                None,
                generator,
                |s| emitted.push(s.clone()),
            )
            .await
            .expect("Quick question should succeed");

        assert_eq!(result.final_answer, "Finished quickly");
        assert_eq!(emitted.len(), 1);
        assert!(emitted[0].duration_ms.is_some());
    }

    #[tokio::test]
    async fn test_stress_json_serialization_camel_case() {
        let payload = AgentStepPayload {
            step: 3,
            thought: "Analyzing results".to_string(),
            action: "search_memory".to_string(),
            action_input: serde_json::json!({"query": "deyana", "limit": 10}),
            observation: "Found 2 items".to_string(),
            status: "completed".to_string(),
            duration_ms: Some(150),
        };

        let json_str = serde_json::to_string(&payload).expect("Failed to serialize AgentStepPayload");
        assert!(json_str.contains("\"actionInput\":"), "Must serialize actionInput in camelCase");
        assert!(json_str.contains("\"durationMs\":"), "Must serialize durationMs in camelCase");
        assert!(!json_str.contains("\"action_input\":"), "Must not contain snake_case action_input");
        assert!(!json_str.contains("\"duration_ms\":"), "Must not contain snake_case duration_ms");

        let deserialized: AgentStepPayload = serde_json::from_str(&json_str).expect("Round-trip deserialization failed");
        assert_eq!(deserialized, payload);
    }

    #[tokio::test]
    async fn test_stress_unicode_and_special_characters() {
        let conn = setup_test_db();
        let engine = ReActEngine::new();
        let mut emitted = Vec::new();

        let unicode_thought = "Thinking about 🚀 AI and emojis: 🎯, \\n newlines and \"quotes\"";
        let unicode_answer = "Found answer: \u{2705} Success with utf-8 \u{1F30D}!";

        let generator = move |_req: LlmGenerateRequest| {
            let resp = format!(
                r#"{{"thought": "{}", "final_answer": "{}"}}"#,
                unicode_thought.replace('"', "\\\""),
                unicode_answer.replace('"', "\\\"")
            );
            async move { Ok(resp) }
        };

        let result = engine
            .execute_with_llm(
                &conn,
                "sys",
                "unicode test",
                None,
                generator,
                |s| emitted.push(s.clone()),
            )
            .await
            .expect("Unicode should succeed");

        assert!(result.final_answer.contains("Found answer: \u{2705} Success"));
        assert_eq!(emitted.len(), 1);
    }

    #[tokio::test]
    async fn test_stress_empty_and_whitespace_outputs() {
        let conn = setup_test_db();
        let engine = ReActEngine::new();
        let mut emitted = Vec::new();

        let generator = |_req: LlmGenerateRequest| async {
            Ok("   \n\t  \r\n  ".to_string())
        };

        let result = engine
            .execute_with_llm(
                &conn,
                "sys",
                "empty",
                None,
                generator,
                |s| emitted.push(s.clone()),
            )
            .await
            .expect("Whitespace output should not fail");

        assert_eq!(result.final_answer, "");
        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0].action, "final_answer");
    }

    #[tokio::test]
    async fn test_stress_arguments_as_array_or_primitive() {
        let conn = setup_test_db();
        let engine = ReActEngine::new();
        let mut emitted = Vec::new();
        let call_count = Arc::new(AtomicUsize::new(0));

        let count_clone = call_count.clone();
        let generator = move |_req: LlmGenerateRequest| {
            let count = count_clone.fetch_add(1, Ordering::SeqCst);
            async move {
                if count == 0 {
                    Ok(r#"{"thought": "array args", "tool": "search_memory", "arguments": ["unexpected", "array"]}"#.to_string())
                } else {
                    Ok(r#"{"thought": "recovered", "final_answer": "Done"}"#.to_string())
                }
            }
        };

        let result = engine
            .execute_with_llm(
                &conn,
                "sys",
                "bad args",
                None,
                generator,
                |s| emitted.push(s.clone()),
            )
            .await
            .expect("Non-object arguments must not panic");

        assert_eq!(result.final_answer, "Done");
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }
}
