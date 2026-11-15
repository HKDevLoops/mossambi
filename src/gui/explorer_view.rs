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
    copy_entry_recursive, format_file_size, fuzzy_match, get_system_drives, is_library_folder_name,
    list_directory_sync, move_entry, parse_breadcrumbs, pluralize_items, remove_entry_recursive,
    resolve_file_icon, resolve_unique_paste_path, spawn_async_dir_scan, DriveInfo, FileItem,
    GestureKind, KinematicGestureTracker, ScanBatch,
};
use crate::gui::design;
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

/// ASCII funnel glyph for the compact filter popover button.
const FILTER_FUNNEL_GLYPH: &str = "|v";

/// Optional-comparison helper that sorts missing values last in the
/// natural (ascending) direction.
pub fn cmp_opt_last<T: Ord>(a: &Option<T>, b: &Option<T>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(x), Some(y)) => x.cmp(y),
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
    }
}

/// Derived Kind sort key - the same category the details table paints.
fn kind_key(item: &FileItem) -> &'static str {
    resolve_file_icon(&item.name, &item.extension, item.is_dir).category
}

/// Pure Miller lineage helper: the ancestor chain of a directory,
/// root-first, excluding the directory itself (Finder column view
/// renders this chain to the LEFT of the current directory column).
/// Empty for a filesystem root and for relative paths (Miller
/// columns are only meaningful for absolute paths).
pub fn ancestor_chain(dir: &Path) -> Vec<PathBuf> {
    if !dir.is_absolute() {
        return Vec::new();
    }
    let mut chain: Vec<PathBuf> = dir.ancestors().skip(1).map(|p| p.to_path_buf()).collect();
    chain.reverse();
    chain
}

/// Pure Miller lineage decision for a navigation target relative to
/// the previously current directory and the opened descendant stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MillerLineage {
    /// Target is the same directory (refresh / fs-op completion):
    /// keep the entire chain and stack untouched.
    KeepStack,
    /// Target is the stack entry at index k: that folder becomes the
    /// current directory, stack[k+1..] stay opened as its
    /// descendants, and stack[0..=k] dissolve into the ancestor
    /// chain.
    ShiftTo(usize),
    /// Target is unrelated to the opened branch: the stack closes
    /// (up-navigation, sideways navigation, bookmarks, fresh dirs).
    ClearStack,
}

/// Classifies a navigation target for the Miller column view.
pub fn lineage_policy(
    new_dir: &Path,
    old_dir: &Path,
    stack: &[(PathBuf, Vec<FileItem>)],
) -> MillerLineage {
    if new_dir == old_dir {
        return MillerLineage::KeepStack;
    }
    if let Some(k) = stack.iter().position(|(p, _)| p.as_path() == new_dir) {
        return MillerLineage::ShiftTo(k);
    }
    MillerLineage::ClearStack
}

/// Which Miller column a shared row renderer is painting: ancestor
/// chain columns left of the current directory, the current
/// directory column itself, or an opened descendant stack column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnRole {
    Ancestor,
    Current,
    Descendant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortColumn {
    Name,
    Size,
    Extension,
    Date,
    Permissions,
    DateCreated,
    Hidden,
    Symlink,
    SymlinkTarget,
    Path,
    Kind,
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
            Self::Ascending => "^",
            Self::Descending => "v",
        }
    }
}

/// Details (list) view columns, Finder parity. `Name` is locked: it
/// is always present and always first; every other column can be
/// shown, hidden, resized, sorted and filtered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DetailColumn {
    Name,
    DateModified,
    Size,
    Kind,
    DateCreated,
    Permissions,
    Hidden,
    Symlink,
    SymlinkTarget,
    Extension,
    Path,
}

impl DetailColumn {
    /// Canonical column set in persisted order.
    pub fn all() -> [DetailColumn; 11] {
        [
            Self::Name,
            Self::DateModified,
            Self::Size,
            Self::Kind,
            Self::DateCreated,
            Self::Permissions,
            Self::Hidden,
            Self::Symlink,
            Self::SymlinkTarget,
            Self::Extension,
            Self::Path,
        ]
    }

    /// Stable persisted id (config key). Variant order IS the id.
    pub fn id(self) -> u8 {
        self as u8
    }

    pub fn from_id(id: u8) -> Option<Self> {
        if id < 11 {
            Some(Self::all()[id as usize])
        } else {
            None
        }
    }

    /// Plain-ASCII header label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::DateModified => "Date Modified",
            Self::Size => "Size",
            Self::Kind => "Kind",
            Self::DateCreated => "Date Created",
            Self::Permissions => "Permissions",
            Self::Hidden => "Hidden",
            Self::Symlink => "Symlink",
            Self::SymlinkTarget => "Symlink Target",
            Self::Extension => "Extension",
            Self::Path => "Path",
        }
    }

    /// Finder-parity default width before any resize.
    pub fn default_w(self) -> f32 {
        match self {
            Self::Name => 260.0,
            Self::DateModified => 170.0,
            Self::Size => 110.0,
            Self::Kind => 140.0,
            Self::DateCreated => 170.0,
            Self::Permissions => 110.0,
            Self::Hidden => 80.0,
            Self::Symlink => 90.0,
            Self::SymlinkTarget => 200.0,
            Self::Extension => 100.0,
            Self::Path => 240.0,
        }
    }

    /// Hard minimum during resize.
    pub fn min_w(self) -> f32 {
        match self {
            Self::Name => 120.0,
            Self::DateModified | Self::DateCreated => 100.0,
            Self::Size => 80.0,
            Self::Kind | Self::Permissions => 90.0,
            Self::Hidden | Self::Symlink => 60.0,
            Self::SymlinkTarget => 100.0,
            Self::Extension => 60.0,
            Self::Path => 120.0,
        }
    }

    pub fn aligns_right(self) -> bool {
        matches!(self, Self::Size)
    }

    /// The SortColumn axis driven by this column header.
    pub fn sort_column(self) -> SortColumn {
        match self {
            Self::Name => SortColumn::Name,
            Self::DateModified => SortColumn::Date,
            Self::Size => SortColumn::Size,
            Self::Kind => SortColumn::Kind,
            Self::DateCreated => SortColumn::DateCreated,
            Self::Permissions => SortColumn::Permissions,
            Self::Hidden => SortColumn::Hidden,
            Self::Symlink => SortColumn::Symlink,
            Self::SymlinkTarget => SortColumn::SymlinkTarget,
            Self::Extension => SortColumn::Extension,
            Self::Path => SortColumn::Path,
        }
    }

    /// Finder default visible set: Name, Date Modified, Size, Kind.
    pub fn default_visible_ids() -> [u8; 4] {
        [0, 1, 2, 3]
    }
}

/// Visible-column order plus per-column widths for the details
/// table. Filters are session-scoped and intentionally not part of
/// this struct; only visibility and widths persist to MossambiConfig.
#[derive(Debug, Clone)]
pub struct ColumnLayout {
    pub visible: Vec<DetailColumn>,
    pub widths: std::collections::BTreeMap<u8, f32>,
}

impl Default for ColumnLayout {
    fn default() -> Self {
        Self::finder_default()
    }
}

impl ColumnLayout {
    /// Finder-parity defaults: Name, Date Modified, Size, Kind with
    /// each column's default width.
    pub fn finder_default() -> Self {
        let visible = DetailColumn::default_visible_ids()
            .iter()
            .filter_map(|&id| DetailColumn::from_id(id))
            .collect();
        let widths = DetailColumn::all()
            .iter()
            .map(|c| (c.id(), c.default_w()))
            .collect();
        Self { visible, widths }
    }

    /// Effective width of a column (persisted width or its default).
    pub fn width(&self, col: DetailColumn) -> f32 {
        self.widths
            .get(&col.id())
            .copied()
            .unwrap_or(col.default_w())
    }

    pub fn set_width(&mut self, col: DetailColumn, w: f32) {
        self.widths.insert(col.id(), w.max(col.min_w()));
    }

    /// Name is locked: always visible and always first.
    pub fn is_locked(col: DetailColumn) -> bool {
        col == DetailColumn::Name
    }

    pub fn show(&mut self, col: DetailColumn) {
        if !self.visible.contains(&col) {
            self.visible.push(col);
        }
    }

    pub fn hide(&mut self, col: DetailColumn) {
        if Self::is_locked(col) {
            return;
        }
        self.visible.retain(|c| *c != col);
    }

    /// Rebuilds from persisted config values; unknown ids are dropped
    /// and Name is re-locked first. An empty persisted list falls back
    /// to the Finder defaults (legacy/missing keys behave the same).
    pub fn from_config(visible_ids: &[u8], widths: &std::collections::BTreeMap<u8, u32>) -> Self {
        let mut layout = Self::finder_default();
        if !visible_ids.is_empty() {
            let mut visible: Vec<DetailColumn> = visible_ids
                .iter()
                .filter_map(|&id| DetailColumn::from_id(id))
                .collect();
            // Name locked first, exactly once.
            visible.retain(|c| *c != DetailColumn::Name);
            visible.insert(0, DetailColumn::Name);
            layout.visible = visible;
        }
        for (id, w) in widths {
            if let Some(col) = DetailColumn::from_id(*id) {
                layout.set_width(col, *w as f32);
            }
        }
        layout
    }

    /// Persists back to config types.
    pub fn to_config(&self) -> (Vec<u8>, std::collections::BTreeMap<u8, u32>) {
        (
            self.visible.iter().map(|c| c.id()).collect(),
            self.widths.iter().map(|(id, w)| (*id, *w as u32)).collect(),
        )
    }
}

/// Tri-state boolean filter (Hidden / Symlink columns).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriState {
    Any,
    Yes,
    No,
}

/// Date bucket filter (Date Modified / Date Created columns).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateBucket {
    Any,
    Today,
    ThisWeek,
    ThisMonth,
    Older,
}

/// Session-scoped per-column filter value. Values are keyed by
/// DetailColumn id in ExplorerView.column_filters and compose AND.
/// Filters are intentionally not persisted.
#[derive(Debug, Clone, PartialEq)]
pub enum ColumnFilterValue {
    /// Case-insensitive substring on the column's text.
    Text(String),
    /// Tri-state on Hidden / Symlink columns.
    TriState(TriState),
    /// Minimum size in bytes (files only).
    SizeMin(u64),
    /// Maximum size in bytes (files only).
    SizeMax(u64),
    /// Date bucket on Date Modified / Date Created columns.
    Date(DateBucket),
}

/// Bucket membership test against a reference "today". Pure so tests
/// can drive it with fixed dates.
pub fn date_in_bucket(
    date: chrono::NaiveDate,
    today: chrono::NaiveDate,
    bucket: DateBucket,
) -> bool {
    use chrono::Datelike;
    match bucket {
        DateBucket::Any => true,
        DateBucket::Today => date == today,
        DateBucket::ThisWeek => date > today - chrono::Duration::days(7),
        DateBucket::ThisMonth => date.year() == today.year() && date.month() == today.month(),
        DateBucket::Older => date < today.with_day(1).unwrap_or(today),
    }
}

fn column_text(item: &FileItem, col: DetailColumn) -> String {
    match col {
        DetailColumn::Name => item.name.clone(),
        DetailColumn::Kind => kind_key(item).to_string(),
        DetailColumn::Extension => item.extension.clone(),
        DetailColumn::SymlinkTarget => item
            .symlink_target
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
        DetailColumn::Path => item.path.to_string_lossy().to_string(),
        _ => String::new(),
    }
}

fn column_bool(item: &FileItem, col: DetailColumn) -> bool {
    match col {
        DetailColumn::Hidden => item.is_hidden,
        DetailColumn::Symlink => item.is_symlink,
        _ => false,
    }
}

fn column_time(item: &FileItem, col: DetailColumn) -> Option<std::time::SystemTime> {
    match col {
        DetailColumn::DateModified => item.modified,
        DetailColumn::DateCreated => item.created,
        _ => None,
    }
}

