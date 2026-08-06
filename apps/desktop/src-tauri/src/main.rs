#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod agent;
mod browser;
mod commands;
mod connectors;
mod daemon;
mod db;
mod models;
mod privacy;
mod process;
mod settings;
mod tools;
mod tray;
mod voice;
mod window;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .manage(process::CoreProcessManager::default())
        .manage(models::LlmState::default())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();
            settings::ensure_settings_dir(&handle)?;

            let app_dir = handle
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from(".deyana"));
            let db_path = app_dir.join("deyana_core.sqlite");
            let db_pool = db::init_db(&db_path)
                .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
            let daemon_manager = daemon::DaemonManager::spawn(handle.clone(), db_pool.clone());
            let privacy_state = privacy::PrivacyState::new(db_pool.clone());
            let connector_state = connectors::ConnectorState::new(db_pool.clone());
            let tool_state = tools::ToolState::new(db_pool.clone());
            let voice_state = voice::VoiceState::new(app_dir.clone(), db_pool.clone());
            let browser_state = browser::BrowserState::new(app_dir.clone(), db_pool.clone());

            app.manage(db_pool);
            app.manage(daemon_manager);
            app.manage(privacy_state);
            app.manage(connector_state);
            app.manage(tool_state);
            app.manage(voice_state);
            app.manage(browser_state);

            let main_window = window::configure_main_window(&handle)?;
            window::attach_position_persistence(&handle, &main_window);
            tray::create_tray(&handle)?;
            let core_manager = app.state::<process::CoreProcessManager>();
            core_manager.start(&handle)?;
            process::record_desktop_ready(&handle)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_desktop_settings,
            commands::set_floating_mode,
            commands::set_always_on_top,
            commands::set_low_power_mode,
            commands::set_reduce_motion,
            commands::dock_floating_window,
            commands::show_main_window,
            commands::hide_main_window,
            commands::get_core_status,
            commands::restart_core,
            commands::stop_core,
            commands::open_vault_folder,
            commands::get_memory_items,
            commands::save_memory_item,
            commands::get_triage_inbox,
            commands::resolve_triage_item,
            commands::save_chat_message,
            commands::get_chat_history,
            commands::generate_response,
            commands::list_models,
            commands::get_model_status,
            privacy::commands::evaluate_privacy_policy,
            privacy::commands::get_privacy_audit_logs,
            privacy::commands::get_privacy_status,
            privacy::commands::update_privacy_rules,
            connectors::commands::list_connectors,
            connectors::commands::get_connector,
            connectors::commands::update_connector_settings,
            connectors::commands::start_connector_oauth,
            connectors::commands::complete_connector_oauth,
            connectors::commands::disconnect_connector,
            connectors::commands::sync_connector,
            connectors::commands::list_connector_sync_runs,
            connectors::commands::get_connector_health,
            tools::commands::list_tools,
            tools::commands::web_search_tool,
            tools::commands::fetch_page_tool,
            tools::commands::read_file_tool,
            tools::commands::git_status_tool,
            tools::commands::git_diff_tool,
            tools::commands::commit_message_tool,
            tools::commands::code_task_tool,
            tools::commands::day_planner_tool,
            voice::commands::get_voice_settings,
            voice::commands::patch_voice_settings,
            voice::commands::get_voice_status,
            voice::commands::transcribe_voice,
            voice::commands::speak_voice,
            voice::commands::interrupt_voice,
            browser::commands::get_browser_status,
            browser::commands::list_browser_sessions,
            browser::commands::disconnect_browser_session,
            browser::commands::list_browser_permissions,
            browser::commands::request_browser_permission,
            browser::commands::revoke_browser_permission,
            browser::commands::update_browser_context,
            browser::commands::read_browser_context,
            browser::commands::summarize_browser_context,
            browser::commands::browser_search,
            browser::commands::open_browser_tab,
            browser::commands::draft_browser_reply,
            browser::commands::fill_browser_field,
            browser::commands::clear_browser_field,
            browser::commands::list_browser_audit,
            browser::commands::list_browser_action_plans,
            browser::commands::create_browser_action_plan,
            browser::commands::confirm_browser_action_plan,
            browser::commands::execute_browser_action_plan,
            browser::commands::cancel_browser_action_plan,
            browser::commands::browser_emergency_stop,
            browser::commands::get_whatsapp_busy_mode_policy,
            browser::commands::patch_whatsapp_busy_mode_policy,
            browser::commands::evaluate_whatsapp_busy_mode,
            browser::commands::send_whatsapp_busy_reply,
            browser::commands::get_browser_personality,
            browser::commands::patch_browser_personality_profile,
            browser::commands::save_browser_contact_tone,
            browser::commands::infer_browser_mood,
            browser::commands::preview_browser_personality,
            browser::commands::route_browser_voice_command,
            agent::commands::agent_chat,
            agent::commands::get_agent_status
        ])
        .build(tauri::generate_context!())
        .expect("failed to build DEYANA desktop shell")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                let core_manager = app.state::<process::CoreProcessManager>();
                let _ = core_manager.stop(app, "app_exit");
            }
        });
}
