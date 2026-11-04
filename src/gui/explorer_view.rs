//! File Pilot-Grade High-Performance File Explorer for Mossambi Studio
//!
//! Features:
//! - Sub-10MB Lightweight Architecture with zero-lag async directory scanning
//! - 100% Native Translucent Acrylic & Tabbed Mica Glass integration (Zero opaque blocks)
//! - Navigation History Engine: Back (Alt+Left), Forward (Alt+Right), Up (Alt+Up/Backspace), Refresh (F5)
//! - System Drives Sidebar with Real-Time Storage Space Usage Meters (GB free of total)
//! - Quick Access Bookmarks (Desktop, Downloads, Documents, Pictures, Videos, Music, Home)
//! - Clickable Breadcrumbs Path Bar with Instant Jump & Direct Path Editing (Ctrl+L / Alt+D)
//! - Multi-View Modes: Details / Table View and Grid / Tiles View with Toolbar Toggle
//! - Multi-Category Filter Tabs: All, Images, Audio, Video, Docs, Archives, Code with Live Count Badges
//! - Interactive Column Sorting: Name, Type, Size, Date with Ascending/Descending Toggle
//! - Full File Operations: In-Place Rename (F2), New Folder (Ctrl+Shift+N), Copy (Ctrl+C), Cut (Ctrl+X), Paste (Ctrl+V), Delete
//! - macOS Quick Look / Peek Display Modal Overlay (Images, Code/Text, Audio/Video, Archives, Hex/SHA-256)
//! - Universal Drag & Drop Ingestion with Glowing Hover Zone Indicator
//! - Rich Right-Click Context Menus for files, folders, and canvas background

