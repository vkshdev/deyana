use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FloatingWindowPosition {
    pub x: i32,
    pub y: i32,
    pub monitor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Phase1Settings {
    pub ui_mode: String,
    pub always_on_top: bool,
    pub low_power_mode: bool,
    pub reduce_motion: bool,
    pub last_position: Option<FloatingWindowPosition>,
}

impl Default for Phase1Settings {
    fn default() -> Self {
        Self {
            ui_mode: "compact".to_string(),
            always_on_top: true,
            low_power_mode: true,
            reduce_motion: false,
            last_position: None,
        }
    }
}

pub fn ensure_settings_dir(app: &AppHandle) -> Result<(), String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?;

    fs::create_dir_all(dir).map_err(|error| error.to_string())
}

pub fn read_settings(app: &AppHandle) -> Phase1Settings {
    let path = match settings_path(app) {
        Ok(path) => path,
        Err(_) => return Phase1Settings::default(),
    };

    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return Phase1Settings::default(),
    };

    serde_json::from_str::<Phase1Settings>(&content).unwrap_or_default()
}

pub fn write_settings(app: &AppHandle, settings: &Phase1Settings) -> Result<(), String> {
    ensure_settings_dir(app)?;
    let path = settings_path(app)?;
    let temp_path = path.with_extension("json.tmp");
    let content = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;

    fs::write(&temp_path, content).map_err(|error| error.to_string())?;
    fs::rename(temp_path, path).map_err(|error| error.to_string())
}

pub fn update_settings(
    app: &AppHandle,
    update: impl FnOnce(&mut Phase1Settings),
) -> Result<Phase1Settings, String> {
    let mut settings = read_settings(app);
    update(&mut settings);
    write_settings(app, &settings)?;
    Ok(settings)
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?
        .join("phase1-settings.json"))
}

