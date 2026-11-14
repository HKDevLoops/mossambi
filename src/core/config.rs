//! Persistent Configuration System for Mossambi

use crate::core::pantone::{PantoneTheme, THEMES};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    // 11 Named Dark Themes (10 canonical + Rangalipi, the Mossambi signature)
    Gruvbox,
    TokyoNight,
    CatppuccinMocha,
    ModusVivendi,
    Nord,
    Dracula,
    Kanagawa,
    OneDark,
    Atom,
    DarkPlus,
    Synthwave,
    // Rangalipi: the default theme. Every legacy dark config id migrates
    // onto it so old config.json files keep loading.
    #[default]
    #[serde(alias = "dark")]
    #[serde(alias = "system")]
    #[serde(alias = "liquid_glass")]
    #[serde(alias = "liquidglass")]
    #[serde(alias = "frosted_glass")]
    #[serde(alias = "frostedglass")]
    #[serde(alias = "dark_anthracite")]
    #[serde(alias = "anthracite_obsidian")]
    #[serde(alias = "dark_forest")]
    #[serde(alias = "deep_forest_emerald")]
    #[serde(alias = "dark_eclipse")]
    #[serde(alias = "midnight_eclipse")]
    #[serde(alias = "dark_pewter")]
    #[serde(alias = "pewter_slate")]
    #[serde(alias = "dark_tangerine")]
    #[serde(alias = "tangerine_nightfall")]
    #[serde(alias = "dark_caviar")]
    #[serde(alias = "citrus_caviar")]
    #[serde(alias = "dark_indigo")]
    #[serde(alias = "abyssal_indigo")]
    #[serde(alias = "dark_carbon")]
    #[serde(alias = "carbon_noir")]
    #[serde(alias = "dark_merlot")]
    #[serde(alias = "vintage_merlot")]
    #[serde(alias = "dark_gunmetal")]
    #[serde(alias = "gunmetal_steel")]
    Rangalipi,
    // Mossambi Punch: the true sweet-lime fruit light theme. Every legacy
    // light config id migrates onto it.
    #[serde(alias = "light")]
    #[serde(alias = "light_brilliant")]
    #[serde(alias = "brilliant_lime")]
    #[serde(alias = "light_zest")]
    #[serde(alias = "sweet_zest_sunlight")]
    #[serde(alias = "light_blanc")]
    #[serde(alias = "blanc_de_blanc")]
    #[serde(alias = "light_glacier")]
    #[serde(alias = "glacier_aqua")]
    #[serde(alias = "light_tangerine")]
    #[serde(alias = "tangerine_sorbet")]
    #[serde(alias = "light_albedo")]
    #[serde(alias = "albedo_pure_ivory")]
    #[serde(alias = "light_quartz")]
    #[serde(alias = "cool_quartz")]
    #[serde(alias = "light_pistachio")]
    #[serde(alias = "pistachio_cream")]
    #[serde(alias = "light_amber")]
    #[serde(alias = "vanilla_amber")]
    #[serde(alias = "light_silver")]
    #[serde(alias = "silver_pearl")]
    #[serde(alias = "light_aqua")]
    #[serde(alias = "lightaqua")]
    MossambiPunch,
}

