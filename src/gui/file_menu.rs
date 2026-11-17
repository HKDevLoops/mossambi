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
use crate::gui::macos_theme::ThemePalette;
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

impl MenuAction {
    /// Keyboard shortcut shown at the right edge of the row (Explorer/Finder
    /// both show these; they double as discoverability for the bindings).
    pub fn shortcut(&self) -> Option<&'static str> {
        use MenuAction::*;
        Some(match self {
            Cut => "Ctrl+X",
            Copy => "Ctrl+C",
            Paste => "Ctrl+V",
            Rename => "F2",
            Delete => "Del",
            Properties => "Alt+Enter",
            SelectAll => "Ctrl+A",
            Refresh => "F5",
            NewFolder => "Ctrl+Shift+N",
            NewTab => "Ctrl+T",
            QuickLook => "Space",
            Terminal => "Ctrl+Shift+T",
            _ => return None,
        })
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
    pack: IconPack,
    action: MenuAction,
    label: &str,
    style: MenuStyle,
    tint: Color32,
) -> Response {
    // Finder menus are text-only; Windows and Linux menus lead with an icon
    // from the active icon pack (so changing the pack restyles menus too).
    let icon = (style != MenuStyle::Finder).then(|| (pack, action.glyph(), tint));
    menu_row(ui, label, icon, action.shortcut())
}

/// Render a menu entry with an arbitrary glyph and explicit label - used by
/// entries that are not plain [`MenuAction`] verbs (counters, plugs, app IDs...).
///
/// The glyph is retained in the signature for call-site compatibility but is
/// never painted: rows are text-first and iconless in every wording style
/// (Phase 3 Finder conformance, resolves the G-1 doc conflict).
pub fn menu_item_glyph(
    ui: &mut Ui,
    pack: IconPack,
    glyph: IconGlyph,
    label: &str,
    style: MenuStyle,
    tint: Color32,
) -> Response {
    let icon = (style != MenuStyle::Finder).then_some((pack, glyph, tint));
    menu_row(ui, label, icon, None)
}

/// Render a menu row with a coloured tag dot in the icon column
/// (Finder-style tag assignment menus). Same layout as [`menu_row`].
pub fn menu_item_tag_dot(
    ui: &mut Ui,
    dot_color: Color32,
    label: &str,
    shortcut: Option<&str>,
) -> Response {
    use eframe::egui::{Align2, FontId, Pos2};
    let text_col = ui.visuals().widgets.inactive.fg_stroke.color;
    let hint_col = ui.visuals().weak_text_color();
    let font = FontId::proportional(14.0);
    let hint_font = FontId::proportional(12.0);
    let lead = 30.0;
    let hint_w = shortcut
        .map(|s| {
            ui.painter()
                .layout_no_wrap(s.to_owned(), hint_font.clone(), hint_col)
                .size()
                .x
                + 18.0
        })
        .unwrap_or(0.0);
    let budget = ui.available_width() - lead - hint_w - 8.0;
    let label = elide_to_width(ui, label, budget, font.clone());
    let galley = ui.painter().layout_no_wrap(label, font, text_col);
    let row_h = (galley.size().y + 10.0).max(26.0);
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
    ui.painter().circle_filled(
        Pos2::new(rect.left() + 16.0, rect.center().y),
        4.5,
        dot_color,
    );
    let pos = Pos2::new(rect.left() + lead, rect.center().y - galley.size().y * 0.5);
    ui.painter().galley(pos, galley.clone(), text_col);
    if let Some(s) = shortcut {
        ui.painter().text(
            Pos2::new(rect.right() - 10.0, rect.center().y),
            Align2::RIGHT_CENTER,
            s,
            hint_font,
            hint_col,
        );
    }
    resp.on_hover_cursor(eframe::egui::CursorIcon::Default)
}

/// One menu row: optional leading icon, label, optional right-aligned
/// shortcut hint. Text uses the theme's text colour (it was hard-coded
/// white, which vanished on every light theme).
fn menu_row(
    ui: &mut Ui,
    label: &str,
    icon: Option<(IconPack, IconGlyph, Color32)>,
    shortcut: Option<&str>,
) -> Response {
    menu_row_styled(ui, label, icon, shortcut, false)
}

