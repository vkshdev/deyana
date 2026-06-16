#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod process;
mod settings;
mod tray;
mod window;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            settings::ensure_settings_dir(&handle)?;
            let main_window = window::configure_main_window(&handle)?;
            window::attach_position_persistence(&handle, &main_window);
            tray::create_tray(&handle)?;
            process::record_phase1_ready(&handle)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_phase1_settings,
            commands::set_floating_mode,
            commands::set_always_on_top,
            commands::show_main_window,
            commands::hide_main_window
        ])
        .run(tauri::generate_context!())
        .expect("failed to run DE'YANA desktop shell");
}

