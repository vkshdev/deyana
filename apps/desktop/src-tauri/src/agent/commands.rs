use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::db::{self, DbPool};
use crate::models::{self, LlmState};
use super::cache::SemanticCache;
use super::prompt::{build_system_prompt, calculate_context_budget};
use super::react::ReActEngine;
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
    let active_n_ctx = state.router.active_n_ctx();
    let (max_items, _, _) = calculate_context_budget(active_n_ctx);

    let query_embedding = state.router.generate_embedding(&prompt).await.ok();

    let conn = pool.get().map_err(|e| e.to_string())?;
    let memory_items = retrieve_memory_context(
        &conn,
        query_embedding.as_deref(),
        &prompt,
        max_items,
    ).unwrap_or_default();

    let context_hash = SemanticCache::compute_context_hash(&memory_items);

    if let (Some(cache), Some(ref q_emb)) = (app.try_state::<SemanticCache>(), &query_embedding) {
        if let Some(cached_response) = cache.lookup(q_emb, &context_hash) {
            let _ = db::save_chat_message(&conn, Some(&sess_id), "user", &prompt, model.as_deref());

            let _ = app.emit(
                "llm:stream_chunk",
                &models::LlmStreamChunk {
                    response: cached_response.clone(),
                    done: true,
                    model: model.clone().or_else(|| Some("semantic_cache".to_string())),
                },
            );

            let _ = db::save_chat_message(
                &conn,
                Some(&sess_id),
                "assistant",
                &cached_response,
                Some("semantic_cache"),
            );

            let _ = app.emit("agent:done", &cached_response);

            if let Some(voice_state) = app.try_state::<crate::voice::VoiceState>() {
                let settings = voice_state.service.read_settings();
                if voice_state.service.is_voice_mode_enabled() && settings.tts_enabled && !cached_response.trim().is_empty() {
                    let app_clone = app.clone();
                    let text_to_speak = cached_response.clone();
                    tauri::async_runtime::spawn_blocking(move || {
                        if let Some(vstate) = app_clone.try_state::<crate::voice::VoiceState>() {
                            let _ = vstate.service.speak(crate::voice::types::VoiceSpeakRequest {
                                text: text_to_speak,
                            });
                        }
                    });
                }
            }

            return Ok(cached_response);
        }
    }

    let _ = db::save_chat_message(&conn, Some(&sess_id), "user", &prompt, model.as_deref());

    let tool_descriptions = ReActEngine::build_tool_descriptions();
    let system_prompt = build_system_prompt(&memory_items, Some(&tool_descriptions), Some(active_n_ctx));

    let engine = ReActEngine::new();
    let app_handle_step = app.clone();
    let app_handle_chunk = app.clone();

    let output = engine
        .execute(
            pool,
            &state.router,
            model.as_deref(),
            &system_prompt,
            &prompt,
            move |payload| {
                let _ = app_handle_step.emit("agent:step", payload);
            },
            move |chunk| {
                let _ = app_handle_chunk.emit("llm:stream_chunk", &chunk);
            },
        )
        .await?;

    let assistant_response = output.final_answer;

    let _ = db::save_chat_message(
        &conn,
        Some(&sess_id),
        "assistant",
        &assistant_response,
        model.as_deref(),
    );

    if let (Some(cache), Some(q_emb)) = (app.try_state::<SemanticCache>(), query_embedding) {
        cache.insert(q_emb, context_hash, assistant_response.clone(), Some(&conn));
    }

    let active_model_name = model.as_deref().unwrap_or("default");
    let _ = models::dataset::log_interaction(
        &conn,
        &prompt,
        &system_prompt,
        &assistant_response,
        active_model_name,
        None,
        Some(&sess_id),
    );

    let _ = app.emit("agent:done", &assistant_response);

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

#[tauri::command]
pub fn clear_semantic_cache(
    pool: State<'_, DbPool>,
    cache: State<'_, SemanticCache>,
) -> Result<usize, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    Ok(cache.clear(Some(&conn)))
}

#[tauri::command]
pub fn get_semantic_cache_stats(
    cache: State<'_, SemanticCache>,
) -> Result<serde_json::Value, String> {
    let (len, cap) = cache.stats();
    Ok(serde_json::json!({
        "itemCount": len,
        "capacity": cap
    }))
}

