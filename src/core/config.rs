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

/// How selection checkboxes are presented in the explorer surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CheckboxMode {
    /// Checkbox always visible on every row / tile.
    #[default]
    Always,
    /// Checkbox only materialises while the pointer is over the row / tile.
    Hover,
    /// No checkboxes at all (multi-select still works via Ctrl / Shift).
    Off,
}

impl CheckboxMode {
    pub fn all() -> &'static [CheckboxMode] {
        &[CheckboxMode::Always, CheckboxMode::Hover, CheckboxMode::Off]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Always => "Always visible",
            Self::Hover => "On hover only",
            Self::Off => "Off",
        }
    }

    pub fn visible_for(&self, hovered: bool) -> bool {
        match self {
            Self::Always => true,
            Self::Hover => hovered,
            Self::Off => false,
        }
    }
}

/// Visual treatment used for the explorer's right-click menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MenuStyle {
    /// Windows 11 Explorer shell layout and wording.
    #[default]
    Windows,
    /// macOS Finder layout and wording.
    Finder,
    /// Linux / Nautilus layout and wording.
    Linux,
}

impl MenuStyle {
    pub fn all() -> &'static [MenuStyle] {
        &[MenuStyle::Windows, MenuStyle::Finder, MenuStyle::Linux]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Windows => "Windows Explorer",
            Self::Finder => "macOS Finder",
            Self::Linux => "Linux (Nautilus)",
        }
    }
}

/// Motion primitive used for a single animated surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AnimStyle {
    /// Critically-damped physical spring - standard Apple HIG motion.
    #[default]
    Spring,
    /// Fast ease-out curve - snappy 150ms with zero overshoot.
    Snappy,
    /// Standard Apple HIG 250ms natural deceleration curve.
    Natural,
    /// Physical spring with dynamic overshoot and elastic settle.
    Bouncy,
    /// Smooth fluid Apple HIG 350ms ease-in-out curve.
    Gentle,
    /// Dramatic 600ms cinematic slow-motion showcase curve.
    Cinematic,
    /// Linear opacity fade.
    Fade,
    /// Motion disabled entirely (instantaneous snap).
    None,
}

impl AnimStyle {
    pub fn all() -> &'static [AnimStyle] {
        &[
            AnimStyle::Spring,
            AnimStyle::Snappy,
            AnimStyle::Natural,
            AnimStyle::Bouncy,
            AnimStyle::Gentle,
            AnimStyle::Cinematic,
            AnimStyle::Fade,
            AnimStyle::None,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Spring => "Spring (Physical)",
            Self::Snappy => "Snappy (150ms)",
            Self::Natural => "Natural (Apple HIG 250ms)",
            Self::Bouncy => "Bouncy (Overshoot)",
            Self::Gentle => "Gentle (Fluid 350ms)",
            Self::Cinematic => "Cinematic (600ms)",
            Self::Fade => "Fade (Linear)",
            Self::None => "None (Instant)",
        }
    }
}

/// Per-feature motion configuration. Every explorer/widget animation can be
/// re-tuned independently from Settings.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AnimationSettings {
    #[serde(default)]
    pub hover: AnimStyle,
    #[serde(default)]
    pub selection: AnimStyle,
    #[serde(default)]
    pub open: AnimStyle,
    #[serde(default)]
    pub peek: AnimStyle,
    #[serde(default)]
    pub toolbar: AnimStyle,
    #[serde(default)]
    pub widget: AnimStyle,
    #[serde(default)]
    pub scroll: AnimStyle,
    #[serde(default)]
    pub tui: AnimStyle,

    // Physics parameters for Spring solver
    #[serde(default = "default_spring_stiffness")]
    pub spring_stiffness: f32,
    #[serde(default = "default_spring_damping")]
    pub spring_damping: f32,
    #[serde(default = "default_spring_mass")]
    pub spring_mass: f32,

    // Feature toggles for unlimited animations
    #[serde(default)]
    pub reduced_motion: bool,
    #[serde(default = "default_true")]
    pub micro_interactions: bool,
    #[serde(default = "default_true")]
    pub shimmer_effects: bool,
    #[serde(default = "default_true")]
    pub widget_progress_ring: bool,
    #[serde(default = "default_true")]
    pub widget_group_fanout: bool,
    #[serde(default = "default_true")]
    pub widget_elastic_hover: bool,
}

