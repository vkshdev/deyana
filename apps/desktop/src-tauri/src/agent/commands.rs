use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};


use crate::db::{self, DbPool};
use crate::models::{self, LlmState};
use super::prompt::build_system_prompt;
use super::retrieval::retrieve_memory_context;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatusResponse {
    pub status: String,
    pub active_session_id: Option<String>,
    pub persona: String,
}

pub async fn execute_agent_chat(
    app: &AppHandle,
    pool: &DbPool,
    state: &LlmState,
    session_id: Option<String>,
    prompt: String,
    model: Option<String>,
) -> Result<String, String> {
    if prompt.trim().is_empty() {
        return Err("Prompt cannot be empty".to_string());
    }

    let sess_id = session_id.unwrap_or_else(|| "default".to_string());

    // 1. Generate query embedding asynchronously via Ollama
    let query_embedding = state.router.generate_embedding(&prompt).await.ok();

    // 2. Retrieve relevant memory context from SQLite using FTS5 + Vector Cosine Similarity
    let conn = pool.get().map_err(|e| e.to_string())?;
    let memory_items = retrieve_memory_context(
        &conn,
        query_embedding.as_deref(),
        &prompt,
        5,
    ).unwrap_or_default();

    // 3. Save user chat message
    let _ = db::save_chat_message(&conn, Some(&sess_id), "user", &prompt, model.as_deref());

    // 4. Build system prompt with persona + memory context
    let system_prompt = build_system_prompt(&memory_items, None);

    // 5. Construct LLM generate request
    let req = models::LlmGenerateRequest {
        prompt: prompt.clone(),
        model: model.clone(),
        system: Some(system_prompt),
        temperature: Some(0.7),
        stream: Some(true),
    };

    let app_handle = app.clone();

    // 6. Generate stream from LLM router emitting stream chunks
    let assistant_response = state
        .router
        .generate_response(&req, move |chunk| {
            let _ = app_handle.emit("llm:stream_chunk", &chunk);
        })
        .await?;

    // 7. Save assistant message to SQLite
    let _ = db::save_chat_message(
        &conn,
        Some(&sess_id),
        "assistant",
        &assistant_response,
        model.as_deref(),
    );

    // 8. Emit agent:done event
    let _ = app.emit("agent:done", &assistant_response);

    // 9. If voice mode is enabled and TTS is enabled, trigger speech playback
    if let Some(voice_state) = app.try_state::<crate::voice::VoiceState>() {
        let settings = voice_state.service.read_settings();
        if voice_state.service.is_voice_mode_enabled() && settings.tts_enabled && !assistant_response.trim().is_empty() {
            let app_clone = app.clone();
            let text_to_speak = assistant_response.clone();
            tauri::async_runtime::spawn_blocking(move || {
                if let Some(vstate) = app_clone.try_state::<crate::voice::VoiceState>() {
                    let _ = vstate.service.speak(crate::voice::types::VoiceSpeakRequest {
                        text: text_to_speak,
                    });
                }
            });
        }
    }

    Ok(assistant_response)
}


#[tauri::command]
pub async fn agent_chat(
    app: AppHandle,
    pool: State<'_, DbPool>,
    state: State<'_, LlmState>,
    session_id: Option<String>,
    prompt: String,
    model: Option<String>,
) -> Result<String, String> {
    execute_agent_chat(&app, &pool, &state, session_id, prompt, model).await
}

#[tauri::command]
pub fn get_agent_status() -> AgentStatusResponse {
    AgentStatusResponse {
        status: "ready".to_string(),
        active_session_id: Some("default".to_string()),
        persona: "Deyana".to_string(),
    }
}
