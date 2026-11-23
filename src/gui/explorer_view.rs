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

pub use crate::core::ViewMode;

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
///
/// `effective_size` is the byte count actually shown in the Size column:
/// `size_bytes` for files, the fresh recursive total for directories. Passing
/// it in keeps this pure (unit-testable) while letting the explorer filter on
/// the same value the user sees instead of a directory's tiny metadata length.
pub fn matches_column_filter_sized(
    item: &FileItem,
    col: DetailColumn,
    value: &ColumnFilterValue,
    effective_size: Option<u64>,
) -> bool {
    match value {
        ColumnFilterValue::SizeMin(min) => effective_size.is_some_and(|s| s >= *min),
        ColumnFilterValue::SizeMax(max) => effective_size.is_some_and(|s| s <= *max),
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

/// Size bounds are files-only here (kept for callers without the size cache);
/// the explorer uses [`matches_column_filter_sized`] so folders sort/filter by
/// their displayed recursive total.
pub fn matches_column_filter(
    item: &FileItem,
    col: DetailColumn,
    value: &ColumnFilterValue,
) -> bool {
    let size = if item.is_dir {
        None
    } else {
        Some(item.size_bytes)
    };
    matches_column_filter_sized(item, col, value, size)
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
pub struct CachedThumb {
    pub mtime: Option<std::time::SystemTime>,
    /// Decode resolution this entry was produced at; a cache hit
    /// requires a stored size >= the requested one so larger zoom
    /// levels re-decode instead of upscaling a small texture.
    pub max_px: u32,
    pub state: ThumbState,
}

pub enum ThumbState {
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
    pub cached_display_indices: Vec<usize>,
    pub display_dirty: bool,
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
    pub show_new_tag_dialog: bool,
    pub new_tag_buffer: String,
    pub clipboard_paths: Vec<PathBuf>,
    pub clipboard_is_cut: bool,

    // Async Scanning
    pub scan_gen: Arc<AtomicU64>,
    pub scan_tx: Sender<ScanBatch>,
    pub scan_rx: Receiver<ScanBatch>,

    // Async Background Filesystem Worker
    pub fs_worker_tx: Sender<FsOperation>,
    pub fs_worker_rx: Receiver<FsOpResult>,
    /// When the current background copy/move batch was queued (toast gate).
    pub fs_op_started: Option<Instant>,

    // Background Folder-Size Worker (Finder-style recursive byte totals)
    pub dir_sizes: crate::core::lru::LruCache<PathBuf, (Option<std::time::SystemTime>, u64)>,
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
    pub properties_sha256: Option<(PathBuf, String)>,
    pub properties_rename_buf: String,

    // Vim & Emacs Navigation State
    pub focused_row: usize,
    pub last_g_press: Option<Instant>,
    pub focus_search_requested: bool,
    pub requested_open_preferences: bool,

    // Stage 2 native pop-out menus: a right-click records a request here;
    // MossambiApp::update renders it in an immediate OS viewport so the menu
    // can escape the window bounds. Result intents are applied next frame.
    pub pending_native_menu: Option<NativeMenuRequest>,
    pub deferred_native_intents: Option<NativeMenuIntents>,
    /// Set once the native menu viewport reports focus, used to dismiss the
    /// menu when focus later leaves it (outside click).
    pub native_menu_focus_seen: bool,

    // macOS Finder & Linux System Features
    pub show_hidden_files: bool,
    pub column_stack: Vec<(PathBuf, Vec<FileItem>)>,
    /// Once-per-open listings for the breadcrumb child-folder popup
    /// (F-2 fix): cleared on every navigate_to.
    pub breadcrumb_popup_cache: std::collections::HashMap<PathBuf, Vec<FileItem>>,
    pub column_preview_item: Option<FileItem>,
    pub column_cache: crate::core::lru::LruCache<PathBuf, Vec<FileItem>>,
    /// One-shot: the Miller hscroll should align to the deepest
    /// column on the next render (set by navigate_to and the
    /// stack-push site).
    pub needs_miller_scroll_align: bool,
    /// One-shot: the breadcrumb path bar should reveal the current
    /// segment on the next render (set by navigate_to when the
    /// directory actually changes).
    pub needs_breadcrumb_scroll_align: bool,

    // Fluid Motion & Transitions
    pub nav_offset_spring: crate::gui::anim::Spring,
    pub nav_alpha_spring: crate::gui::anim::Spring,
    pub click_depth_spring: crate::gui::anim::Spring,
    pub view_mode_crossfade_spring: crate::gui::anim::Spring,
    pub details_transition_spring: crate::gui::anim::Spring,
    pub grid_transition_spring: crate::gui::anim::Spring,
    pub columns_transition_spring: crate::gui::anim::Spring,
    pub new_item_entrance: Option<(PathBuf, crate::gui::anim::Spring)>,
    pub last_view_mode: ViewMode,

    // Windows 11 & macOS Home Dashboard View
    pub is_home_view: bool,
    pub show_internal_toolbar: bool,
    /// B-1 gate: set true every frame by the studio view while the
    /// command palette modal is open. Suppresses all explorer
    /// raw-key handling so palette typing and Enter never leak
    /// into the view underneath.
    pub palette_open: bool,

    // Custom folder icon properties modal state
    pub custom_icon_input: String,
    pub custom_icon_tab: usize,

    // Finder-style long-hover preview (files & folders)
    hover_preview: Option<HoverPreviewState>,
    pub sidebar_hover_active: bool,

    // Drop action menu (Windows/Finder style drag-and-drop popup)
    pub drop_menu: Option<DropMenuState>,

    // Tangerine floating tweak & preview dialog
    pub tweak_dialog: crate::gui::tweak_dialog::TweakDialogState,

    // Lazy image thumbnail cache used by Grid tiles & hover previews
    pub thumb_cache: crate::core::lru::LruCache<PathBuf, CachedThumb>,
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
    /// Independent zoom level for List (Details) view: 0..=4.
    pub list_zoom: u8,
    /// Independent zoom level for Grid (Icons) view: 0..=4.
    pub grid_zoom: u8,
    /// Independent zoom level for Miller Columns view: 0..=4.
    pub miller_zoom: u8,
    /// Smooth spring transition for List row height.
    pub list_zoom_spring: crate::gui::anim::Spring,
    /// Smooth spring transition for Grid icon size.
    pub grid_zoom_spring: crate::gui::anim::Spring,
    /// Smooth spring transition for Miller column width.
    pub miller_zoom_spring: crate::gui::anim::Spring,
    /// Rubber-band marquee: drag origin + the selection snapshot it modifies.
    /// `Some` while a canvas rubber-band drag is in flight.
    marquee: Option<(Pos2, HashSet<PathBuf>)>,

    /// Latched once per press while a native drag hand-off is in flight
    /// (the drag left the window bounds); reset when the pointer is released.
    drag_out_in_flight: bool,

    /// Transient user-facing status note for drag hand-offs and other
    /// non-modal feedback (auto-fades after a few seconds).
    status_note: Option<(String, Instant)>,

    /// Spring-loaded folder hover timer: (folder path, hover start time).
    /// Navigates into the hovered folder after 700ms during an active drag.
    spring_hover_dir: Option<(PathBuf, Instant)>,
    /// Tracks if main window drop target has been registered.
    drop_target_registered: bool,

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
    pub design_tokens: crate::core::DesignTokens,
    pub dashboard_slots: Vec<crate::core::DashboardWidgetSlot>,
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

        let current_dir = crate::core::platform_paths::dashboard_location();
        let drives = get_system_drives();
        let scan_gen = Arc::new(AtomicU64::new(1));

        let initial_tab = ExplorerTab {
            id: 1,
            title: "Home".to_string(),
            icon: "H",
            path: current_dir.clone(),
            is_home: true,
            search_query: String::new(),
        };

        let cfg = crate::core::MossambiConfig::load();
        let mut view = Self {
            current_dir: current_dir.clone(),
            entries: Vec::new(),
            cached_display_indices: Vec::new(),
            display_dirty: true,
            selected_paths: HashSet::new(),
            search_query: String::new(),
            drives,
            last_drives_refresh: None,
            tabs: vec![initial_tab],
            active_tab_index: 0,
            next_tab_id: 2,
            history: vec![current_dir.clone()],
            history_idx: 0,
            view_mode: cfg.view_mode,
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
            is_home_view: true,
            show_internal_toolbar: false,
            palette_open: false,
            new_folder_buffer: "New Folder".to_string(),
            show_new_tag_dialog: false,
            new_tag_buffer: String::new(),
            clipboard_paths: Vec::new(),
            clipboard_is_cut: false,
            scan_gen: Arc::clone(&scan_gen),
            scan_tx: scan_tx.clone(),
            scan_rx,
            fs_worker_tx,
            fs_worker_rx,
            fs_op_started: None,
            dir_sizes: {
                let loaded = crate::core::fs_utils::load_dir_size_cache_from(
                    &crate::core::fs_utils::dir_size_cache_path(),
                );
                let mut cache = crate::core::lru::LruCache::new(10_000);
                for (k, v) in loaded {
                    cache.insert(k, v);
                }
                cache
            },
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
            properties_sha256: None,
            properties_rename_buf: String::new(),
            focused_row: 0,
            last_g_press: None,
            focus_search_requested: false,
            requested_open_preferences: false,
            pending_native_menu: None,
            deferred_native_intents: None,
            native_menu_focus_seen: false,
            show_hidden_files: false,
            column_stack: Vec::new(),
            breadcrumb_popup_cache: std::collections::HashMap::new(),
            column_preview_item: None,
            column_cache: crate::core::lru::LruCache::new(32),
            needs_miller_scroll_align: false,
            needs_breadcrumb_scroll_align: false,
            nav_offset_spring: crate::gui::anim::Spring::new(0.0),
            nav_alpha_spring: crate::gui::anim::Spring::new(1.0),
            click_depth_spring: crate::gui::anim::Spring::new(1.0),
            view_mode_crossfade_spring: crate::gui::anim::Spring::new(1.0),
            details_transition_spring: crate::gui::anim::Spring::new(0.0),
            grid_transition_spring: crate::gui::anim::Spring::new(1.0),
            columns_transition_spring: crate::gui::anim::Spring::new(0.0),
            new_item_entrance: None,
            last_view_mode: cfg.view_mode,
            hover_preview: None,
            sidebar_hover_active: false,
            thumb_cache: crate::core::lru::LruCache::new(256),
            thumb_decode_budget: (0, Instant::now()),
            selection_anchor: None,
            icon_pack: crate::core::icon_packs::IconPack::MacosFinder,
            checkbox_mode: crate::core::config::CheckboxMode::default(),
            menu_style: crate::core::config::MenuStyle::default(),
            animations: crate::core::config::AnimationSettings::default(),
            hover_anims: crate::gui::anim::AnimRegistry::new(),
            explorer_zoom: cfg.grid_zoom.min(4),
            list_zoom: cfg.list_zoom.min(4),
            grid_zoom: cfg.grid_zoom.min(4),
            miller_zoom: cfg.miller_zoom.min(4),
            list_zoom_spring: crate::gui::anim::Spring::new(
                crate::gui::view_zoom::ListZoomMetrics::for_step(cfg.list_zoom.min(4)).row_height,
            ),
            grid_zoom_spring: crate::gui::anim::Spring::new(
                crate::gui::view_zoom::GridZoomMetrics::for_step(cfg.grid_zoom.min(4)).icon_size,
            ),
            miller_zoom_spring: crate::gui::anim::Spring::new(
                crate::gui::view_zoom::MillerZoomMetrics::for_step(cfg.miller_zoom.min(4))
                    .column_width,
            ),
            marquee: None,
            drag_out_in_flight: false,
            custom_icon_input: String::new(),
            custom_icon_tab: 0,
            status_note: None,
            spring_hover_dir: None,
            drop_target_registered: false,
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
            design_tokens: cfg.tokens(),
            dashboard_slots: cfg.dashboard_slots.clone(),
            drop_menu: None,
            tweak_dialog: crate::gui::tweak_dialog::TweakDialogState::default(),
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

    /// Trigger subtle tactile click depression feedback (scale 0.985 -> 1.0).
    pub fn trigger_click_feedback(&mut self) {
        if self.animations.click_feedback && !self.animations.reduced_motion {
            self.click_depth_spring.set(0.985);
        }
    }

    /// Register a newly created folder or file to play its scale+fade entrance animation.
    pub fn register_created_item(&mut self, path: PathBuf) {
        if !self.animations.reduced_motion {
            self.new_item_entrance = Some((path, crate::gui::anim::Spring::new(0.0)));
        }
    }

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

    /// Returns the zoom level (0..=4) for the active view mode.
    pub fn active_mode_zoom(&self) -> u8 {
        match self.view_mode {
            ViewMode::Details => self.list_zoom.min(4),
            ViewMode::Grid => self.grid_zoom.min(4),
            ViewMode::Columns => self.miller_zoom.min(4),
        }
    }

    /// Sets the zoom level for the active view mode, updating springs and bounds.
    pub fn set_active_mode_zoom(&mut self, zoom: u8) {
        let clamped = zoom.min(4);
        match self.view_mode {
            ViewMode::Details => {
                self.list_zoom = clamped;
                let target = crate::gui::view_zoom::ListZoomMetrics::for_step(clamped).row_height;
                if self.animations.reduced_motion {
                    self.list_zoom_spring.value = target;
                    self.list_zoom_spring.target = target;
                    self.list_zoom_spring.velocity = 0.0;
                } else {
                    self.list_zoom_spring.set(target);
                }
            }
            ViewMode::Grid => {
                self.grid_zoom = clamped;
                self.explorer_zoom = clamped;
                let target = crate::gui::view_zoom::GridZoomMetrics::for_step(clamped).icon_size;
                if self.animations.reduced_motion {
                    self.grid_zoom_spring.value = target;
                    self.grid_zoom_spring.target = target;
                    self.grid_zoom_spring.velocity = 0.0;
                } else {
                    self.grid_zoom_spring.set(target);
                }
            }
            ViewMode::Columns => {
                self.miller_zoom = clamped;
                let target =
                    crate::gui::view_zoom::MillerZoomMetrics::for_step(clamped).column_width;
                if self.animations.reduced_motion {
                    self.miller_zoom_spring.value = target;
                    self.miller_zoom_spring.target = target;
                    self.miller_zoom_spring.velocity = 0.0;
                } else {
                    self.miller_zoom_spring.set(target);
                }
            }
        }
    }

    /// Steps the active view mode's zoom level by `step` (+1 or -1) within [0, 4].
    pub fn zoom_step_active_mode(&mut self, step: i32) {
        if step == 0 {
            return;
        }

        let current = self.active_mode_zoom();

        // Custom transition: Details <-> Grid
        if step > 0 && current >= 4 && self.view_mode == ViewMode::Details {
            self.view_mode = ViewMode::Grid;
            self.set_active_mode_zoom(0);
            return;
        }
        if step < 0 && current == 0 && self.view_mode == ViewMode::Grid {
            self.view_mode = ViewMode::Details;
            self.set_active_mode_zoom(4);
            return;
        }

        let next = crate::gui::view_zoom::step_zoom(current, step);
        if next != current {
            log::debug!(
                "Active mode {:?} zoom level: {} -> {}",
                self.view_mode,
                current,
                next
            );
            self.set_active_mode_zoom(next);
        }
    }

    /// Resets the active view mode's zoom level to default (step 2).
    pub fn reset_active_mode_zoom(&mut self) {
        self.set_active_mode_zoom(crate::gui::view_zoom::DEFAULT_ZOOM_STEP);
    }

    /// One Ctrl+Scroll notch or shortcut step on the active view mode's zoom ladder.
    pub fn zoom_ladder_step(&mut self, step: i32) {
        self.zoom_step_active_mode(step);
    }

    /// Advances spring animations for per-mode zoom transitions.
    pub fn advance_zoom_springs(&mut self, ctx: &egui::Context, dt: f32) {
        if self.animations.reduced_motion {
            let list_t =
                crate::gui::view_zoom::ListZoomMetrics::for_step(self.list_zoom).row_height;
            let grid_t = crate::gui::view_zoom::GridZoomMetrics::for_step(self.grid_zoom).icon_size;
            let miller_t =
                crate::gui::view_zoom::MillerZoomMetrics::for_step(self.miller_zoom).column_width;
            self.list_zoom_spring.value = list_t;
            self.list_zoom_spring.target = list_t;
            self.list_zoom_spring.velocity = 0.0;
            self.grid_zoom_spring.value = grid_t;
            self.grid_zoom_spring.target = grid_t;
            self.grid_zoom_spring.velocity = 0.0;
            self.miller_zoom_spring.value = miller_t;
            self.miller_zoom_spring.target = miller_t;
            self.miller_zoom_spring.velocity = 0.0;
            return;
        }

        let mut needs_repaint = false;
        if !self.list_zoom_spring.at_rest() {
            let target =
                crate::gui::view_zoom::ListZoomMetrics::for_step(self.list_zoom).row_height;
            crate::gui::anim::advance(&mut self.list_zoom_spring, target, self.animations.open, dt);
            needs_repaint = true;
        }
        if !self.grid_zoom_spring.at_rest() {
            let target = crate::gui::view_zoom::GridZoomMetrics::for_step(self.grid_zoom).icon_size;
            crate::gui::anim::advance(&mut self.grid_zoom_spring, target, self.animations.open, dt);
            needs_repaint = true;
        }
        if !self.miller_zoom_spring.at_rest() {
            let target =
                crate::gui::view_zoom::MillerZoomMetrics::for_step(self.miller_zoom).column_width;
            crate::gui::anim::advance(
                &mut self.miller_zoom_spring,
                target,
                self.animations.open,
                dt,
            );
            needs_repaint = true;
        }
        if needs_repaint {
            ctx.request_repaint();
        }
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

    /// Navigate to a directory, clearing hover animations and invalidating caches.
    pub fn open_directory(&mut self, dir: PathBuf) {
        self.navigate_to(dir);
    }

    /// Navigate to a new directory asynchronously with zero UI hitching
    pub fn navigate_to(&mut self, dir: PathBuf) {
        if !dir.exists() && !crate::core::platform_paths::is_dashboard_location(&dir) {
            return;
        }

        // Popup listings are cached once per path (F-2 fix); every
        // navigation and refresh invalidates them so the child-folder
        // dropdown never shows stale contents after a fs operation.
        self.breadcrumb_popup_cache.clear();
        self.invalidate_display_cache();
        self.hover_anims.clear();
        // Tag rows read through a per-process cache; every navigation and
        // refresh forgets it so tag edits are visible immediately.
        crate::core::tags::clear_cache();

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
        self.trigger_click_feedback();
        if self.animations.folder_nav_motion && !self.animations.reduced_motion && dir != old_dir {
            if dir.starts_with(&old_dir) {
                self.nav_offset_spring.set(16.0);
                self.nav_alpha_spring.set(0.4);
            } else if old_dir.starts_with(&dir) {
                self.nav_offset_spring.set(-16.0);
                self.nav_alpha_spring.set(0.4);
            } else {
                self.nav_offset_spring.set(8.0);
                self.nav_alpha_spring.set(0.5);
            }
        }
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
                    if !crate::core::platform_paths::is_dashboard_location(&dir) {
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

        self.is_home_view = crate::core::platform_paths::is_dashboard_location(&dir);

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

        if self.is_home_view {
            let user_home = crate::core::platform_paths::home_dir();
            self.dir_watcher.watch(&user_home);
            let new_gen = self.scan_gen.fetch_add(1, Ordering::SeqCst) + 1;
            spawn_async_dir_scan(
                user_home,
                new_gen,
                Arc::clone(&self.scan_gen),
                self.scan_tx.clone(),
            );
        } else {
            self.dir_watcher.watch(&dir);
            let new_gen = self.scan_gen.fetch_add(1, Ordering::SeqCst) + 1;
            spawn_async_dir_scan(
                dir,
                new_gen,
                Arc::clone(&self.scan_gen),
                self.scan_tx.clone(),
            );
        }
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

    /// Syncs Miller column preview and stack when selection changes via keyboard or click.
    pub fn sync_miller_selection(&mut self, item: &FileItem) {
        if self.view_mode != ViewMode::Columns {
            return;
        }
        if !item.is_dir {
            self.column_preview_item = Some(item.clone());
            self.column_stack.clear();
            self.needs_miller_scroll_align = true;
        } else {
            let sub_entries = self.get_column_entries(&item.path);
            self.column_stack.clear();
            self.column_stack.push((item.path.clone(), sub_entries));
            self.column_preview_item = None;
            self.needs_miller_scroll_align = true;
        }
    }

    pub fn add_tab(&mut self, path: PathBuf) {
        let is_home = crate::core::platform_paths::is_dashboard_location(&path);
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

    pub fn trigger_hover_preview(&mut self, item: FileItem, anchor: Pos2) {
        self.sidebar_hover_active = true;
        let is_same = self
            .hover_preview
            .as_ref()
            .map(|h| h.item.path == item.path)
            .unwrap_or(false);
        if !is_same {
            self.hover_preview = Some(HoverPreviewState {
                item,
                anchor,
                started_at: Instant::now(),
                content: HoverContent::Meta,
            });
        }
    }

    pub fn clear_sidebar_hover_preview(&mut self) {
        if self.sidebar_hover_active {
            self.sidebar_hover_active = false;
            self.hover_preview = None;
        }
    }

    pub fn render_floating_preview(&mut self, ui: &Ui, palette: &ThemePalette) {
        if let Some(ref h) = self.hover_preview {
            if h.started_at.elapsed() > std::time::Duration::from_millis(350) {
                self.ensure_hover_content(ui.ctx());
            }
        }
        self.render_hover_preview(ui, palette);
    }

    pub fn can_go_back(&self) -> bool {
        self.history_idx > 0
    }

    pub fn can_go_forward(&self) -> bool {
        self.history_idx + 1 < self.history.len()
    }

    pub fn is_home_directory(&self) -> bool {
        crate::core::platform_paths::is_dashboard_location(&self.current_dir)
            || Some(&self.current_dir) == crate::core::platform_paths::home_dir_opt().as_ref()
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
        self.ensure_display_cache();
        self.selected_paths.clear();
        for &idx in &self.cached_display_indices {
            if let Some(e) = self.entries.get(idx) {
                self.selected_paths.insert(e.path.clone());
            }
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected_paths.clear();
        self.selection_anchor = None;
    }

    /// Selection handling from cached display indices, avoiding full FileItem array cloning.
    pub fn handle_item_click_selection_from_indices(
        &mut self,
        idx: usize,
        path: PathBuf,
        is_selected: bool,
        ctrl: bool,
        shift: bool,
        indices: &[usize],
    ) {
        if ctrl && shift {
            let anchor = self.selection_anchor.unwrap_or(0);
            let (lo, hi) = (anchor.min(idx), anchor.max(idx));
            for &e_idx in indices.iter().take(hi + 1).skip(lo) {
                if let Some(e) = self.entries.get(e_idx) {
                    self.selected_paths.insert(e.path.clone());
                }
            }
            self.selection_anchor = Some(idx);
        } else if shift {
            self.selected_paths.clear();
            let anchor = self.selection_anchor.unwrap_or(0);
            let (lo, hi) = (anchor.min(idx), anchor.max(idx));
            for &e_idx in indices.iter().take(hi + 1).skip(lo) {
                if let Some(e) = self.entries.get(e_idx) {
                    self.selected_paths.insert(e.path.clone());
                }
            }
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(anchor);
            }
        } else if ctrl {
            if is_selected {
                self.selected_paths.remove(&path);
            } else {
                self.selected_paths.insert(path);
            }
            self.selection_anchor = Some(idx);
        } else {
            self.selected_paths.clear();
            self.selected_paths.insert(path);
            self.selection_anchor = Some(idx);
        }
    }

    /// Selection handling from cached display indices without passing external slices.
    pub fn handle_item_click_selection_from_display_index(
        &mut self,
        idx: usize,
        path: PathBuf,
        is_selected: bool,
        ctrl: bool,
        shift: bool,
    ) {
        if ctrl && shift {
            let anchor = self.selection_anchor.unwrap_or(0);
            let (lo, hi) = (anchor.min(idx), anchor.max(idx));
            for i in lo..=hi {
                if let Some(&e_idx) = self.cached_display_indices.get(i) {
                    if let Some(e) = self.entries.get(e_idx) {
                        self.selected_paths.insert(e.path.clone());
                    }
                }
            }
            self.selection_anchor = Some(idx);
        } else if shift {
            self.selected_paths.clear();
            let anchor = self.selection_anchor.unwrap_or(0);
            let (lo, hi) = (anchor.min(idx), anchor.max(idx));
            for i in lo..=hi {
                if let Some(&e_idx) = self.cached_display_indices.get(i) {
                    if let Some(e) = self.entries.get(e_idx) {
                        self.selected_paths.insert(e.path.clone());
                    }
                }
            }
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(anchor);
            }
        } else if ctrl {
            if is_selected {
                self.selected_paths.remove(&path);
            } else {
                self.selected_paths.insert(path);
            }
            self.selection_anchor = Some(idx);
        } else {
            self.selected_paths.clear();
            self.selected_paths.insert(path);
            self.selection_anchor = Some(idx);
        }
    }

    /// Standard Windows Explorer / macOS Finder multi-selection semantics:
    /// - `Ctrl + Shift + Click`: Extends range selection from anchor to `idx` without clearing existing selection.
    /// - `Shift + Click`: Replaces selection with continuous range from anchor (or 0) to `idx`.
    ///   Preserves original anchor so repeated Shift-clicks pivot around the original anchor.
    /// - `Ctrl + Click` / `Cmd + Click`: Toggles the clicked item in/out of selection.
    /// - Plain `Click`: Clears selection and selects only the clicked item.
    pub fn handle_item_click_selection(
        &mut self,
        idx: usize,
        path: PathBuf,
        is_selected: bool,
        ctrl: bool,
        shift: bool,
        entries: &[FileItem],
    ) {
        if ctrl && shift {
            let anchor = self.selection_anchor.unwrap_or(0);
            let (lo, hi) = (anchor.min(idx), anchor.max(idx));
            for e in entries.iter().take(hi + 1).skip(lo) {
                self.selected_paths.insert(e.path.clone());
            }
            self.selection_anchor = Some(idx);
        } else if shift {
            self.selected_paths.clear();
            let anchor = self.selection_anchor.unwrap_or(0);
            let (lo, hi) = (anchor.min(idx), anchor.max(idx));
            for e in entries.iter().take(hi + 1).skip(lo) {
                self.selected_paths.insert(e.path.clone());
            }
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(anchor);
            }
        } else if ctrl {
            if is_selected {
                self.selected_paths.remove(&path);
            } else {
                self.selected_paths.insert(path);
            }
            self.selection_anchor = Some(idx);
        } else {
            self.selected_paths.clear();
            self.selected_paths.insert(path);
            self.selection_anchor = Some(idx);
        }
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
        let target = crate::core::platform_paths::dashboard_location();
        self.navigate_to(target);
    }

    /// Process incoming background directory scanning batches and filesystem watcher events
    pub fn poll_background_scans(&mut self, window_focused: Option<bool>) {
        if self.dir_watcher.try_recv() {
            let dir = if self.is_home_view {
                crate::core::platform_paths::home_dir()
            } else {
                self.current_dir.clone()
            };
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
                self.invalidate_display_cache();
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

            // Toast per the notification gate: enabled AND (unfocused OR
            // the batch ran longer than 10 s). Quick foreground pastes
            // stay silent.
            let elapsed = self
                .fs_op_started
                .take()
                .map(|t| Instant::now().duration_since(t));
            let enabled = crate::core::MossambiConfig::load().notifications_enabled;
            if crate::platform::notify::should_notify(enabled, window_focused, elapsed) {
                let (kind, body) = if res.errors.is_empty() {
                    (
                        crate::platform::notify::NotifyKind::Done,
                        format!("{} {} item(s).", res.op_name, res.count),
                    )
                } else {
                    (
                        crate::platform::notify::NotifyKind::Error,
                        format!(
                            "{} finished with {} error(s).",
                            res.op_name,
                            res.errors.len()
                        ),
                    )
                };
                crate::platform::notify::notify("File operation", &body, kind, None);
            }
        }

        // -- Folder-size worker: drain results, throttle requests, flush --
        while let Ok(res) = self.dir_size_rx.try_recv() {
            self.dir_size_queued.remove(&res.path);
            self.dir_sizes.insert(res.path, (res.mtime, res.total));
            self.dir_size_cache_dirty += 1;
            if self.sort_column == SortColumn::Size {
                self.invalidate_display_cache();
            }
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
            for &idx in self.filtered_and_sorted_indices() {
                if let Some(e) = self.entries.get(idx) {
                    if e.is_dir {
                        candidates.push((e.path.clone(), e.modified));
                    }
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
                if let Some((stored, _)) = self.dir_sizes.peek(&path) {
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
            // Serialize + write on a short-lived worker so the pretty-JSON
            // disk write never stalls the UI frame (a hardware spike). The
            // dirty counter resets here; at most one flush worker is spawned
            // per 30s window, so there is no thread pile-up.
            let flush_path = crate::core::fs_utils::dir_size_cache_path();
            let snapshot: std::collections::HashMap<PathBuf, (Option<std::time::SystemTime>, u64)> =
                self.dir_sizes
                    .iter()
                    .map(|(k, v)| (k.clone(), *v))
                    .collect();
            if let Err(e) = std::thread::Builder::new()
                .name("mossambi-dir-size-flush".into())
                .spawn(move || {
                    if let Err(err) =
                        crate::core::fs_utils::save_dir_size_cache_to(&flush_path, &snapshot)
                    {
                        log::warn!("Folder-size cache flush failed: {err}");
                    }
                })
            {
                log::warn!("Could not spawn folder-size flush worker: {e}");
            }
            self.dir_size_cache_dirty = 0;
            self.dir_size_last_flush = Some(now);
        }
    }

    /// Marks the display list invalidation cache dirty.
    pub fn invalidate_display_cache(&mut self) {
        self.display_dirty = true;
    }

    /// Ensures the cached display indices are fresh before rendering.
    pub fn ensure_display_cache(&mut self) {
        if self.display_dirty {
            self.recompute_display_indices();
        }
    }

    /// Recomputes the filtered and sorted indices into `self.entries`.
    pub fn recompute_display_indices(&mut self) {
        self.cached_display_indices = self.compute_display_indices_pure();
        self.display_dirty = false;
    }

    /// Pure computation of filtered and sorted indices into `self.entries`.
    fn compute_display_indices_pure(&self) -> Vec<usize> {
        let q = self.search_query.trim();
        let is_searching = !q.is_empty();
        let q_lower = q.to_lowercase();
        let active_tag = self.active_tag.as_deref();
        let sort_by_name = matches!(self.sort_column, SortColumn::Name);
        let sort_by_size = matches!(self.sort_column, SortColumn::Size);

        let mut scored_list: Vec<(usize, i64, Option<String>, u64)> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| !crate::core::is_protected_os_file(&e.path))
            .filter(|(_, e)| self.show_hidden_files || !e.is_hidden)
            .filter(|(_, e)| self.category_filter.matches(e))
            .filter(|(_, e)| {
                self.column_filters
                    .iter()
                    .all(|(id, value)| match DetailColumn::from_id(*id) {
                        Some(col) => {
                            matches_column_filter_sized(e, col, value, Some(self.effective_size(e)))
                        }
                        None => true,
                    })
            })
            .filter(|(_, e)| match active_tag {
                Some(tag) => crate::core::tags::read_tags_cache(&e.path)
                    .iter()
                    .any(|t| t == tag),
                None => true,
            })
            .filter_map(|(idx, e)| {
                let name_lower = if is_searching || sort_by_name {
                    Some(e.name.to_lowercase())
                } else {
                    None
                };
                let size = if sort_by_size {
                    self.effective_size(e)
                } else {
                    0
                };
                if is_searching {
                    if !name_lower.as_deref().unwrap_or("").contains(&q_lower) {
                        return None;
                    }
                    let score = fuzzy_match(q, &e.name).unwrap_or(0);
                    Some((idx, score, name_lower, size))
                } else {
                    Some((idx, 0, name_lower, size))
                }
            })
            .collect();

        scored_list.sort_by(
            |&(idx_a, score_a, ref a_lower, a_size), &(idx_b, score_b, ref b_lower, b_size)| {
                let a = &self.entries[idx_a];
                let b = &self.entries[idx_b];
                if a.is_dir != b.is_dir {
                    return b.is_dir.cmp(&a.is_dir);
                }
                let ordering = match self.sort_column {
                    SortColumn::Name => a_lower
                        .as_deref()
                        .unwrap_or("")
                        .cmp(b_lower.as_deref().unwrap_or("")),
                    SortColumn::Size => a_size.cmp(&b_size),
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
                    directed.then_with(|| score_b.cmp(&score_a))
                } else {
                    directed
                }
            },
        );

        scored_list.into_iter().map(|(idx, _, _, _)| idx).collect()
    }

    /// Returns the slice of cached display indices without sorting or cloning.
    #[inline]
    pub fn filtered_and_sorted_indices(&self) -> &[usize] {
        &self.cached_display_indices
    }

    /// Backward-compatible accessor returning references to FileItems.
    /// Uses cache when clean; computes on-the-fly when dirty.
    pub fn filtered_and_sorted_entries(&self) -> Vec<&FileItem> {
        if self.display_dirty {
            let indices = self.compute_display_indices_pure();
            indices
                .into_iter()
                .filter_map(|idx| self.entries.get(idx))
                .collect()
        } else {
            self.cached_display_indices
                .iter()
                .filter_map(|&idx| self.entries.get(idx))
                .collect()
        }
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
        self.invalidate_display_cache();
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
        let window_focused = ui.ctx().input(|i| i.viewport().focused);
        self.poll_background_scans(window_focused);
        self.hover_anims.prune();
        self.ensure_display_cache();

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

        if !self.drop_target_registered {
            let hwnd = crate::gui::dwm_helper::find_main_window_hwnd();
            if hwnd != 0 {
                let _ = crate::core::ole_drop_target::register_main_drop_target(hwnd as usize);
                self.drop_target_registered = true;
            }
        }

        // 1. External OS Drag & Drop Ingestion (winit + Windows OLE IDropTarget)
        let ole_hover = crate::core::ole_drop_target::current_drag_hover();
        let (dropped_files, raw_hovered) =
            ui.input(|i| (i.raw.dropped_files.clone(), !i.raw.hovered_files.is_empty()));
        let is_hovered = raw_hovered || ole_hover.is_active;

        // Drain any external drops received via OLE IDropTarget
        let ole_drops = crate::core::ole_drop_target::poll_external_drops();
        for drop_ev in ole_drops {
            if !drop_ev.paths.is_empty() {
                let copy = drop_ev.effect != 2; // DROPEFFECT_MOVE == 2
                let dest = self.current_dir.clone();
                self.perform_drop(dest, &drop_ev.paths, copy);
            }
        }

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
                let outside = match i.pointer.latest_pos() {
                    Some(p) => !screen_rect.contains(p),
                    // egui clears latest_pos on Event::PointerGone, i.e. the
                    // cursor left the viewport toward another app. Guarded by
                    // any_down && decided below, that means a decided drag is
                    // leaving the app and must hand off to the OS.
                    None => true,
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
            || self.show_new_tag_dialog
            || self.properties_item.is_some()
            || is_search_focused
            || text_field_focused;
        let finder_keybindings = crate::core::MossambiConfig::load().keybinding_profile == 0;
        // Focus changes must happen outside `ui.input`: calling
        // `ui.memory_mut` while the input lock is held deadlocks egui.
        let mut surrender_search_focus = false;

        let nav_indices = self.cached_display_indices.clone();
        let total_nav_items = nav_indices.len();

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
                        self.invalidate_display_cache();
                        self.hover_anims.clear();
                        surrender_search_focus = true;
                    }
                    self.peek_item = None;
                    self.peek_texture = None;
                    self.peek_text_cache = None;
                    self.peek_hash_cache = None;
                    self.renaming_path = None;
                    self.show_new_folder_dialog = false;
                    self.show_new_tag_dialog = false;
                    self.properties_item = None;
                }

                // Vim & Emacs Navigation (Active when not typing in text boxes)
                if !is_typing {
                    // Vim `j` / Down: Cursor Down (Shift+Down extends selection)
                    if total_nav_items > 0
                        && ((i.key_pressed(egui::Key::J)
                            && !i.modifiers.command
                            && !i.modifiers.ctrl
                            && !i.modifiers.alt)
                            || (i.key_pressed(egui::Key::ArrowDown)
                                && !i.modifiers.ctrl
                                && !i.modifiers.command))
                    {
                        let shift = i.modifiers.shift;
                        self.focused_row = (self.focused_row + 1).min(total_nav_items - 1);
                        if let Some(&entry_idx) = nav_indices.get(self.focused_row) {
                            let item = &self.entries[entry_idx];
                            let is_sel = self.selected_paths.contains(&item.path);
                            let item_path = item.path.clone();
                            self.handle_item_click_selection_from_indices(
                                self.focused_row,
                                item_path,
                                is_sel,
                                false,
                                shift,
                                &nav_indices,
                            );
                            if !shift {
                                if let Some(item) = self.entries.get(entry_idx).cloned() {
                                    self.sync_miller_selection(&item);
                                }
                            }
                        }
                    }

                    // Vim `k` / Up / Emacs `Ctrl+P`: Cursor Up (Shift+Up extends selection)
                    if total_nav_items > 0
                        && ((i.key_pressed(egui::Key::K)
                            && !i.modifiers.command
                            && !i.modifiers.ctrl
                            && !i.modifiers.alt)
                            || i.key_pressed(egui::Key::ArrowUp)
                            || ((i.modifiers.ctrl || i.modifiers.command)
                                && i.key_pressed(egui::Key::P)))
                    {
                        let shift = i.modifiers.shift;
                        self.focused_row = self.focused_row.saturating_sub(1);
                        if let Some(&entry_idx) = nav_indices.get(self.focused_row) {
                            let item = &self.entries[entry_idx];
                            let is_sel = self.selected_paths.contains(&item.path);
                            let item_path = item.path.clone();
                            self.handle_item_click_selection_from_indices(
                                self.focused_row,
                                item_path,
                                is_sel,
                                false,
                                shift,
                                &nav_indices,
                            );
                            if !shift {
                                if let Some(item) = self.entries.get(entry_idx).cloned() {
                                    self.sync_miller_selection(&item);
                                }
                            }
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
                        || (self.view_mode == ViewMode::Columns
                            && i.key_pressed(egui::Key::ArrowLeft)
                            && !i.modifiers.alt
                            && !i.modifiers.ctrl
                            && !i.modifiers.command)
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
                        if let Some(item) = nav_indices
                            .get(self.focused_row)
                            .and_then(|&idx| self.entries.get(idx))
                        {
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
                        || (self.view_mode == ViewMode::Columns
                            && i.key_pressed(egui::Key::ArrowRight)
                            && !i.modifiers.alt
                            && !i.modifiers.ctrl
                            && !i.modifiers.command)
                    {
                        if let Some(item) = nav_indices
                            .get(self.focused_row)
                            .and_then(|&idx| self.entries.get(idx))
                        {
                            if item.is_dir {
                                let p = item.path.clone();
                                self.navigate_to(p);
                            } else {
                                open_in_default_app(&item.path);
                                on_stage_file(item.path.clone());
                            }
                        }
                    }

                    // Space: macOS Quick Look / Peek on the focused item (Finder parity)
                    if i.key_pressed(egui::Key::Space) && !i.modifiers.ctrl && !i.modifiers.command
                    {
                        if let Some(item) = nav_indices
                            .get(self.focused_row)
                            .and_then(|&idx| self.entries.get(idx))
                        {
                            self.peek_item = Some(item.clone());
                            self.peek_texture = None;
                            self.peek_text_cache = None;
                            self.peek_hash_cache = None;
                        }
                    }

                    // F2: Rename focused item in place
                    if i.key_pressed(egui::Key::F2) {
                        if let Some(item) = nav_indices
                            .get(self.focused_row)
                            .and_then(|&idx| self.entries.get(idx))
                        {
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
                            if let Some(item) = nav_indices
                                .first()
                                .and_then(|&idx| self.entries.get(idx))
                                .cloned()
                            {
                                self.selected_paths.clear();
                                self.selected_paths.insert(item.path.clone());
                                self.sync_miller_selection(&item);
                            }
                        } else {
                            let now = Instant::now();
                            if let Some(prev) = self.last_g_press {
                                if now.duration_since(prev) < std::time::Duration::from_millis(400)
                                {
                                    self.focused_row = 0;
                                    if let Some(item) = nav_indices
                                        .first()
                                        .and_then(|&idx| self.entries.get(idx))
                                        .cloned()
                                    {
                                        self.selected_paths.clear();
                                        self.selected_paths.insert(item.path.clone());
                                        self.sync_miller_selection(&item);
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
                        if let Some(item) = nav_indices
                            .get(self.focused_row)
                            .and_then(|&idx| self.entries.get(idx))
                            .cloned()
                        {
                            self.selected_paths.clear();
                            self.selected_paths.insert(item.path.clone());
                            self.sync_miller_selection(&item);
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
                        if let Some(item) = nav_indices
                            .get(self.focused_row)
                            .and_then(|&idx| self.entries.get(idx))
                        {
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
                            if let Some(item) = nav_indices
                                .get(self.focused_row)
                                .and_then(|&idx| self.entries.get(idx))
                            {
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
                        if let Some(item) = nav_indices
                            .get(self.focused_row)
                            .and_then(|&idx| self.entries.get(idx))
                        {
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
                        } else if let Some(item) = nav_indices
                            .get(self.focused_row)
                            .and_then(|&idx| self.entries.get(idx))
                        {
                            self.peek_item = Some(item.clone());
                        }
                    }

                    // Alt+Enter / Cmd+I: Properties Dialog
                    if ((i.modifiers.alt && i.key_pressed(egui::Key::Enter))
                        || (i.modifiers.command && i.key_pressed(egui::Key::I)))
                        && !is_typing
                    {
                        if let Some(item) = nav_indices
                            .get(self.focused_row)
                            .and_then(|&idx| self.entries.get(idx))
                        {
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
                    self.invalidate_display_cache();
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
                        if let Some(item) = nav_indices
                            .get(self.focused_row)
                            .and_then(|&idx| self.entries.get(idx))
                        {
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
                        if let Some(item) = nav_indices
                            .get(self.focused_row)
                            .and_then(|&idx| self.entries.get(idx))
                        {
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
                        nav_indices
                            .get(self.focused_row)
                            .and_then(|&idx| self.entries.get(idx))
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
                    self.register_created_item(target_dir.clone());
                    self.navigate_to(self.current_dir.clone());
                }
                self.show_new_folder_dialog = false;
                self.new_folder_buffer = "New Folder".to_string();
            } else if close_dialog {
                self.show_new_folder_dialog = false;
            }
        }

        // 3a. New Tag Modal Dialog (Apple Sheet aesthetic)
        if self.show_new_tag_dialog {
            let mut close_dialog = false;
            let mut create_tag = false;

            egui::Window::new("Create New Tag Dialog")
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
                        egui::RichText::new("New Tag")
                            .size(15.0)
                            .strong()
                            .color(palette.text_primary),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Enter tag name:")
                            .size(12.5)
                            .color(palette.text_secondary),
                    );
                    ui.add_space(6.0);
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.new_tag_buffer).desired_width(260.0),
                    );
                    resp.request_focus();
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        create_tag = true;
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
                                create_tag = true;
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

            if create_tag {
                let custom = crate::core::MossambiConfig::load().custom_tags;
                match crate::core::tags::sanitize_new_tag(&self.new_tag_buffer, &custom) {
                    Ok(name) => {
                        let mut cfg = crate::core::MossambiConfig::load();
                        cfg.custom_tags.push(name.clone());
                        if let Err(e) = cfg.save() {
                            self.show_status_note(Err(format!("Could not save tag: {e}")));
                        } else {
                            self.show_status_note(Ok(format!("Tag \"{name}\" created")));
                            self.show_new_tag_dialog = false;
                            self.new_tag_buffer.clear();
                        }
                    }
                    Err(msg) => self.show_status_note(Err(msg)),
                }
            } else if close_dialog {
                self.show_new_tag_dialog = false;
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
            // Glass mode paints the single translucent layer at the outer
            // CentralPanel; these inner docks must not double-paint window_bg
            // over it (a second layer cancels the acrylic/mica translucency).
            let dock_bg = if palette.is_translucent {
                Color32::TRANSPARENT
            } else {
                palette.window_bg
            };
            if chrome_spec.status_bar {
                let status_resp = egui::TopBottomPanel::bottom("explorer_status_bar")
                    .frame(
                        egui::Frame::none()
                            .fill(dock_bg)
                            .inner_margin(egui::Margin {
                                left: 10.0,
                                right: 10.0,
                                top: 0.0,
                                bottom: 2.0,
                            }),
                    )
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
                let path_resp = egui::TopBottomPanel::bottom("explorer_path_bar")
                    .frame(
                        egui::Frame::none()
                            .fill(dock_bg)
                            .inner_margin(egui::Margin {
                                left: 10.0,
                                right: 10.0,
                                top: 0.0,
                                bottom: 3.0,
                            }),
                    )
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
                        .fill(dock_bg)
                        .inner_margin(egui::Margin {
                            left: 10.0,
                            right: 10.0,
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

        // 7. Tangerine Floating Tweak & Preview Dialog
        if let Some(_opts) = self.tweak_dialog.render(ui.ctx()) {
            let src = self.tweak_dialog.input_path.clone();
            if !src.as_os_str().is_empty() {
                let target = self.tweak_dialog.target_format.clone();
                on_transcode_target(src, target);
            }
        }
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
                        self.invalidate_display_cache();
                        self.hover_anims.clear();
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
                            .desired_width((box_w - 20.0).max(40.0))
                            .min_size(egui::vec2(0.0, 20.0))
                            .margin(egui::Margin::symmetric(10.0, 6.0)),
                    );
                    if search_resp.changed() {
                        self.invalidate_display_cache();
                        self.hover_anims.clear();
                    }
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
                        self.invalidate_display_cache();
                        self.hover_anims.clear();
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
                            .desired_width((box_w - 20.0).max(40.0))
                            .min_size(egui::vec2(0.0, 20.0))
                            .margin(egui::Margin::symmetric(10.0, 6.0)),
                    );
                    if search_resp.changed() {
                        self.invalidate_display_cache();
                        self.hover_anims.clear();
                    }
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
            self.invalidate_display_cache();
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
                let expanded = crate::core::platform_paths::expand_path_str(&self.path_edit_buffer);
                if crate::core::platform_paths::is_dashboard_location(&expanded) {
                    self.navigate_to_home();
                } else if expanded.exists() {
                    self.navigate_to(expanded);
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
                    ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                    for (i, seg) in segments.iter().enumerate() {
                        if i > 0 {
                            ui.add_space(2.0);
                            crate::gui::finder_chrome::paint_path_chevron(ui, palette.text_muted);
                            ui.add_space(2.0);
                        }
                        let is_current = seg.target_path == current_dir;

                        // Authentic macOS Finder segment icon
                        let glyph = if i == 0 {
                            IconGlyph::Drive
                        } else {
                            IconGlyph::Folder
                        };
                        let tint = if is_current {
                            [palette.accent.r(), palette.accent.g(), palette.accent.b()]
                        } else {
                            [
                                palette.text_secondary.r(),
                                palette.text_secondary.g(),
                                palette.text_secondary.b(),
                            ]
                        };

                        let label_font = egui::FontId::proportional(12.5);
                        let text_color = if is_current {
                            palette.text_primary
                        } else {
                            palette.text_secondary
                        };
                        let galley =
                            ui.painter()
                                .layout_no_wrap(seg.label.clone(), label_font, text_color);
                        let icon_side = 16.0;
                        let gap = 4.0;
                        let pad_x = 4.0;
                        let seg_w = pad_x * 2.0 + icon_side + gap + galley.size().x;
                        let (seg_rect, resp) =
                            ui.allocate_exact_size(Vec2::new(seg_w, 24.0), egui::Sense::click());

                        if resp.hovered() {
                            crate::gui::squircle::squircle_filled(
                                ui.painter(),
                                seg_rect,
                                Rounding::same(4.0),
                                palette.card_hover,
                            );
                        }

                        let icon_rect = Rect::from_center_size(
                            Pos2::new(
                                seg_rect.left() + pad_x + icon_side * 0.5,
                                seg_rect.center().y,
                            ),
                            Vec2::splat(icon_side),
                        );
                        crate::core::icon_packs::paint_icon(
                            ui,
                            self.icon_pack,
                            glyph,
                            icon_rect,
                            tint,
                        );

                        let text_pos = Pos2::new(
                            seg_rect.left() + pad_x + icon_side + gap,
                            seg_rect.center().y - galley.size().y * 0.5,
                        );
                        ui.painter().galley(text_pos, galley, text_color);

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
                                    crate::gui::squircle::squircle_stroke(
                                        ui.painter(),
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
                            self.navigate_to_home();
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
                            self.navigate_to(crate::core::platform_paths::home_dir());
                        }

                        ui.separator();
                    }

                    self.render_filter_popover(ui, palette);
                },
            );

            // Right Pinned Container: macOS 3-Segmented Switcher (Icons / List / Columns) & Icon Size
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut switch_mode = self.view_mode;
                crate::gui::finder_chrome::render_styled_segmented_view_switcher(
                    ui,
                    palette,
                    &self.design_tokens,
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
                        self.invalidate_display_cache();
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
            let active_item = self
                .filtered_and_sorted_indices()
                .get(self.focused_row)
                .and_then(|&idx| self.entries.get(idx))
                .cloned();

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

        let dt = crate::gui::anim::frame_dt(ui.ctx());
        if self.animations.folder_nav_motion && !self.animations.reduced_motion {
            let _ = crate::gui::anim::advance(
                &mut self.nav_offset_spring,
                0.0,
                crate::core::config::AnimStyle::Natural,
                dt,
            );
            let alpha = crate::gui::anim::advance(
                &mut self.nav_alpha_spring,
                1.0,
                crate::core::config::AnimStyle::Natural,
                dt,
            );
            if alpha < 0.99 {
                ui.set_opacity(alpha);
            }
            if !self.nav_offset_spring.at_rest() || !self.nav_alpha_spring.at_rest() {
                ui.ctx().request_repaint();
            }
        }

        // Distinct View-Mode Transitions
        if self.last_view_mode != self.view_mode {
            self.last_view_mode = self.view_mode;
            if !self.animations.reduced_motion {
                let view_anim_id = ui.id().with("file_view_mode_fade");
                ui.ctx().animate_value_with_time(view_anim_id, 0.0, 0.0);
                match self.view_mode {
                    ViewMode::Details => self.details_transition_spring.set(1.0),
                    ViewMode::Grid => self.grid_transition_spring.set(0.0),
                    ViewMode::Columns => self.columns_transition_spring.set(1.0),
                }
            }
        }

        if self.animations.reduced_motion {
            self.details_transition_spring.set(0.0);
            self.grid_transition_spring.set(1.0);
            self.columns_transition_spring.set(0.0);
            self.click_depth_spring.set(1.0);
        } else {
            match self.view_mode {
                ViewMode::Details => {
                    if !self.details_transition_spring.at_rest() {
                        crate::gui::anim::advance(
                            &mut self.details_transition_spring,
                            0.0,
                            self.animations.open,
                            dt,
                        );
                        ui.ctx().request_repaint();
                    }
                }
                ViewMode::Grid => {
                    if !self.grid_transition_spring.at_rest() {
                        crate::gui::anim::advance(
                            &mut self.grid_transition_spring,
                            1.0,
                            self.animations.open,
                            dt,
                        );
                        ui.ctx().request_repaint();
                    }
                }
                ViewMode::Columns => {
                    if !self.columns_transition_spring.at_rest() {
                        crate::gui::anim::advance(
                            &mut self.columns_transition_spring,
                            0.0,
                            self.animations.open,
                            dt,
                        );
                        ui.ctx().request_repaint();
                    }
                }
            }

            if self.animations.click_feedback {
                let _ = crate::gui::anim::advance(
                    &mut self.click_depth_spring,
                    1.0,
                    crate::core::config::AnimStyle::Snappy,
                    dt,
                );
                if !self.click_depth_spring.at_rest() {
                    ui.ctx().request_repaint();
                }
            }

            self.advance_zoom_springs(ui.ctx(), dt);

            // Newly created item entrance animation (scale + fade in)
            if let Some((_, ref mut spring)) = self.new_item_entrance {
                crate::gui::anim::advance(spring, 1.0, crate::core::config::AnimStyle::Natural, dt);
                if !spring.at_rest() {
                    ui.ctx().request_repaint();
                } else if spring.value >= 0.99 {
                    self.new_item_entrance = None;
                }
            }
        }

        let view_anim_id = ui.id().with("file_view_mode_fade");
        let view_alpha = if self.animations.reduced_motion {
            1.0
        } else {
            ui.ctx().animate_value_with_time(view_anim_id, 1.0, 0.150)
        };
        if view_alpha < 0.999 {
            ui.set_opacity(view_alpha);
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

        // Drag ghost stack + count badge pill following the pointer:
        // Stack of up to 3 thumbnails with soft shadows, count badge pill
        // filled with palette.accent and high-contrast text_on_accent.
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
            let count = egui::DragAndDrop::payload::<FileDragPayload>(ui.ctx())
                .map(|p| p.paths.len())
                .unwrap_or(1);
            let ghost_layout = crate::core::drag_out::compute_drag_ghost_layout(
                count,
                ctrl,
                shift,
                alt,
                cfg.drop_action_pref,
            );

            if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                let bp = ui.painter().clone().with_layer_id(egui::LayerId::new(
                    egui::Order::Tooltip,
                    egui::Id::new("mossambi_dnd_ghost_stack"),
                ));

                // Render stack of up to 3 cards
                let card_w = 44.0_f32;
                let card_h = 44.0_f32;
                for (idx, &(ox, oy)) in ghost_layout.card_offsets.iter().enumerate().rev() {
                    let card_pos = pos + Vec2::new(12.0 + ox, 12.0 + oy);
                    let card_rect = Rect::from_min_size(card_pos, Vec2::new(card_w, card_h));

                    // Soft shadow
                    if !cfg.animations.reduced_motion {
                        crate::gui::squircle::squircle_filled(
                            &bp,
                            card_rect.translate(Vec2::new(2.0, 3.0)),
                            Rounding::same(6.0),
                            Color32::from_black_alpha(45),
                        );
                    }

                    // Card surface
                    let surface_alpha = if idx == 0 { 245 } else { 190 };
                    let fill = Color32::from_rgba_unmultiplied(
                        palette.card_bg.r(),
                        palette.card_bg.g(),
                        palette.card_bg.b(),
                        surface_alpha,
                    );
                    crate::gui::squircle::squircle_filled(
                        &bp,
                        card_rect,
                        Rounding::same(6.0),
                        fill,
                    );
                    crate::gui::squircle::squircle_stroke(
                        &bp,
                        card_rect,
                        Rounding::same(6.0),
                        Stroke::new(1.0_f32, palette.border),
                    );

                    // Card inner border glyph
                    let inner_rect = card_rect.shrink(8.0);
                    crate::gui::squircle::squircle_stroke(
                        &bp,
                        inner_rect,
                        Rounding::same(3.0),
                        Stroke::new(1.0_f32, palette.text_secondary),
                    );
                }

                // Render count badge pill (palette.accent fill, guaranteed text_on_accent)
                let pill_pos = pos
                    + Vec2::new(
                        12.0 + ghost_layout.badge_offset.0,
                        12.0 + ghost_layout.badge_offset.1,
                    );
                let pill_rect = Rect::from_min_size(
                    pill_pos,
                    Vec2::new(ghost_layout.pill_width, ghost_layout.pill_height),
                );
                if !cfg.animations.reduced_motion {
                    crate::gui::squircle::squircle_filled(
                        &bp,
                        pill_rect.translate(Vec2::new(1.0, 2.0)),
                        Rounding::same(10.0),
                        Color32::from_black_alpha(50),
                    );
                }
                crate::gui::squircle::squircle_filled(
                    &bp,
                    pill_rect,
                    Rounding::same(10.0),
                    palette.accent,
                );

                let text_color = crate::gui::design::contrast::text_on_accent(palette.accent);
                let font = egui::FontId::proportional(11.0);
                let galley =
                    ui.painter()
                        .layout_no_wrap(ghost_layout.badge_text.clone(), font, text_color);
                let text_pos = Pos2::new(
                    pill_rect.center().x - galley.size().x * 0.5,
                    pill_rect.center().y - galley.size().y * 0.5,
                );
                bp.galley(text_pos, galley, text_color);
            }
        }

        // Drop Action Menu popup
        self.render_drop_menu(ui, palette, on_transcode_target);
    }

    fn render_quick_access_cards(
        &self,
        ui: &mut Ui,
        palette: &ThemePalette,
        pinned_folders: &[(String, &'static str, [u8; 3], PathBuf)],
        available_w: f32,
        nav_to: &mut Option<PathBuf>,
        new_dash_hover: &mut Option<(FileItem, Pos2)>,
    ) {
        let grid = crate::gui::dashboard_layout::compute_card_grid(available_w, 110.0, 160.0, 12.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(grid.gap, grid.gap);

            for (label, _glyph, rgb, path) in pinned_folders {
                let display_label = truncate_end(label, 14);
                let card_size = Vec2::new(grid.card_width, 82.0);
                let (rect, resp) = ui.allocate_exact_size(card_size, egui::Sense::click());

                let is_hovered = resp.hovered();
                let bg = if is_hovered {
                    palette.card_hover
                } else {
                    palette.surface_2
                };
                let border_stroke = Stroke::new(
                    self.design_tokens.border_width,
                    if is_hovered {
                        palette.accent
                    } else {
                        palette.border
                    },
                );

                crate::gui::squircle::squircle_filled(
                    ui.painter(),
                    rect,
                    Rounding::same(self.design_tokens.radius_card),
                    bg,
                );
                crate::gui::squircle::squircle_stroke(
                    ui.painter(),
                    rect,
                    Rounding::same(self.design_tokens.radius_card),
                    border_stroke,
                );

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

                ui.painter().text(
                    Pos2::new(rect.center().x, rect.bottom() - 14.0),
                    egui::Align2::CENTER_CENTER,
                    display_label,
                    egui::FontId::proportional(12.0),
                    palette.text_primary,
                );

                if resp.hovered() {
                    let card_anchor = rect.center() + Vec2::new(0.0, rect.height() * 0.5 + 14.0);
                    *new_dash_hover = Some((FileItem::from_path(path.clone()), card_anchor));
                }

                if resp.clicked() || resp.double_clicked() {
                    *nav_to = Some(path.clone());
                }
            }
        });
    }

    fn render_drives_cards(
        &self,
        ui: &mut Ui,
        palette: &ThemePalette,
        tier: crate::gui::dashboard_layout::DashboardTier,
        available_w: f32,
        nav_to: &mut Option<PathBuf>,
    ) {
        if self.drives.is_empty() {
            ui.label(
                egui::RichText::new("No system drives detected.")
                    .size(12.0)
                    .color(palette.text_muted),
            );
            return;
        }

        let (card_w, _cols) =
            crate::gui::dashboard_layout::compute_drive_card_width(available_w, tier, 12.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(12.0, 12.0);

            for drive in &self.drives {
                let card_size = Vec2::new(card_w, 72.0);
                let (rect, resp) = ui.allocate_exact_size(card_size, egui::Sense::click());

                let is_hovered = resp.hovered();
                let bg = if is_hovered {
                    palette.card_hover
                } else {
                    palette.surface_2
                };
                let border_stroke = Stroke::new(
                    self.design_tokens.border_width,
                    if is_hovered {
                        palette.accent
                    } else {
                        palette.border
                    },
                );

                crate::gui::squircle::squircle_filled(
                    ui.painter(),
                    rect,
                    Rounding::same(self.design_tokens.radius_card),
                    bg,
                );
                crate::gui::squircle::squircle_stroke(
                    ui.painter(),
                    rect,
                    Rounding::same(self.design_tokens.radius_card),
                    border_stroke,
                );

                let badge_center = Pos2::new(rect.left() + 24.0, rect.center().y);
                let d_glyph = if drive.is_system {
                    crate::core::icon_packs::IconGlyph::DriveSystem
                } else {
                    crate::core::icon_packs::IconGlyph::Drive
                };
                let big_rect = Rect::from_center_size(badge_center, Vec2::splat(28.0));
                let d_rgb: [u8; 3] = if drive.is_system {
                    [0, 122, 255]
                } else {
                    [100, 116, 139]
                };
                crate::core::icon_packs::paint_icon(ui, self.icon_pack, d_glyph, big_rect, d_rgb);

                let text_left = rect.left() + 48.0;
                ui.painter().text(
                    Pos2::new(text_left, rect.top() + 10.0),
                    egui::Align2::LEFT_TOP,
                    &drive.display_name,
                    egui::FontId::proportional(13.0),
                    palette.text_primary,
                );

                let ratio = drive.usage_ratio().clamp(0.0, 1.0);
                let gauge_color = crate::gui::dashboard_layout::storage_usage_color(ratio, palette);

                // Mini circular storage usage ring on top right
                let ring_center = Pos2::new(rect.right() - 24.0, rect.top() + 24.0);
                let ring_r = 11.0;
                ui.painter().circle_stroke(
                    ring_center,
                    ring_r,
                    Stroke::new(
                        2.0_f32,
                        Color32::from_rgba_unmultiplied(
                            palette.border.r(),
                            palette.border.g(),
                            palette.border.b(),
                            100,
                        ),
                    ),
                );
                if ratio > 0.01 {
                    let num_pts = ((32.0 * ratio) as usize).clamp(4, 32);
                    let start_angle = -std::f32::consts::FRAC_PI_2;
                    let sweep = std::f32::consts::TAU * ratio;
                    let points: Vec<Pos2> = (0..=num_pts)
                        .map(|i| {
                            let theta = start_angle + sweep * (i as f32 / num_pts as f32);
                            Pos2::new(
                                ring_center.x + ring_r * theta.cos(),
                                ring_center.y + ring_r * theta.sin(),
                            )
                        })
                        .collect();
                    ui.painter()
                        .add(egui::Shape::line(points, Stroke::new(2.5_f32, gauge_color)));
                }
                ui.painter().text(
                    Pos2::new(rect.right() - 24.0, rect.top() + 44.0),
                    egui::Align2::CENTER_CENTER,
                    format!("{:.0}%", ratio * 100.0),
                    egui::FontId::monospace(9.5),
                    palette.text_muted,
                );

                // Usage progress bar
                let bar_y = rect.top() + 32.0;
                let bar_w = (card_w - 92.0).max(40.0);
                let bar_h = 4.0_f32;
                let bar_rect =
                    Rect::from_min_size(Pos2::new(text_left, bar_y), Vec2::new(bar_w, bar_h));
                crate::gui::squircle::squircle_filled(
                    ui.painter(),
                    bar_rect,
                    Rounding::same(2.0),
                    Color32::from_rgba_unmultiplied(
                        palette.border.r(),
                        palette.border.g(),
                        palette.border.b(),
                        100,
                    ),
                );
                let fill_w = (bar_w * ratio).max(2.0);
                let fill_rect =
                    Rect::from_min_size(Pos2::new(text_left, bar_y), Vec2::new(fill_w, bar_h));
                crate::gui::squircle::squircle_filled(
                    ui.painter(),
                    fill_rect,
                    Rounding::same(2.0),
                    gauge_color,
                );

                let capacity_text = if drive.total_bytes == 0 {
                    "Capacity unknown"
                } else {
                    &drive.formatted_space()
                };
                ui.painter().text(
                    Pos2::new(text_left, rect.top() + 44.0),
                    egui::Align2::LEFT_TOP,
                    capacity_text,
                    egui::FontId::proportional(11.0),
                    palette.text_secondary,
                );

                if resp.clicked() || resp.double_clicked() {
                    *nav_to = Some(drive.path.clone());
                }
            }
        });
    }

    fn render_quick_actions_bar(
        &self,
        ui: &mut Ui,
        palette: &ThemePalette,
        available_w: f32,
        action_out: &mut Option<u8>,
    ) {
        let actions = [
            (
                "New Folder",
                crate::core::icon_packs::IconGlyph::Folder,
                [235, 175, 60],
                1u8,
            ),
            (
                "Terminal",
                crate::core::icon_packs::IconGlyph::PageCode,
                [139, 92, 246],
                2u8,
            ),
            (
                "Search",
                crate::core::icon_packs::IconGlyph::Search,
                [59, 130, 246],
                3u8,
            ),
            (
                "Recycle Bin",
                crate::core::icon_packs::IconGlyph::RecycleBin,
                [16, 185, 129],
                4u8,
            ),
        ];

        let cols = ((available_w + 8.0) / 130.0).floor().clamp(2.0, 4.0) as usize;
        let gap = 8.0_f32;
        let item_w = ((available_w - (cols - 1) as f32 * gap) / cols as f32).max(90.0);

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(gap, gap);
            for (label, glyph, rgb, action_code) in actions {
                let (rect, resp) =
                    ui.allocate_exact_size(Vec2::new(item_w, 36.0), egui::Sense::click());
                let is_hovered = resp.hovered();
                let bg = if is_hovered {
                    palette.card_hover
                } else {
                    palette.surface_2
                };
                let stroke = Stroke::new(
                    self.design_tokens.border_width,
                    if is_hovered {
                        palette.accent
                    } else {
                        palette.border
                    },
                );

                crate::gui::squircle::squircle_filled(
                    ui.painter(),
                    rect,
                    Rounding::same(self.design_tokens.radius_pill),
                    bg,
                );
                crate::gui::squircle::squircle_stroke(
                    ui.painter(),
                    rect,
                    Rounding::same(self.design_tokens.radius_pill),
                    stroke,
                );

                let icon_rect = Rect::from_center_size(
                    Pos2::new(rect.left() + 18.0, rect.center().y),
                    Vec2::splat(16.0),
                );
                crate::core::icon_packs::paint_icon(ui, self.icon_pack, glyph, icon_rect, rgb);

                ui.painter().text(
                    Pos2::new(rect.left() + 32.0, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    label,
                    egui::FontId::proportional(12.0),
                    palette.text_primary,
                );

                if resp.clicked() {
                    *action_out = Some(action_code);
                }
            }
        });
    }

    fn render_activity_summary(
        &self,
        ui: &mut Ui,
        palette: &ThemePalette,
        available_w: f32,
        pinned_count: usize,
    ) {
        let total_bytes: u64 = self.drives.iter().map(|d| d.total_bytes).sum();
        let free_bytes: u64 = self.drives.iter().map(|d| d.free_bytes).sum();
        let used_bytes = total_bytes.saturating_sub(free_bytes);
        let usage_ratio = if total_bytes > 0 {
            used_bytes as f32 / total_bytes as f32
        } else {
            0.0
        };

        let (card_rect, resp) =
            ui.allocate_exact_size(Vec2::new(available_w, 54.0), egui::Sense::hover());
        let bg = if resp.hovered() {
            palette.card_hover
        } else {
            palette.surface_2
        };

        crate::gui::squircle::squircle_filled(
            ui.painter(),
            card_rect,
            Rounding::same(self.design_tokens.radius_card),
            bg,
        );
        crate::gui::squircle::squircle_stroke(
            ui.painter(),
            card_rect,
            Rounding::same(self.design_tokens.radius_card),
            Stroke::new(self.design_tokens.border_width, palette.border),
        );

        let chip_w = (card_rect.width() / 3.0).floor();

        // Chip 1: Total Storage
        let chip1_rect = Rect::from_min_size(card_rect.min, Vec2::new(chip_w, card_rect.height()));
        ui.painter().text(
            Pos2::new(chip1_rect.center().x, chip1_rect.top() + 14.0),
            egui::Align2::CENTER_CENTER,
            "Total Storage",
            egui::FontId::proportional(11.0),
            palette.text_muted,
        );
        ui.painter().text(
            Pos2::new(chip1_rect.center().x, chip1_rect.bottom() - 14.0),
            egui::Align2::CENTER_CENTER,
            crate::gui::dashboard_layout::format_bytes_ascii(total_bytes),
            egui::FontId::monospace(12.5),
            palette.text_primary,
        );

        // Chip 2: Free Space
        let chip2_rect = Rect::from_min_size(
            Pos2::new(card_rect.left() + chip_w, card_rect.top()),
            Vec2::new(chip_w, card_rect.height()),
        );
        ui.painter().text(
            Pos2::new(chip2_rect.center().x, chip2_rect.top() + 14.0),
            egui::Align2::CENTER_CENTER,
            "Free Space",
            egui::FontId::proportional(11.0),
            palette.text_muted,
        );
        ui.painter().text(
            Pos2::new(chip2_rect.center().x, chip2_rect.bottom() - 14.0),
            egui::Align2::CENTER_CENTER,
            crate::gui::dashboard_layout::format_bytes_ascii(free_bytes),
            egui::FontId::monospace(12.5),
            palette.success,
        );

        // Chip 3: Overall Load & Shortcuts
        let chip3_rect = Rect::from_min_size(
            Pos2::new(card_rect.left() + chip_w * 2.0, card_rect.top()),
            Vec2::new(
                card_rect.right() - (card_rect.left() + chip_w * 2.0),
                card_rect.height(),
            ),
        );
        ui.painter().text(
            Pos2::new(chip3_rect.center().x, chip3_rect.top() + 14.0),
            egui::Align2::CENTER_CENTER,
            format!("{} Shortcuts", pinned_count),
            egui::FontId::proportional(11.0),
            palette.text_muted,
        );
        let usage_color = crate::gui::dashboard_layout::storage_usage_color(usage_ratio, palette);
        ui.painter().text(
            Pos2::new(chip3_rect.center().x, chip3_rect.bottom() - 14.0),
            egui::Align2::CENTER_CENTER,
            format!("{:.1}% Used", usage_ratio * 100.0),
            egui::FontId::monospace(12.5),
            usage_color,
        );
    }

    fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
        let t = t.clamp(0.0, 1.0);
        let r = (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t).round() as u8;
        let g = (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t).round() as u8;
        let b_ch = (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t).round() as u8;
        let alpha = (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t).round() as u8;
        Color32::from_rgba_unmultiplied(r, g, b_ch, alpha)
    }

    fn render_recent_files_view(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        tier: crate::gui::dashboard_layout::DashboardTier,
        available_w: f32,
        recent_files: &[FileItem],
    ) {
        if recent_files.is_empty() {
            ui.label(
                egui::RichText::new("No recent standalone files in home directory. Choose a folder above to browse.")
                    .size(12.0)
                    .color(palette.text_muted),
            );
            return;
        }

        let layout = crate::gui::dashboard_layout::compute_recent_table(available_w, tier);

        let header_h = 24.0;
        let (header_rect, _) =
            ui.allocate_exact_size(Vec2::new(available_w, header_h), egui::Sense::hover());
        crate::gui::squircle::squircle_filled(
            ui.painter(),
            header_rect,
            Rounding::same(self.design_tokens.radius_subtle),
            palette.surface_1,
        );

        let mut x = header_rect.left() + 32.0;
        ui.painter().text(
            Pos2::new(x, header_rect.center().y),
            egui::Align2::LEFT_CENTER,
            "Name",
            egui::FontId::proportional(11.5),
            palette.text_muted,
        );
        x += layout.name_width;

        if layout.show_date {
            ui.painter().text(
                Pos2::new(x, header_rect.center().y),
                egui::Align2::LEFT_CENTER,
                "Date Modified",
                egui::FontId::proportional(11.5),
                palette.text_muted,
            );
            x += layout.date_width;
        }

        if layout.show_kind {
            ui.painter().text(
                Pos2::new(x, header_rect.center().y),
                egui::Align2::LEFT_CENTER,
                "Kind",
                egui::FontId::proportional(11.5),
                palette.text_muted,
            );
            x += layout.kind_width;
        }

        if layout.show_path {
            ui.painter().text(
                Pos2::new(x, header_rect.center().y),
                egui::Align2::LEFT_CENTER,
                "Parent Path",
                egui::FontId::proportional(11.5),
                palette.text_muted,
            );
        }

        ui.painter().text(
            Pos2::new(header_rect.right() - 14.0, header_rect.center().y),
            egui::Align2::RIGHT_CENTER,
            "Size",
            egui::FontId::proportional(11.5),
            palette.text_muted,
        );

        ui.add_space(2.0);

        for (idx, item) in recent_files.iter().enumerate() {
            let row_h = 26.0;
            let (row_rect, row_resp) =
                ui.allocate_exact_size(Vec2::new(available_w, row_h), egui::Sense::click());

            let is_hovered = row_resp.hovered();
            let is_sel = self.selected_paths.contains(&item.path);
            let target_sel = if is_sel { 1.0_f32 } else { 0.0_f32 };
            let sel_t = if self.animations.reduced_motion {
                target_sel
            } else {
                ui.ctx().animate_value_with_time(
                    ui.id().with(("recent_sel", idx)),
                    target_sel,
                    0.150,
                )
            };

            let base_bg = if is_hovered {
                palette.card_hover
            } else if idx % 2 == 1 {
                palette.surface_2
            } else {
                Color32::from_rgba_unmultiplied(
                    palette.surface_2.r(),
                    palette.surface_2.g(),
                    palette.surface_2.b(),
                    180,
                )
            };

            let row_bg = if sel_t > 0.001 {
                Self::lerp_color(base_bg, palette.selection, sel_t)
            } else {
                base_bg
            };

            crate::gui::squircle::squircle_filled(
                ui.painter(),
                row_rect,
                Rounding::same(self.design_tokens.radius_subtle),
                row_bg,
            );

            if sel_t > 0.001 {
                let stroke_col = Color32::from_rgba_unmultiplied(
                    palette.accent.r(),
                    palette.accent.g(),
                    palette.accent.b(),
                    (sel_t * 255.0).round() as u8,
                );
                crate::gui::squircle::squircle_stroke(
                    ui.painter(),
                    row_rect,
                    Rounding::same(self.design_tokens.radius_subtle),
                    Stroke::new(1.0_f32, stroke_col),
                );
            }

            let slide_x = if self.animations.reduced_motion {
                0.0
            } else {
                2.0 * sel_t
            };

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
            let icon_rect = Rect::from_center_size(
                Pos2::new(row_rect.left() + 16.0 + slide_x, row_rect.center().y),
                Vec2::splat(18.0),
            );
            crate::core::icon_packs::paint_icon(
                ui,
                self.icon_pack,
                glyph_ast,
                icon_rect,
                accent_rgb,
            );

            let name_chars = (layout.name_width / 8.5).floor() as usize;
            let display_name = truncate_end(&item.name, name_chars.max(12));
            let mut cur_x = row_rect.left() + 32.0 + slide_x;
            let text_color = if sel_t > 0.001 {
                Self::lerp_color(palette.text_primary, Color32::WHITE, sel_t)
            } else {
                palette.text_primary
            };
            ui.painter().text(
                Pos2::new(cur_x, row_rect.center().y),
                egui::Align2::LEFT_CENTER,
                display_name,
                egui::FontId::proportional(12.5),
                text_color,
            );
            cur_x += layout.name_width;

            if layout.show_date {
                ui.painter().text(
                    Pos2::new(cur_x, row_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    item.formatted_date(),
                    egui::FontId::proportional(11.5),
                    palette.text_secondary,
                );
                cur_x += layout.date_width;
            }

            if layout.show_kind {
                let (tag_color, tag_name) = if icon_spec.is_code {
                    (palette.tag_code, "Code")
                } else if icon_spec.is_media {
                    (palette.tag_media, "Media")
                } else if icon_spec.is_archive {
                    (palette.tag_archive, "Archive")
                } else if icon_spec.is_document {
                    (palette.tag_document, "Document")
                } else if icon_spec.is_image {
                    (palette.tag_image, "Image")
                } else {
                    (palette.text_muted, icon_spec.category)
                };

                let badge_rect = Rect::from_min_size(
                    Pos2::new(cur_x, row_rect.center().y - 8.0),
                    Vec2::new((layout.kind_width - 16.0).max(40.0), 16.0),
                );
                crate::gui::squircle::squircle_filled(
                    ui.painter(),
                    badge_rect,
                    Rounding::same(self.design_tokens.radius_pill),
                    Color32::from_rgba_unmultiplied(
                        tag_color.r(),
                        tag_color.g(),
                        tag_color.b(),
                        40,
                    ),
                );
                ui.painter().text(
                    badge_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    tag_name,
                    egui::FontId::proportional(10.0),
                    tag_color,
                );
                cur_x += layout.kind_width;
            }

            if layout.show_path {
                let parent_str = item
                    .path
                    .parent()
                    .and_then(|p| p.file_name())
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_else(|| "~".to_string());
                ui.painter().text(
                    Pos2::new(cur_x, row_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    truncate_end(&parent_str, 16),
                    egui::FontId::proportional(11.0),
                    palette.text_muted,
                );
            }

            ui.painter().text(
                Pos2::new(row_rect.right() - 14.0, row_rect.center().y),
                egui::Align2::RIGHT_CENTER,
                item.formatted_size(),
                egui::FontId::monospace(11.0),
                palette.text_muted,
            );

            if row_resp.double_clicked() {
                self.trigger_click_feedback();
                open_in_default_app(&item.path);
            } else if row_resp.clicked() {
                self.trigger_click_feedback();
                let ctrl = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
                let shift = ui.input(|i| i.modifiers.shift);
                self.handle_item_click_selection(
                    idx,
                    item.path.clone(),
                    is_sel,
                    ctrl,
                    shift,
                    recent_files,
                );
            }
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
        let mut nav_to: Option<PathBuf> = None;
        let mut new_dash_hover: Option<(FileItem, Pos2)> = None;
        let mut action_out: Option<u8> = None;
        let mut slot_swap: Option<(usize, usize)> = None;

        let total_avail_w = ui.available_width();
        let container = crate::gui::dashboard_layout::compute_container_metrics(
            total_avail_w,
            self.design_tokens.max_content_width,
        );

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

        for entry in &self.entries {
            if !entry.is_dir || entry.is_hidden || entry.is_symlink {
                continue;
            }
            if !is_library_folder_name(&entry.name) {
                continue;
            }
            let lower = entry.name.to_lowercase();
            if pinned_folders
                .iter()
                .any(|(label, _, _, _)| label.to_lowercase() == lower)
            {
                continue;
            }
            pinned_folders.push((
                entry.name.clone(),
                "Folder",
                [235, 175, 60],
                entry.path.clone(),
            ));
        }

        let recycle_target = crate::core::resolve_recycle_bin_target();
        pinned_folders.push((
            "Recycle Bin".to_string(),
            "Recycle Bin",
            [16, 185, 129],
            recycle_target,
        ));

        let mut recent_files: Vec<FileItem> = self
            .entries
            .iter()
            .filter(|e| !e.is_dir && !e.is_hidden)
            .cloned()
            .collect();
        recent_files.sort_by_key(|e| std::cmp::Reverse(e.modified));
        recent_files.truncate(15);

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.add_space(container.margin_x);

                    ui.vertical(|ui| {
                        ui.set_width(container.content_width);
                        ui.set_max_width(container.content_width);
                        let content_w = container.content_width;
                        let tier = container.tier;

                        let bento_frame = egui::Frame::none()
                            .fill(palette.surface_2)
                            .stroke(Stroke::new(self.design_tokens.border_width, palette.border))
                            .rounding(Rounding::same(self.design_tokens.radius_card))
                            .inner_margin(egui::Margin::symmetric(18.0, 16.0));

                        match self.design_tokens.style {
                            crate::core::DesignStyle::ArcaneFluid => {
                                // 1. Quick Access Bento Tile
                                bento_frame.show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let (ir, _) = ui.allocate_exact_size(
                                            Vec2::splat(16.0),
                                            egui::Sense::hover(),
                                        );
                                        crate::core::icon_packs::paint_icon(
                                            ui,
                                            self.icon_pack,
                                            crate::core::icon_packs::IconGlyph::FolderFavorites,
                                            ir,
                                            [palette.accent.r(), palette.accent.g(), palette.accent.b()],
                                        );
                                        ui.add_space(4.0);
                                        ui.label(
                                            egui::RichText::new("Quick Access")
                                                .size(15.0)
                                                .strong()
                                                .color(palette.text_primary),
                                        );
                                    });
                                    ui.add_space(10.0);
                                    self.render_quick_access_cards(
                                        ui,
                                        palette,
                                        &pinned_folders,
                                        content_w - 36.0,
                                        &mut nav_to,
                                        &mut new_dash_hover,
                                    );
                                });

                                ui.add_space(16.0);

                                // 2. Bento Grid Section (Drives + Quick Actions / Activity)
                                if matches!(
                                    tier,
                                    crate::gui::dashboard_layout::DashboardTier::Medium
                                        | crate::gui::dashboard_layout::DashboardTier::Wide
                                        | crate::gui::dashboard_layout::DashboardTier::Ultrawide
                                ) {
                                    let gap = 16.0;
                                    let left_w = ((content_w - gap) * 0.58).floor();
                                    let right_w = content_w - gap - left_w;

                                    ui.horizontal(|ui| {
                                        // Left Bento Tile: Drives & Storage
                                        ui.vertical(|ui| {
                                            ui.set_width(left_w);
                                            ui.set_max_width(left_w);
                                            bento_frame.show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    let (ir, _) = ui.allocate_exact_size(
                                                        Vec2::splat(16.0),
                                                        egui::Sense::hover(),
                                                    );
                                                    crate::core::icon_packs::paint_icon(
                                                        ui,
                                                        self.icon_pack,
                                                        crate::core::icon_packs::IconGlyph::Drive,
                                                        ir,
                                                        [palette.accent.r(), palette.accent.g(), palette.accent.b()],
                                                    );
                                                    ui.add_space(4.0);
                                                    ui.label(
                                                        egui::RichText::new("Drives & Storage")
                                                            .size(15.0)
                                                            .strong()
                                                            .color(palette.text_primary),
                                                    );
                                                });
                                                ui.add_space(10.0);
                                                self.render_drives_cards(
                                                    ui,
                                                    palette,
                                                    tier,
                                                    left_w - 36.0,
                                                    &mut nav_to,
                                                );
                                            });
                                        });

                                        ui.add_space(gap);

                                        // Right Bento Tiles: Quick Actions + Storage Activity
                                        ui.vertical(|ui| {
                                            ui.set_width(right_w);
                                            ui.set_max_width(right_w);

                                            bento_frame.show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    let (ir, _) = ui.allocate_exact_size(
                                                        Vec2::splat(16.0),
                                                        egui::Sense::hover(),
                                                    );
                                                    crate::core::icon_packs::paint_icon(
                                                        ui,
                                                        self.icon_pack,
                                                        crate::core::icon_packs::IconGlyph::ActionMore,
                                                        ir,
                                                        [palette.accent.r(), palette.accent.g(), palette.accent.b()],
                                                    );
                                                    ui.add_space(4.0);
                                                    ui.label(
                                                        egui::RichText::new("Quick Actions")
                                                            .size(15.0)
                                                            .strong()
                                                            .color(palette.text_primary),
                                                    );
                                                });
                                                ui.add_space(8.0);
                                                self.render_quick_actions_bar(
                                                    ui,
                                                    palette,
                                                    right_w - 36.0,
                                                    &mut action_out,
                                                );
                                            });

                                            ui.add_space(12.0);

                                            bento_frame.show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    let (ir, _) = ui.allocate_exact_size(
                                                        Vec2::splat(16.0),
                                                        egui::Sense::hover(),
                                                    );
                                                    crate::core::icon_packs::paint_icon(
                                                        ui,
                                                        self.icon_pack,
                                                        crate::core::icon_packs::IconGlyph::ActionShield,
                                                        ir,
                                                        [palette.accent.r(), palette.accent.g(), palette.accent.b()],
                                                    );
                                                    ui.add_space(4.0);
                                                    ui.label(
                                                        egui::RichText::new("Storage Activity")
                                                            .size(15.0)
                                                            .strong()
                                                            .color(palette.text_primary),
                                                    );
                                                });
                                                ui.add_space(8.0);
                                                self.render_activity_summary(
                                                    ui,
                                                    palette,
                                                    right_w - 36.0,
                                                    pinned_folders.len(),
                                                );
                                            });
                                        });
                                    });
                                } else {
                                    // Compact: single column stack with bento containment
                                    bento_frame.show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new("Drives & Storage")
                                                .size(15.0)
                                                .strong()
                                                .color(palette.text_primary),
                                        );
                                        ui.add_space(8.0);
                                        self.render_drives_cards(
                                            ui,
                                            palette,
                                            tier,
                                            content_w - 36.0,
                                            &mut nav_to,
                                        );
                                    });

                                    ui.add_space(14.0);

                                    bento_frame.show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new("Quick Actions")
                                                .size(15.0)
                                                .strong()
                                                .color(palette.text_primary),
                                        );
                                        ui.add_space(8.0);
                                        self.render_quick_actions_bar(
                                            ui,
                                            palette,
                                            content_w - 36.0,
                                            &mut action_out,
                                        );
                                    });

                                    ui.add_space(14.0);

                                    bento_frame.show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new("Storage Activity")
                                                .size(15.0)
                                                .strong()
                                                .color(palette.text_primary),
                                        );
                                        ui.add_space(8.0);
                                        self.render_activity_summary(
                                            ui,
                                            palette,
                                            content_w - 36.0,
                                            pinned_folders.len(),
                                        );
                                    });
                                }

                                ui.add_space(16.0);

                                // 3. Recent Files Bento Tile
                                bento_frame.show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let (ir, _) = ui.allocate_exact_size(
                                            Vec2::splat(16.0),
                                            egui::Sense::hover(),
                                        );
                                        crate::core::icon_packs::paint_icon(
                                            ui,
                                            self.icon_pack,
                                            crate::core::icon_packs::IconGlyph::ActionList,
                                            ir,
                                            [palette.accent.r(), palette.accent.g(), palette.accent.b()],
                                        );
                                        ui.add_space(4.0);
                                        ui.label(
                                            egui::RichText::new("Recent Files")
                                                .size(15.0)
                                                .strong()
                                                .color(palette.text_primary),
                                        );
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(
                                                egui::RichText::new(format!("{} items", recent_files.len()))
                                                    .size(11.5)
                                                    .color(palette.text_muted),
                                            );
                                        });
                                    });
                                    ui.add_space(8.0);
                                    self.render_recent_files_view(
                                        ui,
                                        palette,
                                        tier,
                                        content_w - 36.0,
                                        &recent_files,
                                    );
                                });
                            }

                            crate::core::DesignStyle::ZenithPro => {
                                let cockpit = crate::gui::dashboard_layout::compute_zenith_cockpit(
                                    content_w, tier,
                                );
                                if cockpit.is_split {
                                    let gap = 16.0;
                                    ui.horizontal(|ui| {
                                        // Left Cockpit Pane (60%)
                                        ui.vertical(|ui| {
                                            ui.set_max_width(cockpit.left_width);
                                            ui.label(
                                                egui::RichText::new("Workspace Cockpit")
                                                    .size(15.0)
                                                    .strong()
                                                    .color(palette.text_primary),
                                            );
                                            ui.add_space(6.0);
                                            self.render_quick_access_cards(
                                                ui,
                                                palette,
                                                &pinned_folders,
                                                cockpit.left_width,
                                                &mut nav_to,
                                                &mut new_dash_hover,
                                            );
                                            ui.add_space(14.0);

                                            ui.label(
                                                egui::RichText::new("Volume Telemetry")
                                                    .size(14.0)
                                                    .strong()
                                                    .color(palette.text_primary),
                                            );
                                            ui.add_space(6.0);
                                            self.render_drives_cards(
                                                ui,
                                                palette,
                                                tier,
                                                cockpit.left_width,
                                                &mut nav_to,
                                            );
                                            ui.add_space(14.0);

                                            ui.label(
                                                egui::RichText::new("Operations")
                                                    .size(14.0)
                                                    .strong()
                                                    .color(palette.text_primary),
                                            );
                                            ui.add_space(6.0);
                                            self.render_quick_actions_bar(
                                                ui,
                                                palette,
                                                cockpit.left_width,
                                                &mut action_out,
                                            );
                                        });

                                        ui.add_space(gap);

                                        // Right Cockpit Pane (40%)
                                        ui.vertical(|ui| {
                                            ui.set_max_width(cockpit.right_width);
                                            ui.label(
                                                egui::RichText::new("Activity Telemetry")
                                                    .size(15.0)
                                                    .strong()
                                                    .color(palette.text_primary),
                                            );
                                            ui.add_space(6.0);
                                            self.render_activity_summary(
                                                ui,
                                                palette,
                                                cockpit.right_width,
                                                pinned_folders.len(),
                                            );
                                            ui.add_space(14.0);

                                            ui.label(
                                                egui::RichText::new("Recent Files")
                                                    .size(14.0)
                                                    .strong()
                                                    .color(palette.text_primary),
                                            );
                                            ui.add_space(6.0);
                                            self.render_recent_files_view(
                                                ui,
                                                palette,
                                                tier,
                                                cockpit.right_width,
                                                &recent_files,
                                            );
                                        });
                                    });
                                } else {
                                    ui.label(
                                        egui::RichText::new("Quick Access")
                                            .size(15.0)
                                            .strong()
                                            .color(palette.text_primary),
                                    );
                                    ui.add_space(6.0);
                                    self.render_quick_access_cards(
                                        ui,
                                        palette,
                                        &pinned_folders,
                                        content_w,
                                        &mut nav_to,
                                        &mut new_dash_hover,
                                    );
                                    ui.add_space(14.0);

                                    ui.label(
                                        egui::RichText::new("Drives & Storage")
                                            .size(15.0)
                                            .strong()
                                            .color(palette.text_primary),
                                    );
                                    ui.add_space(6.0);
                                    self.render_drives_cards(
                                        ui,
                                        palette,
                                        tier,
                                        content_w,
                                        &mut nav_to,
                                    );
                                    ui.add_space(14.0);

                                    ui.label(
                                        egui::RichText::new("Quick Actions")
                                            .size(14.0)
                                            .strong()
                                            .color(palette.text_primary),
                                    );
                                    ui.add_space(6.0);
                                    self.render_quick_actions_bar(
                                        ui,
                                        palette,
                                        content_w,
                                        &mut action_out,
                                    );
                                    ui.add_space(14.0);

                                    ui.label(
                                        egui::RichText::new("Storage Activity")
                                            .size(14.0)
                                            .strong()
                                            .color(palette.text_primary),
                                    );
                                    ui.add_space(6.0);
                                    self.render_activity_summary(
                                        ui,
                                        palette,
                                        content_w,
                                        pinned_folders.len(),
                                    );
                                    ui.add_space(14.0);

                                    ui.label(
                                        egui::RichText::new("Recent Files")
                                            .size(15.0)
                                            .strong()
                                            .color(palette.text_primary),
                                    );
                                    ui.add_space(6.0);
                                    self.render_recent_files_view(
                                        ui,
                                        palette,
                                        tier,
                                        content_w,
                                        &recent_files,
                                    );
                                }
                            }

                            crate::core::DesignStyle::HyperionSlate => {
                                let slots = self.dashboard_slots.clone();
                                let total_slots = slots.len();

                                for (i, slot) in slots.into_iter().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(slot.label())
                                                .size(15.0)
                                                .strong()
                                                .color(palette.text_primary),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if i + 1 < total_slots
                                                    && ui.small_button("v").clicked()
                                                {
                                                    slot_swap = Some((i, i + 1));
                                                }
                                                if i > 0 && ui.small_button("^").clicked() {
                                                    slot_swap = Some((i, i - 1));
                                                }
                                            },
                                        );
                                    });
                                    ui.add_space(8.0);

                                    match slot {
                                        crate::core::DashboardWidgetSlot::QuickAccess => {
                                            self.render_quick_access_cards(
                                                ui,
                                                palette,
                                                &pinned_folders,
                                                content_w,
                                                &mut nav_to,
                                                &mut new_dash_hover,
                                            );
                                        }
                                        crate::core::DashboardWidgetSlot::Drives => {
                                            self.render_drives_cards(
                                                ui,
                                                palette,
                                                tier,
                                                content_w,
                                                &mut nav_to,
                                            );
                                        }
                                        crate::core::DashboardWidgetSlot::QuickActions => {
                                            self.render_quick_actions_bar(
                                                ui,
                                                palette,
                                                content_w,
                                                &mut action_out,
                                            );
                                        }
                                        crate::core::DashboardWidgetSlot::Activity => {
                                            self.render_activity_summary(
                                                ui,
                                                palette,
                                                content_w,
                                                pinned_folders.len(),
                                            );
                                        }
                                        crate::core::DashboardWidgetSlot::RecentFiles => {
                                            self.render_recent_files_view(
                                                ui,
                                                palette,
                                                tier,
                                                content_w,
                                                &recent_files,
                                            );
                                        }
                                    }
                                    ui.add_space(20.0);
                                }
                            }
                        }
                    });

                    if container.margin_x > 0.0 {
                        ui.add_space(container.margin_x);
                    }
                });
            });

        if let Some((a, b)) = slot_swap {
            if a < self.dashboard_slots.len() && b < self.dashboard_slots.len() {
                self.dashboard_slots.swap(a, b);
                let mut cfg = crate::core::config::MossambiConfig::load();
                cfg.dashboard_slots = self.dashboard_slots.clone();
                let _ = cfg.save();
            }
        }

        if let Some(act) = action_out {
            match act {
                1 => {
                    self.show_new_folder_dialog = true;
                }
                2 => {
                    #[cfg(target_os = "windows")]
                    {
                        let _ = std::process::Command::new("powershell.exe")
                            .arg("-NoExit")
                            .arg("-Command")
                            .arg(format!(
                                "Set-Location -LiteralPath '{}'",
                                self.current_dir.display()
                            ))
                            .spawn();
                    }
                }
                3 => {
                    self.focus_search_requested = true;
                }
                4 => {
                    nav_to = Some(crate::core::resolve_recycle_bin_target());
                }
                _ => {}
            }
        }

        if let Some(path) = nav_to {
            self.navigate_to(path);
        }

        // Long-hover preview on dashboard cards (folder contents peek)
        let keep_existing = matches!(
            (&self.hover_preview, &new_dash_hover),
            (Some(active), Some((item, _))) if active.item.path == item.path
        ) || (self.sidebar_hover_active && self.hover_preview.is_some());
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
            self.sidebar_hover_active = false;
            self.hover_preview = Some(HoverPreviewState {
                item,
                anchor,
                started_at: Instant::now(),
                content: HoverContent::Meta,
            });
        } else if !self.sidebar_hover_active {
            self.hover_preview = None;
        }
        self.render_hover_preview(ui, palette);
    }

    fn paint_sort_chevron(
        painter: &egui::Painter,
        center: Pos2,
        direction: SortDirection,
        color: Color32,
    ) {
        let s = 3.5_f32;
        let pts = match direction {
            SortDirection::Ascending => [
                Pos2::new(center.x - s, center.y + s * 0.5),
                Pos2::new(center.x, center.y - s * 0.5),
                Pos2::new(center.x + s, center.y + s * 0.5),
            ],
            SortDirection::Descending => [
                Pos2::new(center.x - s, center.y - s * 0.5),
                Pos2::new(center.x, center.y + s * 0.5),
                Pos2::new(center.x + s, center.y - s * 0.5),
            ],
        };
        painter.line_segment([pts[0], pts[1]], Stroke::new(1.4_f32, color));
        painter.line_segment([pts[1], pts[2]], Stroke::new(1.4_f32, color));
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
        self.ensure_display_cache();
        let total_items = self.cached_display_indices.len();

        let slide_p = self.details_transition_spring.value;
        if slide_p > 0.001 {
            ui.set_opacity((1.0 - slide_p * 0.5).clamp(0.0, 1.0));
        }

        // -- Tools & Converters toolbar (sits directly above the parameter row) --
        self.render_action_toolbar(
            ui,
            palette,
            &[],
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
        let tier = crate::gui::dashboard_layout::tier_for_width(content_width);
        let bounded = crate::gui::view_zoom::compute_bounded_list_columns(
            content_width,
            tier,
            self.design_tokens.style,
        );
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
            let target_flex = (available_for_cols - fixed).max(min_flex);
            widths[flex_idx] = target_flex.min(bounded.name_max_width.max(min_flex));
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
                crate::gui::squircle::squircle_filled(
                    &hp,
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
            let funnel = if filter_active {
                format!(" {}", FILTER_FUNNEL_GLYPH)
            } else {
                String::new()
            };
            let text_color = if is_active || filter_active {
                palette.accent
            } else {
                palette.text_muted
            };
            let label_text = format!("{}{}", col.label(), funnel);
            let font = egui::FontId::proportional(11.5);
            let galley = hp.layout_no_wrap(label_text, font, text_color);
            let text_w = galley.size().x;
            let sort_reserve = if is_active { 12.0 } else { 0.0 };
            let tx = if right_align {
                hrect.right() - col_pad - sort_reserve - text_w
            } else {
                hrect.left() + col_pad
            };
            hp.galley(
                Pos2::new(tx, hrect.center().y - galley.size().y * 0.5),
                galley,
                text_color,
            );
            if is_active {
                let arrow_x = if right_align {
                    hrect.right() - col_pad - 4.0
                } else {
                    tx + text_w + 7.0
                };
                Self::paint_sort_chevron(
                    &hp,
                    Pos2::new(arrow_x, hrect.center().y),
                    self.sort_direction,
                    palette.accent,
                );
            }
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
            self.invalidate_display_cache();
        }

        ui.add_space(2.0);

        let mut nav_into_dir: Option<PathBuf> = None;
        let peek_to_open: Option<FileItem> = None;
        let mut item_intents: Vec<ItemMenuIntents> = Vec::new();
        let mut canvas_intents = CanvasMenuIntents::default();
        let mut new_hover: Option<(FileItem, Pos2)> = None;
        let mut drop_ops: Vec<DropOp> = Vec::new();
        let mut table_start_pos: Option<Pos2> = None;
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

        let zoom_metrics = crate::gui::view_zoom::ListZoomMetrics::for_step(self.list_zoom);
        let row_height = if (self.list_zoom_spring.value - zoom_metrics.row_height).abs() > 0.05 {
            self.list_zoom_spring.value
        } else {
            zoom_metrics.row_height
        };
        let font_sz = zoom_metrics.font_size;
        let icon_sz = zoom_metrics.icon_size;
        let row_spacing = 2.0_f32;
        let stride = row_height + row_spacing;

        ui.spacing_mut().item_spacing = Vec2::new(0.0, row_spacing);

        let ctrl_held = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
        egui::ScrollArea::vertical()
            .enable_scrolling(!ctrl_held)
            .auto_shrink([false; 2])
            .show_rows(ui, row_height, total_items, |ui, row_range| {
                // Canvas interact registered FIRST so rows (registered after it) stay
                // on top: egui gives the pointer to the last-registered widget covering
                // it. This prevents the empty-canvas menu from swallowing row clicks -
                // the long-standing "click on a file does nothing" bug.
                let canvas_resp = ui.interact(
                    ui.clip_rect(),
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
                        total_items > 0,
                        self.sort_column,
                        dir_desc,
                        self.view_mode,
                        &self.current_dir,
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

                ui.spacing_mut().item_spacing = Vec2::new(0.0, row_spacing);

                for idx in row_range {
                    let entry_idx = match self.cached_display_indices.get(idx) {
                        Some(&i) => i,
                        None => continue,
                    };
                    let item = match self.entries.get(entry_idx) {
                        Some(it) => it.clone(),
                        None => continue,
                    };
                    let is_selected = self.selected_paths.contains(&item.path);

                    let (row_rect, row_resp) = ui.allocate_exact_size(
                        Vec2::new(content_width, row_height),
                        egui::Sense::click_and_drag(),
                    );
                    if table_start_pos.is_none() {
                        table_start_pos = Some(Pos2::new(
                            row_rect.left(),
                            row_rect.top() - idx as f32 * stride,
                        ));
                    }
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
                            let ctrl = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
                            let shift = ui.input(|i| i.modifiers.shift);
                            self.handle_item_click_selection_from_display_index(
                                idx,
                                item.path.clone(),
                                is_selected,
                                ctrl || !shift,
                                shift,
                            );
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
                            let icon_side = icon_sz;
                            let icon_rect = Rect::from_center_size(
                                Pos2::new(name_x + 2.0 + icon_side * 0.5, row_rect.center().y),
                                Vec2::splat(icon_side),
                            );
                            self.paint_item_icon(ui, &item, icon_glyph, icon_rect, icon_accent);
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
                            let name_font = egui::FontId::proportional(font_sz + 0.5);
                            // The name text starts after the icon + tag dots.
                            let name_pos_x = icon_rect.right() + 6.0 + dots_w;
                            let name_avail = (cell_rect.right() - name_pos_x - col_pad).max(20.0);
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
                                    egui::FontId::proportional(font_sz - 1.0),
                                    palette.text_secondary,
                                    egui::Align2::LEFT_CENTER,
                                ),
                                DetailColumn::DateModified => (
                                    item.formatted_date(),
                                    if bounded.tabular_figures {
                                        egui::FontId::monospace(font_sz - 1.5)
                                    } else {
                                        egui::FontId::proportional(font_sz - 1.0)
                                    },
                                    palette.text_secondary,
                                    egui::Align2::LEFT_CENTER,
                                ),
                                DetailColumn::DateCreated => (
                                    item.formatted_created(),
                                    if bounded.tabular_figures {
                                        egui::FontId::monospace(font_sz - 1.5)
                                    } else {
                                        egui::FontId::proportional(font_sz - 1.0)
                                    },
                                    palette.text_secondary,
                                    egui::Align2::LEFT_CENTER,
                                ),
                                DetailColumn::Size => {
                                    let size_text = self.dir_size_text(&item);
                                    (
                                        size_text,
                                        egui::FontId::monospace(font_sz - 1.5),
                                        palette.text_secondary,
                                        egui::Align2::RIGHT_CENTER,
                                    )
                                }
                                DetailColumn::Kind => (
                                    icon_spec.category.to_string(),
                                    egui::FontId::proportional(font_sz - 1.0),
                                    palette.text_muted,
                                    egui::Align2::LEFT_CENTER,
                                ),
                                DetailColumn::Permissions => (
                                    item.formatted_permissions(),
                                    egui::FontId::monospace(font_sz - 1.5),
                                    palette.text_secondary,
                                    egui::Align2::LEFT_CENTER,
                                ),
                                DetailColumn::Hidden => (
                                    if item.is_hidden {
                                        "Yes".to_string()
                                    } else {
                                        String::new()
                                    },
                                    egui::FontId::proportional(font_sz - 1.0),
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
                        self.trigger_click_feedback();
                        self.selection_anchor = Some(idx);
                        if item.is_dir {
                            nav_into_dir = Some(item.path.clone());
                        } else {
                            open_in_default_app(&item.path);
                        }
                    } else if row_resp.clicked() {
                        self.trigger_click_feedback();
                        self.focused_row = idx;
                        let ctrl = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
                        let shift = ui.input(|i| i.modifiers.shift);
                        self.handle_item_click_selection_from_display_index(
                            idx,
                            item.path.clone(),
                            is_selected,
                            ctrl,
                            shift,
                        );
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

                    // Lift effect when row is part of the actively dragged selection
                    let is_actively_dragged =
                        egui::DragAndDrop::has_payload_of_type::<FileDragPayload>(ui.ctx())
                            && self.selected_paths.contains(&item.path);
                    if is_actively_dragged && !self.animations.reduced_motion {
                        crate::gui::squircle::squircle_filled(
                            ui.painter(),
                            row_rect.translate(Vec2::new(0.0, 2.0)),
                            Rounding::same(5.0),
                            Color32::from_black_alpha(45),
                        );
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
                                crate::gui::squircle::squircle_filled(
                                    ui.painter(),
                                    row_rect,
                                    Rounding::same(5.0),
                                    Color32::from_rgba_unmultiplied(
                                        palette.accent.r(),
                                        palette.accent.g(),
                                        palette.accent.b(),
                                        (a * 70.0) as u8,
                                    ),
                                );

                                // Spring-loaded folder navigation: 700ms continuous hover opens folder
                                let now = Instant::now();
                                let should_open = match &self.spring_hover_dir {
                                    Some((path, start)) if path == &item.path => {
                                        start.elapsed() >= std::time::Duration::from_millis(700)
                                    }
                                    _ => {
                                        self.spring_hover_dir = Some((item.path.clone(), now));
                                        false
                                    }
                                };
                                if should_open {
                                    self.spring_hover_dir = None;
                                    self.navigate_to(item.path.clone());
                                }
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
                    if row_resp.secondary_clicked() && !self.selected_paths.contains(&item.path) {
                        self.selected_paths.clear();
                        self.selected_paths.insert(item.path.clone());
                    }
                    row_resp.context_menu(|menu_ui| {
                        let targets = if self.selected_paths.len() > 1
                            && self.selected_paths.contains(&item.path)
                        {
                            self.selected_paths.iter().cloned().collect()
                        } else {
                            vec![item.path.clone()]
                        };
                        item_intents.clear();
                        item_intents.push(Self::run_item_context_menu(
                            menu_ui, icon_pack, menu_style, tint, &item, &targets,
                        ));
                    });
                }
            });

        if total_items == 0 {
            paint_empty_listing(ui, palette, &self.search_query);
        }

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
                crate::gui::squircle::squircle_filled(
                    &mp,
                    mrect,
                    Rounding::same(2.0),
                    Color32::from_rgba_unmultiplied(
                        palette.accent.r(),
                        palette.accent.g(),
                        palette.accent.b(),
                        30,
                    ),
                );
                crate::gui::squircle::squircle_stroke(
                    &mp,
                    mrect,
                    Rounding::same(2.0),
                    Stroke::new(1.0_f32, palette.accent),
                );
                let additive =
                    ui.input(|i| i.modifiers.ctrl || i.modifiers.command || i.modifiers.shift);
                let hits: Vec<PathBuf> = if let Some(start_pos) = table_start_pos {
                    let table_left = start_pos.x;
                    let table_top = start_pos.y;
                    let horiz_intersects =
                        mrect.max.x >= table_left && mrect.min.x <= table_left + content_width;
                    if horiz_intersects && mrect.max.y >= table_top && total_items > 0 {
                        let start_f = (mrect.min.y - table_top - row_height) / stride;
                        let start_i = if start_f <= 0.0 {
                            0
                        } else {
                            (start_f.ceil() as usize).min(total_items)
                        };
                        let end_f = (mrect.max.y - table_top) / stride;
                        let end_i =
                            (end_f.floor().max(0.0) as usize).min(total_items.saturating_sub(1));
                        if start_i <= end_i {
                            self.cached_display_indices[start_i..=end_i]
                                .iter()
                                .filter_map(|&entry_idx| {
                                    self.entries.get(entry_idx).map(|e| e.path.clone())
                                })
                                .collect()
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };
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
        if canvas_intents.new_tag {
            self.show_new_tag_dialog = true;
            self.new_tag_buffer.clear();
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
            self.select_all();
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
            self.invalidate_display_cache();
        }
        if let Some(vm) = canvas_intents.view_mode {
            self.view_mode = vm;
        }
        if canvas_intents.invert_selection {
            let listed: std::collections::HashSet<PathBuf> = self
                .cached_display_indices
                .iter()
                .filter_map(|&idx| self.entries.get(idx).map(|e| e.path.clone()))
                .collect();
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
        if canvas_intents.preferences {
            self.requested_open_preferences = true;
        }
        if let Some((session_id, cmd_id)) = canvas_intents.invoke_shell_cmd {
            crate::platform::invoke_shell_command(session_id, cmd_id, (0, 0));
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
        ) || (self.sidebar_hover_active && self.hover_preview.is_some());
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
            self.sidebar_hover_active = false;
            self.hover_preview = Some(HoverPreviewState {
                item,
                anchor,
                started_at: Instant::now(),
                content: HoverContent::Meta,
            });
        } else if !self.sidebar_hover_active {
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
        _entries: &[FileItem],
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
                        if fm::menu_item_labeled(
                            menu_ui,
                            pack,
                            fm::MenuAction::Transcode,
                            "Tune Parameters & Preview...",
                            style,
                            palette.accent,
                        )
                        .clicked()
                        {
                            if let Some(first) = sel_files.first() {
                                let first_fmt = sectors.first().unwrap_or(&"PNG").to_string();
                                self.tweak_dialog =
                                    crate::gui::tweak_dialog::TweakDialogState::open_for_file(
                                        first, &first_fmt,
                                    );
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
                        !self.cached_display_indices.is_empty(),
                    )
                    .clicked()
                    {
                        self.select_all();
                        self.selection_anchor = None;
                    }

                    if has_sel {
                        // Chip budget: space between Clear's left edge and the
                        // left button group's right extent.
                        let clear_left = ui.min_rect().min.x;
                        let budget = clear_left - buttons_right - 12.0;
                        let bytes: u64 = self
                            .cached_display_indices
                            .iter()
                            .filter_map(|&idx| self.entries.get(idx))
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
            || self.show_new_tag_dialog
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
                                    self.paint_item_icon(ui, &item, mglyph, mrect, maccent);
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
        self.ensure_display_cache();
        let total_items = self.cached_display_indices.len();
        let grid_p = self.grid_transition_spring.value;
        if grid_p < 0.999 {
            ui.set_opacity((0.85 + 0.15 * grid_p).clamp(0.85, 1.0));
        }

        let mut nav_into_dir: Option<PathBuf> = None;
        let peek_to_open: Option<FileItem> = None;
        let mut item_intents: Vec<ItemMenuIntents> = Vec::new();
        let mut canvas_intents = CanvasMenuIntents::default();
        let mut new_hover: Option<(FileItem, Pos2)> = None;
        let mut drop_ops: Vec<DropOp> = Vec::new();
        let mut grid_start_pos: Option<Pos2> = None;
        let mut canvas_dragging = false;
        let icon_pack = self.icon_pack;
        let menu_style = self.menu_style;
        let has_clipboard = !self.clipboard_paths.is_empty();

        // -- Tools & Converters toolbar over the grid --
        self.render_action_toolbar(
            ui,
            palette,
            &[],
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

        // Grid zoom & column reflow
        let zoom_metrics = crate::gui::view_zoom::GridZoomMetrics::for_step(self.grid_zoom);
        let grid_checkboxes = self.checkbox_mode;
        let icon_sz = if (self.grid_zoom_spring.value - zoom_metrics.icon_size).abs() > 0.05 {
            self.grid_zoom_spring.value
        } else {
            zoom_metrics.icon_size
        };
        let gap = 14.0_f32;
        let avail_w = (ui.available_width() - 16.0).max(zoom_metrics.min_tile_width);
        let reflow =
            crate::gui::view_zoom::compute_grid_reflow(avail_w, zoom_metrics.min_tile_width, gap);
        let tile_w = reflow.tile_width;
        let label_font = zoom_metrics.font_size;
        let size_font = (label_font - 1.5).max(9.5);
        let label_zone_h = label_font * 2.6;
        let bottom_chrome = label_zone_h + size_font + 10.0;
        let top_pad = (icon_sz * 0.10).clamp(8.0, 26.0);
        let tile_h = top_pad + icon_sz + bottom_chrome;
        let tile_size = Vec2::new(tile_w, tile_h);
        let cols = reflow.columns.max(1);
        let total_rows = total_items.div_ceil(cols);
        let col_stride = tile_w + reflow.gap;
        let row_stride = tile_h + reflow.gap;

        let ctrl_held = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
        egui::ScrollArea::vertical()
            .enable_scrolling(!ctrl_held)
            .auto_shrink([false; 2])
            .show_rows(ui, row_stride, total_rows, |ui, row_range| {
                ui.spacing_mut().item_spacing = Vec2::new(reflow.gap, reflow.gap);

                // Canvas interact registered FIRST so tiles stay on top for clicks
                let canvas_resp = ui.interact(
                    ui.clip_rect(),
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
                        total_items > 0,
                        self.sort_column,
                        dir_desc,
                        self.view_mode,
                        &self.current_dir,
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

                for row_idx in row_range {
                    let start_pos = row_idx * cols;
                    let end_pos = (start_pos + cols).min(total_items);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(reflow.gap, reflow.gap);

                        for display_pos in start_pos..end_pos {
                            let entry_idx = match self.cached_display_indices.get(display_pos) {
                                Some(&i) => i,
                                None => continue,
                            };
                            let item = match self.entries.get(entry_idx) {
                                Some(it) => it.clone(),
                                None => continue,
                            };
                            let is_checked = self.selected_paths.contains(&item.path);
                            let icon_spec =
                                resolve_file_icon(&item.name, &item.extension, item.is_dir);

                            let (rect, resp) =
                                ui.allocate_exact_size(tile_size, egui::Sense::click_and_drag());
                            if grid_start_pos.is_none() {
                                let col_in_row = display_pos - start_pos;
                                grid_start_pos = Some(Pos2::new(
                                    rect.left() - col_in_row as f32 * col_stride,
                                    rect.top() - row_idx as f32 * row_stride,
                                ));
                            }

                            // Card background on hover or selection (subtle Apple HIG outline, never covers icons)
                            // Selection tint: straight accent alpha, 45 dark / 26 light.
                            if is_checked {
                                crate::gui::squircle::squircle_filled(
                                    ui.painter(),
                                    rect,
                                    Rounding::same(8.0),
                                    Color32::from_rgba_unmultiplied(
                                        palette.accent.r(),
                                        palette.accent.g(),
                                        palette.accent.b(),
                                        if palette.is_dark { 45 } else { 26 },
                                    ),
                                );
                                crate::gui::squircle::squircle_stroke(
                                    ui.painter(),
                                    rect,
                                    Rounding::same(8.0),
                                    Stroke::new(1.2_f32, palette.accent),
                                );
                            } else if resp.hovered() {
                                // Hover tint: straight text alpha, 20 dark / 12 light
                                // (premul with raw rgb drowned the tile content).
                                crate::gui::squircle::squircle_filled(
                                    ui.painter(),
                                    rect,
                                    Rounding::same(8.0),
                                    Color32::from_rgba_unmultiplied(
                                        palette.text_primary.r(),
                                        palette.text_primary.g(),
                                        palette.text_primary.b(),
                                        if palette.is_dark { 20 } else { 12 },
                                    ),
                                );
                                crate::gui::squircle::squircle_stroke(
                                    ui.painter(),
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
                                let ctrl = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
                                let shift = ui.input(|i| i.modifiers.shift);
                                self.handle_item_click_selection_from_display_index(
                                    display_pos,
                                    item.path.clone(),
                                    is_checked,
                                    ctrl || !shift,
                                    shift,
                                );
                            }

                            // Drag source (Explorer rule: selected set or the
                            // single tile) + folder-tile drop targets.
                            let primary_drag = resp.drag_started_by(egui::PointerButton::Primary);
                            let secondary_drag =
                                resp.drag_started_by(egui::PointerButton::Secondary);
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
                                        crate::gui::squircle::squircle_filled(
                                            ui.painter(),
                                            rect,
                                            Rounding::same(8.0),
                                            Color32::from_rgba_unmultiplied(
                                                palette.accent.r(),
                                                palette.accent.g(),
                                                palette.accent.b(),
                                                (a * 70.0) as u8,
                                            ),
                                        );
                                        crate::gui::squircle::squircle_stroke(
                                            ui.painter(),
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
                                if let Some(payload) = resp.dnd_release_payload::<FileDragPayload>()
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
                            let icon_center =
                                Pos2::new(rect.center().x, zone_top + icon_zone_h * 0.5);
                            let icon_box_side = icon_zone_h.min(tile_w - 24.0);

                            // Any previewable file: real thumbnail preview
                            // (lazily decoded, cached at the tile resolution)
                            let thumb = if !item.is_dir
                                && preview_kind(&item.extension) != PreviewKind::None
                            {
                                self.get_or_load_thumbnail(
                                    ui.ctx(),
                                    &item.path,
                                    thumb_max_px(icon_sz),
                                )
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
                                crate::gui::squircle::squircle_filled(
                                    ui.painter(),
                                    img_rect,
                                    Rounding::same(6.0),
                                    Color32::from_rgba_unmultiplied(
                                        palette.border.r(),
                                        palette.border.g(),
                                        palette.border.b(),
                                        40,
                                    ),
                                );
                                ui.painter_at(img_rect).image(
                                    tex.id(),
                                    img_rect,
                                    uv,
                                    Color32::WHITE,
                                );
                                crate::gui::squircle::squircle_stroke(
                                    ui.painter(),
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
                                    200,
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
                                paint_tag_dots(
                                    ui,
                                    &tile_tags,
                                    rect.right() - 14.0,
                                    rect.top() + 14.0,
                                );
                            }

                            let size_y = rect.bottom() - 11.0;
                            let size_text = self.dir_size_text(&item);
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
                                self.trigger_click_feedback();
                                if item.is_dir {
                                    nav_into_dir = Some(item.path.clone());
                                } else {
                                    open_in_default_app(&item.path);
                                }
                            } else if resp.clicked() {
                                self.trigger_click_feedback();
                                self.focused_row = display_pos;
                                let ctrl = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
                                let shift = ui.input(|i| i.modifiers.shift);
                                self.handle_item_click_selection_from_display_index(
                                    display_pos,
                                    item.path.clone(),
                                    is_checked,
                                    ctrl,
                                    shift,
                                );
                            }

                            if resp.hovered() {
                                let tile_anchor =
                                    rect.center() + Vec2::new(0.0, tile_h * 0.5 + 16.0);
                                new_hover = Some((item.clone(), tile_anchor));
                            }

                            // Rich Context Menu on Grid Tile (pure builder -> intents).
                            if resp.secondary_clicked() && !self.selected_paths.contains(&item.path)
                            {
                                self.selected_paths.clear();
                                self.selected_paths.insert(item.path.clone());
                            }
                            resp.context_menu(|menu_ui| {
                                let targets = if self.selected_paths.len() > 1
                                    && self.selected_paths.contains(&item.path)
                                {
                                    self.selected_paths.iter().cloned().collect()
                                } else {
                                    vec![item.path.clone()]
                                };
                                item_intents.clear();
                                item_intents.push(Self::run_item_context_menu(
                                    menu_ui,
                                    icon_pack,
                                    menu_style,
                                    palette.text_secondary,
                                    &item,
                                    &targets,
                                ));
                            });
                        }
                    });
                }
            });

        if total_items == 0 {
            paint_empty_listing(ui, palette, &self.search_query);
        }

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
                crate::gui::squircle::squircle_filled(
                    &mp,
                    mrect,
                    Rounding::same(2.0),
                    Color32::from_rgba_unmultiplied(
                        palette.accent.r(),
                        palette.accent.g(),
                        palette.accent.b(),
                        30,
                    ),
                );
                crate::gui::squircle::squircle_stroke(
                    &mp,
                    mrect,
                    Rounding::same(2.0),
                    Stroke::new(1.0_f32, palette.accent),
                );
                let additive =
                    ui.input(|i| i.modifiers.ctrl || i.modifiers.command || i.modifiers.shift);
                let hits: Vec<PathBuf> = if let Some(start_pos) = grid_start_pos {
                    let grid_left = start_pos.x;
                    let grid_top = start_pos.y;
                    if total_items > 0 && total_rows > 0 && mrect.max.y >= grid_top {
                        let start_rf = (mrect.min.y - grid_top - tile_h) / row_stride;
                        let min_r = if start_rf <= 0.0 {
                            0
                        } else {
                            (start_rf.ceil() as usize).min(total_rows)
                        };
                        let end_rf = (mrect.max.y - grid_top) / row_stride;
                        let max_r =
                            (end_rf.floor().max(0.0) as usize).min(total_rows.saturating_sub(1));

                        let start_cf = (mrect.min.x - grid_left - tile_w) / col_stride;
                        let min_c = if start_cf <= 0.0 {
                            0
                        } else {
                            (start_cf.ceil() as usize).min(cols)
                        };
                        let end_cf = (mrect.max.x - grid_left) / col_stride;
                        let max_c = (end_cf.floor().max(0.0) as usize).min(cols.saturating_sub(1));

                        if min_r <= max_r && min_c <= max_c {
                            let mut collected = Vec::new();
                            for r in min_r..=max_r {
                                for c in min_c..=max_c {
                                    let pos = r * cols + c;
                                    if pos < total_items {
                                        if let Some(&entry_idx) =
                                            self.cached_display_indices.get(pos)
                                        {
                                            if let Some(e) = self.entries.get(entry_idx) {
                                                collected.push(e.path.clone());
                                            }
                                        }
                                    }
                                }
                            }
                            collected
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };
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
        if canvas_intents.new_tag {
            self.show_new_tag_dialog = true;
            self.new_tag_buffer.clear();
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
            self.select_all();
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
            self.invalidate_display_cache();
        }
        if let Some(vm) = canvas_intents.view_mode {
            self.view_mode = vm;
        }
        if canvas_intents.invert_selection {
            let listed: std::collections::HashSet<PathBuf> = self
                .cached_display_indices
                .iter()
                .filter_map(|&idx| self.entries.get(idx).map(|e| e.path.clone()))
                .collect();
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
        if canvas_intents.preferences {
            self.requested_open_preferences = true;
        }
        if let Some((session_id, cmd_id)) = canvas_intents.invoke_shell_cmd {
            crate::platform::invoke_shell_command(session_id, cmd_id, (0, 0));
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
        ) || (self.sidebar_hover_active && self.hover_preview.is_some());
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
            self.sidebar_hover_active = false;
            self.hover_preview = Some(HoverPreviewState {
                item,
                anchor,
                started_at: Instant::now(),
                content: HoverContent::Meta,
            });
        } else if !self.sidebar_hover_active {
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
        icon_pack: crate::core::icon_packs::IconPack,
        menu_style: crate::core::config::MenuStyle,
        chain_highlight: Option<&Path>,
        stack_index: usize,
        subfolder_to_load: &mut Option<(usize, PathBuf)>,
        nav_into_dir: &mut Option<PathBuf>,
        preview_to_set: &mut Option<FileItem>,
        column_drop: &mut Option<DropOp>,
        new_hover: &mut Option<(FileItem, Pos2)>,
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
                                        let is_open_in_next = match role {
                                            ColumnRole::Current => self
                                                .column_stack
                                                .first()
                                                .map(|(p, _)| p == &item.path)
                                                .unwrap_or(false),
                                            ColumnRole::Descendant => self
                                                .column_stack
                                                .get(stack_index + 1)
                                                .map(|(p, _)| p == &item.path)
                                                .unwrap_or(false),
                                            ColumnRole::Ancestor => {
                                                chain_highlight == Some(item.path.as_path())
                                            }
                                        };
                                        let is_selected = self.selected_paths.contains(&item.path)
                                            || (role == ColumnRole::Current
                                                && self.focused_row == idx);
                                        let is_highlighted = is_selected || is_open_in_next;
                                        let icon_spec = resolve_file_icon(
                                            &item.name,
                                            &item.extension,
                                            item.is_dir,
                                        );

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
                                                    crate::gui::squircle::squircle_filled(ui.painter(),
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
                                        if is_highlighted {
                                            crate::gui::squircle::squircle_filled(ui.painter(),
                                                row_rect,
                                                Rounding::same(6.0),
                                                palette.selection,
                                            );
                                            crate::gui::squircle::squircle_stroke(ui.painter(),
                                                row_rect,
                                                Rounding::same(6.0),
                                                Stroke::new(1.5_f32, palette.accent),
                                            );
                                            let indicator = Rect::from_min_max(
                                                Pos2::new(row_rect.left() + 1.5, row_rect.top() + 4.0),
                                                Pos2::new(row_rect.left() + 4.5, row_rect.bottom() - 4.0),
                                            );
                                            crate::gui::squircle::squircle_filled(ui.painter(),  indicator, Rounding::same(1.5), palette.accent);
                                        } else if resp.hovered() {
                                            crate::gui::squircle::squircle_filled(ui.painter(),
                                                row_rect,
                                                Rounding::same(6.0),
                                                palette.hover,
                                            );
                                        }

                                        if resp.hovered() {
                                            *new_hover = Some((item.clone(), row_rect.right_top() + Vec2::new(10.0, 0.0)));
                                            resp.clone().on_hover_text(format!("{} ({})", item.name, if item.is_dir { "Folder" } else { &item.extension }));
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
                                        self.paint_item_icon(
                                            ui, item, icon_glyph, icon_rect, accent_rgb,
                                        );
                                        if item.is_symlink {
                                            paint_alias_badge(ui.painter(), icon_rect);
                                        }
                                        // Name text right of icon, tag dots before it
                                        let name_color = if is_highlighted {
                                            palette.text_primary
                                        } else if item.is_hidden {
                                            palette.text_muted
                                        } else {
                                            palette.text_primary
                                        };
                                        let col_tags =
                                            crate::core::tags::read_tags_cache(&item.path);
                                        let col_dots_w = tag_dots_width(&col_tags);
                                        let col_name_x = icon_rect.right() + 6.0 + col_dots_w;
                                        let right_reserved = if item.is_dir { 24.0 } else { 12.0 };
                                        let avail_text_w =
                                            (row_rect.right() - col_name_x - right_reserved).max(20.0);
                                        let font_id = egui::FontId::proportional(13.5);
                                        let display_name = crate::gui::file_menu::elide_to_width(
                                            ui,
                                            &item.name,
                                            avail_text_w,
                                            font_id.clone(),
                                        );
                                        ui.painter().text(
                                            Pos2::new(col_name_x, row_rect.center().y),
                                            egui::Align2::LEFT_CENTER,
                                            &display_name,
                                            font_id,
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
                                            let c = Pos2::new(
                                                row_rect.right() - 10.0,
                                                row_rect.center().y,
                                            );
                                            let s = 3.2_f32;
                                            let ch_col = if is_highlighted {
                                                palette.accent
                                            } else {
                                                palette.text_muted
                                            };
                                            ui.painter().line_segment(
                                                [
                                                    Pos2::new(c.x - s * 0.5, c.y - s),
                                                    Pos2::new(c.x + s * 0.5, c.y),
                                                ],
                                                Stroke::new(1.4_f32, ch_col),
                                            );
                                            ui.painter().line_segment(
                                                [
                                                    Pos2::new(c.x + s * 0.5, c.y),
                                                    Pos2::new(c.x - s * 0.5, c.y + s),
                                                ],
                                                Stroke::new(1.4_f32, ch_col),
                                            );
                                        }
                                        // Role-specific click dispatch
                                        if resp.clicked() {
                                            self.trigger_click_feedback();
                                            match role {
                                                ColumnRole::Ancestor => {
                                                    if !is_open_in_next {
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
                                                    let ctrl = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
                                                    let shift = ui.input(|i| i.modifiers.shift);
                                                    self.handle_item_click_selection(
                                                        idx,
                                                        item.path.clone(),
                                                        is_selected,
                                                        ctrl,
                                                        shift,
                                                        entries,
                                                    );
                                                    if !ctrl && !shift {
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
                                                }
                                                ColumnRole::Descendant => {
                                                    let ctrl = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
                                                    let shift = ui.input(|i| i.modifiers.shift);
                                                    self.handle_item_click_selection(
                                                        idx,
                                                        item.path.clone(),
                                                        is_selected,
                                                        ctrl,
                                                        shift,
                                                        entries,
                                                    );
                                                    if !ctrl && !shift {
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
                                        }
                                        if resp.double_clicked() {
                                            self.trigger_click_feedback();
                                            if item.is_dir {
                                                *nav_into_dir = Some(item.path.clone());
                                            } else {
                                                open_in_default_app(&item.path);
                                            }
                                        }

                                        // Right-click menu (same wiring as list/grid rows).
                                        if resp.secondary_clicked() && !self.selected_paths.contains(&item.path) {
                                            self.selected_paths.clear();
                                            self.selected_paths.insert(item.path.clone());
                                        }
                                        resp.context_menu(|menu_ui| {
                                            let targets = if self.selected_paths.len() > 1
                                                && self.selected_paths.contains(&item.path)
                                            {
                                                self.selected_paths.iter().cloned().collect()
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
        let col_p = self.columns_transition_spring.value;
        if col_p > 0.001 {
            ui.set_opacity((1.0 - col_p * 0.5).clamp(0.0, 1.0));
        }

        let mut nav_into_dir: Option<PathBuf> = None;
        let mut subfolder_to_load: Option<(usize, PathBuf)> = None;
        let mut preview_to_set: Option<FileItem> = None;
        let mut peek_to_open: Option<FileItem> = None;
        let mut column_drop: Option<DropOp> = None;

        // Context-menu intents (same wiring as the list/grid views).
        let mut item_intents: Vec<ItemMenuIntents> = Vec::new();
        let mut canvas_intents = CanvasMenuIntents::default();
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
                    self.needs_miller_scroll_align = true;
                }
            }
        }

        let full_h = ui.available_height().max(320.0);
        let mut new_hover: Option<(FileItem, Pos2)> = None;
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
                        &self.current_dir,
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
                        (avail_w * 0.26).clamp(220.0, 360.0)
                    } else {
                        0.0
                    };
                    // Miller lineage: the navigated directory is the primary first column (Column 0).
                    // Subfolders opened by click form the descendant stack (Columns 1..N).
                    // Double-clicking a folder makes that folder become the first column.
                    let zoom_metrics =
                        crate::gui::view_zoom::MillerZoomMetrics::for_step(self.miller_zoom);
                    let base_col_w =
                        if (self.miller_zoom_spring.value - zoom_metrics.column_width).abs() > 0.05
                        {
                            self.miller_zoom_spring.value
                        } else {
                            zoom_metrics.column_width
                        };
                    let zoom_scale = (base_col_w / 260.0).clamp(0.7, 1.6);
                    let cur_dir = self.current_dir.clone();

                    let compute_col_w = |items: &[FileItem]| -> f32 {
                        let max_len = items
                            .iter()
                            .map(|it| it.name.chars().count())
                            .max()
                            .unwrap_or(8);
                        // Dynamic minimum column width calculated from maximum filename length:
                        // Font 13.5pt avg char width is ~7.8px. Non-text overhead (icon, paddings, tag dots, chevron/scrollbar) is ~84px.
                        let content_w = ((max_len as f32 * 7.8) + 84.0) * zoom_scale;
                        // Bound column width dynamically with respect to Mossambi window size:
                        // A column takes at most 40% of available window width (clamped between 220px and 480px).
                        let max_col_w = (avail_w * 0.40).clamp(220.0, 480.0);
                        // Minimum column width starts at 140px, scaling up with respect to the maximum file length in the folder.
                        let min_col_w = 140.0_f32.min(max_col_w);
                        content_w.clamp(min_col_w, max_col_w)
                    };
                    let col_0_w = compute_col_w(&entries);

                    // ==========================================
                    // PRIMARY COLUMN (Column 0): Current Directory
                    // ==========================================
                    {
                        self.render_miller_column(
                            ui,
                            palette,
                            cur_dir.as_path(),
                            &entries,
                            ColumnRole::Current,
                            col_0_w,
                            full_h,
                            icon_pack,
                            menu_style,
                            None,
                            0,
                            &mut subfolder_to_load,
                            &mut nav_into_dir,
                            &mut preview_to_set,
                            &mut column_drop,
                            &mut new_hover,
                            &mut item_intents,
                        );
                    }

                    // ==========================================
                    // MILLER STACK: opened descendant columns (kept
                    // right of the current directory column)
                    // ==========================================
                    let stack_clone: Vec<(PathBuf, Vec<FileItem>)> = self.column_stack.clone();
                    for (si, (sub_path, sub_items)) in stack_clone.iter().enumerate() {
                        let sub_col_w = compute_col_w(sub_items);
                        self.render_miller_column(
                            ui,
                            palette,
                            sub_path,
                            sub_items,
                            ColumnRole::Descendant,
                            sub_col_w,
                            full_h,
                            icon_pack,
                            menu_style,
                            None,
                            si,
                            &mut subfolder_to_load,
                            &mut nav_into_dir,
                            &mut preview_to_set,
                            &mut column_drop,
                            &mut new_hover,
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
            self.needs_miller_scroll_align = true;
            ui.ctx().request_repaint();
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

        let keep_existing = matches!(
            (&self.hover_preview, &new_hover),
            (Some(active), Some((item, _))) if active.item.path == item.path
        ) || (self.sidebar_hover_active && self.hover_preview.is_some());
        if keep_existing {
            let due = self
                .hover_preview
                .as_ref()
                .map(|h| h.started_at.elapsed() > std::time::Duration::from_millis(350))
                .unwrap_or(false);
            if due {
                self.ensure_hover_content(ui.ctx());
            }
        } else if let Some((item, anchor)) = new_hover {
            self.sidebar_hover_active = false;
            self.hover_preview = Some(HoverPreviewState {
                item,
                anchor,
                started_at: Instant::now(),
                content: HoverContent::Meta,
            });
        } else if !self.sidebar_hover_active {
            self.hover_preview = None;
        }
        self.render_hover_preview(ui, palette);

        // Context-menu intents (mirrors the list view consumption).
        if canvas_intents.new_folder {
            self.show_new_folder_dialog = true;
        }
        if canvas_intents.new_tag {
            self.show_new_tag_dialog = true;
            self.new_tag_buffer.clear();
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
            self.select_all();
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
            self.invalidate_display_cache();
        }
        if let Some(vm) = canvas_intents.view_mode {
            self.view_mode = vm;
        }
        if canvas_intents.invert_selection {
            let listed: std::collections::HashSet<PathBuf> = self
                .cached_display_indices
                .iter()
                .filter_map(|&idx| self.entries.get(idx).map(|e| e.path.clone()))
                .collect();
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
        if canvas_intents.preferences {
            self.requested_open_preferences = true;
        }
        if let Some((session_id, cmd_id)) = canvas_intents.invoke_shell_cmd {
            crate::platform::invoke_shell_command(session_id, cmd_id, (0, 0));
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

    fn render_inspector_action_button(
        &self,
        ui: &mut Ui,
        palette: &ThemePalette,
        glyph: crate::core::icon_packs::IconGlyph,
        label: &str,
        primary: bool,
        width: f32,
    ) -> egui::Response {
        let height = if primary { 32.0 } else { 28.0 };
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::click());

        let is_hovered = resp.hovered();
        let is_active = resp.is_pointer_button_down_on();

        let (bg_col, stroke, fg_col) = if primary {
            let base = palette.accent;
            let fg = crate::gui::design::contrast::text_on_accent(base);
            if is_active {
                (base.linear_multiply(0.85), Stroke::NONE, fg)
            } else if is_hovered {
                (base.linear_multiply(1.1), Stroke::NONE, fg)
            } else {
                (base, Stroke::NONE, fg)
            }
        } else {
            let is_dark = palette.is_dark;
            let (bg, st_col) = if is_active {
                (
                    if is_dark {
                        Color32::from_white_alpha(36)
                    } else {
                        Color32::from_black_alpha(26)
                    },
                    palette.accent,
                )
            } else if is_hovered {
                (
                    if is_dark {
                        Color32::from_white_alpha(22)
                    } else {
                        Color32::from_black_alpha(16)
                    },
                    palette.border,
                )
            } else {
                (
                    if is_dark {
                        Color32::from_white_alpha(10)
                    } else {
                        Color32::from_black_alpha(8)
                    },
                    palette.border.linear_multiply(0.6),
                )
            };
            let fg = if is_hovered || is_active {
                palette.text_primary
            } else {
                palette.text_secondary
            };
            (bg, Stroke::new(1.0_f32, st_col), fg)
        };

        ui.painter().rect(rect, Rounding::same(6.0), bg_col, stroke);

        // Content: Icon + Label centered
        let icon_sz = if primary { 16.0 } else { 14.0 };
        let gap = 6.0;
        let font = if primary {
            egui::FontId::proportional(12.5)
        } else {
            egui::FontId::proportional(11.5)
        };

        let max_label_w = (width - icon_sz - gap - 16.0).max(10.0);
        let elided = crate::gui::file_menu::elide_to_width(ui, label, max_label_w, font.clone());
        let galley = ui.painter().layout_no_wrap(elided.clone(), font, fg_col);

        let total_content_w = icon_sz + gap + galley.size().x;
        let start_x = (rect.center().x - total_content_w * 0.5).max(rect.left() + 6.0);

        // Paint icon
        let icon_rect = Rect::from_center_size(
            Pos2::new(start_x + icon_sz * 0.5, rect.center().y),
            Vec2::splat(icon_sz),
        );
        let fg_rgb = [fg_col.r(), fg_col.g(), fg_col.b()];
        crate::core::icon_packs::paint_icon(ui, self.icon_pack, glyph, icon_rect, fg_rgb);

        // Paint label
        let text_pos = Pos2::new(
            start_x + icon_sz + gap,
            rect.center().y - galley.size().y * 0.5,
        );
        ui.painter().galley(text_pos, galley, fg_col);

        if resp.hovered() && elided != label {
            resp.on_hover_text(label)
        } else {
            resp
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
                                    let max_name_w = (inspector_w - 32.0).max(60.0);
                                    let elided_name = crate::gui::file_menu::elide_to_width(
                                        ui,
                                        &item_clone.name,
                                        max_name_w,
                                        egui::FontId::proportional(14.0),
                                    );
                                    let name_lbl = ui.label(
                                        egui::RichText::new(&elided_name)
                                            .size(14.0)
                                            .strong()
                                            .color(palette.text_primary),
                                    );
                                    if elided_name != item_clone.name {
                                        name_lbl.on_hover_text(&item_clone.name);
                                    }
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
                                                    ui.add(
                                                        egui::Label::new(
                                                            egui::RichText::new(*line)
                                                                .size(10.5)
                                                                .monospace()
                                                                .color(palette.text_primary),
                                                        )
                                                        .truncate(),
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

                        // File Properties Card
                        ui.add_space(4.0);
                        egui::Frame::none()
                            .fill(if palette.is_dark {
                                Color32::from_white_alpha(10)
                            } else {
                                Color32::from_black_alpha(8)
                            })
                            .stroke(Stroke::new(1.0_f32, palette.border.linear_multiply(0.6)))
                            .rounding(Rounding::same(8.0))
                            .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                            .show(ui, |ui| {
                                egui::Grid::new("inspector_props_grid")
                                    .num_columns(2)
                                    .spacing([12.0, 6.0])
                                    .show(ui, |ui| {
                                        let kind_str = if item_clone.is_dir {
                                            "Folder".to_string()
                                        } else if !icon_spec.category.is_empty()
                                            && !item_clone.extension.is_empty()
                                        {
                                            format!(
                                                "{} ({})",
                                                icon_spec.category,
                                                item_clone.extension.to_uppercase()
                                            )
                                        } else if !item_clone.extension.is_empty() {
                                            item_clone.extension.to_uppercase()
                                        } else {
                                            "Document".to_string()
                                        };

                                        ui.label(
                                            egui::RichText::new("Kind:")
                                                .size(11.5)
                                                .color(palette.text_muted),
                                        );
                                        ui.label(
                                            egui::RichText::new(kind_str)
                                                .size(11.5)
                                                .strong()
                                                .color(palette.text_primary),
                                        );
                                        ui.end_row();

                                        ui.label(
                                            egui::RichText::new("Size:")
                                                .size(11.5)
                                                .color(palette.text_muted),
                                        );
                                        ui.label(
                                            egui::RichText::new(item_clone.formatted_size())
                                                .size(11.5)
                                                .color(palette.text_secondary),
                                        );
                                        ui.end_row();

                                        ui.label(
                                            egui::RichText::new("Created:")
                                                .size(11.5)
                                                .color(palette.text_muted),
                                        );
                                        ui.label(
                                            egui::RichText::new(item_clone.formatted_created())
                                                .size(11.5)
                                                .color(palette.text_secondary),
                                        );
                                        ui.end_row();

                                        ui.label(
                                            egui::RichText::new("Modified:")
                                                .size(11.5)
                                                .color(palette.text_muted),
                                        );
                                        ui.label(
                                            egui::RichText::new(item_clone.formatted_date())
                                                .size(11.5)
                                                .color(palette.text_secondary),
                                        );
                                        ui.end_row();

                                        ui.label(
                                            egui::RichText::new("Permissions:")
                                                .size(11.5)
                                                .color(palette.text_muted),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{} ({})",
                                                item_clone.formatted_permissions(),
                                                item_clone.formatted_octal_permissions()
                                            ))
                                            .size(11.5)
                                            .monospace()
                                            .color(palette.text_secondary),
                                        );
                                        ui.end_row();

                                        if item_clone.is_symlink {
                                            ui.label(
                                                egui::RichText::new("Symlink:")
                                                    .size(11.5)
                                                    .color(palette.text_muted),
                                            );
                                            let target_str = item_clone
                                                .symlink_target
                                                .as_ref()
                                                .map(|p| p.to_string_lossy().to_string())
                                                .unwrap_or_else(|| "unknown".into());
                                            ui.label(
                                                egui::RichText::new(format!("> {}", target_str))
                                                    .size(11.5)
                                                    .color(palette.accent),
                                            );
                                            ui.end_row();
                                        }

                                        if item_clone.is_hidden {
                                            ui.label(
                                                egui::RichText::new("Hidden:")
                                                    .size(11.5)
                                                    .color(palette.text_muted),
                                            );
                                            ui.label(
                                                egui::RichText::new("Yes")
                                                    .size(11.5)
                                                    .color(palette.text_secondary),
                                            );
                                            ui.end_row();
                                        }

                                        // SHA-256 Checksum calculation
                                        ui.label(
                                            egui::RichText::new("SHA-256:")
                                                .size(11.5)
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
                                                    .button(
                                                        egui::RichText::new("Compute").size(10.5),
                                                    )
                                                    .clicked()
                                                {
                                                    let h = crate::codec::calculate_sha256(
                                                        &item_clone.path,
                                                    )
                                                    .unwrap_or_else(|_| "Error".into());
                                                    self.peek_hash_cache =
                                                        Some((item_clone.path.clone(), h));
                                                }
                                            });
                                        } else if let Some((cached_p, hash)) = &self.peek_hash_cache
                                        {
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
                                                )
                                                .on_hover_text(hash);
                                            } else {
                                                let h = crate::codec::calculate_sha256(
                                                    &item_clone.path,
                                                )
                                                .unwrap_or_else(|_| "Error".into());
                                                self.peek_hash_cache =
                                                    Some((item_clone.path.clone(), h));
                                            }
                                        } else if item_clone.size_bytes <= 16 * 1024 * 1024 {
                                            let h =
                                                crate::codec::calculate_sha256(&item_clone.path)
                                                    .unwrap_or_else(|_| "Error".into());
                                            self.peek_hash_cache =
                                                Some((item_clone.path.clone(), h));
                                        } else if ui
                                            .button(egui::RichText::new("Compute").size(10.5))
                                            .clicked()
                                        {
                                            let h =
                                                crate::codec::calculate_sha256(&item_clone.path)
                                                    .unwrap_or_else(|_| "Error".into());
                                            self.peek_hash_cache =
                                                Some((item_clone.path.clone(), h));
                                        }
                                        ui.end_row();
                                    });
                            });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);

                        // Quick Action Buttons
                        let full_w = ui.available_width();
                        if self
                            .render_inspector_action_button(
                                ui,
                                palette,
                                crate::core::icon_packs::IconGlyph::ActionEye,
                                "Quick Look (Space)",
                                true,
                                full_w,
                            )
                            .clicked()
                        {
                            *peek_to_open = Some(item_clone.clone());
                        }

                        ui.add_space(6.0);
                        let is_wide = full_w >= 290.0;
                        let sec_btn_w = if is_wide {
                            ((full_w - 6.0) * 0.5).floor()
                        } else {
                            full_w
                        };

                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);

                            if !item_clone.is_dir {
                                if self
                                    .render_inspector_action_button(
                                        ui,
                                        palette,
                                        crate::core::icon_packs::IconGlyph::ActionRefresh,
                                        "Transcode / Convert",
                                        false,
                                        sec_btn_w,
                                    )
                                    .clicked()
                                {
                                    on_transcode_target(item_clone.path.clone(), "ALL".into());
                                }

                                if icon_spec.is_image
                                    && self
                                        .render_inspector_action_button(
                                            ui,
                                            palette,
                                            crate::core::icon_packs::IconGlyph::ActionRotate,
                                            "Rotate 90 deg",
                                            false,
                                            sec_btn_w,
                                        )
                                        .clicked()
                                {
                                    if let Ok(()) = rotate_image_file_90(&item_clone.path) {
                                        self.thumb_cache.remove(&item_clone.path);
                                    }
                                }
                            }

                            if self
                                .render_inspector_action_button(
                                    ui,
                                    palette,
                                    crate::core::icon_packs::IconGlyph::ActionCompress,
                                    "Pack into ZIP",
                                    false,
                                    sec_btn_w,
                                )
                                .clicked()
                            {
                                on_pack_archive(vec![item_clone.path.clone()]);
                            }

                            if self
                                .render_inspector_action_button(
                                    ui,
                                    palette,
                                    crate::core::icon_packs::IconGlyph::ActionFolderOpen,
                                    "Reveal in Explorer",
                                    false,
                                    sec_btn_w,
                                )
                                .clicked()
                            {
                                reveal_in_file_manager(&item_clone.path);
                            }

                            if self
                                .render_inspector_action_button(
                                    ui,
                                    palette,
                                    crate::core::icon_packs::IconGlyph::ActionCopy,
                                    "Copy Full Path",
                                    false,
                                    sec_btn_w,
                                )
                                .clicked()
                            {
                                ui.ctx().output_mut(|o| {
                                    o.copied_text = item_clone.path.to_string_lossy().to_string();
                                });
                            }

                            let terminal_target = if item_clone.is_dir {
                                &item_clone.path
                            } else {
                                item_clone.path.parent().unwrap_or(&item_clone.path)
                            };
                            if self
                                .render_inspector_action_button(
                                    ui,
                                    palette,
                                    crate::core::icon_packs::IconGlyph::ActionTerminal,
                                    "Open Terminal",
                                    false,
                                    sec_btn_w,
                                )
                                .clicked()
                            {
                                open_terminal_in_dir(terminal_target);
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
            self.paint_item_icon(ui, item, glyph, irect, accent);
            ui.add_space(4.0);
            let max_name_w = (ui.available_width() - 20.0).max(60.0);
            let elided_name = crate::gui::file_menu::elide_to_width(
                ui,
                &item.name,
                max_name_w,
                egui::FontId::proportional(14.0),
            );
            let name_lbl = ui.label(
                egui::RichText::new(&elided_name)
                    .size(14.0)
                    .strong()
                    .color(palette.text_primary),
            );
            if elided_name != item.name {
                name_lbl.on_hover_text(&item.name);
            }
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
                                let prefix = if item.is_dir { "DIR   " } else { "      " };
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
                                let prefix = if item.is_dir { "DIR   " } else { "      " };
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
            self.fs_op_started = Some(Instant::now());
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
            self.fs_op_started = Some(Instant::now());
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
        match self.dir_sizes.peek(&item.path) {
            Some((mtime, total)) if item.modified == *mtime => format_file_size(*total),
            _ => "--".to_string(),
        }
    }

    /// Byte count used for Size sorting and size filters. Files use their own
    /// length; directories use the fresh recursive total from the size cache
    /// (falling back to metadata length until it arrives). This makes the sort
    /// order and size filters agree with the size actually shown in the column.
    fn effective_size(&self, item: &FileItem) -> u64 {
        if !item.is_dir {
            return item.size_bytes;
        }
        match self.dir_sizes.peek(&item.path) {
            Some((mtime, total)) if item.modified == *mtime => *total,
            _ => item.size_bytes,
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
                        match self.dir_sizes.peek(&e.path) {
                            Some((mtime, total)) if e.modified == *mtime => *total,
                            _ => 0,
                        }
                    } else {
                        e.size_bytes
                    }
                })
                .sum();
        } else {
            total_items = self.filtered_and_sorted_indices().len();
            total_hidden = self.entries.iter().filter(|e| e.is_hidden).count();
            sel_count = self.selected_paths.len();
            total_selected_bytes = self
                .entries
                .iter()
                .filter(|e| self.selected_paths.contains(&e.path))
                .map(|e| {
                    if e.is_dir {
                        match self.dir_sizes.peek(&e.path) {
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
}

// ---------------------------------------------------------------------------
// File Info & Properties Inspection Helpers (Minimal & Detail-Rich)
// ---------------------------------------------------------------------------

fn format_number_commas(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*ch);
    }
    out
}

fn mime_for_ext(ext: &str) -> &'static str {
    match ext.to_ascii_uppercase().as_str() {
        "PNG" => "image/png",
        "JPG" | "JPEG" => "image/jpeg",
        "GIF" => "image/gif",
        "WEBP" => "image/webp",
        "SVG" => "image/svg+xml",
        "PDF" => "application/pdf",
        "RS" => "text/x-rust",
        "PY" => "text/x-python",
        "JS" => "application/javascript",
        "TS" => "application/typescript",
        "JSON" => "application/json",
        "TOML" => "text/x-toml",
        "MD" | "TXT" => "text/plain",
        "MP4" => "video/mp4",
        "MKV" => "video/x-matroska",
        "MOV" => "video/quicktime",
        "MP3" => "audio/mpeg",
        "FLAC" => "audio/flac",
        "WAV" => "audio/wav",
        "ZIP" => "application/zip",
        "TAR" => "application/x-tar",
        "GZ" => "application/gzip",
        _ => "application/octet-stream",
    }
}

fn compute_file_sha256(path: &Path) -> Option<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;

    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let bytes_read = file.read(&mut buffer).ok()?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    let hash = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in hash {
        use std::fmt::Write;
        let _ = write!(&mut hex, "{:02x}", byte);
    }
    Some(hex)
}

fn format_system_time_short(st: std::time::SystemTime) -> String {
    let dt: chrono::DateTime<chrono::Local> = st.into();
    dt.format("%d %b %Y, %H:%M").to_string()
}

fn format_system_time_full(st: std::time::SystemTime) -> String {
    let dt: chrono::DateTime<chrono::Local> = st.into();
    dt.format("%Y-%m-%d %H:%M:%S (%Z)").to_string()
}

// +2px legibility bump for file-info panel labels (user request).
const INFO_CAPTION_PX: f32 = design::type_scale::MICRO + 2.0;
const INFO_LABEL_PX: f32 = design::type_scale::BODY_SM + 2.0;

fn render_storage_visual(ui: &mut Ui, palette: &ThemePalette, logical_bytes: u64) {
    let cluster_size = 4096u64;
    let on_disk_bytes = if logical_bytes > 0 {
        logical_bytes.div_ceil(cluster_size) * cluster_size
    } else {
        0
    };
    let efficiency = if on_disk_bytes > 0 {
        ((logical_bytes as f64 / on_disk_bytes as f64) * 100.0).min(100.0)
    } else {
        100.0
    };

    let avail_w = ui.available_width();
    let (card_rect, resp) = ui.allocate_exact_size(Vec2::new(avail_w, 64.0), egui::Sense::hover());
    crate::gui::squircle::squircle_filled(
        ui.painter(),
        card_rect,
        Rounding::same(design::radius::SM),
        palette.surface_2,
    );
    crate::gui::squircle::squircle_stroke(
        ui.painter(),
        card_rect,
        Rounding::same(design::radius::SM),
        Stroke::new(1.0_f32, palette.border),
    );

    let col_w = (card_rect.width() - 16.0) * 0.5;
    // Left column: Logical Size
    let left_top = egui::pos2(card_rect.left() + 10.0, card_rect.top() + 8.0);
    ui.painter().text(
        left_top,
        egui::Align2::LEFT_TOP,
        "Logical Size",
        egui::FontId::proportional(INFO_CAPTION_PX),
        palette.text_muted,
    );
    let left_val = egui::pos2(card_rect.left() + 10.0, card_rect.top() + 22.0);
    ui.painter().text(
        left_val,
        egui::Align2::LEFT_TOP,
        format_file_size(logical_bytes),
        egui::FontId::proportional(design::type_scale::BODY),
        palette.text_primary,
    );
    let left_sub = egui::pos2(card_rect.left() + 10.0, card_rect.top() + 42.0);
    ui.painter().text(
        left_sub,
        egui::Align2::LEFT_TOP,
        format!("{} bytes", format_number_commas(logical_bytes)),
        egui::FontId::monospace(12.0),
        palette.text_secondary,
    );

    // Right column: On-Disk Allocation
    let right_top = egui::pos2(card_rect.left() + col_w + 16.0, card_rect.top() + 8.0);
    ui.painter().text(
        right_top,
        egui::Align2::LEFT_TOP,
        "On-Disk Allocation",
        egui::FontId::proportional(INFO_CAPTION_PX),
        palette.text_muted,
    );
    let right_val = egui::pos2(card_rect.left() + col_w + 16.0, card_rect.top() + 22.0);
    ui.painter().text(
        right_val,
        egui::Align2::LEFT_TOP,
        format_file_size(on_disk_bytes),
        egui::FontId::proportional(design::type_scale::BODY),
        palette.text_primary,
    );
    let right_sub = egui::pos2(card_rect.left() + col_w + 16.0, card_rect.top() + 42.0);
    ui.painter().text(
        right_sub,
        egui::Align2::LEFT_TOP,
        format!("{:.1}% cluster efficiency", efficiency),
        egui::FontId::proportional(INFO_CAPTION_PX),
        palette.accent,
    );

    resp.on_hover_text(format!(
        "Storage breakdown:\n- Logical data size: {} bytes\n- Disk allocated: {} bytes (in 4KB filesystem clusters)\n- Storage packing efficiency: {:.2}%",
        format_number_commas(logical_bytes),
        format_number_commas(on_disk_bytes),
        efficiency
    ));
}

fn render_parent_folder_bar(
    ui: &mut Ui,
    palette: &ThemePalette,
    item_bytes: u64,
    parent_total_bytes: u64,
) {
    let total = parent_total_bytes.max(item_bytes).max(1);
    let ratio = (item_bytes as f32 / total as f32).clamp(0.001, 1.0);
    let pct = ratio * 100.0;

    let avail_w = ui.available_width();
    let bar_h = 8.0_f32;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(avail_w, bar_h), egui::Sense::hover());

    crate::gui::squircle::squircle_filled(
        ui.painter(),
        rect,
        Rounding::same(4.0),
        palette.surface_2,
    );
    let filled_w = (rect.width() * ratio).max(4.0);
    let filled_rect = Rect::from_min_size(rect.min, Vec2::new(filled_w, bar_h));
    crate::gui::squircle::squircle_filled(
        ui.painter(),
        filled_rect,
        Rounding::same(4.0),
        palette.accent,
    );

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!(
                "{:.1}% of parent folder contents ({})",
                pct,
                format_file_size(total)
            ))
            .size(INFO_CAPTION_PX)
            .color(palette.text_secondary),
        );
    });

    resp.on_hover_text(format!(
        "Proportion of parent folder:\n- This item: {} bytes ({})\n- Total folder items: {} bytes ({})\n- Ratio: {:.2}%",
        format_number_commas(item_bytes),
        format_file_size(item_bytes),
        format_number_commas(total),
        format_file_size(total),
        pct
    ));
}

fn render_timeline_dots(
    ui: &mut Ui,
    palette: &ThemePalette,
    created: Option<std::time::SystemTime>,
    modified: Option<std::time::SystemTime>,
    accessed: Option<std::time::SystemTime>,
) {
    let events = [
        ("Created", created),
        ("Modified", modified),
        ("Accessed", accessed),
    ];

    ui.vertical(|ui| {
        for (i, (label, time_opt)) in events.iter().enumerate() {
            ui.horizontal(|ui| {
                // Circular timeline node
                let (node_rect, node_resp) =
                    ui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover());
                let is_mod = *label == "Modified";
                let dot_color = if is_mod {
                    palette.accent
                } else {
                    palette.text_secondary
                };
                ui.painter()
                    .circle_filled(node_rect.center(), 3.5, dot_color);
                ui.painter().circle_stroke(
                    node_rect.center(),
                    4.5,
                    Stroke::new(1.0_f32, palette.border),
                );

                let (short_text, full_text) = if let Some(t) = time_opt {
                    (format_system_time_short(*t), format_system_time_full(*t))
                } else {
                    ("--".to_string(), "Timestamp not available".to_string())
                };

                let resp = ui.label(
                    egui::RichText::new(format!("{}: {}", label, short_text))
                        .size(INFO_LABEL_PX)
                        .color(if is_mod {
                            palette.text_primary
                        } else {
                            palette.text_secondary
                        }),
                );

                resp.on_hover_text(format!("{} exact timestamp:\n{}", label, full_text));
                node_resp.on_hover_text(format!("{} exact timestamp:\n{}", label, full_text));
            });

            // Connector line between nodes
            if i < events.len() - 1 {
                let (line_rect, _) =
                    ui.allocate_exact_size(Vec2::new(12.0, 6.0), egui::Sense::hover());
                ui.painter().line_segment(
                    [
                        egui::pos2(line_rect.center().x, line_rect.top()),
                        egui::pos2(line_rect.center().x, line_rect.bottom()),
                    ],
                    Stroke::new(1.5_f32, palette.border),
                );
            }
        }
    });
}

fn render_permission_chips(
    ui: &mut Ui,
    palette: &ThemePalette,
    meta: Option<&std::fs::Metadata>,
    octal_str: &str,
) {
    let is_readonly = meta.map(|m| m.permissions().readonly()).unwrap_or(false);

    let chips = [
        ("Read", "Allowed", true),
        (
            "Write",
            if is_readonly { "Read-Only" } else { "Allowed" },
            !is_readonly,
        ),
        ("POSIX", octal_str, false),
        ("Owner", "Current User", false),
        ("Access", "Private", false),
    ];

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
        for (category, value, is_good) in chips {
            let label = format!("{}: {}", category, value);
            let chip_w = (label.len() as f32 * 7.0 + 16.0).max(64.0);
            let (rect, resp) =
                ui.allocate_exact_size(Vec2::new(chip_w, 22.0), egui::Sense::hover());
            let fill = if is_good {
                Color32::from_rgba_unmultiplied(34, 197, 94, if palette.is_dark { 25 } else { 35 })
            } else {
                palette.surface_2
            };
            crate::gui::squircle::squircle_filled(
                ui.painter(),
                rect,
                Rounding::same(design::radius::PILL),
                fill,
            );
            crate::gui::squircle::squircle_stroke(
                ui.painter(),
                rect,
                Rounding::same(design::radius::PILL),
                Stroke::new(
                    1.0_f32,
                    if is_good {
                        Color32::from_rgb(34, 197, 94)
                    } else {
                        palette.border
                    },
                ),
            );
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                &label,
                egui::FontId::proportional(INFO_CAPTION_PX),
                palette.text_primary,
            );
            resp.on_hover_text(format!("Filesystem access permission: {}", label));
        }
    });
}

fn render_open_with_section(ui: &mut Ui, palette: &ThemePalette, extension: &str) {
    let app_opt = crate::core::shell_icons::get_default_app_for_extension(extension);

    let (card_rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 48.0), egui::Sense::hover());
    crate::gui::squircle::squircle_filled(
        ui.painter(),
        card_rect,
        Rounding::same(design::radius::SM),
        palette.surface_2,
    );
    crate::gui::squircle::squircle_stroke(
        ui.painter(),
        card_rect,
        Rounding::same(design::radius::SM),
        Stroke::new(1.0_f32, palette.border),
    );

    if let Some((app_name, exe_path)) = &app_opt {
        // App shell icon
        let icon_rect = Rect::from_center_size(
            egui::pos2(card_rect.left() + 20.0, card_rect.center().y),
            Vec2::splat(28.0),
        );
        let tex_opt =
            crate::core::shell_icons::get_or_request_shell_icon(ui.ctx(), exe_path, false, false);
        if let Some(tex) = tex_opt {
            let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            ui.painter().image(tex.id(), icon_rect, uv, Color32::WHITE);
        } else {
            crate::core::icon_packs::paint_icon(
                ui,
                crate::core::icon_packs::IconPack::Fluent,
                crate::core::icon_packs::IconGlyph::ActionOpen,
                icon_rect,
                [palette.accent.r(), palette.accent.g(), palette.accent.b()],
            );
        }

        let name_pos = egui::pos2(card_rect.left() + 42.0, card_rect.top() + 8.0);
        ui.painter().text(
            name_pos,
            egui::Align2::LEFT_TOP,
            app_name,
            egui::FontId::proportional(INFO_LABEL_PX),
            palette.text_primary,
        );

        let path_pos = egui::pos2(card_rect.left() + 42.0, card_rect.top() + 26.0);
        let exe_display = exe_path.to_string_lossy();
        let short_exe = if exe_display.len() > 42 {
            format!("...{}", &exe_display[exe_display.len() - 39..])
        } else {
            exe_display.to_string()
        };
        ui.painter().text(
            path_pos,
            egui::Align2::LEFT_TOP,
            short_exe,
            egui::FontId::proportional(INFO_CAPTION_PX),
            palette.text_muted,
        );

        resp.on_hover_text(format!(
            "Default application for .{} files:\n{}\nPath: {}",
            extension,
            app_name,
            exe_path.display()
        ));
    } else {
        let label_pos = egui::pos2(card_rect.left() + 16.0, card_rect.center().y - 8.0);
        ui.painter().text(
            label_pos,
            egui::Align2::LEFT_CENTER,
            "System default registered application",
            egui::FontId::proportional(INFO_LABEL_PX),
            palette.text_secondary,
        );
        resp.on_hover_text(format!("System default handler for .{} files", extension));
    }
}

impl ExplorerView {
    fn render_properties_modal(&mut self, ctx: &egui::Context, palette: &ThemePalette) {
        let mut item = match &self.properties_item {
            Some(i) => i.clone(),
            None => return,
        };

        if self.properties_rename_buf.is_empty() {
            self.properties_rename_buf = item.name.clone();
        }

        let mut close_modal = false;
        let mut reveal_path: Option<PathBuf> = None;
        let mut peek_item: Option<FileItem> = None;
        let mut rename_to: Option<String> = None;

        let icon_spec = resolve_file_icon(&item.name, &item.extension, item.is_dir);

        let default_x = ((ctx.screen_rect().width() - 360.0) * 0.5).max(20.0);
        let default_y = ((ctx.screen_rect().height() - 520.0) * 0.5).max(20.0);
        egui::Window::new(format!("Info - {}", item.name))
            .id(egui::Id::new("mossambi_file_get_info_modal"))
            .collapsible(false)
            .resizable(false)
            .fixed_size(Vec2::new(360.0, 520.0))
            .default_pos(egui::pos2(default_x, default_y))
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(palette.card_bg)
                    .stroke(Stroke::new(1.0_f32, palette.border))
                    .rounding(Rounding::same(16.0))
                    .inner_margin(egui::Margin::same(14.0)),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().scroll.bar_width = 4.0;
                ui.spacing_mut().scroll.floating = true;
                egui::ScrollArea::vertical()
                    .max_height(460.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // 1. Hero Identity Header (64x64 Preview / Shell Icon, Editable Name, Type Badge, Formatted Size)
                        ui.horizontal(|ui| {
                            let (picon_rect, picon_resp) =
                                ui.allocate_exact_size(Vec2::splat(64.0), egui::Sense::hover());

                            let thumb_opt = if icon_spec.is_image {
                                load_image_thumbnail(ctx, &item.path)
                            } else {
                                None
                            };

                            if let Some(tex) = thumb_opt {
                                let uv =
                                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                                ui.painter().image(tex.id(), picon_rect, uv, Color32::WHITE);
                            } else {
                                let (pglyph, paccent) =
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
                                self.paint_item_icon(ui, &item, pglyph, picon_rect, paccent);
                            }

                            picon_resp.on_hover_text(format!(
                                "{}\nPath: {}\nType: {}",
                                item.name,
                                item.path.display(),
                                if item.is_dir {
                                    "Folder"
                                } else {
                                    &item.extension
                                }
                            ));

                            ui.add_space(10.0);
                            ui.vertical(|ui| {
                                // Editable file / folder name
                                ui.horizontal(|ui| {
                                    let edit_resp = ui.add(
                                        egui::TextEdit::singleline(&mut self.properties_rename_buf)
                                            .desired_width(260.0)
                                            .font(egui::FontId::proportional(
                                                design::type_scale::BODY_LG,
                                            )),
                                    );
                                    if edit_resp.lost_focus()
                                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                    {
                                        rename_to =
                                            Some(self.properties_rename_buf.trim().to_string());
                                    }
                                    if ui.small_button("Rename").clicked() {
                                        rename_to =
                                            Some(self.properties_rename_buf.trim().to_string());
                                    }
                                });

                                ui.add_space(2.0);

                                // Type Badge Pill + Size
                                ui.horizontal(|ui| {
                                    let badge_label = if item.is_dir {
                                        "Folder".to_string()
                                    } else {
                                        let ext = item.extension.to_ascii_uppercase();
                                        if ext.is_empty() {
                                            "File".to_string()
                                        } else {
                                            format!("{} File", ext)
                                        }
                                    };

                                    let badge_w = badge_label.len() as f32 * 7.5 + 16.0;
                                    let (badge_rect, _) = ui.allocate_exact_size(
                                        Vec2::new(badge_w, 20.0),
                                        egui::Sense::hover(),
                                    );
                                    crate::gui::squircle::squircle_filled(
                                        ui.painter(),
                                        badge_rect,
                                        Rounding::same(design::radius::PILL),
                                        palette.surface_2,
                                    );
                                    crate::gui::squircle::squircle_stroke(
                                        ui.painter(),
                                        badge_rect,
                                        Rounding::same(design::radius::PILL),
                                        Stroke::new(1.0_f32, palette.border),
                                    );
                                    ui.painter().text(
                                        badge_rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        &badge_label,
                                        egui::FontId::proportional(INFO_CAPTION_PX),
                                        palette.text_primary,
                                    );

                                    ui.add_space(6.0);
                                    let size_lbl = ui.label(
                                        egui::RichText::new(item.formatted_size())
                                            .size(INFO_LABEL_PX)
                                            .color(palette.text_secondary),
                                    );
                                    size_lbl.on_hover_text(format!(
                                        "Exact logical size: {} bytes",
                                        format_number_commas(item.size_bytes)
                                    ));
                                });

                                ui.add_space(2.0);
                                ui.horizontal(|ui| {
                                    let path_str = item.path.to_string_lossy().to_string();
                                    if ui
                                        .small_button("Copy Path")
                                        .on_hover_text(&path_str)
                                        .clicked()
                                    {
                                        ui.output_mut(|o| o.copied_text = path_str);
                                    }
                                    if ui.small_button("Copy Name").clicked() {
                                        ui.output_mut(|o| o.copied_text = item.name.clone());
                                    }
                                });
                            });
                        });

                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(6.0);

                        // 2. Collapsible Section: General
                        egui::CollapsingHeader::new(
                            egui::RichText::new("General")
                                .size(INFO_LABEL_PX)
                                .strong()
                                .color(palette.text_primary),
                        )
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.add_space(4.0);
                            // On-Disk vs Logical Size Split Card
                            render_storage_visual(ui, palette, item.size_bytes);
                            ui.add_space(6.0);

                            // Size Bar vs Parent Folder
                            let parent_total: u64 = self.entries.iter().map(|e| e.size_bytes).sum();
                            render_parent_folder_bar(
                                ui,
                                palette,
                                item.size_bytes,
                                parent_total.max(item.size_bytes),
                            );
                            ui.add_space(6.0);

                            // Location with full path tooltip
                            let parent_str = item
                                .path
                                .parent()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_else(|| "-".to_string());

                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Location:")
                                        .color(palette.text_muted)
                                        .size(INFO_LABEL_PX),
                                );
                                let loc_lbl = ui.label(
                                    egui::RichText::new(truncate_middle(&parent_str, 48))
                                        .size(INFO_LABEL_PX)
                                        .color(palette.text_primary),
                                );
                                loc_lbl.on_hover_text(format!(
                                    "Full parent directory:\n{}",
                                    parent_str
                                ));
                            });

                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new("Timeline")
                                    .size(INFO_CAPTION_PX)
                                    .color(palette.text_muted),
                            );
                            ui.add_space(2.0);
                            let accessed_time = std::fs::metadata(&item.path)
                                .ok()
                                .and_then(|m| m.accessed().ok());
                            // Created / Modified / Accessed Timeline Dots
                            render_timeline_dots(
                                ui,
                                palette,
                                item.created,
                                item.modified,
                                accessed_time,
                            );
                        });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);

                        // 3. Collapsible Section: More Info (Deep Metadata, Checksums)
                        egui::CollapsingHeader::new(
                            egui::RichText::new("More Info")
                                .size(INFO_LABEL_PX)
                                .strong()
                                .color(palette.text_primary),
                        )
                        .default_open(true)
                        .show(ui, |ui| {
                            egui::Grid::new("props_more_info_grid")
                                .num_columns(2)
                                .spacing([12.0, 6.0])
                                .show(ui, |ui| {
                                    if icon_spec.is_image {
                                        ui.label(
                                            egui::RichText::new("Dimensions:")
                                                .color(palette.text_muted),
                                        );
                                        if let Ok((w, h)) = image::image_dimensions(&item.path) {
                                            let mp = (w as f64 * h as f64) / 1_000_000.0;
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{} x {} px ({:.2} MP)",
                                                    w, h, mp
                                                ))
                                                .color(palette.text_primary),
                                            );
                                        } else {
                                            ui.label(
                                                egui::RichText::new("Standard image asset")
                                                    .color(palette.text_secondary),
                                            );
                                        }
                                        ui.end_row();
                                    }

                                    if icon_spec.is_code || icon_spec.is_document {
                                        ui.label(
                                            egui::RichText::new("Content Stats:")
                                                .color(palette.text_muted),
                                        );
                                        if let Ok(text) = std::fs::read_to_string(&item.path) {
                                            let lines = text.lines().count();
                                            let non_empty = text
                                                .lines()
                                                .filter(|l| !l.trim().is_empty())
                                                .count();
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{} lines ({} non-empty)",
                                                    lines, non_empty
                                                ))
                                                .color(palette.text_primary),
                                            );
                                        } else {
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{} document stream",
                                                    item.extension
                                                ))
                                                .color(palette.text_secondary),
                                            );
                                        }
                                        ui.end_row();
                                    }

                                    if item.is_dir {
                                        ui.label(
                                            egui::RichText::new("Folder Items:")
                                                .color(palette.text_muted),
                                        );
                                        if let Ok(entries) = std::fs::read_dir(&item.path) {
                                            let count = entries.count();
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{} immediate items",
                                                    count
                                                ))
                                                .color(palette.text_primary),
                                            );
                                        } else {
                                            ui.label(
                                                egui::RichText::new("Directory container")
                                                    .color(palette.text_secondary),
                                            );
                                        }
                                        ui.end_row();
                                    }

                                    ui.label(
                                        egui::RichText::new("MIME Type:").color(palette.text_muted),
                                    );
                                    ui.label(
                                        egui::RichText::new(mime_for_ext(&item.extension))
                                            .monospace()
                                            .color(palette.text_secondary),
                                    );
                                    ui.end_row();

                                    ui.label(
                                        egui::RichText::new("SHA-256 Checksum:")
                                            .color(palette.text_muted),
                                    );
                                    let has_hash = self
                                        .properties_sha256
                                        .as_ref()
                                        .map(|(p, h)| (p == &item.path, h.clone()));
                                    if let Some((true, hash)) = has_hash {
                                        ui.horizontal(|ui| {
                                            let short_hash = if hash.len() > 16 {
                                                format!(
                                                    "{}...{}",
                                                    &hash[..8],
                                                    &hash[hash.len() - 8..]
                                                )
                                            } else {
                                                hash.clone()
                                            };
                                            ui.label(
                                                egui::RichText::new(short_hash)
                                                    .monospace()
                                                    .color(palette.accent),
                                            );
                                            if ui
                                                .small_button("Copy SHA-256")
                                                .on_hover_text(&hash)
                                                .clicked()
                                            {
                                                ui.output_mut(|o| o.copied_text = hash);
                                            }
                                        });
                                    } else if item.is_dir {
                                        ui.label(
                                            egui::RichText::new("N/A (Directory)")
                                                .color(palette.text_muted),
                                        );
                                    } else if ui.small_button("Compute SHA-256").clicked() {
                                        if let Some(h) = compute_file_sha256(&item.path) {
                                            self.properties_sha256 = Some((item.path.clone(), h));
                                        }
                                    }
                                    ui.end_row();
                                });
                        });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);

                        // 4. Collapsible Section: Permissions & Sharing
                        egui::CollapsingHeader::new(
                            egui::RichText::new("Permissions & Sharing")
                                .size(INFO_LABEL_PX)
                                .strong()
                                .color(palette.text_primary),
                        )
                        .default_open(true)
                        .show(ui, |ui| {
                            let item_meta = std::fs::metadata(&item.path).ok();
                            render_permission_chips(
                                ui,
                                palette,
                                item_meta.as_ref(),
                                &item.formatted_octal_permissions(),
                            );
                        });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);

                        // 5. Collapsible Section: Open With (Official Application Icon + Handler)
                        if !item.is_dir {
                            egui::CollapsingHeader::new(
                                egui::RichText::new("Open With")
                                    .size(INFO_LABEL_PX)
                                    .strong()
                                    .color(palette.text_primary),
                            )
                            .default_open(true)
                            .show(ui, |ui| {
                                render_open_with_section(ui, palette, &item.extension);
                            });
                        }
                    });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                // Action Buttons Footer
                ui.horizontal(|ui| {
                    if ui.button("Reveal in File Explorer").clicked() {
                        reveal_path = item.path.parent().map(|p| p.to_path_buf());
                    }

                    if !item.is_dir && ui.button("Quick Look (Space)").clicked() {
                        peek_item = Some(item.clone());
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            close_modal = true;
                        }
                    });
                });
            });

        // Handle inline rename
        if let Some(new_name) = rename_to {
            if !new_name.is_empty() && new_name != item.name {
                let new_path = item.path.with_file_name(&new_name);
                if std::fs::rename(&item.path, &new_path).is_ok() {
                    crate::core::action_journal::record_rename(item.path.clone(), new_path.clone());
                    item.path = new_path;
                    item.name = new_name;
                    self.properties_item = Some(item.clone());
                    self.properties_rename_buf = item.name.clone();
                    self.navigate_to(self.current_dir.clone());
                }
            }
        }

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