/// True when the item passes the given column filter value.
pub fn matches_column_filter(
    item: &FileItem,
    col: DetailColumn,
    value: &ColumnFilterValue,
) -> bool {
    match value {
        ColumnFilterValue::Text(q) => {
            let q = q.trim();
            if q.is_empty() {
                return true;
            }
            let hay = column_text(item, col);
            let q_lower = q.to_lowercase();
            hay.to_lowercase().contains(&q_lower)
        }
        ColumnFilterValue::TriState(tri) => match tri {
            TriState::Any => true,
            TriState::Yes => column_bool(item, col),
            TriState::No => !column_bool(item, col),
        },
        ColumnFilterValue::SizeMin(min) => !item.is_dir && item.size_bytes >= *min,
        ColumnFilterValue::SizeMax(max) => !item.is_dir && item.size_bytes <= *max,
        ColumnFilterValue::Date(bucket) => {
            if *bucket == DateBucket::Any {
                return true;
            }
            let Some(time) = column_time(item, col) else {
                return false;
            };
            let dt: chrono::DateTime<chrono::Local> = time.into();
            let today = chrono::Local::now().date_naive();
            date_in_bucket(dt.date_naive(), today, *bucket)
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
    /// Plain-ASCII label (the overhaul removed the emoji chips).
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Images => "Images",
            Self::Audio => "Audio",
            Self::Video => "Video",
            Self::Documents => "Docs",
            Self::Archives => "Archives",
            Self::Code => "Code",
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
    /// Decode resolution this entry was produced at; a cache hit
    /// requires a stored size >= the requested one so larger zoom
    /// levels re-decode instead of upscaling a small texture.
    max_px: u32,
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

/// Asynchronous filesystem operation dispatched to background worker
pub enum FsOperation {
    Copy {
        sources: Vec<PathBuf>,
        dest_dir: PathBuf,
    },
    Move {
        sources: Vec<PathBuf>,
        dest_dir: PathBuf,
    },
    Delete {
        targets: Vec<PathBuf>,
    },
}

/// Outcome of an asynchronous filesystem operation
pub struct FsOpResult {
    pub op_name: &'static str,
    pub count: usize,
    pub errors: Vec<String>,
}

pub struct ExplorerView {
    pub current_dir: PathBuf,
    pub entries: Vec<FileItem>,
    pub selected_paths: HashSet<PathBuf>,
    pub search_query: String,
    pub drives: Vec<DriveInfo>,
    /// Throttle for the periodic drive re-enumeration so free space,
    /// mounts, and unmounts stay current (5 s cadence).
    pub last_drives_refresh: Option<Instant>,

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
    pub column_filters: std::collections::BTreeMap<u8, ColumnFilterValue>,
    pub column_layout: ColumnLayout,
    pub sort_direction: SortDirection,
    pub category_filter: CategoryFilter,
    /// Active tag smart-list filter (from the sidebar Tags section).
    /// `None` means no tag filtering is applied.
    pub active_tag: Option<String>,

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

    // Async Background Filesystem Worker
    pub fs_worker_tx: Sender<FsOperation>,
    pub fs_worker_rx: Receiver<FsOpResult>,

    // Background Folder-Size Worker (Finder-style recursive byte totals)
    pub dir_sizes: std::collections::HashMap<PathBuf, (Option<std::time::SystemTime>, u64)>,
    pub dir_size_queued: HashSet<PathBuf>,
    pub dir_size_job_tx: Sender<PathBuf>,
    pub dir_size_rx: Receiver<crate::core::fs_utils::DirSizeResult>,
    pub dir_size_last_request: Option<Instant>,
    pub dir_size_cache_dirty: usize,
    pub dir_size_last_flush: Option<Instant>,

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
    pub column_stack: Vec<(PathBuf, Vec<FileItem>)>,
    /// Once-per-open listings for the breadcrumb child-folder popup
    /// (F-2 fix): cleared on every navigate_to.
    pub breadcrumb_popup_cache: std::collections::HashMap<PathBuf, Vec<FileItem>>,
    pub column_preview_item: Option<FileItem>,
    pub column_cache: std::collections::HashMap<PathBuf, Vec<FileItem>>,
    /// One-shot: the Miller hscroll should align to the deepest
    /// column on the next render (set by navigate_to and the
    /// stack-push site).
    pub needs_miller_scroll_align: bool,
    /// One-shot: the breadcrumb path bar should reveal the current
    /// segment on the next render (set by navigate_to when the
    /// directory actually changes).
    pub needs_breadcrumb_scroll_align: bool,

    // Windows 11 & macOS Home Dashboard View
    pub is_home_view: bool,
    pub show_internal_toolbar: bool,
    /// B-1 gate: set true every frame by the studio view while the
    /// command palette modal is open. Suppresses all explorer
    /// raw-key handling so palette typing and Enter never leak
    /// into the view underneath.
    pub palette_open: bool,

    // Finder-style long-hover preview (files & folders)
    hover_preview: Option<HoverPreviewState>,

    // Drop action menu (Windows/Finder style drag-and-drop popup)
    pub drop_menu: Option<DropMenuState>,

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
    /// Windows-Explorer zoom ladder: five Grid icon-size multiplier levels
    /// (0..=4, persisted). One notch below the smallest level crosses into
    /// the Details view via `zoom_ladder_step`; the auto-switched view mode
    /// itself stays session-only per the confirmed design.
    pub explorer_zoom: u8,
    /// Time of the last Grid<->Details zoom-ladder crossing, used to
    /// debounce touchpad wiggle at the boundary (`zoom_ladder_step`).
    zoom_cross_gate: Option<Instant>,
    /// Rubber-band marquee: drag origin + the selection snapshot it modifies.
    /// `Some` while a canvas rubber-band drag is in flight.
    marquee: Option<(Pos2, HashSet<PathBuf>)>,

    /// Latched once per press while a native drag hand-off is in flight
    /// (the drag left the window bounds); reset when the pointer is released.
    drag_out_in_flight: bool,

    /// Transient user-facing status note for drag hand-offs and other
    /// non-modal feedback (auto-fades after a few seconds).
    status_note: Option<(String, Instant)>,

    // Dual-Pane Split View Architecture (Dolphin / Total Commander style)
    pub split_view_enabled: bool,
    pub active_pane: usize,
    pub secondary_dir: PathBuf,
    pub secondary_entries: Vec<FileItem>,
    pub secondary_selected_paths: HashSet<PathBuf>,
    pub secondary_focused_row: usize,
    pub secondary_history: Vec<PathBuf>,
    pub secondary_history_idx: usize,

    // Right-Side Preview Inspector (Finder-style inspector pane)
    pub show_inspector_pane: bool,

    // Shift+Delete Permanent Deletion Confirmation
    pub permanent_delete_prompt: Option<Vec<PathBuf>>,

    // Finder-Style Multi-Item Batch Renamer
    pub batch_rename: BatchRenameState,

    // Real-Time Filesystem Directory Watchers
    pub dir_watcher: crate::core::DirWatcher,
    pub secondary_dir_watcher: crate::core::DirWatcher,
    pub row_density: crate::core::config::RowDensity,
    pub calculate_checksums: bool,
}

#[derive(Debug, Clone)]
pub struct BatchRenameState {
    pub is_open: bool,
    pub mode: usize,
    pub find_text: String,
    pub replace_text: String,
    pub prefix_text: String,
    pub suffix_text: String,
    pub base_name: String,
    pub start_index: usize,
    pub target_paths: Vec<PathBuf>,
}

impl Default for BatchRenameState {
    fn default() -> Self {
        Self {
            is_open: false,
            mode: 0,
            find_text: String::new(),
            replace_text: String::new(),
            prefix_text: String::new(),
            suffix_text: String::new(),
            base_name: "Item".to_string(),
            start_index: 1,
            target_paths: Vec::new(),
        }
    }
}

/// Spawns an independent, concurrent Mossambi explorer window at the specified directory.
pub fn spawn_new_explorer_window(dir: Option<&Path>) {
    if let Ok(exe) = std::env::current_exe() {
        let mut cmd = std::process::Command::new(exe);
        if let Some(d) = dir {
            cmd.arg(d);
        }
        let _ = cmd.spawn();
    }
}

impl Default for ExplorerView {
    fn default() -> Self {
        let (scan_tx, scan_rx) = channel();
        let (fs_worker_tx, fs_op_rx) = channel::<FsOperation>();
        let (fs_result_tx, fs_worker_rx) = channel::<FsOpResult>();

        std::thread::Builder::new()
            .name("mossambi-fs-worker".into())
            .spawn(move || {
                while let Ok(op) = fs_op_rx.recv() {
                    match op {
                        FsOperation::Copy { sources, dest_dir } => {
                            let mut count = 0;
                            let mut errors = Vec::new();
                            for src in sources {
                                if let Some(name) = src.file_name() {
                                    let dest = dest_dir.join(name);
                                    let unique = resolve_unique_paste_path(&dest);
                                    match copy_entry_recursive(&src, &unique) {
                                        Ok(_) => {
                                            crate::core::action_journal::record_copy(
                                                src.clone(),
                                                unique,
                                            );
                                            count += 1;
                                        }
                                        Err(e) => errors.push(format!("{}: {}", src.display(), e)),
                                    }
                                }
                            }
                            let _ = fs_result_tx.send(FsOpResult {
                                op_name: "Copied",
                                count,
                                errors,
                            });
                        }
                        FsOperation::Move { sources, dest_dir } => {
                            let mut count = 0;
                            let mut errors = Vec::new();
                            for src in sources {
                                if let Some(name) = src.file_name() {
                                    let dest = dest_dir.join(name);
                                    match move_entry(&src, &dest) {
                                        Ok(_) => {
                                            crate::core::action_journal::record_move(
                                                src.clone(),
                                                dest,
                                            );
                                            count += 1;
                                        }
                                        Err(e) => errors.push(format!("{}: {}", src.display(), e)),
                                    }
                                }
                            }
                            let _ = fs_result_tx.send(FsOpResult {
                                op_name: "Moved",
                                count,
                                errors,
                            });
                        }
                        FsOperation::Delete { targets } => {
                            let count = targets.len();
                            let mut errors = Vec::new();
                            if let Err(e) = crate::core::action_journal::stage_delete(&targets) {
                                errors.push(e);
                            }
                            let _ = fs_result_tx.send(FsOpResult {
                                op_name: "Deleted",
                                count,
                                errors,
                            });
                        }
                    }
                }
            })
            .ok();

        // Two persistent folder-size workers sharing one job receiver:
        // Finder-style recursive byte totals computed off the UI thread
        // with strict depth/entry budgets.
        let (dir_size_job_tx, dir_size_job_rx) = channel::<PathBuf>();
        let dir_size_shared_rx = Arc::new(std::sync::Mutex::new(dir_size_job_rx));
        let (dir_size_res_tx, dir_size_rx) = channel::<crate::core::fs_utils::DirSizeResult>();
        for n in 0..2 {
            let job_rx = Arc::clone(&dir_size_shared_rx);
            let res_tx = dir_size_res_tx.clone();
            if let Err(e) = std::thread::Builder::new()
                .name(format!("mossambi-dir-size-{n}"))
                .spawn(move || loop {
                    let job = {
                        let guard = match job_rx.lock() {
                            Ok(g) => g,
                            Err(_) => break,
                        };
                        guard.recv()
                    };
                    match job {
                        Ok(path) => {
                            let mtime = std::fs::metadata(&path)
                                .ok()
                                .and_then(|m| m.modified().ok());
                            let total = crate::core::fs_utils::compute_dir_size(&path, 8, 100_000);
                            let _ = res_tx.send(crate::core::fs_utils::DirSizeResult {
                                path,
                                mtime,
                                total,
                            });
                        }
                        Err(_) => break,
                    }
                })
            {
                log::warn!("Could not spawn folder-size worker {n}: {e}");
            }
        }

        let home_opt = crate::core::platform_paths::home_dir_opt();
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
            icon: if is_home { "H" } else { "D" },
            path: current_dir.clone(),
            is_home,
            search_query: String::new(),
        };

        let cfg = crate::core::MossambiConfig::load();
        let mut view = Self {
            current_dir: current_dir.clone(),
            entries: Vec::new(),
            selected_paths: HashSet::new(),
            search_query: String::new(),
            drives,
            last_drives_refresh: None,
            tabs: vec![initial_tab],
            active_tab_index: 0,
            next_tab_id: 2,
            history: vec![current_dir.clone()],
            history_idx: 0,
            view_mode: match cfg.default_explorer_layout {
                2 => ViewMode::Grid,
                3 => ViewMode::Columns,
                _ => ViewMode::Details,
            },
            icon_size: 96.0,
            sort_column: SortColumn::Name,
            column_filters: std::collections::BTreeMap::new(),
            column_layout: ColumnLayout::from_config(
                &cfg.explorer_columns,
                &cfg.explorer_column_widths,
            ),
            sort_direction: SortDirection::Ascending,
            category_filter: CategoryFilter::All,
            active_tag: None,
            is_editing_path: false,
            path_edit_buffer: current_dir.to_string_lossy().to_string(),
            renaming_path: None,
            show_new_folder_dialog: false,
            is_home_view: home_opt.is_some(),
            show_internal_toolbar: false,
            palette_open: false,
            new_folder_buffer: "New Folder".to_string(),
            clipboard_paths: Vec::new(),
            clipboard_is_cut: false,
            scan_gen: Arc::clone(&scan_gen),
            scan_tx: scan_tx.clone(),
            scan_rx,
            fs_worker_tx,
            fs_worker_rx,
            dir_sizes: crate::core::fs_utils::load_dir_size_cache_from(
                &crate::core::fs_utils::dir_size_cache_path(),
            ),
            dir_size_queued: HashSet::new(),
            dir_size_job_tx,
            dir_size_rx,
            dir_size_last_request: None,
            dir_size_cache_dirty: 0,
            dir_size_last_flush: None,
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
            column_stack: Vec::new(),
            breadcrumb_popup_cache: std::collections::HashMap::new(),
            column_preview_item: None,
            column_cache: std::collections::HashMap::new(),
            needs_miller_scroll_align: false,
            needs_breadcrumb_scroll_align: false,
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
            zoom_cross_gate: None,
            marquee: None,
            drag_out_in_flight: false,
            status_note: None,
            split_view_enabled: false,
            active_pane: 0,
            secondary_dir: current_dir.clone(),
            secondary_entries: Vec::new(),
            secondary_selected_paths: HashSet::new(),
            secondary_focused_row: 0,
            secondary_history: vec![current_dir.clone()],
            secondary_history_idx: 0,
            show_inspector_pane: false,
            permanent_delete_prompt: None,
            batch_rename: BatchRenameState::default(),
            dir_watcher: crate::core::DirWatcher::new(),
            secondary_dir_watcher: crate::core::DirWatcher::new(),
            row_density: cfg.row_density,
            calculate_checksums: cfg.calculate_checksums,
            drop_menu: None,
        };

        view.navigate_to(current_dir);
        view
    }
}

impl ExplorerView {
    /// The five Grid zoom steps (Windows Explorer parity), multiplied over
    /// the user's base grid icon size: Extra-small -> Small -> Medium ->
    /// Large -> Extra-large. The Details form sits one notch below the
    /// smallest step and is reached through `zoom_ladder_step`.
    pub const EXPLORER_ZOOM_LEVELS: [f32; 5] = [0.60, 0.85, 1.0, 1.30, 1.70];

    /// Debounce window for Grid<->Details zoom-ladder crossings: wheel
    /// wiggle inside this window is swallowed (touchpad jitter guard).
    pub const ZOOM_CROSS_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(150);

    /// Human readable label for a zoom level (Explorer's status-bar wording).
    pub fn zoom_level_name(level: u8) -> &'static str {
        match level {
            0 => "Extra small icons",
            1 => "Small icons",
            2 => "Medium icons",
            3 => "Large icons",
            _ => "Extra large icons",
        }
    }

    /// One Ctrl+Scroll notch on the zoom ladder (Windows Explorer parity,
    /// user-confirmed design): five Grid icon-size levels 0..=4; one notch
    /// below the smallest grid size crosses into the Details view and one
    /// notch up from Details returns to the smallest grid size. Columns is
    /// excluded from the ladder entirely, and mode crossings are debounced
    /// so touchpad wiggle at the boundary cannot flicker between modes.
    pub fn zoom_ladder_step(&mut self, step: i32) {
        if step == 0 || self.view_mode == ViewMode::Columns {
            return; // no-op notch, and Columns stays outside the ladder
        }
        if self.view_mode == ViewMode::Details {
            if step > 0 && self.zoom_cross_open() {
                self.view_mode = ViewMode::Grid;
                self.explorer_zoom = 0; // re-enter at the smallest grid size
                self.zoom_cross_gate = Some(Instant::now());
                log::debug!("zoom ladder: Details -> Grid (extra small)");
            }
            return; // Details is the bottom of the ladder
        }
        if step < 0 && self.explorer_zoom == 0 {
            if self.zoom_cross_open() {
                self.view_mode = ViewMode::Details;
                self.zoom_cross_gate = Some(Instant::now());
                log::debug!("zoom ladder: Grid (extra small) -> Details");
            }
            return;
        }
        let next = (self.explorer_zoom as i32 + step).clamp(0, 4) as u8;
        if next != self.explorer_zoom {
            log::debug!("explorer zoom level: {} -> {}", self.explorer_zoom, next);
            self.explorer_zoom = next;
        }
    }

    /// True when a Grid<->Details crossing may fire: no crossing yet, or the
    /// previous crossing is older than the wiggle debounce window.
    fn zoom_cross_open(&self) -> bool {
        self.zoom_cross_gate
            .is_none_or(|t| Instant::now().duration_since(t) >= Self::ZOOM_CROSS_DEBOUNCE)
    }

    /// Toggle Dual-Pane Split View
    pub fn toggle_split_view(&mut self) {
        self.split_view_enabled = !self.split_view_enabled;
        if self.split_view_enabled && self.secondary_entries.is_empty() {
            if self.secondary_dir == self.current_dir {
                self.secondary_dir = crate::core::platform_paths::downloads_dir();
            }
            self.secondary_entries = list_directory_sync(&self.secondary_dir);
        }
    }

    /// Toggle Right-Side Preview Inspector Pane
    pub fn toggle_inspector_pane(&mut self) {
        self.show_inspector_pane = !self.show_inspector_pane;
    }

    /// Navigate to a new directory in the secondary split pane
    pub fn secondary_navigate_to(&mut self, dir: PathBuf) {
        if !dir.exists() {
            return;
        }
        if self.secondary_history.get(self.secondary_history_idx) != Some(&dir) {
            self.secondary_history
                .truncate(self.secondary_history_idx + 1);
            self.secondary_history.push(dir.clone());
            self.secondary_history_idx = self.secondary_history.len() - 1;
        }
        self.secondary_dir = dir.clone();
        self.secondary_entries = list_directory_sync(&dir);
        self.secondary_selected_paths.clear();
        self.secondary_focused_row = 0;
    }

    /// Child-folder popup listing, loaded once per path and cached
    /// (F-2 fix): the popup closure re-runs every frame while open,
    /// so a synchronous re-list each frame would freeze the UI on
    /// slow directories.
    fn popup_child_dirs(&mut self, path: &Path) -> Vec<FileItem> {
        if !self.breadcrumb_popup_cache.contains_key(path) {
            let dirs: Vec<FileItem> = list_directory_sync(path)
                .into_iter()
                .filter(|e| e.is_dir)
                .collect();
            self.breadcrumb_popup_cache.insert(path.to_path_buf(), dirs);
        }
        self.breadcrumb_popup_cache
            .get(path)
            .cloned()
            .unwrap_or_default()
    }

    /// Navigate to a new directory asynchronously with zero UI hitching
    pub fn navigate_to(&mut self, dir: PathBuf) {
        if !dir.exists() {
            return;
        }

        // Popup listings are cached once per path (F-2 fix); every
        // navigation and refresh invalidates them so the child-folder
        // dropdown never shows stale contents after a fs operation.
        self.breadcrumb_popup_cache.clear();

        // Push to navigation history
        if self.history.get(self.history_idx) != Some(&dir) {
            self.history.truncate(self.history_idx + 1);
            self.history.push(dir.clone());
            self.history_idx = self.history.len() - 1;
        }

        // Miller lineage (Finder column-view semantics): refreshes
        // and fs-op completions keep the opened chain and stack;
        // entering an opened stack folder shifts it into the chain
        // and keeps deeper columns; unrelated navigation closes the
        // stack.
        let old_dir = std::mem::replace(&mut self.current_dir, dir.clone());
        let lineage = lineage_policy(&dir, &old_dir, &self.column_stack);
        match lineage {
            MillerLineage::KeepStack => {
                // Same directory: preserve the Miller chain, the
                // stack, any inspector preview, and in-progress
                // rename or path-bar editing.
            }
            MillerLineage::ShiftTo(k) => {
                self.column_stack.drain(0..=k);
                // Finder: entering a folder shows no file preview;
                // a directory placeholder keeps the auto-preview
                // guard dormant without opening the inspector.
                self.column_preview_item = Some(FileItem::from_path(dir.clone()));
                self.needs_miller_scroll_align = true;
            }
            MillerLineage::ClearStack => {
                self.column_stack.clear();
                self.column_preview_item = None;
                self.needs_miller_scroll_align = true;
                if dir.parent() == Some(old_dir.as_path()) {
                    // Pure up-navigation: Finder highlights the
                    // folder just left and shows no phantom
                    // first-file preview. The directory placeholder
                    // preview keeps the auto-preview guard dormant.
                    // B-2 exception: up-navigation into the Home
                    // dashboard must not inherit a selection; the
                    // dashboard is not a listing of the parent and
                    // always starts clean.
                    self.selected_paths.clear();
                    if crate::core::platform_paths::home_dir_opt().as_ref() != Some(&dir) {
                        self.selected_paths.insert(old_dir.clone());
                    }
                    self.column_preview_item = Some(FileItem::from_path(old_dir));
                }
            }
        }
        if lineage != MillerLineage::KeepStack {
            self.path_edit_buffer = dir.to_string_lossy().to_string();
            self.is_editing_path = false;
            self.renaming_path = None;
            self.focused_row = 0;
            // Finder parity: reveal the current segment in the
            // breadcrumb path bar after a real directory change.
            self.needs_breadcrumb_scroll_align = true;
        }

        let home_opt = crate::core::platform_paths::home_dir_opt();
        self.is_home_view = Some(&dir) == home_opt.as_ref();

        // Update active tab metadata
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.path = dir.clone();
            tab.is_home = self.is_home_view;
            tab.title = if self.is_home_view {
                "Home".to_string()
            } else {
                folder_title(&dir)
            };
            tab.icon = if self.is_home_view { "H" } else { "D" };
        }

        self.dir_watcher.watch(&dir);

        let new_gen = self.scan_gen.fetch_add(1, Ordering::SeqCst) + 1;
        spawn_async_dir_scan(
            dir,
            new_gen,
            Arc::clone(&self.scan_gen),
            self.scan_tx.clone(),
        );
    }

    /// Fetches directory entries for Miller columns, using the fast in-memory cache
    /// to avoid freezing the UI with synchronous disk IO on every folder click.
    pub fn get_column_entries(&mut self, path: &Path) -> Vec<FileItem> {
        if let Some(entries) = self.column_cache.get(path) {
            entries.clone()
        } else {
            let entries = list_directory_sync(path);
            self.column_cache
                .insert(path.to_path_buf(), entries.clone());
            entries
        }
    }

    pub fn add_tab(&mut self, path: PathBuf) {
        let home_opt = crate::core::platform_paths::home_dir_opt();
        let is_home = Some(&path) == home_opt.as_ref();
        let title = if is_home {
            "Home".to_string()
        } else {
            folder_title(&path)
        };
        let icon = if is_home { "H" } else { "D" };
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
        let home_opt = crate::core::platform_paths::home_dir_opt();
        Some(&self.current_dir) == home_opt.as_ref()
    }

    pub fn go_back(&mut self) {
        if self.can_go_back() {
            self.history_idx -= 1;
            let dir = self.history[self.history_idx].clone();
            // Route through navigate_to so the Miller column stack, inspector
            // preview, Home flag, tab metadata, and dir watcher reset exactly
            // like a normal navigation. navigate_to skips its history push
            // when the target already sits at history_idx, so back/forward
            // positions are preserved.
            self.navigate_to(dir);
        }
    }

    pub fn go_forward(&mut self) {
        if self.can_go_forward() {
            self.history_idx += 1;
            let dir = self.history[self.history_idx].clone();
            self.navigate_to(dir);
        }
    }

    pub fn select_all(&mut self) {
        let paths: Vec<PathBuf> = self
            .filtered_and_sorted_entries()
            .iter()
            .map(|e| e.path.clone())
            .collect();
        self.selected_paths.clear();
        for p in paths {
            self.selected_paths.insert(p);
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected_paths.clear();
        self.selection_anchor = None;
    }

    pub fn selected_paths(&self) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = self.selected_paths.iter().cloned().collect();
        paths.sort();
        paths
    }

    pub fn open_batch_rename(&mut self) {
        let targets = self.selected_paths();
        if !targets.is_empty() {
            self.batch_rename.target_paths = targets;
            self.batch_rename.is_open = true;
        }
    }

    pub fn refresh(&mut self) {
        self.navigate_to(self.current_dir.clone());
    }

    pub fn navigate_to_home(&mut self) {
        let home_opt = crate::core::platform_paths::home_dir_opt();
        let target = home_opt.unwrap_or_else(|| PathBuf::from("."));
        self.navigate_to(target);
        self.is_home_view = true;
    }

    /// Process incoming background directory scanning batches and filesystem watcher events
    pub fn poll_background_scans(&mut self) {
        if self.dir_watcher.try_recv() {
            let dir = self.current_dir.clone();
            let new_gen = self.scan_gen.fetch_add(1, Ordering::SeqCst) + 1;
            spawn_async_dir_scan(
                dir,
                new_gen,
                Arc::clone(&self.scan_gen),
                self.scan_tx.clone(),
            );
        }
        if self.split_view_enabled && self.secondary_dir_watcher.try_recv() {
            self.secondary_entries = list_directory_sync(&self.secondary_dir);
        }

        while let Ok(batch) = self.scan_rx.try_recv() {
            if batch.gen == self.scan_gen.load(Ordering::Relaxed) {
                self.entries = batch.entries;
                // A selection made in the previous folder must not survive
                // navigation: toolbar actions and the Delete key act on the
                // selection, so a stale one hit files the user can't see.
                // (Columns view keeps cross-column selection by design.)
                if self.view_mode != ViewMode::Columns {
                    // Keep only paths that are still listed (also drops a
                    // just-renamed or deleted item's stale entry).
                    let listed: std::collections::HashSet<&Path> =
                        self.entries.iter().map(|e| e.path.as_path()).collect();
                    self.selected_paths.retain(|p| listed.contains(p.as_path()));
                }
            }
        }

        while let Ok(res) = self.fs_worker_rx.try_recv() {
            if !res.errors.is_empty() {
                log::warn!(
                    "FS operation had {} error(s): {:?}",
                    res.errors.len(),
                    res.errors
                );
                self.show_status_note(Err(format!(
                    "Warning: {} error(s) during {}",
                    res.errors.len(),
                    res.op_name
                )));
            } else if res.count > 0 {
                log::info!(
                    "FS operation completed: {} {} item(s)",
                    res.op_name,
                    res.count
                );
                self.show_status_note(Ok(format!("{} {} item(s)", res.op_name, res.count)));
            }
            let dir = self.current_dir.clone();
            self.navigate_to(dir);
        }

        // -- Folder-size worker: drain results, throttle requests, flush --
        while let Ok(res) = self.dir_size_rx.try_recv() {
            self.dir_size_queued.remove(&res.path);
            self.dir_sizes.insert(res.path, (res.mtime, res.total));
            self.dir_size_cache_dirty += 1;
        }

        let now = Instant::now();
        let request_due = match self.dir_size_last_request {
            Some(t) => now.duration_since(t) >= std::time::Duration::from_millis(200),
            None => true,
        };
        if request_due {
            self.dir_size_last_request = Some(now);
            let mut candidates: Vec<(PathBuf, Option<std::time::SystemTime>)> = Vec::new();
            let mut primary_selected: Vec<&FileItem> = self
                .entries
                .iter()
                .filter(|e| e.is_dir && self.selected_paths.contains(&e.path))
                .collect();
            primary_selected.sort_by(|a, b| a.path.cmp(&b.path));
            candidates.extend(
                primary_selected
                    .into_iter()
                    .map(|e| (e.path.clone(), e.modified)),
            );
            for e in self.filtered_and_sorted_entries() {
                if e.is_dir {
                    candidates.push((e.path.clone(), e.modified));
                }
            }
            if self.split_view_enabled {
                let mut secondary_selected: Vec<&FileItem> = self
                    .secondary_entries
                    .iter()
                    .filter(|e| e.is_dir && self.secondary_selected_paths.contains(&e.path))
                    .collect();
                secondary_selected.sort_by(|a, b| a.path.cmp(&b.path));
                candidates.extend(
                    secondary_selected
                        .into_iter()
                        .map(|e| (e.path.clone(), e.modified)),
                );
                for e in &self.secondary_entries {
                    if e.is_dir {
                        candidates.push((e.path.clone(), e.modified));
                    }
                }
            }

            let mut sent = 0usize;
            for (path, mtime) in candidates {
                if sent >= 64 {
                    break;
                }
                if self.dir_size_queued.contains(&path) {
                    continue;
                }
                if let Some((stored, _)) = self.dir_sizes.get(&path) {
                    if *stored == mtime {
                        continue;
                    }
                }
                self.dir_size_queued.insert(path.clone());
                let _ = self.dir_size_job_tx.send(path);
                sent += 1;
            }
        }

        // Write-behind flush: after 100 new entries or 30 seconds.
        let flush_due = match self.dir_size_last_flush {
            Some(t) => now.duration_since(t) >= std::time::Duration::from_secs(30),
            None => true,
        };
        if self.dir_size_cache_dirty > 0 && (self.dir_size_cache_dirty >= 100 || flush_due) {
            if crate::core::fs_utils::save_dir_size_cache_to(
                &crate::core::fs_utils::dir_size_cache_path(),
                &self.dir_sizes,
            )
            .is_ok()
            {
                self.dir_size_cache_dirty = 0;
            }
            self.dir_size_last_flush = Some(now);
        }
    }

    /// Filter and sort entries according to active filter, search query, and sort column
    pub fn filtered_and_sorted_entries(&self) -> Vec<&FileItem> {
        let q = self.search_query.trim();
        let is_searching = !q.is_empty();
        let q_lower = q.to_lowercase();
        let active_tag = self.active_tag.as_deref();

        let mut scored_list: Vec<(&FileItem, i64)> = self
            .entries
            .iter()
            .filter(|e| !crate::core::is_protected_os_file(&e.path))
            .filter(|e| self.show_hidden_files || !e.is_hidden)
            .filter(|e| self.category_filter.matches(e))
            .filter(|e| {
                self.column_filters
                    .iter()
                    .all(|(id, value)| match DetailColumn::from_id(*id) {
                        Some(col) => matches_column_filter(e, col, value),
                        None => true,
                    })
            })
            .filter(|e| match active_tag {
                // Tag smart-list: keep only items carrying the active tag.
                Some(tag) => crate::core::tags::read_tags_cache(&e.path)
                    .iter()
                    .any(|t| t == tag),
                None => true,
            })
            .filter_map(|e| {
                if is_searching {
                    // Substring filter (Finder): a loose subsequence match
                    // made "dl" list nearly every file in the folder.
                    if !e.name.to_lowercase().contains(&q_lower) {
                        return None;
                    }
                    let score = fuzzy_match(q, &e.name).unwrap_or(0);
                    Some((e, score))
                } else {
                    Some((e, 0))
                }
            })
            .collect();

        scored_list.sort_by(|(a, score_a), (b, score_b)| {
            // Search results keep the chosen sort (Finder): ordering by
            // fuzzy score made a Name-sorted list look shuffled.
            if a.is_dir != b.is_dir {
                return b.is_dir.cmp(&a.is_dir);
            }
            let ordering = match self.sort_column {
                SortColumn::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortColumn::Size => a.size_bytes.cmp(&b.size_bytes),
                SortColumn::Extension => a.extension.cmp(&b.extension),
                SortColumn::Date => cmp_opt_last(&a.modified, &b.modified),
                SortColumn::DateCreated => cmp_opt_last(&a.created, &b.created),
                SortColumn::Permissions => a.permissions_mode.cmp(&b.permissions_mode),
                SortColumn::Hidden => a.is_hidden.cmp(&b.is_hidden),
                SortColumn::Symlink => a.is_symlink.cmp(&b.is_symlink),
                SortColumn::SymlinkTarget => cmp_opt_last(&a.symlink_target, &b.symlink_target),
                SortColumn::Path => a.path.cmp(&b.path),
                SortColumn::Kind => kind_key(a).cmp(kind_key(b)),
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

    /// Persists column visibility + widths to MossambiConfig via the
    /// load-modify-save pattern (filters stay session-scoped).
    pub fn save_column_layout(&mut self) {
        let mut cfg = crate::core::MossambiConfig::load();
        let (cols, widths) = self.column_layout.to_config();
        cfg.explorer_columns = cols;
        cfg.explorer_column_widths = widths;
        if let Err(e) = cfg.save() {
            eprintln!("mossambi: failed to save column layout: {e}");
        }
    }

    /// Returns the count of non-folder items matching a category
    pub fn count_category(&self, filter: CategoryFilter) -> usize {
        self.entries
            .iter()
            .filter(|e| !e.is_dir && filter.matches(e))
            .count()
    }

    /// Sets (or clears) the active tag smart-list filter.
    pub fn set_active_tag(&mut self, tag: Option<String>) {
        self.active_tag = tag;
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

        // Drive refresh hook: re-enumerate volumes every 5 s (cheap Win32
        // lookup) so free space, mounts, and unmounts stay current for the
        // status bar and the sidebar drive list.
        let refresh_due = match self.last_drives_refresh {
            Some(t) => t.elapsed() > std::time::Duration::from_secs(5),
            None => true,
        };
        if refresh_due {
            self.drives = get_system_drives();
            self.last_drives_refresh = Some(Instant::now());
        }

        // 1. External OS Drag & Drop Ingestion
        let (dropped_files, is_hovered) =
            ui.input(|i| (i.raw.dropped_files.clone(), !i.raw.hovered_files.is_empty()));

        if is_hovered {
            ui.add_space(4.0);
            // Window-edge inset: the explorer tab renders flush, so the
            // banner carries its own horizontal padding.
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                let drop_frame = egui::Frame::none()
                    .fill(Color32::from_rgba_unmultiplied(120, 190, 32, 50))
                    .stroke(Stroke::new(1.5_f32, palette.accent))
                    .rounding(Rounding::same(8.0))
                    .inner_margin(egui::Margin::symmetric(14.0, 7.0));
                drop_frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(
                                "Drop external files or folders here to stage, inspect, or transcode",
                            )
                            .strong()
                            .color(palette.accent),
                        );
                    });
                });
            });
            ui.add_space(4.0);
        }

        if !dropped_files.is_empty() {
            // Ingestion rule (Explorer parity): navigate into the first
            // directory of the drop and stage every non-directory file, so a
            // mixed multi-path drop no longer thrashes between folders.
            let mut navigated = false;
            for file in dropped_files {
                if let Some(path) = file.path {
                    if path.is_dir() {
                        if !navigated {
                            self.navigate_to(path);
                            navigated = true;
                        }
                    } else {
                        on_stage_file(path);
                    }
                }
            }
        }

        // 1b. OS drag-out hand-off: when an intra-app drag payload is active
        // and the pointer leaves the viewport with the button held, the drag
        // is leaving the app, so hand it to the native mechanism (Windows
        // OLE `DoDragDrop`; Linux typed file clipboard) and clear the egui
        // payload. Intra-app drops are unaffected: as long as the pointer
        // stays inside, `dnd_release_payload` on the folder rows keeps its
        // Explorer-parity semantics.
        if egui::DragAndDrop::has_payload_of_type::<FileDragPayload>(ui.ctx()) {
            let screen_rect = ui.ctx().screen_rect();
            let (any_down, pos_outside, decided) = ui.input(|i| {
                let p_opt = i.pointer.latest_pos();
                let outside = match p_opt {
                    Some(p) => !screen_rect.contains(p),
                    None => false,
                };
                (
                    i.pointer.any_down(),
                    outside,
                    i.pointer.is_decidedly_dragging(),
                )
            });
            if !any_down {
                self.drag_out_in_flight = false;
            }
            if any_down && decided && !self.drag_out_in_flight && pos_outside {
                self.drag_out_in_flight = true;
                if let Some(payload) = egui::DragAndDrop::take_payload::<FileDragPayload>(ui.ctx())
                {
                    log::info!(
                        "Drag-out hand-off: {} path(s) leaving the window",
                        payload.paths.len()
                    );
                    let note = crate::core::begin_file_drag_out(&payload.paths)
                        .map_err(|e| format!("Drag-out failed: {e}"));
                    self.show_status_note(note);
                    egui::DragAndDrop::clear_payload(ui.ctx());
                }
            }
        }

        // Transient status note (drag hand-off feedback), auto-faded.
        let note_alive = self
            .status_note
            .as_ref()
            .map(|(_, at)| at.elapsed() < std::time::Duration::from_secs(4))
            .unwrap_or(false);
        if note_alive {
            if let Some((note, _)) = &self.status_note {
                ui.label(egui::RichText::new(note).size(16.0).color(palette.accent));
            }
        } else {
            self.status_note = None;
        }

        // 2. Global Explorer Hotkey Interceptions with Vim & Emacs Navigation Engine
        let is_search_focused =
            ui.memory(|m| m.has_focus(egui::Id::new("mossambi_global_search_input")));
        // Any focused text field (search, path bar, rename, dialogs) owns
        // the keyboard: single-letter and editing shortcuts must not fire.
        let text_field_focused = ui.ctx().wants_keyboard_input();
        let is_typing = self.is_editing_path
            || self.renaming_path.is_some()
            || self.show_new_folder_dialog
            || self.properties_item.is_some()
            || is_search_focused
            || text_field_focused;
        let finder_keybindings = crate::core::MossambiConfig::load().keybinding_profile == 0;
        // Focus changes must happen outside `ui.input`: calling
        // `ui.memory_mut` while the input lock is held deadlocks egui.
        let mut surrender_search_focus = false;

        let nav_entries: Vec<FileItem> = self
            .filtered_and_sorted_entries()
            .into_iter()
            .cloned()
            .collect();
        let total_nav_items = nav_entries.len();

        // B-1 gate: while the command palette modal is open the
        // palette owns the keyboard. egui raw input is app-wide, so
        // every handler below would also fire for keys meant for
        // the palette (Enter, vim letters, arrows, Space, Delete).
        // Skip the whole block; the view resumes raw-key handling
        // on the frame after the palette closes.
        if !self.palette_open {
            ui.input(|i| {
                if i.key_pressed(egui::Key::Escape) {
                    if !self.search_query.is_empty() || is_search_focused {
                        self.search_query.clear();
                        surrender_search_focus = true;
                    }
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
                    // Vim `j` / Down: Cursor Down
                    if total_nav_items > 0
                        && ((i.key_pressed(egui::Key::J)
                            && !i.modifiers.command
                            && !i.modifiers.ctrl
                            && !i.modifiers.alt)
                            || (i.key_pressed(egui::Key::ArrowDown)
                                && !i.modifiers.ctrl
                                && !i.modifiers.command))
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
                        || ((i.modifiers.ctrl || i.modifiers.command)
                            && i.key_pressed(egui::Key::B))
                        || i.key_pressed(egui::Key::Backspace)
                    {
                        if let Some(parent) = self.current_dir.parent() {
                            let p = parent.to_path_buf();
                            if !p.as_os_str().is_empty() {
                                self.navigate_to(p);
                            }
                        }
                    }

                    // Keybinding profile (Preferences): Finder = Enter renames,
                    // Ctrl+Down / Ctrl+O opens; Windows Explorer = Enter opens.
                    // (Ctrl+F no longer opens the item: it is Find.)
                    let finder_keys = finder_keybindings;
                    let enter = i.key_pressed(egui::Key::Enter) && !i.modifiers.alt;
                    if finder_keys && enter {
                        if let Some(item) = nav_entries.get(self.focused_row) {
                            if let Some(name) = item.path.file_name() {
                                self.renaming_path =
                                    Some((item.path.clone(), name.to_string_lossy().to_string()));
                            }
                        }
                    }
                    if (i.key_pressed(egui::Key::L)
                        && !i.modifiers.command
                        && !i.modifiers.ctrl
                        && !i.modifiers.alt)
                        || (!finder_keys && enter)
                        || ((i.modifiers.ctrl || i.modifiers.command)
                            && (i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::O)))
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
                    if i.key_pressed(egui::Key::Space) && !i.modifiers.ctrl && !i.modifiers.command
                    {
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
                                if now.duration_since(prev) < std::time::Duration::from_millis(400)
                                {
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

                    // Vim `/` / Neovim / Emacs `Ctrl+S` / `Ctrl+F`: Focus Search Bar
                    if (i.key_pressed(egui::Key::Slash)
                        && !i.modifiers.ctrl
                        && !i.modifiers.command
                        && !i.modifiers.alt)
                        || ((i.modifiers.ctrl || i.modifiers.command)
                            && (i.key_pressed(egui::Key::F) || i.key_pressed(egui::Key::S)))
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
                            self.clipboard_paths =
                                self.selected_paths.iter().cloned().collect::<Vec<_>>();
                        }
                        self.clipboard_is_cut = false;
                    }

                    // Vim `p` / Emacs `Ctrl+Y`: Put / Paste
                    if (i.key_pressed(egui::Key::P)
                        && !i.modifiers.ctrl
                        && !i.modifiers.command
                        && !i.modifiers.alt)
                        || ((i.modifiers.ctrl || i.modifiers.command)
                            && i.key_pressed(egui::Key::Y))
                    {
                        self.paste_from_clipboard();
                    }

                    // Bare `d` no longer deletes: a single stray letter must never
                    // move files to the trash. Delete / Shift+Delete do that.

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
                    || (i.modifiers.command
                        && i.modifiers.shift
                        && i.key_pressed(egui::Key::Period))
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

                // F3: Toggle Dual-Pane Split View
                if i.key_pressed(egui::Key::F3) {
                    self.toggle_split_view();
                }

                // Tab: Switch active pane in Split View (when not typing)
                if !is_typing && self.split_view_enabled && i.key_pressed(egui::Key::Tab) {
                    self.active_pane = 1 - self.active_pane;
                }

                // Standard Explorer Hotkeys
                if i.key_pressed(egui::Key::F5) {
                    if self.split_view_enabled {
                        let (src_paths, dest_dir) = if self.active_pane == 0 {
                            (
                                self.selected_paths.iter().cloned().collect::<Vec<_>>(),
                                self.secondary_dir.clone(),
                            )
                        } else {
                            (
                                self.secondary_selected_paths
                                    .iter()
                                    .cloned()
                                    .collect::<Vec<_>>(),
                                self.current_dir.clone(),
                            )
                        };
                        for src in &src_paths {
                            if let Some(fname) = src.file_name() {
                                let dest = dest_dir.join(fname);
                                let unique = resolve_unique_paste_path(&dest);
                                log_result("Copy", src, copy_entry_recursive(src, &unique));
                            }
                        }
                        self.navigate_to(self.current_dir.clone());
                        self.secondary_navigate_to(self.secondary_dir.clone());
                    } else {
                        self.navigate_to(self.current_dir.clone());
                    }
                }

                // F6: Move cross-pane in Split View
                if self.split_view_enabled && i.key_pressed(egui::Key::F6) {
                    let (src_paths, dest_dir) = if self.active_pane == 0 {
                        (
                            self.selected_paths.iter().cloned().collect::<Vec<_>>(),
                            self.secondary_dir.clone(),
                        )
                    } else {
                        (
                            self.secondary_selected_paths
                                .iter()
                                .cloned()
                                .collect::<Vec<_>>(),
                            self.current_dir.clone(),
                        )
                    };
                    for src in &src_paths {
                        if let Some(fname) = src.file_name() {
                            let dest = dest_dir.join(fname);
                            log_result("Move", src, move_entry(src, &dest));
                        }
                    }
                    self.navigate_to(self.current_dir.clone());
                    self.secondary_navigate_to(self.secondary_dir.clone());
                }

                // Ctrl+Shift+P / Cmd+Shift+P: Toggle Inspector Preview Pane
                if (i.modifiers.ctrl || i.modifiers.command)
                    && i.modifiers.shift
                    && i.key_pressed(egui::Key::P)
                {
                    self.toggle_inspector_pane();
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
                // Ctrl+N / Cmd+N (without Shift): Open New Concurrent Explorer Window
                if (i.modifiers.ctrl || i.modifiers.command)
                    && !i.modifiers.shift
                    && !i.modifiers.alt
                    && i.key_pressed(egui::Key::N)
                {
                    spawn_new_explorer_window(Some(&self.current_dir));
                }
                // Ctrl+T / Cmd+T and Ctrl+W / Cmd+W are handled once globally in
                // StudioView::render (where they also switch to the explorer tab);
                // duplicating them here fired each keystroke's action twice.
                // Ctrl+Tab / Ctrl+Shift+Tab: Cycle Next/Previous Tab
                if (i.modifiers.ctrl || i.modifiers.command)
                    && i.key_pressed(egui::Key::Tab)
                    && !self.tabs.is_empty()
                {
                    if i.modifiers.shift {
                        if self.active_tab_index == 0 {
                            self.active_tab_index = self.tabs.len().saturating_sub(1);
                        } else {
                            self.active_tab_index = self.active_tab_index.saturating_sub(1);
                        }
                    } else {
                        self.active_tab_index = (self.active_tab_index + 1) % self.tabs.len();
                    }
                    if let Some(tab) = self.tabs.get(self.active_tab_index) {
                        let target = tab.path.clone();
                        self.navigate_to(target);
                    }
                }
                if (i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(egui::Key::L) {
                    self.path_edit_buffer = self.current_dir.to_string_lossy().to_string();
                    self.is_editing_path = true;
                }
                // Ctrl+Z / Cmd+Z: reverse the last journaled action.
                //
                // The journal records rename/move/copy/create/delete/tag, so this
                // single binding covers every reversible mutation the explorer
                // performs. Ctrl+Shift+Z redoes is intentionally NOT offered: the
                // journal appends an Undo marker instead of replaying, so the
                // stack always walks backwards.
                // Not while typing: Ctrl+Z in a text field is text undo.
                if !is_typing
                    && (i.modifiers.ctrl || i.modifiers.command)
                    && i.key_pressed(egui::Key::Z)
                {
                    let note = crate::core::action_journal::undo_last();
                    self.show_status_note(note);
                    // The undo may have changed the current directory (a moved or
                    // restored folder), so rescan whatever we are looking at.
                    self.navigate_to(self.current_dir.clone());
                }
                // Context-tagged keystroke logging. The journal keeps these
                // separate from the file actions; they record what the user was
                // doing, so a future history panel can explain an action.
                {
                    let context = if self.renaming_path.is_some() {
                        "rename-edit"
                    } else if self.is_editing_path {
                        "path-edit"
                    } else if is_search_focused {
                        "text-input"
                    } else {
                        "explorer-nav"
                    };
                    let modifiers = modifier_string(i.modifiers);
                    for ev in &i.events {
                        if let egui::Event::Key {
                            key,
                            pressed: true,
                            repeat: false,
                            ..
                        } = ev
                        {
                            crate::core::action_journal::record_keystroke(
                                context,
                                &format!("{key:?}"),
                                &modifiers,
                            );
                        }
                    }
                }
                if !is_typing
                    && (i.modifiers.ctrl || i.modifiers.command)
                    && i.key_pressed(egui::Key::C)
                {
                    if self.selected_paths.is_empty() {
                        if let Some(item) = nav_entries.get(self.focused_row) {
                            self.clipboard_paths = vec![item.path.clone()];
                        }
                    } else {
                        self.clipboard_paths =
                            self.selected_paths.iter().cloned().collect::<Vec<_>>();
                    }
                    self.clipboard_is_cut = false;
                }
                if !is_typing
                    && (i.modifiers.ctrl || i.modifiers.command)
                    && i.key_pressed(egui::Key::X)
                {
                    if self.selected_paths.is_empty() {
                        if let Some(item) = nav_entries.get(self.focused_row) {
                            self.clipboard_paths = vec![item.path.clone()];
                        }
                    } else {
                        self.clipboard_paths =
                            self.selected_paths.iter().cloned().collect::<Vec<_>>();
                    }
                    self.clipboard_is_cut = true;
                }
                if !is_typing
                    && (i.modifiers.ctrl || i.modifiers.command)
                    && i.key_pressed(egui::Key::V)
                {
                    self.paste_from_clipboard();
                }
                if !is_typing && i.key_pressed(egui::Key::Delete) {
                    let paths_to_delete: Vec<PathBuf> = if self.selected_paths.is_empty() {
                        nav_entries
                            .get(self.focused_row)
                            .map(|item| vec![item.path.clone()])
                            .unwrap_or_default()
                    } else {
                        self.selected_paths.iter().cloned().collect()
                    };

                    let paths_to_delete: Vec<PathBuf> = paths_to_delete
                        .into_iter()
                        .filter(|p| !crate::core::is_protected_os_file(p))
                        .collect();

                    if !paths_to_delete.is_empty() {
                        if i.modifiers.shift {
                            self.permanent_delete_prompt = Some(paths_to_delete);
                        } else {
                            let r = crate::core::action_journal::stage_delete(&paths_to_delete);
                            log_result("Delete (Recycle Bin)", &paths_to_delete[0], r);
                            self.selected_paths.clear();
                            self.navigate_to(self.current_dir.clone());
                        }
                    }
                }
            });
        }
        if surrender_search_focus {
            ui.memory_mut(|m| m.surrender_focus(egui::Id::new("mossambi_global_search_input")));
        }

        let modal_bg = if palette.is_dark {
            Color32::from_rgb(30, 32, 36)
        } else {
            Color32::from_rgb(250, 250, 252)
        };

        // 3. New Folder Modal Dialog (Apple Sheet aesthetic)
        if self.show_new_folder_dialog {
            let mut close_dialog = false;
            let mut create_folder = false;

            egui::Window::new("Create New Folder Dialog")
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .frame(
                    egui::Frame::none()
                        .fill(modal_bg)
                        .stroke(Stroke::new(1.0_f32, palette.border))
                        .rounding(Rounding::same(12.0))
                        .inner_margin(egui::Margin::same(18.0)),
                )
                .show(ui.ctx(), |ui| {
                    ui.label(
                        egui::RichText::new("New Folder")
                            .size(15.0)
                            .strong()
                            .color(palette.text_primary),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Enter folder name:")
                            .size(12.5)
                            .color(palette.text_secondary),
                    );
                    ui.add_space(6.0);
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.new_folder_buffer)
                            .desired_width(260.0),
                    );
                    resp.request_focus();
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        create_folder = true;
                    }

                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let create_btn = ui.add(
                                egui::Button::new(
                                    egui::RichText::new("Create").strong().color(Color32::WHITE),
                                )
                                .fill(palette.accent)
                                .rounding(Rounding::same(6.0))
                                .min_size(Vec2::new(72.0, 26.0)),
                            );
                            if create_btn.clicked() {
                                create_folder = true;
                            }
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Cancel").color(palette.text_primary),
                                    )
                                    .fill(palette.card_bg)
                                    .stroke(Stroke::new(1.0_f32, palette.border))
                                    .rounding(Rounding::same(6.0))
                                    .min_size(Vec2::new(72.0, 26.0)),
                                )
                                .clicked()
                            {
                                close_dialog = true;
                            }
                        });
                    });
                });

            if create_folder {
                let clean_name = self.new_folder_buffer.trim();
                if !clean_name.is_empty() {
                    let target_dir = self.current_dir.join(clean_name);
                    let _ = std::fs::create_dir_all(&target_dir);
                    crate::core::action_journal::record_create_dir(target_dir.clone());
                    self.navigate_to(self.current_dir.clone());
                }
                self.show_new_folder_dialog = false;
                self.new_folder_buffer = "New Folder".to_string();
            } else if close_dialog {
                self.show_new_folder_dialog = false;
            }
        }

        // 3b. Shift+Delete Permanent Deletion Confirmation Modal
        if let Some(ref targets) = self.permanent_delete_prompt.clone() {
            let mut close_dialog = false;
            let mut do_permanent_delete = false;

            egui::Window::new("Permanent Deletion Confirmation")
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .frame(
                    egui::Frame::none()
                        .fill(modal_bg)
                        .stroke(Stroke::new(1.0_f32, palette.border))
                        .rounding(Rounding::same(12.0))
                        .inner_margin(egui::Margin::same(18.0)),
                )
                .show(ui.ctx(), |ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "Permanently delete {} item(s)?",
                            targets.len()
                        ))
                        .size(15.0)
                        .strong()
                        .color(palette.text_primary),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(
                            "This action cannot be undone. Files will bypass the Recycle Bin.",
                        )
                        .size(12.5)
                        .color(Color32::from_rgb(255, 90, 90)),
                    );
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Delete Permanently")
                                            .color(Color32::WHITE)
                                            .strong(),
                                    )
                                    .fill(Color32::from_rgb(220, 50, 50))
                                    .rounding(Rounding::same(6.0))
                                    .min_size(Vec2::new(130.0, 26.0)),
                                )
                                .clicked()
                            {
                                do_permanent_delete = true;
                                close_dialog = true;
                            }
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Cancel").color(palette.text_primary),
                                    )
                                    .fill(palette.card_bg)
                                    .stroke(Stroke::new(1.0_f32, palette.border))
                                    .rounding(Rounding::same(6.0))
                                    .min_size(Vec2::new(72.0, 26.0)),
                                )
                                .clicked()
                            {
                                close_dialog = true;
                            }
                        });
                    });
                });

            if do_permanent_delete {
                for path in targets {
                    log_result("Permanent Delete", path, remove_entry_recursive(path));
                }
                self.selected_paths.clear();
                self.navigate_to(self.current_dir.clone());
            }
            if close_dialog {
                self.permanent_delete_prompt = None;
            }
        }

        // 4. Rename Item Modal Dialog (Apple Sheet style - neutral, zero lime tint!)
        if let Some((old_path, mut new_name)) = self.renaming_path.clone() {
            let mut close_dialog = false;
            let mut do_rename = false;
            // One-shot flag: pre-select the name stem on the first frame only.
            let sel_init_key = egui::Id::new(("mossambi_rename_sel_init", &old_path));
            let old_display = old_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();

            egui::Window::new("Rename Item Dialog")
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .frame(
                    egui::Frame::none()
                        .fill(modal_bg)
                        .stroke(Stroke::new(1.0_f32, palette.border))
                        .rounding(Rounding::same(12.0))
                        .inner_margin(egui::Margin::same(18.0)),
                )
                .show(ui.ctx(), |ui| {
                    ui.label(
                        egui::RichText::new("Rename Item")
                            .size(15.0)
                            .strong()
                            .color(palette.text_primary),
                    );
                    ui.add_space(4.0);
                    // Name the target: the list selection can change while
                    // the sheet is open, so say exactly what gets renamed.
                    ui.label(
                        egui::RichText::new(format!("New name for \"{old_display}\":"))
                            .size(12.5)
                            .color(palette.text_secondary),
                    );
                    ui.add_space(6.0);
                    let out = egui::TextEdit::singleline(&mut new_name)
                        .desired_width(280.0)
                        .show(ui);
                    let resp = out.response;
                    if !ui.data(|d| d.get_temp::<bool>(sel_init_key).unwrap_or(false)) {
                        // Finder/Explorer: select the name without its
                        // extension so typing replaces it (typing used to
                        // append: "alpha.txtrenamed").
                        let stem_chars = rename_stem_chars(&new_name, old_path.is_dir());
                        let mut state = out.state;
                        state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::two(
                                egui::text::CCursor::new(0),
                                egui::text::CCursor::new(stem_chars),
                            )));
                        state.store(ui.ctx(), resp.id);
                        ui.data_mut(|d| d.insert_temp(sel_init_key, true));
                    }
                    resp.request_focus();
                    // Enter commits. (The old check needed lost_focus, which
                    // never happened because focus is re-requested every
                    // frame, so Enter did nothing.)
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        do_rename = true;
                    }

                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let rename_btn = ui.add(
                                egui::Button::new(
                                    egui::RichText::new("Rename").strong().color(Color32::WHITE),
                                )
                                .fill(palette.accent)
                                .rounding(Rounding::same(6.0))
                                .min_size(Vec2::new(72.0, 26.0)),
                            );
                            if rename_btn.clicked() {
                                do_rename = true;
                            }
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Cancel").color(palette.text_primary),
                                    )
                                    .fill(palette.card_bg)
                                    .stroke(Stroke::new(1.0_f32, palette.border))
                                    .rounding(Rounding::same(6.0))
                                    .min_size(Vec2::new(72.0, 26.0)),
                                )
                                .clicked()
                            {
                                close_dialog = true;
                            }
                        });
                    });
                });

            if do_rename {
                let trimmed = new_name.trim();
                let mut keep_open = false;
                if !trimmed.is_empty() && trimmed != old_display {
                    if let Some(parent) = old_path.parent() {
                        let new_path = parent.join(trimmed);
                        // Never clobber: on Linux/macOS rename() silently
                        // replaces an existing file.
                        let case_only = trimmed.eq_ignore_ascii_case(&old_display);
                        let result = if new_path.exists() && !case_only {
                            Err(format!("\"{trimmed}\" already exists in this folder"))
                        } else {
                            std::fs::rename(&old_path, &new_path)
                                .map_err(|e| format!("Could not rename: {e}"))
                        };
                        match result {
                            Ok(()) => {
                                // Journal only renames that really happened.
                                crate::core::action_journal::record_rename(
                                    old_path.clone(),
                                    new_path.clone(),
                                );
                                self.navigate_to(self.current_dir.clone());
                            }
                            Err(msg) => {
                                self.show_status_note(Err(msg));
                                keep_open = true;
                            }
                        }
                    }
                }
                if keep_open {
                    self.renaming_path = Some((old_path, new_name));
                } else {
                    self.renaming_path = None;
                    ui.ctx().data_mut(|d| d.remove::<bool>(sel_init_key));
                }
            } else if close_dialog {
                self.renaming_path = None;
                ui.ctx().data_mut(|d| d.remove::<bool>(sel_init_key));
            } else {
                self.renaming_path = Some((old_path, new_name));
            }
        }

        // 4b. Finder-Style Multi-Item Batch Renamer Modal Dialog
        if self.batch_rename.is_open {
            let mut close_dialog = false;
            let mut do_execute = false;
            let count = self.batch_rename.target_paths.len();

            egui::Window::new(format!("Batch Rename ({})", pluralize_items(count)))
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .frame(
                    egui::Frame::none()
                        .fill(modal_bg)
                        .stroke(Stroke::new(1.0_f32, palette.border))
                        .rounding(Rounding::same(12.0))
                        .inner_margin(egui::Margin::same(18.0)),
                )
                .show(ui.ctx(), |ui| {
                    ui.label(
                        egui::RichText::new(format!("Batch Rename ({})", pluralize_items(count)))
                            .size(15.0)
                            .strong()
                            .color(palette.text_primary),
                    );
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(self.batch_rename.mode == 0, "Replace Text")
                            .clicked()
                        {
                            self.batch_rename.mode = 0;
                        }
                        if ui
                            .selectable_label(self.batch_rename.mode == 1, "Add Prefix/Suffix")
                            .clicked()
                        {
                            self.batch_rename.mode = 1;
                        }
                        if ui
                            .selectable_label(self.batch_rename.mode == 2, "Format with Numbers")
                            .clicked()
                        {
                            self.batch_rename.mode = 2;
                        }
                    });

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);

                    if self.batch_rename.mode == 0 {
                        ui.horizontal(|ui| {
                            ui.label("Find:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.batch_rename.find_text)
                                    .desired_width(180.0),
                            );
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label("Replace with:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.batch_rename.replace_text)
                                    .desired_width(180.0),
                            );
                        });
                    } else if self.batch_rename.mode == 1 {
                        ui.horizontal(|ui| {
                            ui.label("Prefix:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.batch_rename.prefix_text)
                                    .desired_width(180.0),
                            );
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label("Suffix:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.batch_rename.suffix_text)
                                    .desired_width(180.0),
                            );
                        });
                    } else {
                        ui.horizontal(|ui| {
                            ui.label("Base Name:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.batch_rename.base_name)
                                    .desired_width(160.0),
                            );
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label("Start Index:");
                            ui.add(
                                egui::DragValue::new(&mut self.batch_rename.start_index)
                                    .range(1..=9999),
                            );
                        });
                    }

                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new("Preview:")
                            .strong()
                            .size(12.0)
                            .color(palette.accent),
                    );
                    let preview_count = count.min(3);
                    for i in 0..preview_count {
                        if let Some(path) = self.batch_rename.target_paths.get(i) {
                            let old_name = path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();
                            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                            let stem = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or(&old_name);

                            let new_name = match self.batch_rename.mode {
                                0 => {
                                    if !self.batch_rename.find_text.is_empty() {
                                        old_name.replace(
                                            &self.batch_rename.find_text,
                                            &self.batch_rename.replace_text,
                                        )
                                    } else {
                                        old_name.clone()
                                    }
                                }
                                1 => {
                                    if ext.is_empty() {
                                        format!(
                                            "{}{}{}",
                                            self.batch_rename.prefix_text,
                                            stem,
                                            self.batch_rename.suffix_text
                                        )
                                    } else {
                                        format!(
                                            "{}{}{}.{}",
                                            self.batch_rename.prefix_text,
                                            stem,
                                            self.batch_rename.suffix_text,
                                            ext
                                        )
                                    }
                                }
                                _ => {
                                    let num = self.batch_rename.start_index + i;
                                    if ext.is_empty() {
                                        format!("{}_{:03}", self.batch_rename.base_name, num)
                                    } else {
                                        format!(
                                            "{}_{:03}.{}",
                                            self.batch_rename.base_name, num, ext
                                        )
                                    }
                                }
                            };
                            ui.label(
                                egui::RichText::new(format!("{} -> {}", old_name, new_name))
                                    .size(11.0)
                                    .color(palette.text_secondary),
                            );
                        }
                    }

                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        let btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new(format!("Rename {} Files", count))
                                    .color(Color32::WHITE)
                                    .strong(),
                            )
                            .fill(palette.accent),
                        );
                        if btn.clicked() {
                            do_execute = true;
                            close_dialog = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close_dialog = true;
                        }
                    });
                });

            if do_execute {
                for (i, old_path) in self.batch_rename.target_paths.iter().enumerate() {
                    let old_name = old_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let ext = old_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    let stem = old_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or(&old_name);

                    let new_name = match self.batch_rename.mode {
                        0 => {
                            if !self.batch_rename.find_text.is_empty() {
                                old_name.replace(
                                    &self.batch_rename.find_text,
                                    &self.batch_rename.replace_text,
                                )
                            } else {
                                old_name.clone()
                            }
                        }
                        1 => {
                            if ext.is_empty() {
                                format!(
                                    "{}{}{}",
                                    self.batch_rename.prefix_text,
                                    stem,
                                    self.batch_rename.suffix_text
                                )
                            } else {
                                format!(
                                    "{}{}{}.{}",
                                    self.batch_rename.prefix_text,
                                    stem,
                                    self.batch_rename.suffix_text,
                                    ext
                                )
                            }
                        }
                        _ => {
                            let num = self.batch_rename.start_index + i;
                            if ext.is_empty() {
                                format!("{}_{:03}", self.batch_rename.base_name, num)
                            } else {
                                format!("{}_{:03}.{}", self.batch_rename.base_name, num, ext)
                            }
                        }
                    };

                    if new_name != old_name {
                        let new_path = old_path.with_file_name(&new_name);
                        crate::core::action_journal::record_rename(
                            old_path.clone(),
                            new_path.clone(),
                        );
                        let _ = std::fs::rename(old_path, &new_path);
                    }
                }
                self.refresh();
                self.batch_rename.is_open = false;
            }

            if close_dialog {
                self.batch_rename.is_open = false;
            }
        }

        // 5. Explorer Layout: Full Width macOS Finder Canvas
        ui.vertical(|ui| {
            // Top chrome padding: the app CentralPanel is flush for the
            // explorer tab, so the bars carry their own window inset.
            egui::Frame::none()
                .inner_margin(egui::Margin {
                    left: 10.0,
                    right: 10.0,
                    top: 6.0,
                    bottom: 0.0,
                })
                .show(ui, |ui| {
                    if self.show_internal_toolbar {
                        // Top Navigation & Breadcrumbs Bar (macOS Toolbar style)
                        self.render_breadcrumb_bar(ui, palette);
                        ui.add_space(6.0);
                        if !self.is_home_view {
                            // Category Filter Tabs Bar & View Mode Toggle (Segmented control)
                            self.render_category_filter_bar(ui, palette);
                            ui.add_space(6.0);
                        }
                    }
                });

            // Docked bottom chrome: honor ChromeSpec status_bar and path_bar_bottom
            let chrome_spec = crate::core::MossambiConfig::load().chrome_spec();
            if chrome_spec.status_bar {
                let status_resp =
                    egui::TopBottomPanel::bottom("explorer_status_bar")
                        .frame(egui::Frame::none().fill(palette.window_bg).inner_margin(
                            egui::Margin {
                                left: 10.0,
                                right: 10.0,
                                top: 0.0,
                                bottom: 2.0,
                            },
                        ))
                        .show_separator_line(false)
                        .show_inside(ui, |ui| {
                            self.render_status_bar(ui, palette);
                        });
                let status_rect = status_resp.response.rect;
                ui.painter().line_segment(
                    [
                        Pos2::new(status_rect.left(), status_rect.top() + 0.5),
                        Pos2::new(status_rect.right(), status_rect.top() + 0.5),
                    ],
                    Stroke::new(1.0_f32, palette.border),
                );
            }

            let path_bar_visible = chrome_spec.path_bar_bottom
                && !self.is_home_view
                && !parse_breadcrumbs(&self.current_dir).is_empty();
            if path_bar_visible {
                let path_resp =
                    egui::TopBottomPanel::bottom("explorer_path_bar")
                        .frame(egui::Frame::none().fill(palette.window_bg).inner_margin(
                            egui::Margin {
                                left: 10.0,
                                right: 10.0,
                                top: 0.0,
                                bottom: 3.0,
                            },
                        ))
                        .show_separator_line(false)
                        .show_inside(ui, |ui| {
                            self.render_bottom_path_bar(ui, palette);
                        });
                let path_rect = path_resp.response.rect;
                ui.painter().line_segment(
                    [
                        Pos2::new(path_rect.left(), path_rect.top() + 0.5),
                        Pos2::new(path_rect.right(), path_rect.top() + 0.5),
                    ],
                    Stroke::new(1.0_f32, palette.border),
                );
            }

            // File Table, Grid, or Miller Columns View fills the exact
            // remainder between the top chrome and the docked bars.
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::none()
                        .fill(palette.window_bg)
                        .inner_margin(egui::Margin {
                            left: 10.0,
                            right: 0.0,
                            top: 4.0,
                            bottom: 0.0,
                        }),
                )
                .show_inside(ui, |ui| {
                    self.render_file_view(
                        ui,
                        palette,
                        &mut on_stage_file,
                        &mut on_transcode_target,
                        &mut on_exam_conform,
                        &mut on_pack_archive,
                        &mut on_inspect_file,
                    );
                });
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
                        && nav_icon_button(ui, IconGlyph::ActionClose, 16.0)
                            .on_hover_text("Clear filter (Esc)")
                            .clicked()
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
                            .id(egui::Id::new("mossambi_global_search_input"))
                            .hint_text("Filter files (/)...")
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
                        && nav_icon_button(ui, IconGlyph::ActionClose, 16.0)
                            .on_hover_text("Clear filter (Esc)")
                            .clicked()
                    {
                        self.search_query.clear();
                        ui.memory_mut(|m| {
                            m.surrender_focus(egui::Id::new("mossambi_global_search_input"))
                        });
                    }
                    let box_w = if self.search_query.is_empty() {
                        avail_w
                    } else {
                        avail_w - 28.0
                    };
                    let search_resp = ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .id(egui::Id::new("mossambi_global_search_input"))
                            .hint_text("Filter files (/)...")
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
        let back_rounding = Rounding {
            nw: 6.0,
            sw: 6.0,
            ne: 0.0,
            se: 0.0,
        };
        let back_btn = match crate::core::icon_packs::icon_texture(
            ui.ctx(),
            pack,
            IconGlyph::ActionBack,
            20.0,
            nav_tint,
        ) {
            Some(t) => {
                egui::Button::image(egui::load::SizedTexture::new(t.id(), Vec2::splat(20.0)))
            }
            None => egui::Button::new("Back"),
        }
        .fill(if can_back {
            palette.card_bg
        } else {
            Color32::TRANSPARENT
        })
        .rounding(back_rounding);
        let back_resp = ui
            .add_enabled(can_back, back_btn)
            .on_hover_text("Back (Alt+Left / h)");
        if can_back {
            design::paint_pill_ring(
                ui.painter(),
                back_resp.rect,
                back_rounding,
                design::pill_state(&back_resp, false),
                palette.accent,
            );
        }
        if back_resp.clicked() {
            self.go_back();
        }

        let can_fwd = self.can_go_forward();
        let fwd_rounding = Rounding {
            nw: 0.0,
            sw: 0.0,
            ne: 6.0,
            se: 6.0,
        };
        let fwd_btn = match crate::core::icon_packs::icon_texture(
            ui.ctx(),
            pack,
            IconGlyph::ActionForward,
            20.0,
            nav_tint,
        ) {
            Some(t) => {
                egui::Button::image(egui::load::SizedTexture::new(t.id(), Vec2::splat(20.0)))
            }
            None => egui::Button::new("Forward"),
        }
        .fill(if can_fwd {
            palette.card_bg
        } else {
            Color32::TRANSPARENT
        })
        .rounding(fwd_rounding);
        let fwd_resp = ui
            .add_enabled(can_fwd, fwd_btn)
            .on_hover_text("Forward (Alt+Right)");
        if can_fwd {
            design::paint_pill_ring(
                ui.painter(),
                fwd_resp.rect,
                fwd_rounding,
                design::pill_state(&fwd_resp, false),
                palette.accent,
            );
        }
        if fwd_resp.clicked() {
            self.go_forward();
        }

        // Up Folder Button
        let up_btn = match crate::core::icon_packs::icon_texture(
            ui.ctx(),
            pack,
            IconGlyph::ActionUp,
            20.0,
            nav_tint,
        ) {
            Some(t) => {
                egui::Button::image(egui::load::SizedTexture::new(t.id(), Vec2::splat(20.0)))
            }
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
            ui.ctx(),
            pack,
            IconGlyph::ActionRefresh,
            20.0,
            nav_tint,
        ) {
            Some(t) => {
                egui::Button::image(egui::load::SizedTexture::new(t.id(), Vec2::splat(20.0)))
            }
            None => egui::Button::new("Refresh"),
        };
        if ui
            .add(refresh_btn)
            .on_hover_text("Refresh directory (F5)")
            .clicked()
        {
            self.navigate_to(self.current_dir.clone());
        }

        // New Folder Button
        let newfolder_btn = match crate::core::icon_packs::icon_texture(
            ui.ctx(),
            pack,
            IconGlyph::ActionNewFolder,
            20.0,
            nav_tint,
        ) {
            Some(t) => {
                egui::Button::image(egui::load::SizedTexture::new(t.id(), Vec2::splat(20.0)))
            }
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
        let dot_btn = egui::Button::new(egui::RichText::new(".dotfiles").size(12.0).color(dot_fg))
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
            // Finder-style title: the CURRENT FOLDER NAME only.
            //
            // The full ancestor chain lives in the bottom path bar, so
            // repeating it up here would duplicate the chrome and squeeze
            // the search field. The title is also the click target that opens
            // the inline path editor.
            let folder_name = self
                .current_dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| self.current_dir.to_string_lossy().into_owned());
            let title_text = egui::RichText::new(&folder_name)
                .size(15.0)
                .strong()
                .color(palette.text_primary);
            if ui
                .add(
                    egui::Button::new(title_text)
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::NONE),
                )
                .on_hover_text("Current folder - click to edit the path (Ctrl+L)")
                .clicked()
            {
                self.path_edit_buffer = self.current_dir.to_string_lossy().to_string();
                self.is_editing_path = true;
            }

            // Edit Path button
            if ui
                .button("Edit path")
                .on_hover_text("Edit path directly (Ctrl+L / Alt+D)")
                .clicked()
            {
                self.path_edit_buffer = self.current_dir.to_string_lossy().to_string();
                self.is_editing_path = true;
            }
        }
    }

    /// Renders the Finder-style path bar at the BOTTOM of the file area.
    ///
    /// macOS puts the path bar under the content, not above it: the top
    /// carries navigation and view controls, the bottom carries "where am I".
    /// Each segment is clickable (navigates to that ancestor) and is a drop
    /// target (drag a file onto an ancestor to move/copy it there).
    fn render_bottom_path_bar(&mut self, ui: &mut Ui, palette: &ThemePalette) {
        if self.is_home_view {
            return;
        }
        let segments = parse_breadcrumbs(&self.current_dir);
        if segments.is_empty() {
            return;
        }
        let mut breadcrumb_drop: Option<PendingDrop> = None;
        let mut breadcrumb_nav: Option<PathBuf> = None;
        let current_dir = self.current_dir.clone();

        // Finder parity: a long ancestor trail scrolls horizontally
        // instead of clipping (scrollbar appears only when needed).
        egui::ScrollArea::horizontal()
            .id_salt("breadcrumb_hscroll")
            .auto_shrink([false, true])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                    for (i, seg) in segments.iter().enumerate() {
                        if i > 0 {
                            crate::gui::finder_chrome::paint_path_chevron(ui, palette.text_muted);
                        }
                        let is_current = seg.target_path == current_dir;

                        // Authentic macOS Finder segment icon
                        let glyph = if i == 0 {
                            IconGlyph::Drive
                        } else {
                            IconGlyph::Folder
                        };
                        let (icon_rect, _) =
                            ui.allocate_exact_size(Vec2::splat(18.0), egui::Sense::hover());
                        let tint = if is_current {
                            [palette.accent.r(), palette.accent.g(), palette.accent.b()]
                        } else {
                            [
                                palette.text_secondary.r(),
                                palette.text_secondary.g(),
                                palette.text_secondary.b(),
                            ]
                        };
                        crate::core::icon_packs::paint_icon(
                            ui,
                            self.icon_pack,
                            glyph,
                            icon_rect,
                            tint,
                        );

                        let text =
                            egui::RichText::new(&seg.label)
                                .size(12.5)
                                .color(if is_current {
                                    palette.text_primary
                                } else {
                                    palette.text_secondary
                                });
                        let btn = egui::Button::new(text)
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE)
                            .min_size(Vec2::new(0.0, 24.0));
                        let resp = ui.add(btn);
                        if resp.clicked() {
                            let target = seg.target_path.clone();
                            self.navigate_to(target);
                        }

                        // Child-folder popup: Finder shows no caret
                        // affordance; right-click a segment to browse
                        // its subfolders (same interaction as the
                        // studio ribbon).
                        let pop_id = ui.make_persistent_id(format!(
                            "bc_pop_{}_{}",
                            i,
                            seg.target_path.display()
                        ));
                        if resp.secondary_clicked() {
                            ui.memory_mut(|m| m.toggle_popup(pop_id));
                        }
                        egui::popup::popup_below_widget(
                            ui,
                            pop_id,
                            &resp,
                            egui::PopupCloseBehavior::CloseOnClick,
                            |popup_ui| {
                                popup_ui.set_max_width(220.0);
                                popup_ui.set_max_height(260.0);
                                egui::ScrollArea::vertical().show(popup_ui, |pui| {
                                    // Once-per-open cache (F-2 fix): this
                                    // closure re-runs every frame while the
                                    // popup is open.
                                    let dirs = self.popup_child_dirs(&seg.target_path);
                                    if dirs.is_empty() {
                                        pui.label(
                                            egui::RichText::new("No subfolders")
                                                .size(11.0)
                                                .color(palette.text_muted),
                                        );
                                    } else {
                                        for sub in dirs {
                                            if pui
                                                .button(
                                                    egui::RichText::new(format!("> {}", sub.name))
                                                        .size(11.5),
                                                )
                                                .clicked()
                                            {
                                                breadcrumb_nav = Some(sub.path);
                                            }
                                        }
                                    }
                                });
                            },
                        );

                        // Dragging a file onto an ancestor path segment moves it there.
                        if !is_current {
                            if let Some(payload) = resp.dnd_hover_payload::<FileDragPayload>() {
                                let legal = payload
                                    .paths
                                    .iter()
                                    .all(|p| !same_or_within(&seg.target_path, p));
                                if legal {
                                    let a = self.hover_anims.drive(
                                        drop_highlight_id(&seg.target_path),
                                        1.0,
                                        self.animations.hover,
                                        crate::gui::anim::frame_dt(ui.ctx()),
                                    );
                                    ui.painter().rect_stroke(
                                        resp.rect,
                                        Rounding::same(5.0),
                                        Stroke::new(
                                            (a * 2.0).max(1.0),
                                            Color32::from_rgba_unmultiplied(
                                                palette.accent.r(),
                                                palette.accent.g(),
                                                palette.accent.b(),
                                                (a * 200.0) as u8,
                                            ),
                                        ),
                                    );
                                }
                            } else {
                                self.hover_anims.drive(
                                    drop_highlight_id(&seg.target_path),
                                    0.0,
                                    self.animations.hover,
                                    crate::gui::anim::frame_dt(ui.ctx()),
                                );
                            }
                            if let Some(payload) = resp.dnd_release_payload::<FileDragPayload>() {
                                let (ctrl, shift, alt) = ui.input(|i| {
                                    (
                                        i.modifiers.ctrl || i.modifiers.command,
                                        i.modifiers.shift,
                                        i.modifiers.alt,
                                    )
                                });
                                let released_right = ui.input(|i| {
                                    i.pointer.button_released(egui::PointerButton::Secondary)
                                });
                                let button = if released_right
                                    || payload.button
                                        == crate::core::drop_action::DropMouseButton::Right
                                {
                                    crate::core::drop_action::DropMouseButton::Right
                                } else {
                                    crate::core::drop_action::DropMouseButton::Left
                                };
                                let drop_pos = ui
                                    .input(|i| i.pointer.hover_pos())
                                    .unwrap_or(resp.rect.center());
                                breadcrumb_drop = Some(PendingDrop {
                                    dest_dir: seg.target_path.clone(),
                                    sources: payload.paths.to_vec(),
                                    button,
                                    modifiers: crate::core::drop_action::DropModifiers {
                                        ctrl,
                                        shift,
                                        alt,
                                    },
                                    position: drop_pos,
                                });
                            }
                        }
                    }
                });

                // Finder behavior: after navigation the path bar reveals
                // the current (right-most) segment.
                if self.needs_breadcrumb_scroll_align {
                    self.needs_breadcrumb_scroll_align = false;
                    ui.scroll_to_cursor(Some(egui::Align::Max));
                }
            });

        if let Some(pending) = breadcrumb_drop {
            self.handle_drop(pending);
        }
        if let Some(target) = breadcrumb_nav {
            self.navigate_to(target);
        }
    }

    fn render_category_filter_bar(&mut self, ui: &mut Ui, palette: &ThemePalette) {
        let avail_w = ui.available_width();
        let is_grid = self.view_mode == ViewMode::Grid;
        let toggle_w = if is_grid { 290.0_f32 } else { 210.0_f32 };
        let pills_w = (avail_w - toggle_w - 12.0).max(180.0);

        ui.horizontal(|ui| {
            // Left Container: Home toggle + compact filter popover.
            //
            // The file-type filter chips row was removed: a full row of
            // All/Images/Audio/Video/Docs/Archives/Code buttons crowded the
            // top chrome and broke the Finder-style layout. Filtering now
            // survives only inside a compact popover.
            ui.allocate_ui_with_layout(
                Vec2::new(pills_w, 28.0),
                egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(true),
                |ui| {
                    if self.is_home_directory() {
                        let btn_dash = egui::Button::new(
                            egui::RichText::new("Home Dashboard")
                                .size(12.0)
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
                        let dash_resp = ui
                            .add(btn_dash)
                            .on_hover_text("Show Quick Access & System Drives Dashboard");
                        design::paint_pill_ring(
                            ui.painter(),
                            dash_resp.rect,
                            Rounding::same(6.0),
                            design::pill_state(&dash_resp, self.is_home_view),
                            palette.accent,
                        );
                        if dash_resp.clicked() {
                            self.is_home_view = true;
                        }

                        let btn_files = egui::Button::new(
                            egui::RichText::new("Folder Files")
                                .size(12.0)
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
                        let files_resp = ui
                            .add(btn_files)
                            .on_hover_text("Browse all raw files in folder");
                        design::paint_pill_ring(
                            ui.painter(),
                            files_resp.rect,
                            Rounding::same(6.0),
                            design::pill_state(&files_resp, !self.is_home_view),
                            palette.accent,
                        );
                        if files_resp.clicked() {
                            self.is_home_view = false;
                        }

                        ui.separator();
                    }

                    self.render_filter_popover(ui, palette);
                },
            );

            // Right Pinned Container: macOS 3-Segmented Switcher (Icons / List / Columns) & Icon Size
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut switch_mode = self.view_mode;
                crate::gui::finder_chrome::render_segmented_view_switcher(
                    ui,
                    palette,
                    self.icon_pack,
                    self.view_mode,
                    |new_m| switch_mode = new_m,
                );
                self.view_mode = switch_mode;

                ui.add_space(6.0);

                let split_active = self.split_view_enabled;
                let split_btn = egui::Button::new(
                    egui::RichText::new("Dual (F3)")
                        .size(11.5)
                        .color(if split_active {
                            Color32::WHITE
                        } else {
                            palette.text_secondary
                        })
                        .strong(),
                )
                .fill(if split_active {
                    palette.accent
                } else {
                    palette.card_bg
                })
                .rounding(Rounding::same(6.0));
                let split_resp = ui
                    .add(split_btn)
                    .on_hover_text("Toggle Dual-Pane Split View (F3)");
                design::paint_pill_ring(
                    ui.painter(),
                    split_resp.rect,
                    Rounding::same(6.0),
                    design::pill_state(&split_resp, split_active),
                    palette.accent,
                );
                if split_resp.clicked() {
                    self.toggle_split_view();
                }

                let prev_active = self.show_inspector_pane;
                let prev_btn = egui::Button::new(
                    egui::RichText::new("Preview")
                        .size(11.5)
                        .color(if prev_active {
                            Color32::WHITE
                        } else {
                            palette.text_secondary
                        })
                        .strong(),
                )
                .fill(if prev_active {
                    palette.accent
                } else {
                    palette.card_bg
                })
                .rounding(Rounding::same(6.0));
                let prev_resp = ui
                    .add(prev_btn)
                    .on_hover_text("Toggle Inspector Preview Pane (Ctrl+Shift+P)");
                design::paint_pill_ring(
                    ui.painter(),
                    prev_resp.rect,
                    Rounding::same(6.0),
                    design::pill_state(&prev_resp, prev_active),
                    palette.accent,
                );
                if prev_resp.clicked() {
                    self.toggle_inspector_pane();
                }

                if is_grid {
                    ui.add_space(4.0);
                    ui.add_sized(
                        [75.0, 18.0],
                        egui::Slider::new(&mut self.icon_size, 64.0..=300.0).show_value(false),
                    );
                    ui.label(
                        egui::RichText::new(format!("{:.0}px", self.icon_size))
                            .size(11.0)
                            .color(palette.text_muted),
                    );
                }
            });
        });
    }

    /// Compact Finder-style filter popover that replaces the removed
    /// file-type chips row.
    ///
    /// The button shows a funnel glyph plus the active filter name; the
    /// popup lists every category with its live item count.
    fn render_filter_popover(&mut self, ui: &mut Ui, palette: &ThemePalette) {
        const CATEGORIES: [CategoryFilter; 7] = [
            CategoryFilter::All,
            CategoryFilter::Images,
            CategoryFilter::Audio,
            CategoryFilter::Video,
            CategoryFilter::Documents,
            CategoryFilter::Archives,
            CategoryFilter::Code,
        ];

        let active = self.category_filter;
        let is_filtered = active != CategoryFilter::All;
        let tint: [u8; 3] = if is_filtered {
            [255, 255, 255]
        } else {
            [
                palette.text_secondary.r(),
                palette.text_secondary.g(),
                palette.text_secondary.b(),
            ]
        };
        let btn = match crate::core::icon_packs::icon_texture(
            ui.ctx(),
            self.icon_pack,
            IconGlyph::ActionFilter,
            14.0,
            tint,
        ) {
            Some(tex) => egui::Button::image_and_text(
                egui::load::SizedTexture::new(tex.id(), Vec2::splat(14.0)),
                egui::RichText::new(active.label())
                    .size(12.0)
                    .strong()
                    .color(if is_filtered {
                        Color32::WHITE
                    } else {
                        palette.text_secondary
                    }),
            ),
            None => egui::Button::new(
                egui::RichText::new(format!("{} {}", FILTER_FUNNEL_GLYPH, active.label()))
                    .size(12.0)
                    .strong()
                    .color(if is_filtered {
                        Color32::WHITE
                    } else {
                        palette.text_secondary
                    }),
            ),
        }
        .fill(if is_filtered {
            palette.accent
        } else {
            palette.card_bg
        })
        .rounding(Rounding::same(6.0));

        let response = ui.add(btn).on_hover_text("Filter files by type");
        let popup_id = ui.make_persistent_id("explorer_filter_popover");
        if response.clicked() {
            ui.memory_mut(|mem| mem.toggle_popup(popup_id));
        }

        egui::popup::popup_below_widget(
            ui,
            popup_id,
            &response,
            egui::popup::PopupCloseBehavior::CloseOnClickOutside,
            |popup_ui| {
                popup_ui.set_min_width(180.0);
                popup_ui.label(
                    egui::RichText::new("Filter by Type")
                        .size(13.0)
                        .strong()
                        .color(palette.text_primary),
                );
                popup_ui.separator();

                for cat in CATEGORIES {
                    let is_active = self.category_filter == cat;
                    let count = match cat {
                        CategoryFilter::All => self.entries.len(),
                        _ => self.count_category(cat),
                    };
                    let row = egui::Button::new(
                        egui::RichText::new(format!("{}  ({})", cat.label(), count))
                            .size(12.0)
                            .color(if is_active {
                                palette.accent
                            } else {
                                palette.text_secondary
                            }),
                    )
                    .fill(if is_active {
                        palette.card_hover
                    } else {
                        Color32::TRANSPARENT
                    })
                    .rounding(Rounding::same(5.0));

                    if popup_ui.add(row).clicked() {
                        self.category_filter = cat;
                        popup_ui.close_menu();
                    }
                }
            },
        );
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

        if self.split_view_enabled {
            self.render_dual_pane_view(
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

        if self.show_inspector_pane && self.view_mode != ViewMode::Columns {
            let inspector_w = 280.0_f32;
            let main_w = (ui.available_width() - inspector_w - 8.0).max(300.0);
            let avail_h = ui.available_height();
            let nav_entries: Vec<FileItem> = self
                .filtered_and_sorted_entries()
                .into_iter()
                .cloned()
                .collect();
            let active_item = nav_entries.get(self.focused_row).cloned();

            let mut peek_to_open: Option<FileItem> = None;

            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    Vec2::new(main_w, avail_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| match self.view_mode {
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
                        ViewMode::Columns => unreachable!(),
                    },
                );

                if let Some(ref item) = active_item {
                    self.render_inspector_card(
                        ui,
                        palette,
                        item,
                        inspector_w,
                        avail_h,
                        on_transcode_target,
                        on_pack_archive,
                        &mut peek_to_open,
                    );
                }
            });

            if let Some(item) = peek_to_open {
                self.peek_item = Some(item);
            }
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

        // Drag ghost badge hovering at the pointer: live text "Move" / "Copy" / "Link"
        // based on active modifiers and user preference.
        if egui::DragAndDrop::has_payload_of_type::<FileDragPayload>(ui.ctx()) {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(16));
            let (ctrl, shift, alt) = ui.input(|i| {
                (
                    i.modifiers.ctrl || i.modifiers.command,
                    i.modifiers.shift,
                    i.modifiers.alt,
                )
            });
            let cfg = crate::core::MossambiConfig::load();
            let badge_text = if alt || (ctrl && shift) {
                "Link"
            } else if shift {
                "Move"
            } else if ctrl {
                "Copy"
            } else {
                match cfg.drop_action_pref {
                    crate::core::DropActionPreference::AlwaysCopy => "Copy",
                    crate::core::DropActionPreference::AlwaysMove => "Move",
                    crate::core::DropActionPreference::AlwaysAsk => "Ask",
                    crate::core::DropActionPreference::SmartDefault => "Move",
                }
            };
            if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                let bp = ui.painter().clone().with_layer_id(egui::LayerId::new(
                    egui::Order::Tooltip,
                    egui::Id::new("mossambi_dnd_badge"),
                ));
                let font = egui::FontId::proportional(13.0);
                let galley = ui.painter().layout_no_wrap(
                    badge_text.to_string(),
                    font.clone(),
                    palette.text_primary,
                );
                let pill_w = (galley.size().x + 16.0).max(28.0);
                let pill_h = 22.0;
                let r = Rect::from_min_size(pos + Vec2::new(14.0, 14.0), Vec2::new(pill_w, pill_h));
                bp.rect_filled(r, Rounding::same(6.0), palette.card_bg);
                bp.rect_stroke(r, Rounding::same(6.0), Stroke::new(1.0_f32, palette.accent));
                let text_pos = Pos2::new(
                    r.center().x - galley.size().x * 0.5,
                    r.center().y - galley.size().y * 0.5,
                );
                bp.galley(text_pos, galley, palette.text_primary);
            }
        }

        // Drop Action Menu popup
        self.render_drop_menu(ui, palette, on_transcode_target);
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
                    ui.label(egui::RichText::new("Quick Access").size(15.0).strong().color(palette.text_primary));
                });
                ui.add_space(8.0);

                let mut pinned_folders = vec![
                    (
                        "Desktop".to_string(),
                        "Desktop",
                        [0, 122, 255],
                        crate::core::platform_paths::desktop_dir(),
                    ),
                    (
                        "Downloads".to_string(),
                        "Downloads",
                        [20, 184, 166],
                        crate::core::platform_paths::downloads_dir(),
                    ),
                    (
                        "Documents".to_string(),
                        "Documents",
                        [59, 130, 246],
                        crate::core::platform_paths::documents_dir(),
                    ),
                    (
                        "Pictures".to_string(),
                        "Pictures",
                        [14, 165, 233],
                        crate::core::platform_paths::pictures_dir(),
                    ),
                    (
                        "Music".to_string(),
                        "Music",
                        [244, 63, 94],
                        crate::core::platform_paths::music_dir(),
                    ),
                    (
                        "Videos".to_string(),
                        "Videos",
                        [139, 92, 246],
                        crate::core::platform_paths::videos_dir(),
                    ),
                ];

                // Only real library folders belong in Quick access. The
                // previous code appended every visible subdirectory of the
                // user profile, which dumped NT junctions (NetHood,
                // PrintHood, SendTo, Start Menu, Recent, Templates, Local
                // Settings) into the grid.
                for entry in &self.entries {
                    if !entry.is_dir || entry.is_hidden {
                        continue;
                    }
                    if !is_library_folder_name(&entry.name) {
                        continue;
                    }
                    // Skip a junction: `is_symlink` is set for reparse
                    // points that point elsewhere in the profile tree.
                    if entry.is_symlink {
                        continue;
                    }
                    let lower = entry.name.to_lowercase();
                    if pinned_folders
                        .iter()
                        .any(|(label, _, _, _)| label.to_lowercase() == lower)
                    {
                        continue;
                    }
                    pinned_folders.push((entry.name.clone(), "Folder", [235, 175, 60], entry.path.clone()));
                }

                // Append Recycle Bin -> resolved cross-platform Recycle Bin target
                let recycle_target = crate::core::resolve_recycle_bin_target();
                pinned_folders.push((
                    "Recycle Bin".to_string(),
                    "Recycle Bin",
                    [16, 185, 129],
                    recycle_target,
                ));

                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(12.0, 12.0);

                    for (label, _glyph, rgb, path) in &pinned_folders {
                        let display_label = truncate_end(label, 14);
                        let card_size = Vec2::new(96.0, 80.0);
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
                        let badge_center = Pos2::new(rect.center().x, rect.top() + 30.0);
                        let big_rect = Rect::from_center_size(badge_center, Vec2::splat(32.0));
                        crate::core::icon_packs::paint_icon(
                            ui,
                            self.icon_pack,
                            icon_key,
                            big_rect,
                            [rgb[0], rgb[1], rgb[2]],
                        );

                        // Label below badge
                        ui.painter().text(
                            Pos2::new(rect.center().x, rect.bottom() - 14.0),
                            egui::Align2::CENTER_CENTER,
                            display_label,
                            egui::FontId::proportional(12.0),
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
                    ui.label(egui::RichText::new("Drives").size(15.0).strong().color(palette.text_primary));
                });
                ui.add_space(8.0);

                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(14.0, 12.0);

                    for drive in &self.drives {
                        let card_size = Vec2::new(210.0, 64.0);
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
                        let badge_center = Pos2::new(rect.left() + 24.0, rect.center().y);
                        let d_glyph = if drive.is_system {
                            crate::core::icon_packs::IconGlyph::DriveSystem
                        } else {
                            crate::core::icon_packs::IconGlyph::Drive
                        };
                        let big_rect = Rect::from_center_size(badge_center, Vec2::splat(28.0));
                        let d_rgb: [u8; 3] = if drive.is_system { [0, 122, 255] } else { [100, 116, 139] };
                        crate::core::icon_packs::paint_icon(ui, self.icon_pack, d_glyph, big_rect, d_rgb);

                        // Right: Title, Progress Bar, and Space details
                        let text_left = rect.left() + 48.0;
                        ui.painter().text(
                            Pos2::new(text_left, rect.top() + 10.0),
                            egui::Align2::LEFT_TOP,
                            &drive.display_name,
                            egui::FontId::proportional(13.0),
                            palette.text_primary,
                        );

                        // Usage progress bar
                        let bar_y = rect.top() + 30.0;
                        let bar_w = 145.0_f32;
                        let bar_h = 4.0_f32;
                        let bar_rect = Rect::from_min_size(Pos2::new(text_left, bar_y), Vec2::new(bar_w, bar_h));
                        ui.painter().rect_filled(
                            bar_rect,
                            Rounding::same(2.0),
                            Color32::from_rgba_unmultiplied(palette.border.r(), palette.border.g(), palette.border.b(), 120),
                        );
                        let fill_w = (bar_w * drive.usage_ratio().clamp(0.0, 1.0)).max(2.0);
                        let fill_rect = Rect::from_min_size(Pos2::new(text_left, bar_y), Vec2::new(fill_w, bar_h));
                        ui.painter().rect_filled(fill_rect, Rounding::same(2.0), palette.accent);

                        // Storage text (a drive whose capacity query failed gets an
                        // explicit note instead of an empty caption)
                        let capacity_text = if drive.total_bytes == 0 {
                            "Capacity unknown"
                        } else {
                            &drive.formatted_space()
                        };
                        ui.painter().text(
                            Pos2::new(text_left, rect.top() + 40.0),
                            egui::Align2::LEFT_TOP,
                            capacity_text,
                            egui::FontId::proportional(11.0),
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
                    ui.label(egui::RichText::new("Network Locations").size(15.0).strong().color(palette.text_primary));
                });
                ui.add_space(6.0);

                // Empty state, not an alert: no accent badge and no dead
                // "Disable" button (it never did anything).
                ui.label(
                    egui::RichText::new("No network locations found.")
                        .size(12.5)
                        .color(palette.text_muted),
                );
                ui.add_space(20.0);

                // ==========================================
                // 4. Recent Files List
                // ==========================================
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Recent Files").size(15.0).strong().color(palette.text_primary));
                });
                ui.add_space(8.0);

                let mut recent_files: Vec<&FileItem> = self
                    .entries
                    .iter()
                    .filter(|e| !e.is_dir && !e.is_hidden)
                    .collect();
                recent_files.sort_by_key(|e| std::cmp::Reverse(e.modified));
                recent_files.truncate(15);
                if recent_files.is_empty() {
                    ui.label(egui::RichText::new("No recent standalone files in home directory. Choose a folder above to browse.").size(12.0).color(palette.text_muted));
                } else {
                    for item in recent_files {
                        let row_h = 24.0;
                        let (row_rect, row_resp) = ui.allocate_exact_size(Vec2::new(ui.available_width() - 14.0, row_h), egui::Sense::click());

                        if row_resp.hovered() {
                            ui.painter().rect_filled(row_rect, Rounding::same(4.0), palette.card_hover);
                        }

                        let icon_spec = resolve_file_icon(&item.name, &item.extension, item.is_dir);
                        let (glyph_ast, accent_rgb) = crate::core::icon_packs::IconGlyph::resolve_with_path(
                            Some(&item.path),
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
                            egui::FontId::proportional(13.0),
                            palette.text_primary,
                        );

                        // Size and Modified date on right
                        ui.painter().text(
                            Pos2::new(row_rect.right() - 12.0, row_rect.center().y),
                            egui::Align2::RIGHT_CENTER,
                            item.formatted_size(),
                            egui::FontId::monospace(11.0),
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

        // -- Tools & Converters toolbar (sits directly above the parameter row) --
        self.render_action_toolbar(
            ui,
            palette,
            &entries,
            on_transcode_target,
            on_exam_conform,
            on_pack_archive,
            on_inspect_file,
        );

        // Finder-style column metrics: [checkbox] + one header per
        // ColumnLayout.visible column. Widths come from the persisted
        // layout; the NAME column grows to absorb the spare width so long
        // names are not middle-elided while the window is mostly empty
        // (without Name, the last visible column fills instead).
        let scrollbar_reserve = 12.0;
        let content_width = (ui.available_width() - scrollbar_reserve).max(320.0);
        let col_pad = 6.0_f32;
        // Checkbox door is reserved for Always and Hover modes alike (stable
        // layout, no jump on reveal); Off collapses it entirely.
        let checkbox_w = match self.checkbox_mode {
            CheckboxMode::Off => 0.0_f32,
            _ => 30.0_f32,
        };
        let available_for_cols = (content_width - checkbox_w).max(320.0);

        let visible: Vec<DetailColumn> = self.column_layout.visible.clone();
        let mut widths: Vec<f32> = visible
            .iter()
            .map(|c| self.column_layout.width(*c))
            .collect();
        let n_cols = visible.len();
        let flex_idx = visible
            .iter()
            .position(|c| *c == DetailColumn::Name)
            .unwrap_or(n_cols.saturating_sub(1));
        if n_cols > 0 {
            let fixed: f32 = widths
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != flex_idx)
                .map(|(_, w)| *w)
                .sum();
            let min_flex = visible[flex_idx].min_w().max(widths[flex_idx]);
            widths[flex_idx] = (available_for_cols - fixed).max(min_flex);
        }

        // Column header: flat interactive labels, no button boxes, hairline underneath
        let header_h = 26.0_f32;
        let (header_rect, _) =
            ui.allocate_exact_size(Vec2::new(content_width, header_h), egui::Sense::hover());
        let hp = ui.painter_at(header_rect);

        let mut header_sort: Option<SortColumn> = None;
        // Dynamic header cells over the visible column order. Each cell is
        // click-to-sort, right-click opens the visibility menu (Finder
        // parity; Name is locked), and carries a 6 px resize strip on its
        // right boundary (except the auto-filling last column).
        let mut header_specs: Vec<(Rect, DetailColumn)> = Vec::with_capacity(n_cols);
        let mut col_x = header_rect.left() + checkbox_w;
        for (ci, col) in visible.iter().enumerate() {
            header_specs.push((
                Rect::from_min_size(
                    Pos2::new(col_x, header_rect.top()),
                    Vec2::new(widths[ci], header_h),
                ),
                *col,
            ));
            col_x += widths[ci];
        }

        for (hrect, col) in header_specs.iter() {
            let right_align = col.aligns_right();
            let is_active = self.sort_column == col.sort_column();
            let filter_active = self.column_filters.contains_key(&col.id());
            let id = ui.make_persistent_id(("hdr", col.id()));
            let resp = ui.interact(*hrect, id, egui::Sense::click());
            if resp.hovered() {
                // Straight-alpha hover band: premul with raw card_hover rgb
                // rendered near-opaque over the header row.
                hp.rect_filled(
                    hrect.shrink2(Vec2::new(0.0, 3.0)),
                    Rounding::same(4.0),
                    Color32::from_rgba_unmultiplied(
                        palette.card_hover.r(),
                        palette.card_hover.g(),
                        palette.card_hover.b(),
                        if palette.is_dark { 45 } else { 28 },
                    ),
                );
            }
            let arrow = if is_active {
                format!(" {}", self.sort_direction.indicator())
            } else {
                String::new()
            };
            let funnel = if filter_active {
                format!(" {}", FILTER_FUNNEL_GLYPH)
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
                format!("{}{}{}", col.label(), arrow, funnel),
                egui::FontId::proportional(11.5),
                if is_active || filter_active {
                    palette.accent
                } else {
                    palette.text_muted
                },
            );
            if resp.clicked() {
                header_sort = Some(col.sort_column());
            }
            // Finder parity: right-click a header to choose which columns
            // are shown. Name is locked (always visible, always first).
            resp.context_menu(|menu_ui| {
                for (i, choice) in DetailColumn::all().into_iter().enumerate() {
                    if i > 0 {
                        menu_ui.add_space(3.0);
                    }
                    let shown = self.column_layout.visible.contains(&choice);
                    let locked = ColumnLayout::is_locked(choice);
                    let row_label = if locked {
                        format!("{} (locked)", choice.label())
                    } else {
                        choice.label().to_string()
                    };
                    let row = crate::gui::file_menu::menu_item_toggle(
                        menu_ui, &row_label, shown, locked, palette,
                    );
                    if row.clicked() && !locked {
                        if shown {
                            self.column_layout.hide(choice);
                        } else {
                            self.column_layout.show(choice);
                        }
                        self.save_column_layout();
                    }
                }
            });
        }

        // 6 px resize strips on header boundaries: dragging resizes the
        // column LEFT of the strip (Finder rule). The last visible column
        // auto-fills instead, so it gets no strip.
        for (i, (hrect, col)) in header_specs.iter().enumerate() {
            if i + 1 >= header_specs.len() {
                break;
            }
            let strip = Rect::from_min_size(
                Pos2::new(hrect.right() - 3.0, header_rect.top()),
                Vec2::new(6.0, header_h),
            );
            let sid = ui.make_persistent_id(("colsz", col.id()));
            let sresp = ui
                .interact(strip, sid, egui::Sense::drag())
                .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
            if sresp.dragged() {
                let delta = sresp.drag_delta().x;
                if delta != 0.0 {
                    let cur = self.column_layout.width(*col);
                    self.column_layout.set_width(*col, cur + delta);
                }
            }
            if sresp.drag_stopped() {
                self.save_column_layout();
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
        let peek_to_open: Option<FileItem> = None;
        let mut item_intents: Vec<ItemMenuIntents> = Vec::new();
        let mut canvas_intents = CanvasMenuIntents::default();
        let selection_snapshot: Vec<PathBuf> =
            self.selected_paths.iter().cloned().collect::<Vec<_>>();
        let mut new_hover: Option<(FileItem, Pos2)> = None;
        let mut drop_ops: Vec<DropOp> = Vec::new();
        let mut marquee_rects: Vec<Rect> = Vec::new();
        let mut canvas_dragging = false;
        let thumb_budget_reset =
            self.thumb_decode_budget.1.elapsed() > std::time::Duration::from_millis(300);
        if thumb_budget_reset {
            self.thumb_decode_budget = (0, Instant::now());
        }

        let icon_pack = self.icon_pack;
        let menu_style = self.menu_style;
        let has_clipboard = !self.clipboard_paths.is_empty();
        let tint = palette.text_secondary;

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                // Canvas interact registered FIRST so rows (registered after it) stay
                // on top: egui gives the pointer to the last-registered widget covering
                // it. This prevents the empty-canvas menu from swallowing row clicks -
                // the long-standing "click on a file does nothing" bug.
                let canvas_resp = ui.interact(
                    ui.available_rect_before_wrap(),
                    ui.make_persistent_id("explorer_canvas_bg"),
                    egui::Sense::click_and_drag(),
                );
                canvas_resp.context_menu(|menu_ui| {
                    let dir_desc = self.sort_direction == SortDirection::Descending;
                    canvas_intents = Self::run_canvas_context_menu(
                        menu_ui,
                        icon_pack,
                        menu_style,
                        palette.text_secondary,
                        has_clipboard,
                        !entries.is_empty(),
                        self.sort_column,
                        dir_desc,
                        self.view_mode,
                    );
                });

                // Rubber-band marquee can only start when the press began on
                // empty canvas: rows register after the canvas and steal the
                // pointer whenever the cursor is over them.
                if canvas_resp.drag_started_by(egui::PointerButton::Primary)
                    && !egui::DragAndDrop::has_payload_of_type::<FileDragPayload>(ui.ctx())
                {
                    if let Some(origin) = ui.input(|i| i.pointer.interact_pos()) {
                        self.marquee = Some((origin, self.selected_paths.clone()));
                    }
                }
                canvas_dragging = canvas_resp.dragged();

                ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);

                for (idx, item) in entries.iter().enumerate() {
                    let is_selected = self.selected_paths.contains(&item.path);
                    let row_height = self.row_density.details_row_height();

                    let (row_rect, row_resp) = ui.allocate_exact_size(
                        Vec2::new(content_width, row_height),
                        egui::Sense::click_and_drag(),
                    );
                    marquee_rects.push(row_rect);
                    // Off-screen rows keep their layout slot (scroll extent,
                    // marquee hit-test) but skip painting and per-row work.
                    if !ui.is_rect_visible(row_rect) {
                        continue;
                    }

                    // Selection disclosure is the checkbox ONLY (never a row band).
                    // Hover/Always reveal drives the alpha with the pack's spring.
                    if checkbox_w > 0.5 {
                        let cb_rect = Rect::from_center_size(
                            Pos2::new(row_rect.left() + checkbox_w * 0.5, row_rect.center().y),
                            Vec2::splat(16.0_f32.min(checkbox_w - 12.0)),
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
                                ui,
                                cb_rect,
                                is_selected,
                                chk_alpha,
                                palette,
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
                    let (icon_glyph, icon_accent) =
                        crate::core::icon_packs::IconGlyph::resolve_with_path(
                            Some(&item.path),
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

                    let row_tags = crate::core::tags::read_tags_cache(&item.path);
                    let dots_w = tag_dots_width(&row_tags);

                    // Dynamic per-column row cells over the visible column
                    // order (Finder parity). The name cell keeps its special
                    // painting: icon, tag dots, symlink suffix, hidden dim.
                    let mut cell_x = row_rect.left() + checkbox_w;
                    let rp = ui.painter();
                    for (ci, col) in visible.iter().enumerate() {
                        let w = widths[ci];
                        let cell_rect = Rect::from_min_size(
                            Pos2::new(cell_x, row_rect.top()),
                            Vec2::new(w, row_height),
                        );
                        if *col == DetailColumn::Name {
                            let name_x = cell_rect.left() + col_pad;
                            let icon_side = 22.0;
                            let icon_rect = Rect::from_center_size(
                                Pos2::new(name_x + 2.0 + icon_side * 0.5, row_rect.center().y),
                                Vec2::splat(icon_side),
                            );
                            crate::core::icon_packs::paint_icon(
                                ui,
                                self.icon_pack,
                                icon_glyph,
                                icon_rect,
                                icon_accent,
                            );
                            if item.is_symlink {
                                paint_alias_badge(ui.painter(), icon_rect);
                            }
                            // High-contrast text: selection is disclosed by the
                            // checkbox, so the name always sits on the normal
                            // fill (Finder detail).
                            let mut name_color = palette.text_primary;
                            if item.is_hidden {
                                name_color = Color32::from_rgba_unmultiplied(
                                    name_color.r(),
                                    name_color.g(),
                                    name_color.b(),
                                    130,
                                );
                            }
                            let name_font = egui::FontId::proportional(13.5);
                            // The name text starts after the icon + tag dots.
                            let name_pos_x = name_x + 28.0 + dots_w;
                            let name_avail = w - 32.0 - col_pad - dots_w;
                            let display = truncate_middle(&item.name, (name_avail / 7.0) as usize);
                            rp.text(
                                Pos2::new(name_pos_x, row_rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                display,
                                name_font,
                                name_color,
                            );
                            if !row_tags.is_empty() {
                                paint_tag_dots(
                                    ui,
                                    &row_tags,
                                    name_pos_x - 4.0,
                                    row_rect.center().y,
                                );
                            }
                        } else {
                            let (text, font, color, align) = match col {
                                DetailColumn::Name => (
                                    String::new(),
                                    egui::FontId::proportional(12.0),
                                    palette.text_secondary,
                                    egui::Align2::LEFT_CENTER,
                                ),
                                DetailColumn::DateModified => (
                                    item.formatted_date(),
                                    egui::FontId::proportional(12.0),
                                    palette.text_secondary,
                                    egui::Align2::LEFT_CENTER,
                                ),
                                DetailColumn::DateCreated => (
                                    item.formatted_created(),
                                    egui::FontId::proportional(12.0),
                                    palette.text_secondary,
                                    egui::Align2::LEFT_CENTER,
                                ),
                                DetailColumn::Size => {
                                    let size_text = self.dir_size_text(item);
                                    (
                                        size_text,
                                        egui::FontId::monospace(11.5),
                                        palette.text_secondary,
                                        egui::Align2::RIGHT_CENTER,
                                    )
                                }
                                DetailColumn::Kind => (
                                    icon_spec.category.to_string(),
                                    egui::FontId::proportional(12.0),
                                    palette.text_muted,
                                    egui::Align2::LEFT_CENTER,
                                ),
                                DetailColumn::Permissions => (
                                    item.formatted_permissions(),
                                    egui::FontId::monospace(11.5),
                                    palette.text_secondary,
                                    egui::Align2::LEFT_CENTER,
                                ),
                                DetailColumn::Hidden => (
                                    if item.is_hidden {
                                        "Yes".to_string()
                                    } else {
                                        String::new()
                                    },
                                    egui::FontId::proportional(12.0),
                                    palette.text_secondary,
                                    egui::Align2::LEFT_CENTER,
                                ),
                                DetailColumn::Symlink => (
                                    if item.is_symlink {
                                        "Link".to_string()
                                    } else {
                                        String::new()
                                    },
                                    egui::FontId::proportional(12.0),
                                    palette.text_secondary,
                                    egui::Align2::LEFT_CENTER,
                                ),
                                DetailColumn::SymlinkTarget => (
                                    item.symlink_target
                                        .as_ref()
                                        .map(|p| p.to_string_lossy().to_string())
                                        .unwrap_or_default(),
                                    egui::FontId::proportional(12.0),
                                    palette.text_secondary,
                                    egui::Align2::LEFT_CENTER,
                                ),
                                DetailColumn::Extension => (
                                    item.extension.clone(),
                                    egui::FontId::proportional(12.0),
                                    palette.text_secondary,
                                    egui::Align2::LEFT_CENTER,
                                ),
                                DetailColumn::Path => (
                                    item.path.to_string_lossy().to_string(),
                                    egui::FontId::proportional(12.0),
                                    palette.text_muted,
                                    egui::Align2::LEFT_CENTER,
                                ),
                            };
                            let tx_x = if col.aligns_right() {
                                cell_rect.right() - 4.0
                            } else {
                                cell_rect.left() + col_pad
                            };
                            let budget = ((w - 2.0 * col_pad) / 7.0).max(3.0) as usize;
                            let display = truncate_middle(&text, budget);
                            rp.text(
                                Pos2::new(tx_x, row_rect.center().y),
                                align,
                                display,
                                font,
                                color,
                            );
                        }
                        cell_x += w;
                    }

                    // -- Interaction: Finder semantics --
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

                    // Drag source (Explorer rule): dragging a selected item
                    // drags the whole selection; an unselected item is
                    // selected first, then dragged alone.
                    let primary_drag = row_resp.drag_started_by(egui::PointerButton::Primary);
                    let secondary_drag = row_resp.drag_started_by(egui::PointerButton::Secondary);
                    if primary_drag || secondary_drag {
                        if !is_selected {
                            self.selected_paths.clear();
                            self.selected_paths.insert(item.path.clone());
                            self.selection_anchor = Some(idx);
                        }
                        let paths: Vec<PathBuf> =
                            self.selected_paths.iter().cloned().collect::<Vec<_>>();
                        let button = if secondary_drag {
                            crate::core::drop_action::DropMouseButton::Right
                        } else {
                            crate::core::drop_action::DropMouseButton::Left
                        };
                        egui::DragAndDrop::set_payload(ui.ctx(), FileDragPayload { paths, button });
                    }

                    // Folder rows double as drop targets (move into dir,
                    // Ctrl/Cmd-drag = copy). Highlight animates via spring.
                    if item.is_dir {
                        let drop_hl = drop_highlight_id(&item.path);
                        if let Some(payload) = row_resp.dnd_hover_payload::<FileDragPayload>() {
                            let legal =
                                payload.paths.iter().all(|p| !same_or_within(&item.path, p));
                            if legal {
                                let a = self.hover_anims.drive(
                                    drop_hl,
                                    1.0,
                                    self.animations.hover,
                                    crate::gui::anim::frame_dt(ui.ctx()),
                                );
                                ui.painter().rect_filled(
                                    row_rect,
                                    Rounding::same(5.0),
                                    Color32::from_rgba_unmultiplied(
                                        palette.accent.r(),
                                        palette.accent.g(),
                                        palette.accent.b(),
                                        (a * 70.0) as u8,
                                    ),
                                );
                            }
                        } else {
                            self.hover_anims.drive(
                                drop_hl,
                                0.0,
                                self.animations.hover,
                                crate::gui::anim::frame_dt(ui.ctx()),
                            );
                        }
                        if let Some(payload) = row_resp.dnd_release_payload::<FileDragPayload>() {
                            let (ctrl, shift, alt) = ui.input(|i| {
                                (
                                    i.modifiers.ctrl || i.modifiers.command,
                                    i.modifiers.shift,
                                    i.modifiers.alt,
                                )
                            });
                            let released_right = ui.input(|i| {
                                i.pointer.button_released(egui::PointerButton::Secondary)
                            });
                            let button = if released_right
                                || payload.button
                                    == crate::core::drop_action::DropMouseButton::Right
                            {
                                crate::core::drop_action::DropMouseButton::Right
                            } else {
                                crate::core::drop_action::DropMouseButton::Left
                            };
                            let drop_pos = ui
                                .input(|i| i.pointer.hover_pos())
                                .unwrap_or(row_resp.rect.center());
                            drop_ops.push(PendingDrop {
                                dest_dir: item.path.clone(),
                                sources: payload.paths.to_vec(),
                                button,
                                modifiers: crate::core::drop_action::DropModifiers {
                                    ctrl,
                                    shift,
                                    alt,
                                },
                                position: drop_pos,
                            });
                        }
                    }

                    if row_resp.hovered() {
                        // Anchor next to the pointer: the row spans the whole
                        // width, so its right edge put the card far away.
                        let anchor = ui
                            .input(|i| i.pointer.hover_pos())
                            .unwrap_or(row_rect.right_center())
                            + Vec2::new(18.0, 14.0);
                        new_hover = Some((item.clone(), anchor));
                    }

                    // Comprehensive Right-Click Context Menu for the File Row.
                    // Pure menu builder returns intents; mutations apply below.
                    row_resp.context_menu(|menu_ui| {
                        let targets = if selection_snapshot.len() > 1
                            && selection_snapshot.contains(&item.path)
                        {
                            selection_snapshot.clone()
                        } else {
                            vec![item.path.clone()]
                        };
                        item_intents.clear();
                        item_intents.push(Self::run_item_context_menu(
                            menu_ui, icon_pack, menu_style, tint, item, &targets,
                        ));
                    });
                }
                if entries.is_empty() {
                    paint_empty_listing(ui, palette, &self.search_query);
                }
            });

        // (The empty-canvas context menu is registered FIRST inside the scroll
        // area above, so row interactions always win the pointer.)

        // Drop onto empty canvas targets the current directory. Rows above
        // already took their own releases; a surviving payload means the
        // pointer let go over empty space.
        if ui.input(|i| i.pointer.any_released()) {
            if let Some(payload) = egui::DragAndDrop::take_payload::<FileDragPayload>(ui.ctx()) {
                let (ctrl, shift, alt) = ui.input(|i| {
                    (
                        i.modifiers.ctrl || i.modifiers.command,
                        i.modifiers.shift,
                        i.modifiers.alt,
                    )
                });
                let released_right =
                    ui.input(|i| i.pointer.button_released(egui::PointerButton::Secondary));
                let button = if released_right
                    || payload.button == crate::core::drop_action::DropMouseButton::Right
                {
                    crate::core::drop_action::DropMouseButton::Right
                } else {
                    crate::core::drop_action::DropMouseButton::Left
                };
                let drop_pos = ui
                    .input(|i| i.pointer.hover_pos())
                    .unwrap_or_else(|| ui.available_rect_before_wrap().center());
                drop_ops.push(PendingDrop {
                    dest_dir: self.current_dir.clone(),
                    sources: payload.paths.to_vec(),
                    button,
                    modifiers: crate::core::drop_action::DropModifiers { ctrl, shift, alt },
                    position: drop_pos,
                });
            }
        }

        // Rubber-band marquee: live union of intersecting rows into the
        // selection (Ctrl adds to the pre-drag selection, plain drag replaces).
        if let Some((origin, ref base)) = self.marquee {
            if canvas_dragging {
                let cur = ui.input(|i| i.pointer.hover_pos()).unwrap_or(origin);
                let mrect = Rect::from_two_pos(origin, cur);
                let mp = ui.painter().clone().with_layer_id(egui::LayerId::new(
                    egui::Order::Foreground,
                    egui::Id::new("mossambi_marquee"),
                ));
                mp.rect_filled(
                    mrect,
                    Rounding::same(2.0),
                    Color32::from_rgba_unmultiplied(
                        palette.accent.r(),
                        palette.accent.g(),
                        palette.accent.b(),
                        30,
                    ),
                );
                mp.rect_stroke(
                    mrect,
                    Rounding::same(2.0),
                    Stroke::new(1.0_f32, palette.accent),
                );
                let additive = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
                let hits: Vec<PathBuf> = marquee_rects
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| r.intersects(mrect))
                    .filter_map(|(i, _)| entries.get(i).map(|e| e.path.clone()))
                    .collect();
                if additive {
                    for p in base.iter() {
                        self.selected_paths.insert(p.clone());
                    }
                } else {
                    self.selected_paths.clear();
                }
                for p in hits {
                    self.selected_paths.insert(p);
                }
            } else {
                self.marquee = None;
            }
        }

        // Keep springs + marquee + drag badge animating between pointer moves.
        if canvas_dragging || egui::DragAndDrop::has_payload_of_type::<FileDragPayload>(ui.ctx()) {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(16));
        }

        // Commit completed drops (rows push dir targets; empty canvas falls
        // back to the current directory above).
        for pending in drop_ops {
            self.handle_drop(pending);
        }

        // Apply menu intents (mutations live OUTSIDE the popup closures).
        if canvas_intents.new_folder {
            self.show_new_folder_dialog = true;
        }
        if canvas_intents.new_window {
            spawn_new_explorer_window(Some(&self.current_dir));
        }
        if canvas_intents.new_tab {
            self.add_tab(self.current_dir.clone());
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
        if let Some(col) = canvas_intents.sort_by {
            if self.sort_column == col {
                self.sort_direction = self.sort_direction.toggle();
            } else {
                self.sort_column = col;
                self.sort_direction = SortDirection::Ascending;
            }
        }
        if let Some(vm) = canvas_intents.view_mode {
            self.view_mode = vm;
        }
        if canvas_intents.invert_selection {
            let listed: std::collections::HashSet<PathBuf> =
                entries.iter().map(|e| e.path.clone()).collect();
            let currently: std::collections::HashSet<PathBuf> =
                self.selected_paths.intersection(&listed).cloned().collect();
            self.selected_paths = listed.difference(&currently).cloned().collect();
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
            self.apply_item_intent(
                ui,
                intent,
                on_transcode_target,
                on_exam_conform,
                on_pack_archive,
                on_inspect_file,
            );
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
        let style = self.menu_style;
        let bar_h = 28.0_f32;
        let (bar_rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), bar_h), egui::Sense::hover());

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
                let all_sel: Vec<PathBuf> = self.selected_paths.iter().cloned().collect::<Vec<_>>();
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
                    let text = egui::RichText::new(label).size(12.5).color(if enabled {
                        palette.text_primary
                    } else {
                        palette.text_muted
                    });
                    let tex = glyph.and_then(|g| {
                        crate::core::icon_packs::icon_texture(ui.ctx(), pack, g, 14.0, tint_rgb)
                    });
                    let mut btn = match tex {
                        Some(t) => egui::Button::image_and_text(
                            egui::load::SizedTexture::new(t.id(), Vec2::splat(14.0)),
                            text,
                        ),
                        None => egui::Button::new(text),
                    };
                    btn = btn
                        .fill(if enabled {
                            palette.card_bg
                        } else {
                            Color32::TRANSPARENT
                        })
                        .rounding(Rounding::same(6.0));
                    let pill_resp = ui.add_enabled(enabled, btn);
                    if enabled {
                        design::paint_pill_ring(
                            ui.painter(),
                            pill_resp.rect,
                            Rounding::same(6.0),
                            design::pill_state(&pill_resp, false),
                            palette.accent,
                        );
                    }
                    pill_resp
                };

                // -- CONVERT (Tools & Converters first group) --
                // Targets follow the FIRST selected file's type via the same
                // UniversalRegistry engine the radial wheel and the Studio
                // sidebar use - the menu is type-aware, not a fixed list.
                if has_files {
                    let lead = sel_files
                        .first()
                        .cloned()
                        .unwrap_or_else(|| self.current_dir.clone());
                    let sectors =
                        crate::core::registry::UniversalRegistry::get_sectors_for_file(&lead);
                    ui.menu_button("Convert", |menu_ui| {
                        menu_ui.set_min_width(200.0);
                        for fmt in &sectors {
                            match *fmt {
                                "MORE" | "ALL" => continue,
                                label => {
                                    if fm::menu_item_labeled(
                                        menu_ui,
                                        pack,
                                        fm::MenuAction::Transcode,
                                        &format!("Convert to {label}"),
                                        style,
                                        palette.accent,
                                    )
                                    .clicked()
                                    {
                                        for f in &sel_files {
                                            on_transcode_target(f.clone(), label.to_string());
                                        }
                                        menu_ui.close_menu();
                                    }
                                }
                            }
                        }
                        menu_ui.separator();
                        if fm::menu_item_labeled(
                            menu_ui,
                            pack,
                            fm::MenuAction::Transcode,
                            "All formats (batch)",
                            style,
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
                        egui::RichText::new("Convert")
                            .size(12.5)
                            .color(palette.text_muted),
                    )
                    .fill(Color32::TRANSPARENT)
                    .rounding(Rounding::same(6.0));
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
                    for f in &sel_files {
                        on_inspect_file(f.clone());
                    }
                }

                ui.separator();

                // -- Clipboard & file ops --
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
                if pill(
                    ui,
                    Some(IconGlyph::ActionRename),
                    "Rename",
                    self.selected_paths.len() == 1,
                )
                .clicked()
                {
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

                // -- Right-side: selection summary + select all/clear --
                // Width arbitration: the RTL group anchors at the row's right
                // edge with no knowledge of how far the left button pills
                // extend, so an overflowing item paints straight over them
                // (same bug class as the retired Select All chip below).
                // Measure the left group's right extent and budget the
                // selection chip into the gap between the two groups.
                let buttons_right = ui.min_rect().max.x;
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
                        // Chip budget: space between Clear's left edge and the
                        // left button group's right extent.
                        let clear_left = ui.min_rect().min.x;
                        let budget = clear_left - buttons_right - 12.0;
                        let bytes: u64 = entries
                            .iter()
                            .filter(|e| self.selected_paths.contains(&e.path) && !e.is_dir)
                            .map(|e| e.size_bytes)
                            .sum();
                        let full = if bytes > 0 {
                            format!(
                                "{} selected | {}",
                                pluralize_items(all_sel.len()),
                                format_file_size(bytes)
                            )
                        } else {
                            format!("{} selected", pluralize_items(all_sel.len()))
                        };
                        let short = format!("{} selected", pluralize_items(all_sel.len()));
                        let font = egui::FontId::proportional(12.0);
                        let full_w = ui
                            .painter()
                            .layout_no_wrap(full.clone(), font.clone(), palette.accent)
                            .size()
                            .x;
                        let label = if full_w <= budget {
                            full
                        } else {
                            let short_w = ui
                                .painter()
                                .layout_no_wrap(short.clone(), font.clone(), palette.accent)
                                .size()
                                .x;
                            if short_w <= budget {
                                short
                            } else {
                                // No room at all: the status bar already
                                // reports the same count and size, so hide
                                // the chip rather than paint over buttons.
                                String::new()
                            }
                        };
                        if !label.is_empty() {
                            ui.add_space(10.0);
                            ui.label(
                                egui::RichText::new(label)
                                    .size(12.0)
                                    .color(palette.text_secondary),
                            );
                        }
                    }
                    // No count label in the no-selection state: the status bar
                    // already reports the item total, and drawing it here
                    // collided with the "Select All" pill (both were painted
                    // into the same right-aligned slot).
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
        } else if preview_kind(&item.extension) != PreviewKind::None {
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
        // Never float a preview over an open menu or a modal dialog.
        if ui.ctx().memory(|m| m.any_popup_open())
            || self.renaming_path.is_some()
            || self.properties_item.is_some()
            || self.show_new_folder_dialog
            || self.permanent_delete_prompt.is_some()
        {
            return;
        }
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
                    .fill(Color32::from_rgba_unmultiplied(
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
                                        .size(14.0)
                                        .color(palette.text_primary),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} | {}",
                                        item.formatted_size(),
                                        resolve_file_icon(&item.name, &item.extension, false)
                                            .category
                                    ))
                                    .size(12.0)
                                    .color(palette.text_secondary),
                                );
                            }
                            HoverContent::Children(children, total) => {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}  |  {}",
                                        truncate_end(&item.name, 30),
                                        pluralize_items(*total)
                                    ))
                                    .strong()
                                    .size(14.0)
                                    .color(palette.text_primary),
                                );
                                ui.add_space(6.0);
                                for c in children.iter() {
                                    let spec = resolve_file_icon(&c.name, &c.extension, c.is_dir);
                                    let (cglyph, caccent) =
                                        crate::core::icon_packs::IconGlyph::resolve_with_path(
                                            Some(&c.path),
                                            &c.name,
                                            &c.extension,
                                            c.is_dir,
                                            spec.is_image,
                                            spec.is_media,
                                            spec.is_archive,
                                            spec.is_document,
                                            spec.is_code,
                                            spec.color_rgb,
                                        );
                                    ui.horizontal(|ui| {
                                        let (crect, _) = ui.allocate_exact_size(
                                            Vec2::splat(15.0),
                                            egui::Sense::hover(),
                                        );
                                        crate::core::icon_packs::paint_icon(
                                            ui,
                                            self.icon_pack,
                                            cglyph,
                                            crect,
                                            caccent,
                                        );
                                        ui.label(
                                            egui::RichText::new(truncate_end(&c.name, 30))
                                                .size(12.0)
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
                                            "... +{} more",
                                            total - children.len()
                                        ))
                                        .size(11.5)
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
                                    let (mglyph, maccent) =
                                        crate::core::icon_packs::IconGlyph::resolve_with_path(
                                            Some(&item.path),
                                            &item.name,
                                            &item.extension,
                                            item.is_dir,
                                            spec.is_image,
                                            spec.is_media,
                                            spec.is_archive,
                                            spec.is_document,
                                            spec.is_code,
                                            spec.color_rgb,
                                        );
                                    crate::core::icon_packs::paint_icon(
                                        ui,
                                        self.icon_pack,
                                        mglyph,
                                        mrect,
                                        maccent,
                                    );
                                    ui.vertical(|ui| {
                                        ui.label(
                                            egui::RichText::new(truncate_end(&item.name, 32))
                                                .strong()
                                                .size(14.0)
                                                .color(palette.text_primary),
                                        );
                                        ui.label(
                                            egui::RichText::new(spec.category)
                                                .size(12.0)
                                                .color(palette.text_secondary),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{} | {}",
                                                item.formatted_size(),
                                                item.formatted_date()
                                            ))
                                            .size(11.5)
                                            .color(palette.text_muted),
                                        );
                                    });
                                });
                                if item.is_dir {
                                    ui.add_space(4.0);
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} | Hold to peek contents",
                                            item.formatted_date()
                                        ))
                                        .size(11.5)
                                        .color(palette.text_muted),
                                    );
                                }
                            }
                        }
                    });
            });
        // Ensure repaint so the card animates in promptly
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(120));
    }

    /// Get (or lazily decode) a cached image thumbnail. Returns None while the
    /// per-frame decode budget is exhausted so the UI stays responsive.
    fn get_or_load_thumbnail(
        &mut self,
        ctx: &egui::Context,
        path: &Path,
        max_px: u32,
    ) -> Option<egui::TextureHandle> {
        // Per-frame decode budget: at most 6 decodes per 300ms window
        // (images + PDFs + video frames; the video path is additionally
        // capped at one concurrent ffmpeg spawn).
        if self.thumb_decode_budget.0 >= 6 {
            if self.thumb_decode_budget.1.elapsed() < std::time::Duration::from_millis(250) {
                ctx.request_repaint_after(std::time::Duration::from_millis(40));
                return None;
            }
            self.thumb_decode_budget = (0, Instant::now());
        }

        let mtime = std::fs::metadata(path).ok().and_then(|m| m.modified().ok());
        if let Some(entry) = self.thumb_cache.get(path) {
            if entry.mtime == mtime && entry.max_px >= max_px {
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
                format!("thumb_{}@{}", path.display(), max_px),
                img,
                egui::TextureOptions::LINEAR,
            )
        });
        self.thumb_cache.insert(
            path.to_path_buf(),
            CachedThumb {
                mtime,
                max_px,
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
        let peek_to_open: Option<FileItem> = None;
        let mut item_intents: Vec<ItemMenuIntents> = Vec::new();
        let mut canvas_intents = CanvasMenuIntents::default();
        let selection_snapshot: Vec<PathBuf> =
            self.selected_paths.iter().cloned().collect::<Vec<_>>();
        let mut new_hover: Option<(FileItem, Pos2)> = None;
        let mut drop_ops: Vec<DropOp> = Vec::new();
        let mut marquee_rects: Vec<Rect> = Vec::new();
        let mut canvas_dragging = false;
        let icon_pack = self.icon_pack;
        let menu_style = self.menu_style;
        let has_clipboard = !self.clipboard_paths.is_empty();

        // -- Tools & Converters toolbar over the grid --
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
            let budget =
                self.thumb_decode_budget.1.elapsed() > std::time::Duration::from_millis(300);
            if budget {
                self.thumb_decode_budget = (0, Instant::now());
            }
        }

        // Icon size scaled by the zoom level (Small -> Medium -> Large -> Extra large)
        let grid_zoom_scale = Self::EXPLORER_ZOOM_LEVELS[(self.explorer_zoom as usize).min(4)];
        let grid_checkboxes = self.checkbox_mode;
        let icon_sz = (self.icon_size * grid_zoom_scale).clamp(64.0, 300.0);
        // Proportional tile chrome: label and padding scale with the
        // icon so the name never crowds the glyph at small sizes and
        // the icon always paints at the requested size.
        let label_font = (icon_sz * 0.15).clamp(12.0, 22.0);
        let size_font = (label_font - 2.0).clamp(9.5, 12.0);
        let label_zone_h = label_font * 2.6;
        let bottom_chrome = label_zone_h + size_font + 10.0;
        let top_pad = (icon_sz * 0.10).clamp(8.0, 26.0);
        let tile_w = (icon_sz + 32.0).max(96.0);
        let tile_h = top_pad + icon_sz + bottom_chrome;
        let tile_size = Vec2::new(tile_w, tile_h);

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                // Canvas interact registered FIRST so tiles stay on top for clicks
                let canvas_resp = ui.interact(
                    ui.available_rect_before_wrap(),
                    ui.make_persistent_id("explorer_grid_canvas_bg"),
                    egui::Sense::click_and_drag(),
                );
                canvas_resp.context_menu(|menu_ui| {
                    let dir_desc = self.sort_direction == SortDirection::Descending;
                    canvas_intents = Self::run_canvas_context_menu(
                        menu_ui,
                        icon_pack,
                        menu_style,
                        palette.text_secondary,
                        has_clipboard,
                        !entries.is_empty(),
                        self.sort_column,
                        dir_desc,
                        self.view_mode,
                    );
                });

                // Rubber-band marquee starts only from empty canvas space.
                if canvas_resp.drag_started_by(egui::PointerButton::Primary)
                    && !egui::DragAndDrop::has_payload_of_type::<FileDragPayload>(ui.ctx())
                {
                    if let Some(origin) = ui.input(|i| i.pointer.interact_pos()) {
                        self.marquee = Some((origin, self.selected_paths.clone()));
                    }
                }
                canvas_dragging = canvas_resp.dragged();

                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(14.0, 14.0);

                    for item in &entries {
                        let is_checked = self.selected_paths.contains(&item.path);
                        let icon_spec = resolve_file_icon(&item.name, &item.extension, item.is_dir);

                        let (rect, resp) =
                            ui.allocate_exact_size(tile_size, egui::Sense::click_and_drag());
                        marquee_rects.push(rect);

                        // Card background on hover or selection (subtle Apple HIG outline, never covers icons)
                        // Selection tint: straight accent alpha, 45 dark / 26 light.
                        if is_checked {
                            ui.painter().rect_filled(
                                rect,
                                Rounding::same(8.0),
                                Color32::from_rgba_unmultiplied(
                                    palette.accent.r(),
                                    palette.accent.g(),
                                    palette.accent.b(),
                                    if palette.is_dark { 45 } else { 26 },
                                ),
                            );
                            ui.painter().rect_stroke(
                                rect,
                                Rounding::same(8.0),
                                Stroke::new(1.2_f32, palette.accent),
                            );
                        } else if resp.hovered() {
                            // Hover tint: straight text alpha, 20 dark / 12 light
                            // (premul with raw rgb drowned the tile content).
                            ui.painter().rect_filled(
                                rect,
                                Rounding::same(8.0),
                                Color32::from_rgba_unmultiplied(
                                    palette.text_primary.r(),
                                    palette.text_primary.g(),
                                    palette.text_primary.b(),
                                    if palette.is_dark { 20 } else { 12 },
                                ),
                            );
                            ui.painter().rect_stroke(
                                rect,
                                Rounding::same(8.0),
                                Stroke::new(
                                    1.0_f32,
                                    Color32::from_rgba_unmultiplied(
                                        palette.border.r(),
                                        palette.border.g(),
                                        palette.border.b(),
                                        120,
                                    ),
                                ),
                            );
                        }

                        // Top zone: delicate selection checkbox in top-left
                        let cb_sz = 13.0_f32;
                        let cb_rect = Rect::from_min_size(
                            rect.left_top() + Vec2::new(8.0, 8.0),
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
                        if grid_checkboxes != CheckboxMode::Off
                            && (grid_checkboxes == CheckboxMode::Always || chk_alpha > 0.02)
                        {
                            Self::paint_selection_checkbox(
                                ui, cb_rect, is_checked, chk_alpha, palette,
                            );
                        }
                        if cb_resp.clicked() {
                            if is_checked {
                                self.selected_paths.remove(&item.path);
                            } else {
                                self.selected_paths.insert(item.path.clone());
                            }
                        }

                        // Drag source (Explorer rule: selected set or the
                        // single tile) + folder-tile drop targets.
                        let primary_drag = resp.drag_started_by(egui::PointerButton::Primary);
                        let secondary_drag = resp.drag_started_by(egui::PointerButton::Secondary);
                        if primary_drag || secondary_drag {
                            if !is_checked {
                                self.selected_paths.clear();
                                self.selected_paths.insert(item.path.clone());
                            }
                            let paths: Vec<PathBuf> =
                                self.selected_paths.iter().cloned().collect::<Vec<_>>();
                            let button = if secondary_drag {
                                crate::core::drop_action::DropMouseButton::Right
                            } else {
                                crate::core::drop_action::DropMouseButton::Left
                            };
                            egui::DragAndDrop::set_payload(
                                ui.ctx(),
                                FileDragPayload { paths, button },
                            );
                        }
                        if item.is_dir {
                            let drop_hl = drop_highlight_id(&item.path);
                            if let Some(payload) = resp.dnd_hover_payload::<FileDragPayload>() {
                                let legal =
                                    payload.paths.iter().all(|p| !same_or_within(&item.path, p));
                                if legal {
                                    let a = self.hover_anims.drive(
                                        drop_hl,
                                        1.0,
                                        self.animations.hover,
                                        crate::gui::anim::frame_dt(ui.ctx()),
                                    );
                                    ui.painter().rect_filled(
                                        rect,
                                        Rounding::same(8.0),
                                        Color32::from_rgba_unmultiplied(
                                            palette.accent.r(),
                                            palette.accent.g(),
                                            palette.accent.b(),
                                            (a * 70.0) as u8,
                                        ),
                                    );
                                    ui.painter().rect_stroke(
                                        rect,
                                        Rounding::same(8.0),
                                        Stroke::new(2.0_f32, palette.accent),
                                    );
                                }
                            } else {
                                self.hover_anims.drive(
                                    drop_hl,
                                    0.0,
                                    self.animations.hover,
                                    crate::gui::anim::frame_dt(ui.ctx()),
                                );
                            }
                            if let Some(payload) = resp.dnd_release_payload::<FileDragPayload>() {
                                let (ctrl, shift, alt) = ui.input(|i| {
                                    (
                                        i.modifiers.ctrl || i.modifiers.command,
                                        i.modifiers.shift,
                                        i.modifiers.alt,
                                    )
                                });
                                let released_right = ui.input(|i| {
                                    i.pointer.button_released(egui::PointerButton::Secondary)
                                });
                                let button = if released_right
                                    || payload.button
                                        == crate::core::drop_action::DropMouseButton::Right
                                {
                                    crate::core::drop_action::DropMouseButton::Right
                                } else {
                                    crate::core::drop_action::DropMouseButton::Left
                                };
                                let drop_pos = ui
                                    .input(|i| i.pointer.hover_pos())
                                    .unwrap_or(resp.rect.center());
                                drop_ops.push(PendingDrop {
                                    dest_dir: item.path.clone(),
                                    sources: payload.paths.to_vec(),
                                    button,
                                    modifiers: crate::core::drop_action::DropModifiers {
                                        ctrl,
                                        shift,
                                        alt,
                                    },
                                    position: drop_pos,
                                });
                            }
                        }

                        // Middle zone: Icon / Thumbnail between the
                        // proportional top padding and the label chrome
                        let zone_top = rect.top() + top_pad;
                        let zone_bottom = rect.bottom() - bottom_chrome;
                        let icon_zone_h = zone_bottom - zone_top;
                        let icon_center = Pos2::new(rect.center().x, zone_top + icon_zone_h * 0.5);
                        let icon_box_side = icon_zone_h.min(tile_w - 24.0);

                        // Any previewable file: real thumbnail preview
                        // (lazily decoded, cached at the tile resolution)
                        let thumb = if !item.is_dir
                            && preview_kind(&item.extension) != PreviewKind::None
                        {
                            self.get_or_load_thumbnail(ui.ctx(), &item.path, thumb_max_px(icon_sz))
                        } else {
                            None
                        };

                        if let Some(tex) = &thumb {
                            let img_rect =
                                Rect::from_center_size(icon_center, Vec2::splat(icon_box_side));
                            let tex_wh = tex.size_vec2();
                            let target_wh = img_rect.size();
                            let scale = (target_wh.x / tex_wh.x).max(target_wh.y / tex_wh.y);
                            let shown = tex_wh * scale;
                            let crop = ((shown - target_wh) * 0.5) / shown;
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
                                Color32::from_rgba_unmultiplied(
                                    palette.border.r(),
                                    palette.border.g(),
                                    palette.border.b(),
                                    40,
                                ),
                            );
                            ui.painter_at(img_rect)
                                .image(tex.id(), img_rect, uv, Color32::WHITE);
                            ui.painter().rect_stroke(
                                img_rect,
                                Rounding::same(6.0),
                                Stroke::new(
                                    1.0_f32,
                                    if palette.is_dark {
                                        Color32::from_rgba_unmultiplied(255, 255, 255, 25)
                                    } else {
                                        Color32::from_rgba_unmultiplied(0, 0, 0, 25)
                                    },
                                ),
                            );
                        } else {
                            let (glyph, accent) =
                                crate::core::icon_packs::IconGlyph::resolve_with_path(
                                    Some(&item.path),
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
                            let irect =
                                Rect::from_center_size(icon_center, Vec2::splat(icon_box_side));
                            crate::core::icon_packs::paint_icon(
                                ui,
                                self.icon_pack,
                                glyph,
                                irect,
                                accent,
                            );
                            if item.is_symlink {
                                paint_alias_badge(ui.painter(), irect);
                            }
                        }

                        // Bottom zone: Filename (wrapped up to 2 lines, macOS Finder style) + Crisp File Size
                        let tile_tags = crate::core::tags::read_tags_cache(&item.path);
                        let raw_name = item.name.clone();

                        let tile_name_color = if item.is_hidden {
                            Color32::from_rgba_unmultiplied(
                                palette.text_primary.r(),
                                palette.text_primary.g(),
                                palette.text_primary.b(),
                                130,
                            )
                        } else {
                            palette.text_primary
                        };

                        let font_id = egui::FontId::proportional(label_font);
                        let max_text_w = (tile_w - 16.0).max(60.0);
                        let mut job = egui::text::LayoutJob::simple(
                            raw_name,
                            font_id,
                            tile_name_color,
                            max_text_w,
                        );
                        // Finder wraps at word boundaries and centres each line;
                        // break_anywhere split words mid-way ("Te / mplates").
                        job.wrap.max_rows = 2;
                        job.wrap.break_anywhere = false;
                        job.halign = egui::Align::Center;
                        let galley = ui.fonts(|f| f.layout_job(job));
                        let name_h = galley.size().y;
                        let name_x = rect.center().x;
                        let name_y =
                            (rect.bottom() - size_font - 10.0 - name_h).max(zone_bottom + 2.0);
                        ui.painter_at(rect).galley(
                            Pos2::new(name_x, name_y),
                            galley,
                            tile_name_color,
                        );

                        if !tile_tags.is_empty() {
                            paint_tag_dots(ui, &tile_tags, rect.right() - 14.0, rect.top() + 14.0);
                        }

                        let size_y = rect.bottom() - 11.0;
                        let size_text = self.dir_size_text(item);
                        let size_color = if palette.is_dark {
                            Color32::from_rgb(150, 160, 175)
                        } else {
                            Color32::from_rgb(105, 115, 130)
                        };
                        ui.painter_at(rect).text(
                            Pos2::new(rect.center().x, size_y),
                            egui::Align2::CENTER_CENTER,
                            size_text.as_str(),
                            egui::FontId::proportional(size_font),
                            size_color,
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

                        // Rich Context Menu on Grid Tile (pure builder -> intents).
                        resp.context_menu(|menu_ui| {
                            let targets = if selection_snapshot.len() > 1
                                && selection_snapshot.contains(&item.path)
                            {
                                selection_snapshot.clone()
                            } else {
                                vec![item.path.clone()]
                            };
                            item_intents.clear();
                            item_intents.push(Self::run_item_context_menu(
                                menu_ui,
                                icon_pack,
                                menu_style,
                                palette.text_secondary,
                                item,
                                &targets,
                            ));
                        });
                    }
                });
                if entries.is_empty() {
                    paint_empty_listing(ui, palette, &self.search_query);
                }
            });

        // (The grid empty-canvas context menu is registered FIRST inside the
        // scroll area above, so tile interactions always win the pointer.)

        // Drop onto empty canvas targets the current directory.
        if ui.input(|i| i.pointer.any_released()) {
            if let Some(payload) = egui::DragAndDrop::take_payload::<FileDragPayload>(ui.ctx()) {
                let (ctrl, shift, alt) = ui.input(|i| {
                    (
                        i.modifiers.ctrl || i.modifiers.command,
                        i.modifiers.shift,
                        i.modifiers.alt,
                    )
                });
                let released_right =
                    ui.input(|i| i.pointer.button_released(egui::PointerButton::Secondary));
                let button = if released_right
                    || payload.button == crate::core::drop_action::DropMouseButton::Right
                {
                    crate::core::drop_action::DropMouseButton::Right
                } else {
                    crate::core::drop_action::DropMouseButton::Left
                };
                let drop_pos = ui
                    .input(|i| i.pointer.hover_pos())
                    .unwrap_or_else(|| ui.available_rect_before_wrap().center());
                drop_ops.push(PendingDrop {
                    dest_dir: self.current_dir.clone(),
                    sources: payload.paths.to_vec(),
                    button,
                    modifiers: crate::core::drop_action::DropModifiers { ctrl, shift, alt },
                    position: drop_pos,
                });
            }
        }

        // Rubber-band marquee (tiles).
        if let Some((origin, ref base)) = self.marquee {
            if canvas_dragging {
                let cur = ui.input(|i| i.pointer.hover_pos()).unwrap_or(origin);
                let mrect = Rect::from_two_pos(origin, cur);
                let mp = ui.painter().clone().with_layer_id(egui::LayerId::new(
                    egui::Order::Foreground,
                    egui::Id::new("mossambi_marquee_grid"),
                ));
                mp.rect_filled(
                    mrect,
                    Rounding::same(2.0),
                    Color32::from_rgba_unmultiplied(
                        palette.accent.r(),
                        palette.accent.g(),
                        palette.accent.b(),
                        30,
                    ),
                );
                mp.rect_stroke(
                    mrect,
                    Rounding::same(2.0),
                    Stroke::new(1.0_f32, palette.accent),
                );
                let additive = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
                let hits: Vec<PathBuf> = marquee_rects
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| r.intersects(mrect))
                    .filter_map(|(i, _)| entries.get(i).map(|e| e.path.clone()))
                    .collect();
                if additive {
                    for p in base.iter() {
                        self.selected_paths.insert(p.clone());
                    }
                } else {
                    self.selected_paths.clear();
                }
                for p in hits {
                    self.selected_paths.insert(p);
                }
            } else {
                self.marquee = None;
            }
        }
        if canvas_dragging || egui::DragAndDrop::has_payload_of_type::<FileDragPayload>(ui.ctx()) {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(16));
        }
        for pending in drop_ops {
            self.handle_drop(pending);
        }

        // Apply menu intents (mutations live OUTSIDE the popup closures).
        if canvas_intents.new_folder {
            self.show_new_folder_dialog = true;
        }
        if canvas_intents.new_window {
            spawn_new_explorer_window(Some(&self.current_dir));
        }
        if canvas_intents.new_tab {
            self.add_tab(self.current_dir.clone());
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
        if let Some(col) = canvas_intents.sort_by {
            if self.sort_column == col {
                self.sort_direction = self.sort_direction.toggle();
            } else {
                self.sort_column = col;
                self.sort_direction = SortDirection::Ascending;
            }
        }
        if let Some(vm) = canvas_intents.view_mode {
            self.view_mode = vm;
        }
        if canvas_intents.invert_selection {
            let listed: std::collections::HashSet<PathBuf> =
                entries.iter().map(|e| e.path.clone()).collect();
            let currently: std::collections::HashSet<PathBuf> =
                self.selected_paths.intersection(&listed).cloned().collect();
            self.selected_paths = listed.difference(&currently).cloned().collect();
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
            self.apply_item_intent(
                ui,
                intent,
                on_transcode_target,
                on_exam_conform,
                on_pack_archive,
                on_inspect_file,
            );
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

    /// Shared Miller row renderer: one column body for the ancestor
    /// chain columns, the current directory column, and the opened
    /// descendant stack columns (Finder column-view parity -
    /// identical visuals and interaction in every role, with
    /// role-specific click dispatch). `col_salt` keys the per-column
    /// scroll state so freshly navigated columns reset their scroll
    /// once; `chain_highlight` marks the row that is currently open
    /// in the next column (Finder blue tint).
    #[allow(clippy::too_many_arguments)]
    fn render_miller_column(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        col_salt: &Path,
        entries: &[FileItem],
        role: ColumnRole,
        col_w: f32,
        full_h: f32,
        selection_snapshot: &[PathBuf],
        icon_pack: crate::core::icon_packs::IconPack,
        menu_style: crate::core::config::MenuStyle,
        chain_highlight: Option<&Path>,
        stack_index: usize,
        subfolder_to_load: &mut Option<(usize, PathBuf)>,
        nav_into_dir: &mut Option<PathBuf>,
        preview_to_set: &mut Option<FileItem>,
        column_drop: &mut Option<DropOp>,
        item_intents: &mut Vec<ItemMenuIntents>,
    ) {
        ui.allocate_ui_with_layout(
            Vec2::new(col_w, full_h),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                egui::Frame::none()
                    .fill(palette.card_bg)
                    .stroke(Stroke::new(1.0_f32, palette.border))
                    .show(ui, |ui| {
                        ui.set_min_size(Vec2::new(col_w, full_h));
                        egui::ScrollArea::vertical()
                            .id_salt(("miller_col_scroll", col_salt))
                            .auto_shrink([false, false])
                            .scroll_bar_visibility(
                                egui::scroll_area::ScrollBarVisibility::AlwaysVisible,
                            )
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    for (idx, item) in entries.iter().enumerate() {
                                        let chain_sel = role == ColumnRole::Ancestor
                                            && chain_highlight == Some(item.path.as_path());
                                        let is_selected = self.selected_paths.contains(&item.path)
                                            || (role == ColumnRole::Current
                                                && self.focused_row == idx)
                                            || chain_sel;
                                        let icon_spec = resolve_file_icon(
                                            &item.name,
                                            &item.extension,
                                            item.is_dir,
                                        );

                                        let display_name =
                                            truncate_middle(&item.name, (col_w / 8.5) as usize);

                                        let (row_rect, resp) = ui.allocate_exact_size(
                                            Vec2::new(col_w - 6.0, 30.0),
                                            egui::Sense::click_and_drag(),
                                        );
                                        // Drag source + folder-row drop targets
                                        let primary_drag = resp.drag_started_by(egui::PointerButton::Primary);
                                        let secondary_drag = resp.drag_started_by(egui::PointerButton::Secondary);
                                        if primary_drag || secondary_drag {
                                            if !is_selected {
                                                self.selected_paths.clear();
                                                self.selected_paths.insert(item.path.clone());
                                            }
                                            let paths: Vec<PathBuf> = self
                                                .selected_paths
                                                .iter()
                                                .cloned()
                                                .collect::<Vec<_>>();
                                            let button = if secondary_drag {
                                                crate::core::drop_action::DropMouseButton::Right
                                            } else {
                                                crate::core::drop_action::DropMouseButton::Left
                                            };
                                            egui::DragAndDrop::set_payload(
                                                ui.ctx(),
                                                FileDragPayload { paths, button },
                                            );
                                        }
                                        if item.is_dir {
                                            let drop_hl = drop_highlight_id(&item.path);
                                            if let Some(payload) =
                                                resp.dnd_hover_payload::<FileDragPayload>()
                                            {
                                                let legal = payload
                                                    .paths
                                                    .iter()
                                                    .all(|p| !same_or_within(&item.path, p));
                                                if legal {
                                                    let a = self.hover_anims.drive(
                                                        drop_hl,
                                                        1.0,
                                                        self.animations.hover,
                                                        crate::gui::anim::frame_dt(ui.ctx()),
                                                    );
                                                    ui.painter().rect_filled(
                                                        row_rect,
                                                        Rounding::same(5.0),
                                                        Color32::from_rgba_unmultiplied(
                                                            palette.accent.r(),
                                                            palette.accent.g(),
                                                            palette.accent.b(),
                                                            (a * 80.0) as u8,
                                                        ),
                                                    );
                                                }
                                            } else {
                                                self.hover_anims.drive(
                                                    drop_hl,
                                                    0.0,
                                                    self.animations.hover,
                                                    crate::gui::anim::frame_dt(ui.ctx()),
                                                );
                                            }
                                            if let Some(payload) =
                                                resp.dnd_release_payload::<FileDragPayload>()
                                            {
                                                let (ctrl, shift, alt) = ui.input(|i| {
                                                    (
                                                        i.modifiers.ctrl || i.modifiers.command,
                                                        i.modifiers.shift,
                                                        i.modifiers.alt,
                                                    )
                                                });
                                                let released_right = ui.input(|i| {
                                                    i.pointer.button_released(egui::PointerButton::Secondary)
                                                });
                                                let button = if released_right
                                                    || payload.button
                                                        == crate::core::drop_action::DropMouseButton::Right
                                                {
                                                    crate::core::drop_action::DropMouseButton::Right
                                                } else {
                                                    crate::core::drop_action::DropMouseButton::Left
                                                };
                                                let drop_pos = ui
                                                    .input(|i| i.pointer.hover_pos())
                                                    .unwrap_or(resp.rect.center());
                                                *column_drop = Some(PendingDrop {
                                                    dest_dir: item.path.clone(),
                                                    sources: payload.paths.to_vec(),
                                                    button,
                                                    modifiers: crate::core::drop_action::DropModifiers {
                                                        ctrl,
                                                        shift,
                                                        alt,
                                                    },
                                                    position: drop_pos,
                                                });
                                            }
                                        }
                                        // Row band: hover/selected/opened chain row
                                        if is_selected {
                                            // Selection tint: straight accent alpha,
                                            // 45 dark / 26 light (premul with raw
                                            // rgb rendered near-opaque).
                                            ui.painter().rect_filled(
                                                row_rect,
                                                Rounding::same(5.0),
                                                Color32::from_rgba_unmultiplied(
                                                    palette.accent.r(),
                                                    palette.accent.g(),
                                                    palette.accent.b(),
                                                    if palette.is_dark { 45 } else { 26 },
                                                ),
                                            );
                                        } else if resp.hovered() {
                                            ui.painter().rect_filled(
                                                row_rect,
                                                Rounding::same(5.0),
                                                Color32::from_rgba_unmultiplied(
                                                    palette.accent.r(),
                                                    palette.accent.g(),
                                                    palette.accent.b(),
                                                    18,
                                                ),
                                            );
                                        }
                                        // Pack vector icon on left
                                        let (icon_glyph, accent_rgb) =
                                            crate::core::icon_packs::IconGlyph::resolve_with_path(
                                                Some(&item.path),
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
                                            Pos2::new(row_rect.left() + 16.0, row_rect.center().y),
                                            Vec2::splat(22.0),
                                        );
                                        crate::core::icon_packs::paint_icon(
                                            ui, icon_pack, icon_glyph, icon_rect, accent_rgb,
                                        );
                                        if item.is_symlink {
                                            paint_alias_badge(ui.painter(), icon_rect);
                                        }
                                        // Name text right of icon, tag dots before it
                                        let name_color = if is_selected {
                                            palette.text_primary
                                        } else if item.is_hidden {
                                            palette.text_muted
                                        } else {
                                            palette.text_primary
                                        };
                                        let col_tags =
                                            crate::core::tags::read_tags_cache(&item.path);
                                        let col_dots_w = tag_dots_width(&col_tags);
                                        let col_name_x = row_rect.left() + 34.0 + col_dots_w;
                                        ui.painter().text(
                                            Pos2::new(col_name_x, row_rect.center().y),
                                            egui::Align2::LEFT_CENTER,
                                            &display_name,
                                            egui::FontId::proportional(13.5),
                                            name_color,
                                        );
                                        if !col_tags.is_empty() {
                                            paint_tag_dots(
                                                ui,
                                                &col_tags,
                                                col_name_x - 4.0,
                                                row_rect.center().y,
                                            );
                                        }
                                        // Folder chevron on far right (Finder style)
                                        if item.is_dir {
                                            ui.painter().text(
                                                Pos2::new(
                                                    row_rect.right() - 8.0,
                                                    row_rect.center().y,
                                                ),
                                                egui::Align2::RIGHT_CENTER,
                                                ">",
                                                egui::FontId::proportional(14.0),
                                                if is_selected {
                                                    palette.accent
                                                } else {
                                                    palette.text_muted
                                                },
                                            );
                                        }
                                        // Role-specific click dispatch
                                        if resp.clicked() {
                                            match role {
                                                ColumnRole::Ancestor => {
                                                    if !chain_sel {
                                                        self.selected_paths.clear();
                                                        self.selected_paths
                                                            .insert(item.path.clone());
                                                        if item.is_dir {
                                                            *nav_into_dir = Some(item.path.clone());
                                                        } else {
                                                            *preview_to_set = Some(item.clone());
                                                        }
                                                    }
                                                }
                                                ColumnRole::Current => {
                                                    self.focused_row = idx;
                                                    self.selected_paths.clear();
                                                    self.selected_paths.insert(item.path.clone());
                                                    if item.is_dir {
                                                        *subfolder_to_load =
                                                            Some((0, item.path.clone()));
                                                        self.column_preview_item = None;
                                                        *preview_to_set = None;
                                                    } else {
                                                        *preview_to_set = Some(item.clone());
                                                        self.column_stack.clear();
                                                    }
                                                }
                                                ColumnRole::Descendant => {
                                                    self.selected_paths.clear();
                                                    self.selected_paths.insert(item.path.clone());
                                                    if item.is_dir {
                                                        *subfolder_to_load = Some((
                                                            stack_index + 1,
                                                            item.path.clone(),
                                                        ));
                                                        self.column_preview_item = None;
                                                        *preview_to_set = None;
                                                    } else {
                                                        *preview_to_set = Some(item.clone());
                                                        self.column_stack.truncate(stack_index + 1);
                                                    }
                                                }
                                            }
                                        }
                                        if resp.double_clicked() {
                                            if item.is_dir {
                                                *nav_into_dir = Some(item.path.clone());
                                            } else {
                                                open_in_default_app(&item.path);
                                            }
                                        }

                                        // Right-click menu (same wiring as list/grid rows).
                                        resp.context_menu(|menu_ui| {
                                            let targets = if selection_snapshot.len() > 1
                                                && selection_snapshot.contains(&item.path)
                                            {
                                                selection_snapshot.to_vec()
                                            } else {
                                                vec![item.path.clone()]
                                            };
                                            item_intents.clear();
                                            item_intents.push(Self::run_item_context_menu(
                                                menu_ui,
                                                icon_pack,
                                                menu_style,
                                                palette.text_secondary,
                                                item,
                                                &targets,
                                            ));
                                        });
                                    }
                                });
                            });
                    });
            },
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn render_columns_view(
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
        let mut subfolder_to_load: Option<(usize, PathBuf)> = None;
        let mut preview_to_set: Option<FileItem> = None;
        let mut peek_to_open: Option<FileItem> = None;
        let mut column_drop: Option<DropOp> = None;

        // Context-menu intents (same wiring as the list/grid views).
        let mut item_intents: Vec<ItemMenuIntents> = Vec::new();
        let mut canvas_intents = CanvasMenuIntents::default();
        let selection_snapshot: Vec<PathBuf> =
            self.selected_paths.iter().cloned().collect::<Vec<_>>();
        let icon_pack = self.icon_pack;
        let menu_style = self.menu_style;
        let has_clipboard = !self.clipboard_paths.is_empty();

        // Auto-preview: open the inspector for the first file when the columns
        // view shows a folder with nothing selected yet. Directory rows are
        // intentionally NOT auto-expanded here, and the preview only fires for
        // entries that actually belong to the current directory: on the first
        // render after a navigation the async directory scan may still be in
        // flight and `entries` can momentarily hold the previous folder's
        // listing, which used to push a phantom column from an unrelated
        // directory (the anaconda3 root) into the column stack.
        if self.column_stack.is_empty() && self.column_preview_item.is_none() {
            if let Some(first) = entries.first() {
                if !first.is_dir && first.path.parent() == Some(self.current_dir.as_path()) {
                    self.column_preview_item = Some(first.clone());
                }
            }
        }

        let full_h = ui.available_height().max(320.0);
        egui::ScrollArea::horizontal()
            .id_salt("miller_outer_hscroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_height(full_h);

                // Canvas interact registered FIRST so column rows (registered
                // after it) keep pointer priority - same pattern as the
                // list/grid empty-canvas menus.
                let canvas_resp = ui.interact(
                    ui.available_rect_before_wrap(),
                    ui.make_persistent_id("explorer_miller_canvas_bg"),
                    egui::Sense::click_and_drag(),
                );
                canvas_resp.context_menu(|menu_ui| {
                    let dir_desc = self.sort_direction == SortDirection::Descending;
                    canvas_intents = Self::run_canvas_context_menu(
                        menu_ui,
                        icon_pack,
                        menu_style,
                        palette.text_secondary,
                        has_clipboard,
                        !entries.is_empty(),
                        self.sort_column,
                        dir_desc,
                        self.view_mode,
                    );
                });

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(1.0, 0.0);

                    // ==========================================
                    // DYNAMIC MILLER COLUMNS: Maximize Space Utilization
                    // ==========================================
                    let active_preview = self.column_preview_item.clone();
                    let has_inspector = active_preview
                        .as_ref()
                        .map(|item| !item.is_dir)
                        .unwrap_or(false);
                    let avail_w = ui.available_width().max(400.0);
                    let inspector_w = if has_inspector {
                        (avail_w * 0.28).clamp(280.0, 420.0)
                    } else {
                        0.0
                    };
                    // Miller lineage: columns are the ancestor chain
                    // (root-first) + the current directory + the opened
                    // descendant stack (Finder column-view layout).
                    let cur_dir = self.current_dir.clone();
                    let chain_cols: Vec<(PathBuf, Vec<FileItem>)> = ancestor_chain(&cur_dir)
                        .into_iter()
                        .map(|p| {
                            let sub = self.get_column_entries(&p);
                            (p, sub)
                        })
                        .collect();
                    let num_cols = (chain_cols.len() + 1 + self.column_stack.len()) as f32;
                    let cols_avail_w = (avail_w - inspector_w - (num_cols * 2.0)).max(220.0);
                    let natural_col_w = cols_avail_w / num_cols;
                    let col_w = natural_col_w.max(230.0);

                    // ==========================================
                    // MILLER CHAIN: ancestor columns, root-first (Finder
                    // column view renders the whole chain left of the
                    // current directory column)
                    // ==========================================
                    for (ci, (chain_path, chain_entries)) in chain_cols.iter().enumerate() {
                        let next_chain: PathBuf = chain_cols
                            .get(ci + 1)
                            .map(|(p, _)| p.clone())
                            .unwrap_or_else(|| cur_dir.clone());
                        self.render_miller_column(
                            ui,
                            palette,
                            chain_path,
                            chain_entries,
                            ColumnRole::Ancestor,
                            col_w,
                            full_h,
                            &selection_snapshot,
                            icon_pack,
                            menu_style,
                            Some(next_chain.as_path()),
                            0,
                            &mut subfolder_to_load,
                            &mut nav_into_dir,
                            &mut preview_to_set,
                            &mut column_drop,
                            &mut item_intents,
                        );
                    }

                    // Current directory column (click behavior identical
                    // to the old primary column; the vestigial subfolder
                    // state write is gone).
                    {
                        self.render_miller_column(
                            ui,
                            palette,
                            cur_dir.as_path(),
                            &entries,
                            ColumnRole::Current,
                            col_w,
                            full_h,
                            &selection_snapshot,
                            icon_pack,
                            menu_style,
                            None,
                            0,
                            &mut subfolder_to_load,
                            &mut nav_into_dir,
                            &mut preview_to_set,
                            &mut column_drop,
                            &mut item_intents,
                        );
                    }

                    // ==========================================
                    // MILLER STACK: opened descendant columns (kept
                    // right of the current directory column)
                    // ==========================================
                    let stack_clone: Vec<(PathBuf, Vec<FileItem>)> = self.column_stack.clone();
                    for (si, (sub_path, sub_items)) in stack_clone.iter().enumerate() {
                        self.render_miller_column(
                            ui,
                            palette,
                            sub_path,
                            sub_items,
                            ColumnRole::Descendant,
                            col_w,
                            full_h,
                            &selection_snapshot,
                            icon_pack,
                            menu_style,
                            None,
                            si,
                            &mut subfolder_to_load,
                            &mut nav_into_dir,
                            &mut preview_to_set,
                            &mut column_drop,
                            &mut item_intents,
                        );
                    }

                    // ==========================================
                    // macOS Inspector Card / File Preview (Files Only!)
                    // ==========================================
                    if let Some(ref item) = active_preview {
                        if !item.is_dir {
                            ui.allocate_ui_with_layout(
                                Vec2::new(inspector_w, full_h),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    self.render_inspector_card(
                                        ui,
                                        palette,
                                        item,
                                        inspector_w,
                                        full_h,
                                        on_transcode_target,
                                        on_pack_archive,
                                        &mut peek_to_open,
                                    );
                                },
                            );
                        }
                    }

                    // Finder behavior: after navigation or a newly opened
                    // column, align the hscroll to the deepest column and
                    // the inspector (the right-most content).
                    if self.needs_miller_scroll_align {
                        self.needs_miller_scroll_align = false;
                        ui.scroll_to_cursor(Some(egui::Align::Max));
                    }
                });
            });

        if let Some((insert_idx, sub_path)) = subfolder_to_load {
            let sub_entries = self.get_column_entries(&sub_path);
            self.column_stack.truncate(insert_idx);
            self.column_stack.push((sub_path, sub_entries));
            // Reveal the newly opened deepest column next frame.
            self.needs_miller_scroll_align = true;
        }
        if let Some(preview_item) = preview_to_set {
            self.column_preview_item = Some(preview_item);
        }
        if let Some(peek_item) = peek_to_open {
            self.peek_item = Some(peek_item);
        }
        if let Some(pending) = column_drop {
            self.handle_drop(pending);
        }
        if let Some(new_dir) = nav_into_dir {
            self.navigate_to(new_dir);
        }

        // Context-menu intents (mirrors the list view consumption).
        if canvas_intents.new_folder {
            self.show_new_folder_dialog = true;
        }
        if canvas_intents.new_window {
            spawn_new_explorer_window(Some(&self.current_dir));
        }
        if canvas_intents.new_tab {
            self.add_tab(self.current_dir.clone());
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
        if let Some(col) = canvas_intents.sort_by {
            if self.sort_column == col {
                self.sort_direction = self.sort_direction.toggle();
            } else {
                self.sort_column = col;
                self.sort_direction = SortDirection::Ascending;
            }
        }
        if let Some(vm) = canvas_intents.view_mode {
            self.view_mode = vm;
        }
        if canvas_intents.invert_selection {
            let listed: std::collections::HashSet<PathBuf> =
                entries.iter().map(|e| e.path.clone()).collect();
            let currently: std::collections::HashSet<PathBuf> =
                self.selected_paths.intersection(&listed).cloned().collect();
            self.selected_paths = listed.difference(&currently).cloned().collect();
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
            self.apply_item_intent(
                ui,
                intent,
                on_transcode_target,
                on_exam_conform,
                on_pack_archive,
                on_inspect_file,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_inspector_card(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        item: &FileItem,
        inspector_w: f32,
        col_h: f32,
        on_transcode_target: &mut impl FnMut(PathBuf, String),
        on_pack_archive: &mut impl FnMut(Vec<PathBuf>),
        peek_to_open: &mut Option<FileItem>,
    ) {
        let item_clone = item.clone();

        egui::Frame::none()
            .fill(palette.card_bg)
            .stroke(Stroke::new(1.0_f32, palette.border))
            .inner_margin(egui::Margin::same(12.0))
            .show(ui, |ui| {
                ui.set_min_size(Vec2::new(inspector_w, col_h));
                egui::ScrollArea::vertical()
                    .id_salt("miller_col_scroll_inspector")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let icon_spec = resolve_file_icon(
                            &item_clone.name,
                            &item_clone.extension,
                            item_clone.is_dir,
                        );

                        // Any previewable file Live Thumbnail or Media Scrubber or Icon
                        if preview_kind(&item_clone.extension) != PreviewKind::None {
                            let thumb_tex =
                                self.get_or_load_thumbnail(ui.ctx(), &item_clone.path, 320);
                            if let Some(tex) = thumb_tex {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(6.0);
                                    let tex_size = tex.size_vec2();
                                    let max_w = (inspector_w - 24.0).max(64.0);
                                    let max_h = 220.0_f32;
                                    // Aspect-ratio fitting: never blow up small icons past native 1.0x scale
                                    let scale =
                                        (max_w / tex_size.x).min(max_h / tex_size.y).min(1.0);
                                    let draw_w = (tex_size.x * scale).round().max(16.0);
                                    let draw_h = (tex_size.y * scale).round().max(16.0);
                                    ui.add(
                                        egui::Image::new(&tex)
                                            .fit_to_exact_size(Vec2::new(draw_w, draw_h))
                                            .rounding(Rounding::same(8.0)),
                                    );
                                    ui.add_space(4.0);
                                    ui.label(
                                        egui::RichText::new(&item_clone.name)
                                            .size(14.0)
                                            .strong()
                                            .color(palette.text_primary),
                                    );
                                    ui.label(
                                        egui::RichText::new(icon_spec.category)
                                            .size(12.0)
                                            .color(palette.text_muted),
                                    );
                                });
                            } else {
                                self.paint_inspector_icon_header(
                                    ui,
                                    palette,
                                    &item_clone,
                                    &icon_spec,
                                );
                            }
                        } else if icon_spec.is_media {
                            self.paint_inspector_icon_header(ui, palette, &item_clone, &icon_spec);
                            ui.add_space(4.0);
                            egui::Frame::none()
                                .fill(palette.card_bg)
                                .stroke(Stroke::new(1.0_f32, palette.border))
                                .rounding(Rounding::same(6.0))
                                .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new("[ > Play ]")
                                                .size(11.0)
                                                .color(palette.accent)
                                                .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new("00:00 / 03:45")
                                                .size(11.0)
                                                .color(palette.text_secondary),
                                        );
                                    });
                                    let mut scrub = 0.25_f32;
                                    ui.add(
                                        egui::Slider::new(&mut scrub, 0.0..=1.0).show_value(false),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "Codec / Format: {}",
                                            item_clone.extension
                                        ))
                                        .size(10.5)
                                        .color(palette.text_muted),
                                    );
                                });
                        } else if icon_spec.is_code
                            || icon_spec.is_document
                            || matches!(
                                item_clone.extension.as_str(),
                                "TXT" | "RS" | "PY" | "JSON" | "TOML" | "MD" | "YAML" | "YML"
                            )
                        {
                            self.paint_inspector_icon_header(ui, palette, &item_clone, &icon_spec);
                            if let Ok(content) = std::fs::read_to_string(&item_clone.path) {
                                let snippet: Vec<&str> = content.lines().take(6).collect();
                                if !snippet.is_empty() {
                                    ui.add_space(4.0);
                                    egui::Frame::none()
                                        .fill(Color32::from_black_alpha(40))
                                        .rounding(Rounding::same(4.0))
                                        .inner_margin(egui::Margin::same(6.0))
                                        .show(ui, |ui| {
                                            for (idx, line) in snippet.iter().enumerate() {
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        egui::RichText::new(format!(
                                                            "{:2} |",
                                                            idx + 1
                                                        ))
                                                        .size(10.5)
                                                        .monospace()
                                                        .color(palette.text_muted),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(*line)
                                                            .size(10.5)
                                                            .monospace()
                                                            .color(palette.text_primary),
                                                    );
                                                });
                                            }
                                        });
                                }
                            }
                        } else {
                            self.paint_inspector_icon_header(ui, palette, &item_clone, &icon_spec);
                        }

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);

                        // File Properties Grid
                        egui::Grid::new("inspector_props_grid")
                            .num_columns(2)
                            .spacing([12.0, 5.0])
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("Kind:")
                                        .size(12.0)
                                        .color(palette.text_muted),
                                );
                                ui.label(
                                    egui::RichText::new(if item_clone.is_dir {
                                        "Folder"
                                    } else {
                                        &item_clone.extension
                                    })
                                    .size(12.0)
                                    .strong()
                                    .color(palette.text_primary),
                                );
                                ui.end_row();

                                ui.label(
                                    egui::RichText::new("Size:")
                                        .size(12.0)
                                        .color(palette.text_muted),
                                );
                                ui.label(
                                    egui::RichText::new(item_clone.formatted_size())
                                        .size(12.0)
                                        .color(palette.text_secondary),
                                );
                                ui.end_row();

                                ui.label(
                                    egui::RichText::new("Modified:")
                                        .size(12.0)
                                        .color(palette.text_muted),
                                );
                                ui.label(
                                    egui::RichText::new(item_clone.formatted_date())
                                        .size(12.0)
                                        .color(palette.text_secondary),
                                );
                                ui.end_row();

                                ui.label(
                                    egui::RichText::new("Permissions:")
                                        .size(12.0)
                                        .color(palette.text_muted),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} ({})",
                                        item_clone.formatted_permissions(),
                                        item_clone.formatted_octal_permissions()
                                    ))
                                    .size(12.0)
                                    .monospace()
                                    .color(palette.text_secondary),
                                );
                                ui.end_row();

                                if item_clone.is_symlink {
                                    ui.label(
                                        egui::RichText::new("Symlink:")
                                            .size(12.0)
                                            .color(palette.text_muted),
                                    );
                                    let target_str = item_clone
                                        .symlink_target
                                        .as_ref()
                                        .map(|p| p.to_string_lossy().to_string())
                                        .unwrap_or_else(|| "unknown".into());
                                    ui.label(
                                        egui::RichText::new(format!("> {}", target_str))
                                            .size(12.0)
                                            .color(palette.accent),
                                    );
                                    ui.end_row();
                                }

                                ui.label(
                                    egui::RichText::new("Hidden:")
                                        .size(12.0)
                                        .color(palette.text_muted),
                                );
                                ui.label(
                                    egui::RichText::new(if item_clone.is_hidden {
                                        "Yes"
                                    } else {
                                        "No"
                                    })
                                    .size(12.0)
                                    .color(palette.text_secondary),
                                );
                                ui.end_row();

                                // SHA-256 Checksum calculation
                                ui.label(
                                    egui::RichText::new("SHA-256:")
                                        .size(12.0)
                                        .color(palette.text_muted),
                                );
                                if !self.calculate_checksums {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new("Off in Settings")
                                                .size(11.0)
                                                .italics()
                                                .color(palette.text_muted),
                                        );
                                        if ui
                                            .button(egui::RichText::new("Compute").size(10.5))
                                            .clicked()
                                        {
                                            let h =
                                                crate::codec::calculate_sha256(&item_clone.path)
                                                    .unwrap_or_else(|_| "Error".into());
                                            self.peek_hash_cache =
                                                Some((item_clone.path.clone(), h));
                                        }
                                    });
                                } else if let Some((cached_p, hash)) = &self.peek_hash_cache {
                                    if cached_p == &item_clone.path {
                                        let short_hash = if hash.len() > 16 {
                                            format!("{}...", &hash[..16])
                                        } else {
                                            hash.clone()
                                        };
                                        ui.label(
                                            egui::RichText::new(short_hash)
                                                .size(11.0)
                                                .monospace()
                                                .color(palette.accent),
                                        );
                                    } else {
                                        let h = crate::codec::calculate_sha256(&item_clone.path)
                                            .unwrap_or_else(|_| "Error".into());
                                        self.peek_hash_cache = Some((item_clone.path.clone(), h));
                                    }
                                } else if item_clone.size_bytes <= 16 * 1024 * 1024 {
                                    let h = crate::codec::calculate_sha256(&item_clone.path)
                                        .unwrap_or_else(|_| "Error".into());
                                    self.peek_hash_cache = Some((item_clone.path.clone(), h));
                                } else if ui
                                    .button(egui::RichText::new("Compute").size(10.5))
                                    .clicked()
                                {
                                    let h = crate::codec::calculate_sha256(&item_clone.path)
                                        .unwrap_or_else(|_| "Error".into());
                                    self.peek_hash_cache = Some((item_clone.path.clone(), h));
                                }
                                ui.end_row();
                            });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);

                        // Quick Action Buttons
                        ui.vertical_centered_justified(|ui| {
                            if ui
                                .button(egui::RichText::new("Quick Look (Space)").strong())
                                .clicked()
                            {
                                *peek_to_open = Some(item_clone.clone());
                            }
                            if !item_clone.is_dir {
                                if ui.button("Transcode / Convert").clicked() {
                                    on_transcode_target(item_clone.path.clone(), "ALL".into());
                                }
                                if icon_spec.is_image && ui.button("Rotate 90 deg").clicked() {
                                    if let Ok(()) = rotate_image_file_90(&item_clone.path) {
                                        self.thumb_cache.remove(&item_clone.path);
                                    }
                                }
                            } else if ui.button("Pack into ZIP Archive").clicked() {
                                on_pack_archive(vec![item_clone.path.clone()]);
                            }
                            if ui.button("Reveal in System Explorer").clicked() {
                                reveal_in_file_manager(&item_clone.path);
                            }
                            if ui.button("Copy Full Path").clicked() {
                                ui.ctx().output_mut(|o| {
                                    o.copied_text = item_clone.path.to_string_lossy().to_string();
                                });
                            }
                            if ui.button("Open Terminal Here").clicked() {
                                let dir_target = if item_clone.is_dir {
                                    &item_clone.path
                                } else {
                                    item_clone.path.parent().unwrap_or(&item_clone.path)
                                };
                                open_terminal_in_dir(dir_target);
                            }
                        });
                    });
            });
    }

    fn paint_inspector_icon_header(
        &self,
        ui: &mut Ui,
        palette: &ThemePalette,
        item: &FileItem,
        icon_spec: &crate::core::file_icons::FileIconSpec,
    ) {
        ui.vertical_centered(|ui| {
            ui.add_space(6.0);
            let (glyph, accent) = crate::core::icon_packs::IconGlyph::resolve_with_path(
                Some(&item.path),
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
            let (irect, _) = ui.allocate_exact_size(Vec2::splat(72.0), egui::Sense::hover());
            crate::core::icon_packs::paint_icon(ui, self.icon_pack, glyph, irect, accent);
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(&item.name)
                    .size(14.0)
                    .strong()
                    .color(palette.text_primary),
            );
            ui.label(
                egui::RichText::new(icon_spec.category)
                    .size(12.0)
                    .color(palette.text_muted),
            );
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn render_dual_pane_view(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        _on_stage_file: &mut impl FnMut(PathBuf),
        _on_transcode_target: &mut impl FnMut(PathBuf, String),
        _on_exam_conform: &mut impl FnMut(PathBuf),
        _on_pack_archive: &mut impl FnMut(Vec<PathBuf>),
        _on_inspect_file: &mut impl FnMut(PathBuf),
    ) {
        let avail_w = ui.available_width();
        let pane_w = (avail_w - 12.0) * 0.5;

        let mut primary_nav_target: Option<PathBuf> = None;
        let mut secondary_nav_target: Option<PathBuf> = None;
        let mut trigger_copy = false;
        let mut trigger_move = false;

        // Top Split Control Bar: F5 Copy, F6 Move, Tab Switch
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Dual-Pane Commander")
                    .size(12.0)
                    .strong()
                    .color(palette.text_primary),
            );
            ui.separator();
            if ui
                .button(egui::RichText::new("F5 Copy to Other").size(11.5))
                .clicked()
            {
                trigger_copy = true;
            }
            if ui
                .button(egui::RichText::new("F6 Move to Other").size(11.5))
                .clicked()
            {
                trigger_move = true;
            }
            if ui
                .button(egui::RichText::new("Tab Switch Focus").size(11.5))
                .clicked()
            {
                self.active_pane = 1 - self.active_pane;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(egui::RichText::new("Close Split (F3)").size(11.0))
                    .clicked()
                {
                    self.split_view_enabled = false;
                }
            });
        });
        ui.add_space(4.0);

        // Pane height from the exact remainder - no double reservation
        // for the split toolbar rendered above.
        let pane_h = ui.available_height().max(120.0);

        ui.horizontal(|ui| {
            // LEFT PANE (Primary)
            let is_primary_active = self.active_pane == 0;
            let primary_stroke = if is_primary_active {
                Stroke::new(1.5_f32, palette.accent)
            } else {
                Stroke::new(1.0_f32, palette.border)
            };

            egui::Frame::none()
                .fill(palette.card_bg)
                .stroke(primary_stroke)
                .rounding(Rounding::same(8.0))
                .inner_margin(egui::Margin::same(6.0))
                .show(ui, |ui| {
                    ui.set_width(pane_w);
                    ui.set_height(pane_h);
                    ui.horizontal(|ui| {
                        let active_badge = if is_primary_active {
                            "[Active]"
                        } else {
                            "[Pane 1]"
                        };
                        if ui
                            .button(
                                egui::RichText::new(active_badge)
                                    .size(11.0)
                                    .color(if is_primary_active {
                                        palette.accent
                                    } else {
                                        palette.text_muted
                                    })
                                    .strong(),
                            )
                            .clicked()
                        {
                            self.active_pane = 0;
                        }
                        let folder_name = self
                            .current_dir
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| self.current_dir.to_string_lossy().to_string());
                        ui.label(
                            egui::RichText::new(folder_name)
                                .size(12.0)
                                .strong()
                                .color(palette.text_primary),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(pluralize_items(self.entries.len()))
                                    .size(11.0)
                                    .color(palette.text_muted),
                            );
                        });
                    });
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .id_salt("left_pane_scroll")
                        .show(ui, |ui| {
                            let entries_clone = self.entries.clone();
                            for item in entries_clone {
                                let is_sel = self.selected_paths.contains(&item.path);
                                let prefix = if item.is_dir { "[DIR] " } else { "      " };
                                let label_text = format!("{}{}", prefix, item.name);
                                let resp = ui.selectable_label(
                                    is_sel,
                                    egui::RichText::new(label_text).size(12.0).color(if is_sel {
                                        palette.accent
                                    } else {
                                        palette.text_primary
                                    }),
                                );
                                if resp.clicked() {
                                    self.active_pane = 0;
                                    self.selected_paths.clear();
                                    self.selected_paths.insert(item.path.clone());
                                }
                                if resp.double_clicked() {
                                    self.active_pane = 0;
                                    if item.is_dir {
                                        primary_nav_target = Some(item.path);
                                    } else {
                                        open_in_default_app(&item.path);
                                    }
                                }
                            }
                        });
                });

            ui.add_space(4.0);

            // RIGHT PANE (Secondary)
            let is_secondary_active = self.active_pane == 1;
            let secondary_stroke = if is_secondary_active {
                Stroke::new(1.5_f32, palette.accent)
            } else {
                Stroke::new(1.0_f32, palette.border)
            };

            egui::Frame::none()
                .fill(palette.card_bg)
                .stroke(secondary_stroke)
                .rounding(Rounding::same(8.0))
                .inner_margin(egui::Margin::same(6.0))
                .show(ui, |ui| {
                    ui.set_width(pane_w);
                    ui.set_height(pane_h);
                    ui.horizontal(|ui| {
                        let active_badge = if is_secondary_active {
                            "[Active]"
                        } else {
                            "[Pane 2]"
                        };
                        if ui
                            .button(
                                egui::RichText::new(active_badge)
                                    .size(11.0)
                                    .color(if is_secondary_active {
                                        palette.accent
                                    } else {
                                        palette.text_muted
                                    })
                                    .strong(),
                            )
                            .clicked()
                        {
                            self.active_pane = 1;
                        }
                        let folder_name = self
                            .secondary_dir
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| self.secondary_dir.to_string_lossy().to_string());
                        ui.label(
                            egui::RichText::new(folder_name)
                                .size(12.0)
                                .strong()
                                .color(palette.text_primary),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(pluralize_items(self.secondary_entries.len()))
                                    .size(11.0)
                                    .color(palette.text_muted),
                            );
                        });
                    });
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .id_salt("right_pane_scroll")
                        .show(ui, |ui| {
                            let entries_clone = self.secondary_entries.clone();
                            for item in entries_clone {
                                let is_sel = self.secondary_selected_paths.contains(&item.path);
                                let prefix = if item.is_dir { "[DIR] " } else { "      " };
                                let label_text = format!("{}{}", prefix, item.name);
                                let resp = ui.selectable_label(
                                    is_sel,
                                    egui::RichText::new(label_text).size(12.0).color(if is_sel {
                                        palette.accent
                                    } else {
                                        palette.text_primary
                                    }),
                                );
                                if resp.clicked() {
                                    self.active_pane = 1;
                                    self.secondary_selected_paths.clear();
                                    self.secondary_selected_paths.insert(item.path.clone());
                                }
                                if resp.double_clicked() {
                                    self.active_pane = 1;
                                    if item.is_dir {
                                        secondary_nav_target = Some(item.path);
                                    } else {
                                        open_in_default_app(&item.path);
                                    }
                                }
                            }
                        });
                });
        });

        if trigger_copy {
            let (src_paths, dest_dir) = if self.active_pane == 0 {
                (
                    self.selected_paths.iter().cloned().collect::<Vec<_>>(),
                    self.secondary_dir.clone(),
                )
            } else {
                (
                    self.secondary_selected_paths
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>(),
                    self.current_dir.clone(),
                )
            };
            let _ = self.fs_worker_tx.send(FsOperation::Copy {
                sources: src_paths,
                dest_dir,
            });
        }

        if trigger_move {
            let (src_paths, dest_dir) = if self.active_pane == 0 {
                (
                    self.selected_paths.iter().cloned().collect::<Vec<_>>(),
                    self.secondary_dir.clone(),
                )
            } else {
                (
                    self.secondary_selected_paths
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>(),
                    self.current_dir.clone(),
                )
            };
            let _ = self.fs_worker_tx.send(FsOperation::Move {
                sources: src_paths,
                dest_dir,
            });
        }

        if let Some(target) = primary_nav_target {
            self.navigate_to(target);
        }
        if let Some(target) = secondary_nav_target {
            self.secondary_navigate_to(target);
        }
    }

    /// Finder-style size text: files show their formatted byte size;
    /// directories show the fresh computed recursive total, else "--".
    fn dir_size_text(&self, item: &FileItem) -> String {
        if !item.is_dir {
            return item.formatted_size();
        }
        match self.dir_sizes.get(&item.path) {
            Some((mtime, total)) if item.modified == *mtime => format_file_size(*total),
            _ => "--".to_string(),
        }
    }

    fn render_status_bar(&mut self, ui: &mut Ui, palette: &ThemePalette) {
        let bar_h = 26.0_f32;

        // Active-pane awareness: the shared status bar reflects the
        // active pane's directory, items, and selection in split mode.
        let use_secondary = self.split_view_enabled && self.active_pane == 1;
        let drive_dir = if use_secondary {
            self.secondary_dir.clone()
        } else {
            self.current_dir.clone()
        };
        let (total_items, total_hidden, sel_count, total_selected_bytes);
        if use_secondary {
            total_items = self
                .secondary_entries
                .iter()
                .filter(|e| !e.is_hidden)
                .count();
            total_hidden = self
                .secondary_entries
                .iter()
                .filter(|e| e.is_hidden)
                .count();
            sel_count = self.secondary_selected_paths.len();
            total_selected_bytes = self
                .secondary_entries
                .iter()
                .filter(|e| self.secondary_selected_paths.contains(&e.path))
                .map(|e| {
                    if e.is_dir {
                        match self.dir_sizes.get(&e.path) {
                            Some((mtime, total)) if e.modified == *mtime => *total,
                            _ => 0,
                        }
                    } else {
                        e.size_bytes
                    }
                })
                .sum();
        } else {
            total_items = self.filtered_and_sorted_entries().len();
            total_hidden = self.entries.iter().filter(|e| e.is_hidden).count();
            sel_count = self.selected_paths.len();
            total_selected_bytes = self
                .entries
                .iter()
                .filter(|e| self.selected_paths.contains(&e.path))
                .map(|e| {
                    if e.is_dir {
                        match self.dir_sizes.get(&e.path) {
                            Some((mtime, total)) if e.modified == *mtime => *total,
                            _ => 0,
                        }
                    } else {
                        e.size_bytes
                    }
                })
                .sum();
        }

        // Per-volume free space (Finder: "N items, X GB available").
        let drive_info = self
            .drives
            .iter()
            .find(|d| drive_dir.starts_with(&d.path))
            .map(|d| format_file_size(d.free_bytes));

        // Left: selection REPLACES the item count (Finder semantics).
        let items_text = if sel_count > 0 {
            // Finder: "2 of 53 selected, 4.1 MB".
            format!(
                "{} of {} selected, {}",
                sel_count,
                total_items,
                format_file_size(total_selected_bytes)
            )
        } else {
            let mut text = pluralize_items(total_items);
            if let Some(free) = &drive_info {
                text.push_str(", ");
                text.push_str(free);
                text.push_str(" available");
            }
            if total_hidden > 0 && !self.show_hidden_files {
                text.push_str(&format!(" ({} hidden)", total_hidden));
            }
            text
        };

        // Informational text stays neutral: the accent (often red) on
        // status text read like an error.
        let items_label = egui::RichText::new(items_text)
            .size(11.5)
            .color(palette.text_secondary);

        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), bar_h),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.label(items_label);

                // Right: zoom slider (Grid), dotfiles indicator. Finder
                // separates with spacing, not pipes.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.view_mode == ViewMode::Grid {
                        ui.add_sized(
                            [70.0, 16.0],
                            egui::Slider::new(&mut self.icon_size, 64.0..=300.0).show_value(false),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.0}px", self.icon_size))
                                .size(11.0)
                                .color(palette.text_muted),
                        );
                    }

                    if self.show_hidden_files {
                        ui.label(
                            egui::RichText::new("Hidden files shown")
                                .size(11.0)
                                .color(palette.text_muted),
                        );
                    }
                });
            },
        );
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

        let window_title = format!("Quick Look - {}", item.name);

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
                            .size(16.0)
                            .strong()
                            .color(palette.text_primary),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close (Esc)").clicked() {
                            close_peek = true;
                        }
                    });
                });

                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("Size: {}", item.formatted_size()))
                            .color(palette.text_secondary),
                    );
                    ui.label("|");
                    ui.label(
                        egui::RichText::new(format!("Modified: {}", item.formatted_date()))
                            .color(palette.text_secondary),
                    );
                    if !item.extension.is_empty() {
                        ui.label("|");
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

                let is_image = matches!(
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
                        | "HEIC"
                        | "RAW"
                );
                let is_pdf = item.extension == "PDF";

                if is_image || is_pdf {
                    if self.peek_texture.as_ref().map(|(p, _)| p) != Some(&item.path) {
                        self.peek_texture = load_image_thumbnail(ctx, &item.path)
                            .map(|tex| (item.path.clone(), tex));
                    }

                    if let Some((_, ref tex)) = self.peek_texture {
                        ui.vertical_centered(|ui| {
                            let tex_size = tex.size_vec2();
                            let max_w = 560.0_f32;
                            let max_h = 320.0_f32;
                            let scale = (max_w / tex_size.x).min(max_h / tex_size.y).min(1.0);
                            let draw_w = (tex_size.x * scale).round().max(16.0);
                            let draw_h = (tex_size.y * scale).round().max(16.0);
                            ui.add(
                                egui::Image::new(tex)
                                    .fit_to_exact_size(Vec2::new(draw_w, draw_h))
                                    .rounding(Rounding::same(8.0)),
                            );
                            ui.add_space(4.0);
                            let kind_label = if is_pdf {
                                "PDF Document Page"
                            } else {
                                "Image Dimensions"
                            };
                            ui.label(
                                egui::RichText::new(format!(
                                    "{}: {} x {} px  |  {:.1} KB",
                                    kind_label,
                                    tex_size.x as u32,
                                    tex_size.y as u32,
                                    item.size_bytes as f64 / 1024.0
                                ))
                                .size(11.0)
                                .color(palette.text_muted),
                            );
                        });
                    } else {
                        ui.label(
                            egui::RichText::new("Preview not available or unsupported raw format.")
                                .color(palette.text_muted),
                        );
                    }
                } else if is_text
                    || matches!(
                        item.extension.as_str(),
                        "DOCX"
                            | "ODT"
                            | "XLSX"
                            | "XLS"
                            | "ODS"
                            | "CSV"
                            | "TSV"
                            | "PPTX"
                            | "ODP"
                            | "RTF"
                    )
                {
                    if self.peek_text_cache.as_ref().map(|(p, _)| p) != Some(&item.path) {
                        let text = crate::core::preview::extract_preview_text(&item.path)
                            .unwrap_or_else(|e| format!("Preview unavailable: {}", e));
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
                            .size(12.0)
                            .monospace()
                            .color(palette.text_primary),
                    );

                    if !self.calculate_checksums {
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new("SHA-256: Off in Settings")
                                .size(11.5)
                                .italics()
                                .color(palette.text_muted),
                        );
                    } else {
                        if self.peek_hash_cache.as_ref().map(|(p, _)| p) != Some(&item.path) {
                            let hash = if item.size_bytes <= 8 * 1024 * 1024 {
                                crate::codec::calculate_sha256(&item.path)
                                    .unwrap_or_else(|_| "Unavailable".to_string())
                            } else {
                                format!(
                                    "{:.1} MB binary (SHA-256 deferred to avoid UI freeze)",
                                    item.size_bytes as f64 / (1024.0 * 1024.0)
                                )
                            };
                            self.peek_hash_cache = Some((item.path.clone(), hash));
                        }

                        if let Some((_, ref hash)) = self.peek_hash_cache {
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new(format!("SHA-256: {}", hash))
                                    .size(11.5)
                                    .monospace()
                                    .color(palette.accent),
                            );
                        }
                    }

                    if let Ok(bytes) = std::fs::read(&item.path) {
                        let sample = &bytes[..bytes.len().min(48)];
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("Binary Header Hex Dump:")
                                .size(12.0)
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
                                        .size(11.0)
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
                        if ui.button("Exam Conform").clicked() {
                            exam_to_open = true;
                        }
                        if ui.button("Quality Inspect").clicked() {
                            inspect_to_open = true;
                        }
                    }
                    if ui.button("Pack (ZIP)").clicked() {
                        pack_to_open = true;
                    }
                    if ui.button("Open Folder").clicked() {
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

        egui::Window::new(format!("Properties - {}", item.name))
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
                    let (picon_rect, _) =
                        ui.allocate_exact_size(Vec2::splat(64.0), egui::Sense::hover());
                    let (pglyph, paccent) = crate::core::icon_packs::IconGlyph::resolve_with_path(
                        Some(&item.path),
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
                    crate::core::icon_packs::paint_icon(
                        ui,
                        self.icon_pack,
                        pglyph,
                        picon_rect,
                        paccent,
                    );
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(&item.name)
                                .size(16.0)
                                .strong()
                                .color(palette.text_primary),
                        );
                        ui.label(
                            egui::RichText::new(if item.is_dir {
                                "Folder".to_string()
                            } else {
                                format!("{} File", item.extension)
                            })
                            .size(12.0)
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
                            .unwrap_or_else(|| "-".to_string());
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
                            if ui.button("Copy Path").clicked() {
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
                    if ui.button("Reveal in Explorer").clicked() {
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

fn rotate_image_file_90(path: &Path) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
    let rotated = img.rotate90();
    rotated.save(path).map_err(|e| e.to_string())
}

fn load_image_thumbnail(ctx: &egui::Context, path: &Path) -> Option<egui::TextureHandle> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_uppercase();

    if ext == "PDF" {
        if let Ok(rgba) = crate::codec::render_pdf_first_page_thumbnail(path, 640) {
            let (w, h) = rgba.dimensions();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [w as usize, h as usize],
                &rgba.into_raw(),
            );
            return Some(ctx.load_texture(
                path.to_string_lossy(),
                color_image,
                egui::TextureOptions::LINEAR,
            ));
        }
    }

    if let Ok(bytes) = std::fs::read(path) {
        if let Ok(img) = image::load_from_memory(&bytes) {
            let (orig_w, orig_h) = (img.width(), img.height());
            let thumb = if orig_w <= 520 && orig_h <= 320 {
                img.to_rgba8()
            } else {
                img.resize(520, 320, image::imageops::FilterType::Lanczos3)
                    .to_rgba8()
            };
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

/// Which kind of raster preview a file extension can produce.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PreviewKind {
    /// No raster preview; the painted vector icon is used.
    None,
    /// Decodable still image (GIF uses its first frame).
    Image,
    /// PDF first page.
    Pdf,
    /// Video first frame (ffmpeg; degrades to the vector icon when
    /// ffmpeg is unavailable).
    Video,
}

/// Map a file extension (any case) to its raster-preview capability.
pub fn preview_kind(ext: &str) -> PreviewKind {
    match ext.to_ascii_uppercase().as_str() {
        "JPG" | "JPEG" | "PNG" | "WEBP" | "GIF" | "AVIF" | "TIFF" | "BMP" | "ICO" | "HEIC"
        | "RAW" => PreviewKind::Image,
        "PDF" => PreviewKind::Pdf,
        "MP4" | "MKV" | "AVI" | "MOV" | "WEBM" | "M4V" | "WMV" | "FLV" | "TS" | "M2TS" | "MTS"
        | "3GP" | "3G2" | "OGV" | "VOB" | "MPG" | "MPEG" | "F4V" => PreviewKind::Video,
        _ => PreviewKind::None,
    }
}

/// Thumbnail decode resolution for a grid tile of `icon_sz` css px:
/// never below 128 (avoids re-decoding during small zoom changes),
/// never above 512.
pub fn thumb_max_px(icon_sz: f32) -> u32 {
    (icon_sz.ceil() as u32).clamp(128, 512)
}

/// Decode any previewable file down to a `max_px` thumbnail ColorImage
fn decode_thumbnail(path: &Path, max_px: u32) -> Option<egui::ColorImage> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_uppercase();

    match preview_kind(&ext) {
        PreviewKind::Pdf => {
            let rgba = crate::codec::render_pdf_first_page_thumbnail(path, max_px).ok()?;
            let (w, h) = rgba.dimensions();
            Some(egui::ColorImage::from_rgba_unmultiplied(
                [w as usize, h as usize],
                &rgba.into_raw(),
            ))
        }
        PreviewKind::Video => {
            let rgba = crate::codec::video_first_frame_thumbnail(path, max_px).ok()?;
            let (w, h) = rgba.dimensions();
            Some(egui::ColorImage::from_rgba_unmultiplied(
                [w as usize, h as usize],
                &rgba.into_raw(),
            ))
        }
        PreviewKind::Image => {
            let bytes = std::fs::read(path).ok()?;
            let img = image::load_from_memory(&bytes).ok()?;
            let (orig_w, orig_h) = (img.width(), img.height());
            let thumb = if orig_w <= max_px && orig_h <= max_px {
                img.to_rgba8()
            } else {
                img.resize(max_px, max_px, image::imageops::FilterType::Lanczos3)
                    .to_rgba8()
            };
            let (w, h) = thumb.dimensions();
            Some(egui::ColorImage::from_rgba_unmultiplied(
                [w as usize, h as usize],
                &thumb.into_raw(),
            ))
        }
        PreviewKind::None => None,
    }
}

/// Truncate a label in the middle ("verylongfilename.txt" -> "verylo...ame.txt")
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
    format!("{}...{}", head_str, tail_str)
}

/// Truncate at the end ("panchang-widget-desktop" -> "panchang-wi...")
fn truncate_end(s: &str, max_chars: usize) -> String {
    let total = s.chars().count();
    if total <= max_chars {
        return s.to_string();
    }
    let keep = max_chars.saturating_sub(1);
    format!("{}...", s.chars().take(keep).collect::<String>())
}

/// Renders the active modifier set as a compact, ASCII, stable string.
///
/// The journal stores this alongside every keystroke so a history panel can
/// tell "Ctrl+Z" apart from a bare "z".
fn modifier_string(mods: egui::Modifiers) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if mods.ctrl {
        parts.push("CTRL");
    }
    if mods.shift {
        parts.push("SHIFT");
    }
    if mods.alt {
        parts.push("ALT");
    }
    if mods.command {
        parts.push("CMD");
    }
    if parts.is_empty() {
        "NONE".to_string()
    } else {
        parts.join("+")
    }
}