impl ThemeMode {
    pub fn to_pantone_theme(&self) -> &'static PantoneTheme {
        match self {
            Self::Gruvbox => &THEMES[0],
            Self::TokyoNight => &THEMES[1],
            Self::CatppuccinMocha => &THEMES[2],
            Self::ModusVivendi => &THEMES[3],
            Self::Nord => &THEMES[4],
            Self::Dracula => &THEMES[5],
            Self::Kanagawa => &THEMES[6],
            Self::OneDark => &THEMES[7],
            Self::Atom => &THEMES[8],
            Self::DarkPlus => &THEMES[9],
            Self::Synthwave => &THEMES[10],
            Self::Rangalipi => &THEMES[11],
            Self::MossambiPunch => &THEMES[12],
        }
    }

    pub fn name(&self) -> &'static str {
        self.to_pantone_theme().name
    }

    pub fn is_dark(&self) -> bool {
        self.to_pantone_theme().is_dark
    }

    pub fn all() -> &'static [ThemeMode] {
        static ALL: [ThemeMode; 13] = [
            ThemeMode::Gruvbox,
            ThemeMode::TokyoNight,
            ThemeMode::CatppuccinMocha,
            ThemeMode::ModusVivendi,
            ThemeMode::Nord,
            ThemeMode::Dracula,
            ThemeMode::Kanagawa,
            ThemeMode::OneDark,
            ThemeMode::Atom,
            ThemeMode::DarkPlus,
            ThemeMode::Synthwave,
            ThemeMode::Rangalipi,
            ThemeMode::MossambiPunch,
        ];
        &ALL
    }

    pub fn dark_themes() -> &'static [ThemeMode] {
        &Self::all()[0..12]
    }

    pub fn light_themes() -> &'static [ThemeMode] {
        &Self::all()[12..13]
    }

    pub fn next(&self) -> Self {
        let all = Self::all();
        let current_pos = all.iter().position(|m| m == self).unwrap_or(0);
        all[(current_pos + 1) % all.len()]
    }

    pub fn toggle_dark_light(&self) -> Self {
        if self.is_dark() {
            Self::MossambiPunch
        } else {
            Self::Rangalipi
        }
    }

    pub fn from_id_or_name(query: &str) -> Option<Self> {
        let q = query.to_lowercase().replace([' ', '-', '_'], "");
        // Legacy glass strings and generic dark resolve to the dark default
        if matches!(
            q.as_str(),
            "liquidglass" | "frostedglass" | "frosted" | "sonoma" | "dark" | "system"
        ) {
            return Some(Self::Rangalipi);
        }
        if matches!(q.as_str(), "lightaqua" | "aqua" | "light") {
            return Some(Self::MossambiPunch);
        }
        // Retired 20-theme ids keep resolving, mapped by darkness
        if matches!(
            q.as_str(),
            "darkanthracite"
                | "anthraciteobsidian"
                | "darkforest"
                | "deepforestemerald"
                | "darkeclipse"
                | "midnighteclipse"
                | "darkpewter"
                | "pewterslate"
                | "darktangerine"
                | "tangerinenightfall"
                | "darkcaviar"
                | "citruscaviar"
                | "darkindigo"
                | "abyssalindigo"
                | "darkcarbon"
                | "carbonnoir"
                | "darkmerlot"
                | "vintagemerlot"
                | "darkgunmetal"
                | "gunmetalsteel"
        ) {
            return Some(Self::Rangalipi);
        }
        if matches!(
            q.as_str(),
            "lightbrilliant"
                | "brilliantlime"
                | "lightzest"
                | "sweetzestsunlight"
                | "lightblanc"
                | "blancdeblanc"
                | "lightglacier"
                | "glacieraqua"
                | "lighttangerine"
                | "tangerinesorbet"
                | "lightalbedo"
                | "albedopureivory"
                | "lightquartz"
                | "coolquartz"
                | "lightpistachio"
                | "pistachiocream"
                | "lightamber"
                | "vanillaamber"
                | "lightsilver"
                | "silverpearl"
        ) {
            return Some(Self::MossambiPunch);
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

/// Light/dark appearance applied on top of the chosen theme family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Appearance {
    /// Use the theme's own flavour (Gruvbox dark, Mossambi Punch light...).
    #[default]
    ThemeDefault,
    Light,
    Dark,
    /// Follow the Windows "app mode" setting.
    System,
}

impl Appearance {
    pub fn all() -> [Appearance; 4] {
        [
            Appearance::ThemeDefault,
            Appearance::Light,
            Appearance::Dark,
            Appearance::System,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Appearance::ThemeDefault => "Theme default",
            Appearance::Light => "Light",
            Appearance::Dark => "Dark",
            Appearance::System => "Match Windows",
        }
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

/// Look presets bundling operating-system styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LookPreset {
    #[serde(alias = "mac", alias = "macos")]
    Finder,
    #[serde(alias = "windows", alias = "win", alias = "explorer")]
    WindowsExplorer,
    #[serde(alias = "gnome")]
    Nautilus,
    #[serde(alias = "xfce")]
    Thunar,
}

impl LookPreset {
    pub fn all() -> [LookPreset; 4] {
        [
            Self::Finder,
            Self::WindowsExplorer,
            Self::Nautilus,
            Self::Thunar,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Finder => "Finder",
            Self::WindowsExplorer => "Windows Explorer",
            Self::Nautilus => "Nautilus/GNOME",
            Self::Thunar => "Thunar",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Finder => "macOS Finder: Miller columns, 28px density, Sonoma icons, rounded corners, Rangalipi theme",
            Self::WindowsExplorer => "Windows 11: Details view, 32px density, Win11 icons, subtle corners, Dark+ theme",
            Self::Nautilus => "Linux GNOME: Grid view, 36px density, Adwaita icons, spacious corners, Tokyo Night theme",
            Self::Thunar => "Linux XFCE: Compact list, 24px density, classic icons, sharp corners, Kanagawa theme",
        }
    }
}

/// Visual row density across explorer lists and tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RowDensity {
    Compact, // 24px (Thunar)
    #[default]
    Default, // 28px (Finder)
    Standard, // 32px (Windows Explorer)
    Spacious, // 36px (Nautilus)
}

impl RowDensity {
    pub fn all() -> [RowDensity; 4] {
        [Self::Compact, Self::Default, Self::Standard, Self::Spacious]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Compact => "Compact (24px)",
            Self::Default => "Default (28px)",
            Self::Standard => "Standard (32px)",
            Self::Spacious => "Spacious (36px)",
        }
    }

    pub fn details_row_height(&self) -> f32 {
        match self {
            Self::Compact => 24.0,
            Self::Default => 28.0,
            Self::Standard => 32.0,
            Self::Spacious => 36.0,
        }
    }

    pub fn sidebar_row_height(&self) -> f32 {
        match self {
            Self::Compact => 24.0,
            Self::Default => 30.0,
            Self::Standard => 32.0,
            Self::Spacious => 36.0,
        }
    }
}

