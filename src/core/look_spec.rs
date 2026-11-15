//! Layout and Chrome Specifications for operating-system Look Presets.
//!
//! Provides a declarative `ChromeSpec` for orchestrating window controls, title bar,
//! tab positioning, menubars, toolbars, address presentation, search placement,
//! sidebars, details panes, and status bar presence across Finder, Windows Explorer,
//! Nautilus, and Thunar designs.

use crate::core::config::LookPreset;
use crate::core::icon_packs::IconGlyph;
use serde::{Deserialize, Serialize};

/// Window control buttons layout style (traffic lights vs caption buttons).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WindowControlsPosition {
    /// macOS Finder traffic lights on the LEFT of the header.
    LeftTraffic,
    /// Windows / GNOME caption buttons on the RIGHT of the header (Min, Max, Close).
    RightCaption,
}

/// Title bar layout style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TitleBarStyle {
    /// Modern unified title bar merging tabs, navigation, and window controls.
    Unified,
    /// Classic separate title bar above toolbar / menubar.
    Separate,
}

/// Position of the tab strip relative to the window chrome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TabPosition {
    /// Tabs at the very top of the window (Windows 11 Explorer / Google Chrome).
    Top,
    /// Tabs integrated inside the unified header row (macOS Finder).
    InHeader,
    /// No tabs strip displayed (Thunar / minimal single-view).
    None,
}

/// Address and navigation trail presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AddressBarStyle {
    /// Interactive breadcrumb trail with drop-down menus (Windows 11 Explorer).
    Breadcrumb,
    /// Clickable path segment buttons (Thunar / Linux XFCE).
    PathButtons,
    /// Centered folder icon + title pill in header; deep path bar at bottom (Finder).
    PillPathBar,
    /// Plain text path edit box.
    TextEntry,
}

/// Placement and interaction model of search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SearchPosition {
    /// Pinned on the far right of the toolbar (Finder / Windows 11).
    Right,
    /// Search icon button toggles an expandable search popover (Nautilus / GNOME).
    Toggle,
    /// Embedded inline in the address row.
    InAddressRow,
}

/// Side details / preview pane position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DetailsPanePosition {
    /// Details pane docked on the right side of the file area.
    Right,
    /// No details pane (or popup / sheet only).
    None,
}

/// Individual toolbar action button or widget item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolbarItem {
    Back,
    Forward,
    Up,
    Refresh,
    NewFolder,
    ViewSwitcher,
    Search,
    Settings,
    NewWindow,
    // Windows 11 Explorer Command Bar items
    Cut,
    Copy,
    Paste,
    Rename,
    Share,
    Delete,
    Sort,
    // Linux Nautilus / Thunar items
    Hamburger,
    Group,
    Tags,
}

impl ToolbarItem {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Back => "Back",
            Self::Forward => "Forward",
            Self::Up => "Up",
            Self::Refresh => "Refresh",
            Self::NewFolder => "New Folder",
            Self::ViewSwitcher => "View",
            Self::Search => "Search",
            Self::Settings => "Preferences",
            Self::NewWindow => "New Window",
            Self::Cut => "Cut",
            Self::Copy => "Copy",
            Self::Paste => "Paste",
            Self::Rename => "Rename",
            Self::Share => "Share",
            Self::Delete => "Delete",
            Self::Sort => "Sort",
            Self::Hamburger => "Menu",
            Self::Group => "Group",
            Self::Tags => "Tags",
        }
    }

    pub fn icon_glyph(&self) -> IconGlyph {
        match self {
            Self::Back => IconGlyph::ActionBack,
            Self::Forward => IconGlyph::ActionForward,
            Self::Up => IconGlyph::ActionUp,
            Self::Refresh => IconGlyph::ActionRefresh,
            Self::NewFolder => IconGlyph::ActionNewFolder,
            Self::ViewSwitcher => IconGlyph::ActionView,
            Self::Search => IconGlyph::Search,
            Self::Settings => IconGlyph::Gear,
            Self::NewWindow => IconGlyph::ActionNewFolder,
            Self::Cut => IconGlyph::ActionCut,
            Self::Copy => IconGlyph::ActionCopy,
            Self::Paste => IconGlyph::ActionPaste,
            Self::Rename => IconGlyph::ActionRename,
            Self::Share => IconGlyph::ActionShortcut,
            Self::Delete => IconGlyph::ActionDelete,
            Self::Sort => IconGlyph::ActionSort,
            Self::Hamburger => IconGlyph::ActionMore,
            Self::Group => IconGlyph::ActionFilter,
            Self::Tags => IconGlyph::ActionPin,
        }
    }

    pub fn shortcut_hint(&self) -> &'static str {
        match self {
            Self::Back => "Alt+Left",
            Self::Forward => "Alt+Right",
            Self::Up => "Alt+Up / Backspace",
            Self::Refresh => "F5",
            Self::NewFolder => "Ctrl+Shift+N",
            Self::ViewSwitcher => "Ctrl+1..4",
            Self::Search => "/",
            Self::Settings => "Cmd+,",
            Self::NewWindow => "Ctrl+N",
            Self::Cut => "Ctrl+X",
            Self::Copy => "Ctrl+C",
            Self::Paste => "Ctrl+V",
            Self::Rename => "F2",
            Self::Share => "",
            Self::Delete => "Delete",
            Self::Sort => "",
            Self::Hamburger => "",
            Self::Group => "",
            Self::Tags => "",
        }
    }
}

