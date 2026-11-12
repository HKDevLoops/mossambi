//! Explorer-grade context menus.
//!
//! A single logical action is rendered with three different vocabularies:
//!
//! * [`MenuStyle::Windows`] - Windows 11 Explorer shell wording and order.
//! * [`MenuStyle::Finder`]   - macOS Finder wording and order.
//! * [`MenuStyle::Linux`]    - GNOME Nautilus wording and order.
//!
//! Every entry is prefixed with a vector glyph from the active icon pack, so
//! the menu never falls back to emoji typography.

use crate::core::config::MenuStyle;
use crate::core::icon_packs::{IconGlyph, IconPack};
use eframe::egui::{Color32, Response, Sense, Ui, Vec2};

/// Every verb Mossambi can offer from a context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    OpenFolder,
    OpenFile,
    OpenWith,
    Edit,
    Play,
    Print,
    Terminal,
    Reveal,
    QuickLook,
    Wallpaper,
    Rotate,
    Extract,
    Defender,
    PackZip,
    Transcode,
    Conform,
    Inspect,
    CopyPath,
    Shortcut,
    Duplicate,
    SendTo,
    Cut,
    Copy,
    Paste,
    Rename,
    Delete,
    Properties,
    SelectAll,
    Refresh,
    NewFolder,
    SortBy,
    ViewMode,
    Compress,
    Tags,
    PinSidebar,
    NewWindow,
    NewTab,
}