/// Visual style applied to the sidebar panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SidebarStyle {
    #[default]
    Finder,
    Windows,
    Nautilus,
    Thunar,
}

impl SidebarStyle {
    pub fn all() -> [SidebarStyle; 4] {
        [Self::Finder, Self::Windows, Self::Nautilus, Self::Thunar]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Finder => "macOS Finder",
            Self::Windows => "Windows Explorer",
            Self::Nautilus => "Linux (Nautilus)",
            Self::Thunar => "Linux (Thunar)",
        }
    }

    pub fn sidebar_row_height(&self) -> f32 {
        match self {
            Self::Thunar => 24.0,
            Self::Finder => 30.0,
            Self::Windows => 32.0,
            Self::Nautilus => 34.0,
        }
    }
}

/// Corner radius styling presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CornerStyle {
    Sharp,  // 2px (Thunar)
    Subtle, // 4px (Windows Explorer)
    #[default]
    Rounded, // 8px (Finder)
    Spacious, // 12px (Nautilus)
}

impl CornerStyle {
    pub fn all() -> [CornerStyle; 4] {
        [Self::Sharp, Self::Subtle, Self::Rounded, Self::Spacious]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Sharp => "Sharp (2px)",
            Self::Subtle => "Subtle (4px)",
            Self::Rounded => "Rounded (8px)",
            Self::Spacious => "Spacious (12px)",
        }
    }

    pub fn radius_sm(&self) -> f32 {
        match self {
            Self::Sharp => 2.0,
            Self::Subtle => 4.0,
            Self::Rounded => 6.0,
            Self::Spacious => 8.0,
        }
    }

    pub fn radius_card(&self) -> f32 {
        match self {
            Self::Sharp => 3.0,
            Self::Subtle => 6.0,
            Self::Rounded => 10.0,
            Self::Spacious => 14.0,
        }
    }
}

/// Explorer chrome style: macOS Finder vs Windows Explorer vs Linux desktop layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExplorerStyle {
    /// Authentic macOS Finder layout: traffic lights on left, unified single toolbar,
    /// dynamic path ribbon at the bottom with item status.
    #[default]
    #[serde(alias = "macos", alias = "mac", alias = "finder")]
    MacOS,
    /// Windows Explorer layout: top navigation buttons with breadcrumb address bar
    /// and quick edit mode, status bar at bottom.
    #[serde(alias = "windows", alias = "win")]
    Windows,
    /// Linux GNOME Nautilus layout: top pill breadcrumb path bar with centered controls.
    #[serde(alias = "nautilus", alias = "gnome")]
    Nautilus,
    /// Linux XFCE Thunar layout: classic compact path bar and tool buttons.
    #[serde(alias = "thunar", alias = "xfce")]
    Thunar,
}

