//! Persistent Configuration System for Mossambi

use crate::core::pantone::{PantoneTheme, DARK_THEMES, LIGHT_THEMES};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    // 10 Certified Dark Themes
    #[default]
    #[serde(alias = "dark", alias = "system", alias = "anthracite_obsidian")]
    DarkAnthracite,
    #[serde(alias = "dark_forest", alias = "deep_forest_emerald")]
    DarkForest,
    #[serde(alias = "dark_eclipse", alias = "midnight_eclipse")]
    DarkEclipse,
    #[serde(alias = "dark_pewter", alias = "pewter_slate")]
    DarkPewter,
    #[serde(alias = "dark_tangerine", alias = "tangerine_nightfall")]
    DarkTangerine,
    #[serde(alias = "dark_caviar", alias = "citrus_caviar")]
    DarkCaviar,
    #[serde(alias = "dark_indigo", alias = "abyssal_indigo")]
    DarkIndigo,
    #[serde(alias = "dark_carbon", alias = "carbon_noir")]
    DarkCarbon,
    #[serde(alias = "dark_merlot", alias = "vintage_merlot")]
    DarkMerlot,
    #[serde(alias = "dark_gunmetal", alias = "gunmetal_steel")]
    DarkGunmetal,

    // 10 Certified Light Themes
    #[serde(alias = "light", alias = "light_brilliant", alias = "brilliant_lime")]
    LightBrilliant,
    #[serde(alias = "light_zest", alias = "sweet_zest_sunlight")]
    LightZest,
    #[serde(alias = "light_blanc", alias = "blanc_de_blanc")]
    LightBlanc,
    #[serde(alias = "light_glacier", alias = "glacier_aqua")]
    LightGlacier,
    #[serde(alias = "light_tangerine", alias = "tangerine_sorbet")]
    LightTangerine,
    #[serde(alias = "light_albedo", alias = "albedo_pure_ivory")]
    LightAlbedo,
    #[serde(alias = "light_quartz", alias = "cool_quartz")]
    LightQuartz,
    #[serde(alias = "light_pistachio", alias = "pistachio_cream")]
    LightPistachio,
    #[serde(alias = "light_amber", alias = "vanilla_amber")]
    LightAmber,
    #[serde(alias = "light_silver", alias = "silver_pearl")]
    LightSilver,

    // Legacy variants for code compatibility
    #[serde(alias = "liquidglass")]
    LiquidGlass,
    #[serde(alias = "frostedglass")]
    FrostedGlass,
    #[serde(alias = "lightaqua")]
    LightAqua,
}