use crate::core::{
    copy_entry_recursive, format_file_size, fuzzy_match, get_system_drives, list_directory_sync,
    move_entry, parse_breadcrumbs, remove_entry_recursive, resolve_file_icon,
    resolve_unique_paste_path, spawn_async_dir_scan, DriveInfo, FileItem, GestureKind,
    KinematicGestureTracker, ScanBatch,
};
use crate::gui::macos_theme::ThemePalette;
use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Stroke, Ui, Vec2};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Details,
    Grid,
    Columns,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortColumn {
    Name,
    Size,
    Extension,
    Date,
    Permissions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl SortDirection {
    pub fn toggle(self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }

    pub fn indicator(self) -> &'static str {
        match self {
            Self::Ascending => "▲",
            Self::Descending => "▼",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryFilter {
    All,
    Images,
    Audio,
    Video,
    Documents,
    Archives,
    Code,
}

impl CategoryFilter {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "🌐 All",
            Self::Images => "🖼 Images",
            Self::Audio => "🎵 Audio",
            Self::Video => "🎬 Video",
            Self::Documents => "📄 Docs",
            Self::Archives => "📦 Archives",
            Self::Code => "💻 Code",
        }
    }

    pub fn matches(self, item: &FileItem) -> bool {
        if self == Self::All {
            return true;
        }
        if item.is_dir {
            return false;
        }
        match self {
            Self::All => true,
            Self::Images => matches!(
                item.extension.as_str(),
                "JPG"
                    | "JPEG"
                    | "PNG"
                    | "WEBP"
                    | "GIF"
                    | "AVIF"
                    | "TIFF"
                    | "BMP"
                    | "ICO"
                    | "SVG"
                    | "HEIC"
                    | "RAW"
            ),
            Self::Audio => matches!(
                item.extension.as_str(),
                "MP3" | "WAV" | "FLAC" | "AAC" | "OGG" | "M4A" | "OPUS" | "AIFF" | "WMA"
            ),
            Self::Video => matches!(
                item.extension.as_str(),
                "MP4" | "MKV" | "MOV" | "AVI" | "WEBM" | "FLV" | "WMV" | "M4V" | "TS"
            ),
            Self::Documents => matches!(
                item.extension.as_str(),
                "PDF"
                    | "DOC"
                    | "DOCX"
                    | "XLS"
                    | "XLSX"
                    | "PPT"
                    | "PPTX"
                    | "TXT"
                    | "MD"
                    | "CSV"
                    | "RTF"
                    | "ODT"
                    | "ODS"
            ),
            Self::Archives => matches!(
                item.extension.as_str(),
                "ZIP" | "TAR" | "GZ" | "XZ" | "ZST" | "7Z" | "BZ2" | "RAR" | "TGZ"
            ),
            Self::Code => matches!(
                item.extension.as_str(),
                "RS" | "PY"
                    | "JS"
                    | "TS"
                    | "HTML"
                    | "CSS"
                    | "C"
                    | "CPP"
                    | "H"
                    | "JSON"
                    | "XML"
                    | "YAML"
                    | "YML"
                    | "TOML"
                    | "SH"
                    | "BAT"
                    | "PS1"
                    | "LOG"
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExplorerTab {
    pub id: u64,
    pub title: String,
    pub icon: &'static str,
    pub path: PathBuf,
    pub is_home: bool,
    pub search_query: String,
}

/// Lazy per-path image thumbnail cache entry (path -> mtime -> texture)
struct CachedThumb {
    mtime: Option<std::time::SystemTime>,
    state: ThumbState,
}

enum ThumbState {
    /// Decoded texture is ready (None = load failed / not an image)
    Ready(Option<egui::TextureHandle>),
}

/// Finder-style long-hover preview card state
struct HoverPreviewState {
    item: FileItem,
    /// Pivots the floating card to the right of the hovered row / below a tile
    anchor: Pos2,
    started_at: Instant,
    content: HoverContent,
}

enum HoverContent {
    /// Resolved image thumbnail ready to show
    Image(egui::TextureHandle),
    /// Folder children (first slice + total child count)
    Children(Vec<FileItem>, usize),
    /// Icon + metadata card (non-image files)
    Meta,
}

pub struct ExplorerView {
    pub current_dir: PathBuf,
    pub entries: Vec<FileItem>,
    pub selected_paths: HashSet<PathBuf>,
    pub search_query: String,
    pub drives: Vec<DriveInfo>,

    // Multi-Tab Explorer Architecture
    pub tabs: Vec<ExplorerTab>,
    pub active_tab_index: usize,
    pub next_tab_id: u64,

    // Navigation History
    pub history: Vec<PathBuf>,
    pub history_idx: usize,

    // Sorting & View Modes
    pub view_mode: ViewMode,
    pub icon_size: f32,
    pub sort_column: SortColumn,
    pub sort_direction: SortDirection,
    pub category_filter: CategoryFilter,

    // Path Editing
    pub is_editing_path: bool,
    pub path_edit_buffer: String,

    // File Operations State
    pub renaming_path: Option<(PathBuf, String)>,
    pub show_new_folder_dialog: bool,
    pub new_folder_buffer: String,
    pub clipboard_paths: Vec<PathBuf>,
    pub clipboard_is_cut: bool,

    // Async Scanning
    pub scan_gen: Arc<AtomicU64>,
    pub scan_tx: Sender<ScanBatch>,
    pub scan_rx: Receiver<ScanBatch>,

    // Quick Look / Peek Modal Overlay State
    pub peek_item: Option<FileItem>,
    pub peek_texture: Option<(PathBuf, egui::TextureHandle)>,
    pub peek_text_cache: Option<(PathBuf, String)>,
    pub peek_hash_cache: Option<(PathBuf, String)>,

    // In-Explorer Gesture Tracker
    pub gesture_tracker: KinematicGestureTracker,
    pub gesture_active: bool,
    pub last_triggered_gesture: Option<GestureKind>,

    // Properties Modal State
    pub properties_item: Option<FileItem>,

    // Vim & Emacs Navigation State
    pub focused_row: usize,
    pub last_g_press: Option<Instant>,
    pub focus_search_requested: bool,

    // macOS Finder & Linux System Features
    pub show_hidden_files: bool,
    pub column_subfolder: Option<(PathBuf, Vec<FileItem>)>,
    pub column_preview_item: Option<FileItem>,

    // Windows 11 & macOS Home Dashboard View
    pub is_home_view: bool,
    pub show_internal_toolbar: bool,

    // Finder-style long-hover preview (files & folders)
    hover_preview: Option<HoverPreviewState>,

    // Lazy image thumbnail cache used by Grid tiles & hover previews
    thumb_cache: std::collections::HashMap<PathBuf, CachedThumb>,
    thumb_decode_budget: (u32, Instant),

    // Shift-range selection anchor (Finder parity)
    selection_anchor: Option<usize>,

    // Active icon pack (loaded from MossambiConfig every frame)
    pub icon_pack: crate::core::icon_packs::IconPack,

    // Explorer preference mirror (synced by studio_app from MossambiConfig)
    pub checkbox_mode: crate::core::config::CheckboxMode,
    pub menu_style: crate::core::config::MenuStyle,
    pub animations: crate::core::config::AnimationSettings,
    pub hover_anims: crate::gui::anim::AnimRegistry,
    /// Windows-Explorer zoom levels: 0 = Details form, 1-4 = Grid icon sizes.
    pub explorer_zoom: u8,
}

impl Default for ExplorerView {
    fn default() -> Self {
        let (scan_tx, scan_rx) = channel();
        let home_opt = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .ok()
            .map(PathBuf::from);
        let current_dir = home_opt
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let drives = get_system_drives();
        let scan_gen = Arc::new(AtomicU64::new(1));

        let is_home = home_opt.is_some();
        let initial_tab = ExplorerTab {
            id: 1,
            title: if is_home {
                "Home".to_string()
            } else {
                current_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Files")
                    .to_string()
            },
            icon: if is_home { "🏠" } else { "📁" },
            path: current_dir.clone(),
            is_home,
            search_query: String::new(),
        };

        let mut view = Self {
            current_dir: current_dir.clone(),
            entries: Vec::new(),
            selected_paths: HashSet::new(),
            search_query: String::new(),
            drives,
            tabs: vec![initial_tab],
            active_tab_index: 0,
            next_tab_id: 2,
            history: vec![current_dir.clone()],
            history_idx: 0,
            view_mode: ViewMode::Details,
            icon_size: 96.0,
            sort_column: SortColumn::Name,
            sort_direction: SortDirection::Ascending,
            category_filter: CategoryFilter::All,
            is_editing_path: false,
            path_edit_buffer: current_dir.to_string_lossy().to_string(),
            renaming_path: None,
            show_new_folder_dialog: false,
            is_home_view: home_opt.is_some(),
            show_internal_toolbar: false,
            new_folder_buffer: "New Folder".to_string(),
            clipboard_paths: Vec::new(),
            clipboard_is_cut: false,
            scan_gen: Arc::clone(&scan_gen),
            scan_tx: scan_tx.clone(),
            scan_rx,
            peek_item: None,
            peek_texture: None,
            peek_text_cache: None,
            peek_hash_cache: None,
            gesture_tracker: KinematicGestureTracker::new(true, true),
            gesture_active: false,
            last_triggered_gesture: None,
            properties_item: None,
            focused_row: 0,
            last_g_press: None,
            focus_search_requested: false,
            show_hidden_files: false,
            column_subfolder: None,
            column_preview_item: None,
            hover_preview: None,
            thumb_cache: std::collections::HashMap::new(),
            thumb_decode_budget: (0, Instant::now()),
            selection_anchor: None,
            icon_pack: crate::core::icon_packs::IconPack::MacosFinder,
            checkbox_mode: crate::core::config::CheckboxMode::default(),
            menu_style: crate::core::config::MenuStyle::default(),
            animations: crate::core::config::AnimationSettings::default(),
            hover_anims: crate::gui::anim::AnimRegistry::new(),
            explorer_zoom: 2,
        };

        view.navigate_to(current_dir);
        view
    }
}

impl ExplorerView {
    /// The five Explorer zoom steps (Windows Explorer parity):
    /// level 0 is the Details/list **form** — the icon-size scale is not
    /// applied there; levels 1-4 are Grid icon sizes Small → Medium → Large →
    /// Extra-large, multiplied over the user's base grid icon size.
    pub const EXPLORER_ZOOM_LEVELS: [f32; 5] = [0.60, 0.85, 1.0, 1.30, 1.70];

    /// Human readable label for a zoom level (Explorer's status-bar wording).
    pub fn zoom_level_name(level: u8) -> &'static str {
        match level {
            0 => "Details",
            1 => "Small icons",
            2 => "Medium icons",
            3 => "Large icons",
            _ => "Extra large icons",
        }
    }

    /// Navigate to a new directory asynchronously with zero UI hitching
    pub fn navigate_to(&mut self, dir: PathBuf) {
        if !dir.exists() {
            return;
        }

        // Push to navigation history
        if self.history.get(self.history_idx) != Some(&dir) {
            self.history.truncate(self.history_idx + 1);
            self.history.push(dir.clone());
            self.history_idx = self.history.len() - 1;
        }

        self.current_dir = dir.clone();
        self.path_edit_buffer = dir.to_string_lossy().to_string();
        self.is_editing_path = false;
        self.renaming_path = None;
        self.focused_row = 0;

        let home_opt = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .ok()
            .map(PathBuf::from);
        self.is_home_view = Some(&dir) == home_opt.as_ref();

        // Update active tab metadata
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.path = dir.clone();
            tab.is_home = self.is_home_view;
            tab.title = if self.is_home_view {
                "Home".to_string()
            } else {
                dir.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Folder")
                    .to_string()
            };
            tab.icon = if self.is_home_view { "🏠" } else { "📁" };
        }

        let new_gen = self.scan_gen.fetch_add(1, Ordering::SeqCst) + 1;
        spawn_async_dir_scan(
            dir,
            new_gen,
            Arc::clone(&self.scan_gen),
            self.scan_tx.clone(),
        );
    }

    pub fn add_tab(&mut self, path: PathBuf) {
        let home_opt = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .ok()
            .map(PathBuf::from);
        let is_home = Some(&path) == home_opt.as_ref();
        let title = if is_home {
            "Home".to_string()
        } else {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Folder")
                .to_string()
        };
        let icon = if is_home { "🏠" } else { "📁" };
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        self.tabs.push(ExplorerTab {
            id,
            title,
            icon,
            path: path.clone(),
            is_home,
            search_query: String::new(),
        });
        self.active_tab_index = self.tabs.len() - 1;
        self.navigate_to(path);
    }

    pub fn close_tab(&mut self, idx: usize) {
        if self.tabs.len() > 1 && idx < self.tabs.len() {
            self.tabs.remove(idx);
            if self.active_tab_index >= self.tabs.len() {
                self.active_tab_index = self.tabs.len() - 1;
            }
            let target = self.tabs[self.active_tab_index].path.clone();
            self.navigate_to(target);
        }
    }

    pub fn switch_to_tab(&mut self, idx: usize) {
        if idx < self.tabs.len() {
            self.active_tab_index = idx;
            let tab = &self.tabs[idx];
            let target = tab.path.clone();
            self.search_query = tab.search_query.clone();
            self.navigate_to(target);
        }
    }

    pub fn can_go_back(&self) -> bool {
        self.history_idx > 0
    }

    pub fn can_go_forward(&self) -> bool {
        self.history_idx + 1 < self.history.len()
    }

    pub fn is_home_directory(&self) -> bool {
        let home_opt = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .ok()
            .map(PathBuf::from);
        Some(&self.current_dir) == home_opt.as_ref()
    }

    pub fn go_back(&mut self) {
        if self.can_go_back() {
            self.history_idx -= 1;
            let dir = self.history[self.history_idx].clone();
            self.current_dir = dir.clone();
            self.path_edit_buffer = dir.to_string_lossy().to_string();
            self.is_editing_path = false;
            let new_gen = self.scan_gen.fetch_add(1, Ordering::SeqCst) + 1;
            spawn_async_dir_scan(
                dir,
                new_gen,
                Arc::clone(&self.scan_gen),
                self.scan_tx.clone(),
            );
        }
    }

    pub fn go_forward(&mut self) {
        if self.can_go_forward() {
            self.history_idx += 1;
            let dir = self.history[self.history_idx].clone();
            self.current_dir = dir.clone();
            self.path_edit_buffer = dir.to_string_lossy().to_string();
            self.is_editing_path = false;
            let new_gen = self.scan_gen.fetch_add(1, Ordering::SeqCst) + 1;
            spawn_async_dir_scan(
                dir,
                new_gen,
                Arc::clone(&self.scan_gen),
                self.scan_tx.clone(),
            );
        }
    }

    /// Process incoming background directory scanning batches
    pub fn poll_background_scans(&mut self) {
        while let Ok(batch) = self.scan_rx.try_recv() {
            if batch.gen == self.scan_gen.load(Ordering::Relaxed) {
                self.entries = batch.entries;
            }
        }
    }

    /// Filter and sort entries according to active filter, search query, and sort column
    pub fn filtered_and_sorted_entries(&self) -> Vec<&FileItem> {
        let q = self.search_query.trim();
        let is_searching = !q.is_empty();

        let mut scored_list: Vec<(&FileItem, i64)> = self
            .entries
            .iter()
            .filter(|e| self.show_hidden_files || !e.is_hidden)
            .filter(|e| self.category_filter.matches(e))
            .filter_map(|e| {
                if is_searching {
                    let score = fuzzy_match(q, &e.name)?;
                    Some((e, score))
                } else {
                    Some((e, 0))
                }
            })
            .collect();

        scored_list.sort_by(|(a, score_a), (b, score_b)| {
            if is_searching
                && self.sort_column == SortColumn::Name
                && self.sort_direction == SortDirection::Ascending
            {
                return score_b
                    .cmp(score_a)
                    .then_with(|| b.is_dir.cmp(&a.is_dir))
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            }

            if a.is_dir != b.is_dir {
                return b.is_dir.cmp(&a.is_dir);
            }
            let ordering = match self.sort_column {
                SortColumn::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortColumn::Size => a.size_bytes.cmp(&b.size_bytes),
                SortColumn::Extension => a.extension.cmp(&b.extension),
                SortColumn::Date => a.modified.cmp(&b.modified),
                SortColumn::Permissions => a.permissions_mode.cmp(&b.permissions_mode),
            };
            let directed = match self.sort_direction {
                SortDirection::Ascending => ordering,
                SortDirection::Descending => ordering.reverse(),
            };
            if is_searching {
                directed.then_with(|| score_b.cmp(score_a))
            } else {
                directed
            }
        });

        scored_list.into_iter().map(|(item, _)| item).collect()
    }

    /// Returns the count of non-folder items matching a category
    pub fn count_category(&self, filter: CategoryFilter) -> usize {
        self.entries
            .iter()
            .filter(|e| !e.is_dir && filter.matches(e))
            .count()
    }

    /// Renders the complete File Pilot-Grade Explorer interface
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        mut on_stage_file: impl FnMut(PathBuf),
        mut on_transcode_target: impl FnMut(PathBuf, String),
        mut on_exam_conform: impl FnMut(PathBuf),
        mut on_pack_archive: impl FnMut(Vec<PathBuf>),
        mut on_inspect_file: impl FnMut(PathBuf),
    ) {
        self.poll_background_scans();

        // 1. External OS Drag & Drop Ingestion
        let (dropped_files, is_hovered) =
            ui.input(|i| (i.raw.dropped_files.clone(), !i.raw.hovered_files.is_empty()));

        if is_hovered {
            ui.add_space(4.0);
            let drop_frame = egui::Frame::none()
                .fill(Color32::from_rgba_premultiplied(120, 190, 32, 50))
                .stroke(Stroke::new(1.5_f32, palette.accent))
                .rounding(Rounding::same(8.0))
                .inner_margin(egui::Margin::symmetric(14.0, 7.0));
            drop_frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("📥 Drop external files or folders here to stage, inspect, or transcode")
                            .strong()
                            .color(Color32::from_rgb(215, 255, 120)),
                    );
                });
            });
            ui.add_space(4.0);
        }

        if !dropped_files.is_empty() {
            for file in dropped_files {
                if let Some(path) = file.path {
                    if path.is_dir() {
                        self.navigate_to(path);
                    } else {
                        on_stage_file(path);
                    }
                }
            }
        }

        // 2. Global Explorer Hotkey Interceptions with Vim & Emacs Navigation Engine
        let is_typing = self.is_editing_path
            || self.renaming_path.is_some()
            || self.show_new_folder_dialog
            || self.properties_item.is_some()
            || ui.memory(|m| m.focused().is_some());

        let nav_entries: Vec<FileItem> = self
            .filtered_and_sorted_entries()
            .into_iter()
            .cloned()
            .collect();
        let total_nav_items = nav_entries.len();

        ui.input(|i| {
            if i.key_pressed(egui::Key::Escape) {
                self.peek_item = None;
                self.peek_texture = None;
                self.peek_text_cache = None;
                self.peek_hash_cache = None;
                self.renaming_path = None;
                self.show_new_folder_dialog = false;
                self.properties_item = None;
            }

            // Vim & Emacs Navigation (Active when not typing in text boxes)
            if !is_typing {
                // Vim `j` / Down / Emacs `Ctrl+N`: Cursor Down
                if total_nav_items > 0
                    && ((i.key_pressed(egui::Key::J)
                        && !i.modifiers.command
                        && !i.modifiers.ctrl
                        && !i.modifiers.alt)
                        || i.key_pressed(egui::Key::ArrowDown)
                        || ((i.modifiers.ctrl || i.modifiers.command)
                            && i.key_pressed(egui::Key::N)))
                {
                    self.focused_row = (self.focused_row + 1).min(total_nav_items - 1);
                    if let Some(item) = nav_entries.get(self.focused_row) {
                        self.selected_paths.clear();
                        self.selected_paths.insert(item.path.clone());
                    }
                }

                // Vim `k` / Up / Emacs `Ctrl+P`: Cursor Up
                if total_nav_items > 0
                    && ((i.key_pressed(egui::Key::K)
                        && !i.modifiers.command
                        && !i.modifiers.ctrl
                        && !i.modifiers.alt)
                        || i.key_pressed(egui::Key::ArrowUp)
                        || ((i.modifiers.ctrl || i.modifiers.command)
                            && i.key_pressed(egui::Key::P)))
                {
                    self.focused_row = self.focused_row.saturating_sub(1);
                    if let Some(item) = nav_entries.get(self.focused_row) {
                        self.selected_paths.clear();
                        self.selected_paths.insert(item.path.clone());
                    }
                }

                // Vim `h` / Left / Emacs `Ctrl+B`: Parent Directory
                if (i.key_pressed(egui::Key::H)
                    && !i.modifiers.command
                    && !i.modifiers.ctrl
                    && !i.modifiers.alt)
                    || (i.modifiers.alt && i.key_pressed(egui::Key::ArrowLeft))
                    || ((i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(egui::Key::B))
                    || i.key_pressed(egui::Key::Backspace)
                {
                    if let Some(parent) = self.current_dir.parent() {
                        let p = parent.to_path_buf();
                        if !p.as_os_str().is_empty() {
                            self.navigate_to(p);
                        }
                    }
                }

                // Vim `l` / Right / Enter / Emacs `Ctrl+F`: Enter Directory or Open File (Finder parity)
                if (i.key_pressed(egui::Key::L)
                    && !i.modifiers.command
                    && !i.modifiers.ctrl
                    && !i.modifiers.alt)
                    || i.key_pressed(egui::Key::Enter)
                    || ((i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(egui::Key::F))
                {
                    if let Some(item) = nav_entries.get(self.focused_row) {
                        if item.is_dir {
                            self.navigate_to(item.path.clone());
                        } else {
                            open_in_default_app(&item.path);
                            on_stage_file(item.path.clone());
                        }
                    }
                }

                // Space: macOS Quick Look / Peek on the focused item (Finder parity)
                if i.key_pressed(egui::Key::Space) && !i.modifiers.ctrl && !i.modifiers.command {
                    if let Some(item) = nav_entries.get(self.focused_row) {
                        self.peek_item = Some(item.clone());
                        self.peek_texture = None;
                        self.peek_text_cache = None;
                        self.peek_hash_cache = None;
                    }
                }

                // F2: Rename focused item in place
                if i.key_pressed(egui::Key::F2) {
                    if let Some(item) = nav_entries.get(self.focused_row) {
                        if let Some(name) = item.path.file_name() {
                            self.renaming_path =
                                Some((item.path.clone(), name.to_string_lossy().to_string()));
                        }
                    }
                }

                // F5: Refresh directory
                if i.key_pressed(egui::Key::F5) {
                    let dir = self.current_dir.clone();
                    self.navigate_to(dir);
                }

                // Vim `gg` / Emacs `Alt+<`: Jump to First Item
                if (i.key_pressed(egui::Key::G)
                    && !i.modifiers.shift
                    && !i.modifiers.ctrl
                    && !i.modifiers.command
                    && !i.modifiers.alt)
                    || (i.modifiers.alt && i.key_pressed(egui::Key::Comma))
                {
                    if i.modifiers.alt {
                        self.focused_row = 0;
                        if let Some(item) = nav_entries.first() {
                            self.selected_paths.clear();
                            self.selected_paths.insert(item.path.clone());
                        }
                    } else {
                        let now = Instant::now();
                        if let Some(prev) = self.last_g_press {
                            if now.duration_since(prev) < std::time::Duration::from_millis(400) {
                                self.focused_row = 0;
                                if let Some(item) = nav_entries.first() {
                                    self.selected_paths.clear();
                                    self.selected_paths.insert(item.path.clone());
                                }
                                self.last_g_press = None;
                            } else {
                                self.last_g_press = Some(now);
                            }
                        } else {
                            self.last_g_press = Some(now);
                        }
                    }
                }

                // Vim `G` (Shift+G) / Emacs `Alt+>`: Jump to Last Item
                if total_nav_items > 0
                    && ((i.key_pressed(egui::Key::G)
                        && i.modifiers.shift
                        && !i.modifiers.ctrl
                        && !i.modifiers.command)
                        || (i.modifiers.alt && i.key_pressed(egui::Key::Period)))
                {
                    self.focused_row = total_nav_items - 1;
                    if let Some(item) = nav_entries.get(self.focused_row) {
                        self.selected_paths.clear();
                        self.selected_paths.insert(item.path.clone());
                    }
                }

                // Vim `/` / Emacs `Ctrl+S`: Focus Search Bar
                if (i.key_pressed(egui::Key::Slash)
                    && !i.modifiers.ctrl
                    && !i.modifiers.command
                    && !i.modifiers.alt)
                    || ((i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(egui::Key::S))
                {
                    self.focus_search_requested = true;
                }

                // Vim `v`: Visual Multi-Selection Toggle
                if i.key_pressed(egui::Key::V)
                    && !i.modifiers.ctrl
                    && !i.modifiers.command
                    && !i.modifiers.alt
                {
                    if let Some(item) = nav_entries.get(self.focused_row) {
                        if self.selected_paths.contains(&item.path) {
                            self.selected_paths.remove(&item.path);
                        } else {
                            self.selected_paths.insert(item.path.clone());
                        }
                    }
                }

                // Vim `y` / Emacs `Alt+W`: Yank / Copy
                if (i.key_pressed(egui::Key::Y)
                    && !i.modifiers.ctrl
                    && !i.modifiers.command
                    && !i.modifiers.alt)
                    || (i.modifiers.alt && i.key_pressed(egui::Key::W))
                {
                    if self.selected_paths.is_empty() {
                        if let Some(item) = nav_entries.get(self.focused_row) {
                            self.clipboard_paths = vec![item.path.clone()];
                        }
                    } else {
                        self.clipboard_paths = self.selected_paths.iter().cloned().collect();
                    }
                    self.clipboard_is_cut = false;
                }

                // Emacs `Ctrl+W`: Cut Region
                if (i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(egui::Key::W) {
                    if self.selected_paths.is_empty() {
                        if let Some(item) = nav_entries.get(self.focused_row) {
                            self.clipboard_paths = vec![item.path.clone()];
                        }
                    } else {
                        self.clipboard_paths = self.selected_paths.iter().cloned().collect();
                    }
                    self.clipboard_is_cut = true;
                }

                // Vim `p` / Emacs `Ctrl+Y`: Put / Paste
                if (i.key_pressed(egui::Key::P)
                    && !i.modifiers.ctrl
                    && !i.modifiers.command
                    && !i.modifiers.alt)
                    || ((i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(egui::Key::Y))
                {
                    let mut pasted = false;
                    for src in &self.clipboard_paths {
                        if let Some(file_name) = src.file_name() {
                            let dest = self.current_dir.join(file_name);
                            if self.clipboard_is_cut {
                                log_result("Move", src, move_entry(src, &dest));
                            } else {
                                let unique = resolve_unique_paste_path(&dest);
                                log_result("Copy", src, copy_entry_recursive(src, &unique));
                            }
                            pasted = true;
                        }
                    }
                    if self.clipboard_is_cut {
                        self.clipboard_paths.clear();
                        self.clipboard_is_cut = false;
                    }
                    if pasted {
                        self.navigate_to(self.current_dir.clone());
                    }
                }

                // Vim `d` (without modifiers): Delete focused/selected
                if i.key_pressed(egui::Key::D)
                    && !i.modifiers.ctrl
                    && !i.modifiers.command
                    && !i.modifiers.alt
                {
                    let mut deleted = false;
                    if self.selected_paths.is_empty() {
                        if let Some(item) = nav_entries.get(self.focused_row) {
                            log_result("Delete", &item.path, remove_entry_recursive(&item.path));
                            deleted = true;
                        }
                    } else {
                        for path in &self.selected_paths {
                            log_result("Delete", path, remove_entry_recursive(path));
                            deleted = true;
                        }
                        self.selected_paths.clear();
                    }
                    if deleted {
                        self.navigate_to(self.current_dir.clone());
                    }
                }

                // Vim `r`: In-place Rename
                if i.key_pressed(egui::Key::R)
                    && !i.modifiers.ctrl
                    && !i.modifiers.command
                    && !i.modifiers.alt
                {
                    if let Some(item) = nav_entries.get(self.focused_row) {
                        self.renaming_path = Some((item.path.clone(), item.name.clone()));
                    }
                }

                // Space: Quick Look / Peek Toggle
                if i.key_pressed(egui::Key::Space) {
                    if self.peek_item.is_some() {
                        self.peek_item = None;
                        self.peek_texture = None;
                        self.peek_text_cache = None;
                        self.peek_hash_cache = None;
                    } else if let Some(item) = nav_entries.get(self.focused_row) {
                        self.peek_item = Some(item.clone());
                    }
                }

                // Alt+Enter / Cmd+I: Properties Dialog
                if ((i.modifiers.alt && i.key_pressed(egui::Key::Enter))
                    || (i.modifiers.command && i.key_pressed(egui::Key::I)))
                    && !is_typing
                {
                    if let Some(item) = nav_entries.get(self.focused_row) {
                        self.properties_item = Some(item.clone());
                    }
                }
            }

            // Ctrl+H (Linux) / Cmd+Shift+. (macOS): Toggle Hidden Files
            if ((i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(egui::Key::H))
                || (i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::Period))
            {
                self.show_hidden_files = !self.show_hidden_files;
            }

            // Ctrl+Shift+T: Open Terminal in Directory
            if (i.modifiers.ctrl || i.modifiers.command)
                && i.modifiers.shift
                && i.key_pressed(egui::Key::T)
            {
                open_terminal_in_dir(&self.current_dir);
            }

            // Standard Explorer Hotkeys
            if i.key_pressed(egui::Key::F5) {
                self.navigate_to(self.current_dir.clone());
            }
            if i.modifiers.alt && i.key_pressed(egui::Key::ArrowRight) {
                self.go_forward();
            }
            if (i.modifiers.ctrl || i.modifiers.command)
                && i.modifiers.shift
                && i.key_pressed(egui::Key::N)
            {
                self.show_new_folder_dialog = true;
            }
            if i.key_pressed(egui::Key::F2) {
                if let Some(item) = nav_entries.get(self.focused_row) {
                    self.renaming_path = Some((item.path.clone(), item.name.clone()));
                }
            }
            if (i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(egui::Key::L) {
                self.path_edit_buffer = self.current_dir.to_string_lossy().to_string();
                self.is_editing_path = true;
            }
            if (i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(egui::Key::C) {
                if self.selected_paths.is_empty() {
                    if let Some(item) = nav_entries.get(self.focused_row) {
                        self.clipboard_paths = vec![item.path.clone()];
                    }
                } else {
                    self.clipboard_paths = self.selected_paths.iter().cloned().collect();
                }
                self.clipboard_is_cut = false;
            }
            if (i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(egui::Key::X) {
                if self.selected_paths.is_empty() {
                    if let Some(item) = nav_entries.get(self.focused_row) {
                        self.clipboard_paths = vec![item.path.clone()];
                    }
                } else {
                    self.clipboard_paths = self.selected_paths.iter().cloned().collect();
                }
                self.clipboard_is_cut = true;
            }
            if (i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(egui::Key::V) {
                let mut pasted = false;
                for src in &self.clipboard_paths {
                    if let Some(file_name) = src.file_name() {
                        let dest = self.current_dir.join(file_name);
                        if self.clipboard_is_cut {
                            log_result("Move", src, move_entry(src, &dest));
                        } else {
                            let unique = resolve_unique_paste_path(&dest);
                            log_result("Copy", src, copy_entry_recursive(src, &unique));
                        }
                        pasted = true;
                    }
                }
                if self.clipboard_is_cut {
                    self.clipboard_paths.clear();
                    self.clipboard_is_cut = false;
                }
                if pasted {
                    self.navigate_to(self.current_dir.clone());
                }
            }
            if i.key_pressed(egui::Key::Delete) {
                let mut deleted = false;
                if self.selected_paths.is_empty() {
                    if let Some(item) = nav_entries.get(self.focused_row) {
                        log_result("Delete", &item.path, remove_entry_recursive(&item.path));
                        deleted = true;
                    }
                } else {
                    for path in &self.selected_paths {
                        log_result("Delete", path, remove_entry_recursive(path));
                        deleted = true;
                    }
                    self.selected_paths.clear();
                }
                if deleted {
                    self.navigate_to(self.current_dir.clone());
                }
            }
        });

        // 3. New Folder Modal Dialog
        if self.show_new_folder_dialog {
            let mut close_dialog = false;
            let mut create_folder = false;

            egui::Window::new("📁 Create New Folder")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .frame(
                    egui::Frame::window(&ui.ctx().style())
                        .fill(palette.card_bg)
                        .stroke(Stroke::new(1.0_f32, palette.border))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(egui::Margin::same(16.0)),
                )
                .show(ui.ctx(), |ui| {
                    ui.label(
                        egui::RichText::new("Enter new folder name:").color(palette.text_primary),
                    );
                    ui.add_space(6.0);
                    let resp = ui.text_edit_singleline(&mut self.new_folder_buffer);
                    resp.request_focus();
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        create_folder = true;
                    }

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui
                            .button(egui::RichText::new("Create").strong().color(Color32::WHITE))
                            .clicked()
                        {
                            create_folder = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close_dialog = true;
                        }
                    });
                });

            if create_folder {
                let clean_name = self.new_folder_buffer.trim();
                if !clean_name.is_empty() {
                    let target_dir = self.current_dir.join(clean_name);
                    let _ = std::fs::create_dir_all(&target_dir);
                    self.navigate_to(self.current_dir.clone());
                }
                self.show_new_folder_dialog = false;
                self.new_folder_buffer = "New Folder".to_string();
            } else if close_dialog {
                self.show_new_folder_dialog = false;
            }
        }

        // 4. Rename Item Modal Dialog
        if let Some((old_path, mut new_name)) = self.renaming_path.clone() {
            let mut close_dialog = false;
            let mut do_rename = false;

            egui::Window::new("✏ Rename Item")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .frame(
                    egui::Frame::window(&ui.ctx().style())
                        .fill(palette.card_bg)
                        .stroke(Stroke::new(1.0_f32, palette.border))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(egui::Margin::same(16.0)),
                )
                .show(ui.ctx(), |ui| {
                    ui.label(egui::RichText::new("Enter new name:").color(palette.text_primary));
                    ui.add_space(6.0);
                    let resp = ui.text_edit_singleline(&mut new_name);
                    resp.request_focus();
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        do_rename = true;
                    }

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui
                            .button(egui::RichText::new("Rename").strong().color(Color32::WHITE))
                            .clicked()
                        {
                            do_rename = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close_dialog = true;
                        }
                    });
                });

            if do_rename {
                let trimmed = new_name.trim();
                if !trimmed.is_empty() {
                    if let Some(parent) = old_path.parent() {
                        let new_path = parent.join(trimmed);
                        let _ = std::fs::rename(&old_path, &new_path);
                        self.navigate_to(self.current_dir.clone());
                    }
                }
                self.renaming_path = None;
            } else if close_dialog {
                self.renaming_path = None;
            } else {
                self.renaming_path = Some((old_path, new_name));
            }
        }

        // 5. Explorer Layout: Full Width macOS Finder Canvas
        ui.vertical(|ui| {
            if self.show_internal_toolbar {
                // Top Navigation & Breadcrumbs Bar (macOS Toolbar style)
                self.render_breadcrumb_bar(ui, palette);
                ui.add_space(6.0);
            }

            if !self.is_home_view {
                // Category Filter Tabs Bar & View Mode Toggle (Segmented control)
                self.render_category_filter_bar(ui, palette);
                ui.add_space(6.0);
            }

            // File Table, Grid, or Miller Columns View
            let avail_h = (ui.available_height() - 28.0).max(120.0);
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), avail_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    self.render_file_view(
                        ui,
                        palette,
                        &mut on_stage_file,
                        &mut on_transcode_target,
                        &mut on_exam_conform,
                        &mut on_pack_archive,
                        &mut on_inspect_file,
                    );
                },
            );

            // macOS Finder Bottom Status Bar (Items, Available Space, Zoom)
            self.render_status_bar(ui, palette);
        });

        // 5. Modal Quick Look / Peek Display Overlay (macOS Finder style)
        self.render_peek_overlay(
            ui.ctx(),
            palette,
            &mut on_transcode_target,
            &mut on_exam_conform,
            &mut on_pack_archive,
            &mut on_inspect_file,
        );

        // 6. Properties Modal Overlay (Alt+Enter / Right-Click)
        self.render_properties_modal(ui.ctx(), palette);
    }

    fn render_breadcrumb_bar(&mut self, ui: &mut Ui, palette: &ThemePalette) {
        let avail_w = ui.available_width();
        let search_w = 185.0_f32;
        let is_compact = avail_w < 700.0;

        if !is_compact {
            ui.horizontal(|ui| {
                // Pin Search Box on the Right, give remaining left space to breadcrumb navigation
                let nav_w = (avail_w - search_w - 20.0).max(180.0);
                ui.allocate_ui_with_layout(
                    Vec2::new(nav_w, 28.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        self.render_breadcrumb_nav_buttons(ui, palette, nav_w);
                    },
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !self.search_query.is_empty()
                        && nav_icon_button(ui, IconGlyph::ActionClose, 16.0).on_hover_text("Clear filter (Esc)").clicked()
                    {
                        self.search_query.clear();
                    }
                    let box_w = if self.search_query.is_empty() {
                        search_w
                    } else {
                        search_w - 28.0
                    };
                    let search_resp = ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text("🔍 Filter files...")
                            .desired_width(box_w),
                    );
                    if self.focus_search_requested {
                        search_resp.request_focus();
                        self.focus_search_requested = false;
                    }
                });
            });
        } else {
            // Stacked layout for compact / mobile / narrow viewports
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    self.render_breadcrumb_nav_buttons(ui, palette, avail_w);
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if !self.search_query.is_empty()
                        && nav_icon_button(ui, IconGlyph::ActionClose, 16.0).on_hover_text("Clear filter (Esc)").clicked()
                    {
                        self.search_query.clear();
                    }
                    let box_w = if self.search_query.is_empty() {
                        avail_w
                    } else {
                        avail_w - 28.0
                    };
                    let search_resp = ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text("🔍 Filter files...")
                            .desired_width(box_w),
                    );
                    if self.focus_search_requested {
                        search_resp.request_focus();
                        self.focus_search_requested = false;
                    }
                });
            });
        }
    }

    fn render_breadcrumb_nav_buttons(&mut self, ui: &mut Ui, palette: &ThemePalette, max_w: f32) {
        // macOS Segmented Navigation Pill: Back & Forward (vector glyph buttons)
        let pack = self.icon_pack;
        let nav_tint = [
            palette.text_primary.r(),
            palette.text_primary.g(),
            palette.text_primary.b(),
        ];
        let can_back = self.can_go_back();
        let back_btn = match crate::core::icon_packs::icon_texture(
            ui.ctx(), pack, IconGlyph::ActionBack, 18.0, nav_tint,
        ) {
            Some(t) => egui::Button::image(egui::load::SizedTexture::new(t.id(), Vec2::splat(15.0))),
            None => egui::Button::new("Back"),
        }
        .fill(if can_back { palette.card_bg } else { Color32::TRANSPARENT })
        .rounding(Rounding {
            nw: 6.0,
            sw: 6.0,
            ne: 0.0,
            se: 0.0,
        });
        if ui
            .add_enabled(can_back, back_btn)
            .on_hover_text("Back (Alt+Left / h)")
            .clicked()
        {
            self.go_back();
        }

        let can_fwd = self.can_go_forward();
        let fwd_btn = match crate::core::icon_packs::icon_texture(
            ui.ctx(), pack, IconGlyph::ActionForward, 18.0, nav_tint,
        ) {
            Some(t) => egui::Button::image(egui::load::SizedTexture::new(t.id(), Vec2::splat(15.0))),
            None => egui::Button::new("Forward"),
        }
        .fill(if can_fwd { palette.card_bg } else { Color32::TRANSPARENT })
        .rounding(Rounding {
            nw: 0.0,
            sw: 0.0,
            ne: 6.0,
            se: 6.0,
        });
        if ui
            .add_enabled(can_fwd, fwd_btn)
            .on_hover_text("Forward (Alt+Right)")
            .clicked()
        {
            self.go_forward();
        }

        // Up Folder Button
        let up_btn = match crate::core::icon_packs::icon_texture(
            ui.ctx(), pack, IconGlyph::ActionUp, 18.0, nav_tint,
        ) {
            Some(t) => egui::Button::image(egui::load::SizedTexture::new(t.id(), Vec2::splat(15.0))),
            None => egui::Button::new("Up"),
        };
        if ui
            .add(up_btn)
            .on_hover_text("Parent Folder (Alt+Up / Backspace / h)")
            .clicked()
        {
            if let Some(parent) = self.current_dir.parent() {
                let p = parent.to_path_buf();
                if !p.as_os_str().is_empty() {
                    self.navigate_to(p);
                }
            }
        }

        // Refresh Button
        let refresh_btn = match crate::core::icon_packs::icon_texture(
            ui.ctx(), pack, IconGlyph::ActionRefresh, 18.0, nav_tint,
        ) {
            Some(t) => egui::Button::image(egui::load::SizedTexture::new(t.id(), Vec2::splat(15.0))),
            None => egui::Button::new("Refresh"),
        };
        if ui.add(refresh_btn).on_hover_text("Refresh directory (F5)").clicked() {
            self.navigate_to(self.current_dir.clone());
        }

        // New Folder Button
        let newfolder_btn = match crate::core::icon_packs::icon_texture(
            ui.ctx(), pack, IconGlyph::ActionNewFolder, 18.0, nav_tint,
        ) {
            Some(t) => egui::Button::image(egui::load::SizedTexture::new(t.id(), Vec2::splat(15.0))),
            None => egui::Button::new("New Folder"),
        };
        if ui
            .add(newfolder_btn)
            .on_hover_text("New Folder (Ctrl+Shift+N)")
            .clicked()
        {
            self.show_new_folder_dialog = true;
        }

        // Toggle Hidden Dotfiles Button
        let dot_bg = if self.show_hidden_files {
            palette.accent
        } else {
            palette.card_bg
        };
        let dot_fg = if self.show_hidden_files {
            Color32::WHITE
        } else {
            palette.text_secondary
        };
        let dot_btn = egui::Button::new(egui::RichText::new(".dotfiles").size(15.0).color(dot_fg))
            .fill(dot_bg)
            .rounding(Rounding::same(6.0));
        if ui
            .add(dot_btn)
            .on_hover_text("Toggle Hidden / Dotfiles (Ctrl+H / Cmd+Shift+.)")
            .clicked()
        {
            self.show_hidden_files = !self.show_hidden_files;
        }

        ui.separator();

        if self.is_editing_path {
            let edit_w = (max_w - 220.0).max(120.0);
            let text_resp = ui.add(
                egui::TextEdit::singleline(&mut self.path_edit_buffer)
                    .desired_width(edit_w)
                    .font(egui::TextStyle::Monospace),
            );
            if text_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let new_p = PathBuf::from(&self.path_edit_buffer);
                if new_p.exists() {
                    self.navigate_to(new_p);
                }
                self.is_editing_path = false;
            }
            if ui.button("Cancel").clicked() {
                self.is_editing_path = false;
            }
        } else {
            // Interactive Breadcrumb Segments
            let segments = parse_breadcrumbs(&self.current_dir);
            let seg_count = segments.len();
            let (to_skip, has_ellipsis) = if seg_count > 4 && max_w < 550.0 {
                (seg_count - 3, true)
            } else {
                (0, false)
            };

            if has_ellipsis {
                if ui
                    .button("…")
                    .on_hover_text("Intermediate folders")
                    .clicked()
                {
                    if let Some(first) = segments.first() {
                        self.navigate_to(first.target_path.clone());
                    }
                }
                ui.label(egui::RichText::new("›").color(palette.text_muted));
            }

            for seg in segments.into_iter().skip(to_skip) {
                let is_current = seg.target_path == self.current_dir;
                let btn = egui::Button::new(egui::RichText::new(&seg.label).strong().color(
                    if is_current {
                        palette.accent
                    } else {
                        palette.text_primary
                    },
                ))
                .fill(if is_current {
                    palette.card_hover
                } else {
                    palette.card_bg
                })
                .rounding(Rounding::same(6.0));

                if ui.add(btn).clicked() {
                    self.navigate_to(seg.target_path);
                }
                ui.label(egui::RichText::new("›").color(palette.text_muted));
            }

            // Edit Path button
            if ui
                .button("✏")
                .on_hover_text("Edit path directly (Ctrl+L / Alt+D)")
                .clicked()
            {
                self.path_edit_buffer = self.current_dir.to_string_lossy().to_string();
                self.is_editing_path = true;
            }
        }
    }

    fn render_category_filter_bar(&mut self, ui: &mut Ui, palette: &ThemePalette) {
        let avail_w = ui.available_width();
        let is_grid = self.view_mode == ViewMode::Grid;
        let toggle_w = if is_grid { 290.0_f32 } else { 210.0_f32 };
        let pills_w = (avail_w - toggle_w - 12.0).max(180.0);

        ui.horizontal(|ui| {
            // Left Container: Category Tabs with Wrapping
            ui.allocate_ui_with_layout(
                Vec2::new(pills_w, 28.0),
                egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(true),
                |ui| {
                    if self.is_home_directory() {
                        let btn_dash = egui::Button::new(
                            egui::RichText::new("🏠 Home Dashboard")
                                .size(15.0)
                                .strong()
                                .color(if self.is_home_view {
                                    Color32::WHITE
                                } else {
                                    palette.text_secondary
                                }),
                        )
                        .fill(if self.is_home_view {
                            palette.accent
                        } else {
                            palette.card_bg
                        })
                        .rounding(Rounding::same(6.0));
                        if ui
                            .add(btn_dash)
                            .on_hover_text("Show Quick Access & System Drives Dashboard")
                            .clicked()
                        {
                            self.is_home_view = true;
                        }

                        let btn_files = egui::Button::new(
                            egui::RichText::new("📂 Folder Files")
                                .size(15.0)
                                .strong()
                                .color(if !self.is_home_view {
                                    Color32::WHITE
                                } else {
                                    palette.text_secondary
                                }),
                        )
                        .fill(if !self.is_home_view {
                            palette.accent
                        } else {
                            palette.card_bg
                        })
                        .rounding(Rounding::same(6.0));
                        if ui
                            .add(btn_files)
                            .on_hover_text("Browse all raw files in folder")
                            .clicked()
                        {
                            self.is_home_view = false;
                        }

                        ui.separator();
                    }

                    let categories = [
                        CategoryFilter::All,
                        CategoryFilter::Images,
                        CategoryFilter::Audio,
                        CategoryFilter::Video,
                        CategoryFilter::Documents,
                        CategoryFilter::Archives,
                        CategoryFilter::Code,
                    ];

                    for cat in categories {
                        let is_active = self.category_filter == cat;
                        let count = match cat {
                            CategoryFilter::All => self.entries.len(),
                            _ => self.count_category(cat),
                        };

                        let label_text = format!("{} ({})", cat.label(), count);
                        let btn = egui::Button::new(
                            egui::RichText::new(label_text).size(15.0).strong().color(
                                if is_active {
                                    Color32::WHITE
                                } else {
                                    palette.text_secondary
                                },
                            ),
                        )
                        .fill(if is_active {
                            palette.accent
                        } else {
                            palette.card_bg
                        })
                        .rounding(Rounding::same(6.0));

                        if ui.add(btn).clicked() {
                            self.category_filter = cat;
                        }
                    }
                },
            );

            // Right Pinned Container: macOS 3-Segmented Switcher (List / Icons / Columns) & Icon Size
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let switch_tint = [
                    palette.text_primary.r(),
                    palette.text_primary.g(),
                    palette.text_primary.b(),
                ];
                let glyph_button = |ui: &mut Ui,
                                    glyph: IconGlyph,
                                    fallback: &str,
                                    is_active: bool,
                                    rounding: egui::Rounding|
                      -> egui::Response {
                    let btn = match crate::core::icon_packs::icon_texture(
                        ui.ctx(), self.icon_pack, glyph, 18.0, switch_tint,
                    ) {
                        Some(t) => egui::Button::image(egui::load::SizedTexture::new(
                            t.id(),
                            Vec2::splat(15.0),
                        )), // icon-only segmented switcher (glyph is self-describing)
                        None => menu_fallback_button(fallback),
                    }
                    .fill(if is_active { palette.accent } else { palette.card_bg })
                    .rounding(rounding);
                    ui.add(btn).on_hover_text(match glyph {
                        IconGlyph::ActionColumns => "Miller Columns View",
                        IconGlyph::ActionList => "Details Table View",
                        _ => "Grid / Tiles View",
                    })
                };

                let is_columns = self.view_mode == ViewMode::Columns;
                if glyph_button(
                    ui,
                    IconGlyph::ActionColumns,
                    "Columns",
                    is_columns,
                    Rounding { nw: 0.0, sw: 0.0, ne: 6.0, se: 6.0 },
                )
                .clicked()
                {
                    self.view_mode = ViewMode::Columns;
                }

                let is_grid = self.view_mode == ViewMode::Grid;
                if glyph_button(
                    ui,
                    IconGlyph::ActionView,
                    "Icons",
                    is_grid,
                    Rounding::ZERO,
                )
                .clicked()
                {
                    self.view_mode = ViewMode::Grid;
                }

                let is_details = self.view_mode == ViewMode::Details;
                if glyph_button(
                    ui,
                    IconGlyph::ActionList,
                    "List",
                    is_details,
                    Rounding { nw: 6.0, sw: 6.0, ne: 0.0, se: 0.0 },
                )
                .clicked()
                {
                    self.view_mode = ViewMode::Details;
                }

                if is_grid {
                    ui.add_space(4.0);
                    ui.add_sized(
                        [75.0, 18.0],
                        egui::Slider::new(&mut self.icon_size, 64.0..=160.0).show_value(false),
                    );
                    ui.label(
                        egui::RichText::new(format!("{:.0}px", self.icon_size))
                            .size(14.0)
                            .color(palette.text_muted),
                    );
                }
            });
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn render_file_view(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        on_stage_file: &mut impl FnMut(PathBuf),
        on_transcode_target: &mut impl FnMut(PathBuf, String),
        on_exam_conform: &mut impl FnMut(PathBuf),
        on_pack_archive: &mut impl FnMut(Vec<PathBuf>),
        on_inspect_file: &mut impl FnMut(PathBuf),
    ) {
        if self.is_home_view {
            self.render_home_dashboard(
                ui,
                palette,
                on_stage_file,
                on_transcode_target,
                on_exam_conform,
                on_pack_archive,
                on_inspect_file,
            );
            return;
        }

        match self.view_mode {
            ViewMode::Details => self.render_details_table(
                ui,
                palette,
                on_stage_file,
                on_transcode_target,
                on_exam_conform,
                on_pack_archive,
                on_inspect_file,
            ),
            ViewMode::Grid => self.render_grid_view(
                ui,
                palette,
                on_stage_file,
                on_transcode_target,
                on_exam_conform,
                on_pack_archive,
                on_inspect_file,
            ),
            ViewMode::Columns => self.render_columns_view(
                ui,
                palette,
                on_stage_file,
                on_transcode_target,
                on_exam_conform,
                on_pack_archive,
                on_inspect_file,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_home_dashboard(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        _on_stage_file: &mut impl FnMut(PathBuf),
        _on_transcode_target: &mut impl FnMut(PathBuf, String),
        _on_exam_conform: &mut impl FnMut(PathBuf),
        _on_pack_archive: &mut impl FnMut(Vec<PathBuf>),
        _on_inspect_file: &mut impl FnMut(PathBuf),
    ) {
        let home_opt = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .ok()
            .map(PathBuf::from);
        let base = home_opt.unwrap_or_else(|| PathBuf::from("."));

        let mut nav_to: Option<PathBuf> = None;
        let mut new_dash_hover: Option<(FileItem, Pos2)> = None;

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.add_space(8.0);

                // ==========================================
                // 1. Quick access Header & Cards Grid
                // ==========================================
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("⌃").size(16.0).color(palette.text_muted));
                    ui.label(egui::RichText::new("Quick access").size(16.5).strong().color(palette.text_primary));
                });
                ui.add_space(8.0);

                let mut pinned_folders = vec![
                    ("Desktop".to_string(), "🖥", [0, 122, 255], base.join("Desktop")),
                    ("Downloads".to_string(), "📥", [20, 184, 166], base.join("Downloads")),
                    ("Documents".to_string(), "📄", [59, 130, 246], base.join("Documents")),
                    ("Pictures".to_string(), "🖼", [14, 165, 233], base.join("Pictures")),
                    ("Music".to_string(), "🎵", [244, 63, 94], base.join("Music")),
                    ("Videos".to_string(), "🎬", [139, 92, 246], base.join("Videos")),
                ];

                // Append custom subdirectories found in home folder (like between_pcs, wallpapers)
                for entry in &self.entries {
                    if entry.is_dir && !entry.is_hidden {
                        let lower = entry.name.to_lowercase();
                        if !["desktop", "downloads", "documents", "pictures", "music", "videos", "appdata", "contacts", "searches", "saved games", "links"].contains(&lower.as_str()) {
                            pinned_folders.push((entry.name.clone(), "📁", [235, 175, 60], entry.path.clone()));
                        }
                    }
                }

                // Append Recycle Bin -> real system-drive $Recycle.Bin when available
                let recycle_target = self
                    .drives
                    .iter()
                    .find(|d| d.is_system)
                    .map(|d| d.path.join("$Recycle.Bin"))
                    .unwrap_or(base.clone());
                pinned_folders.push((
                    "Recycle Bin".to_string(),
                    "🗑",
                    [16, 185, 129],
                    recycle_target,
                ));

                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(12.0, 12.0);

                    for (label, _glyph, rgb, path) in &pinned_folders {
                        let display_label = truncate_end(label, 14);
                        let card_size = Vec2::new(104.0, 78.0);
                        let (rect, resp) = ui.allocate_exact_size(card_size, egui::Sense::click());

                        let is_hovered = resp.hovered();
                        let bg = if is_hovered {
                            palette.card_hover
                        } else {
                            palette.card_bg
                        };
                        let border_stroke = Stroke::new(
                            1.0_f32,
                            if is_hovered { palette.accent } else { palette.border },
                        );

                        // Card frame with subtle depth
                        ui.painter().rect_filled(rect, Rounding::same(8.0), bg);
                        ui.painter().rect_stroke(rect, Rounding::same(8.0), border_stroke);

                        // Large squircle-less pack icon (crisp, antialiased)
                        let icon_key = match label.as_str() {
                            "Desktop" => crate::core::icon_packs::IconGlyph::FolderDesktop,
                            "Downloads" => crate::core::icon_packs::IconGlyph::FolderDownloads,
                            "Documents" => crate::core::icon_packs::IconGlyph::FolderDocuments,
                            "Pictures" => crate::core::icon_packs::IconGlyph::FolderPictures,
                            "Music" => crate::core::icon_packs::IconGlyph::FolderMusic,
                            "Videos" => crate::core::icon_packs::IconGlyph::FolderVideos,
                            "Recycle Bin" => crate::core::icon_packs::IconGlyph::RecycleBin,
                            _ => crate::core::icon_packs::IconGlyph::Folder,
                        };
                        let badge_center = Pos2::new(rect.center().x, rect.top() + 40.0);
                        let big_rect = Rect::from_center_size(badge_center, Vec2::splat(42.0));
                        crate::core::icon_packs::paint_icon(
                            ui,
                            self.icon_pack,
                            icon_key,
                            big_rect,
                            [rgb[0], rgb[1], rgb[2]],
                        );

                        // Label below badge
                        ui.painter().text(
                            Pos2::new(rect.center().x, rect.bottom() - 12.0),
                            egui::Align2::CENTER_CENTER,
                            display_label,
                            egui::FontId::proportional(14.5),
                            palette.text_primary,
                        );

                        // Full name tooltip + hover preview anchor for long hover
                        if resp.hovered() {
                            let card_anchor =
                                rect.center() + Vec2::new(0.0, rect.height() * 0.5 + 14.0);
                            new_dash_hover = Some((
                                FileItem::from_path(path.clone()),
                                card_anchor,
                            ));
                        }

                        if resp.clicked() || resp.double_clicked() {
                            nav_to = Some(path.clone());
                        }
                    }
                });

                ui.add_space(20.0);

                // ==========================================
                // 2. Drives Header & Storage Meter Cards
                // ==========================================
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("⌃").size(16.0).color(palette.text_muted));
                    ui.label(egui::RichText::new("Drives").size(16.5).strong().color(palette.text_primary));
                });
                ui.add_space(8.0);

                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(14.0, 12.0);

                    for drive in &self.drives {
                        let card_size = Vec2::new(230.0, 70.0);
                        let (rect, resp) = ui.allocate_exact_size(card_size, egui::Sense::click());

                        let is_hovered = resp.hovered();
                        let bg = if is_hovered {
                            palette.card_hover
                        } else {
                            palette.card_bg
                        };
                        let border_stroke = Stroke::new(
                            1.0_f32,
                            if is_hovered { palette.accent } else { palette.border },
                        );

                        ui.painter().rect_filled(rect, Rounding::same(8.0), bg);
                        ui.painter().rect_stroke(rect, Rounding::same(8.0), border_stroke);

                        // Left: Vector drive icon
                        let badge_center = Pos2::new(rect.left() + 26.0, rect.center().y);
                        let d_glyph = if drive.is_system {
                            crate::core::icon_packs::IconGlyph::DriveSystem
                        } else {
                            crate::core::icon_packs::IconGlyph::Drive
                        };
                        let big_rect = Rect::from_center_size(badge_center, Vec2::splat(30.0));
                        let d_rgb: [u8; 3] = if drive.is_system { [0, 122, 255] } else { [100, 116, 139] };
                        crate::core::icon_packs::paint_icon(ui, self.icon_pack, d_glyph, big_rect, d_rgb);

                        // Right: Title, Progress Bar, and Space details
                        let text_left = rect.left() + 52.0;
                        ui.painter().text(
                            Pos2::new(text_left, rect.top() + 14.0),
                            egui::Align2::LEFT_TOP,
                            &drive.display_name,
                            egui::FontId::proportional(15.5),
                            palette.text_primary,
                        );

                        // Usage progress bar
                        let bar_y = rect.top() + 34.0;
                        let bar_w = 160.0_f32;
                        let bar_h = 4.5_f32;
                        let bar_rect = Rect::from_min_size(Pos2::new(text_left, bar_y), Vec2::new(bar_w, bar_h));
                        ui.painter().rect_filled(
                            bar_rect,
                            Rounding::same(2.0),
                            Color32::from_rgba_premultiplied(palette.border.r(), palette.border.g(), palette.border.b(), 120),
                        );
                        let fill_w = (bar_w * drive.usage_ratio().clamp(0.0, 1.0)).max(2.0);
                        let fill_rect = Rect::from_min_size(Pos2::new(text_left, bar_y), Vec2::new(fill_w, bar_h));
                        ui.painter().rect_filled(fill_rect, Rounding::same(2.0), palette.accent);

                        // Storage text
                        ui.painter().text(
                            Pos2::new(text_left, rect.top() + 44.0),
                            egui::Align2::LEFT_TOP,
                            drive.formatted_space(),
                            egui::FontId::proportional(13.5),
                            palette.text_secondary,
                        );

                        if resp.clicked() || resp.double_clicked() {
                            nav_to = Some(drive.path.clone());
                        }
                    }
                });

                ui.add_space(20.0);

                // ==========================================
                // 3. Network locations
                // ==========================================
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("⌃").size(16.0).color(palette.text_muted));
                    ui.label(egui::RichText::new("Network locations").size(16.5).strong().color(palette.text_primary));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new("⋯").size(17.0).color(palette.text_muted));
                    });
                });
                ui.add_space(6.0);

                let banner_h = 42.0;
                let banner_w = (ui.available_width() - 14.0).max(200.0);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(banner_w, banner_h), egui::Sense::hover());
                ui.painter().rect_filled(rect, Rounding::same(6.0), palette.card_bg);
                ui.painter().rect_stroke(rect, Rounding::same(6.0), Stroke::new(1.0_f32, palette.border));

                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        let info_badge_center = Pos2::new(ui.cursor().min.x + 8.0, rect.center().y);
                        ui.painter().circle_filled(info_badge_center, 8.0, palette.accent);
                        ui.painter().text(
                            info_badge_center,
                            egui::Align2::CENTER_CENTER,
                            "ℹ",
                            egui::FontId::proportional(14.5),
                            Color32::WHITE,
                        );
                        ui.add_space(24.0);
                        ui.label(
                            egui::RichText::new("There are no network locations. If you don't use network locations, you can disable the widget.")
                                .size(15.0)
                                .color(palette.text_secondary)
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(12.0);
                            let dis_btn = egui::Button::new(egui::RichText::new("Disable").size(15.0).color(palette.text_primary))
                                .fill(palette.card_hover)
                                .rounding(Rounding::same(4.0));
                            let _ = ui.add(dis_btn).on_hover_text("Disable network location discovery");
                        });
                    });
                });

                ui.add_space(20.0);

                // ==========================================
                // 4. Recent Files List
                // ==========================================
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("⌄").size(16.0).color(palette.text_muted));
                    ui.label(egui::RichText::new("Recent files").size(16.5).strong().color(palette.text_primary));
                });
                ui.add_space(8.0);

                let recent_files: Vec<&FileItem> = self.entries.iter().filter(|e| !e.is_dir).take(15).collect();
                if recent_files.is_empty() {
                    ui.label(egui::RichText::new("No recent standalone files in home directory. Choose a folder above to browse.").size(14.5).color(palette.text_muted));
                } else {
                    for item in recent_files {
                        let row_h = 24.0;
                        let (row_rect, row_resp) = ui.allocate_exact_size(Vec2::new(ui.available_width() - 14.0, row_h), egui::Sense::click());

                        if row_resp.hovered() {
                            ui.painter().rect_filled(row_rect, Rounding::same(4.0), palette.card_hover);
                        }

                        let icon_spec = resolve_file_icon(&item.name, &item.extension, item.is_dir);
                        let (glyph_ast, accent_rgb) = crate::core::icon_packs::IconGlyph::resolve(
                            &item.name,
                            &item.extension,
                            item.is_dir,
                            icon_spec.is_image,
                            icon_spec.is_media,
                            icon_spec.is_archive,
                            icon_spec.is_document,
                            icon_spec.is_code,
                            icon_spec.color_rgb,
                        );
                        let icon_rect = egui::Rect::from_center_size(
                            Pos2::new(row_rect.left() + 14.0, row_rect.center().y),
                            Vec2::splat(18.0),
                        );
                        crate::core::icon_packs::paint_icon(ui, self.icon_pack, glyph_ast, icon_rect, accent_rgb);

                        // File name
                        ui.painter().text(
                            Pos2::new(row_rect.left() + 32.0, row_rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            &item.name,
                            egui::FontId::proportional(15.0),
                            palette.text_primary,
                        );

                        // Size and Modified date on right
                        ui.painter().text(
                            Pos2::new(row_rect.right() - 12.0, row_rect.center().y),
                            egui::Align2::RIGHT_CENTER,
                            item.formatted_size(),
                            egui::FontId::monospace(14.0),
                            palette.text_muted,
                        );

                        if row_resp.double_clicked() {
                            open_in_default_app(&item.path);
                        }
                    }
                }
            });

        if let Some(path) = nav_to {
            self.navigate_to(path);
        }

        // Long-hover preview on dashboard cards (folder contents peek)
        let keep_existing = matches!(
            (&self.hover_preview, &new_dash_hover),
            (Some(active), Some((item, _))) if active.item.path == item.path
        );
        if keep_existing {
            let due = self
                .hover_preview
                .as_ref()
                .map(|h| h.started_at.elapsed() > std::time::Duration::from_millis(450))
                .unwrap_or(false);
            if due {
                self.ensure_hover_content(ui.ctx());
            }
        } else if let Some((item, anchor)) = new_dash_hover {
            self.hover_preview = Some(HoverPreviewState {
                item,
                anchor,
                started_at: Instant::now(),
                content: HoverContent::Meta,
            });
        } else {
            self.hover_preview = None;
        }
        self.render_hover_preview(ui, palette);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_details_table(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        _on_stage_file: &mut impl FnMut(PathBuf),
        on_transcode_target: &mut impl FnMut(PathBuf, String),
        on_exam_conform: &mut impl FnMut(PathBuf),
        on_pack_archive: &mut impl FnMut(Vec<PathBuf>),
        on_inspect_file: &mut impl FnMut(PathBuf),
    ) {
        let entries: Vec<FileItem> = self
            .filtered_and_sorted_entries()
            .into_iter()
            .cloned()
            .collect();

        // ── Tools & Converters toolbar (sits directly above the parameter row) ──
        self.render_action_toolbar(
            ui,
            palette,
            &entries,
            on_transcode_target,
            on_exam_conform,
            on_pack_archive,
            on_inspect_file,
        );

        // Finder-style column metrics: [checkbox] Name | Kind | Size | Date Modified
        let scrollbar_reserve = 12.0;
        let content_width = (ui.available_width() - scrollbar_reserve).max(320.0);
        let kind_w = 118.0_f32;
        let size_w = 92.0_f32;
        let date_w = 138.0_f32;
        let col_pad = 6.0_f32;
        // Checkbox door is reserved for Always and Hover modes alike (stable
        // layout, no jump on reveal); Off collapses it entirely.
        let checkbox_w = match self.checkbox_mode {
            CheckboxMode::Off => 0.0_f32,
            _ => 30.0_f32,
        };
        let name_w = (content_width - checkbox_w - kind_w - size_w - date_w).max(220.0);

        // Column header: flat interactive labels, no button boxes, hairline underneath
        let header_h = 26.0_f32;
        let (header_rect, _) =
            ui.allocate_exact_size(Vec2::new(content_width, header_h), egui::Sense::hover());
        let hp = ui.painter_at(header_rect);

        // Column rects (name starts with left inset so it aligns with row glyphs)
        let cols = {
            let x0 = header_rect.left() + checkbox_w;
            let y0 = header_rect.top();
            (
                Rect::from_min_size(Pos2::new(x0 + col_pad, y0), Vec2::new(name_w, header_h)),
                Rect::from_min_size(Pos2::new(x0 + name_w, y0), Vec2::new(kind_w, header_h)),
                Rect::from_min_size(
                    Pos2::new(x0 + name_w + kind_w, y0),
                    Vec2::new(size_w, header_h),
                ),
                Rect::from_min_size(
                    Pos2::new(x0 + name_w + kind_w + size_w, y0),
                    Vec2::new(date_w + col_pad, header_h),
                ),
            )
        };

        let mut header_sort: Option<SortColumn> = None;
        let header_cells: [(&str, Rect, SortColumn, bool); 4] = [
            ("Name", cols.0, SortColumn::Name, false),
            ("Kind", cols.1, SortColumn::Extension, false),
            ("Size", cols.2, SortColumn::Size, true),
            ("Date Modified", cols.3, SortColumn::Date, true),
        ];

        for (label, hrect, col, right_align) in header_cells {
            let is_active = self.sort_column == col;
            let id = ui.make_persistent_id(("hdr", col));
            let resp = ui.interact(hrect, id, egui::Sense::click());
            if resp.hovered() {
                hp.rect_filled(
                    hrect.shrink2(Vec2::new(0.0, 3.0)),
                    Rounding::same(4.0),
                    Color32::from_rgba_premultiplied(
                        palette.card_hover.r(),
                        palette.card_hover.g(),
                        palette.card_hover.b(),
                        160,
                    ),
                );
            }
            let arrow = if is_active {
                format!(" {}", self.sort_direction.indicator())
            } else {
                String::new()
            };
            let align = if right_align {
                egui::Align2::RIGHT_CENTER
            } else {
                egui::Align2::LEFT_CENTER
            };
            let tx = if right_align {
                hrect.right() - col_pad
            } else {
                hrect.left() + col_pad
            };
            hp.text(
                Pos2::new(tx, hrect.center().y),
                align,
                format!("{}{}", label, arrow),
                egui::FontId::proportional(15.5),
                if is_active { palette.accent } else { palette.text_muted },
            );
            if resp.clicked() {
                header_sort = Some(col);
            }
        }

        // Hairline under the header (Finder table rule)
        hp.line_segment(
            [
                Pos2::new(header_rect.left(), header_rect.bottom()),
                Pos2::new(header_rect.right(), header_rect.bottom()),
            ],
            Stroke::new(1.0_f32, palette.border),
        );

        if let Some(col) = header_sort {
            if self.sort_column == col {
                self.sort_direction = self.sort_direction.toggle();
            } else {
                self.sort_column = col;
                self.sort_direction = if col == SortColumn::Name {
                    SortDirection::Ascending
                } else if col == SortColumn::Date {
                    SortDirection::Descending
                } else {
                    SortDirection::Ascending
                };
            }
        }

        ui.add_space(2.0);

        let mut nav_into_dir: Option<PathBuf> = None;
        let mut peek_to_open: Option<FileItem> = None;
        let mut item_intents: Vec<ItemMenuIntents> = Vec::new();
        let mut canvas_intents = CanvasMenuIntents::default();
        let mut new_hover: Option<(FileItem, Pos2)> = None;
        let thumb_budget_reset = self.thumb_decode_budget.1.elapsed() > std::time::Duration::from_millis(300);
        if thumb_budget_reset {
            self.thumb_decode_budget = (0, Instant::now());
        }

        let icon_pack = self.icon_pack;
        let menu_style = self.menu_style;
        let has_clipboard = !self.clipboard_paths.is_empty();
        let tint = palette.accent;

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                // Canvas interact registered FIRST so rows (registered after it) stay
                // on top: egui gives the pointer to the last-registered widget covering
                // it. This prevents the empty-canvas menu from swallowing row clicks —
                // the long-standing "click on a file does nothing" bug.
                let canvas_resp = ui.interact(
                    ui.available_rect_before_wrap(),
                    ui.make_persistent_id("explorer_canvas_bg"),
                    egui::Sense::click(),
                );
                canvas_resp.context_menu(|menu_ui| {
                    canvas_intents = Self::run_canvas_context_menu(
                        menu_ui,
                        icon_pack,
                        menu_style,
                        palette.accent,
                        has_clipboard,
                        !entries.is_empty(),
                    );
                });

                ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);

                for (idx, item) in entries.iter().enumerate() {
                    let is_selected = self.selected_paths.contains(&item.path);
                    let row_height = 36.0_f32;

                    let (row_rect, row_resp) = ui.allocate_exact_size(
                        Vec2::new(content_width, row_height),
                        egui::Sense::click(),
                    );

                    // Selection disclosure is the checkbox ONLY (never a row band).
                    // Hover/Always reveal drives the alpha with the pack's spring.
                    if checkbox_w > 0.5 {
                        let cb_rect = Rect::from_center_size(
                            Pos2::new(
                                row_rect.left() + checkbox_w * 0.5,
                                row_rect.center().y,
                            ),
                            Vec2::splat(18.0_f32.min(checkbox_w - 12.0)),
                        );
                        let cb_resp = ui.interact(
                            cb_rect,
                            ui.make_persistent_id(("sel_box_row", &item.path)),
                            egui::Sense::click(),
                        );
                        let reveal = row_resp.hovered()
                            || cb_resp.hovered()
                            || is_selected
                            || self.checkbox_mode == CheckboxMode::Always;
                        let chk_alpha = self.hover_anims.drive(
                            crate::gui::anim::path_id(&item.path),
                            if reveal { 1.0 } else { 0.0 },
                            self.animations.hover,
                            crate::gui::anim::frame_dt(ui.ctx()),
                        );
                        if self.checkbox_mode == CheckboxMode::Always || chk_alpha > 0.02 {
                            Self::paint_selection_checkbox(
                                ui, cb_rect, is_selected, chk_alpha, palette,
                            );
                        }
                        if cb_resp.clicked() {
                            if is_selected {
                                self.selected_paths.remove(&item.path);
                            } else {
                                self.selected_paths.insert(item.path.clone());
                            }
                            self.selection_anchor = Some(idx);
                        }
                    }

                    let icon_spec = resolve_file_icon(&item.name, &item.extension, item.is_dir);
                    let (icon_glyph, icon_accent) = crate::core::icon_packs::IconGlyph::resolve(
                        &item.name,
                        &item.extension,
                        item.is_dir,
                        icon_spec.is_image,
                        icon_spec.is_media,
                        icon_spec.is_archive,
                        icon_spec.is_document,
                        icon_spec.is_code,
                        icon_spec.color_rgb,
                    );

                    // 1. Name cell: icon + name (right-truncated, Finder semantics)
                    let name_x = row_rect.left() + col_pad + checkbox_w;
                    let icon_rect = Rect::from_center_size(
                        Pos2::new(name_x + 8.0 + 16.0, row_rect.center().y),
                        Vec2::splat(32.0),
                    );
                    crate::core::icon_packs::paint_icon(
                        ui,
                        self.icon_pack,
                        icon_glyph,
                        icon_rect,
                        icon_accent,
                    );
                    // High-contrast text: selection is disclosed by the checkbox,
                    // so the name always sits on the normal fill (Finder detail).
                    let mut name_color = palette.text_primary;
                    if item.is_hidden {
                        name_color = Color32::from_rgba_premultiplied(
                            name_color.r(),
                            name_color.g(),
                            name_color.b(),
                            130,
                        );
                    }
                    let name_font = egui::FontId::proportional(if item.is_dir { 16.0 } else { 15.5 });
                    let name_pos_x = name_x + 48.0;
                    let name_avail = name_w - 54.0 - col_pad;
                    let suffix = if item.is_symlink { " ↗" } else { "" };
                    let display = truncate_middle(
                        &format!("{}{}", item.name, suffix),
                        (name_avail / 7.0) as usize,
                    );
                    let rp = ui.painter();
                    rp.text(
                        Pos2::new(name_pos_x, row_rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        display,
                        name_font,
                        name_color,
                    );

                    // 2. Kind cell (Finder "Kind": human category, dim)
                    let kind_rect = Rect::from_min_size(
                        Pos2::new(
                            row_rect.left() + checkbox_w + name_w,
                            row_rect.top(),
                        ),
                        Vec2::new(kind_w, row_height),
                    );
                    rp.text(
                        Pos2::new(kind_rect.left() + 2.0, kind_rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        icon_spec.category,
                        egui::FontId::proportional(14.5),
                        palette.text_secondary,
                    );

                    // 3. Size cell (right-aligned, monospace). Dirs report "--".
                    let size_rect = Rect::from_min_size(
                        Pos2::new(row_rect.left() + checkbox_w + name_w + kind_w, row_rect.top()),
                        Vec2::new(size_w, row_height),
                    );
                    let size_text = if item.is_dir {
                        String::from("--")
                    } else {
                        item.formatted_size()
                    };
                    rp.text(
                        Pos2::new(size_rect.right() - 4.0, size_rect.center().y),
                        egui::Align2::RIGHT_CENTER,
                        size_text.as_str(),
                        egui::FontId::monospace(14.5),
                        palette.text_secondary,
                    );

                    // 4. Date Modified cell (right-aligned, dim)
                    let date_rect = Rect::from_min_size(
                        Pos2::new(
                            row_rect.left() + checkbox_w + name_w + kind_w + size_w,
                            row_rect.top(),
                        ),
                        Vec2::new(date_w, row_height),
                    );
                    rp.text(
                        Pos2::new(date_rect.right() - col_pad, date_rect.center().y),
                        egui::Align2::RIGHT_CENTER,
                        item.formatted_date(),
                        egui::FontId::proportional(14.5),
                        palette.text_muted,
                    );

                    // ── Interaction: Finder semantics ──
                    if row_resp.double_clicked() {
                        self.selection_anchor = Some(idx);
                        if item.is_dir {
                            nav_into_dir = Some(item.path.clone());
                        } else {
                            open_in_default_app(&item.path);
                        }
                    } else if row_resp.clicked() {
                        self.focused_row = idx;
                        let ctrl = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
                        let shift = ui.input(|i| i.modifiers.shift);
                        if ctrl {
                            if is_selected {
                                self.selected_paths.remove(&item.path);
                            } else {
                                self.selected_paths.insert(item.path.clone());
                            }
                            self.selection_anchor = Some(idx);
                        } else if shift {
                            if let Some(anchor) = self.selection_anchor {
                                self.selected_paths.clear();
                                let (lo, hi) = (anchor.min(idx), anchor.max(idx));
                                for e in entries.iter().take(hi + 1).skip(lo) {
                                    self.selected_paths.insert(e.path.clone());
                                }
                            } else {
                                self.selected_paths.clear();
                                self.selected_paths.insert(item.path.clone());
                                self.selection_anchor = Some(idx);
                            }
                        } else {
                            self.selected_paths.clear();
                            self.selected_paths.insert(item.path.clone());
                            self.selection_anchor = Some(idx);
                        }
                    }

                    if row_resp.hovered() {
                        new_hover = Some((item.clone(), row_rect.right_center() + Vec2::new(14.0, 0.0)));
                    }

                    // Comprehensive Right-Click Context Menu for the File Row.
                    // Pure menu builder returns intents; mutations apply below.
                    row_resp.context_menu(|menu_ui| {
                        item_intents.clear();
                        item_intents.push(Self::run_item_context_menu(
                            menu_ui,
                            icon_pack,
                            menu_style,
                            tint,
                            item,
                        ));
                    });
                }
            });

        // (The empty-canvas context menu is registered FIRST inside the scroll
        // area above, so row interactions always win the pointer.)

        // Apply menu intents (mutations live OUTSIDE the popup closures).
        if canvas_intents.new_folder {
            self.show_new_folder_dialog = true;
        }
        if canvas_intents.refresh {
            self.navigate_to(self.current_dir.clone());
        }
        if canvas_intents.go_up {
            if let Some(parent) = self.current_dir.parent() {
                let p = parent.to_path_buf();
                if !p.as_os_str().is_empty() {
                    self.navigate_to(p);
                }
            }
        }
        if canvas_intents.select_all {
            for e in &entries {
                self.selected_paths.insert(e.path.clone());
            }
        }
        if canvas_intents.clear_selection {
            self.selected_paths.clear();
        }
        if canvas_intents.paste.is_some() {
            self.paste_from_clipboard();
        }
        if canvas_intents.terminal {
            open_terminal_in_dir(&self.current_dir);
        }
        if canvas_intents.open_in_file_manager {
            open_in_file_manager(&self.current_dir);
        }
        if canvas_intents.properties_dir {
            self.properties_item = Some(FileItem::from_path(self.current_dir.clone()));
        }
        for intent in item_intents {
            if let Some(dir) = intent.nav_into_dir {
                nav_into_dir = Some(dir);
            }
            if let Some(item) = intent.peek {
                peek_to_open = Some(item);
            }
            if let Some((path, fmt)) = intent.transcode {
                on_transcode_target(path, fmt);
            }
            if let Some(path) = intent.exam_conform {
                on_exam_conform(path);
            }
            if let Some(paths) = intent.pack {
                on_pack_archive(paths);
            }
            if let Some(path) = intent.inspect {
                on_inspect_file(path);
            }
            if let Some(path) = intent.extract {
                if let Some(parent) = path.parent() {
                    let parent = parent.to_path_buf();
                    log_result("Extract", &path, crate::archive::unpack_archive(&path, &parent));
                    self.navigate_to(self.current_dir.clone());
                }
            }
            if let Some((path, name)) = intent.rename {
                self.renaming_path = Some((path, name));
            }
            if let Some(item) = intent.properties {
                self.properties_item = Some(item);
            }
            if let Some(path) = intent.deleted {
                let _ = remove_entry_recursive(&path);
                self.selected_paths.remove(&path);
                self.navigate_to(self.current_dir.clone());
            }
            if let Some(paths) = intent.cut {
                self.clipboard_paths = paths.clone();
                self.clipboard_is_cut = true;
                crate::gui::context_menu::set_clipboard_files_with_effect(&paths, true);
            }
            if let Some(paths) = intent.copy {
                self.clipboard_paths = paths.clone();
                self.clipboard_is_cut = false;
                crate::gui::context_menu::set_clipboard_files_with_effect(&paths, false);
            }
        }

        // Apply hover preview state (Finder long-hover preview)
        let keep_existing = matches!(
            (&self.hover_preview, &new_hover),
            (Some(active), Some((item, _))) if active.item.path == item.path
        );
        if keep_existing {
            let due = self
                .hover_preview
                .as_ref()
                .map(|h| h.started_at.elapsed() > std::time::Duration::from_millis(450))
                .unwrap_or(false);
            if due {
                self.ensure_hover_content(ui.ctx());
            }
        } else if let Some((item, anchor)) = new_hover {
            self.hover_preview = Some(HoverPreviewState {
                item,
                anchor,
                started_at: Instant::now(),
                content: HoverContent::Meta,
            });
        } else {
            self.hover_preview = None;
        }
        self.render_hover_preview(ui, palette);

        if let Some(dir) = nav_into_dir {
            self.navigate_to(dir);
        }

        if let Some(item) = peek_to_open {
            self.peek_item = Some(item);
            self.peek_texture = None;
            self.peek_text_cache = None;
            self.peek_hash_cache = None;
        }
    }

    /// Finder-style Action Toolbar: Tools & Converters sitting directly above the
    /// Name / Kind / Size / Date parameter row. Ghost-pill styling, enabled by selection.
    #[allow(clippy::too_many_arguments)]
    fn render_action_toolbar(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        entries: &[FileItem],
        on_transcode_target: &mut impl FnMut(PathBuf, String),
        on_exam_conform: &mut impl FnMut(PathBuf),
        on_pack_archive: &mut impl FnMut(Vec<PathBuf>),
        on_inspect_file: &mut impl FnMut(PathBuf),
    ) {
        let bar_h = 34.0_f32;
        let (bar_rect, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), bar_h),
            egui::Sense::hover(),
        );

        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(bar_rect), |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);
                ui.add_space(2.0);

                let sel_files: Vec<PathBuf> = self
                    .selected_paths
                    .iter()
                    .filter(|p| p.is_file())
                    .cloned()
                    .collect();
                let all_sel: Vec<PathBuf> = self.selected_paths.iter().cloned().collect();
                let has_files = !sel_files.is_empty();
                let has_sel = !all_sel.is_empty();

                let pack = self.icon_pack;
                let tint_rgb = [
                    palette.text_secondary.r(),
                    palette.text_secondary.g(),
                    palette.text_secondary.b(),
                ];
                let pill = |ui: &mut Ui,
                            glyph: Option<IconGlyph>,
                            label: &str,
                            enabled: bool|
                      -> egui::Response {
                    let text = egui::RichText::new(label)
                        .size(15.0)
                        .color(if enabled {
                            palette.text_primary
                        } else {
                            palette.text_muted
                        });
                    let tex = glyph.and_then(|g| {
                        crate::core::icon_packs::icon_texture(ui.ctx(), pack, g, 16.0, tint_rgb)
                    });
                    let mut btn = match tex {
                        Some(t) => egui::Button::image_and_text(
                            egui::load::SizedTexture::new(t.id(), Vec2::splat(15.0)),
                            text,
                        ),
                        None => egui::Button::new(text),
                    };
                    btn = btn
                        .fill(if enabled { palette.card_bg } else { Color32::TRANSPARENT })
                        .rounding(Rounding::same(7.0));
                    ui.add_enabled(enabled, btn)
                };

                // ── CONVERT (Tools & Converters first group) ──
                if has_files {
                    ui.menu_button("Convert", |menu_ui| {
                        menu_ui.set_min_width(180.0);
                        for fmt in ["JPG", "PNG", "WEBP", "PDF", "MP3", "FLAC", "ZIP"] {
                            if fm::menu_item_labeled(
                                menu_ui,
                                pack,
                                fm::MenuAction::Transcode,
                                &format!("Convert to {fmt}"),
                                palette.accent,
                            )
                            .clicked()
                            {
                                for f in &sel_files {
                                    on_transcode_target(f.clone(), fmt.to_string());
                                }
                                menu_ui.close_menu();
                            }
                        }
                        menu_ui.separator();
                        if fm::menu_item_labeled(
                            menu_ui,
                            pack,
                            fm::MenuAction::Transcode,
                            "All formats (batch)",
                            palette.accent,
                        )
                        .clicked()
                        {
                            for f in &sel_files {
                                on_transcode_target(f.clone(), "ALL".into());
                            }
                            menu_ui.close_menu();
                        }
                    });
                } else {
                    let btn = egui::Button::new(
                        egui::RichText::new("Convert").size(15.0).color(palette.text_muted),
                    )
                    .fill(Color32::TRANSPARENT)
                    .rounding(Rounding::same(7.0));
                    let _ = ui.add_enabled(false, btn);
                }

                if pill(ui, Some(IconGlyph::ActionCheck), "Conform", has_files).clicked() {
                    for f in &sel_files {
                        on_exam_conform(f.clone());
                    }
                }
                if pill(ui, Some(IconGlyph::PageArchive), "Pack ZIP", has_sel).clicked() {
                    on_pack_archive(all_sel.clone());
                }
                if pill(ui, Some(IconGlyph::ActionShield), "Inspect", has_files).clicked() {
                    if let Some(f) = sel_files.first() {
                        on_inspect_file(f.clone());
                    }
                }

                ui.separator();

                // ── Clipboard & file ops ──
                if pill(ui, Some(IconGlyph::ActionCut), "Cut", has_sel).clicked() {
                    self.clipboard_paths = all_sel.clone();
                    self.clipboard_is_cut = true;
                    crate::gui::context_menu::set_clipboard_files_with_effect(&all_sel, true);
                }
                if pill(ui, Some(IconGlyph::ActionCopy), "Copy", has_sel).clicked() {
                    self.clipboard_paths = all_sel.clone();
                    self.clipboard_is_cut = false;
                    crate::gui::context_menu::set_clipboard_files_with_effect(&all_sel, false);
                }
                let paste_to = self.current_dir.clone();
                let clipboard = self.clipboard_paths.clone();
                let clipboard_is_cut = self.clipboard_is_cut;
                let can_paste = !clipboard.is_empty();
                if pill(ui, Some(IconGlyph::ActionPaste), "Paste", can_paste).clicked() {
                    for src in &clipboard {
                        if let Some(name) = src.file_name() {
                            let dest = paste_to.join(name);
                            if clipboard_is_cut {
                                log_result("Move", src, move_entry(src, &dest));
                            } else {
                                let unique = resolve_unique_paste_path(&dest);
                                log_result("Copy", src, copy_entry_recursive(src, &unique));
                            }
                        }
                    }
                    if clipboard_is_cut {
                        self.clipboard_paths.clear();
                        self.clipboard_is_cut = false;
                    }
                    let dir = paste_to.clone();
                    self.navigate_to(dir);
                }
                if pill(ui, Some(IconGlyph::ActionRename), "Rename", self.selected_paths.len() == 1).clicked() {
                    if let Some(p) = all_sel.first() {
                        if let Some(name) = p.file_name() {
                            self.renaming_path =
                                Some((p.clone(), name.to_string_lossy().to_string()));
                        }
                    }
                }
                if pill(ui, Some(IconGlyph::ActionDelete), "Delete", has_sel).clicked() {
                    for path in &all_sel {
                        log_result("Delete", path, remove_entry_recursive(path));
                    }
                    self.selected_paths.clear();
                    self.selection_anchor = None;
                    let dir = self.current_dir.clone();
                    self.navigate_to(dir);
                }

                // ── Right-side: selection summary + select all/clear ──
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(8.0);
                    if has_sel {
                        if pill(ui, Some(IconGlyph::ActionClose), "Clear", true).clicked() {
                            self.selected_paths.clear();
                            self.selection_anchor = None;
                        }
                    } else if pill(
                        ui,
                        Some(IconGlyph::ActionSelectAll),
                        "Select All",
                        !entries.is_empty(),
                    )
                    .clicked()
                    {
                        for e in entries {
                            self.selected_paths.insert(e.path.clone());
                        }
                        self.selection_anchor = None;
                    }

                    if has_sel {
                        let bytes: u64 = entries
                            .iter()
                            .filter(|e| self.selected_paths.contains(&e.path) && !e.is_dir)
                            .map(|e| e.size_bytes)
                            .sum();
                        let label = if bytes > 0 {
                            format!("{} selected • {}", all_sel.len(), format_file_size(bytes))
                        } else {
                            format!("{} selected", all_sel.len())
                        };
                        ui.label(
                            egui::RichText::new(label)
                                .size(14.5)
                                .strong()
                                .color(palette.accent),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new(format!("{} items enabled", entries.len()))
                                .size(14.5)
                                .color(palette.text_muted),
                        );
                    }
                });
            });
        });
        ui.add_space(2.0);
    }

    /// Lazily resolves hover-card content (folder children or image texture)
    fn ensure_hover_content(&mut self, ctx: &egui::Context) {
        let should_resolve = match self.hover_preview.as_ref() {
            Some(state) => matches!(state.content, HoverContent::Meta),
            None => false,
        };
        if !should_resolve {
            return;
        }
        let item = self.hover_preview.as_ref().unwrap().item.clone();

        if item.is_dir {
            let mut children = list_directory_sync(&item.path);
            if !self.show_hidden_files {
                children.retain(|c| !c.is_hidden);
            }
            children.sort_by(|a, b| {
                b.is_dir
                    .cmp(&a.is_dir)
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            });
            let total = children.len();
            children.truncate(10);
            if let Some(state) = self.hover_preview.as_mut() {
                state.content = HoverContent::Children(children, total);
            }
        } else if is_image_extension(&item.extension) {
            let tex = self.get_or_load_thumbnail(ctx, &item.path, 260);
            if tex.is_none() {
                ctx.request_repaint_after(std::time::Duration::from_millis(60));
            }
            if let Some(state) = self.hover_preview.as_mut() {
                if let Some(tex) = tex {
                    state.content = HoverContent::Image(tex);
                }
            }
        }
    }

    /// Floating Finder-style preview card: folder contents / image preview / metadata
    fn render_hover_preview(&mut self, ui: &Ui, palette: &ThemePalette) {
        let show = self
            .hover_preview
            .as_ref()
            .map(|h| h.started_at.elapsed() > std::time::Duration::from_millis(450))
            .unwrap_or(false);
        if !show {
            return;
        }

        let screen = ui.ctx().screen_rect();
        let card_w = 250.0_f32;
        let card_h = match self.hover_preview.as_ref().map(|h| &h.content) {
            Some(HoverContent::Children(_, _)) => 236.0,
            Some(HoverContent::Meta) => 116.0,
            Some(HoverContent::Image(tex)) => (tex.size()[1] as f32).clamp(120.0, 200.0) + 58.0,
            _ => 116.0,
        };
        let Some(state) = self.hover_preview.as_ref() else {
            return;
        };

        // Anchor placement: right side of row (or under a tile), clamped inside screen
        let x = if state.anchor.x + card_w > screen.right() - 8.0 {
            (state.anchor.x - card_w - 20.0).max(screen.left() + 8.0)
        } else {
            state.anchor.x
        };
        let y = state.anchor.y.min(screen.bottom() - card_h - 12.0);

        let item = state.item.clone();
        let content_tag = format!("hover_preview_{}", item.path.display());
        egui::Area::new(egui::Id::new(content_tag))
            .fixed_pos(egui::pos2(x, y))
            .order(egui::Order::Tooltip)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .fill(Color32::from_rgba_premultiplied(
                        palette.window_bg.r(),
                        palette.window_bg.g(),
                        palette.window_bg.b(),
                        245,
                    ))
                    .stroke(Stroke::new(1.0_f32, palette.border))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(egui::Margin::same(12.0))
                    .show(ui, |ui| {
                        ui.set_width(card_w - 26.0);
                        match &state.content {
                            HoverContent::Image(tex) => {
                                let img_rect = Rect::from_min_size(
                                    ui.cursor().min,
                                    Vec2::new(card_w - 28.0, card_h - 58.0),
                                );
                                let uv = egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                );
                                ui.painter().image(tex.id(), img_rect, uv, Color32::WHITE);
                                ui.advance_cursor_after_rect(img_rect);
                                ui.add_space(6.0);
                                ui.label(
                                    egui::RichText::new(truncate_end(&item.name, 34))
                                        .strong()
                                        .size(15.5)
                                        .color(palette.text_primary),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} • {}",
                                        item.formatted_size(),
                                        resolve_file_icon(&item.name, &item.extension, false)
                                            .category
                                    ))
                                    .size(14.0)
                                    .color(palette.text_secondary),
                                );
                            }
                            HoverContent::Children(children, total) => {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}  •  {} items",
                                        truncate_end(&item.name, 30),
                                        total
                                    ))
                                    .strong()
                                    .size(15.5)
                                    .color(palette.text_primary),
                                );
                                ui.add_space(6.0);
                                for c in children.iter() {
                                    let spec = resolve_file_icon(&c.name, &c.extension, c.is_dir);
                                    let (cglyph, caccent) = crate::core::icon_packs::IconGlyph::resolve(
                                        &c.name, &c.extension, c.is_dir,
                                        spec.is_image, spec.is_media, spec.is_archive,
                                        spec.is_document, spec.is_code, spec.color_rgb,
                                    );
                                    ui.horizontal(|ui| {
                                        let (crect, _) = ui.allocate_exact_size(
                                            Vec2::splat(15.0),
                                            egui::Sense::hover(),
                                        );
                                        crate::core::icon_packs::paint_icon(
                                            ui, self.icon_pack, cglyph, crect, caccent,
                                        );
                                        ui.label(
                                            egui::RichText::new(truncate_end(&c.name, 30))
                                                .size(14.5)
                                                .color(if c.is_dir {
                                                    palette.text_primary
                                                } else {
                                                    palette.text_secondary
                                                }),
                                        );
                                    });
                                }
                                if *total > children.len() {
                                    ui.add_space(3.0);
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "… +{} more",
                                            total - children.len()
                                        ))
                                        .size(14.0)
                                        .color(palette.text_muted),
                                    );
                                }
                            }
                            HoverContent::Meta => {
                                let spec =
                                    resolve_file_icon(&item.name, &item.extension, item.is_dir);
                                ui.horizontal(|ui| {
                                    let (mrect, _) = ui.allocate_exact_size(
                                        Vec2::splat(40.0),
                                        egui::Sense::hover(),
                                    );
                                    let (mglyph, maccent) = crate::core::icon_packs::IconGlyph::resolve(
                                        &item.name, &item.extension, item.is_dir,
                                        spec.is_image, spec.is_media, spec.is_archive,
                                        spec.is_document, spec.is_code, spec.color_rgb,
                                    );
                                    crate::core::icon_packs::paint_icon(
                                        ui, self.icon_pack, mglyph, mrect, maccent,
                                    );
                                    ui.vertical(|ui| {
                                        ui.label(
                                            egui::RichText::new(truncate_end(&item.name, 32))
                                                .strong()
                                                .size(15.5)
                                                .color(palette.text_primary),
                                        );
                                        ui.label(
                                            egui::RichText::new(spec.category)
                                                .size(14.0)
                                                .color(palette.text_secondary),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{} • {}",
                                                item.formatted_size(),
                                                item.formatted_date()
                                            ))
                                            .size(14.0)
                                            .color(palette.text_muted),
                                        );
                                    });
                                });
                                if item.is_dir {
                                    ui.add_space(4.0);
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} • Hold to peek contents",
                                            item.formatted_date()
                                        ))
                                        .size(14.0)
                                        .color(palette.text_muted),
                                    );
                                }
                            }
                        }
                    });
            });
        // Ensure repaint so the card animates in promptly
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(120));
    }

    /// Get (or lazily decode) a cached image thumbnail. Returns None while the
    /// per-frame decode budget is exhausted so the UI stays responsive.
    fn get_or_load_thumbnail(
        &mut self,
        ctx: &egui::Context,
        path: &Path,
        max_px: u32,
    ) -> Option<egui::TextureHandle> {
        // Per-frame decode budget: at most 3 image decodes per 300ms window
        if self.thumb_decode_budget.0 >= 3 {
            if self.thumb_decode_budget.1.elapsed() < std::time::Duration::from_millis(250) {
                ctx.request_repaint_after(std::time::Duration::from_millis(40));
                return None;
            }
            self.thumb_decode_budget = (0, Instant::now());
        }

        let mtime = std::fs::metadata(path).ok().and_then(|m| m.modified().ok());
        if let Some(entry) = self.thumb_cache.get(path) {
            if entry.mtime == mtime {
                let tex = match &entry.state {
                    ThumbState::Ready(t) => t.clone(),
                };
                return tex;
            }
        }

        let decoded = decode_thumbnail(path, max_px);
        self.thumb_decode_budget.0 += 1;

        let tex = decoded.map(|img| {
            ctx.load_texture(
                format!("thumb_{}", path.display()),
                img,
                egui::TextureOptions::LINEAR,
            )
        });
        self.thumb_cache.insert(
            path.to_path_buf(),
            CachedThumb {
                mtime,
                state: ThumbState::Ready(tex),
            },
        );
        ctx.request_repaint_after(std::time::Duration::from_millis(10));
        let tex = self.thumb_cache.get(path).map(|e| match &e.state {
            ThumbState::Ready(t) => t.clone(),
        });
        tex.flatten()
    }

    #[allow(clippy::too_many_arguments)]
    fn render_grid_view(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        _on_stage_file: &mut impl FnMut(PathBuf),
        on_transcode_target: &mut impl FnMut(PathBuf, String),
        on_exam_conform: &mut impl FnMut(PathBuf),
        on_pack_archive: &mut impl FnMut(Vec<PathBuf>),
        on_inspect_file: &mut impl FnMut(PathBuf),
    ) {
        let entries: Vec<FileItem> = self
            .filtered_and_sorted_entries()
            .into_iter()
            .cloned()
            .collect();
        let mut nav_into_dir: Option<PathBuf> = None;
        let mut peek_to_open: Option<FileItem> = None;
        let mut item_intents: Vec<ItemMenuIntents> = Vec::new();
        let mut canvas_intents = CanvasMenuIntents::default();
        let mut new_hover: Option<(FileItem, Pos2)> = None;
        let icon_pack = self.icon_pack;
        let menu_style = self.menu_style;
        let has_clipboard = !self.clipboard_paths.is_empty();

        // ── Tools & Converters toolbar over the grid ──
        self.render_action_toolbar(
            ui,
            palette,
            &entries,
            on_transcode_target,
            on_exam_conform,
            on_pack_archive,
            on_inspect_file,
        );
        {
            let budget = self.thumb_decode_budget.1.elapsed()
                > std::time::Duration::from_millis(300);
            if budget {
                self.thumb_decode_budget = (0, Instant::now());
            }
        }

        // Icon size is the user's base grid size doubled (2x icon rule) and
        // scaled by the zoom level (Small → Medium → Large → Extra large).
        let grid_zoom_scale = Self::EXPLORER_ZOOM_LEVELS[(self.explorer_zoom as usize).min(4)];
        let grid_checkboxes = self.checkbox_mode;
        let icon_sz = (self.icon_size * 2.0 * grid_zoom_scale).clamp(64.0, 416.0);
        let tile_w = icon_sz * 1.18;
        let tile_h = icon_sz * 1.15;
        let tile_size = Vec2::new(tile_w, tile_h);

        // Labels grow with the icon ramp (apps +4px font bump included).
        let name_font_sz = (icon_sz * 0.115 + 4.0).clamp(14.0, 34.0);
        let size_font_sz = (icon_sz * 0.095 + 3.0).clamp(12.5, 26.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                // Canvas interact registered FIRST so tiles stay on top for clicks
                let canvas_resp = ui.interact(
                    ui.available_rect_before_wrap(),
                    ui.make_persistent_id("explorer_grid_canvas_bg"),
                    egui::Sense::click(),
                );
                canvas_resp.context_menu(|menu_ui| {
                    canvas_intents = Self::run_canvas_context_menu(
                        menu_ui,
                        icon_pack,
                        menu_style,
                        palette.accent,
                        has_clipboard,
                        !entries.is_empty(),
                    );
                });

                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);

                    for item in &entries {
                        let is_checked = self.selected_paths.contains(&item.path);
                        let icon_spec = resolve_file_icon(&item.name, &item.extension, item.is_dir);

                        let (rect, resp) = ui.allocate_exact_size(tile_size, egui::Sense::click());

                        // Selection disclosure is the checkbox ONLY (no tile wash).
                        let cb_sz = 18.0_f32;
                        let cb_rect = Rect::from_min_size(
                            rect.left_top() + Vec2::new(9.0, 9.0),
                            Vec2::splat(cb_sz),
                        );
                        let cb_resp = ui.interact(
                            cb_rect,
                            ui.make_persistent_id(("sel_box_grid", &item.path)),
                            egui::Sense::click(),
                        );
                        let reveal = resp.hovered()
                            || cb_resp.hovered()
                            || is_checked
                            || grid_checkboxes == CheckboxMode::Always;
                        let chk_alpha = self.hover_anims.drive(
                            crate::gui::anim::path_id(&item.path),
                            if reveal { 1.0 } else { 0.0 },
                            self.animations.hover,
                            crate::gui::anim::frame_dt(ui.ctx()),
                        );
                        if grid_checkboxes != CheckboxMode::Off {
                            if grid_checkboxes == CheckboxMode::Always || chk_alpha > 0.02 {
                                Self::paint_selection_checkbox(
                                    ui, cb_rect, is_checked, chk_alpha, palette,
                                );
                            }
                        }
                        if cb_resp.clicked() {
                            if is_checked {
                                self.selected_paths.remove(&item.path);
                            } else {
                                self.selected_paths.insert(item.path.clone());
                            }
                        }

                        // Center vector icon & file name (Apple/Finder proportions)
                        let center = rect.center();
                        let icon_y_offset = tile_h * 0.14;
                        let badge_center = center - Vec2::new(0.0, icon_y_offset);

                        // Image files: real thumbnail preview (lazily decoded, cached)
                        let thumb = if !item.is_dir && is_image_extension(&item.extension) {
                            self.get_or_load_thumbnail(ui.ctx(), &item.path, 220)
                        } else {
                            None
                        };

                        if let Some(tex) = &thumb {
                            // Cover-crop the thumbnail into the badge region
                            let img_sz = (tile_h * 0.58).min(tile_w - 12.0);
                            let img_rect =
                                Rect::from_center_size(badge_center, Vec2::splat(img_sz));
                            let tex_wh = tex.size_vec2();
                            let target_wh = img_rect.size();
                            let scale = (target_wh.x / tex_wh.x).max(target_wh.y / tex_wh.y);
                            let shown = tex_wh * scale;
                            let crop = ((shown - target_wh) * 0.5) / (shown);
                            let uv = egui::Rect::from_min_max(
                                egui::pos2(crop.x.max(0.0), crop.y.max(0.0)),
                                egui::pos2(
                                    (1.0 - crop.x).clamp(0.0, 1.0),
                                    (1.0 - crop.y).clamp(0.0, 1.0),
                                ),
                            );
                            ui.painter().rect_filled(
                                img_rect,
                                Rounding::same(6.0),
                                Color32::from_rgba_premultiplied(
                                    palette.border.r(), palette.border.g(), palette.border.b(), 40,
                                ),
                            );
                            ui.painter_at(img_rect).image(tex.id(), img_rect, uv, Color32::WHITE);
                        } else {
                            // Large embedded vector icon (pack-agnostic, antialiased)
                            let (glyph, accent) = crate::core::icon_packs::IconGlyph::resolve(
                                &item.name, &item.extension, item.is_dir,
                                icon_spec.is_image, icon_spec.is_media, icon_spec.is_archive,
                                icon_spec.is_document, icon_spec.is_code, icon_spec.color_rgb,
                            );
                            let icon_block = (tile_w * 0.52).min(tile_h * 0.56);
                            let irect = Rect::from_center_size(badge_center, Vec2::splat(icon_block));
                            crate::core::icon_packs::paint_icon(ui, self.icon_pack, glyph, irect, accent);
                        }

                        let max_chars = ((tile_w / 8.0) as usize).clamp(8, 22);
                        let display_name = truncate_middle(&item.name, max_chars);

                        let name_y_offset = tile_h * 0.18;
                        ui.painter().text(
                            center + Vec2::new(0.0, name_y_offset),
                            egui::Align2::CENTER_CENTER,
                            display_name,
                            egui::FontId::proportional(name_font_sz),
                            palette.text_primary,
                        );

                        let size_y_offset = tile_h * 0.33;
                        let size_text = if item.is_dir {
                            String::from("--")
                        } else {
                            item.formatted_size()
                        };
                        ui.painter().text(
                            center + Vec2::new(0.0, size_y_offset),
                            egui::Align2::CENTER_CENTER,
                            size_text.as_str(),
                            egui::FontId::monospace(size_font_sz),
                            palette.text_muted,
                        );

                        if resp.double_clicked() {
                            if item.is_dir {
                                nav_into_dir = Some(item.path.clone());
                            } else {
                                open_in_default_app(&item.path);
                            }
                        } else if resp.clicked() {
                            if ui.input(|i| i.modifiers.command || i.modifiers.ctrl) {
                                if is_checked {
                                    self.selected_paths.remove(&item.path);
                                } else {
                                    self.selected_paths.insert(item.path.clone());
                                }
                            } else {
                                self.selected_paths.clear();
                                self.selected_paths.insert(item.path.clone());
                            }
                        }

                        if resp.hovered() {
                            let tile_anchor = rect.center() + Vec2::new(0.0, tile_h * 0.5 + 16.0);
                            new_hover = Some((item.clone(), tile_anchor));
                        }

                        // Rich Context Menu on Grid Tile (pure builder → intents).
                        resp.context_menu(|menu_ui| {
                            item_intents.clear();
                            item_intents.push(Self::run_item_context_menu(
                                menu_ui,
                                icon_pack,
                                menu_style,
                                palette.accent,
                                item,
                            ));
                        });
                    }
                });
            });

        // (The grid empty-canvas context menu is registered FIRST inside the
        // scroll area above, so tile interactions always win the pointer.)

        // Apply menu intents (mutations live OUTSIDE the popup closures).
        if canvas_intents.new_folder {
            self.show_new_folder_dialog = true;
        }
        if canvas_intents.refresh {
            self.navigate_to(self.current_dir.clone());
        }
        if canvas_intents.go_up {
            if let Some(parent) = self.current_dir.parent() {
                let p = parent.to_path_buf();
                if !p.as_os_str().is_empty() {
                    self.navigate_to(p);
                }
            }
        }
        if canvas_intents.select_all {
            for e in &entries {
                self.selected_paths.insert(e.path.clone());
            }
        }
        if canvas_intents.clear_selection {
            self.selected_paths.clear();
        }
        if canvas_intents.paste.is_some() {
            self.paste_from_clipboard();
        }
        if canvas_intents.terminal {
            open_terminal_in_dir(&self.current_dir);
        }
        if canvas_intents.open_in_file_manager {
            open_in_file_manager(&self.current_dir);
        }
        if canvas_intents.properties_dir {
            self.properties_item = Some(FileItem::from_path(self.current_dir.clone()));
        }
        for intent in item_intents {
            if let Some(dir) = intent.nav_into_dir {
                nav_into_dir = Some(dir);
            }
            if let Some(item) = intent.peek {
                peek_to_open = Some(item);
            }
            if let Some((path, fmt)) = intent.transcode {
                on_transcode_target(path, fmt);
            }
            if let Some(path) = intent.exam_conform {
                on_exam_conform(path);
            }
            if let Some(paths) = intent.pack {
                on_pack_archive(paths);
            }
            if let Some(path) = intent.inspect {
                on_inspect_file(path);
            }
            if let Some(path) = intent.extract {
                if let Some(parent) = path.parent() {
                    let parent = parent.to_path_buf();
                    log_result("Extract", &path, crate::archive::unpack_archive(&path, &parent));
                    self.navigate_to(self.current_dir.clone());
                }
            }
            if let Some((path, name)) = intent.rename {
                self.renaming_path = Some((path, name));
            }
            if let Some(item) = intent.properties {
                self.properties_item = Some(item);
            }
            if let Some(path) = intent.deleted {
                let _ = remove_entry_recursive(&path);
                self.selected_paths.remove(&path);
                self.navigate_to(self.current_dir.clone());
            }
            if let Some(paths) = intent.cut {
                self.clipboard_paths = paths.clone();
                self.clipboard_is_cut = true;
                crate::gui::context_menu::set_clipboard_files_with_effect(&paths, true);
            }
            if let Some(paths) = intent.copy {
                self.clipboard_paths = paths.clone();
                self.clipboard_is_cut = false;
                crate::gui::context_menu::set_clipboard_files_with_effect(&paths, false);
            }
        }

        if let Some(dir) = nav_into_dir {
            self.navigate_to(dir);
        }

        // Apply hover preview state (grid tiles)
        let keep_existing = matches!(
            (&self.hover_preview, &new_hover),
            (Some(active), Some((item, _))) if active.item.path == item.path
        );
        if keep_existing {
            let due = self
                .hover_preview
                .as_ref()
                .map(|h| h.started_at.elapsed() > std::time::Duration::from_millis(450))
                .unwrap_or(false);
            if due {
                self.ensure_hover_content(ui.ctx());
            }
        } else if let Some((item, anchor)) = new_hover {
            self.hover_preview = Some(HoverPreviewState {
                item,
                anchor,
                started_at: Instant::now(),
                content: HoverContent::Meta,
            });
        } else {
            self.hover_preview = None;
        }
        self.render_hover_preview(ui, palette);

        if let Some(item) = peek_to_open {
            self.peek_item = Some(item);
            self.peek_texture = None;
            self.peek_text_cache = None;
            self.peek_hash_cache = None;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_columns_view(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        _on_stage_file: &mut impl FnMut(PathBuf),
        on_transcode_target: &mut impl FnMut(PathBuf, String),
        _on_exam_conform: &mut impl FnMut(PathBuf),
        on_pack_archive: &mut impl FnMut(Vec<PathBuf>),
        _on_inspect_file: &mut impl FnMut(PathBuf),
    ) {
        let entries: Vec<FileItem> = self
            .filtered_and_sorted_entries()
            .into_iter()
            .cloned()
            .collect();
        let mut nav_into_dir: Option<PathBuf> = None;
        let mut subfolder_to_load: Option<PathBuf> = None;
        let mut preview_to_set: Option<FileItem> = None;
        let mut peek_to_open: Option<FileItem> = None;

        egui::ScrollArea::horizontal()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(1.0, 0.0);

                    // ==========================================
                    // COLUMN 1: Current Directory
                    // ==========================================
                    let col1_w = 240.0_f32;
                    let col_h = ui.available_height();

                    egui::Frame::none()
                        .fill(Color32::from_rgba_premultiplied(
                            palette.card_bg.r(),
                            palette.card_bg.g(),
                            palette.card_bg.b(),
                            180,
                        ))
                        .stroke(Stroke::new(1.0_f32, palette.border))
                        .show(ui, |ui| {
                            ui.set_width(col1_w);
                            ui.set_height(col_h);
                            egui::ScrollArea::vertical()
                                .auto_shrink([false; 2])
                                .show(ui, |ui| {
                                    for (idx, item) in entries.iter().enumerate() {
                                        let is_selected = self.selected_paths.contains(&item.path)
                                            || self.focused_row == idx;
                                        let icon_spec = resolve_file_icon(
                                            &item.name,
                                            &item.extension,
                                            item.is_dir,
                                        );

                                        let symlink_suffix =
                                            if item.is_symlink { " ↗" } else { "" };
                                        let display_name = truncate_middle(
                                            &format!("{}{}", item.name, symlink_suffix),
                                            (col1_w / 7.0) as usize,
                                        );
                                        let name_text = if item.is_dir {
                                            format!("{}  ›", display_name)
                                        } else {
                                            display_name
                                        };

                                        let (row_rect, resp) = ui.allocate_exact_size(
                                            Vec2::new(col1_w - 8.0, 24.0),
                                            egui::Sense::click(),
                                        );
                                        // Row band: hover/selected
                                        if is_selected {
                                            ui.painter().rect_filled(
                                                row_rect,
                                                Rounding::same(5.0),
                                                Color32::from_rgba_premultiplied(
                                                    palette.accent.r(),
                                                    palette.accent.g(),
                                                    palette.accent.b(),
                                                    38,
                                                ),
                                            );
                                        } else if resp.hovered() {
                                            ui.painter().rect_filled(
                                                row_rect,
                                                Rounding::same(5.0),
                                                Color32::from_rgba_premultiplied(
                                                    palette.accent.r(),
                                                    palette.accent.g(),
                                                    palette.accent.b(),
                                                    18,
                                                ),
                                            );
                                        }
                                        // Pack vector icon on left
                                        let (icon_glyph, accent_rgb) =
                                            crate::core::icon_packs::IconGlyph::resolve(
                                                &item.name,
                                                &item.extension,
                                                item.is_dir,
                                                icon_spec.is_image,
                                                icon_spec.is_media,
                                                icon_spec.is_archive,
                                                icon_spec.is_document,
                                                icon_spec.is_code,
                                                icon_spec.color_rgb,
                                            );
                                        let icon_rect = Rect::from_center_size(
                                            Pos2::new(row_rect.left() + 10.0, row_rect.center().y),
                                            Vec2::splat(17.0),
                                        );
                                        crate::core::icon_packs::paint_icon(
                                            ui,
                                            self.icon_pack,
                                            icon_glyph,
                                            icon_rect,
                                            accent_rgb,
                                        );
                                        // Name text right of icon
                                        let name_color = if is_selected {
                                            palette.text_primary
                                        } else if item.is_hidden {
                                            palette.text_muted
                                        } else {
                                            palette.text_primary
                                        };
                                        ui.painter().text(
                                            Pos2::new(row_rect.left() + 24.0, row_rect.center().y),
                                            egui::Align2::LEFT_CENTER,
                                            name_text,
                                            egui::FontId::proportional(15.5),
                                            name_color,
                                        );
                                        if resp.clicked() {
                                            self.focused_row = idx;
                                            self.selected_paths.clear();
                                            self.selected_paths.insert(item.path.clone());
                                            if item.is_dir {
                                                subfolder_to_load = Some(item.path.clone());
                                                preview_to_set = None;
                                            } else {
                                                preview_to_set = Some(item.clone());
                                                self.column_subfolder = None;
                                            }
                                        }
                                        if resp.double_clicked() {
                                            if item.is_dir {
                                                nav_into_dir = Some(item.path.clone());
                                            } else {
                                                open_in_default_app(&item.path);
                                            }
                                        }
                                    }
                                });
                        });

                    // ==========================================
                    // COLUMN 2: Subfolder Contents (if any)
                    // ==========================================
                    if let Some((sub_path, sub_items)) = &self.column_subfolder {
                        let col2_w = 240.0_f32;
                        egui::Frame::none()
                            .fill(Color32::from_rgba_premultiplied(
                                palette.card_bg.r(),
                                palette.card_bg.g(),
                                palette.card_bg.b(),
                                180,
                            ))
                            .stroke(Stroke::new(1.0_f32, palette.border))
                            .show(ui, |ui| {
                                ui.set_width(col2_w);
                                ui.set_height(col_h);
                                egui::ScrollArea::vertical().auto_shrink([false; 2]).show(
                                    ui,
                                    |ui| {
                                        let sub_name = sub_path
                                            .file_name()
                                            .map(|n| n.to_string_lossy().to_string())
                                            .unwrap_or_else(|| "Folder".into());
                                        ui.add_space(4.0);
                                        ui.horizontal(|ui| {
                                            ui.add_space(6.0);
                                            ui.label(
                                                egui::RichText::new(&sub_name)
                                                    .size(14.5)
                                                    .strong()
                                                    .color(palette.text_muted),
                                            );
                                        });
                                        ui.separator();

                                        for sub_item in sub_items {
                                            let is_sel =
                                                self.selected_paths.contains(&sub_item.path);
                                            let icon_spec = resolve_file_icon(
                                                &sub_item.name,
                                                &sub_item.extension,
                                                sub_item.is_dir,
                                            );

                                            let symlink_suffix =
                                                if sub_item.is_symlink { " ↗" } else { "" };
                                            let display_name = truncate_middle(
                                                &format!("{}{}", sub_item.name, symlink_suffix),
                                                (col2_w / 7.0) as usize,
                                            );
                                            let name_text = if sub_item.is_dir {
                                                format!("{}  ›", display_name)
                                            } else {
                                                display_name
                                            };

                                            let (row_rect, resp) = ui.allocate_exact_size(
                                                Vec2::new(col2_w - 8.0, 24.0),
                                                egui::Sense::click(),
                                            );
                                            if is_sel {
                                                ui.painter().rect_filled(
                                                    row_rect,
                                                    Rounding::same(5.0),
                                                    Color32::from_rgba_premultiplied(
                                                        palette.accent.r(),
                                                        palette.accent.g(),
                                                        palette.accent.b(),
                                                        38,
                                                    ),
                                                );
                                            } else if resp.hovered() {
                                                ui.painter().rect_filled(
                                                    row_rect,
                                                    Rounding::same(5.0),
                                                    Color32::from_rgba_premultiplied(
                                                        palette.accent.r(),
                                                        palette.accent.g(),
                                                        palette.accent.b(),
                                                        18,
                                                    ),
                                                );
                                            }
                                            let (cglyph, caccent) =
                                                crate::core::icon_packs::IconGlyph::resolve(
                                                    &sub_item.name,
                                                    &sub_item.extension,
                                                    sub_item.is_dir,
                                                    icon_spec.is_image,
                                                    icon_spec.is_media,
                                                    icon_spec.is_archive,
                                                    icon_spec.is_document,
                                                    icon_spec.is_code,
                                                    icon_spec.color_rgb,
                                                );
                                            let icon_rect = Rect::from_center_size(
                                                Pos2::new(
                                                    row_rect.left() + 10.0,
                                                    row_rect.center().y,
                                                ),
                                                Vec2::splat(17.0),
                                            );
                                            crate::core::icon_packs::paint_icon(
                                                ui,
                                                self.icon_pack,
                                                cglyph,
                                                icon_rect,
                                                caccent,
                                            );
                                            let name_color = if is_sel {
                                                palette.text_primary
                                            } else if sub_item.is_hidden {
                                                palette.text_muted
                                            } else {
                                                palette.text_primary
                                            };
                                            ui.painter().text(
                                                Pos2::new(
                                                    row_rect.left() + 24.0,
                                                    row_rect.center().y,
                                                ),
                                                egui::Align2::LEFT_CENTER,
                                                name_text,
                                                egui::FontId::proportional(15.5),
                                                name_color,
                                            );
                                            if resp.clicked() {
                                                self.selected_paths.clear();
                                                self.selected_paths.insert(sub_item.path.clone());
                                                preview_to_set = Some(sub_item.clone());
                                            }
                                            if resp.double_clicked() {
                                                if sub_item.is_dir {
                                                    nav_into_dir = Some(sub_item.path.clone());
                                                } else {
                                                    open_in_default_app(&sub_item.path);
                                                }
                                            }
                                        }
                                    },
                                );
                            });
                    }

                    // ==========================================
                    // COLUMN 3: macOS Inspector Card / File Preview
                    // ==========================================
                    let active_preview = self
                        .column_preview_item
                        .as_ref()
                        .or_else(|| entries.get(self.focused_row));

                    if let Some(item) = active_preview {
                        let inspector_w = 280.0_f32;
                        let item_clone = item.clone();

                        egui::Frame::none()
                            .fill(Color32::from_rgba_premultiplied(
                                palette.card_bg.r(),
                                palette.card_bg.g(),
                                palette.card_bg.b(),
                                220,
                            ))
                            .stroke(Stroke::new(1.0_f32, palette.border))
                            .inner_margin(egui::Margin::same(14.0))
                            .show(ui, |ui| {
                                ui.set_width(inspector_w);
                                ui.set_height(col_h);
                                egui::ScrollArea::vertical().auto_shrink([false; 2]).show(
                                    ui,
                                    |ui| {
                                        let icon_spec = resolve_file_icon(
                                            &item_clone.name,
                                            &item_clone.extension,
                                            item_clone.is_dir,
                                        );
                                        // Large centered vector icon
                                        ui.vertical_centered(|ui| {
                                            ui.add_space(10.0);
                                            let (glyph, accent) =
                                                crate::core::icon_packs::IconGlyph::resolve(
                                                    &item_clone.name,
                                                    &item_clone.extension,
                                                    item_clone.is_dir,
                                                    icon_spec.is_image,
                                                    icon_spec.is_media,
                                                    icon_spec.is_archive,
                                                    icon_spec.is_document,
                                                    icon_spec.is_code,
                                                    icon_spec.color_rgb,
                                                );
                                            let (irect, _) = ui.allocate_exact_size(
                                                Vec2::splat(88.0),
                                                egui::Sense::hover(),
                                            );
                                            crate::core::icon_packs::paint_icon(
                                                ui, self.icon_pack, glyph, irect, accent,
                                            );
                                            ui.add_space(4.0);
                                            ui.label(
                                                egui::RichText::new(&item_clone.name)
                                                    .size(17.0)
                                                    .strong()
                                                    .color(palette.text_primary),
                                            );
                                            ui.label(
                                                egui::RichText::new(icon_spec.category)
                                                    .size(14.5)
                                                    .color(palette.text_muted),
                                            );
                                        });

                                        ui.add_space(10.0);
                                        ui.separator();
                                        ui.add_space(8.0);

                                        // File Properties / Key-Value Grid
                                        egui::Grid::new("column_inspector_grid")
                                            .num_columns(2)
                                            .spacing([12.0, 6.0])
                                            .show(ui, |ui| {
                                                ui.label(
                                                    egui::RichText::new("Kind:")
                                                        .size(15.0)
                                                        .color(palette.text_muted),
                                                );
                                                ui.label(
                                                    egui::RichText::new(if item_clone.is_dir {
                                                        "Folder"
                                                    } else {
                                                        &item_clone.extension
                                                    })
                                                    .size(15.0)
                                                    .strong()
                                                    .color(palette.text_primary),
                                                );
                                                ui.end_row();

                                                ui.label(
                                                    egui::RichText::new("Size:")
                                                        .size(15.0)
                                                        .color(palette.text_muted),
                                                );
                                                ui.label(
                                                    egui::RichText::new(
                                                        item_clone.formatted_size(),
                                                    )
                                                    .size(15.0)
                                                    .color(palette.text_secondary),
                                                );
                                                ui.end_row();

                                                ui.label(
                                                    egui::RichText::new("Modified:")
                                                        .size(15.0)
                                                        .color(palette.text_muted),
                                                );
                                                ui.label(
                                                    egui::RichText::new(
                                                        item_clone.formatted_date(),
                                                    )
                                                    .size(14.5)
                                                    .color(palette.text_secondary),
                                                );
                                                ui.end_row();

                                                ui.label(
                                                    egui::RichText::new("Permissions:")
                                                        .size(15.0)
                                                        .color(palette.text_muted),
                                                );
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "{} ({})",
                                                        item_clone.formatted_permissions(),
                                                        item_clone.formatted_octal_permissions()
                                                    ))
                                                    .size(14.5)
                                                    .monospace()
                                                    .color(palette.text_secondary),
                                                );
                                                ui.end_row();

                                                if item_clone.is_symlink {
                                                    ui.label(
                                                        egui::RichText::new("Symlink:")
                                                            .size(15.0)
                                                            .color(palette.text_muted),
                                                    );
                                                    let target_str = item_clone
                                                        .symlink_target
                                                        .as_ref()
                                                        .map(|p| p.to_string_lossy().to_string())
                                                        .unwrap_or_else(|| "unknown".into());
                                                    ui.label(
                                                        egui::RichText::new(format!(
                                                            "↗ {}",
                                                            target_str
                                                        ))
                                                        .size(14.5)
                                                        .color(palette.accent),
                                                    );
                                                    ui.end_row();
                                                }

                                                ui.label(
                                                    egui::RichText::new("Hidden:")
                                                        .size(15.0)
                                                        .color(palette.text_muted),
                                                );
                                                ui.label(
                                                    egui::RichText::new(if item_clone.is_hidden {
                                                        "Yes"
                                                    } else {
                                                        "No"
                                                    })
                                                    .size(15.0)
                                                    .color(palette.text_secondary),
                                                );
                                                ui.end_row();
                                            });

                                        ui.add_space(12.0);
                                        ui.separator();
                                        ui.add_space(8.0);

                                        // Quick Action Buttons
                                        ui.vertical_centered_justified(|ui| {
                                            if ui
                                                .button(
                                                    egui::RichText::new("Quick Look (Space)")
                                                        .strong(),
                                                )
                                                .clicked()
                                            {
                                                peek_to_open = Some(item_clone.clone());
                                            }
                                            if !item_clone.is_dir {
                                                if ui.button("Transcode / Convert").clicked() {
                                                    on_transcode_target(
                                                        item_clone.path.clone(),
                                                        "ALL".into(),
                                                    );
                                                }
                                            } else if ui
                                                .button("📦 Pack into ZIP Archive")
                                                .clicked()
                                            {
                                                on_pack_archive(vec![item_clone.path.clone()]);
                                            }
                                            if ui.button("📂 Reveal in System Explorer").clicked()
                                            {
                                                reveal_in_file_manager(&item_clone.path);
                                            }
                                            if ui.button("📋 Copy Full Path").clicked() {
                                                ui.ctx().output_mut(|o| {
                                                    o.copied_text = item_clone
                                                        .path
                                                        .to_string_lossy()
                                                        .to_string()
                                                });
                                            }
                                            if ui.button("💻 Open Terminal Here").clicked() {
                                                let dir_target = if item_clone.is_dir {
                                                    &item_clone.path
                                                } else {
                                                    item_clone
                                                        .path
                                                        .parent()
                                                        .unwrap_or(&item_clone.path)
                                                };
                                                open_terminal_in_dir(dir_target);
                                            }
                                        });
                                    },
                                );
                            });
                    }
                });
            });

        if let Some(sub_path) = subfolder_to_load {
            let sub_entries = list_directory_sync(&sub_path);
            self.column_subfolder = Some((sub_path, sub_entries));
        }
        if let Some(preview_item) = preview_to_set {
            self.column_preview_item = Some(preview_item);
        }
        if let Some(peek_item) = peek_to_open {
            self.peek_item = Some(peek_item);
        }
        if let Some(new_dir) = nav_into_dir {
            self.navigate_to(new_dir);
        }
    }

    fn render_status_bar(&mut self, ui: &mut Ui, palette: &ThemePalette) {
        let bar_h = 24.0_f32;
        let entries = self.filtered_and_sorted_entries();
        let total_items = entries.len();
        let total_hidden = self.entries.iter().filter(|e| e.is_hidden).count();
        let sel_count = self.selected_paths.len();

        let total_selected_bytes: u64 = self
            .entries
            .iter()
            .filter(|e| self.selected_paths.contains(&e.path) && !e.is_dir)
            .map(|e| e.size_bytes)
            .sum();

        // Find drive space for current path
        let drive_info = self
            .drives
            .iter()
            .find(|d| self.current_dir.starts_with(&d.path))
            .map(|d| format!("💾 {} available on {}", d.formatted_space(), d.display_name));

        egui::Frame::none()
            .fill(Color32::from_rgba_premultiplied(
                palette.card_bg.r(),
                palette.card_bg.g(),
                palette.card_bg.b(),
                220,
            ))
            .stroke(Stroke::new(1.0_f32, palette.border))
            .inner_margin(egui::Margin::symmetric(10.0, 3.0))
            .show(ui, |ui| {
                ui.allocate_ui_with_layout(
                    Vec2::new(ui.available_width(), bar_h),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        // Left: Item count & selection
                        let count_text = if total_hidden > 0 && !self.show_hidden_files {
                            format!("{} items ({} hidden)", total_items, total_hidden)
                        } else {
                            format!("{} items", total_items)
                        };
                        ui.label(
                            egui::RichText::new(count_text)
                                .size(14.5)
                                .color(palette.text_secondary),
                        );

                        if sel_count > 0 {
                            ui.label(
                                egui::RichText::new("•")
                                    .size(14.5)
                                    .color(palette.text_muted),
                            );
                            let sel_text = format!(
                                "{} selected ({})",
                                sel_count,
                                format_file_size(total_selected_bytes)
                            );
                            ui.label(
                                egui::RichText::new(sel_text)
                                    .size(14.5)
                                    .strong()
                                    .color(palette.accent),
                            );
                        }

                        // Right: Drive available space, zoom slider (if Grid), dotfiles indicator
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if self.view_mode == ViewMode::Grid {
                                ui.add_sized(
                                    [70.0, 16.0],
                                    egui::Slider::new(&mut self.icon_size, 64.0..=160.0)
                                        .show_value(false),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.0}px", self.icon_size))
                                        .size(14.0)
                                        .color(palette.text_muted),
                                );
                                ui.label(
                                    egui::RichText::new("•")
                                        .size(14.5)
                                        .color(palette.text_muted),
                                );
                            }

                            if self.show_hidden_files {
                                ui.label(
                                    egui::RichText::new("Dotfiles visible")
                                        .size(14.0)
                                        .color(palette.accent),
                                );
                                ui.label(
                                    egui::RichText::new("•")
                                        .size(14.5)
                                        .color(palette.text_muted),
                                );
                            }

                            if let Some(info) = drive_info {
                                ui.label(
                                    egui::RichText::new(info)
                                        .size(14.5)
                                        .color(palette.text_muted),
                                );
                            }
                        });
                    },
                );
            });
    }

    /// Renders the macOS Quick Look / Peek Display Modal Overlay
    fn render_peek_overlay(
        &mut self,
        ctx: &egui::Context,
        palette: &ThemePalette,
        on_transcode_target: &mut impl FnMut(PathBuf, String),
        on_exam_conform: &mut impl FnMut(PathBuf),
        on_pack_archive: &mut impl FnMut(Vec<PathBuf>),
        on_inspect_file: &mut impl FnMut(PathBuf),
    ) {
        let item = match self.peek_item.clone() {
            Some(i) => i,
            None => return,
        };

        let mut close_peek = false;
        let mut transcode_to: Option<String> = None;
        let mut exam_to_open = false;
        let mut pack_to_open = false;
        let mut inspect_to_open = false;

        let window_title = format!("Quick Look — {}", item.name);

        egui::Window::new(window_title)
            .collapsible(false)
            .resizable(true)
            .default_size([620.0, 480.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(palette.card_bg)
                    .stroke(Stroke::new(1.0_f32, palette.border))
                    .rounding(Rounding::same(12.0))
                    .inner_margin(egui::Margin::same(16.0)),
            )
            .show(ctx, |ui| {
                // 1. Top Meta Header
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&item.name)
                            .size(19.0)
                            .strong()
                            .color(palette.text_primary),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✕ Close (Esc)").clicked() {
                            close_peek = true;
                        }
                    });
                });

                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("Size: {}", item.formatted_size()))
                            .color(palette.text_secondary),
                    );
                    ui.label("•");
                    ui.label(
                        egui::RichText::new(format!("Modified: {}", item.formatted_date()))
                            .color(palette.text_secondary),
                    );
                    if !item.extension.is_empty() {
                        ui.label("•");
                        ui.label(
                            egui::RichText::new(&item.extension)
                                .color(palette.accent)
                                .strong(),
                        );
                    }
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // 2. Category-Specific Deep Preview
                let is_image = matches!(
                    item.extension.as_str(),
                    "JPG" | "JPEG" | "PNG" | "WEBP" | "GIF" | "AVIF" | "TIFF" | "BMP" | "ICO"
                );
                let is_text = matches!(
                    item.extension.as_str(),
                    "TXT"
                        | "MD"
                        | "RS"
                        | "PY"
                        | "JSON"
                        | "XML"
                        | "YAML"
                        | "YML"
                        | "LOG"
                        | "CSV"
                        | "TOML"
                        | "HTML"
                        | "CSS"
                        | "C"
                        | "CPP"
                        | "H"
                        | "JS"
                        | "TS"
                );

                if is_image {
                    if self.peek_texture.as_ref().map(|(p, _)| p) != Some(&item.path) {
                        self.peek_texture = load_image_thumbnail(ctx, &item.path)
                            .map(|tex| (item.path.clone(), tex));
                    }

                    if let Some((_, ref tex)) = self.peek_texture {
                        ui.vertical_centered(|ui| {
                            ui.add(
                                egui::Image::new(tex)
                                    .max_size(Vec2::new(560.0, 260.0))
                                    .rounding(Rounding::same(8.0)),
                            );
                        });
                    } else {
                        ui.label(
                            egui::RichText::new(
                                "📷 Image preview not available or unsupported raw format.",
                            )
                            .color(palette.text_muted),
                        );
                    }
                } else if is_text {
                    if self.peek_text_cache.as_ref().map(|(p, _)| p) != Some(&item.path) {
                        let text = std::fs::read(&item.path)
                            .map(|b| String::from_utf8_lossy(&b[..b.len().min(16384)]).to_string())
                            .unwrap_or_else(|e| format!("Failed to read file: {}", e));
                        self.peek_text_cache = Some((item.path.clone(), text));
                    }

                    if let Some((_, ref text)) = self.peek_text_cache {
                        egui::ScrollArea::vertical()
                            .max_height(250.0)
                            .show(ui, |ui| {
                                for (idx, line) in text.lines().take(100).enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(format!("{:3} |", idx + 1))
                                                .color(palette.text_muted)
                                                .monospace(),
                                        );
                                        ui.label(
                                            egui::RichText::new(line)
                                                .color(palette.text_primary)
                                                .monospace(),
                                        );
                                    });
                                }
                            });
                    }
                } else {
                    ui.label(
                        egui::RichText::new(format!("File Path: {}", item.path.display()))
                            .size(15.5)
                            .monospace()
                            .color(palette.text_primary),
                    );

                    if self.peek_hash_cache.as_ref().map(|(p, _)| p) != Some(&item.path) {
                        let hash = crate::codec::calculate_sha256(&item.path)
                            .unwrap_or_else(|_| "Unavailable".to_string());
                        self.peek_hash_cache = Some((item.path.clone(), hash));
                    }

                    if let Some((_, ref hash)) = self.peek_hash_cache {
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(format!("SHA-256: {}", hash))
                                .size(15.0)
                                .monospace()
                                .color(palette.accent),
                        );
                    }

                    if let Ok(bytes) = std::fs::read(&item.path) {
                        let sample = &bytes[..bytes.len().min(48)];
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("Binary Header Hex Dump:")
                                .size(15.0)
                                .strong()
                                .color(palette.text_secondary),
                        );
                        egui::Frame::none()
                            .fill(Color32::from_black_alpha(80))
                            .inner_margin(egui::Margin::same(8.0))
                            .show(ui, |ui| {
                                for chunk in sample.chunks(16) {
                                    let hex_str = chunk
                                        .iter()
                                        .map(|b| format!("{:02X}", b))
                                        .collect::<Vec<_>>()
                                        .join(" ");
                                    let ascii_str: String = chunk
                                        .iter()
                                        .map(|b| {
                                            if b.is_ascii_graphic() {
                                                *b as char
                                            } else {
                                                '.'
                                            }
                                        })
                                        .collect();
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{:<48}  |  {}",
                                            hex_str, ascii_str
                                        ))
                                        .size(14.5)
                                        .monospace()
                                        .color(Color32::from_rgb(180, 215, 255)),
                                    );
                                }
                            });
                    }
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);

                // 3. Action Toolbar at bottom of Quick Look
                ui.horizontal_wrapped(|ui| {
                    if !item.is_dir {
                        if ui.button("Transcode JPG").clicked() {
                            transcode_to = Some("JPG".into());
                        }
                        if ui.button("Transcode PNG").clicked() {
                            transcode_to = Some("PNG".into());
                        }
                        if ui.button("Transcode WEBP").clicked() {
                            transcode_to = Some("WEBP".into());
                        }
                        if ui.button("Transcode PDF").clicked() {
                            transcode_to = Some("PDF".into());
                        }
                        if ui.button("🎯 Exam Conform").clicked() {
                            exam_to_open = true;
                        }
                        if ui.button("🔬 Quality Inspect").clicked() {
                            inspect_to_open = true;
                        }
                    }
                    if ui.button("📦 Pack (ZIP)").clicked() {
                        pack_to_open = true;
                    }
                    if ui.button("📂 Open Folder").clicked() {
                        if let Some(parent) = item.path.parent() {
                            open_in_file_manager(parent);
                        }
                    }
                });
            });

        if close_peek {
            self.peek_item = None;
            self.peek_texture = None;
            self.peek_text_cache = None;
            self.peek_hash_cache = None;
        }
        if let Some(target) = transcode_to {
            on_transcode_target(item.path.clone(), target);
            self.peek_item = None;
        }
        if exam_to_open {
            on_exam_conform(item.path.clone());
            self.peek_item = None;
        }
        if pack_to_open {
            on_pack_archive(vec![item.path.clone()]);
            self.peek_item = None;
        }
        if inspect_to_open {
            on_inspect_file(item.path.clone());
            self.peek_item = None;
        }
    }

    fn render_properties_modal(&mut self, ctx: &egui::Context, palette: &ThemePalette) {
        let item = match &self.properties_item {
            Some(i) => i.clone(),
            None => return,
        };

        let mut close_modal = false;
        let mut reveal_path: Option<PathBuf> = None;
        let mut peek_item: Option<FileItem> = None;

        let icon_spec = resolve_file_icon(&item.name, &item.extension, item.is_dir);

        egui::Window::new(format!("ℹ Properties — {}", item.name))
            .collapsible(false)
            .resizable(false)
            .default_width(380.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(palette.card_bg)
                    .stroke(Stroke::new(1.0_f32, palette.border))
                    .rounding(Rounding::same(12.0))
                    .inner_margin(egui::Margin::same(18.0)),
            )
            .show(ctx, |ui| {
                // Header with large icon and name
                ui.horizontal(|ui| {
                    let (picon_rect, _) = ui.allocate_exact_size(
                        Vec2::splat(64.0),
                        egui::Sense::hover(),
                    );
                    let (pglyph, paccent) = crate::core::icon_packs::IconGlyph::resolve(
                        &item.name, &item.extension, item.is_dir,
                        icon_spec.is_image, icon_spec.is_media, icon_spec.is_archive,
                        icon_spec.is_document, icon_spec.is_code, icon_spec.color_rgb,
                    );
                    crate::core::icon_packs::paint_icon(ui, self.icon_pack, pglyph, picon_rect, paccent);
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(&item.name)
                                .size(19.0)
                                .strong()
                                .color(palette.text_primary),
                        );
                        ui.label(
                            egui::RichText::new(if item.is_dir {
                                "Folder".to_string()
                            } else {
                                format!("{} File", item.extension)
                            })
                            .size(15.0)
                            .color(palette.text_secondary),
                        );
                    });
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                // Properties Grid
                egui::Grid::new("properties_info_grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Location:").color(palette.text_muted));
                        let parent_str = item
                            .path
                            .parent()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|| "—".to_string());
                        ui.label(egui::RichText::new(parent_str).color(palette.text_primary));
                        ui.end_row();

                        ui.label(egui::RichText::new("Size:").color(palette.text_muted));
                        let size_detail =
                            format!("{} ({} bytes)", item.formatted_size(), item.size_bytes);
                        ui.label(
                            egui::RichText::new(size_detail)
                                .monospace()
                                .color(palette.text_primary),
                        );
                        ui.end_row();

                        ui.label(egui::RichText::new("Modified:").color(palette.text_muted));
                        ui.label(
                            egui::RichText::new(item.formatted_date()).color(palette.text_primary),
                        );
                        ui.end_row();

                        ui.label(egui::RichText::new("Category:").color(palette.text_muted));
                        ui.label(egui::RichText::new(icon_spec.category).color(palette.accent));
                        ui.end_row();

                        ui.label(egui::RichText::new("Full Path:").color(palette.text_muted));
                        ui.horizontal(|ui| {
                            let path_str = item.path.to_string_lossy().to_string();
                            if ui.button("📋 Copy Path").clicked() {
                                ui.output_mut(|o| o.copied_text = path_str);
                            }
                        });
                        ui.end_row();
                    });

                ui.add_space(14.0);
                ui.separator();
                ui.add_space(8.0);

                // Action Buttons Footer
                ui.horizontal(|ui| {
                    if ui.button("📂 Reveal in Explorer").clicked() {
                        reveal_path = item.path.parent().map(|p| p.to_path_buf());
                    }

                    if !item.is_dir && ui.button("Quick Look").clicked() {
                        peek_item = Some(item.clone());
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            close_modal = true;
                        }
                    });
                });
            });

        if let Some(parent) = reveal_path {
            open_in_file_manager(&parent);
        }
        if let Some(p) = peek_item {
            self.peek_item = Some(p);
            self.properties_item = None;
        }
        if close_modal {
            self.properties_item = None;
        }
    }
}