impl ExplorerStyle {
    pub fn all() -> &'static [ExplorerStyle] {
        &[
            ExplorerStyle::MacOS,
            ExplorerStyle::Windows,
            ExplorerStyle::Nautilus,
            ExplorerStyle::Thunar,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::MacOS => "macOS Finder",
            Self::Windows => "Windows Explorer",
            Self::Nautilus => "Linux (Nautilus)",
            Self::Thunar => "Linux (Thunar)",
        }
    }
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
    #[serde(alias = "windows", alias = "win")]
    Windows,
    /// macOS Finder layout and wording.
    #[serde(alias = "finder", alias = "macos", alias = "mac")]
    Finder,
    /// Linux / Nautilus layout and wording.
    #[serde(alias = "linux", alias = "nautilus")]
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

/// The execution kind of a user-defined context-menu entry (Phase 4
/// customization, configured in Preferences > Context Menu).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CustomMenuKind {
    /// Run `value` as a full command line through `cmd.exe /C`.
    #[default]
    Command,
    /// Launch an executable directly (CreateProcessW) with optional args.
    App,
    /// Open a script file with the shell "open" verb.
    Script,
    /// Open a URL or document with the shell "open" verb.
    Link,
}

impl CustomMenuKind {
    pub fn all() -> &'static [CustomMenuKind] {
        &[
            CustomMenuKind::Command,
            CustomMenuKind::App,
            CustomMenuKind::Script,
            CustomMenuKind::Link,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Command => "Command (cmd.exe)",
            Self::App => "App (executable)",
            Self::Script => "Script",
            Self::Link => "Link (URL)",
        }
    }
}

/// Which menus a user-defined context-menu entry appears in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CustomMenuScope {
    /// Item menus and the background canvas menu.
    #[default]
    All,
    /// Item menus of files only.
    Files,
    /// Item menus of folders only.
    Folders,
    /// The background (empty-canvas) menu only.
    Background,
}

impl CustomMenuScope {
    pub fn all() -> &'static [CustomMenuScope] {
        &[
            CustomMenuScope::All,
            CustomMenuScope::Files,
            CustomMenuScope::Folders,
            CustomMenuScope::Background,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::All => "All menus",
            Self::Files => "Files only",
            Self::Folders => "Folders only",
            Self::Background => "Background only",
        }
    }
}