/// Header style for sidebar sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SidebarHeaderStyle {
    /// Collapsible chevron zone header e.g. "Favorites v".
    CollapsibleZone,
    /// Clean muted uppercase section title e.g. "FAVORITES".
    SectionTitle,
    /// No section header title.
    None,
}

/// Sidebar layout specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SidebarSpec {
    pub width: f32,
    pub show_favorites: bool,
    pub show_locations: bool,
    pub show_tags: bool,
    pub show_tools: bool,
    pub header_style: SidebarHeaderStyle,
}

impl Default for SidebarSpec {
    fn default() -> Self {
        Self {
            width: 175.0,
            show_favorites: true,
            show_locations: true,
            show_tags: true,
            show_tools: true,
            header_style: SidebarHeaderStyle::CollapsibleZone,
        }
    }
}

/// Spacing and metric tokens for chrome layout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpacingTokens {
    pub toolbar_item_spacing: f32,
    pub toolbar_padding_y: f32,
    pub sidebar_item_height: f32,
}

impl Default for SpacingTokens {
    fn default() -> Self {
        Self {
            toolbar_item_spacing: 6.0,
            toolbar_padding_y: 4.0,
            sidebar_item_height: 28.0,
        }
    }
}

/// Complete declarative specification of the window chrome and explorer architecture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChromeSpec {
    pub window_controls: WindowControlsPosition,
    pub title_bar: TitleBarStyle,
    pub tab_position: TabPosition,
    pub menubar: bool,
    pub toolbar: Vec<ToolbarItem>,
    pub address: AddressBarStyle,
    pub search: SearchPosition,
    pub sidebar: SidebarSpec,
    pub details_pane: DetailsPanePosition,
    pub status_bar: bool,
    pub path_bar_bottom: bool,
    pub spacing: SpacingTokens,
}

impl Default for ChromeSpec {
    fn default() -> Self {
        Self::for_preset(LookPreset::Finder)
    }
}