fn default_spring_stiffness() -> f32 {
    240.0
}

fn default_spring_damping() -> f32 {
    1.0
}

fn default_spring_mass() -> f32 {
    1.0
}

impl Default for AnimationSettings {
    fn default() -> Self {
        Self {
            hover: AnimStyle::Spring,
            selection: AnimStyle::Snappy,
            open: AnimStyle::Natural,
            peek: AnimStyle::Gentle,
            toolbar: AnimStyle::Natural,
            widget: AnimStyle::Bouncy,
            scroll: AnimStyle::Natural,
            tui: AnimStyle::Natural,
            spring_stiffness: 240.0,
            spring_damping: 1.0,
            spring_mass: 1.0,
            reduced_motion: false,
            micro_interactions: true,
            shimmer_effects: true,
            widget_progress_ring: true,
            widget_group_fanout: true,
            widget_elastic_hover: true,
        }
    }
}

/// Accessor signature for one animated field inside [`AnimationSettings`].
pub type AnimFieldAccessor = fn(&mut AnimationSettings) -> &mut AnimStyle;

/// The full tuple describing one animation dropdown in Settings.
pub type AnimField = (&'static str, &'static str, AnimFieldAccessor);

impl AnimationSettings {
    pub fn all_fields() -> [AnimField; 8] {
        [
            ("hover", "Hover", |s| &mut s.hover),
            ("selection", "Selection", |s| &mut s.selection),
            ("open", "Open / close", |s| &mut s.open),
            ("peek", "Peek preview", |s| &mut s.peek),
            ("toolbar", "Toolbar", |s| &mut s.toolbar),
            ("widget", "Radial widget", |s| &mut s.widget),
            ("scroll", "Scroll", |s| &mut s.scroll),
            ("tui", "Terminal TUI", |s| &mut s.tui),
        ]
    }
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
    #[serde(default = "default_grid_icon_size")]
    pub grid_icon_size: u32, // 64..160
    #[serde(default)]
    pub checkbox_mode: CheckboxMode,
    #[serde(default)]
    pub menu_style: MenuStyle,
    #[serde(default)]
    pub animations: AnimationSettings,
    #[serde(default = "default_explorer_zoom")]
    pub explorer_zoom: u8, // 0 = smallest, 4 = largest
    #[serde(default = "default_background_work")]
    pub background_work: bool,
    #[serde(default = "default_worker_threads")]
    pub worker_threads: u8,
    /// Uncapped frame rate: disables vsync and drives continuous repaints
    /// for the highest possible responsiveness ("Max FPS" in Performance).
    #[serde(default = "default_max_fps")]
    pub max_fps: bool,
    /// Conversion target used when files are dropped on the global Drop
    /// Overlay from other apps. "AUTO" picks per file category
    /// (image→PNG, video→MP4, audio→FLAC, doc→PDF, …); any other value is
    /// treated as a literal target format (e.g. "PDF").
    #[serde(default = "default_drop_target")]
    pub drop_target_default: String,
    /// Show the Tags smart-list section in the sidebar (optional Finder-like
    /// tags feature). Off by default per user preference.
    #[serde(default)]
    pub show_tags_section: bool,
    /// Show user-defined pinned folders in the sidebar below the standard ones.
    /// Custom pins are added from a folder's right-click menu ("Pin to Sidebar").
    #[serde(default)]
    pub show_custom_pinned: bool,
    /// Custom pinned folder paths (absolute). Rendered only when
    /// `show_custom_pinned` is on.
    #[serde(default)]
    pub pinned_custom: Vec<String>,
    /// Keybinding profile: 0 = macOS Finder, 1 = Windows Explorer
    #[serde(default = "default_keybinding_profile")]
    pub keybinding_profile: u8,
    /// Enable local AI acceleration (Ollama / LM Studio)
    #[serde(default)]
    pub local_ai_enabled: bool,
    /// Local AI HTTP API endpoint (e.g. http://localhost:11434/api or http://localhost:1234/v1)
    #[serde(default = "default_local_ai_endpoint")]
    pub local_ai_endpoint: String,
    /// Local AI model name (e.g. qwen2.5-vl:7b, llama3.2-vision)
    #[serde(default = "default_local_ai_model")]
    pub local_ai_model: String,
    /// Local AI temperature / effort
    #[serde(default = "default_local_ai_temperature")]
    pub local_ai_temperature: f32,
    /// Retention period in days for staged deletion batches (.mossambi/Trash)
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
    /// Donut toolbar profiles and appearance controls (Filee parity)
    #[serde(default)]
    pub donut: crate::core::donut_config::DonutConfig,
    /// 12 Toggleable domain fields matching Filee Fields architecture
    #[serde(default)]
    pub fields: crate::core::fields_config::FieldsConfig,
    /// Keep the original file creation/modification dates for every conversion
    #[serde(default)]
    pub keep_original_file_dates: bool,
    /// Save every geospatial conversion result as a packaged ZIP
    #[serde(default)]
    pub save_geospatial_as_zip: bool,
    /// Target download staging directory for browser drag-out
    #[serde(default)]
    pub browser_staging_dir: Option<PathBuf>,
    /// Save only converted file and clean up downloaded browser temp artifact
    #[serde(default = "default_true")]
    pub browser_save_converted_only: bool,
    /// Whether to disable recording recent conversions history
    #[serde(default)]
    pub disable_conversion_history: bool,
    /// Maximum count of recent conversions kept in history
    #[serde(default = "default_conversion_history_limit")]
    pub conversion_history_limit: usize,
}