/// One user-defined context-menu entry (Phase 4 customization). `%p` and
/// `%1` inside `value`/`args` are substituted with the quoted target path
/// (empty on the background menu).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomMenuItem {
    /// Menu label as shown to the user.
    pub label: String,
    /// What kind of launcher this entry is.
    pub kind: CustomMenuKind,
    /// Command line / executable path / script path / URL.
    pub value: String,
    /// Optional argument template appended after `value`.
    #[serde(default)]
    pub args: String,
    /// Which menus the entry appears in.
    #[serde(default)]
    pub show_in: CustomMenuScope,
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
    #[serde(default)]
    pub appearance: Appearance,
    /// Interface size multiplier (1.0 = native). Larger values render text
    /// and icons with more pixels, which helps on low-density monitors.
    #[serde(default = "default_ui_scale")]
    pub ui_scale: f32,
    /// "Convert with Mossambi" entry in the Windows Explorer file menu.
    #[serde(default)]
    pub shell_context_menu: bool,
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
    pub grid_icon_size: u32, // 64..300
    #[serde(default)]
    pub checkbox_mode: CheckboxMode,
    #[serde(default)]
    pub menu_style: MenuStyle,
    /// User-defined context-menu entries (Phase 4 customization).
    #[serde(default)]
    pub context_menu_items: Vec<CustomMenuItem>,
    /// Details-view visible columns (DetailColumn ids, persisted
    /// order). Name (id 0) is locked first. Missing in old configs ->
    /// Finder defaults.
    #[serde(default = "default_explorer_columns")]
    pub explorer_columns: Vec<u8>,
    /// Persisted per-column widths keyed by DetailColumn id.
    #[serde(default)]
    pub explorer_column_widths: std::collections::BTreeMap<u8, u32>,
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
    /// Sidebar zone collapse states (Zen/Arc mechanics). Persisted so the
    /// sidebar keeps its shape across restarts; all zones default expanded.
    #[serde(default = "default_true")]
    pub sidebar_favorites_expanded: bool,
    #[serde(default = "default_true")]
    pub sidebar_locations_expanded: bool,
    #[serde(default = "default_true")]
    pub sidebar_tags_expanded: bool,
    #[serde(default = "default_true")]
    pub sidebar_tools_expanded: bool,
    /// Look preset: Finder, Windows Explorer, Nautilus/GNOME, Thunar
    #[serde(default)]
    pub look_preset: Option<LookPreset>,
    /// Sidebar visual style: Finder, Windows, Nautilus, Thunar
    #[serde(default)]
    pub sidebar_style: SidebarStyle,
    /// Row density in file lists and tables
    #[serde(default)]
    pub row_density: RowDensity,
    /// Corner radius styling (Sharp, Subtle, Rounded, Spacious)
    #[serde(default)]
    pub corner_style: CornerStyle,
    /// Explorer style: macOS Finder vs Windows Explorer.
    #[serde(default)]
    pub explorer_style: ExplorerStyle,
    /// Keybinding profile: 0 = macOS Finder, 1 = Windows Explorer
    #[serde(default = "default_keybinding_profile")]
    pub keybinding_profile: u8,
    /// Persistent default: image conversion quality percent (10-100)
    #[serde(default = "default_conv_image_quality")]
    pub conv_image_quality: u8,
    /// Persistent default: encode images losslessly
    #[serde(default)]
    pub conv_image_lossless: bool,
    /// Persistent default: image DPI (72-600)
    #[serde(default = "default_conv_image_dpi")]
    pub conv_image_dpi: u32,
    /// Persistent default: audio bitrate string (e.g. "256k"; "" = default)
    #[serde(default = "default_conv_audio_bitrate")]
    pub conv_audio_bitrate: String,
    /// Persistent default: audio strict-lossless mode
    #[serde(default = "default_true")]
    pub conv_audio_strict_lossless: bool,
    /// Persistent default: video CRF quality (0-51)
    #[serde(default = "default_conv_video_crf")]
    pub conv_video_crf: u8,
    /// Persistent default: video encoder preset label
    #[serde(default = "default_conv_video_preset")]
    pub conv_video_preset: String,
    /// Persistent default: use hardware acceleration for video
    #[serde(default = "default_true")]
    pub conv_video_hw_accel: bool,
    /// Persistent default: archive compression level label
    #[serde(default = "default_conv_archive_compression")]
    pub conv_archive_compression: String,
    /// Persistent default: exam-envelope constraints enabled
    #[serde(default)]
    pub conv_exam_mode_enabled: bool,
    /// Rebindable app shortcuts: action id -> display binding ("Ctrl+P").
    /// Missing or blank ids fall back to DEFAULT_SHORTCUTS.
    #[serde(default)]
    pub shortcuts: std::collections::BTreeMap<String, String>,
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

fn default_conv_image_quality() -> u8 {
    95
}

fn default_conv_image_dpi() -> u32 {
    150
}

fn default_conv_audio_bitrate() -> String {
    "256k".into()
}

fn default_conv_video_crf() -> u8 {
    16
}

fn default_conv_video_preset() -> String {
    "High Quality".into()
}