/// The width a row must reserve for its tag dots at the given tag count.
///
/// Finder shows one coloured dot per assigned tag immediately before the
/// name. The dots are painted rather than laid out, so the caller reserves
/// this width up front and the name never collides with them.
pub fn tag_dots_width(tags: &[String]) -> f32 {
    if tags.is_empty() {
        0.0
    } else {
        (tags.len() as f32) * 10.0 + 4.0
    }
}

/// Draws the tag dots for `tags` with the right edge at `right_x`.
///
/// Tags are drawn right-to-left so the first tag sits closest to the name,
/// matching Finder's ordering.
fn paint_tag_dots(ui: &Ui, tags: &[String], right_x: f32, center_y: f32) {
    if tags.is_empty() {
        return;
    }
    let painter = ui.painter();
    let mut x = right_x;
    for tag in tags.iter().rev() {
        let color = crate::core::tags::tag_color(tag);
        painter.circle_filled(egui::Pos2::new(x - 4.0, center_y), 3.5, color);
        x -= 10.0;
    }
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

/// Observability floor for file operations: every filesystem side effect is
/// logged (error level on failure, debug on success) so the explorer never
/// fails silently and post-mortems have a paper trail.
fn log_result<T, E: std::fmt::Display>(what: &str, path: &Path, res: Result<T, E>) {
    match res {
        Ok(_) => log::debug!("{what} ok: {}", path.display()),
        Err(e) => log::error!("{what} failed for {}: {e}", path.display()),
    }
}

/// Payload carried while files are dragged inside the explorer (Explorer
/// parity: drag to move, Ctrl-drag to copy). When such a drag leaves the
/// window bounds, the render loop hands it to the native drag mechanism
/// (Windows OLE / Linux file clipboard) via `begin_file_drag_out`.
#[derive(Clone, Debug)]
pub struct FileDragPayload {
    pub paths: Vec<PathBuf>,
    pub button: crate::core::drop_action::DropMouseButton,
}

/// One pending drop: collected during a render frame, then dispatched
/// to `handle_drop`.
#[derive(Clone, Debug)]
pub struct PendingDrop {
    pub dest_dir: PathBuf,
    pub sources: Vec<PathBuf>,
    pub button: crate::core::drop_action::DropMouseButton,
    pub modifiers: crate::core::drop_action::DropModifiers,
    pub position: egui::Pos2,
}

/// Active drop action menu popup state.
#[derive(Clone, Debug)]
pub struct DropMenuState {
    pub position: egui::Pos2,
    pub sources: Vec<PathBuf>,
    pub target_dir: PathBuf,
    pub default_action: crate::core::drop_action::DropActionKind,
}

type DropOp = PendingDrop;

/// True when `child` is `ancestor` itself or lives underneath it.
fn same_or_within(child: &Path, ancestor: &Path) -> bool {
    child.starts_with(ancestor)
}

/// Stable animation key for a directory's drop-target highlight.
fn drop_highlight_id(dir: &Path) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    "mossambi_drop_hl".hash(&mut h);
    dir.hash(&mut h);
    h.finish()
}

