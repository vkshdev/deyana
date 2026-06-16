use tauri::{
    AppHandle, LogicalSize, Manager, PhysicalPosition, Position, Size, WebviewWindow, WindowEvent,
};

use crate::settings::{self, FloatingWindowPosition};

const COMPACT_WIDTH: f64 = 92.0;
const COMPACT_HEIGHT: f64 = 144.0;
const EXPANDED_WIDTH: f64 = 408.0;
const EXPANDED_HEIGHT: f64 = 652.0;
const DEFAULT_TOP_OFFSET: f64 = 108.0;
const DEFAULT_RIGHT_OFFSET: f64 = 24.0;
const MIN_EDGE_PADDING: f64 = 8.0;

pub fn configure_main_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window is not registered".to_string())?;
    let settings = settings::read_settings(app);

    apply_window_mode(&window, &settings.ui_mode, false)?;

    if let Some(position) = settings.last_position {
        window
            .set_position(Position::Physical(PhysicalPosition::new(position.x, position.y)))
            .map_err(|error| error.to_string())?;
    } else {
        place_default(&window, current_width(&settings.ui_mode))?;
    }

    window
        .set_always_on_top(settings.always_on_top)
        .map_err(|error| error.to_string())?;
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())?;

    Ok(window)
}

pub fn set_mode(app: &AppHandle, mode: &str) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window is not registered".to_string())?;

    apply_window_mode(&window, mode, true)?;
    record_current_position(app, &window)
}

pub fn attach_position_persistence(app: &AppHandle, window: &WebviewWindow) {
    let app = app.clone();
    let observed_window = window.clone();
    let listener_window = window.clone();

    listener_window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Moved(_)) {
            let _ = record_current_position(&app, &observed_window);
        }
    });
}

fn apply_window_mode(window: &WebviewWindow, mode: &str, preserve_right_edge: bool) -> Result<(), String> {
    let (width, height) = dimensions_for_mode(mode)?;

    let right_edge = if preserve_right_edge {
        window.outer_position().ok().zip(window.outer_size().ok()).map(|(position, size)| {
            position.x + size.width as i32
        })
    } else {
        None
    };

    window
        .set_size(Size::Logical(LogicalSize::new(width, height)))
        .map_err(|error| error.to_string())?;

    if let Some(right_edge) = right_edge {
        snap_after_resize(window, width, right_edge)?;
    }

    Ok(())
}

fn snap_after_resize(window: &WebviewWindow, width: f64, right_edge: i32) -> Result<(), String> {
    let monitor = active_monitor(window)?;
    let scale = monitor.as_ref().map(|monitor| monitor.scale_factor()).unwrap_or(1.0);
    let width_px = (width * scale).round() as i32;
    let edge_padding = (MIN_EDGE_PADDING * scale).round() as i32;
    let right_offset = (DEFAULT_RIGHT_OFFSET * scale).round() as i32;
    let current_y = window.outer_position().map(|position| position.y).unwrap_or_else(|_| {
        monitor
            .as_ref()
            .map(|monitor| monitor.position().y + (DEFAULT_TOP_OFFSET * scale).round() as i32)
            .unwrap_or((DEFAULT_TOP_OFFSET * scale).round() as i32)
    });

    let next_x = if let Some(monitor) = monitor {
        let left_limit = monitor.position().x + edge_padding;
        let right_limit = monitor.position().x + monitor.size().width as i32 - width_px - right_offset;
        (right_edge - width_px).clamp(left_limit, right_limit)
    } else {
        right_edge - width_px
    };

    window
        .set_position(Position::Physical(PhysicalPosition::new(next_x, current_y)))
        .map_err(|error| error.to_string())
}

fn place_default(window: &WebviewWindow, width: f64) -> Result<(), String> {
    let monitor = active_monitor(window)?;
    let scale = monitor.as_ref().map(|monitor| monitor.scale_factor()).unwrap_or(1.0);
    let width_px = (width * scale).round() as i32;
    let right_offset = (DEFAULT_RIGHT_OFFSET * scale).round() as i32;
    let top_offset = (DEFAULT_TOP_OFFSET * scale).round() as i32;

    let (x, y) = if let Some(monitor) = monitor {
        (
            monitor.position().x + monitor.size().width as i32 - width_px - right_offset,
            monitor.position().y + top_offset,
        )
    } else {
        (right_offset, top_offset)
    };

    window
        .set_position(Position::Physical(PhysicalPosition::new(x, y)))
        .map_err(|error| error.to_string())
}

fn record_current_position(app: &AppHandle, window: &WebviewWindow) -> Result<(), String> {
    let position = window.outer_position().map_err(|error| error.to_string())?;
    let monitor = active_monitor(window)?
        .map(|monitor| format!("{}x{}", monitor.size().width, monitor.size().height));

    settings::update_settings(app, |settings| {
        settings.last_position = Some(FloatingWindowPosition {
            x: position.x,
            y: position.y,
            monitor,
        });
    })?;

    Ok(())
}

fn current_width(mode: &str) -> f64 {
    dimensions_for_mode(mode)
        .map(|(width, _)| width)
        .unwrap_or(COMPACT_WIDTH)
}

fn dimensions_for_mode(mode: &str) -> Result<(f64, f64), String> {
    match mode {
        "compact" => Ok((COMPACT_WIDTH, COMPACT_HEIGHT)),
        "expanded" => Ok((EXPANDED_WIDTH, EXPANDED_HEIGHT)),
        _ => Err(format!("unsupported floating mode: {mode}")),
    }
}

fn active_monitor(window: &WebviewWindow) -> Result<Option<tauri::Monitor>, String> {
    match window.current_monitor().map_err(|error| error.to_string())? {
        Some(monitor) => Ok(Some(monitor)),
        None => window.primary_monitor().map_err(|error| error.to_string()),
    }
}