/// Menu row with optional bold styling (used for default actions in drop action menu).
fn menu_row_styled(
    ui: &mut Ui,
    label: &str,
    icon: Option<(IconPack, IconGlyph, Color32)>,
    shortcut: Option<&str>,
    bold: bool,
) -> Response {
    use eframe::egui::{Align2, FontId, Pos2, Rect};
    // Primary text colour (text_color() is the secondary tier, too faint
    // for menu labels on light themes).
    let text_col = if bold {
        ui.visuals().strong_text_color()
    } else {
        ui.visuals().widgets.inactive.fg_stroke.color
    };
    let hint_col = ui.visuals().weak_text_color();
    let font = if bold {
        FontId::proportional(14.5)
    } else {
        FontId::proportional(14.0)
    };
    let hint_font = FontId::proportional(12.0);
    let lead = if icon.is_some() { 30.0 } else { 10.0 };
    let hint_w = shortcut
        .map(|s| {
            ui.painter()
                .layout_no_wrap(s.to_owned(), hint_font.clone(), hint_col)
                .size()
                .x
                + 18.0
        })
        .unwrap_or(0.0);
    let budget = ui.available_width() - lead - hint_w - 8.0;
    let label = elide_to_width(ui, label, budget, font.clone());
    let galley = ui.painter().layout_no_wrap(label, font, text_col);
    let row_h = (galley.size().y + 10.0).max(26.0);
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
    if let Some((pack, glyph, tint)) = icon {
        let r = Rect::from_center_size(
            Pos2::new(rect.left() + 16.0, rect.center().y),
            Vec2::splat(16.0),
        );
        crate::core::icon_packs::paint_icon(ui, pack, glyph, r, rgb(tint));
    }
    let pos = Pos2::new(rect.left() + lead, rect.center().y - galley.size().y * 0.5);
    ui.painter().galley(pos, galley.clone(), text_col);
    if bold {
        ui.painter()
            .galley(pos + Vec2::new(0.5, 0.0), galley, text_col);
    }
    if let Some(s) = shortcut {
        ui.painter().text(
            Pos2::new(rect.right() - 10.0, rect.center().y),
            Align2::RIGHT_CENTER,
            s,
            hint_font,
            hint_col,
        );
    }
    // Cursor discipline: menu rows always show the default arrow, never the
    // text pointer (I-beam only in text-editing widgets, per standing rule).
    resp.on_hover_cursor(eframe::egui::CursorIcon::Default)
}

/// Render a menu entry with an explicit override label and optional bold styling.
pub fn menu_item_labeled_bold(
    ui: &mut Ui,
    pack: IconPack,
    action: MenuAction,
    label: &str,
    style: MenuStyle,
    tint: Color32,
    bold: bool,
) -> Response {
    let icon = (style != MenuStyle::Finder).then(|| (pack, action.glyph(), tint));
    menu_row_styled(ui, label, icon, action.shortcut(), bold)
}