/// Small square icon button used for inline controls (filter clear etc.).
fn nav_icon_button(ui: &mut Ui, glyph: IconGlyph, px: f32) -> egui::Response {
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
        None => egui::Button::new("x"),
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
            // PowerShell single-quoted strings double embedded single
            // quotes; a literal `'` in a path would otherwise terminate
            // the quoting and run as injected code (S-1 security fix).
            let ps_path = path.display().to_string().replace('\'', "''");
            let _ = std::process::Command::new("powershell.exe")
                .arg("-NoExit")
                .arg("-Command")
                .arg(format!("Set-Location -LiteralPath '{}'", ps_path))
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

// ===========================================================================
// Shared context menus - Windows 11 Explorer / macOS Finder / GNOME Nautilus
// wording from `file_menu::MenuAction`, glyphs from the active icon pack.
//
// The menu builders are pure: they render entries via `file_menu` and RETURN
// intents. All mutations of ExplorerView state happen in the caller after the
// popup closes - popups never touch `&mut self` directly.
// ===========================================================================

use crate::core::config::{CheckboxMode, MenuStyle};
use crate::core::icon_packs::{IconGlyph, IconPack};
use crate::gui::file_menu as fm;

/// Everything the empty-canvas menu may ask the caller to do.
#[derive(Default, Debug, Clone)]
pub struct CanvasMenuIntents {
    pub new_folder: bool,
    pub new_window: bool,
    pub new_tab: bool,
    pub refresh: bool,
    pub go_up: bool,
    pub select_all: bool,
    pub clear_selection: bool,
    /// Clipboard sources the caller should paste into the current directory.
    pub paste: Option<Vec<PathBuf>>,
    pub terminal: bool,
    pub open_in_file_manager: bool,
    pub properties_dir: bool,
    /// Requested sort column (caller toggles direction when re-clicked).
    pub sort_by: Option<SortColumn>,
    /// Requested view mode (Details/Grid/Columns submenu).
    pub view_mode: Option<ViewMode>,
    /// Invert selection within the current filtered listing.
    pub invert_selection: bool,
}

/// Variants of "copy as path" (Explorer 11's flyout + terminal niceties).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyPathKind {
    /// Absolute path, native separators.
    Full,
    /// File name only.
    Name,
    /// Absolute path wrapped in double quotes (paste-safe for CLIs).
    Quoted,
    /// Absolute path with forward slashes (paste-safe for *nix tools).
    ForwardSlashes,
    /// The containing directory path.
    Parent,
}