fn load_image_thumbnail(ctx: &egui::Context, path: &Path) -> Option<egui::TextureHandle> {
    if let Ok(bytes) = std::fs::read(path) {
        if let Ok(img) = image::load_from_memory(&bytes) {
            let thumb = img.thumbnail(520, 320).to_rgba8();
            let (w, h) = thumb.dimensions();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [w as usize, h as usize],
                &thumb.into_raw(),
            );
            return Some(ctx.load_texture(
                path.to_string_lossy(),
                color_image,
                egui::TextureOptions::LINEAR,
            ));
        }
    }
    None
}

/// True if the extension maps to an image category we can thumbnail
fn is_image_extension(ext: &str) -> bool {
    matches!(
        ext,
        "JPG" | "JPEG" | "PNG" | "WEBP" | "GIF" | "AVIF" | "TIFF" | "BMP" | "ICO" | "HEIC" | "RAW"
    )
}

/// Decode an image file down to a `max_px` thumbnail ColorImage
fn decode_thumbnail(path: &Path, max_px: u32) -> Option<egui::ColorImage> {
    let bytes = std::fs::read(path).ok()?;
    let img = image::load_from_memory(&bytes).ok()?;
    let thumb = img.thumbnail(max_px, max_px).to_rgba8();
    let (w, h) = thumb.dimensions();
    Some(egui::ColorImage::from_rgba_unmultiplied(
        [w as usize, h as usize],
        &thumb.into_raw(),
    ))
}

