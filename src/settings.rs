//! Cấu hình người dùng, lưu giữa các lần chạy.
//!
//! Windows: `%APPDATA%\VieEdit\settings.json`
//! Linux:   `~/.config/viedit/settings.json`

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Settings {
    pub library_root: Option<String>,
    pub lang: Option<String>,
}

pub fn config_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let base = std::env::var_os("APPDATA")?;
        Some(PathBuf::from(base).join("VieEdit"))
    }
    #[cfg(not(windows))]
    {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(xdg).join("viedit"));
        }
        let home = std::env::var_os("HOME")?;
        Some(PathBuf::from(home).join(".config").join("viedit"))
    }
}

fn path() -> Option<PathBuf> {
    Some(config_dir()?.join("settings.json"))
}

pub fn load() -> Settings {
    let Some(p) = path() else {
        return Settings::default();
    };
    let Ok(text) = std::fs::read_to_string(p) else {
        return Settings::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

pub fn save(settings: &Settings) {
    let Some(p) = path() else { return };
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(p, text);
    }
}