/// Everything an item (row/tile) menu may ask the caller to do.
///
/// Multi-select: when the right-clicked item is part of the current
/// selection, `deleted`/`cut`/`copy`/`duplicate`/`pack`/`extract`/
/// `inspect`/`exam_conform`/`transcode`/`print` carry EVERY selected path
/// (Explorer rule); otherwise just the clicked one.
#[derive(Default, Debug, Clone)]
pub struct ItemMenuIntents {
    pub nav_into_dir: Option<PathBuf>,
    pub open_in_new_window: Option<PathBuf>,
    pub open_in_new_tab: Option<PathBuf>,
    pub peek: Option<FileItem>,
    pub open_default: Option<PathBuf>,
    pub open_in_editor: Option<PathBuf>,
    pub open_with: Option<PathBuf>,
    /// Launch directly with an enumerated Open With command line.
    pub open_with_launch: Option<(String, PathBuf)>,
    /// Run a user-defined custom menu entry (Phase 4); the optional path
    /// is `None` for background-menu entries.
    pub custom_item: Option<(crate::core::config::CustomMenuItem, Option<PathBuf>)>,
    pub play: Option<PathBuf>,
    pub print: Option<Vec<PathBuf>>,
    pub transcode: Option<(Vec<PathBuf>, String)>,
    pub exam_conform: Option<Vec<PathBuf>>,
    pub pack: Option<Vec<PathBuf>>,
    pub inspect: Option<Vec<PathBuf>>,
    pub extract: Option<Vec<PathBuf>>,
    pub rename: Option<(PathBuf, String)>,
    pub batch_rename: Option<Vec<PathBuf>>,
    pub properties: Option<FileItem>,
    pub deleted: Option<Vec<PathBuf>>,
    pub cut: Option<Vec<PathBuf>>,
    pub copy: Option<Vec<PathBuf>>,
    pub duplicate: Option<Vec<PathBuf>>,
    pub defender: Option<PathBuf>,
    pub wallpaper: Option<PathBuf>,
    pub rotate: Option<PathBuf>,
    pub shortcut: Option<PathBuf>,
    pub copy_path: Option<(PathBuf, CopyPathKind)>,
    pub send_to: Option<(Vec<PathBuf>, PathBuf, String)>,
    pub terminal_here: Option<PathBuf>,
    pub reveal: Option<PathBuf>,
    /// Set a tag on the affected items: (paths, tag name).
    pub set_tag: Option<(Vec<PathBuf>, String)>,
    /// Clear a tag from the affected items: (paths, tag name).
    pub clear_tag: Option<(Vec<PathBuf>, String)>,
    /// Pin/unpin a folder to the sidebar: (path, pin=true) / (path, pin=false).
    pub pin_sidebar: Option<(PathBuf, bool)>,
}