fn default_keybinding_profile() -> u8 {
    0
}

fn default_local_ai_endpoint() -> String {
    "http://localhost:11434/api".into()
}

fn default_local_ai_model() -> String {
    "qwen2.5-vl:7b".into()
}

fn default_local_ai_temperature() -> f32 {
    0.2
}

fn default_retention_days() -> u32 {
    7
}

fn default_true() -> bool {
    true
}

fn default_conversion_history_limit() -> usize {
    200
}

fn default_background_work() -> bool {
    true
}

fn default_worker_threads() -> u8 {
    0 // 0 = auto (available_parallelism)
}

fn default_max_fps() -> bool {
    true
}

fn default_drop_target() -> String {
    "AUTO".into()
}

fn default_explorer_zoom() -> u8 {
    2
}

fn default_grid_icon_size() -> u32 {
    92
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
            grid_icon_size: default_grid_icon_size(),
            checkbox_mode: CheckboxMode::default(),
            menu_style: MenuStyle::default(),
            animations: AnimationSettings::default(),
            explorer_zoom: default_explorer_zoom(),
            background_work: default_background_work(),
            worker_threads: default_worker_threads(),
            max_fps: default_max_fps(),
            drop_target_default: default_drop_target(),
            show_tags_section: false,
            show_custom_pinned: false,
            pinned_custom: Vec::new(),
            keybinding_profile: default_keybinding_profile(),
            local_ai_enabled: false,
            local_ai_endpoint: default_local_ai_endpoint(),
            local_ai_model: default_local_ai_model(),
            local_ai_temperature: default_local_ai_temperature(),
            retention_days: default_retention_days(),
            donut: crate::core::donut_config::DonutConfig::default(),
            fields: crate::core::fields_config::FieldsConfig::default(),
            keep_original_file_dates: false,
            save_geospatial_as_zip: false,
            browser_staging_dir: None,
            browser_save_converted_only: true,
            disable_conversion_history: false,
            conversion_history_limit: default_conversion_history_limit(),
        }
    }
}