impl MenuAction {
    /// Platform-specific wording.
    pub fn label(&self, style: MenuStyle) -> &'static str {
        use MenuAction::*;
        use MenuStyle::{Finder, Linux, Windows};
        match (self, style) {
            (OpenFolder, Windows) => "Open",
            (OpenFolder, Finder) => "Open",
            (OpenFolder, Linux) => "Open",

            (OpenFile, Windows) => "Open",
            (OpenFile, Finder) => "Open",
            (OpenFile, Linux) => "Open",

            (OpenWith, Windows) => "Open with",
            (OpenWith, Finder) => "Open With",
            (OpenWith, Linux) => "Open With Other Application",

            (Edit, Windows) => "Edit",
            (Edit, Finder) => "Open in Editor",
            (Edit, Linux) => "Open With Text Editor",

            (Play, Windows) => "Play",
            (Play, Finder) => "Play",
            (Play, Linux) => "Play",

            (Print, Windows) => "Print",
            (Print, Finder) => "Print",
            (Print, Linux) => "Print",

            (Terminal, Windows) => "Open in Terminal",
            (Terminal, Finder) => "Open Terminal",
            (Terminal, Linux) => "Open in Terminal",

            (Reveal, Windows) => "Show in Explorer",
            (Reveal, Finder) => "Show in Finder",
            (Reveal, Linux) => "Show in File Manager",

            (QuickLook, Windows) => "Quick look",
            (QuickLook, Finder) => "Quick Look",
            (QuickLook, Linux) => "Preview",

            (Wallpaper, Windows) => "Set as desktop background",
            (Wallpaper, Finder) => "Set as Desktop Picture",
            (Wallpaper, Linux) => "Set as Wallpaper",

            (Rotate, Windows) => "Rotate right",
            (Rotate, Finder) => "Rotate Right",
            (Rotate, Linux) => "Rotate Right",

            (Extract, Windows) => "Extract here",
            (Extract, Finder) => "Extract Here",
            (Extract, Linux) => "Extract Here",

            (Defender, Windows) => "Scan with Microsoft Defender",
            (Defender, Finder) => "Scan with Microsoft Defender",
            (Defender, Linux) => "Scan with Microsoft Defender",

            (PackZip, Windows) => "Compress to ZIP file",
            (PackZip, Finder) => "Compress",
            (PackZip, Linux) => "Compress",

            (Compress, Windows) => "Compress & Optimize",
            (Compress, Finder) => "Compress & Optimize",
            (Compress, Linux) => "Compress & Optimize",

            (Transcode, Windows) => "Convert with Mossambi",
            (Transcode, Finder) => "Convert Using Mossambi",
            (Transcode, Linux) => "Convert Using Mossambi",

            (Conform, Windows) => "Conform to exam specification",
            (Conform, Finder) => "Conform to Exam Specification",
            (Conform, Linux) => "Conform to Exam Specification",

            (Inspect, Windows) => "Quality & integrity inspect",
            (Inspect, Finder) => "Quality & Integrity Inspect",
            (Inspect, Linux) => "Quality & Integrity Inspect",

            (CopyPath, Windows) => "Copy as path",
            (CopyPath, Finder) => "Copy Path",
            (CopyPath, Linux) => "Copy as Path",

            (Shortcut, Windows) => "Create shortcut",
            (Shortcut, Finder) => "Make Alias",
            (Shortcut, Linux) => "Create Link",

            (Duplicate, Windows) => "Create a copy",
            (Duplicate, Finder) => "Duplicate",
            (Duplicate, Linux) => "Duplicate",

            (SendTo, Windows) => "Send to",
            (SendTo, Finder) => "Send To",
            (SendTo, Linux) => "Send To...",

            (Cut, Windows) => "Cut",
            (Cut, Finder) => "Cut",
            (Cut, Linux) => "Cut",

            (Copy, Windows) => "Copy",
            (Copy, Finder) => "Copy",
            (Copy, Linux) => "Copy",

            (Paste, Windows) => "Paste",
            (Paste, Finder) => "Paste",
            (Paste, Linux) => "Paste",

            (Rename, Windows) => "Rename",
            (Rename, Finder) => "Rename",
            (Rename, Linux) => "Rename",

            (Delete, Windows) => "Delete",
            (Delete, Finder) => "Move to Trash",
            (Delete, Linux) => "Move to Trash",

            (Properties, Windows) => "Properties",
            (Properties, Finder) => "Get Info",
            (Properties, Linux) => "Properties",

            (SelectAll, Windows) => "Select all",
            (SelectAll, Finder) => "Select All",
            (SelectAll, Linux) => "Select All",

            (Refresh, Windows) => "Refresh",
            (Refresh, Finder) => "Refresh",
            (Refresh, Linux) => "Reload",

            (NewFolder, Windows) => "New folder",
            (NewFolder, Finder) => "New Folder",
            (NewFolder, Linux) => "New Folder",

            (SortBy, Windows) => "Sort by",
            (SortBy, Finder) => "Sort By",
            (SortBy, Linux) => "Sort By",

            (ViewMode, Windows) => "View",
            (ViewMode, Finder) => "View",
            (ViewMode, Linux) => "View",

            (Tags, Windows) => "Tags",
            (Tags, Finder) => "Tags",
            (Tags, Linux) => "Tags",

            (PinSidebar, Windows) => "Pin to Sidebar",
            (PinSidebar, Finder) => "Pin to Sidebar",
            (PinSidebar, Linux) => "Pin to Sidebar",

            (NewWindow, Windows) => "Open in new window",
            (NewWindow, Finder) => "New Finder Window",
            (NewWindow, Linux) => "Open in New Window",

            (NewTab, Windows) => "Open in new tab",
            (NewTab, Finder) => "New Tab",
            (NewTab, Linux) => "Open in New Tab",
        }
    }

    /// Vector glyph that precedes the label.
    pub fn glyph(&self) -> IconGlyph {
        use MenuAction::*;
        match self {
            NewWindow => IconGlyph::DriveSystem,
            NewTab => IconGlyph::ActionNewFolder,
            OpenFolder => IconGlyph::ActionFolderOpen,
            OpenFile => IconGlyph::ActionOpen,
            OpenWith => IconGlyph::ActionView,
            Edit => IconGlyph::ActionEdit,
            Play => IconGlyph::ActionMedia,
            Print => IconGlyph::ActionPrint,
            Terminal => IconGlyph::ActionTerminal,
            Reveal => IconGlyph::ActionFolderOpen,
            QuickLook => IconGlyph::ActionEye,
            Wallpaper => IconGlyph::ActionWallpaper,
            Rotate => IconGlyph::ActionRotate,
            Extract => IconGlyph::ActionExtract,
            Defender => IconGlyph::ActionShield,
            PackZip | Compress => IconGlyph::PageArchive,
            Transcode => IconGlyph::ActionRefresh,
            Conform => IconGlyph::ActionCheck,
            Inspect => IconGlyph::ActionShield,
            CopyPath => IconGlyph::ActionPath,
            Shortcut => IconGlyph::ActionShortcut,
            Duplicate => IconGlyph::ActionCopy,
            SendTo => IconGlyph::ActionMore,
            Cut => IconGlyph::ActionCut,
            Copy => IconGlyph::ActionCopy,
            Paste => IconGlyph::ActionPaste,
            Rename => IconGlyph::ActionRename,
            Delete => IconGlyph::ActionDelete,
            Properties => IconGlyph::ActionProperties,
            SelectAll => IconGlyph::ActionSelectAll,
            Refresh => IconGlyph::ActionRefresh,
            NewFolder => IconGlyph::ActionNewFolder,
            SortBy => IconGlyph::ActionSort,
            ViewMode => IconGlyph::ActionView,
            Tags => IconGlyph::ActionMore,
            PinSidebar => IconGlyph::ActionPin,
        }
    }
}

/// Convert an egui colour into the `[r, g, b]` triple the icon engine wants.
pub fn rgb(c: Color32) -> [u8; 3] {
    [c.r(), c.g(), c.b()]
}

