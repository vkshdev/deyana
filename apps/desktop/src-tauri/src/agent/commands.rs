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

    let cognition_state = app.try_state::<crate::cognition::CognitionState>();
    let current_affective = cognition_state
        .as_ref()
        .map(|cs| cs.affective.read().unwrap().current());
    let pending_correction = cognition_state
        .as_ref()
        .and_then(|cs| cs.pending_correction.write().unwrap().take());
    let self_model = db::get_self_model(&conn)
        .ok()
        .flatten()
        .or_else(|| db::seed_self_model_if_empty(&conn).ok());

    let tool_descriptions = ReActEngine::build_tool_descriptions();
    let system_prompt = build_system_prompt(
        &memory_items,
        Some(&tool_descriptions),
        Some(active_n_ctx),
        current_affective.as_ref(),
        self_model.as_ref(),
        pending_correction.as_deref(),
    );

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

    // Cognition: Update affective state based on turn signals
    if let Some(cs) = &cognition_state {
        let tool_failures = output.steps.iter().filter(|s| s.status == "failed").count();
        let new_state = cs.affective.write().unwrap().evaluate_turn_signals(
            &prompt,
            &assistant_response,
            tool_failures,
            None,
        );
        let _ = app.emit("cognition:affective_state_changed", &new_state);
    }

    // Background Async Task: Automated conversation fact extraction
    let app_fact = app.clone();
    let pool_fact = pool.clone();
    let router_fact = state.router.clone();
    let prompt_fact = prompt.clone();
    let resp_fact = assistant_response.clone();
    let sess_fact = Some(sess_id.clone());
    tauri::async_runtime::spawn(async move {
        crate::cognition::fact_extractor::FactExtractor::process_turn_async(
            app_fact,
            pool_fact,
            router_fact,
            prompt_fact,
            resp_fact,
            sess_fact,
        ).await;
    });

    // Background Async Task: Metacognitive self-reflection pass
    if let Some(cs) = &cognition_state {
        let app_refl = app.clone();
        let pool_refl = pool.clone();
        let router_refl = state.router.clone();
        let cs_refl = cs.inner().clone();
        let prompt_refl = prompt.clone();
        let resp_refl = assistant_response.clone();
        let sess_refl = Some(sess_id.clone());
        tauri::async_runtime::spawn(async move {
            crate::cognition::reflection::MetacognitiveMonitor::evaluate_turn(
                app_refl,
                pool_refl,
                router_refl,
                cs_refl,
                prompt_refl,
                resp_refl,
                sess_refl,
            ).await;
        });
    }

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

#[tauri::command]
pub fn get_mind_state(
    state: State<'_, crate::cognition::CognitionState>,
    pool: State<'_, DbPool>,
) -> Result<db::MindCognitionState, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    let affective_state = state.affective.read().unwrap().current();
    let self_model = db::get_self_model(&conn)
        .map_err(|e| e.to_string())?
        .or_else(|| db::seed_self_model_if_empty(&conn).ok())
        .ok_or_else(|| "Failed to retrieve or initialize self model".to_string())?;
    let pending_correction_note = state.pending_correction.read().unwrap().clone();
    let recent_reflections = db::get_recent_reflections(&conn, Some(10)).unwrap_or_default();
    let recent_proactive_thoughts = db::get_recent_proactive_thoughts(&conn, Some(10)).unwrap_or_default();

    Ok(db::MindCognitionState {
        affective_state,
        self_model,
        pending_correction_note,
        recent_reflections,
        recent_proactive_thoughts,
    })
}

#[tauri::command]
pub fn get_self_model(
    pool: State<'_, DbPool>,
) -> Result<db::SelfModel, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    db::get_self_model(&conn)
        .map_err(|e| e.to_string())?
        .or_else(|| db::seed_self_model_if_empty(&conn).ok())
        .ok_or_else(|| "Self model not found".to_string())
}

#[tauri::command]
pub fn update_self_model(
    pool: State<'_, DbPool>,
    field: String,
    new_value: serde_json::Value,
    rationale: String,
) -> Result<db::SelfModel, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    db::update_self_model_field(&conn, &field, &new_value, &rationale, "user_action")
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_self_model_audit_log(
    pool: State<'_, DbPool>,
    limit: Option<usize>,
) -> Result<Vec<db::SelfModelAuditEntry>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    db::get_self_model_audit_log(&conn, limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_affective_state(
    state: State<'_, crate::cognition::CognitionState>,
) -> Result<crate::cognition::AffectiveState, String> {
    Ok(state.affective.read().unwrap().current())
}

#[tauri::command]
pub async fn trigger_proactive_thought(
    app: AppHandle,
    pool: State<'_, DbPool>,
    state: State<'_, LlmState>,
) -> Result<db::ProactiveThoughtRecord, String> {
    let result = crate::cognition::proactive::ProactiveThoughtGenerator::execute_cycle(
        &app,
        &pool,
        &state.router,
        "user_manual_trigger",
    ).await?;

    result.ok_or_else(|| "Insufficient memory items stored to generate cross-memory proactive insight (requires at least 2 memories).".to_string())
}

#[tauri::command]
pub fn list_pending_memory_saves(
    pool: State<'_, DbPool>,
) -> Result<Vec<db::MemorySaveSuggestion>, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    db::list_pending_memory_saves(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn approve_memory_save(
    app: AppHandle,
    pool: State<'_, DbPool>,
    cache: State<'_, SemanticCache>,
    id: String,
    title: Option<String>,
    fact_text: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<db::MemoryItem, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    let saved = db::approve_memory_save(&conn, &id, title.as_deref(), fact_text.as_deref(), tags)
        .map_err(|e| e.to_string())?;
    cache.clear(Some(&conn));
    let _ = app.emit("memory:cache_invalidated", ());
    Ok(saved)
}

#[tauri::command]
pub fn dismiss_memory_save(
    pool: State<'_, DbPool>,
    id: String,
) -> Result<bool, String> {
    let conn = pool.get().map_err(|e| e.to_string())?;
    db::dismiss_memory_save(&conn, &id).map_err(|e| e.to_string())
}


