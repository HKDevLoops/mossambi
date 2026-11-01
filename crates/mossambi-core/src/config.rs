//! Persistent Configuration System for Mossambi

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    #[serde(alias = "system", alias = "liquidglass")]
    LiquidGlass,
    #[serde(alias = "frostedglass")]
    FrostedGlass,
    #[serde(alias = "dark", alias = "darkanthracite")]
    DarkAnthracite,
    #[serde(alias = "light", alias = "lightaqua")]
    LightAqua,
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
    #[serde(default)]
    pub autostart_prompted: Option<bool>,
}

impl Default for MossambiConfig {
    fn default() -> Self {
        Self {
            theme_mode: ThemeMode::LiquidGlass,
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
            autostart_prompted: None,
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

    /// Configures or removes Mossambi daemon from system autostart on boot/login.
    #[cfg(target_os = "windows")]
    pub fn set_autostart(enabled: bool) -> anyhow::Result<()> {
        use std::process::Command;
        let exe_path = std::env::current_exe()?;
        let exe_str = exe_path.to_string_lossy();
        if enabled {
            let cmd_str = format!("\"{}\" daemon run", exe_str);
            let _ = Command::new("reg")
                .args([
                    "add",
                    "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                    "/v",
                    "Mossambi",
                    "/t",
                    "REG_SZ",
                    "/d",
                    &cmd_str,
                    "/f",
                ])
                .output()?;
        } else {
            let _ = Command::new("reg")
                .args([
                    "delete",
                    "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                    "/v",
                    "Mossambi",
                    "/f",
                ])
                .output()?;
        }
        Ok(())
    }

    /// Checks if Mossambi is configured to run at startup.
    #[cfg(target_os = "windows")]
    pub fn is_autostart_configured() -> bool {
        use std::process::Command;
        if let Ok(output) = Command::new("reg")
            .args([
                "query",
                "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                "/v",
                "Mossambi",
            ])
            .output()
        {
            output.status.success()
        } else {
            false
        }
    }

    /// Configures or removes Mossambi daemon from system autostart on boot/login.
    #[cfg(not(target_os = "windows"))]
    pub fn set_autostart(enabled: bool) -> anyhow::Result<()> {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let autostart_dir = PathBuf::from(&home).join(".config").join("autostart");
        let desktop_file = autostart_dir.join("mossambi-daemon.desktop");
        if enabled {
            fs::create_dir_all(&autostart_dir)?;
            let exe_path = std::env::current_exe()?;
            let content = format!(
                "[Desktop Entry]\nType=Application\nName=Mossambi Daemon\nComment=Mossambi Background Service (Super+Shift+C / Super+Ctrl+C)\nExec={} daemon run\nHidden=false\nNoDisplay=true\nX-GNOME-Autostart-enabled=true\n",
                exe_path.display()
            );
            fs::write(desktop_file, content)?;
        } else if desktop_file.exists() {
            let _ = fs::remove_file(desktop_file);
        }
        Ok(())
    }

    /// Checks if Mossambi is configured to run at startup.
    #[cfg(not(target_os = "windows"))]
    pub fn is_autostart_configured() -> bool {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let desktop_file = PathBuf::from(&home).join(".config").join("autostart").join("mossambi-daemon.desktop");
        desktop_file.exists()
    }
}
