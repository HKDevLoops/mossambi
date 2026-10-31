//! Persistent Configuration System for Mossambi

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputMode {
    Sibling,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MossambiConfig {
    pub theme_mode: ThemeMode,
    pub citrus_shade: String, // "rawpeel", "sweetzest", "ripepulp", "citrusgold", "deepforest", "tangerine"
    pub output_mode: OutputMode,
    pub custom_output_dir: Option<PathBuf>,
    pub always_on_top: bool,
    pub play_sound: bool,
    pub launch_at_login: bool,
    pub wheel_size: u32, // 380, 440, 500
    pub enable_gesture_wakeup: bool,
    pub enable_hotkey_wakeup: bool,
    pub enforce_zero_loss: bool,
}

impl Default for MossambiConfig {
    fn default() -> Self {
        Self {
            theme_mode: ThemeMode::System,
            citrus_shade: "rawpeel".to_string(),
            output_mode: OutputMode::Sibling,
            custom_output_dir: None,
            always_on_top: true,
            play_sound: true,
            launch_at_login: false,
            wheel_size: 440,
            enable_gesture_wakeup: true,
            enable_hotkey_wakeup: true,
            enforce_zero_loss: true,
        }
    }
}

impl MossambiConfig {
    pub fn config_path() -> PathBuf {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".mossambi").join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str(&content) {
                    return cfg;
                }
            }
        }
        let default_cfg = Self::default();
        let _ = default_cfg.save();
        default_cfg
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)?;
        fs::write(path, data)?;
        Ok(())
    }
}