/// Truncate a label in the middle ("verylongfilename.txt" -> "verylo…ame.txt")
fn truncate_middle(s: &str, max_chars: usize) -> String {
    let total = s.chars().count();
    if total <= max_chars || max_chars < 6 {
        return s.to_string();
    }
    let keep = max_chars - 1;
    let head = keep * 2 / 3;
    let tail = keep - head;
    let head_str: String = s.chars().take(head).collect();
    let tail_str: String = s.chars().skip(total.saturating_sub(tail)).collect();
    format!("{}…{}", head_str, tail_str)
}

/// Truncate at the end ("panchang-widget-desktop" -> "panchang-wi…")
fn truncate_end(s: &str, max_chars: usize) -> String {
    let total = s.chars().count();
    if total <= max_chars {
        return s.to_string();
    }
    let keep = max_chars.saturating_sub(1);
    format!("{}…", s.chars().take(keep).collect::<String>())
}

fn open_in_file_manager(path: &Path) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(path).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(path).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
}

/// Text-only fallback for a segmented switcher while rasterization warms up.
fn menu_fallback_button(label: &str) -> egui::Button<'static> {
    egui::Button::new(label.to_string())
}

/// Observability floor for file operations: every filesystem side effect is
/// logged (error level on failure, debug on success) so the explorer never
/// fails silently and post-mortems have a paper trail.
fn log_result<T, E: std::fmt::Display>(what: &str, path: &Path, res: Result<T, E>) {
    match res {
        Ok(_) => log::debug!("{what} ok: {}", path.display()),
        Err(e) => log::error!("{what} failed for {}: {e}", path.display()),
    }
}