pub(crate) fn open_in_default_app(path: &Path) {
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
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = path;
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
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = path;
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
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = path;
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
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = path;
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

/// A context-menu request deferred from a right-click to a native pop-out
/// viewport. egui draws ordinary context menus as `Area`s clipped to the
/// parent window; rendering in a separate immediate viewport lets the menu
/// escape the window bounds like a real OS menu.
#[derive(Clone)]
pub enum NativeMenuRequest {
    Canvas {
        anchor: Pos2,
        pack: crate::core::icon_packs::IconPack,
        style: crate::core::config::MenuStyle,
        tint: Color32,
        has_clipboard: bool,
        has_entries: bool,
        sort: SortColumn,
        sort_desc: bool,
        view: ViewMode,
        dir: PathBuf,
    },
    Item {
        anchor: Pos2,
        pack: crate::core::icon_packs::IconPack,
        style: crate::core::config::MenuStyle,
        tint: Color32,
        item: FileItem,
        targets: Vec<PathBuf>,
    },
}

impl NativeMenuRequest {
    /// Monitor-space position where the pop-out window should open.
    pub fn anchor(&self) -> Pos2 {
        match self {
            Self::Canvas { anchor, .. } | Self::Item { anchor, .. } => *anchor,
        }
    }
}

/// Intents produced by a native menu viewport, applied on the next frame
/// through the explorer's existing intent-application path.
#[allow(clippy::large_enum_variant)]
pub enum NativeMenuIntents {
    Canvas(CanvasMenuIntents),
    Item(ItemMenuIntents),
}

/// Everything the empty-canvas menu may ask the caller to do.
#[derive(Default, Debug, Clone)]
pub struct CanvasMenuIntents {
    pub new_folder: bool,
    pub new_tag: bool,
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
    pub preferences: bool,
    /// Requested sort column (caller toggles direction when re-clicked).
    pub sort_by: Option<SortColumn>,
    /// Requested view mode (Details/Grid/Columns submenu).
    pub view_mode: Option<ViewMode>,
    /// Invert selection within the current filtered listing.
    pub invert_selection: bool,
    /// Invocation request for native shell context menu verb: (session_id, cmd_id).
    pub invoke_shell_cmd: Option<(u64, u32)>,
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
    pub tweak_transcode: Option<(PathBuf, String)>,
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
    /// Request to open the New Tag dialog.
    pub new_tag: bool,
    /// Invocation request for native shell context menu verb: (session_id, cmd_id).
    pub invoke_shell_cmd: Option<(u64, u32)>,
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
    /// Paints the official icon for a file, directory or app.
    /// Uses real Windows shell icons (SHGetFileInfo) off the UI thread where available,
    /// and falls back to official Microsoft Fluent UI system icons.
    pub fn paint_item_icon(
        &self,
        ui: &Ui,
        item: &FileItem,
        fallback_glyph: IconGlyph,
        rect: Rect,
        fallback_accent: [u8; 3],
    ) {
        let large = rect.width() > 24.0;
        if let Some(tex) = crate::core::shell_icons::get_or_request_shell_icon(
            ui.ctx(),
            &item.path,
            item.is_dir,
            large,
        ) {
            let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            ui.painter().image(tex.id(), rect, uv, Color32::WHITE);
        } else {
            crate::core::icon_packs::paint_icon(
                ui,
                self.icon_pack,
                fallback_glyph,
                rect,
                fallback_accent,
            );
        }
    }

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
        self.fs_op_started = Some(Instant::now());
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
        if let Some((p, fmt)) = intent.tweak_transcode {
            self.tweak_dialog = crate::gui::tweak_dialog::TweakDialogState::open_for_file(&p, &fmt);
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

        if intent.new_tag {
            self.show_new_tag_dialog = true;
            self.new_tag_buffer.clear();
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
                    Err(e) => {
                        log::warn!("Tag failed for {}: {e}", p.display());
                        self.show_status_note(Err(e));
                    }
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
                    Err(e) => {
                        log::warn!("Tag removal failed for {}: {e}", p.display());
                        self.show_status_note(Err(e));
                    }
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

        if let Some((session_id, cmd_id)) = intent.invoke_shell_cmd {
            crate::platform::invoke_shell_command(session_id, cmd_id, (0, 0));
        }
    }

    pub fn paste_from_clipboard(&mut self) {
        let sources: Vec<PathBuf> = self.clipboard_paths.clone();
        if sources.is_empty() {
            return;
        }
        let cut = self.clipboard_is_cut;
        let dest_dir = self.current_dir.clone();
        self.fs_op_started = Some(Instant::now());
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
        current_dir: &Path,
    ) -> CanvasMenuIntents {
        let max_h = 340.0_f32;
        menu_ui.spacing_mut().scroll.bar_width = 4.0;
        menu_ui.spacing_mut().scroll.floating = true;
        egui::ScrollArea::vertical()
            .max_height(max_h)
            .auto_shrink([true, true])
            .show(menu_ui, |menu_ui| {
                menu_ui.set_min_width(240.0);
                menu_ui.set_max_width(240.0);
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
                if menu_row(menu_ui, true, pack, fm::MenuAction::Properties, style, tint).clicked()
                {
                    intent.properties_dir = true;
                    menu_ui.close_menu();
                }
                if menu_row_glyph(
                    menu_ui,
                    true,
                    pack,
                    IconGlyph::Gear,
                    "Preferences (Ctrl+,)",
                    style,
                    tint,
                )
                .clicked()
                {
                    intent.preferences = true;
                    menu_ui.close_menu();
                }

                let shift_held = menu_ui.input(|i| i.modifiers.shift);
                let session_id = crate::platform::ensure_shell_menu_requested(
                    menu_ui.ctx(),
                    &[current_dir.to_path_buf()],
                    true,
                    shift_held,
                    0,
                );
                crate::platform::render_shell_menu_section(
                    menu_ui,
                    session_id,
                    &mut intent.invoke_shell_cmd,
                );
                intent
            })
            .inner
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
        let max_h = 340.0_f32;
        menu_ui.spacing_mut().scroll.bar_width = 4.0;
        menu_ui.spacing_mut().scroll.floating = true;
        egui::ScrollArea::vertical()
            .max_height(max_h)
            .auto_shrink([true, true])
            .show(menu_ui, |menu_ui| {
                menu_ui.set_min_width(240.0);
                menu_ui.set_max_width(240.0);
                let mut intent = ItemMenuIntents::default();
                let is_dir = item.is_dir;
                let path: PathBuf = item.path.clone();

                // Multi-select bookkeeping (Explorer rule).
                let multi = targets.len() > 1;
                let file_targets: Vec<PathBuf> =
                    targets.iter().filter(|p| p.is_file()).cloned().collect();
                let archive_targets: Vec<PathBuf> = targets
                    .iter()
                    .filter(|p| {
                        p.extension()
                            .map(|e| {
                                item_kind(&e.to_string_lossy().to_ascii_uppercase()).is_archive
                            })
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect();

                let header = if multi {
                    pluralize_items(targets.len())
                } else {
                    item.name.clone()
                };
                let icon_spec = resolve_file_icon(&item.name, &item.extension, item.is_dir);
                let (hdr_glyph, _) = crate::core::icon_packs::IconGlyph::resolve_with_path(
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
                fm::menu_header_with_icon(menu_ui, pack, hdr_glyph, &header, tint);
                menu_ui.separator();

                // -- Open group --
                if is_dir {
                    if menu_row(menu_ui, true, pack, fm::MenuAction::OpenFolder, style, tint)
                        .clicked()
                    {
                        intent.nav_into_dir = Some(path.clone());
                        menu_ui.close_menu();
                    }
                    if menu_row(menu_ui, true, pack, fm::MenuAction::NewWindow, style, tint)
                        .clicked()
                    {
                        intent.open_in_new_window = Some(path.clone());
                        menu_ui.close_menu();
                    }
                    if menu_row(menu_ui, true, pack, fm::MenuAction::NewTab, style, tint).clicked()
                    {
                        intent.open_in_new_tab = Some(path.clone());
                        menu_ui.close_menu();
                    }
                } else {
                    if menu_row(menu_ui, true, pack, fm::MenuAction::OpenFile, style, tint)
                        .clicked()
                    {
                        intent.open_default = Some(path.clone());
                        menu_ui.close_menu();
                    }
                    if menu_row(menu_ui, !multi, pack, fm::MenuAction::Edit, style, tint).clicked()
                    {
                        intent.open_in_editor = Some(path.clone());
                        menu_ui.close_menu();
                    }
                    let kind = item_kind(&item.extension);
                    if kind.is_media
                        && menu_row(menu_ui, !multi, pack, fm::MenuAction::Play, style, tint)
                            .clicked()
                    {
                        intent.play = Some(path.clone());
                        menu_ui.close_menu();
                    }
                    if !file_targets.is_empty()
                        && file_targets.iter().any(|p| item_kind_ext(p).is_printable)
                        && menu_row(menu_ui, true, pack, fm::MenuAction::Print, style, tint)
                            .clicked()
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
                        && menu_row(menu_ui, true, pack, fm::MenuAction::Wallpaper, style, tint)
                            .clicked()
                    {
                        intent.wallpaper = Some(path.clone());
                        menu_ui.close_menu();
                    }
                    if kind.is_image
                        && menu_row(menu_ui, true, pack, fm::MenuAction::Rotate, style, tint)
                            .clicked()
                    {
                        intent.rotate = Some(path.clone());
                        menu_ui.close_menu();
                    }
                    // Type-aware Convert submenu - the SAME engine the radial wheel
                    // uses (UniversalRegistry::get_sectors_for_file): every converter
                    // the Studio's left column offers, scoped to this file's type, so
                    // irrelevant formats never appear.
                    let sectors =
                        crate::core::registry::UniversalRegistry::get_sectors_for_file(&path);
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
                                            intent.transcode =
                                                Some((targets.to_vec(), label.to_string()));
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
                                "Tune Parameters & Preview...",
                                style,
                                tint,
                            )
                            .clicked()
                            {
                                let first_fmt =
                                    sectors.first().cloned().unwrap_or("PNG").to_string();
                                intent.tweak_transcode = Some((path.clone(), first_fmt));
                                sub_ui.close_menu();
                            }
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
                    if menu_row(menu_ui, true, pack, fm::MenuAction::Conform, style, tint).clicked()
                    {
                        intent.exam_conform = Some(file_targets.clone());
                        menu_ui.close_menu();
                    }
                    if menu_row(menu_ui, true, pack, fm::MenuAction::Inspect, style, tint).clicked()
                    {
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
                                        intent.open_with_launch =
                                            Some((command.clone(), path.clone()));
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
                                    intent.send_to =
                                        Some((targets.to_vec(), dest.clone(), label.clone()));
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
                            if fm::menu_item_glyph(
                                sub_ui,
                                pack,
                                IconGlyph::ActionPath,
                                label,
                                style,
                                tint,
                            )
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
                    && menu_row(menu_ui, true, pack, fm::MenuAction::Shortcut, style, tint)
                        .clicked()
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
                            c.show_in == wanted
                                || c.show_in == crate::core::config::CustomMenuScope::All
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
                    let custom_tags = crate::core::MossambiConfig::load().custom_tags;
                    fm::submenu(
                        menu_ui,
                        fm::MenuAction::Tags.label(style),
                        pack,
                        style,
                        tint,
                        fm::MenuAction::Tags,
                        |sub_ui| {
                            sub_ui.set_min_width(190.0);
                            for tag_name in crate::core::tags::available_tags_in(&custom_tags) {
                                let is_tagged = current_tags
                                    .iter()
                                    .any(|t| t.eq_ignore_ascii_case(&tag_name));
                                let label = if is_tagged {
                                    format!("Remove {tag_name}")
                                } else {
                                    format!("Add {tag_name}")
                                };
                                if fm::menu_item_tag_dot(
                                    sub_ui,
                                    crate::core::tags::tag_color(&tag_name),
                                    &label,
                                    None,
                                )
                                .clicked()
                                {
                                    if is_tagged {
                                        intent.clear_tag =
                                            Some((targets.to_vec(), tag_name.clone()));
                                    } else {
                                        intent.set_tag = Some((targets.to_vec(), tag_name.clone()));
                                    }
                                    sub_ui.close_menu();
                                }
                            }
                            if fm::menu_item_labeled(
                                sub_ui,
                                pack,
                                fm::MenuAction::Tags,
                                "New Tag...",
                                style,
                                tint,
                            )
                            .clicked()
                            {
                                intent.new_tag = true;
                                sub_ui.close_menu();
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
                } else if menu_row(menu_ui, true, pack, fm::MenuAction::Rename, style, tint)
                    .clicked()
                {
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
                    && menu_row(menu_ui, true, pack, fm::MenuAction::Defender, style, tint)
                        .clicked()
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

                let shift_held = menu_ui.input(|i| i.modifiers.shift);
                let session_id = crate::platform::ensure_shell_menu_requested(
                    menu_ui.ctx(),
                    targets,
                    false,
                    shift_held,
                    0,
                );
                crate::platform::render_shell_menu_section(
                    menu_ui,
                    session_id,
                    &mut intent.invoke_shell_cmd,
                );

                intent
            })
            .inner
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
            crate::gui::squircle::squircle_filled(ui.painter(), rect, rounding, fill);
            crate::gui::squircle::squircle_stroke(
                ui.painter(),
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
            crate::gui::squircle::squircle_filled(ui.painter(), rect, rounding, bg_col);

            let border_alpha = (55.0 * alpha) as u8;
            let border_col = if palette.is_dark {
                Color32::from_rgba_unmultiplied(160, 165, 175, border_alpha)
            } else {
                Color32::from_rgba_unmultiplied(110, 115, 125, border_alpha)
            };
            crate::gui::squircle::squircle_stroke(
                ui.painter(),
                rect,
                rounding,
                Stroke::new(0.85_f32, border_col),
            );
        }
    }
}

// ======= end of explorer_view.rs shared-menu engine =======

/// Tab / window title for a folder: its name, or the drive label for a
/// root such as `C:\` (which has no file name and used to show "Folder").
fn folder_title(dir: &Path) -> String {
    let s = dir.to_string_lossy();
    let trimmed = s.trim_end_matches(['\\', '/']);
    if trimmed.is_empty() {
        return s.into_owned();
    }
    if trimmed.len() == 2 && trimmed.ends_with(':') {
        return trimmed.to_string();
    }
    match trimmed.split(['\\', '/']).rfind(|p| !p.is_empty()) {
        Some(name) => name.to_string(),
        None => trimmed.to_string(),
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

#[cfg(test)]
mod multi_selection_tests {
    use super::*;
    use std::path::PathBuf;

    fn make_test_items(count: usize) -> Vec<FileItem> {
        (0..count)
            .map(|i| FileItem::from_path(PathBuf::from(format!("/test/file_{i}.txt"))))
            .collect()
    }

    #[test]
    fn test_plain_click_selects_single() {
        let mut view = ExplorerView::default();
        let items = make_test_items(10);

        view.handle_item_click_selection(3, items[3].path.clone(), false, false, false, &items);
        assert_eq!(view.selected_paths.len(), 1);
        assert!(view.selected_paths.contains(&items[3].path));
        assert_eq!(view.selection_anchor, Some(3));
    }

    #[test]
    fn test_shift_click_range_selection() {
        let mut view = ExplorerView::default();
        let items = make_test_items(10);

        // Click item 2 (sets anchor to 2)
        view.handle_item_click_selection(2, items[2].path.clone(), false, false, false, &items);
        assert_eq!(view.selected_paths.len(), 1);

        // Shift-click item 5 (selects items 2, 3, 4, 5)
        view.handle_item_click_selection(5, items[5].path.clone(), false, false, true, &items);
        assert_eq!(view.selected_paths.len(), 4);
        for item in items.iter().take(6).skip(2) {
            assert!(view.selected_paths.contains(&item.path));
        }
        // Anchor remains at 2 for re-pivoting
        assert_eq!(view.selection_anchor, Some(2));

        // Re-pivot: Shift-click item 3 (narrows selection to 2, 3)
        view.handle_item_click_selection(3, items[3].path.clone(), true, false, true, &items);
        assert_eq!(view.selected_paths.len(), 2);
        assert!(view.selected_paths.contains(&items[2].path));
        assert!(view.selected_paths.contains(&items[3].path));
        assert!(!view.selected_paths.contains(&items[4].path));
    }

    #[test]
    fn test_ctrl_click_toggle_and_ctrl_shift_union() {
        let mut view = ExplorerView::default();
        let items = make_test_items(10);

        // Plain click item 1
        view.handle_item_click_selection(1, items[1].path.clone(), false, false, false, &items);
        // Ctrl-click item 7 (toggles item 7 on, anchor becomes 7)
        view.handle_item_click_selection(7, items[7].path.clone(), false, true, false, &items);
        assert_eq!(view.selected_paths.len(), 2);
        assert!(view.selected_paths.contains(&items[1].path));
        assert!(view.selected_paths.contains(&items[7].path));
        assert_eq!(view.selection_anchor, Some(7));

        // Ctrl+Shift click item 9 (unions range 7..=9 with existing items without clearing item 1)
        view.handle_item_click_selection(9, items[9].path.clone(), false, true, true, &items);
        assert_eq!(view.selected_paths.len(), 4);
        assert!(view.selected_paths.contains(&items[1].path));
        assert!(view.selected_paths.contains(&items[7].path));
        assert!(view.selected_paths.contains(&items[8].path));
        assert!(view.selected_paths.contains(&items[9].path));
    }
}