impl ChromeSpec {
    /// Derives the full declarative `ChromeSpec` for an operating-system look preset.
    pub fn for_preset(preset: LookPreset) -> Self {
        match preset {
            LookPreset::Finder => Self {
                window_controls: WindowControlsPosition::LeftTraffic,
                title_bar: TitleBarStyle::Unified,
                tab_position: TabPosition::InHeader,
                menubar: false,
                toolbar: vec![
                    ToolbarItem::Back,
                    ToolbarItem::Forward,
                    ToolbarItem::ViewSwitcher,
                    ToolbarItem::Group,
                    ToolbarItem::Share,
                    ToolbarItem::Tags,
                    ToolbarItem::Search,
                    ToolbarItem::Settings,
                ],
                address: AddressBarStyle::PillPathBar,
                search: SearchPosition::Right,
                sidebar: SidebarSpec {
                    width: 175.0,
                    show_favorites: true,
                    show_locations: true,
                    show_tags: true,
                    show_tools: true,
                    header_style: SidebarHeaderStyle::CollapsibleZone,
                },
                details_pane: DetailsPanePosition::None,
                status_bar: true,
                path_bar_bottom: true,
                spacing: SpacingTokens {
                    toolbar_item_spacing: 6.0,
                    toolbar_padding_y: 4.0,
                    sidebar_item_height: 28.0,
                },
            },
            LookPreset::WindowsExplorer => Self {
                window_controls: WindowControlsPosition::RightCaption,
                title_bar: TitleBarStyle::Separate,
                tab_position: TabPosition::Top,
                menubar: false,
                toolbar: vec![
                    ToolbarItem::NewFolder,
                    ToolbarItem::Cut,
                    ToolbarItem::Copy,
                    ToolbarItem::Paste,
                    ToolbarItem::Rename,
                    ToolbarItem::Share,
                    ToolbarItem::Delete,
                    ToolbarItem::Sort,
                    ToolbarItem::ViewSwitcher,
                ],
                address: AddressBarStyle::Breadcrumb,
                search: SearchPosition::Right,
                sidebar: SidebarSpec {
                    width: 190.0,
                    show_favorites: true,
                    show_locations: true,
                    show_tags: false,
                    show_tools: false,
                    header_style: SidebarHeaderStyle::CollapsibleZone,
                },
                details_pane: DetailsPanePosition::Right,
                status_bar: true,
                path_bar_bottom: false,
                spacing: SpacingTokens {
                    toolbar_item_spacing: 4.0,
                    toolbar_padding_y: 2.0,
                    sidebar_item_height: 32.0,
                },
            },
            LookPreset::Nautilus => Self {
                window_controls: WindowControlsPosition::RightCaption,
                title_bar: TitleBarStyle::Unified,
                tab_position: TabPosition::None,
                menubar: false,
                toolbar: vec![
                    ToolbarItem::Back,
                    ToolbarItem::Forward,
                    ToolbarItem::ViewSwitcher,
                    ToolbarItem::Search,
                    ToolbarItem::Hamburger,
                ],
                address: AddressBarStyle::PillPathBar,
                search: SearchPosition::Toggle,
                sidebar: SidebarSpec {
                    width: 180.0,
                    show_favorites: true,
                    show_locations: true,
                    show_tags: false,
                    show_tools: false,
                    header_style: SidebarHeaderStyle::SectionTitle,
                },
                details_pane: DetailsPanePosition::None,
                status_bar: false,
                path_bar_bottom: false,
                spacing: SpacingTokens {
                    toolbar_item_spacing: 8.0,
                    toolbar_padding_y: 6.0,
                    sidebar_item_height: 36.0,
                },
            },
            LookPreset::Thunar => Self {
                window_controls: WindowControlsPosition::RightCaption,
                title_bar: TitleBarStyle::Separate,
                tab_position: TabPosition::None,
                menubar: true,
                toolbar: vec![
                    ToolbarItem::Back,
                    ToolbarItem::Forward,
                    ToolbarItem::Up,
                    ToolbarItem::Refresh,
                    ToolbarItem::NewFolder,
                    ToolbarItem::ViewSwitcher,
                ],
                address: AddressBarStyle::PathButtons,
                search: SearchPosition::Right,
                sidebar: SidebarSpec {
                    width: 170.0,
                    show_favorites: true,
                    show_locations: true,
                    show_tags: false,
                    show_tools: false,
                    header_style: SidebarHeaderStyle::SectionTitle,
                },
                details_pane: DetailsPanePosition::None,
                status_bar: true,
                path_bar_bottom: false,
                spacing: SpacingTokens {
                    toolbar_item_spacing: 2.0,
                    toolbar_padding_y: 2.0,
                    sidebar_item_height: 24.0,
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_all_look_presets_map_to_valid_spec() {
        for preset in LookPreset::all() {
            let spec = ChromeSpec::for_preset(preset);

            // Toolbar items must be unique within a preset
            let mut seen = HashSet::new();
            for item in &spec.toolbar {
                assert!(seen.insert(*item), "Duplicate toolbar item {:?}", item);
                assert!(!item.label().is_empty());
            }

            // Sidebar width must be positive and within reasonable bounds
            assert!(spec.sidebar.width >= 120.0 && spec.sidebar.width <= 300.0);

            // Spacing tokens must be positive
            assert!(spec.spacing.toolbar_item_spacing > 0.0);
            assert!(spec.spacing.toolbar_padding_y > 0.0);
            assert!(spec.spacing.sidebar_item_height >= 20.0);
        }
    }

    #[test]
    fn test_finder_spec_details() {
        let spec = ChromeSpec::for_preset(LookPreset::Finder);
        assert_eq!(spec.window_controls, WindowControlsPosition::LeftTraffic);
        assert_eq!(spec.title_bar, TitleBarStyle::Unified);
        assert_eq!(spec.tab_position, TabPosition::InHeader);
        assert!(!spec.menubar);
        assert!(spec.path_bar_bottom);
        assert!(spec.status_bar);
        assert_eq!(spec.details_pane, DetailsPanePosition::None);
    }

    #[test]
    fn test_windows_explorer_spec_details() {
        let spec = ChromeSpec::for_preset(LookPreset::WindowsExplorer);
        assert_eq!(spec.window_controls, WindowControlsPosition::RightCaption);
        assert_eq!(spec.title_bar, TitleBarStyle::Separate);
        assert_eq!(spec.tab_position, TabPosition::Top);
        assert_eq!(spec.address, AddressBarStyle::Breadcrumb);
        assert_eq!(spec.details_pane, DetailsPanePosition::Right);
        assert!(spec.toolbar.contains(&ToolbarItem::Cut));
        assert!(spec.toolbar.contains(&ToolbarItem::Copy));
        assert!(spec.toolbar.contains(&ToolbarItem::Paste));
        assert!(spec.toolbar.contains(&ToolbarItem::Rename));
        assert!(spec.toolbar.contains(&ToolbarItem::Delete));
    }

    #[test]
    fn test_nautilus_spec_details() {
        let spec = ChromeSpec::for_preset(LookPreset::Nautilus);
        assert_eq!(spec.window_controls, WindowControlsPosition::RightCaption);
        assert_eq!(spec.title_bar, TitleBarStyle::Unified);
        assert_eq!(spec.tab_position, TabPosition::None);
        assert_eq!(spec.search, SearchPosition::Toggle);
        assert!(!spec.status_bar);
        assert!(spec.toolbar.contains(&ToolbarItem::Hamburger));
    }

    #[test]
    fn test_thunar_spec_details() {
        let spec = ChromeSpec::for_preset(LookPreset::Thunar);
        assert_eq!(spec.window_controls, WindowControlsPosition::RightCaption);
        assert_eq!(spec.title_bar, TitleBarStyle::Separate);
        assert_eq!(spec.tab_position, TabPosition::None);
        assert!(spec.menubar);
        assert_eq!(spec.address, AddressBarStyle::PathButtons);
        assert!(spec.status_bar);
        assert_eq!(spec.spacing.sidebar_item_height, 24.0);
    }

    #[test]
    fn test_default_is_finder_for_zero_visual_change() {
        let default_spec = ChromeSpec::default();
        let finder_spec = ChromeSpec::for_preset(LookPreset::Finder);
        assert_eq!(default_spec, finder_spec);
    }

    #[test]
    fn test_overrides_survive_preset_switch() {
        let mut cfg = crate::core::MossambiConfig::default();
        cfg.apply_look_preset(LookPreset::Finder);
        assert_eq!(cfg.chrome_spec().sidebar.show_tags, cfg.show_tags_section);

        cfg.show_tags_section = false;
        assert!(!cfg.chrome_spec().sidebar.show_tags);

        // Switch preset to WindowsExplorer
        cfg.apply_look_preset(LookPreset::WindowsExplorer);
        assert_eq!(cfg.look_preset, Some(LookPreset::WindowsExplorer));
        assert_eq!(cfg.icon_pack, "win11");

        // Override icon pack and verify override persists
        cfg.icon_pack = "papirus".to_string();
        assert_eq!(cfg.icon_pack, "papirus");
        assert!(!cfg.chrome_spec().sidebar.show_tags);
    }
}