/// Extension classification shared by both item menus.
struct ItemKind {
    is_media: bool,
    is_printable: bool,
    is_image: bool,
    is_archive: bool,
}

/// Classifies via a path's extension (multi-select-friendly).
fn item_kind_ext(p: &Path) -> ItemKind {
    let ext = p
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_uppercase())
        .unwrap_or_default();
    item_kind(&ext)
}

fn item_kind(extension: &str) -> ItemKind {
    let ext = extension.to_ascii_uppercase();
    ItemKind {
        is_media: matches!(
            ext.as_str(),
            "MP3"
                | "FLAC"
                | "WAV"
                | "AAC"
                | "M4A"
                | "OGG"
                | "OPUS"
                | "MP4"
                | "MKV"
                | "MOV"
                | "WEBM"
                | "AVI"
                | "M4V"
        ),
        is_printable: matches!(
            ext.as_str(),
            "PDF" | "DOC" | "DOCX" | "XLS" | "XLSX" | "PPT" | "PPTX" | "TXT" | "RTF"
        ),
        is_image: matches!(
            ext.as_str(),
            "JPG"
                | "JPEG"
                | "PNG"
                | "WEBP"
                | "GIF"
                | "AVIF"
                | "BMP"
                | "ICO"
                | "TIFF"
                | "HEIC"
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
    style: MenuStyle,
    tint: Color32,
) -> egui::Response {
    menu_ui
        .add_enabled_ui(enabled, |ui| {
            fm::menu_item_glyph(ui, pack, glyph, label, style, tint)
        })
        .inner
}

impl ExplorerView {
    /// Records a transient user-visible status note (auto-fades in render).
    pub fn show_status_note(&mut self, note: Result<String, String>) {
        let text = match note {
            Ok(message) => message,
            // Callers word their own errors (rename, undo, fs ops, drag-out);
            // a fixed "Drag-out failed:" prefix mislabelled all of them.
            Err(error) => error,
        };
        self.status_note = Some((text, Instant::now()));
    }

    /// Paste everything currently staged in Mossambi's inner clipboard into
    /// the current directory (fresh-name resolution, cut => move).
    /// Commits a completed drag: move or copy `paths` into `dest_dir`.
    /// Mirrors `paste_from_clipboard` semantics; logs every op; refreshes the
    /// view. Silently refuses illegal drops (folder into itself/subtree/parent).
    pub fn perform_drop(&mut self, dest_dir: PathBuf, paths: &[PathBuf], copy: bool) {
        if paths.is_empty() || dest_dir.as_os_str().is_empty() {
            return;
        }
        let illegal = paths.iter().any(|p| {
            same_or_within(&dest_dir, p) || p.parent().map(|q| q == dest_dir).unwrap_or(false)
        });
        if illegal {
            log::debug!(
                "Drop rejected: destination {} holds/contains source",
                dest_dir.display()
            );
            return;
        }

        let sources = paths.to_vec();
        if copy {
            log::info!(
                "Dispatching async copy of {} item(s) to {}",
                sources.len(),
                dest_dir.display()
            );
            let _ = self
                .fs_worker_tx
                .send(FsOperation::Copy { sources, dest_dir });
        } else {
            for src in &sources {
                self.selected_paths.remove(src);
            }
            log::info!(
                "Dispatching async move of {} item(s) to {}",
                sources.len(),
                dest_dir.display()
            );
            let _ = self
                .fs_worker_tx
                .send(FsOperation::Move { sources, dest_dir });
        }
        self.selection_anchor = None;
    }

    /// Dispatches a drop event using the pure decision engine.
    pub fn handle_drop(&mut self, pending: PendingDrop) {
        let same_volume =
            crate::core::drop_action::paths_on_same_volume(&pending.sources, &pending.dest_dir);
        let cfg = crate::core::MossambiConfig::load();
        let decision = crate::core::drop_action::decide_drop_action(
            &pending.sources,
            &pending.dest_dir,
            pending.modifiers,
            pending.button,
            same_volume,
            cfg.drop_action_pref,
        );

        match decision {
            crate::core::DropDecision::Immediate(crate::core::DropActionKind::Move) => {
                self.perform_drop(pending.dest_dir, &pending.sources, false);
            }
            crate::core::DropDecision::Immediate(crate::core::DropActionKind::Copy) => {
                self.perform_drop(pending.dest_dir, &pending.sources, true);
            }
            crate::core::DropDecision::Immediate(crate::core::DropActionKind::Link) => {
                self.perform_link(pending.dest_dir, &pending.sources);
            }
            crate::core::DropDecision::ShowMenu { default } => {
                self.drop_menu = Some(DropMenuState {
                    position: pending.position,
                    sources: pending.sources,
                    target_dir: pending.dest_dir,
                    default_action: default,
                });
            }
            crate::core::DropDecision::Reject(reason) => {
                log::debug!("Drop rejected: {}", reason.message());
                if reason != crate::core::DropRejectReason::SameDirectoryMove {
                    self.show_status_note(Err(reason.message().to_string()));
                }
            }
        }
    }

    /// Creates shortcuts (.lnk on Windows, symlinks on Unix) for `sources` in `dest_dir`.
    pub fn perform_link(&mut self, dest_dir: PathBuf, sources: &[PathBuf]) {
        let mut count = 0;
        for src in sources {
            if crate::gui::context_menu::create_shortcut_in(src, &dest_dir) {
                count += 1;
            }
        }
        if count > 0 {
            self.show_status_note(Ok(format!("Created shortcut for {} item(s)", count)));
            let dir = self.current_dir.clone();
            self.navigate_to(dir);
        } else {
            self.show_status_note(Err("Failed to create shortcut".to_string()));
        }
    }

    /// Extracts dropped archives into `dest_dir`.
    pub fn perform_extract_drop(&mut self, dest_dir: PathBuf, sources: &[PathBuf]) {
        let mut count = 0;
        for src in sources {
            if crate::archive::unpack_archive(src, &dest_dir).is_ok() {
                count += 1;
            }
        }
        if count > 0 {
            self.show_status_note(Ok(format!("Extracted {} archive(s)", count)));
            let dir = self.current_dir.clone();
            self.navigate_to(dir);
        }
    }

    /// Renders the Finder/Windows-style drop action menu popup at the drop position.
    fn render_drop_menu(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        on_transcode_target: &mut impl FnMut(PathBuf, String),
    ) {
        let Some(menu) = self.drop_menu.clone() else {
            return;
        };

        let screen = ui.ctx().screen_rect();
        let menu_w = 260.0;
        let x = menu
            .position
            .x
            .min(screen.right() - menu_w - 8.0)
            .max(screen.left() + 8.0);
        let y = menu
            .position
            .y
            .min(screen.bottom() - 260.0)
            .max(screen.top() + 8.0);

        let cfg = crate::core::MossambiConfig::load();
        let pack = crate::core::icon_packs::IconPack::from_config_id(&cfg.icon_pack);
        let style = cfg.menu_style;
        let tint = palette.accent;

        let mut action_to_execute: Option<crate::core::DropActionKind> = None;
        let mut extract_to_execute = false;
        let mut close_menu = false;

        // Keyboard navigation: Enter = default, Esc = cancel
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            close_menu = true;
        } else if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            action_to_execute = Some(menu.default_action);
        }

        let area_id = egui::Id::new("mossambi_drop_action_menu");
        let area_resp = egui::Area::new(area_id)
            .fixed_pos(egui::pos2(x, y))
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                let frame = egui::Frame::menu(ui.style())
                    .fill(palette.card_bg)
                    .stroke(Stroke::new(1.0_f32, palette.border))
                    .rounding(Rounding::same(cfg.corner_style.radius_sm()))
                    .inner_margin(egui::Margin::symmetric(6.0, 6.0));

                frame.show(ui, |ui| {
                    ui.set_width(menu_w);

                    // Header: count + first filename
                    let first_name = menu
                        .sources
                        .first()
                        .and_then(|p| p.file_name())
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "item".to_string());
                    let header_text = if menu.sources.len() == 1 {
                        format!("1 item: {}", first_name)
                    } else {
                        format!("{} items ({})", menu.sources.len(), first_name)
                    };
                    fm::menu_header(ui, &header_text, tint);
                    ui.separator();

                    // 1. Move here
                    let move_is_default = menu.default_action == crate::core::DropActionKind::Move;
                    if fm::menu_item_labeled_bold(
                        ui,
                        pack,
                        fm::MenuAction::Cut,
                        "Move here",
                        style,
                        tint,
                        move_is_default,
                    )
                    .clicked()
                    {
                        action_to_execute = Some(crate::core::DropActionKind::Move);
                    }

                    // 2. Copy here
                    let copy_is_default = menu.default_action == crate::core::DropActionKind::Copy;
                    if fm::menu_item_labeled_bold(
                        ui,
                        pack,
                        fm::MenuAction::Copy,
                        "Copy here",
                        style,
                        tint,
                        copy_is_default,
                    )
                    .clicked()
                    {
                        action_to_execute = Some(crate::core::DropActionKind::Copy);
                    }

                    // 3. Create shortcut here
                    let link_is_default = menu.default_action == crate::core::DropActionKind::Link;
                    if fm::menu_item_labeled_bold(
                        ui,
                        pack,
                        fm::MenuAction::Shortcut,
                        "Create shortcut here",
                        style,
                        tint,
                        link_is_default,
                    )
                    .clicked()
                    {
                        action_to_execute = Some(crate::core::DropActionKind::Link);
                    }

                    // 4. Convert here > (only if sources are convertible)
                    let mut convertible_formats: Vec<String> = Vec::new();
                    for src in &menu.sources {
                        let sectors =
                            crate::core::registry::UniversalRegistry::get_sectors_for_file(src);
                        for fmt in sectors {
                            if fmt != "MORE" && fmt != "ALL" && fmt != "EXTRACT" {
                                let fmt_str = fmt.to_string();
                                if !convertible_formats.contains(&fmt_str) {
                                    convertible_formats.push(fmt_str);
                                }
                            }
                        }
                    }

                    if !convertible_formats.is_empty() {
                        fm::submenu(
                            ui,
                            "Convert here",
                            pack,
                            style,
                            tint,
                            fm::MenuAction::Transcode,
                            |sub_ui| {
                                sub_ui.set_min_width(200.0);
                                for fmt in &convertible_formats {
                                    let label = format!("Convert to {fmt}");
                                    if fm::menu_item_labeled(
                                        sub_ui,
                                        pack,
                                        fm::MenuAction::Transcode,
                                        &label,
                                        style,
                                        tint,
                                    )
                                    .clicked()
                                    {
                                        for src in &menu.sources {
                                            on_transcode_target(src.clone(), fmt.clone());
                                        }
                                        close_menu = true;
                                        sub_ui.close_menu();
                                    }
                                }
                            },
                        );
                    }

                    // 5. Extract here (only for archives)
                    let has_archive = menu.sources.iter().any(|p| {
                        p.extension()
                            .and_then(|e| e.to_str())
                            .and_then(crate::archive::ArchiveType::from_extension)
                            .is_some()
                    });
                    if has_archive
                        && fm::menu_item_labeled_bold(
                            ui,
                            pack,
                            fm::MenuAction::Extract,
                            "Extract here",
                            style,
                            tint,
                            false,
                        )
                        .clicked()
                    {
                        extract_to_execute = true;
                    }

                    ui.separator();

                    // 6. Cancel
                    if fm::menu_item_glyph(
                        ui,
                        pack,
                        crate::core::icon_packs::IconGlyph::ActionClose,
                        "Cancel",
                        style,
                        tint,
                    )
                    .clicked()
                    {
                        close_menu = true;
                    }
                });
            });

        // Dismiss on click outside
        if ui.input(|i| i.pointer.any_pressed()) {
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                if !area_resp.response.rect.contains(pos) {
                    close_menu = true;
                }
            }
        }

        if let Some(action) = action_to_execute {
            self.drop_menu = None;
            match action {
                crate::core::DropActionKind::Move => {
                    self.perform_drop(menu.target_dir, &menu.sources, false);
                }
                crate::core::DropActionKind::Copy => {
                    self.perform_drop(menu.target_dir, &menu.sources, true);
                }
                crate::core::DropActionKind::Link => {
                    self.perform_link(menu.target_dir, &menu.sources);
                }
            }
        } else if extract_to_execute {
            self.drop_menu = None;
            self.perform_extract_drop(menu.target_dir, &menu.sources);
        } else if close_menu {
            self.drop_menu = None;
        }
    }

    /// Applies one item menu's intents. All filesystem side effects are
    /// logged; conversions/inspections route through the app-level callbacks.
    /// Trash-aware delete with a logged permanent-delete fallback.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_item_intent(
        &mut self,
        ui: &mut Ui,
        intent: ItemMenuIntents,
        on_transcode_target: &mut impl FnMut(PathBuf, String),
        on_exam_conform: &mut impl FnMut(PathBuf),
        on_pack_archive: &mut impl FnMut(Vec<PathBuf>),
        on_inspect_file: &mut impl FnMut(PathBuf),
    ) {
        use crate::gui::context_menu as cm;
        use CopyPathKind::*;

        if let Some(dir) = intent.nav_into_dir {
            self.navigate_to(dir);
        }
        if let Some(dir) = intent.open_in_new_window {
            spawn_new_explorer_window(Some(&dir));
        }
        if let Some(dir) = intent.open_in_new_tab {
            self.add_tab(dir);
        }
        if let Some(file) = intent.peek {
            self.peek_item = Some(file);
            self.peek_texture = None;
            self.peek_text_cache = None;
            self.peek_hash_cache = None;
        }
        if let Some(p) = intent.open_default {
            log::debug!("Open (default app): {}", p.display());
            open_in_default_app(&p);
        }
        if let Some(p) = intent.open_in_editor {
            log::debug!("Open in editor: {}", p.display());
            open_in_default_editor(&p);
        }
        if let Some(p) = intent.open_with {
            if !cm::open_with_choose(&p) {
                log::warn!("Open With chooser unavailable for {}", p.display());
            }
        }
        if let Some((cmd, p)) = intent.open_with_launch {
            if !cm::launch_open_with_cmd(&cmd, &p) {
                log::warn!("Open With launch failed for {}", p.display());
            }
        }
        if let Some((item, target)) = intent.custom_item {
            if !cm::run_custom_item(&item, target.as_deref()) {
                log::warn!("Custom menu item '{}' failed to run", item.label);
            }
        }
        if let Some(p) = intent.play {
            if !cm::play_file(&p) {
                log::warn!("Play verb unavailable for {}", p.display());
            }
        }
        if let Some(paths) = intent.print {
            for p in &paths {
                if !cm::print_file(p) {
                    log::warn!("Print verb unavailable for {}", p.display());
                }
            }
        }
        if let Some((paths, fmt)) = intent.transcode {
            if paths.len() > 1 {
                log::info!("Transcode batch: {} file(s) -> {}", paths.len(), fmt);
            }
            for p in paths {
                on_transcode_target(p, fmt.clone());
            }
        }
        if let Some(paths) = intent.exam_conform {
            for p in paths {
                on_exam_conform(p);
            }
        }
        if let Some(paths) = intent.pack {
            on_pack_archive(paths);
        }
        if let Some(paths) = intent.inspect {
            for p in paths {
                on_inspect_file(p);
            }
        }
        if let Some(paths) = intent.extract {
            for path in paths {
                if let Some(parent) = path.parent() {
                    let parent = parent.to_path_buf();
                    log_result(
                        "Extract",
                        &path,
                        crate::archive::unpack_archive(&path, &parent),
                    );
                }
            }
            self.navigate_to(self.current_dir.clone());
        }
        if let Some((path, name)) = intent.rename {
            self.renaming_path = Some((path, name));
        }
        if let Some(targets) = intent.batch_rename {
            self.batch_rename.target_paths = targets;
            self.batch_rename.is_open = true;
        }
        if let Some(item) = intent.properties {
            self.properties_item = Some(item);
        }
        if let Some(paths) = intent.deleted {
            let paths: Vec<PathBuf> = paths
                .into_iter()
                .filter(|p| !crate::core::is_protected_os_file(p))
                .collect();
            if !paths.is_empty() {
                if cm::delete_to_trash(&paths) {
                    log::info!("Moved {} item(s) to Recycle Bin/Trash", paths.len());
                } else {
                    for p in &paths {
                        log_result("Delete (permanent fallback)", p, remove_entry_recursive(p));
                    }
                }
                for p in &paths {
                    self.selected_paths.remove(p);
                }
                self.selection_anchor = None;
                self.navigate_to(self.current_dir.clone());
            }
        }
        if let Some(paths) = intent.cut {
            self.clipboard_paths = paths.clone();
            self.clipboard_is_cut = true;
            if !cm::set_clipboard_files_with_effect(&paths, true) {
                log::warn!("OS clipboard (cut) sync failed for {} item(s)", paths.len());
            }
        }
        if let Some(paths) = intent.copy {
            self.clipboard_paths = paths.clone();
            self.clipboard_is_cut = false;
            if !cm::set_clipboard_files_with_effect(&paths, false) {
                log::warn!(
                    "OS clipboard (copy) sync failed for {} item(s)",
                    paths.len()
                );
            }
        }
        if let Some(paths) = intent.duplicate {
            for src in &paths {
                let name = src
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let parent = src.parent().unwrap_or(src);
                let copy_name = match src.extension() {
                    Some(ext) => format!(
                        "{} copy.{}",
                        src.file_stem()
                            .map(|s| s.to_string_lossy())
                            .unwrap_or_default(),
                        ext.to_string_lossy()
                    ),
                    None => format!("{name} copy"),
                };
                let target = resolve_unique_paste_path(&parent.join(copy_name));
                log_result(
                    "Duplicate",
                    src,
                    crate::core::fs_utils::copy_entry_recursive(src, &target).map(|_| ()),
                );
            }
            self.navigate_to(self.current_dir.clone());
        }
        if let Some(p) = intent.defender {
            if !cm::scan_with_defender(&p) {
                log::warn!("Defender scan unavailable for {}", p.display());
            }
        }
        if let Some(p) = intent.wallpaper {
            if !cm::set_desktop_wallpaper(&p) {
                log::warn!("Set wallpaper unavailable for {}", p.display());
            } else {
                log::info!("Desktop wallpaper set: {}", p.display());
            }
        }
        if let Some(p) = intent.rotate {
            if !cm::rotate_image_90(&p) {
                log::warn!("Rotate 90 deg failed for {}", p.display());
            } else {
                log::info!("Rotated 90 deg: {}", p.display());
                self.navigate_to(self.current_dir.clone());
            }
        }
        if let Some(p) = intent.shortcut {
            if !cm::create_shortcut(&p) {
                log::warn!("Shortcut creation unavailable for {}", p.display());
            }
        }
        if let Some((p, kind)) = intent.copy_path {
            let text = match kind {
                Full => p.to_string_lossy().to_string(),
                Name => p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                Quoted => format!("\"{}\"", p.to_string_lossy()),
                ForwardSlashes => p.to_string_lossy().replace('\\', "/"),
                Parent => p
                    .parent()
                    .map(|d| d.to_string_lossy().to_string())
                    .unwrap_or_default(),
            };
            let verb = match kind {
                Full => "path",
                Name => "name",
                Quoted => "quoted path",
                ForwardSlashes => "forward-slash path",
                Parent => "parent path",
            };
            ui.output_mut(|o| o.copied_text = text.clone());
            log::debug!("Copied {verb}: {text}");
        }
        if let Some((paths, dest, label)) = intent.send_to {
            log::info!("Send to '{label}': {} item(s)", paths.len());
            if !cm::send_to(&paths, &dest) {
                log::warn!("Send to '{label}' failed for {} item(s)", paths.len());
            }
        }
        if let Some(dir) = intent.terminal_here {
            log::debug!("Terminal at {}", dir.display());
            open_terminal_in_dir(&dir);
        }
        if let Some(p) = intent.reveal {
            reveal_in_file_manager(&p);
        }

        // -- Tags --
        //
        // Every tag change is journaled with its before/after state so the
        // undo stack can reverse it.
        if let Some((paths, tag)) = intent.set_tag {
            for p in &paths {
                let before = crate::core::tags::get_tags(p);
                match crate::core::tags::set_tag(p, &tag) {
                    Ok(()) => {
                        let after = crate::core::tags::get_tags(p);
                        crate::core::action_journal::record_tag_set(p.clone(), before, after);
                        log::info!("Tagged {} as {}", p.display(), tag);
                    }
                    Err(e) => log::warn!("Tag failed for {}: {e}", p.display()),
                }
            }
            self.navigate_to(self.current_dir.clone());
        }
        if let Some((paths, tag)) = intent.clear_tag {
            for p in &paths {
                let before = crate::core::tags::get_tags(p);
                match crate::core::tags::clear_tag(p, &tag) {
                    Ok(()) => {
                        let after = crate::core::tags::get_tags(p);
                        crate::core::action_journal::record_tag_set(p.clone(), before, after);
                        log::info!("Removed tag {tag} from {}", p.display());
                    }
                    Err(e) => log::warn!("Tag removal failed for {}: {e}", p.display()),
                }
            }
            self.navigate_to(self.current_dir.clone());
        }

        // -- Pin / Unpin to the sidebar --
        if let Some((path, pin)) = intent.pin_sidebar {
            let mut cfg = crate::core::MossambiConfig::load();
            if pin {
                crate::core::pin_add(&mut cfg.pinned_custom, &path);
            } else {
                crate::core::pin_remove(&mut cfg.pinned_custom, &path);
            }
            if let Err(e) = cfg.save() {
                log::warn!("Could not persist sidebar pins: {e}");
            } else {
                log::info!(
                    "{} {} to the sidebar",
                    if pin { "Pinned" } else { "Unpinned" },
                    path.display()
                );
            }
        }
    }

    pub fn paste_from_clipboard(&mut self) {
        let sources: Vec<PathBuf> = self.clipboard_paths.clone();
        if sources.is_empty() {
            return;
        }
        let cut = self.clipboard_is_cut;
        let dest_dir = self.current_dir.clone();
        if cut {
            let _ = self
                .fs_worker_tx
                .send(FsOperation::Move { sources, dest_dir });
            self.clipboard_paths.clear();
            self.clipboard_is_cut = false;
        } else {
            let _ = self
                .fs_worker_tx
                .send(FsOperation::Copy { sources, dest_dir });
        }
    }

    pub fn copy_selected_to_clipboard(&mut self) {
        if !self.selected_paths.is_empty() {
            self.clipboard_paths = self.selected_paths.iter().cloned().collect();
            self.clipboard_is_cut = false;
        }
    }

    pub fn cut_selected_to_clipboard(&mut self) {
        if !self.selected_paths.is_empty() {
            self.clipboard_paths = self.selected_paths.iter().cloned().collect();
            self.clipboard_is_cut = true;
        }
    }

    pub fn delete_selected(&mut self) {
        let paths: Vec<PathBuf> = self
            .selected_paths
            .iter()
            .filter(|&p| !crate::core::is_protected_os_file(p))
            .cloned()
            .collect();
        if !paths.is_empty() {
            let _ = crate::core::action_journal::stage_delete(&paths);
            self.selected_paths.clear();
            self.navigate_to(self.current_dir.clone());
        }
    }

    pub fn rename_selected(&mut self) {
        if let Some(path) = self.selected_paths.iter().next().cloned() {
            if let Some(name) = path.file_name().map(|n| n.to_string_lossy().to_string()) {
                self.renaming_path = Some((path, name));
            }
        }
    }

    pub fn copy_current_path_to_clipboard(&mut self) {
        crate::gui::context_menu::set_clipboard_files(std::slice::from_ref(&self.current_dir));
    }

    /// Shared empty-canvas context menu for the Details and Grid canvases.
    #[allow(clippy::too_many_arguments)]
    pub fn run_canvas_context_menu(
        menu_ui: &mut Ui,
        pack: IconPack,
        style: MenuStyle,
        tint: Color32,
        has_clipboard: bool,
        has_entries: bool,
        current_sort: SortColumn,
        sort_desc: bool,
        current_view: ViewMode,
    ) -> CanvasMenuIntents {
        let mut intent = CanvasMenuIntents::default();
        fm::menu_header(menu_ui, "Explorer", tint);
        menu_ui.separator();

        if menu_row(menu_ui, true, pack, fm::MenuAction::NewFolder, style, tint).clicked() {
            intent.new_folder = true;
            menu_ui.close_menu();
        }
        if menu_row(menu_ui, true, pack, fm::MenuAction::NewWindow, style, tint).clicked() {
            intent.new_window = true;
            menu_ui.close_menu();
        }
        if menu_row(menu_ui, true, pack, fm::MenuAction::NewTab, style, tint).clicked() {
            intent.new_tab = true;
            menu_ui.close_menu();
        }
        if menu_row(menu_ui, true, pack, fm::MenuAction::Refresh, style, tint).clicked() {
            intent.refresh = true;
            menu_ui.close_menu();
        }
        if menu_row_glyph(
            menu_ui,
            true,
            pack,
            IconGlyph::ActionUp,
            "Go to Parent Folder",
            style,
            tint,
        )
        .clicked()
        {
            intent.go_up = true;
            menu_ui.close_menu();
        }

        menu_ui.separator();
        // -- Sort by submenu (re-picking the active column reverses order) --
        fm::submenu(
            menu_ui,
            "Sort by",
            pack,
            style,
            tint,
            fm::MenuAction::SortBy,
            |sub_ui| {
                sub_ui.set_min_width(170.0);
                for (col, label) in [
                    (SortColumn::Name, "Name"),
                    (SortColumn::Extension, "Kind"),
                    (SortColumn::Size, "Size"),
                    (SortColumn::Date, "Date Modified"),
                    (SortColumn::Permissions, "Permissions"),
                ] {
                    let mark = if current_sort == col {
                        if sort_desc {
                            " (desc)"
                        } else {
                            " (asc)"
                        }
                    } else {
                        ""
                    };
                    let item_text = format!("{label}{mark}");
                    if fm::menu_item_glyph(
                        sub_ui,
                        pack,
                        IconGlyph::ActionSort,
                        &item_text,
                        style,
                        tint,
                    )
                    .clicked()
                    {
                        intent.sort_by = Some(col);
                        sub_ui.close_menu();
                    }
                }
            },
        );
        // -- View submenu (Details / Grid / Columns) --
        fm::submenu(
            menu_ui,
            "View",
            pack,
            style,
            tint,
            fm::MenuAction::ViewMode,
            |sub_ui| {
                sub_ui.set_min_width(170.0);
                for (vm, label) in [
                    (ViewMode::Details, "Details / List"),
                    (ViewMode::Grid, "Icons / Grid"),
                    (ViewMode::Columns, "Miller Columns"),
                ] {
                    let mark = if current_view == vm { " *" } else { "" };
                    let item_text = format!("{label}{mark}");
                    if fm::menu_item_glyph(
                        sub_ui,
                        pack,
                        IconGlyph::ActionView,
                        &item_text,
                        style,
                        tint,
                    )
                    .clicked()
                    {
                        intent.view_mode = Some(vm);
                        sub_ui.close_menu();
                    }
                }
            },
        );

        // -- Custom user entries (Phase 4 menu customization) --
        // Background-scope config entries; no target path is passed.
        {
            let cfg = crate::core::MossambiConfig::load();
            let customs: Vec<_> = cfg
                .context_menu_items
                .iter()
                .filter(|c| {
                    c.show_in == crate::core::config::CustomMenuScope::All
                        || c.show_in == crate::core::config::CustomMenuScope::Background
                })
                .collect();
            if !customs.is_empty() {
                menu_ui.separator();
                for entry in customs {
                    if fm::menu_item_custom(menu_ui, &entry.label).clicked() {
                        let item = entry.clone();
                        menu_ui.close_menu();
                        crate::gui::context_menu::run_custom_item(&item, None);
                    }
                }
            }
        }

        menu_ui.separator();
        if menu_row(
            menu_ui,
            has_entries,
            pack,
            fm::MenuAction::SelectAll,
            style,
            tint,
        )
        .clicked()
        {
            intent.select_all = true;
            menu_ui.close_menu();
        }
        if menu_row_glyph(
            menu_ui,
            has_entries,
            pack,
            IconGlyph::ActionMore,
            "Invert Selection",
            style,
            tint,
        )
        .clicked()
        {
            intent.invert_selection = true;
            menu_ui.close_menu();
        }
        if menu_row_glyph(
            menu_ui,
            true,
            pack,
            IconGlyph::ActionClose,
            "Clear Selection",
            style,
            tint,
        )
        .clicked()
        {
            intent.clear_selection = true;
            menu_ui.close_menu();
        }
        if menu_row(
            menu_ui,
            has_clipboard,
            pack,
            fm::MenuAction::Paste,
            style,
            tint,
        )
        .clicked()
        {
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
    ///
    /// `targets` is the effective selection: the clicked path plus every
    /// co-selected path when the click landed inside the selection (Explorer
    /// rule). File-only groups (print/convert/conform/inspect) automatically
    /// restrict to files; hirez verbs stay on the clicked item only.
    #[allow(clippy::too_many_lines)]
    pub fn run_item_context_menu(
        menu_ui: &mut Ui,
        pack: IconPack,
        style: MenuStyle,
        tint: Color32,
        item: &FileItem,
        targets: &[PathBuf],
    ) -> ItemMenuIntents {
        let mut intent = ItemMenuIntents::default();
        let is_dir = item.is_dir;
        let path: PathBuf = item.path.clone();

        // Multi-select bookkeeping (Explorer rule).
        let multi = targets.len() > 1;
        let file_targets: Vec<PathBuf> = targets.iter().filter(|p| p.is_file()).cloned().collect();
        let archive_targets: Vec<PathBuf> = targets
            .iter()
            .filter(|p| {
                p.extension()
                    .map(|e| item_kind(&e.to_string_lossy().to_ascii_uppercase()).is_archive)
                    .unwrap_or(false)
            })
            .cloned()
            .collect();

        let header = if multi {
            pluralize_items(targets.len())
        } else {
            item.name.clone()
        };
        fm::menu_header(menu_ui, &header, tint);
        menu_ui.separator();

        // -- Open group --
        if is_dir {
            if menu_row(menu_ui, true, pack, fm::MenuAction::OpenFolder, style, tint).clicked() {
                intent.nav_into_dir = Some(path.clone());
                menu_ui.close_menu();
            }
            if menu_row(menu_ui, true, pack, fm::MenuAction::NewWindow, style, tint).clicked() {
                intent.open_in_new_window = Some(path.clone());
                menu_ui.close_menu();
            }
            if menu_row(menu_ui, true, pack, fm::MenuAction::NewTab, style, tint).clicked() {
                intent.open_in_new_tab = Some(path.clone());
                menu_ui.close_menu();
            }
        } else {
            if menu_row(menu_ui, true, pack, fm::MenuAction::OpenFile, style, tint).clicked() {
                intent.open_default = Some(path.clone());
                menu_ui.close_menu();
            }
            if menu_row(menu_ui, !multi, pack, fm::MenuAction::Edit, style, tint).clicked() {
                intent.open_in_editor = Some(path.clone());
                menu_ui.close_menu();
            }
            let kind = item_kind(&item.extension);
            if kind.is_media
                && menu_row(menu_ui, !multi, pack, fm::MenuAction::Play, style, tint).clicked()
            {
                intent.play = Some(path.clone());
                menu_ui.close_menu();
            }
            if !file_targets.is_empty()
                && file_targets.iter().any(|p| item_kind_ext(p).is_printable)
                && menu_row(menu_ui, true, pack, fm::MenuAction::Print, style, tint).clicked()
            {
                intent.print = Some(
                    file_targets
                        .iter()
                        .filter(|p| item_kind_ext(p).is_printable)
                        .cloned()
                        .collect(),
                );
                menu_ui.close_menu();
            }
            if menu_row(
                menu_ui,
                !multi,
                pack,
                fm::MenuAction::QuickLook,
                style,
                tint,
            )
            .clicked()
            {
                intent.peek = Some(item.clone());
                menu_ui.close_menu();
            }
        }

        menu_ui.separator();

        // -- Media/image verbs (single item, non-dir) --
        if !is_dir && !multi {
            let kind = item_kind(&item.extension);
            if kind.is_image
                && menu_row(menu_ui, true, pack, fm::MenuAction::Wallpaper, style, tint).clicked()
            {
                intent.wallpaper = Some(path.clone());
                menu_ui.close_menu();
            }
            if kind.is_image
                && menu_row(menu_ui, true, pack, fm::MenuAction::Rotate, style, tint).clicked()
            {
                intent.rotate = Some(path.clone());
                menu_ui.close_menu();
            }
            // Type-aware Convert submenu - the SAME engine the radial wheel
            // uses (UniversalRegistry::get_sectors_for_file): every converter
            // the Studio's left column offers, scoped to this file's type, so
            // irrelevant formats never appear.
            let sectors = crate::core::registry::UniversalRegistry::get_sectors_for_file(&path);
            let in_ext = item.extension.to_ascii_uppercase();
            fm::submenu(
                menu_ui,
                "Convert",
                pack,
                style,
                tint,
                fm::MenuAction::Transcode,
                |sub_ui| {
                    sub_ui.set_min_width(210.0);
                    for fmt in &sectors {
                        match *fmt {
                            // MORE folds into the full list below; ALL gets its
                            // own explicit row after the separator.
                            "MORE" | "ALL" => continue,
                            // Nothing to extract FROM a directory entry itself.
                            "EXTRACT" if is_dir => continue,
                            label => {
                                let text = if label.eq_ignore_ascii_case(&in_ext) {
                                    format!("Rebuild {label} (zero-loss)")
                                } else {
                                    format!("Convert to {label}")
                                };
                                if fm::menu_item_labeled(
                                    sub_ui,
                                    pack,
                                    fm::MenuAction::Transcode,
                                    &text,
                                    style,
                                    tint,
                                )
                                .clicked()
                                {
                                    intent.transcode = Some((targets.to_vec(), label.to_string()));
                                    sub_ui.close_menu();
                                }
                            }
                        }
                    }
                    sub_ui.separator();
                    if fm::menu_item_labeled(
                        sub_ui,
                        pack,
                        fm::MenuAction::Transcode,
                        "All formats (batch)",
                        style,
                        tint,
                    )
                    .clicked()
                    {
                        intent.transcode = Some((targets.to_vec(), "ALL".to_string()));
                        sub_ui.close_menu();
                    }
                },
            );
        }
        if !archive_targets.is_empty()
            && menu_row(menu_ui, true, pack, fm::MenuAction::Extract, style, tint).clicked()
        {
            intent.extract = Some(archive_targets.clone());
            menu_ui.close_menu();
        }
        if menu_row(menu_ui, true, pack, fm::MenuAction::PackZip, style, tint).clicked() {
            intent.pack = Some(targets.to_vec());
            menu_ui.close_menu();
        }
        if !file_targets.is_empty() {
            if menu_row(menu_ui, true, pack, fm::MenuAction::Conform, style, tint).clicked() {
                intent.exam_conform = Some(file_targets.clone());
                menu_ui.close_menu();
            }
            if menu_row(menu_ui, true, pack, fm::MenuAction::Inspect, style, tint).clicked() {
                intent.inspect = Some(file_targets.clone());
                menu_ui.close_menu();
            }
            if !multi {
                let candidates = crate::gui::context_menu::open_with_candidates(&path);
                fm::submenu(
                    menu_ui,
                    fm::MenuAction::OpenWith.label(style),
                    pack,
                    style,
                    tint,
                    fm::MenuAction::OpenWith,
                    |sub_ui| {
                        sub_ui.set_min_width(170.0);
                        for (label, command) in candidates.iter() {
                            if fm::menu_item_labeled(
                                sub_ui,
                                pack,
                                fm::MenuAction::OpenWith,
                                label,
                                style,
                                tint,
                            )
                            .clicked()
                            {
                                intent.open_with_launch = Some((command.clone(), path.clone()));
                                sub_ui.close_menu();
                            }
                        }
                        if !candidates.is_empty() {
                            sub_ui.separator();
                        }
                        if fm::menu_item_labeled(
                            sub_ui,
                            pack,
                            fm::MenuAction::OpenWith,
                            "Choose another app...",
                            style,
                            tint,
                        )
                        .clicked()
                        {
                            intent.open_with = Some(path.clone());
                            sub_ui.close_menu();
                        }
                    },
                );
            }
        }
        if menu_row(menu_ui, true, pack, fm::MenuAction::Duplicate, style, tint).clicked() {
            intent.duplicate = Some(targets.to_vec());
            menu_ui.close_menu();
        }

        menu_ui.separator();
        // -- Send to (Windows SendTo folder entries) --
        let send_dests = crate::gui::context_menu::send_to_targets();
        if !send_dests.is_empty() {
            fm::submenu(
                menu_ui,
                fm::MenuAction::SendTo.label(style),
                pack,
                style,
                tint,
                fm::MenuAction::SendTo,
                |sub_ui| {
                    sub_ui.set_min_width(170.0);
                    for (label, dest) in send_dests.iter().take(12) {
                        if fm::menu_item_glyph(
                            sub_ui,
                            pack,
                            IconGlyph::ActionShortcut,
                            label,
                            style,
                            tint,
                        )
                        .clicked()
                        {
                            intent.send_to = Some((targets.to_vec(), dest.clone(), label.clone()));
                            sub_ui.close_menu();
                        }
                    }
                },
            );
        }

        menu_ui.separator();

        // -- Shell helpers --
        fm::submenu(
            menu_ui,
            "Copy as path",
            pack,
            style,
            tint,
            fm::MenuAction::CopyPath,
            |sub_ui| {
                sub_ui.set_min_width(190.0);
                for (kind, label) in [
                    (CopyPathKind::Full, "Native separators"),
                    (CopyPathKind::Name, "Name only"),
                    (CopyPathKind::Quoted, "Quoted (\"...\")"),
                    (CopyPathKind::ForwardSlashes, "Forward slashes (/)"),
                    (CopyPathKind::Parent, "Containing folder"),
                ] {
                    if fm::menu_item_glyph(sub_ui, pack, IconGlyph::ActionPath, label, style, tint)
                        .clicked()
                    {
                        intent.copy_path = Some((path.clone(), kind));
                        sub_ui.close_menu();
                    }
                }
            },
        );
        if menu_row(menu_ui, true, pack, fm::MenuAction::Reveal, style, tint).clicked() {
            intent.reveal = Some(path.clone());
            menu_ui.close_menu();
        }
        if menu_row(menu_ui, true, pack, fm::MenuAction::Terminal, style, tint).clicked() {
            let dir_target = if is_dir {
                path.clone()
            } else {
                path.parent().unwrap_or(&path).to_path_buf()
            };
            intent.terminal_here = Some(dir_target);
            menu_ui.close_menu();
        }
        if cfg!(windows)
            && menu_row(menu_ui, true, pack, fm::MenuAction::Shortcut, style, tint).clicked()
        {
            intent.shortcut = Some(path.clone());
            menu_ui.close_menu();
        }

        // -- Custom user entries (Phase 4 menu customization) --
        {
            let wanted = if is_dir {
                crate::core::config::CustomMenuScope::Folders
            } else {
                crate::core::config::CustomMenuScope::Files
            };
            let cfg = crate::core::MossambiConfig::load();
            let customs: Vec<_> = cfg
                .context_menu_items
                .iter()
                .filter(|c| {
                    c.show_in == wanted || c.show_in == crate::core::config::CustomMenuScope::All
                })
                .collect();
            if !customs.is_empty() {
                menu_ui.separator();
                for entry in customs {
                    if fm::menu_item_custom(menu_ui, &entry.label).clicked() {
                        intent.custom_item = Some((entry.clone(), Some(path.clone())));
                        menu_ui.close_menu();
                    }
                }
            }
        }

        // -- Clipboard group --
        menu_ui.separator();
        if menu_row(menu_ui, true, pack, fm::MenuAction::Cut, style, tint).clicked() {
            intent.cut = Some(targets.to_vec());
            menu_ui.close_menu();
        }
        if menu_row(menu_ui, true, pack, fm::MenuAction::Copy, style, tint).clicked() {
            intent.copy = Some(targets.to_vec());
            menu_ui.close_menu();
        }

        // -- Tags submenu (Finder-style coloured tag assignment) --
        menu_ui.separator();
        {
            // Current tags of the clicked item drive the checkbox state.
            let current_tags = crate::core::tags::read_tags_cache(&path);
            fm::submenu(
                menu_ui,
                fm::MenuAction::Tags.label(style),
                pack,
                style,
                tint,
                fm::MenuAction::Tags,
                |sub_ui| {
                    sub_ui.set_min_width(190.0);
                    for tag_name in crate::core::tags::TAG_PRESETS {
                        let is_tagged = current_tags.iter().any(|t| t == tag_name);
                        let label = if is_tagged {
                            format!("Remove {tag_name}")
                        } else {
                            format!("Add {tag_name}")
                        };
                        if fm::menu_item_labeled(
                            sub_ui,
                            pack,
                            fm::MenuAction::Tags,
                            &label,
                            style,
                            tint,
                        )
                        .clicked()
                        {
                            if is_tagged {
                                intent.clear_tag = Some((targets.to_vec(), tag_name.to_string()));
                            } else {
                                intent.set_tag = Some((targets.to_vec(), tag_name.to_string()));
                            }
                            sub_ui.close_menu();
                        }
                    }
                },
            );
        }

        // -- Pin to Sidebar (folders only) --
        if is_dir {
            let is_pinned = {
                let cfg = crate::core::MossambiConfig::load();
                crate::core::pin_contains(&cfg.pinned_custom, &path)
            };
            let label = if is_pinned {
                "Unpin from Sidebar"
            } else {
                fm::MenuAction::PinSidebar.label(style)
            };
            if fm::menu_item_labeled(
                menu_ui,
                pack,
                fm::MenuAction::PinSidebar,
                label,
                style,
                tint,
            )
            .clicked()
            {
                intent.pin_sidebar = Some((path.clone(), !is_pinned));
                menu_ui.close_menu();
            }
        }

        // -- Danger group --
        menu_ui.separator();
        if multi {
            if menu_ui
                .button(format!(
                    "Batch Rename ({})...",
                    pluralize_items(targets.len())
                ))
                .clicked()
            {
                intent.batch_rename = Some(targets.to_vec());
                menu_ui.close_menu();
            }
        } else if menu_row(menu_ui, true, pack, fm::MenuAction::Rename, style, tint).clicked() {
            if let Some(name) = path.file_name() {
                intent.rename = Some((path.clone(), name.to_string_lossy().to_string()));
            }
            menu_ui.close_menu();
        }
        if menu_row(menu_ui, true, pack, fm::MenuAction::Delete, style, tint).clicked() {
            intent.deleted = Some(targets.to_vec());
            menu_ui.close_menu();
        }
        if crate::gui::context_menu::win_defender_available()
            && menu_row(menu_ui, true, pack, fm::MenuAction::Defender, style, tint).clicked()
        {
            intent.defender = Some(path.clone());
            menu_ui.close_menu();
        }

        menu_ui.separator();
        if menu_row(
            menu_ui,
            !multi,
            pack,
            fm::MenuAction::Properties,
            style,
            tint,
        )
        .clicked()
        {
            intent.properties = Some(item.clone());
            menu_ui.close_menu();
        }
        intent
    }

    /// Delicate Apple HIG selection checkbox with authentic vector checkmark path.
    fn paint_selection_checkbox(
        ui: &Ui,
        rect: Rect,
        checked: bool,
        alpha: f32,
        palette: &ThemePalette,
    ) {
        let alpha = alpha.clamp(0.0, 1.0);
        if alpha < 0.01 {
            return;
        }
        let rounding = Rounding::same(4.0);
        let cx = rect.center().x;
        let cy = rect.center().y;

        if checked {
            let fill = Color32::from_rgba_premultiplied(
                (palette.accent.r() as f32 * alpha) as u8,
                (palette.accent.g() as f32 * alpha) as u8,
                (palette.accent.b() as f32 * alpha) as u8,
                (245.0 * alpha) as u8,
            );
            ui.painter().rect_filled(rect, rounding, fill);
            ui.painter().rect_stroke(
                rect,
                rounding,
                Stroke::new(
                    1.0_f32,
                    Color32::from_rgba_premultiplied(
                        (palette.accent.r() as f32 * alpha) as u8,
                        (palette.accent.g() as f32 * alpha) as u8,
                        (palette.accent.b() as f32 * alpha) as u8,
                        (255.0 * alpha) as u8,
                    ),
                ),
            );

            // Vector checkmark path: 3 points (p1 -> p2 -> p3)
            let p1 = Pos2::new(cx - 3.2, cy + 0.1);
            let p2 = Pos2::new(cx - 0.7, cy + 2.7);
            let p3 = Pos2::new(cx + 3.4, cy - 2.5);
            let check_color = Color32::from_rgba_premultiplied(
                (255.0 * alpha) as u8,
                (255.0 * alpha) as u8,
                (255.0 * alpha) as u8,
                (255.0 * alpha) as u8,
            );
            let stroke = Stroke::new(1.3_f32, check_color);
            ui.painter().line_segment([p1, p2], stroke);
            ui.painter().line_segment([p2, p3], stroke);
        } else {
            let bg_alpha = (14.0 * alpha) as u8;
            let bg_col = if palette.is_dark {
                Color32::from_rgba_unmultiplied(255, 255, 255, bg_alpha)
            } else {
                Color32::from_rgba_unmultiplied(0, 0, 0, bg_alpha)
            };
            ui.painter().rect_filled(rect, rounding, bg_col);

            let border_alpha = (55.0 * alpha) as u8;
            let border_col = if palette.is_dark {
                Color32::from_rgba_unmultiplied(160, 165, 175, border_alpha)
            } else {
                Color32::from_rgba_unmultiplied(110, 115, 125, border_alpha)
            };
            ui.painter()
                .rect_stroke(rect, rounding, Stroke::new(0.85_f32, border_col));
        }
    }
}

// ======= end of explorer_view.rs shared-menu engine =======

/// Tab / window title for a folder: its name, or the drive label for a
/// root such as `C:\` (which has no file name and used to show "Folder").
fn folder_title(dir: &Path) -> String {
    match dir.file_name().and_then(|n| n.to_str()) {
        Some(name) => name.to_string(),
        None => {
            let s = dir.to_string_lossy();
            let trimmed = s.trim_end_matches(['\\', '/']);
            if trimmed.is_empty() {
                s.into_owned()
            } else {
                trimmed.to_string()
            }
        }
    }
}

/// Finder alias badge: a small arrow chip on the icon's bottom-left corner
/// marks symlinks and junctions (replaces the old " ->" name suffix).
fn paint_alias_badge(painter: &egui::Painter, icon_rect: Rect) {
    let r = (icon_rect.width() * 0.2).clamp(4.5, 12.0);
    let c = Pos2::new(icon_rect.left() + r, icon_rect.bottom() - r);
    painter.circle_filled(c, r, Color32::from_rgb(248, 248, 250));
    painter.circle_stroke(
        c,
        r,
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(0, 0, 0, 90)),
    );
    let s = r * 0.45;
    let ink = Stroke::new((r * 0.22).max(1.2), Color32::from_rgb(40, 40, 46));
    let tail = Pos2::new(c.x - s, c.y + s);
    let tip = Pos2::new(c.x + s, c.y - s);
    painter.line_segment([tail, tip], ink);
    painter.line_segment([tip, Pos2::new(tip.x - s * 1.1, tip.y)], ink);
    painter.line_segment([tip, Pos2::new(tip.x, tip.y + s * 1.1)], ink);
}