/// Parsed-config cache keyed by (path, mtime).
///
/// The GUI calls [`MossambiConfig::load`] once per egui frame. Re-reading and
/// re-parsing the JSON document every frame was a measurable source of
/// frame-time jitter, so the parsed value is memoised and only refreshed
/// when the file on disk actually changes.
/// The memoised configuration document.
type CachedConfig = (PathBuf, Option<std::time::SystemTime>, MossambiConfig);

static CONFIG_CACHE: std::sync::OnceLock<std::sync::Mutex<Option<CachedConfig>>> =
    std::sync::OnceLock::new();

impl MossambiConfig {
    pub fn config_path() -> PathBuf {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".mossambi").join("config.json")
    }

    fn file_stamp(path: &std::path::Path) -> Option<std::time::SystemTime> {
        fs::metadata(path).ok().and_then(|m| m.modified().ok())
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        let stamp = Self::file_stamp(&path);

        let cache = CONFIG_CACHE.get_or_init(|| std::sync::Mutex::new(None));
        if let Ok(guard) = cache.lock() {
            if let Some((cached_path, cached_stamp, cfg)) = guard.as_ref() {
                if cached_path == &path && cached_stamp == &stamp {
                    return cfg.clone();
                }
            }
        }

        // NOTE: load() is side-effect free. A missing config simply starts
        // from defaults in memory; the first user-initiated `save()`
        // materialises the file. Writing defaults from inside load() used to
        // race with the per-frame GUI load and clobber freshly written prefs.
        let cfg = if stamp.is_some() {
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<MossambiConfig>(&content) {
                    Ok(mut parsed) => {
                        // Validation clamp: 0 stays "auto", values above the
                        // 64-thread ceiling are capped on load.
                        let raw_threads = parsed.worker_threads;
                        parsed.worker_threads = raw_threads.min(64);
                        if parsed.worker_threads != raw_threads {
                            log::warn!(
                                "config.worker_threads = {} exceeds the 64-thread ceiling — clamped to {}",
                                raw_threads,
                                parsed.worker_threads
                            );
                        }
                        parsed
                    }
                    Err(parse_err) => {
                        // Corrupt config: keep a recoverable backup before
                        // defaults take over, so the next save() can never
                        // destroy the user's restorable configuration.
                        let ts = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        let backup = path.with_extension(format!("json.backup-{}", ts));
                        match fs::copy(&path, &backup) {
                            Ok(_) => log::warn!(
                                "Corrupt config at {} ({}); backed up to {} and loaded defaults",
                                path.display(),
                                parse_err,
                                backup.display()
                            ),
                            Err(copy_err) => log::warn!(
                                "Corrupt config at {} ({}); backup to {} failed ({}), loaded defaults",
                                path.display(),
                                parse_err,
                                backup.display(),
                                copy_err
                            ),
                        }
                        Self::default()
                    }
                },
                Err(read_err) => {
                    log::warn!(
                        "Config file unreadable at {} ({}); loaded defaults",
                        path.display(),
                        read_err
                    );
                    Self::default()
                }
            }
        } else {
            Self::default()
        };

        if let Ok(mut guard) = cache.lock() {
            *guard = Some((path, stamp, cfg.clone()));
        }
        cfg
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)?;
        fs::write(&path, data)?;
        // Keep the memoised copy in sync so the very next `load()` is free.
        let cache = CONFIG_CACHE.get_or_init(|| std::sync::Mutex::new(None));
        if let Ok(mut guard) = cache.lock() {
            let stamp = Self::file_stamp(&path);
            *guard = Some((path, stamp, self.clone()));
        }
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
