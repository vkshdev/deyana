use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

use crate::{process, settings, window};

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show DEYANA", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "Hide", true, None::<&str>)?;
    let compact = MenuItem::with_id(app, "compact", "Compact mode", true, None::<&str>)?;
    let expanded = MenuItem::with_id(app, "expanded", "Expanded panel", true, None::<&str>)?;
    let always_on_top = MenuItem::with_id(app, "always_on_top", "Toggle always on top", true, None::<&str>)?;
    let restart_core = MenuItem::with_id(app, "restart_core", "Restart backend", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[
            &show,
            &hide,
            &separator,
            &compact,
            &expanded,
            &always_on_top,
            &separator,
            &restart_core,
            &separator,
            &quit,
        ],
    )?;

    TrayIconBuilder::with_id("main-tray")
        .tooltip("DEYANA")
        .icon(tray_icon())
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "hide" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            "compact" => {
                let _ = window::set_mode(app, "compact");
                let _ = settings::update_settings(app, |settings| {
                    settings.ui_mode = "compact".to_string();
                });
            }
            "expanded" => {
                let _ = window::set_mode(app, "expanded");
                let _ = settings::update_settings(app, |settings| {
                    settings.ui_mode = "expanded".to_string();
                });
            }
            "always_on_top" => {
                let next = !settings::read_settings(app).always_on_top;
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_always_on_top(next);
                }
                let _ = settings::update_settings(app, |settings| {
                    settings.always_on_top = next;
                });
            }
            "restart_core" => {
                let manager = app.state::<process::CoreProcessManager>();
                let _ = manager.restart(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

fn tray_icon() -> Image<'static> {
    const SIZE: u32 = 32;
    let mut rgba = vec![0_u8; (SIZE * SIZE * 4) as usize];
    let center = (SIZE as f32 - 1.0) / 2.0;

    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let distance = (dx * dx + dy * dy).sqrt();
            let index = ((y * SIZE + x) * 4) as usize;

            if distance <= 14.0 {
                rgba[index] = 12;
                rgba[index + 1] = 18;
                rgba[index + 2] = 22;
                rgba[index + 3] = 230;
            }

            if (11.0..=14.0).contains(&distance) {
                rgba[index] = 102;
                rgba[index + 1] = 227;
                rgba[index + 2] = 255;
                rgba[index + 3] = 255;
            }
        }
    }

    Image::new_owned(rgba, SIZE, SIZE)
}
