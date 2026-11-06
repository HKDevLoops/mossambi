//! Explorer-grade context menus.
//!
//! A single logical action is rendered with three different vocabularies:
//!
//! * [`MenuStyle::Windows`] — Windows 11 Explorer shell wording and order.
//! * [`MenuStyle::Finder`]   — macOS Finder wording and order.
//! * [`MenuStyle::Linux`]    — GNOME Nautilus wording and order.
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
            (SendTo, Linux) => "Send To…",

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
        }
    }

    /// Vector glyph that precedes the label.
    pub fn glyph(&self) -> IconGlyph {
        use MenuAction::*;
        match self {
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
        }
    }
}

/// Convert an egui colour into the `[r, g, b]` triple the icon engine wants.
pub fn rgb(c: Color32) -> [u8; 3] {
    [c.r(), c.g(), c.b()]
}

/// Render a menu entry: vector glyph + platform wording.
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
/// Apple HIG (Finder style): menus are text-only — no icon chips, 15 px type,
/// generous row padding. Windows/Linux keep the glyph + label layout.
pub fn menu_item_labeled(
    ui: &mut Ui,
    pack: IconPack,
    action: MenuAction,
    label: &str,
    style: MenuStyle,
    tint: Color32,
) -> Response {
    let rgb = [tint.r(), tint.g(), tint.b()];
    if style == MenuStyle::Finder {
        return ui
            .horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);
                ui.add_space(6.0);
                ui.label(
                    eframe::egui::RichText::new(label)
                        .size(15.0)
                        .color(eframe::egui::Color32::WHITE),
                );
            })
            .response;
    }
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(9.0, 9.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(19.0), Sense::hover());
        crate::core::icon_packs::paint_icon(ui, pack, action.glyph(), rect, rgb);
        ui.label(egui_label(label));
    })
    .response
}

/// Render a menu entry with an arbitrary glyph and explicit label — used by
/// entries that are not plain [`MenuAction`] verbs (counters, plugs, app IDs…).
pub fn menu_item_glyph(
    ui: &mut Ui,
    pack: IconPack,
    glyph: IconGlyph,
    label: &str,
    style: MenuStyle,
    tint: Color32,
) -> Response {
    let rgb = [tint.r(), tint.g(), tint.b()];
    if style == MenuStyle::Finder {
        return ui
            .horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);
                ui.add_space(6.0);
                ui.label(
                    eframe::egui::RichText::new(label)
                        .size(15.0)
                        .color(eframe::egui::Color32::WHITE),
                );
            })
            .response;
    }
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(9.0, 9.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(19.0), Sense::hover());
        crate::core::icon_packs::paint_icon(ui, pack, glyph, rect, rgb);
        ui.label(egui_label(label));
    })
    .response
}

/// Bold header line for the selected item's name (Apple principle: the only
/// labeled part of a menu is the context header — groups are separated by
/// hairlines, not captions).
pub fn menu_header(ui: &mut Ui, name: &str, tint: Color32) {
    ui.label(
        eframe::egui::RichText::new(name)
            .strong()
            .size(13.5)
            .color(tint),
    );
    ui.add_space(1.0);
}

fn egui_label(text: &str) -> eframe::egui::WidgetText {
    eframe::egui::WidgetText::RichText(eframe::egui::RichText::new(text))
}