impl ThemeMode {
    pub fn to_pantone_theme(&self) -> &'static PantoneTheme {
        match self {
            Self::DarkAnthracite | Self::LiquidGlass => &DARK_THEMES[0],
            Self::DarkForest => &DARK_THEMES[1],
            Self::DarkEclipse => &DARK_THEMES[2],
            Self::DarkPewter | Self::FrostedGlass => &DARK_THEMES[3],
            Self::DarkTangerine => &DARK_THEMES[4],
            Self::DarkCaviar => &DARK_THEMES[5],
            Self::DarkIndigo => &DARK_THEMES[6],
            Self::DarkCarbon => &DARK_THEMES[7],
            Self::DarkMerlot => &DARK_THEMES[8],
            Self::DarkGunmetal => &DARK_THEMES[9],

            Self::LightBrilliant => &LIGHT_THEMES[0],
            Self::LightZest => &LIGHT_THEMES[1],
            Self::LightBlanc => &LIGHT_THEMES[2],
            Self::LightGlacier | Self::LightAqua => &LIGHT_THEMES[3],
            Self::LightTangerine => &LIGHT_THEMES[4],
            Self::LightAlbedo => &LIGHT_THEMES[5],
            Self::LightQuartz => &LIGHT_THEMES[6],
            Self::LightPistachio => &LIGHT_THEMES[7],
            Self::LightAmber => &LIGHT_THEMES[8],
            Self::LightSilver => &LIGHT_THEMES[9],
        }
    }

    pub fn name(&self) -> &'static str {
        self.to_pantone_theme().name
    }

    pub fn is_dark(&self) -> bool {
        self.to_pantone_theme().is_dark
    }

    pub fn all() -> &'static [ThemeMode] {
        static ALL: [ThemeMode; 20] = [
            ThemeMode::DarkAnthracite,
            ThemeMode::DarkForest,
            ThemeMode::DarkEclipse,
            ThemeMode::DarkPewter,
            ThemeMode::DarkTangerine,
            ThemeMode::DarkCaviar,
            ThemeMode::DarkIndigo,
            ThemeMode::DarkCarbon,
            ThemeMode::DarkMerlot,
            ThemeMode::DarkGunmetal,
            ThemeMode::LightBrilliant,
            ThemeMode::LightZest,
            ThemeMode::LightBlanc,
            ThemeMode::LightGlacier,
            ThemeMode::LightTangerine,
            ThemeMode::LightAlbedo,
            ThemeMode::LightQuartz,
            ThemeMode::LightPistachio,
            ThemeMode::LightAmber,
            ThemeMode::LightSilver,
        ];
        &ALL
    }

    pub fn dark_themes() -> &'static [ThemeMode] {
        &Self::all()[0..10]
    }

    pub fn light_themes() -> &'static [ThemeMode] {
        &Self::all()[10..20]
    }

    pub fn next(&self) -> Self {
        let all = Self::all();
        let current_pos = all.iter().position(|m| m == self).unwrap_or(0);
        all[(current_pos + 1) % all.len()]
    }

    pub fn toggle_dark_light(&self) -> Self {
        if self.is_dark() {
            Self::LightBrilliant
        } else {
            Self::DarkAnthracite
        }
    }

    pub fn from_id_or_name(query: &str) -> Option<Self> {
        let q = query.to_lowercase().replace([' ', '-', '_'], "");
        if q == "liquidglass" || q == "system" {
            return Some(Self::LiquidGlass);
        }
        if q == "frostedglass" || q == "frosted" || q == "sonoma" {
            return Some(Self::FrostedGlass);
        }
        if q == "lightaqua" || q == "aqua" {
            return Some(Self::LightAqua);
        }
        if q == "dark" {
            return Some(Self::DarkAnthracite);
        }
        if q == "light" {
            return Some(Self::LightBrilliant);
        }
        for mode in Self::all() {
            let id = mode
                .to_pantone_theme()
                .id
                .to_lowercase()
                .replace([' ', '-', '_'], "");
            let name = mode
                .to_pantone_theme()
                .name
                .to_lowercase()
                .replace([' ', '-', '_'], "");
            if id == q || name == q {
                return Some(*mode);
            }
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GlassEffect {
    #[default]
    #[serde(alias = "frosted")]
    FrostedGlass,
    #[serde(alias = "liquid")]
    LiquidGlass,
    Opaque,
}

impl GlassEffect {
    pub fn all() -> &'static [GlassEffect] {
        &[
            GlassEffect::FrostedGlass,
            GlassEffect::LiquidGlass,
            GlassEffect::Opaque,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::FrostedGlass => "Frosted Glass (Sonoma Blur)",
            Self::LiquidGlass => "Liquid Glass (Sequoia / VisionOS)",
            Self::Opaque => "Opaque (Solid Container)",
        }
    }
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
    #[serde(default)]
    pub glass_effect: GlassEffect,
    #[serde(default = "default_window_opacity")]
    pub window_opacity: f32, // 0.20 to 1.00 (translucency)
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
    #[serde(default = "default_gesture_type")]
    pub gesture_type: String, // "shake", "circle", "both"
    #[serde(default = "default_shake_sensitivity")]
    pub shake_sensitivity_px: f32,
    #[serde(default = "default_gesture_timeout")]
    pub gesture_timeout_ms: u64,
    #[serde(default = "default_deep_drag")]
    pub deep_drag_detection_only: bool,
    #[serde(default = "default_show_hidden")]
    pub show_hidden_files: bool,
    #[serde(default = "default_explorer_layout")]
    pub default_explorer_layout: u8, // 0 = Miller 3-Column, 1 = Full-Screen, 2 = Dual-Pane
    #[serde(default = "default_calculate_checksums")]
    pub calculate_checksums: bool,
    #[serde(default = "default_tui_transparent")]
    pub tui_transparent_bg: bool,
    #[serde(default = "default_icon_pack")]
    pub icon_pack: String,
    #[serde(default)]
    pub font_family: Option<String>,
}

fn default_tui_transparent() -> bool {
    true
}

fn default_icon_pack() -> String {
    "macos".to_string()
}

fn default_show_hidden() -> bool {
    false
}

fn default_explorer_layout() -> u8 {
    0
}

fn default_calculate_checksums() -> bool {
    true
}

fn default_gesture_type() -> String {
    "both".to_string()
}

fn default_shake_sensitivity() -> f32 {
    25.0
}

fn default_gesture_timeout() -> u64 {
    450
}

fn default_deep_drag() -> bool {
    true
}

fn default_window_opacity() -> f32 {
    0.88
}

impl Default for MossambiConfig {
    fn default() -> Self {
        Self {
            theme_mode: ThemeMode::LightGlacier,
            glass_effect: GlassEffect::FrostedGlass,
            window_opacity: 0.92,
            citrus_shade: "classicblue".to_string(),
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
            gesture_type: "both".to_string(),
            shake_sensitivity_px: 25.0,
            gesture_timeout_ms: 450,
            deep_drag_detection_only: true,
            show_hidden_files: false,
            default_explorer_layout: 0,
            calculate_checksums: true,
            tui_transparent_bg: true,
            icon_pack: "macos".to_string(),
            font_family: None,
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
        let desktop_file = PathBuf::from(&home)
            .join(".config")
            .join("autostart")
            .join("mossambi-daemon.desktop");
        desktop_file.exists()
    }
}