/// Render a menu entry with the platform wording (row rendering is
/// text-first and iconless in every style - see menu_row_text).
pub fn menu_item(
    ui: &mut Ui,
    pack: IconPack,
    action: MenuAction,
    style: MenuStyle,
    tint: Color32,
) -> Response {
    menu_item_labeled(ui, pack, action, action.label(style), style, tint)
}

/// Render a menu entry with an explicit override label (e.g. "Transcode to JPG").
///
/// Phase 3 Finder conformance: every wording style renders the same
/// text-first, iconless row (macOS Finder design directive - the wording
/// style only controls row order/sets at the call sites).
pub fn menu_item_labeled(
    ui: &mut Ui,
    _pack: IconPack,
    _action: MenuAction,
    label: &str,
    _style: MenuStyle,
    _tint: Color32,
) -> Response {
    menu_row_text(ui, label)
}

/// Render a menu entry with an arbitrary glyph and explicit label - used by
/// entries that are not plain [`MenuAction`] verbs (counters, plugs, app IDs...).
///
/// The glyph is retained in the signature for call-site compatibility but is
/// never painted: rows are text-first and iconless in every wording style
/// (Phase 3 Finder conformance, resolves the G-1 doc conflict).
pub fn menu_item_glyph(
    ui: &mut Ui,
    _pack: IconPack,
    _glyph: IconGlyph,
    label: &str,
    _style: MenuStyle,
    _tint: Color32,
) -> Response {
    menu_row_text(ui, label)
}

/// One Finder-style menu row: full-width, text-first, iconless.
///
/// The row is a single click-sensed widget (allocation-native fresh id, the
/// 95cc29a click-registration fix refined), paints the egui widget-state
/// accent fill behind the label on hover/press (SubMenuButton idiom), and
/// uses 15 px white type per the macOS Finder design directive.
fn menu_row_text(ui: &mut Ui, label: &str) -> Response {
    let font = eframe::egui::FontId::proportional(15.0);
    let budget = ui.available_width() - 12.0;
    let label = elide_to_width(ui, label, budget, font.clone());
    let galley = ui
        .painter()
        .layout_no_wrap(label, font, eframe::egui::Color32::WHITE);
    let row_h = galley.size().y + 8.0;
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), Sense::click());
    let visuals = &ui.style().visuals.widgets;
    let state = if resp.is_pointer_button_down_on() {
        Some(&visuals.active)
    } else if resp.hovered() {
        Some(&visuals.hovered)
    } else {
        None
    };
    if let Some(v) = state {
        ui.painter()
            .rect_filled(rect.expand(v.expansion), v.rounding, v.weak_bg_fill);
    }
    let pos = eframe::egui::Align2::LEFT_CENTER
        .align_size_within_rect(galley.size(), rect.shrink2(Vec2::new(6.0, 0.0)))
        .min;
    ui.painter()
        .galley(pos, galley, eframe::egui::Color32::WHITE);
    // Cursor discipline: menu rows always show the default arrow, never the
    // text pointer (I-beam only in text-editing widgets, per standing rule).
    resp.on_hover_cursor(eframe::egui::CursorIcon::Default)
}

/// A user-defined custom context-menu row (Phase 4 customization).
///
/// Renders identically to built-in actions - the Finder-style text-first
/// iconless row - with the label coming from the user's configuration.
pub fn menu_item_custom(ui: &mut Ui, label: &str) -> Response {
    menu_row_text(ui, label)
}

/// Measures a label and chops it with an ASCII ellipsis until it fits the
/// given width budget (Finder truncation style: one line, ellipsis at end).
fn elide_to_width(ui: &Ui, label: &str, max_width: f32, font: eframe::egui::FontId) -> String {
    if max_width <= 0.0 {
        return label.to_owned();
    }
    let fits = |text: &str| {
        ui.painter()
            .layout_no_wrap(text.to_owned(), font.clone(), eframe::egui::Color32::WHITE)
            .size()
            .x
            <= max_width
    };
    if fits(label) {
        return label.to_owned();
    }
    let mut out: String = label.to_owned();
    loop {
        if fits(&format!("{out}...")) {
            return format!("{out}...");
        }
        if out.pop().is_none() {
            return "...".to_owned();
        }
    }
}

/// Bold header line for the selected item's name (Apple principle: the only
/// labeled part of a menu is the context header - groups are separated by
/// hairlines, not captions).
pub fn menu_header(ui: &mut Ui, name: &str, tint: Color32) {
    let font = eframe::egui::FontId::proportional(13.5);
    let name = elide_to_width(ui, name, ui.available_width() - 12.0, font);
    ui.label(
        eframe::egui::RichText::new(name)
            .strong()
            .size(13.5)
            .color(tint),
    );
    ui.add_space(1.0);
}