/// Centered placeholder for an empty listing: either the search matched
/// nothing or the folder really has no (visible) items.
fn paint_empty_listing(ui: &mut Ui, palette: &ThemePalette, query: &str) {
    let q = query.trim();
    let (title, hint) = if q.is_empty() {
        ("This folder is empty".to_string(), String::new())
    } else {
        (
            format!("No items match \"{q}\""),
            "Search looks in this folder only. Press Esc to clear it.".to_string(),
        )
    };
    ui.add_space(72.0);
    ui.vertical_centered(|ui| {
        ui.label(
            egui::RichText::new(title)
                .size(15.0)
                .color(palette.text_secondary),
        );
        if !hint.is_empty() {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(hint)
                    .size(12.0)
                    .color(palette.text_muted),
            );
        }
    });
}

#[cfg(test)]
mod ux_helper_tests {
    use super::*;

    #[test]
    fn folder_title_uses_drive_label_for_roots() {
        assert_eq!(folder_title(Path::new("C:\\")), "C:");
        assert_eq!(folder_title(Path::new("/")), "/");
        assert_eq!(folder_title(Path::new("C:\\Users\\haris")), "haris");
    }
}

/// Characters of `name` that a rename pre-selects: the stem without the
/// final extension for files ("report.final.pdf" -> "report.final"), the
/// whole name for folders and dotfiles (".gitignore").
fn rename_stem_chars(name: &str, is_dir: bool) -> usize {
    if !is_dir {
        if let Some(idx) = name.rfind('.') {
            if idx > 0 {
                return name[..idx].chars().count();
            }
        }
    }
    name.chars().count()
}

#[cfg(test)]
mod rename_stem_tests {
    use super::rename_stem_chars;

    #[test]
    fn stem_excludes_only_the_last_extension() {
        assert_eq!(rename_stem_chars("alpha.txt", false), 5);
        assert_eq!(rename_stem_chars("report.final.pdf", false), 12);
        assert_eq!(rename_stem_chars(".gitignore", false), 10);
        assert_eq!(rename_stem_chars("folder.v2", true), 9);
        assert_eq!(rename_stem_chars("noext", false), 5);
    }
}