/// Small square icon button used for inline controls (filter clear etc.).
fn nav_icon_button(
    ui: &mut Ui,
    glyph: IconGlyph,
    px: f32,
) -> egui::Response {
    let inactive = ui.style().visuals.widgets.inactive;
    let tint = inactive.fg_stroke.color;
    let tint = [tint.r(), tint.g(), tint.b()];
    let btn = match crate::core::icon_packs::icon_texture(
        ui.ctx(),
        crate::core::icon_packs::IconPack::from_config_id(
            &crate::core::MossambiConfig::load().icon_pack,
        ),
        glyph,
        px,
        tint,
    ) {
        Some(t) => egui::Button::image(egui::load::SizedTexture::new(t.id(), Vec2::splat(px))),
        None => egui::Button::new("×"),
    };
    ui.add(btn)
}

fn open_in_default_app(path: &Path) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        wide.push(0);

        let mut op_open: Vec<u16> = std::ffi::OsStr::new("open").encode_wide().collect();
        op_open.push(0);

        unsafe {
            let res = ShellExecuteW(
                std::ptr::null_mut(),
                op_open.as_ptr(),
                wide.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            );
            // ShellExecuteW returns an instance handle > 32 on success; fall back
            // to `cmd /c start` so a failed association never silently drops a launch.
            if res as usize <= 32 {
                use std::os::windows::process::CommandExt;
                let _ = std::process::Command::new("cmd")
                    .args(["/c", "start", "", path.to_string_lossy().as_ref()])
                    .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
                    .spawn();
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(path).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
}

fn open_in_default_editor(path: &Path) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        wide.push(0);

        let mut op_edit: Vec<u16> = std::ffi::OsStr::new("edit").encode_wide().collect();
        op_edit.push(0);

        unsafe {
            let res = ShellExecuteW(
                std::ptr::null_mut(),
                op_edit.as_ptr(),
                wide.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            );
            if res as usize <= 32 {
                open_in_default_app(path);
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("-t")
            .arg(path)
            .spawn();
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(editor) = std::env::var("EDITOR") {
            let _ = std::process::Command::new(editor).arg(path).spawn();
        } else {
            let _ = std::process::Command::new("xdg-open").arg(path).spawn();
        }
    }
}

fn reveal_in_file_manager(path: &Path) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer")
            .arg(format!("/select,\"{}\"", path.display()))
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn();
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(parent) = path.parent() {
            let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
        } else {
            let _ = std::process::Command::new("xdg-open").arg(path).spawn();
        }
    }
}

