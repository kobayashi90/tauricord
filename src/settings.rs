use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowGeometry {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Default for WindowGeometry {
    fn default() -> Self {
        Self { x: -1.0, y: -1.0, width: 800.0, height: 600.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub hide_on_close: bool,
    pub start_minimized: bool,
    pub launch_on_boot: bool,
    pub custom_css_path: Option<String>,
    pub spellcheck_language: Option<String>,
    pub memory_saver_minutes: Option<u64>,
    pub window_geometry: WindowGeometry,
    pub gpu_rendering: bool,
    pub version: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hide_on_close: true,
            start_minimized: false,
            launch_on_boot: false,
            custom_css_path: None,
            spellcheck_language: None,
            memory_saver_minutes: None,
            window_geometry: WindowGeometry::default(),
            gpu_rendering: true,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

impl Settings {
    pub fn config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
        let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
        Ok(dir.join("settings.json"))
    }

    pub fn load(app: &tauri::AppHandle) -> Self {
        let path = match Self::config_path(app) {
            Ok(p) => p,
            Err(_) => return Self::default(),
        };

        if !path.exists() {
            let settings = Self::default();
            let _ = settings.save(app);
            return settings;
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => {
                serde_json::from_str(&content).unwrap_or_else(|e| {
                    log::warn!("Failed to parse settings, using defaults: {e}");
                    Self::default()
                })
            }
            Err(e) => {
                log::warn!("Failed to read settings, using defaults: {e}");
                Self::default()
            }
        }
    }

    pub fn save(&self, app: &tauri::AppHandle) -> Result<(), String> {
        let path = Self::config_path(app)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, content).map_err(|e| e.to_string())?;
        Ok(())
    }
}