fn default_conv_archive_compression() -> String {
    "Ultra".into()
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

fn default_explorer_columns() -> Vec<u8> {
    // Finder defaults: Name(0), DateModified(1), Size(2), Kind(3).
    vec![0, 1, 2, 3]
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

fn default_ui_scale() -> f32 {
    1.0
}

fn default_window_opacity() -> f32 {
    0.88
}

impl Default for MossambiConfig {
    fn default() -> Self {
        Self {
            theme_mode: ThemeMode::Rangalipi,
            glass_effect: GlassEffect::FrostedGlass,
            appearance: Appearance::default(),
            shell_context_menu: false,
            ui_scale: default_ui_scale(),
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
            context_menu_items: Vec::new(),
            explorer_columns: default_explorer_columns(),
            explorer_column_widths: std::collections::BTreeMap::new(),
            animations: AnimationSettings::default(),
            explorer_zoom: default_explorer_zoom(),
            background_work: default_background_work(),
            worker_threads: default_worker_threads(),
            max_fps: default_max_fps(),
            drop_target_default: default_drop_target(),
            show_tags_section: false,
            show_custom_pinned: false,
            pinned_custom: Vec::new(),
            sidebar_favorites_expanded: true,
            sidebar_locations_expanded: true,
            sidebar_tags_expanded: true,
            sidebar_tools_expanded: true,
            look_preset: None,
            sidebar_style: SidebarStyle::default(),
            row_density: RowDensity::default(),
            corner_style: CornerStyle::default(),
            explorer_style: ExplorerStyle::default(),
            keybinding_profile: default_keybinding_profile(),
            conv_image_quality: default_conv_image_quality(),
            conv_image_lossless: false,
            conv_image_dpi: default_conv_image_dpi(),
            conv_audio_bitrate: default_conv_audio_bitrate(),
            conv_audio_strict_lossless: true,
            conv_video_crf: default_conv_video_crf(),
            conv_video_preset: default_conv_video_preset(),
            conv_video_hw_accel: true,
            conv_archive_compression: default_conv_archive_compression(),
            conv_exam_mode_enabled: false,
            shortcuts: DEFAULT_SHORTCUTS
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
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

/// Rebindable app shortcut action ids and their default bindings. The
/// Shortcuts & Gestures page rebinds these; the studio dispatch reads them.
pub const DEFAULT_SHORTCUTS: &[(&str, &str)] = &[
    ("palette", "Ctrl+P"),
    ("new_tab", "Ctrl+T"),
    ("close_tab", "Ctrl+W"),
];

impl MossambiConfig {
    /// Applies an operating-system look preset while keeping individual settings overridable.
    pub fn apply_look_preset(&mut self, preset: LookPreset) {
        self.look_preset = Some(preset);
        match preset {
            LookPreset::Finder => {
                self.sidebar_style = SidebarStyle::Finder;
                self.explorer_style = ExplorerStyle::MacOS;
                self.default_explorer_layout = 3; // Miller columns
                self.row_density = RowDensity::Default; // 28px
                self.icon_pack = "macos".to_string();
                self.corner_style = CornerStyle::Rounded;
                self.theme_mode = ThemeMode::Rangalipi;
                self.menu_style = MenuStyle::Finder;
            }
            LookPreset::WindowsExplorer => {
                self.sidebar_style = SidebarStyle::Windows;
                self.explorer_style = ExplorerStyle::Windows;
                self.default_explorer_layout = 0; // Details
                self.row_density = RowDensity::Standard; // 32px
                self.icon_pack = "win11".to_string();
                self.corner_style = CornerStyle::Subtle;
                self.theme_mode = ThemeMode::DarkPlus;
                self.menu_style = MenuStyle::Windows;
            }
            LookPreset::Nautilus => {
                self.sidebar_style = SidebarStyle::Nautilus;
                self.explorer_style = ExplorerStyle::Nautilus;
                self.default_explorer_layout = 2; // Grid
                self.row_density = RowDensity::Spacious; // 36px
                self.icon_pack = "adwaita".to_string();
                self.corner_style = CornerStyle::Spacious;
                self.theme_mode = ThemeMode::TokyoNight;
                self.menu_style = MenuStyle::Linux;
            }
            LookPreset::Thunar => {
                self.sidebar_style = SidebarStyle::Thunar;
                self.explorer_style = ExplorerStyle::Thunar;
                self.default_explorer_layout = 0; // Details
                self.row_density = RowDensity::Compact; // 24px
                self.icon_pack = "cinnamon".to_string();
                self.corner_style = CornerStyle::Sharp;
                self.theme_mode = ThemeMode::Kanagawa;
                self.menu_style = MenuStyle::Linux;
            }
        }
    }

    /// Returns the display binding for a rebindable shortcut action id,
    /// falling back to DEFAULT_SHORTCUTS when unset or blank.
    pub fn shortcut_for(&self, id: &str) -> String {
        match self.shortcuts.get(id) {
            Some(b) if !b.trim().is_empty() => b.clone(),
            _ => match DEFAULT_SHORTCUTS.iter().find(|(k, _)| *k == id) {
                Some((_, v)) => v.to_string(),
                None => String::new(),
            },
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

    /// Adds or removes the per-user "Convert with Mossambi" verb for all
    /// files in Windows Explorer (HKCU only, no admin rights needed).
    #[cfg(target_os = "windows")]
    pub fn set_shell_context_menu(enabled: bool) -> anyhow::Result<()> {
        use std::process::Command;
        let key = "HKCU\\Software\\Classes\\*\\shell\\Mossambi";
        if enabled {
            let exe = std::env::current_exe()?;
            let exe = exe.to_string_lossy();
            let cmd = format!("\"{exe}\" widget \"%1\"");
            Command::new("reg")
                .args(["add", key, "/ve", "/d", "Convert with Mossambi", "/f"])
                .output()?;
            Command::new("reg")
                .args(["add", key, "/v", "Icon", "/d", &exe, "/f"])
                .output()?;
            Command::new("reg")
                .args(["add", &format!("{key}\\command"), "/ve", "/d", &cmd, "/f"])
                .output()?;
        } else {
            let _ = Command::new("reg").args(["delete", key, "/f"]).output()?;
        }
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    pub fn set_shell_context_menu(_enabled: bool) -> anyhow::Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_look_presets_apply_expected_values() {
        let mut cfg = MossambiConfig::default();

        cfg.apply_look_preset(LookPreset::Finder);
        assert_eq!(cfg.sidebar_style, SidebarStyle::Finder);
        assert_eq!(cfg.explorer_style, ExplorerStyle::MacOS);
        assert_eq!(cfg.default_explorer_layout, 3);
        assert_eq!(cfg.row_density, RowDensity::Default);
        assert_eq!(cfg.icon_pack, "macos");
        assert_eq!(cfg.corner_style, CornerStyle::Rounded);
        assert_eq!(cfg.theme_mode, ThemeMode::Rangalipi);

        cfg.apply_look_preset(LookPreset::WindowsExplorer);
        assert_eq!(cfg.sidebar_style, SidebarStyle::Windows);
        assert_eq!(cfg.explorer_style, ExplorerStyle::Windows);
        assert_eq!(cfg.default_explorer_layout, 0);
        assert_eq!(cfg.row_density, RowDensity::Standard);
        assert_eq!(cfg.icon_pack, "win11");
        assert_eq!(cfg.corner_style, CornerStyle::Subtle);
        assert_eq!(cfg.theme_mode, ThemeMode::DarkPlus);

        cfg.apply_look_preset(LookPreset::Nautilus);
        assert_eq!(cfg.sidebar_style, SidebarStyle::Nautilus);
        assert_eq!(cfg.explorer_style, ExplorerStyle::Nautilus);
        assert_eq!(cfg.default_explorer_layout, 2);
        assert_eq!(cfg.row_density, RowDensity::Spacious);
        assert_eq!(cfg.icon_pack, "adwaita");
        assert_eq!(cfg.corner_style, CornerStyle::Spacious);
        assert_eq!(cfg.theme_mode, ThemeMode::TokyoNight);

        cfg.apply_look_preset(LookPreset::Thunar);
        assert_eq!(cfg.sidebar_style, SidebarStyle::Thunar);
        assert_eq!(cfg.explorer_style, ExplorerStyle::Thunar);
        assert_eq!(cfg.default_explorer_layout, 0);
        assert_eq!(cfg.row_density, RowDensity::Compact);
        assert_eq!(cfg.icon_pack, "cinnamon");
        assert_eq!(cfg.corner_style, CornerStyle::Sharp);
        assert_eq!(cfg.theme_mode, ThemeMode::Kanagawa);
    }

    #[test]
    fn test_row_density_and_corner_styles() {
        assert_eq!(RowDensity::Compact.details_row_height(), 24.0);
        assert_eq!(RowDensity::Default.details_row_height(), 28.0);
        assert_eq!(RowDensity::Standard.details_row_height(), 32.0);
        assert_eq!(RowDensity::Spacious.details_row_height(), 36.0);

        assert_eq!(SidebarStyle::Thunar.sidebar_row_height(), 24.0);
        assert_eq!(SidebarStyle::Finder.sidebar_row_height(), 30.0);
        assert_eq!(SidebarStyle::Windows.sidebar_row_height(), 32.0);
        assert_eq!(SidebarStyle::Nautilus.sidebar_row_height(), 34.0);

        assert_eq!(CornerStyle::Sharp.radius_sm(), 2.0);
        assert_eq!(CornerStyle::Subtle.radius_sm(), 4.0);
        assert_eq!(CornerStyle::Rounded.radius_sm(), 6.0);
        assert_eq!(CornerStyle::Spacious.radius_sm(), 8.0);
    }
}