fn open_terminal_in_dir(path: &Path) {
    #[cfg(target_os = "windows")]
    {
        let wt_result = std::process::Command::new("wt.exe")
            .arg("-d")
            .arg(path)
            .spawn();

        if wt_result.is_err() {
            let _ = std::process::Command::new("powershell.exe")
                .arg("-NoExit")
                .arg("-Command")
                .arg(format!("Set-Location -LiteralPath '{}'", path.display()))
                .spawn();
        }
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("-a")
            .arg("Terminal")
            .arg(path)
            .spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let terminals = [
            "x-terminal-emulator",
            "gnome-terminal",
            "konsole",
            "alacritty",
            "kitty",
            "xfce4-terminal",
        ];
        let mut launched = false;
        for term in terminals {
            if std::process::Command::new(term)
                .current_dir(path)
                .spawn()
                .is_ok()
            {
                launched = true;
                break;
            }
        }
        if !launched {
            let _ = std::process::Command::new("sh").current_dir(path).spawn();
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Shared context menus — Windows 11 Explorer / macOS Finder / GNOME Nautilus
// wording from `file_menu::MenuAction`, glyphs from the active icon pack.
//
// The menu builders are pure: they render entries via `file_menu` and RETURN
// intents. All mutations of ExplorerView state happen in the caller after the
// popup closes — popups never touch `&mut self` directly.
// ═══════════════════════════════════════════════════════════════════════════

use crate::core::config::{CheckboxMode, MenuStyle};
use crate::core::icon_packs::{IconGlyph, IconPack};
use crate::gui::file_menu as fm;

/// Everything the empty-canvas menu may ask the caller to do.
#[derive(Default, Debug, Clone)]
pub struct CanvasMenuIntents {
    pub new_folder: bool,
    pub refresh: bool,
    pub go_up: bool,
    pub select_all: bool,
    pub clear_selection: bool,
    /// Clipboard sources the caller should paste into the current directory.
    pub paste: Option<Vec<PathBuf>>,
    pub terminal: bool,
    pub open_in_file_manager: bool,
    pub properties_dir: bool,
}

/// Everything an item (row/tile) menu may ask the caller to do.
#[derive(Default, Debug, Clone)]
pub struct ItemMenuIntents {
    pub nav_into_dir: Option<PathBuf>,
    pub peek: Option<FileItem>,
    pub transcode: Option<(PathBuf, String)>,
    pub exam_conform: Option<PathBuf>,
    pub pack: Option<Vec<PathBuf>>,
    pub inspect: Option<PathBuf>,
    pub extract: Option<PathBuf>,
    pub rename: Option<(PathBuf, String)>,
    pub properties: Option<FileItem>,
    pub deleted: Option<PathBuf>,
    pub cut: Option<Vec<PathBuf>>,
    pub copy: Option<Vec<PathBuf>>,
}

/// Extension classification shared by both item menus.
struct ItemKind {
    is_media: bool,
    is_printable: bool,
    is_image: bool,
    is_archive: bool,
}

fn item_kind(extension: &str) -> ItemKind {
    let ext = extension.to_ascii_uppercase();
    ItemKind {
        is_media: matches!(
            ext.as_str(),
            "MP3" | "FLAC" | "WAV" | "AAC" | "M4A" | "OGG" | "OPUS" | "MP4" | "MKV" | "MOV"
                | "WEBM" | "AVI" | "M4V"
        ),
        is_printable: matches!(
            ext.as_str(),
            "PDF" | "DOC" | "DOCX" | "XLS" | "XLSX" | "PPT" | "PPTX" | "TXT" | "RTF"
        ),
        is_image: matches!(
            ext.as_str(),
            "JPG" | "JPEG" | "PNG" | "WEBP" | "GIF" | "AVIF" | "BMP" | "ICO" | "TIFF" | "HEIC"
                | "PSD"
        ),
        is_archive: matches!(
            ext.as_str(),
            "ZIP" | "7Z" | "TAR" | "GZ" | "XZ" | "ZST" | "BZ2" | "RAR"
        ),
    }
}

/// A menu row honouring the enabled state without leaving a highlight box.
fn menu_row(
    menu_ui: &mut Ui,
    enabled: bool,
    pack: IconPack,
    action: fm::MenuAction,
    style: MenuStyle,
    tint: Color32,
) -> egui::Response {
    menu_ui
        .add_enabled_ui(enabled, |ui| fm::menu_item(ui, pack, action, style, tint))
        .inner
}

fn menu_row_glyph(
    menu_ui: &mut Ui,
    enabled: bool,
    pack: IconPack,
    glyph: IconGlyph,
    label: &str,
    tint: Color32,
) -> egui::Response {
    menu_ui
        .add_enabled_ui(enabled, |ui| {
            fm::menu_item_glyph(ui, pack, glyph, label, tint)
        })
        .inner
}

impl ExplorerView {
    /// Paste everything currently staged in Mossambi's inner clipboard into
    /// the current directory (fresh-name resolution, cut ⇒ move).
    pub fn paste_from_clipboard(&mut self) {
        let srcs: Vec<PathBuf> = self.clipboard_paths.iter().cloned().collect();
        let cut = self.clipboard_is_cut;
        for src in &srcs {
            if let Some(name) = src.file_name() {
                let dest = self.current_dir.join(name);
                if cut {
                    log_result("Paste move", src, move_entry(src, &dest));
                } else {
                    let unique = resolve_unique_paste_path(&dest);
                    log_result("Paste copy", src, copy_entry_recursive(src, &unique));
                }
            }
        }
        if cut {
            self.clipboard_paths.clear();
            self.clipboard_is_cut = false;
        }
        self.navigate_to(self.current_dir.clone());
    }

    /// Shared empty-canvas context menu for the Details and Grid canvases.
    pub fn run_canvas_context_menu(
        menu_ui: &mut Ui,
        pack: IconPack,
        style: MenuStyle,
        tint: Color32,
        has_clipboard: bool,
        has_entries: bool,
    ) -> CanvasMenuIntents {
        let mut intent = CanvasMenuIntents::default();
        fm::menu_header(menu_ui, "Explorer", tint);
        menu_ui.separator();

        if menu_row(menu_ui, true, pack, fm::MenuAction::NewFolder, style, tint).clicked() {
            intent.new_folder = true;
            menu_ui.close_menu();
        }
        if menu_row(menu_ui, true, pack, fm::MenuAction::Refresh, style, tint).clicked() {
            intent.refresh = true;
            menu_ui.close_menu();
        }
        if menu_row_glyph(menu_ui, true, pack, IconGlyph::ActionUp, "Go to Parent Folder", tint)
            .clicked()
        {
            intent.go_up = true;
            menu_ui.close_menu();
        }

        menu_ui.separator();
        if menu_row(menu_ui, has_entries, pack, fm::MenuAction::SelectAll, style, tint).clicked() {
            intent.select_all = true;
            menu_ui.close_menu();
        }
        if menu_row_glyph(menu_ui, true, pack, IconGlyph::ActionClose, "Clear Selection", tint)
            .clicked()
        {
            intent.clear_selection = true;
            menu_ui.close_menu();
        }
        if menu_row(menu_ui, has_clipboard, pack, fm::MenuAction::Paste, style, tint).clicked() {
            intent.paste = Some(Vec::new()); // caller reads clipboard_paths itself
            menu_ui.close_menu();
        }

        menu_ui.separator();
        if menu_row(menu_ui, true, pack, fm::MenuAction::Terminal, style, tint).clicked() {
            intent.terminal = true;
            menu_ui.close_menu();
        }
        if menu_row(menu_ui, true, pack, fm::MenuAction::Reveal, style, tint).clicked() {
            intent.open_in_file_manager = true;
            menu_ui.close_menu();
        }
        if menu_row(menu_ui, true, pack, fm::MenuAction::Properties, style, tint).clicked() {
            intent.properties_dir = true;
            menu_ui.close_menu();
        }
        intent
    }

    /// Shared file/folder item context menu for Details rows and Grid tiles.
    #[allow(clippy::too_many_lines)]
    pub fn run_item_context_menu(
        menu_ui: &mut Ui,
        pack: IconPack,
        style: MenuStyle,
        tint: Color32,
        item: &FileItem,
    ) -> ItemMenuIntents {
        let mut intent = ItemMenuIntents::default();
        let is_dir = item.is_dir;
        let spec = resolve_file_icon(&item.name, &item.extension, is_dir);
        let kind = item_kind(&item.extension);
        let path: PathBuf = item.path.clone();

        fm::menu_header(menu_ui, &item.name, tint);
        menu_ui.separator();

        // ── Open group ──
        if is_dir {
            if menu_row(menu_ui, true, pack, fm::MenuAction::OpenFolder, style, tint).clicked() {
                intent.nav_into_dir = Some(path.clone());
                menu_ui.close_menu();
            }
        } else {
            if menu_row(menu_ui, true, pack, fm::MenuAction::OpenFile, style, tint).clicked() {
                open_in_default_app(&path);
                menu_ui.close_menu();
            }
            if menu_row(menu_ui, true, pack, fm::MenuAction::Edit, style, tint).clicked() {
                open_in_default_editor(&path);
                menu_ui.close_menu();
            }
            if kind.is_media
                && menu_row(menu_ui, true, pack, fm::MenuAction::Play, style, tint).clicked()
            {
                crate::gui::context_menu::play_file(&path);
                menu_ui.close_menu();
            }
            if kind.is_printable
                && menu_row(menu_ui, true, pack, fm::MenuAction::Print, style, tint).clicked()
            {
                crate::gui::context_menu::print_file(&path);
                menu_ui.close_menu();
            }
            if menu_row(menu_ui, true, pack, fm::MenuAction::QuickLook, style, tint).clicked() {
                intent.peek = Some(item.clone());
                menu_ui.close_menu();
            }
        }

        menu_ui.separator();

        // ── Media/image group (files only; directories are packed instead) ──
        if !is_dir {
            if kind.is_image
                && menu_row(menu_ui, true, pack, fm::MenuAction::Wallpaper, style, tint).clicked()
            {
                crate::gui::context_menu::set_desktop_wallpaper(&path);
                menu_ui.close_menu();
            }
            if kind.is_image
                && menu_row(menu_ui, true, pack, fm::MenuAction::Rotate, style, tint).clicked()
            {
                crate::gui::context_menu::rotate_image_90(&path);
                menu_ui.close_menu();
            }
            if is_image_extension(&item.extension) || kind.is_media || kind.is_printable {
                menu_ui.menu_button("Convert", |sub_ui| {
                    sub_ui.set_min_width(180.0);
                    for fmt in ["JPG", "PNG", "WEBP", "PDF", "MP3", "FLAC"] {
                        if fm::menu_item_labeled(
                            sub_ui,
                            pack,
                            fm::MenuAction::Transcode,
                            &format!("Convert to {fmt}"),
                            tint,
                        )
                        .clicked()
                        {
                            intent.transcode = Some((path.clone(), fmt.to_string()));
                            sub_ui.close_menu();
                        }
                    }
                    sub_ui.separator();
                    if fm::menu_item_labeled(
                        sub_ui,
                        pack,
                        fm::MenuAction::Transcode,
                        "All formats (batch)",
                        tint,
                    )
                    .clicked()
                    {
                        intent.transcode = Some((path.clone(), "ALL".to_string()));
                        sub_ui.close_menu();
                    }
                });
            }
        }
        if !is_dir
            && kind.is_archive
            && menu_row(menu_ui, true, pack, fm::MenuAction::Extract, style, tint).clicked()
        {
            intent.extract = Some(path.clone());
            menu_ui.close_menu();
        }
        if menu_row(menu_ui, true, pack, fm::MenuAction::PackZip, style, tint).clicked() {
            intent.pack = Some(vec![path.clone()]);
            menu_ui.close_menu();
        }
        if !is_dir {
            if menu_row(menu_ui, true, pack, fm::MenuAction::Conform, style, tint).clicked() {
                intent.exam_conform = Some(path.clone());
                menu_ui.close_menu();
            }
            if menu_row(menu_ui, true, pack, fm::MenuAction::Inspect, style, tint).clicked() {
                intent.inspect = Some(path.clone());
                menu_ui.close_menu();
            }
        }

        menu_ui.separator();

        // ── Shell helpers row ──
        if menu_row_glyph(menu_ui, true, pack, IconGlyph::ActionPath, fm::MenuAction::CopyPath.label(style), tint)
            .clicked()
        {
            menu_ui.output_mut(|o| o.copied_text = path.to_string_lossy().to_string());
            menu_ui.close_menu();
        }
        if menu_row(menu_ui, true, pack, fm::MenuAction::Reveal, style, tint).clicked() {
            reveal_in_file_manager(&path);
            menu_ui.close_menu();
        }
        if menu_row(menu_ui, true, pack, fm::MenuAction::Terminal, style, tint).clicked() {
            let dir_target = if is_dir {
                path.clone()
            } else {
                path.parent().unwrap_or(&path).to_path_buf()
            };
            open_terminal_in_dir(&dir_target);
            menu_ui.close_menu();
        }
        if menu_row_glyph(menu_ui, true, pack, IconGlyph::ActionShortcut, fm::MenuAction::Shortcut.label(style), tint)
            .clicked()
        {
            crate::gui::context_menu::create_shortcut(&path);
            menu_ui.close_menu();
        }

        // ── Clipboard group ──
        menu_ui.separator();
        if menu_row(menu_ui, true, pack, fm::MenuAction::Cut, style, tint).clicked() {
            intent.cut = Some(vec![path.clone()]);
            menu_ui.close_menu();
        }
        if menu_row(menu_ui, true, pack, fm::MenuAction::Copy, style, tint).clicked() {
            intent.copy = Some(vec![path.clone()]);
            menu_ui.close_menu();
        }

        // ── Danger group ──
        menu_ui.separator();
        if menu_row(menu_ui, true, pack, fm::MenuAction::Rename, style, tint).clicked() {
            if let Some(name) = path.file_name() {
                intent.rename = Some((path.clone(), name.to_string_lossy().to_string()));
            }
            menu_ui.close_menu();
        }
        if menu_row(menu_ui, true, pack, fm::MenuAction::Delete, style, tint).clicked() {
            intent.deleted = Some(path.clone());
            menu_ui.close_menu();
        }
        if crate::gui::context_menu::win_defender_available()
            && menu_row(menu_ui, true, pack, fm::MenuAction::Defender, style, tint).clicked()
        {
            crate::gui::context_menu::scan_with_defender(&path);
            menu_ui.close_menu();
        }

        menu_ui.separator();
        if menu_row(menu_ui, true, pack, fm::MenuAction::Properties, style, tint).clicked() {
            intent.properties = Some(item.clone());
            menu_ui.close_menu();
        }
        intent
    }

    /// Finder-style selection checkbox (checkbox-only selection disclosure).
    fn paint_selection_checkbox(
        ui: &Ui,
        rect: Rect,
        checked: bool,
        alpha: f32,
        palette: &ThemePalette,
    ) {
        let alpha = alpha.clamp(0.0, 1.0);
        let rounding = Rounding::same(4.5);
        let stroke_col = if checked { palette.accent } else { palette.text_muted };
        let stroke_col = Color32::from_rgba_premultiplied(
            (stroke_col.r() as f32 * alpha) as u8,
            (stroke_col.g() as f32 * alpha) as u8,
            (stroke_col.b() as f32 * alpha) as u8,
            (stroke_col.a() as f32 * alpha) as u8,
        );
        ui.painter().rect_stroke(rect, rounding, Stroke::new(1.4, stroke_col));
        if checked {
            let fill = Color32::from_rgba_premultiplied(
                (palette.accent.r() as f32 * alpha) as u8,
                (palette.accent.g() as f32 * alpha) as u8,
                (palette.accent.b() as f32 * alpha) as u8,
                235,
            );
            ui.painter().rect_filled(rect, rounding, fill);
            let dot = rect.center();
            let dot_r = (rect.width() * 0.20).max(1.5);
            ui.painter()
                .circle_filled(dot, dot_r, Color32::WHITE.gamma_multiply(alpha.max(0.15)));
        }
    }
}

// ═══════ end of explorer_view.rs shared-menu engine ═══════
