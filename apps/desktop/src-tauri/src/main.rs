#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod process;
mod settings;
mod tray;
mod window;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .manage(process::CoreProcessManager::default())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();
            settings::ensure_settings_dir(&handle)?;
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
            commands::open_vault_folder
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