/// A submenu row that matches [`menu_row`]: same 14 px label and, outside
/// Finder style, a leading icon from the active pack (egui's plain
/// menu_button had neither, so submenus looked out of line).
pub fn submenu<R>(
    ui: &mut Ui,
    label: &str,
    pack: IconPack,
    style: MenuStyle,
    tint: Color32,
    action: MenuAction,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> eframe::egui::InnerResponse<Option<R>> {
    if style == MenuStyle::Finder {
        return ui.menu_button(eframe::egui::RichText::new(label).size(14.0), add_contents);
    }
    // egui's sub-menu button cannot lead with our vector icons, so reserve
    // the icon column with leading spaces and paint the icon into it; the
    // label then lines up with the plain rows above and below.
    let padded = format!("{}{}", " ".repeat(7), label);
    let out = ui.menu_button(eframe::egui::RichText::new(padded).size(14.0), add_contents);
    let r = out.response.rect;
    let icon_rect = eframe::egui::Rect::from_center_size(
        eframe::egui::Pos2::new(r.left() + 16.0, r.center().y),
        Vec2::splat(16.0),
    );
    crate::core::icon_packs::paint_icon(ui, pack, action.glyph(), icon_rect, rgb(tint));
    out
}

/// A user-defined custom context-menu row (Phase 4 customization).
///
/// Renders identically to built-in actions - the Finder-style text-first
/// iconless row - with the label coming from the user's configuration.
pub fn menu_item_custom(ui: &mut Ui, label: &str) -> Response {
    menu_row(ui, label, None, None)
}

/// Column-chooser toggle row (accent-highlight method).
///
/// State is carried by accent colour, not `[x]`/`[ ]` text glyphs: shown
/// rows paint the soft accent selection fill (45 alpha dark / 26 light -
/// the design.rs selection language) with a painted check square, hidden
/// rows stay flat and muted with an outlined empty square, and locked
/// rows render as a pinned non-interactive chip. Text keeps full contrast
/// over the tint (text_primary on fills, text_muted on flat rows) per the
/// standing legibility rule, and menu rows keep the default arrow cursor.
pub fn menu_item_toggle(
    ui: &mut Ui,
    label: &str,
    shown: bool,
    locked: bool,
    palette: &ThemePalette,
) -> Response {
    let font = eframe::egui::FontId::proportional(15.0);
    let budget = ui.available_width() - 34.0;
    let label = elide_to_width(ui, label, budget, font.clone());
    let text_color = if shown && !locked {
        palette.text_primary
    } else {
        palette.text_muted
    };
    let galley = ui.painter().layout_no_wrap(label, font, text_color);
    let row_h = galley.size().y + 8.0;
    let sense = if locked {
        Sense::hover()
    } else {
        Sense::click()
    };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), sense);
    let (base_alpha, hover_alpha) = if palette.is_dark {
        (45_u8, 65_u8)
    } else {
        (26_u8, 42_u8)
    };
    let tinted = |c: Color32, a: u8| Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a);
    if locked {
        ui.painter().rect_filled(
            rect,
            eframe::egui::Rounding::same(4.0),
            tinted(palette.card_hover, if palette.is_dark { 20 } else { 14 }),
        );
    } else if shown {
        let a = if resp.hovered() || resp.is_pointer_button_down_on() {
            hover_alpha
        } else {
            base_alpha
        };
        ui.painter().rect_filled(
            rect,
            eframe::egui::Rounding::same(4.0),
            tinted(palette.accent, a),
        );
    } else if resp.hovered() || resp.is_pointer_button_down_on() {
        ui.painter().rect_filled(
            rect,
            eframe::egui::Rounding::same(4.0),
            tinted(palette.card_hover, if palette.is_dark { 45 } else { 28 }),
        );
    }
    if !locked {
        let sq = eframe::egui::Rect::from_center_size(
            eframe::egui::Pos2::new(rect.left() + 14.0, rect.center().y),
            Vec2::splat(10.0),
        );
        if shown {
            ui.painter()
                .rect_filled(sq, eframe::egui::Rounding::same(3.0), palette.accent);
            let check = if palette.is_dark {
                Color32::from_rgb(18, 18, 18)
            } else {
                Color32::WHITE
            };
            let c = sq.center();
            ui.painter().line_segment(
                [
                    eframe::egui::Pos2::new(c.x - 2.4, c.y + 0.6),
                    eframe::egui::Pos2::new(c.x - 0.6, c.y + 2.6),
                ],
                eframe::egui::Stroke::new(1.6_f32, check),
            );
            ui.painter().line_segment(
                [
                    eframe::egui::Pos2::new(c.x - 0.6, c.y + 2.6),
                    eframe::egui::Pos2::new(c.x + 3.2, c.y - 2.4),
                ],
                eframe::egui::Stroke::new(1.6_f32, check),
            );
        } else {
            ui.painter().rect_stroke(
                sq,
                eframe::egui::Rounding::same(3.0),
                eframe::egui::Stroke::new(1.0_f32, palette.text_muted),
            );
        }
    }
    let pos = eframe::egui::Align2::LEFT_CENTER
        .align_size_within_rect(galley.size(), rect.shrink2(Vec2::new(26.0, 0.0)))
        .min;
    ui.painter().galley(pos, galley, text_color);
    // Cursor discipline: menu rows always show the default arrow, never
    // the text pointer (standing rule).
    resp.on_hover_cursor(eframe::egui::CursorIcon::Default)
}

/// Measures a label and chops it with an ASCII ellipsis until it fits the
/// given width budget (Finder truncation style: one line, ellipsis at end).
pub(crate) fn elide_to_width(
    ui: &Ui,
    label: &str,
    max_width: f32,
    font: eframe::egui::FontId,
) -> String {
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
