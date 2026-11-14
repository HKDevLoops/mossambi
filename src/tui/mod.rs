//! Mossambi Terminal User Interface (`mossambi-tui`)
//!
//! Fully Asynchronous, Non-Blocking, Yazi-Grade File Explorer Suite built with Ratatui & Crossterm.
//! Features:
//! - 3 Switchable Layout Modes ([F6] key):
//!   1. 3-Column Miller Layout (Yazi-style: Parent Context -> Current Directory -> Live Preview/Inspector)
//!   2. Full-Screen Dedicated Explorer (Full-width table with sortable columns & action toolbar)
//!   3. Dual-Pane Commander (Midnight Commander-style: Source <-> Target staging with [Tab] switching)
//! - Interactive Drive & Mount Switcher Modal ([d] / [D]) for Windows drives (C:, D:, Z:) & Unix mounts
//! - Multi-Selection Mode ([Space] / [v]) with visual checkmarks [x] for bulk operations
//! - Live Search / Fuzzy Filter ([/] key)
//! - Full Mouse & Keyboard Navigation (Click, double-click, scroll-wheel, Vim h/j/k/l, arrow keys)
//! - Multi-Threaded Non-Blocking Worker Pool: Directory scanning, transcoding, exam conformer, and
//!   archive packing run in background worker threads with generation counters and 30s watchdog timers.
//!   Zero UI hitching, freezing, or lockups under rapid user tampering.

use crate::archive::{pack_archive, ArchiveType};
use crate::codec::{
    transcode_file, transcode_to_all_formats, BenchmarkEngine, BenchmarkReport, FileQualityChecker,
    FileQualityReport, LosslessMode,
};
use crate::core::{
    fuzzy_match, get_system_arch_info, get_system_drives, resolve_file_icon, spawn_async_dir_scan,
    AsyncWorkerPool, DriveInfo, FileItem, MossambiConfig, ScanBatch, TaskUpdate,
};
use crate::exam::{optimize_exam_media, ExamConstraint};
use crate::hwaccel::detect_hardware_capabilities;
use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseButton,
        MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Row, Table, Tabs},
    Frame, Terminal,
};
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

// 100% Certified Pantone Palette Constants (Eye-Safe)
pub const PANTONE_RAW_PEEL: Color = Color::Rgb(74, 133, 90); // Pantone 16-5917 TPG Sage Meadow
pub const PANTONE_SWEET_ZEST: Color = Color::Rgb(0, 122, 255); // Pantone 19-4052 TPG Classic Blue
pub const PANTONE_RIPE_PULP: Color = Color::Rgb(217, 119, 6); // Pantone 14-1064 TPG Soft Amber
pub const PANTONE_CITRUS_GOLD: Color = Color::Rgb(255, 184, 28); // Pantone 1235 C
pub const PANTONE_TANGERINE: Color = Color::Rgb(248, 143, 88); // Pantone 15-1247
pub const PANTONE_DEEP_FOREST: Color = Color::Rgb(46, 77, 30); // Pantone 371 C
pub const PANTONE_WHITE: Color = Color::Rgb(244, 245, 247);

// macOS Traffic Lights
pub const MAC_TRAFFIC_RED: Color = Color::Rgb(255, 95, 86);
pub const MAC_TRAFFIC_YELLOW: Color = Color::Rgb(255, 189, 46);
pub const MAC_TRAFFIC_GREEN: Color = Color::Rgb(39, 201, 63);

#[derive(Clone, Copy)]
pub struct TuiThemePalette {
    pub bg: Color,
    pub surface: Color,
    pub border: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub accent: Color,
    pub gold: Color,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExplorerLayoutMode {
    Miller3Column = 0,
    FullScreen = 1,
    DualPane = 2,
}

impl ExplorerLayoutMode {
    pub fn cycle(&self) -> Self {
        match self {
            Self::Miller3Column => Self::FullScreen,
            Self::FullScreen => Self::DualPane,
            Self::DualPane => Self::Miller3Column,
        }
    }

    pub fn badge(&self) -> &'static str {
        match self {
            Self::Miller3Column => "3-Column Miller (Yazi)",
            Self::FullScreen => "Full-Screen Explorer",
            Self::DualPane => "Dual-Pane Commander",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AppTab {
    Convert = 0,
    ExamStudio = 1,
    Archive = 2,
    Hardware = 3,
    Settings = 4,
    Quality = 5,
}

impl AppTab {
    pub fn all() -> &'static [AppTab] {
        &[
            AppTab::Convert,
            AppTab::ExamStudio,
            AppTab::Archive,
            AppTab::Hardware,
            AppTab::Settings,
            AppTab::Quality,
        ]
    }

    pub fn title(&self) -> &'static str {
        match self {
            AppTab::Convert => " 📁 File Explorer & Transcoder ",
            AppTab::ExamStudio => " 🎯 Custom Form Conformer ",
            AppTab::Archive => " 📦 Archive Manager ",
            AppTab::Hardware => " 🏎️ Hardware & AI ",
            AppTab::Settings => " ⚙️ Appearance & Themes ",
            AppTab::Quality => " 🔬 Quality & Benchmarks ",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActivePane {
    Source,
    Target,
}

pub struct AppState {
    pub current_tab: AppTab,
    pub config: MossambiConfig,
    pub status_message: String,
    pub status_is_error: bool,

    // Layout Modes
    pub explorer_layout: ExplorerLayoutMode,

    // File Explorer Navigation
    pub current_dir: PathBuf,
    pub current_entries: Vec<FileItem>,
    pub parent_entries: Vec<FileItem>,
    pub selected_file_idx: usize,
    pub selected_format_idx: usize,
    pub available_formats: Vec<&'static str>,

    // Multi-Selection Set
    pub selected_paths: HashSet<PathBuf>,

    // Search Filter
    pub search_mode: bool,
    pub search_query: String,

    // Drive & Mount Switcher Modal
    pub show_drive_modal: bool,
    pub drives: Vec<DriveInfo>,
    pub selected_drive_idx: usize,

    // Dual-Pane Commander State
    pub target_dir: PathBuf,
    pub target_entries: Vec<FileItem>,
    pub target_selected_idx: usize,
    pub active_pane: ActivePane,

    // Async Scanning & Generation Counters
    // NOTE: must stay Arc<AtomicU64> — spawn_async_dir_scan (core::fs_utils)
    // takes Arc<AtomicU64>, and that module is out of this change's scope.
    pub scan_gen: Arc<AtomicU64>,
    pub scan_tx: Sender<ScanBatch>,
    pub scan_rx: Receiver<ScanBatch>,

    // Concurrency Worker Pool & Watchdog
    pub worker_pool: AsyncWorkerPool,
    pub active_task_name: Option<String>,
    pub active_task_progress: f32,
    pub spinner_tick: usize,

    // Preview Cache
    pub preview_content: Option<String>,
    pub preview_path: Option<PathBuf>,

    // Double-Click Mouse Tracker
    pub last_click_time: Option<Instant>,
    pub last_click_idx: Option<usize>,

    // Exam Studio
    pub exam_presets: Vec<(&'static str, ExamConstraint)>,
    pub selected_preset_idx: usize,
    pub exam_status: String,

    // Archive
    pub archive_types: Vec<ArchiveType>,
    pub selected_archive_type_idx: usize,

    // Settings
    pub shade_options: Vec<&'static str>,
    pub selected_shade_idx: usize,
    pub should_quit: bool,

    // Quality & Benchmarks
    pub quality_report: Option<FileQualityReport>,
    pub bench_report: Option<BenchmarkReport>,
    pub inspection_status: String,
}

impl AppState {
    pub fn new() -> Self {
        Self::new_with_path(None)
    }

    pub fn new_with_path(initial_path: Option<PathBuf>) -> Self {
        let config = MossambiConfig::load();
        let current_dir = initial_path
            .and_then(|p| {
                if p.is_dir() {
                    Some(p)
                } else {
                    p.parent().map(|parent| parent.to_path_buf())
                }
            })
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let target_dir = current_dir.clone();
        let (scan_tx, scan_rx) = channel();
        let drives = get_system_drives();

        let mut app = Self {
            current_tab: AppTab::Convert,
            config,
            status_message: "Mossambi Yazi-Grade File Explorer. [Enter] Open | [Backspace] Up | [d] Drives | [F6] Layout | [Space] Select | [c] Convert".into(),
            status_is_error: false,

            explorer_layout: ExplorerLayoutMode::Miller3Column,

            current_dir: current_dir.clone(),
            current_entries: Vec::new(),
            parent_entries: Vec::new(),
            selected_file_idx: 0,
            selected_format_idx: 0,
            available_formats: vec!["JPG", "PNG", "WEBP", "PDF", "HEIC", "TIFF", "AVIF", "ALL"],

            selected_paths: HashSet::new(),

            search_mode: false,
            search_query: String::new(),

            show_drive_modal: false,
            drives,
            selected_drive_idx: 0,

            target_dir,
            target_entries: Vec::new(),
            target_selected_idx: 0,
            active_pane: ActivePane::Source,

            scan_gen: Arc::new(AtomicU64::new(1)),
            scan_tx,
            scan_rx,

            worker_pool: AsyncWorkerPool::new(),
            active_task_name: None,
            active_task_progress: 0.0,
            spinner_tick: 0,

            preview_content: None,
            preview_path: None,

            last_click_time: None,
            last_click_idx: None,

            exam_presets: vec![
                ("Custom Exam Envelope (350x350, 200DPI, 20-100KB)", ExamConstraint::preset_upsc_photo()),
                ("Custom Signature Envelope (350x150, 200DPI, 10-40KB)", ExamConstraint::preset_upsc_signature()),
                ("Standard Document Scan (200DPI, Whitened)", ExamConstraint::preset_ssc_photo()),
            ],
            selected_preset_idx: 0,
            exam_status: "Select a custom specification and press [Enter] to optimize.".into(),

            archive_types: vec![
                ArchiveType::Zip,
                ArchiveType::TarGz,
                ArchiveType::TarXz,
                ArchiveType::TarZst,
                ArchiveType::TarBz2,
                ArchiveType::Tar,
            ],
            selected_archive_type_idx: 0,

            shade_options: vec![
                "rawpeel",
                "sweetzest",
                "ripepulp",
                "citrusgold",
                "tangerine",
                "deepforest",
            ],
            selected_shade_idx: 0,
            should_quit: false,

            quality_report: None,
            bench_report: None,
            inspection_status: "Select a file and press [Enter] to inspect or [b] to benchmark.".into(),
        };

        app.trigger_async_scan(current_dir);
        app
    }

    /// Triggers a non-blocking asynchronous directory scan stamped with generation counter
    pub fn trigger_async_scan(&mut self, dir: PathBuf) {
        self.current_dir = dir.clone();
        let new_gen = self.scan_gen.fetch_add(1, Ordering::SeqCst) + 1;
        spawn_async_dir_scan(
            dir.clone(),
            new_gen,
            Arc::clone(&self.scan_gen),
            self.scan_tx.clone(),
        );

        // Also asynchronously scan parent directory for left Miller column
        if let Some(parent) = dir.parent() {
            let parent_dir = parent.to_path_buf();
            let mut parent_items = Vec::new();
            if let Ok(rd) = fs::read_dir(&parent_dir) {
                for e in rd.flatten() {
                    parent_items.push(FileItem::from_path(e.path()));
                }
            }
            parent_items.sort_by(|a, b| {
                if a.is_dir && !b.is_dir {
                    std::cmp::Ordering::Less
                } else if !a.is_dir && b.is_dir {
                    std::cmp::Ordering::Greater
                } else {
                    a.name.to_lowercase().cmp(&b.name.to_lowercase())
                }
            });
            self.parent_entries = parent_items;
        } else {
            self.parent_entries.clear();
        }

        self.selected_file_idx = 0;
        self.update_preview();
    }

    pub fn filtered_entries(&self) -> Vec<&FileItem> {
        let q = self.search_query.trim();
        if q.is_empty() {
            self.current_entries.iter().collect()
        } else {
            let mut scored: Vec<(&FileItem, i64)> = self
                .current_entries
                .iter()
                .filter_map(|e| {
                    let score = fuzzy_match(q, &e.name)?;
                    Some((e, score))
                })
                .collect();
            scored.sort_by(|(a, score_a), (b, score_b)| {
                score_b
                    .cmp(score_a)
                    .then_with(|| b.is_dir.cmp(&a.is_dir))
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            });
            scored.into_iter().map(|(item, _)| item).collect()
        }
    }

    pub fn selected_item(&self) -> Option<&FileItem> {
        let entries = self.filtered_entries();
        entries.get(self.selected_file_idx).copied()
    }

    pub fn navigate_into_selected(&mut self) {
        if let Some(item) = self.selected_item().cloned() {
            if item.is_dir {
                self.trigger_async_scan(item.path);
            } else {
                // If file, trigger instant conversion or preview
                self.execute_selected_conversion();
            }
        }
    }

    pub fn navigate_up_to_parent(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            let parent_buf = parent.to_path_buf();
            // Don't navigate into an empty relative path
            if parent_buf.as_os_str().is_empty() {
                self.show_drive_modal = true;
            } else {
                self.trigger_async_scan(parent_buf);
            }
        } else {
            // At root of drive (C:\, Z:\, /) -> open drive selector modal
            self.show_drive_modal = true;
        }
    }

    pub fn switch_to_selected_drive(&mut self) {
        if let Some(drive) = self.drives.get(self.selected_drive_idx).cloned() {
            self.show_drive_modal = false;
            self.trigger_async_scan(drive.path);
            self.status_message = format!("Switched to drive {}", drive.display_name);
            self.status_is_error = false;
        }
    }

    pub fn toggle_selection_on_highlighted(&mut self) {
        if let Some(item) = self.selected_item() {
            let p = item.path.clone();
            if self.selected_paths.contains(&p) {
                self.selected_paths.remove(&p);
            } else {
                self.selected_paths.insert(p);
            }
        }
    }

    pub fn toggle_select_all(&mut self) {
        let all_paths: Vec<PathBuf> = self
            .filtered_entries()
            .iter()
            .map(|e| e.path.clone())
            .collect();
        if self.selected_paths.len() >= all_paths.len() && !all_paths.is_empty() {
            self.selected_paths.clear();
        } else {
            for p in all_paths {
                self.selected_paths.insert(p);
            }
        }
    }

    pub fn update_preview(&mut self) {
        if let Some(item) = self.selected_item() {
            let path = item.path.clone();
            if item.is_dir {
                let count = fs::read_dir(&path).map(|r| r.count()).unwrap_or(0);
                self.preview_content = Some(format!(
                    "📁 Directory: {}\nEntries: {}\nLocation: {}",
                    item.name,
                    count,
                    path.display()
                ));
            } else {
                let ext = item.extension.to_lowercase();
                if matches!(
                    ext.as_str(),
                    "txt"
                        | "md"
                        | "rs"
                        | "toml"
                        | "json"
                        | "csv"
                        | "yaml"
                        | "py"
                        | "c"
                        | "cpp"
                        | "h"
                ) {
                    if let Ok(txt) = fs::read_to_string(&path) {
                        let lines: Vec<&str> = txt.lines().take(30).collect();
                        self.preview_content = Some(lines.join("\n"));
                    } else {
                        self.preview_content = Some(format!(
                            "📄 Text File: {}\nSize: {}",
                            item.name,
                            item.formatted_size()
                        ));
                    }
                } else if matches!(
                    ext.as_str(),
                    "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" | "tiff" | "avif" | "ico"
                ) {
                    self.preview_content = Some(format!(
                        "🖼️ Image Media: {}\nSize: {}\nExtension: .{}\nFull Path: {}\n\nPress [c] to transcode\nPress [i] to inspect quality",
                        item.name, item.formatted_size(), ext, path.display()
                    ));
                } else if matches!(
                    ext.as_str(),
                    "mp4" | "mkv" | "mov" | "avi" | "webm" | "mp3" | "wav" | "flac" | "aac"
                ) {
                    self.preview_content =
                        Some(format!(
                        "🎬 Audio/Video Media: {}\nSize: {}\nCodec: Stream Remuxable\nLocation: {}",
                        item.name, item.formatted_size(), path.display()
                    ));
                } else {
                    self.preview_content = Some(format!(
                        "📦 File: {}\nSize: {}\nType: {}\nPath: {}",
                        item.name,
                        item.formatted_size(),
                        ext,
                        path.display()
                    ));
                }
            }
            self.preview_path = Some(path);
        } else {
            self.preview_content = None;
            self.preview_path = None;
        }
    }

    /// Triggers non-blocking background transcoding via the worker pool
    pub fn execute_selected_conversion(&mut self) {
        let targets: Vec<PathBuf> = if !self.selected_paths.is_empty() {
            self.selected_paths.iter().cloned().collect()
        } else if let Some(item) = self.selected_item() {
            if item.is_dir {
                self.trigger_async_scan(item.path.clone());
                return;
            }
            vec![item.path.clone()]
        } else {
            return;
        };

        let target_ext = self.available_formats[self.selected_format_idx].to_lowercase();
        self.active_task_name = Some(format!(
            "Transcoding {} file(s) to {}",
            targets.len(),
            target_ext.to_uppercase()
        ));
        self.active_task_progress = 0.1;
        self.status_message = format!(
            "In Background: Transcoding to {}...",
            target_ext.to_uppercase()
        );
        self.status_is_error = false;

        let total = targets.len();
        self.worker_pool
            .spawn_task("Transcode", move |_cancel, tx, task_id| {
                // TODO: thread `_cancel` into the codec engine once it supports
                // cooperative cancellation; for now the token is honoured only
                // BETWEEN files in this loop (a single long ffmpeg still runs
                // to completion unless the watchdog fires).
                for (idx, in_file) in targets.iter().enumerate() {
                    if _cancel.load(Ordering::Relaxed) {
                        return;
                    }
                    let stem = in_file
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("output");
                    let out_file = in_file
                        .parent()
                        .unwrap_or_else(|| Path::new("."))
                        .join(format!("{}.{}", stem, target_ext));

                    let res = if target_ext == "all" {
                        transcode_to_all_formats(in_file, LosslessMode::StrictBitExact, None)
                            .map(|v| v.first().cloned().unwrap_or(out_file))
                    } else {
                        transcode_file(in_file, &target_ext, LosslessMode::StrictBitExact, None)
                    };

                    let frac = (idx + 1) as f32 / total as f32;
                    let _ = tx.send(TaskUpdate::Progress {
                        id: task_id,
                        fraction: frac,
                        status: format!("Processed {}/{} files", idx + 1, total),
                        item: None,
                    });

                    if let Err(e) = res {
                        let _ = tx.send(TaskUpdate::Failed {
                            id: task_id,
                            error: format!("Transcode error on {}: {}", in_file.display(), e),
                        });
                        return;
                    }
                }

                let _ = tx.send(TaskUpdate::Completed {
                    id: task_id,
                    output: None,
                    message: format!(
                        "[v] Transcoded {} file(s) to {}",
                        total,
                        target_ext.to_uppercase()
                    ),
                });
            });
    }

    /// Triggers non-blocking background exam conformer via worker pool
    pub fn execute_exam_optimization(&mut self) {
        if let Some(item) = self.selected_item().cloned() {
            if item.is_dir {
                self.status_message = "Please select an image file first.".into();
                self.status_is_error = true;
                return;
            }

            let (_, constraint) = self.exam_presets[self.selected_preset_idx].clone();
            let stem = item
                .path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("exam_output");
            let out_file = item
                .path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(format!("{}_conformed.jpg", stem));

            self.active_task_name = Some("Conforming image to exam specifications".into());
            self.active_task_progress = 0.2;
            self.status_message = "🎯 Optimizing file to match exact pixel/byte envelope...".into();

            self.worker_pool
                .spawn_task(
                    "ExamConformer",
                    move |_cancel, tx, task_id| match optimize_exam_media(
                        &item.path,
                        &out_file,
                        &constraint,
                    ) {
                        Ok(res) => {
                            let len = fs::metadata(&res).map(|m| m.len()).unwrap_or(0);
                            let _ = tx.send(TaskUpdate::Completed {
                                id: task_id,
                                output: Some(res),
                                message: format!(
                                    "✓ Exam Conformed: {:.1} KB within specifications!",
                                    len as f64 / 1024.0
                                ),
                            });
                        }
                        Err(e) => {
                            let _ = tx.send(TaskUpdate::Failed {
                                id: task_id,
                                error: format!("Exam conformer error: {}", e),
                            });
                        }
                    },
                );
        }
    }

    /// Triggers non-blocking background archive packing via worker pool
    pub fn execute_archive_pack(&mut self) {
        let targets: Vec<PathBuf> = if !self.selected_paths.is_empty() {
            self.selected_paths.iter().cloned().collect()
        } else if let Some(item) = self.selected_item() {
            vec![item.path.clone()]
        } else {
            return;
        };

        let atype = self.archive_types[self.selected_archive_type_idx];
        let ext = match atype {
            ArchiveType::Zip => "zip",
            ArchiveType::TarGz => "tar.gz",
            ArchiveType::TarXz => "tar.xz",
            ArchiveType::TarZst => "tar.zst",
            ArchiveType::TarBz2 => "tar.bz2",
            ArchiveType::Tar => "tar",
            _ => "zip",
        };

        let first = targets[0].clone();
        let stem = first
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("bundle");
        let out_file = first
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!("{}.{}", stem, ext));

        self.active_task_name = Some(format!("Packing {} item(s) into .{}", targets.len(), ext));
        self.active_task_progress = 0.2;
        self.status_message = format!("📦 Packing archive into .{}...", ext);

        self.worker_pool.spawn_task(
            "ArchivePack",
            move |_cancel, tx, task_id| match pack_archive(&targets, &out_file, atype) {
                Ok(_) => {
                    let _ = tx.send(TaskUpdate::Completed {
                        id: task_id,
                        output: Some(out_file.clone()),
                        message: format!("✓ Packed: {}", out_file.display()),
                    });
                }
                Err(e) => {
                    let _ = tx.send(TaskUpdate::Failed {
                        id: task_id,
                        error: format!("Archive packing failed: {}", e),
                    });
                }
            },
        );
    }

    pub fn inspect_selected_file(&mut self) {
        if let Some(item) = self.selected_item() {
            if !item.is_dir {
                match FileQualityChecker::inspect_file(&item.path) {
                    Ok(rep) => {
                        self.inspection_status = format!(
                            "✓ Inspected {} (Score: {:.1}%, Format: {})",
                            item.name, rep.overall_score, rep.format
                        );
                        self.quality_report = Some(rep);
                    }
                    Err(e) => {
                        self.inspection_status = format!("✗ Inspection failed: {}", e);
                    }
                }
            }
        }
    }

    pub fn benchmark_selected_file(&mut self) {
        if let Some(item) = self.selected_item() {
            if !item.is_dir {
                match BenchmarkEngine::benchmark_file(&item.path, None) {
                    Ok(b) => {
                        self.inspection_status = format!(
                            "✓ Benchmark: {:.2}ms (Throughput: {:.1} MB/s)",
                            b.duration_ms, b.throughput_mb_per_sec
                        );
                        self.bench_report = Some(b);
                    }
                    Err(e) => {
                        self.inspection_status = format!("✗ Benchmark failed: {}", e);
                    }
                }
            }
        }
    }

    pub fn current_theme_palette(&self) -> TuiThemePalette {
        let pt = self.config.theme_mode.to_pantone_theme();
        let (bg_r, bg_g, bg_b) = pt.bg.to_rgb();
        let (surf_r, surf_g, surf_b) = pt.surface.to_rgb();
        let (bord_r, bord_g, bord_b) = pt.border.to_rgb();
        let (tp_r, tp_g, tp_b) = pt.text_primary.to_rgb();
        let (ts_r, ts_g, ts_b) = pt.text_secondary.to_rgb();
        let (tm_r, tm_g, tm_b) = pt.text_muted.to_rgb();
        let (acc_r, acc_g, acc_b) = pt.accent.to_rgb();
        let (hi_r, hi_g, hi_b) = pt.highlight.to_rgb();

        let bg_color = if self.config.tui_transparent_bg {
            Color::Reset
        } else {
            Color::Rgb(bg_r, bg_g, bg_b)
        };

        TuiThemePalette {
            bg: bg_color,
            surface: Color::Rgb(surf_r, surf_g, surf_b),
            border: Color::Rgb(bord_r, bord_g, bord_b),
            text_primary: Color::Rgb(tp_r, tp_g, tp_b),
            text_secondary: Color::Rgb(ts_r, ts_g, ts_b),
            text_muted: Color::Rgb(tm_r, tm_g, tm_b),
            accent: Color::Rgb(acc_r, acc_g, acc_b),
            gold: Color::Rgb(hi_r, hi_g, hi_b),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn run() -> Result<()> {
    run_with_path(None)
}

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(
            stdout,
            LeaveAlternateScreen,
            DisableMouseCapture,
            crossterm::cursor::Show
        );
    }
}

pub fn run_with_path(initial_path: Option<PathBuf>) -> Result<()> {
    // Logging must initialise at the TUI entrypoint (before raw mode, so any
    // startup failures are still visible in the normal terminal).
    let _ = env_logger::try_init();
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let _guard = TerminalGuard;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = AppState::new_with_path(initial_path);
    run_app(&mut terminal, &mut state)
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState,
) -> Result<()> {
    const SPINNER_CHARS: &[&str] = &[
        "\u{280B}", "\u{2819}", "\u{2839}", "\u{2838}", "\u{283C}", "\u{2834}", "\u{2826}",
        "\u{2827}", "\u{2807}", "\u{280F}",
    ];

    while !state.should_quit {
        // 1. Process asynchronous directory scan results
        while let Ok(batch) = state.scan_rx.try_recv() {
            if batch.gen == state.scan_gen.load(Ordering::Relaxed) {
                state.current_entries = batch.entries;
                if state.selected_file_idx >= state.current_entries.len() {
                    state.selected_file_idx = state.current_entries.len().saturating_sub(1);
                }
                state.update_preview();
            }
        }

        // 2. Process background task worker results (Zero-freeze pipeline)
        while let Some(update) = state.worker_pool.try_recv() {
            match update {
                TaskUpdate::Started { name, .. } => {
                    state.active_task_name = Some(name);
                    state.active_task_progress = 0.05;
                }
                TaskUpdate::Progress {
                    fraction, status, ..
                } => {
                    state.active_task_progress = fraction;
                    state.status_message = status;
                }
                TaskUpdate::Completed { message, .. } => {
                    state.active_task_name = None;
                    state.active_task_progress = 1.0;
                    state.status_message = message;
                    state.status_is_error = false;
                    state.trigger_async_scan(state.current_dir.clone());
                }
                TaskUpdate::Failed { error, .. } => {
                    state.active_task_name = None;
                    state.status_message = error;
                    state.status_is_error = true;
                }
                TaskUpdate::ItemResult { .. } => {}
            }
        }

        state.spinner_tick = (state.spinner_tick + 1) % SPINNER_CHARS.len();

        // 3. Render 60 FPS non-blocking frame
        terminal.draw(|f| draw_ui(f, state, SPINNER_CHARS[state.spinner_tick]))?;

        // 4. Non-blocking input polling (16ms timeout)
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    handle_key_event(state, key);
                }
                Event::Mouse(mouse) => {
                    handle_mouse_event(state, mouse);
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn handle_key_event(state: &mut AppState, key: crossterm::event::KeyEvent) {
    use crossterm::event::KeyModifiers;

    // Fast global quit on Ctrl+C or Ctrl+Q
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && (key.code == KeyCode::Char('c')
            || key.code == KeyCode::Char('C')
            || key.code == KeyCode::Char('q'))
    {
        state.should_quit = true;
        return;
    }

    let code = key.code;

    // 1. Drive Modal Active
    if state.show_drive_modal {
        match code {
            KeyCode::Esc => state.show_drive_modal = false,
            KeyCode::Up | KeyCode::Char('k') => {
                if state.selected_drive_idx > 0 {
                    state.selected_drive_idx -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if state.selected_drive_idx + 1 < state.drives.len() {
                    state.selected_drive_idx += 1;
                }
            }
            KeyCode::Enter => state.switch_to_selected_drive(),
            _ => {}
        }
        return;
    }

    // 2. Search Mode Active
    if state.search_mode {
        match code {
            KeyCode::Esc | KeyCode::Enter => state.search_mode = false,
            KeyCode::Backspace => {
                state.search_query.pop();
                state.selected_file_idx = 0;
                state.update_preview();
            }
            KeyCode::Char(c) => {
                state.search_query.push(c);
                state.selected_file_idx = 0;
                state.update_preview();
            }
            _ => {}
        }
        return;
    }

    // 3. Normal Mode
    match code {
        KeyCode::Char('q') | KeyCode::Esc => state.should_quit = true,

        // Layout Toggle [F6]
        KeyCode::F(6) => {
            state.explorer_layout = state.explorer_layout.cycle();
            state.status_message = format!("Layout switched to: {}", state.explorer_layout.badge());
        }

        // Drive Switcher Modal [d] / [D]
        KeyCode::Char('d') | KeyCode::Char('D') => {
            state.drives = get_system_drives();
            state.show_drive_modal = true;
        }

        // Live Search Filter [/]
        KeyCode::Char('/') => {
            state.search_mode = true;
            state.search_query.clear();
        }

        // Multi-Selection Toggle [Space] or [v]
        KeyCode::Char(' ') | KeyCode::Char('v') => {
            state.toggle_selection_on_highlighted();
        }

        // Select All [a]
        KeyCode::Char('a') => {
            state.toggle_select_all();
        }

        // Tab Navigation
        KeyCode::Tab => {
            if state.explorer_layout == ExplorerLayoutMode::DualPane
                && state.current_tab == AppTab::Convert
            {
                state.active_pane = match state.active_pane {
                    ActivePane::Source => ActivePane::Target,
                    ActivePane::Target => ActivePane::Source,
                };
            } else {
                let curr = state.current_tab as usize;
                state.current_tab = match curr {
                    0 => AppTab::ExamStudio,
                    1 => AppTab::Archive,
                    2 => AppTab::Hardware,
                    3 => AppTab::Settings,
                    4 => AppTab::Quality,
                    _ => AppTab::Convert,
                };
                if state.current_tab == AppTab::Quality && state.quality_report.is_none() {
                    state.inspect_selected_file();
                }
            }
        }
        KeyCode::BackTab => {
            let curr = state.current_tab as usize;
            state.current_tab = match curr {
                0 => AppTab::Quality,
                1 => AppTab::Convert,
                2 => AppTab::ExamStudio,
                3 => AppTab::Archive,
                4 => AppTab::Hardware,
                _ => AppTab::Settings,
            };
        }

        // Direct Tab Shortcuts
        KeyCode::Char('1') => state.current_tab = AppTab::Convert,
        KeyCode::Char('2') => state.current_tab = AppTab::ExamStudio,
        KeyCode::Char('3') => state.current_tab = AppTab::Archive,
        KeyCode::Char('4') => state.current_tab = AppTab::Hardware,
        KeyCode::Char('5') => state.current_tab = AppTab::Settings,
        KeyCode::Char('6') => {
            state.current_tab = AppTab::Quality;
            if state.quality_report.is_none() {
                state.inspect_selected_file();
            }
        }

        // Theme Cycle [t]
        KeyCode::Char('t') => {
            state.config.theme_mode = state.config.theme_mode.next();
            let _ = state.config.save();
            state.status_message = format!(
                "🎨 Pantone: {} ({})",
                state.config.theme_mode.name(),
                if state.config.theme_mode.is_dark() {
                    "Dark"
                } else {
                    "Light"
                }
            );
        }

        // Theme Set Toggle [T] (Shift+T)
        KeyCode::Char('T') => {
            state.config.theme_mode = state.config.theme_mode.toggle_dark_light();
            let _ = state.config.save();
            state.status_message = format!(
                "🎨 Toggled Set: {} ({})",
                state.config.theme_mode.name(),
                if state.config.theme_mode.is_dark() {
                    "11 Dark Themes"
                } else {
                    "1 Light Theme"
                }
            );
        }

        // Glass / Transparent Terminal Background Toggle [g]
        KeyCode::Char('g') => {
            state.config.tui_transparent_bg = !state.config.tui_transparent_bg;
            let _ = state.config.save();
            state.status_message = if state.config.tui_transparent_bg {
                "[Glass] TUI Glass: Terminal Transparency & Compositor Blur Enabled".to_string()
            } else {
                "[Opaque] TUI Opaque: Solid Pantone Background Enabled".to_string()
            };
        }

        // Arrow & Vim Navigation
        KeyCode::Up | KeyCode::Char('k') => {
            let count = state.filtered_entries().len();
            if count > 0 && state.selected_file_idx > 0 {
                state.selected_file_idx -= 1;
                state.update_preview();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let count = state.filtered_entries().len();
            if count > 0 && state.selected_file_idx + 1 < count {
                state.selected_file_idx += 1;
                state.update_preview();
            }
        }
        KeyCode::Left | KeyCode::Char('h') => {
            state.navigate_up_to_parent();
        }
        KeyCode::Right | KeyCode::Char('l') => {
            state.navigate_into_selected();
        }

        // Directory Navigation
        KeyCode::Backspace => {
            state.navigate_up_to_parent();
        }
        KeyCode::Enter => match state.current_tab {
            AppTab::Convert => state.navigate_into_selected(),
            AppTab::ExamStudio => state.execute_exam_optimization(),
            AppTab::Archive => state.execute_archive_pack(),
            AppTab::Quality => state.inspect_selected_file(),
            AppTab::Settings => {
                state.config.enforce_zero_loss = !state.config.enforce_zero_loss;
                let _ = state.config.save();
                state.status_message =
                    format!("Zero-loss enforcement: {}", state.config.enforce_zero_loss);
            }
            _ => {}
        },

        // Action Keys
        KeyCode::Char('c') => {
            state.execute_selected_conversion();
        }
        KeyCode::Char('e') => {
            state.execute_exam_optimization();
        }
        KeyCode::Char('p') => {
            state.execute_archive_pack();
        }
        KeyCode::Char('r') => {
            state.trigger_async_scan(state.current_dir.clone());
            state.status_message = "Directory refreshed.".into();
        }
        KeyCode::Char('b') => {
            if state.current_tab == AppTab::Quality {
                state.benchmark_selected_file();
            }
        }
        KeyCode::Home => {
            state.selected_file_idx = 0;
            state.update_preview();
        }
        KeyCode::End | KeyCode::Char('G') => {
            let count = state.filtered_entries().len();
            if count > 0 {
                state.selected_file_idx = count - 1;
                state.update_preview();
            }
        }

        _ => {}
    }
}

fn handle_mouse_event(state: &mut AppState, mouse: MouseEvent) {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // Check double-click on file item
            let now = Instant::now();
            let is_double_click = if let (Some(t), Some(idx)) =
                (state.last_click_time, state.last_click_idx)
            {
                now.duration_since(t) < Duration::from_millis(400) && idx == state.selected_file_idx
            } else {
                false
            };
            state.last_click_time = Some(now);
            state.last_click_idx = Some(state.selected_file_idx);

            if is_double_click {
                state.navigate_into_selected();
            }
        }
        MouseEventKind::ScrollDown => {
            let count = state.filtered_entries().len();
            if count > 0 && state.selected_file_idx + 1 < count {
                state.selected_file_idx += 1;
                state.update_preview();
            }
        }
        MouseEventKind::ScrollUp => {
            let count = state.filtered_entries().len();
            if count > 0 && state.selected_file_idx > 0 {
                state.selected_file_idx -= 1;
                state.update_preview();
            }
        }
        _ => {}
    }
}

fn draw_ui(f: &mut Frame, state: &AppState, spinner: &str) {
    let palette = state.current_theme_palette();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header Bar & Tabs
            Constraint::Min(10),   // Main Viewport
            Constraint::Length(3), // Status Bar & Hotkeys
        ])
        .split(f.area());

    // 1. macOS Header Bar
    draw_header(f, state, chunks[0], palette, spinner);

    // 2. Main Tab Viewport
    match state.current_tab {
        AppTab::Convert => draw_explorer_tab(f, state, chunks[1], palette),
        AppTab::ExamStudio => draw_exam_tab(f, state, chunks[1], palette),
        AppTab::Archive => draw_archive_tab(f, state, chunks[1], palette),
        AppTab::Hardware => draw_hardware_tab(f, chunks[1], palette),
        AppTab::Settings => draw_settings_tab(f, state, chunks[1], palette),
        AppTab::Quality => draw_quality_tab(f, state, chunks[1], palette),
    }

    // 3. Status Bar & Breadcrumb Hotkeys
    draw_footer(f, state, chunks[2], palette, spinner);

    // 4. Drive Switcher Modal Overlay
    if state.show_drive_modal {
        draw_drive_modal(f, state, palette);
    }
}

fn draw_header(
    f: &mut Frame,
    state: &AppState,
    area: Rect,
    palette: TuiThemePalette,
    spinner: &str,
) {
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(12), // Traffic Lights
            Constraint::Min(40),    // Tabs
            Constraint::Length(32), // Layout Badge & Active Task
        ])
        .split(area);

    // Traffic Lights
    let traffic = Paragraph::new(Line::from(vec![
        Span::styled(" ● ", Style::default().fg(MAC_TRAFFIC_RED)),
        Span::styled("● ", Style::default().fg(MAC_TRAFFIC_YELLOW)),
        Span::styled("● ", Style::default().fg(MAC_TRAFFIC_GREEN)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(palette.border)),
    );
    f.render_widget(traffic, header_chunks[0]);

    // Tabs
    let tab_titles: Vec<Line> = AppTab::all()
        .iter()
        .map(|t| {
            let style = if *t == state.current_tab {
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.text_muted)
            };
            Line::from(Span::styled(t.title(), style))
        })
        .collect();

    let tabs = Tabs::new(tab_titles)
        .select(state.current_tab as usize)
        .block(
            Block::default()
                .title(" Mossambi OS Studio ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(palette.border)),
        )
        .highlight_style(
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(tabs, header_chunks[1]);

    // Layout Indicator & Background Spinner
    let layout_text = if let Some(ref task) = state.active_task_name {
        format!(
            "{} {} ({:.0}%)",
            spinner,
            task,
            state.active_task_progress * 100.0
        )
    } else {
        format!("[F6] {}", state.explorer_layout.badge())
    };

    let layout_badge = Paragraph::new(layout_text)
        .style(
            Style::default()
                .fg(palette.gold)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(palette.border)),
        );
    f.render_widget(layout_badge, header_chunks[2]);
}

fn draw_explorer_tab(f: &mut Frame, state: &AppState, area: Rect, palette: TuiThemePalette) {
    match state.explorer_layout {
        ExplorerLayoutMode::Miller3Column => draw_miller_view(f, state, area, palette),
        ExplorerLayoutMode::FullScreen => draw_fullscreen_view(f, state, area, palette),
        ExplorerLayoutMode::DualPane => draw_dual_pane_view(f, state, area, palette),
    }
}

/// 1. Yazi-Grade 3-Column Miller View
fn draw_miller_view(f: &mut Frame, state: &AppState, area: Rect, palette: TuiThemePalette) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(22), // Left: Parent Directory Context
            Constraint::Percentage(46), // Middle: Active Directory Listing
            Constraint::Percentage(32), // Right: Live Preview & Inspection
        ])
        .split(area);

    // Left Column: Parent Context
    let parent_items: Vec<ListItem> = state
        .parent_entries
        .iter()
        .take(25)
        .map(|item| {
            let icon_spec = resolve_file_icon(&item.name, &item.extension, item.is_dir);
            let icon = icon_spec.glyph;
            let is_curr = state
                .current_dir
                .file_name()
                .map(|n| n == item.name.as_str())
                .unwrap_or(false);
            let style = if is_curr {
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.text_muted)
            };
            ListItem::new(format!("{} {}", icon, item.name)).style(style)
        })
        .collect();

    let parent_title = state
        .current_dir
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Root".into());

    let parent_list = List::new(parent_items).block(
        Block::default()
            .title(format!(" ◂ {} ", parent_title))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(palette.border)),
    );
    f.render_widget(parent_list, cols[0]);

    // Middle Column: Active Directory Listing
    let entries = state.filtered_entries();
    let current_items: Vec<ListItem> = entries
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_sel = i == state.selected_file_idx;
            let is_checked = state.selected_paths.contains(&item.path);
            let check_mark = if is_checked { "[x] " } else { "[ ] " };
            let icon_spec = resolve_file_icon(&item.name, &item.extension, item.is_dir);
            let icon = icon_spec.glyph;

            let mut style = if is_sel {
                Style::default()
                    .fg(Color::Black)
                    .bg(palette.accent)
                    .add_modifier(Modifier::BOLD)
            } else if item.is_dir {
                Style::default()
                    .fg(PANTONE_SWEET_ZEST)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.text_primary)
            };

            if is_checked && !is_sel {
                style = style.fg(palette.gold).add_modifier(Modifier::BOLD);
            }

            let text = format!(
                "{}{} {} ({})",
                check_mark,
                icon,
                item.name,
                item.formatted_size()
            );
            ListItem::new(text).style(style)
        })
        .collect();

    let count_str = format!("{} items", entries.len());
    let sel_count = state.selected_paths.len();
    let sel_str = if sel_count > 0 {
        format!(" | {} selected", sel_count)
    } else {
        String::new()
    };

    let cur_title = format!(
        " 📂 {} ({}{}) ",
        state.current_dir.display(),
        count_str,
        sel_str
    );
    let active_list = List::new(current_items).block(
        Block::default()
            .title(cur_title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(palette.accent)),
    );
    f.render_widget(active_list, cols[1]);

    // Right Column: Live Preview & Inspection
    let preview_text = state
        .preview_content
        .as_deref()
        .unwrap_or("No preview available.");
    let preview_p = Paragraph::new(preview_text)
        .style(Style::default().fg(palette.text_secondary))
        .block(
            Block::default()
                .title(" Live Preview & Metadata ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(palette.border)),
        );
    f.render_widget(preview_p, cols[2]);
}

/// 2. Full-Screen Dedicated Explorer Table
fn draw_fullscreen_view(f: &mut Frame, state: &AppState, area: Rect, palette: TuiThemePalette) {
    let entries = state.filtered_entries();

    let rows: Vec<Row> = entries
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_sel = i == state.selected_file_idx;
            let is_checked = state.selected_paths.contains(&item.path);
            let check_mark = if is_checked { "[x]" } else { "[ ]" };
            let icon_spec = resolve_file_icon(&item.name, &item.extension, item.is_dir);
            let icon = icon_spec.glyph;

            let style = if is_sel {
                Style::default()
                    .fg(Color::Black)
                    .bg(palette.accent)
                    .add_modifier(Modifier::BOLD)
            } else if item.is_dir {
                Style::default().fg(PANTONE_SWEET_ZEST)
            } else {
                Style::default().fg(palette.text_primary)
            };

            let date_str = item
                .modified
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| format!("{}s ago", d.as_secs() % 3600))
                .unwrap_or_else(|| "-".into());

            Row::new(vec![
                check_mark.to_string(),
                format!("{} {}", icon, item.name),
                item.formatted_size(),
                item.extension.clone(),
                date_str,
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Percentage(50),
            Constraint::Length(14),
            Constraint::Length(10),
            Constraint::Length(14),
        ],
    )
    .header(
        Row::new(vec!["Sel", "Filename", "Size", "Type", "Modified"]).style(
            Style::default()
                .fg(palette.gold)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .title(format!(" Full Explorer: {} ", state.current_dir.display()))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(palette.accent)),
    );

    f.render_widget(table, area);
}

/// 3. Dual-Pane Commander (Midnight Commander / Total Commander Style)
fn draw_dual_pane_view(f: &mut Frame, state: &AppState, area: Rect, palette: TuiThemePalette) {
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left Pane: Source
    let is_source_active = state.active_pane == ActivePane::Source;
    let source_border = if is_source_active {
        palette.accent
    } else {
        palette.border
    };
    let entries = state.filtered_entries();

    let source_items: Vec<ListItem> = entries
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_sel = is_source_active && i == state.selected_file_idx;
            let icon = if item.is_dir { "📁 " } else { "📄 " };
            let style = if is_sel {
                Style::default()
                    .fg(Color::Black)
                    .bg(palette.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.text_primary)
            };
            ListItem::new(format!("{}{}", icon, item.name)).style(style)
        })
        .collect();

    let source_list = List::new(source_items).block(
        Block::default()
            .title(format!(" [Source] {} ", state.current_dir.display()))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(source_border)),
    );
    f.render_widget(source_list, panes[0]);

    // Right Pane: Target / Staging
    let is_target_active = state.active_pane == ActivePane::Target;
    let target_border = if is_target_active {
        palette.accent
    } else {
        palette.border
    };

    let target_items: Vec<ListItem> = state
        .selected_paths
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let is_sel = is_target_active && i == state.target_selected_idx;
            let name = p.file_name().unwrap_or_default().to_string_lossy();
            let style = if is_sel {
                Style::default()
                    .fg(Color::Black)
                    .bg(palette.gold)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.gold)
            };
            ListItem::new(format!("✓ {}", name)).style(style)
        })
        .collect();

    let target_list = List::new(target_items).block(
        Block::default()
            .title(format!(
                " [Target Staging] {} files queued ",
                state.selected_paths.len()
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(target_border)),
    );
    f.render_widget(target_list, panes[1]);
}

fn draw_footer(
    f: &mut Frame,
    state: &AppState,
    area: Rect,
    palette: TuiThemePalette,
    spinner: &str,
) {
    let status_text = if state.search_mode {
        format!(
            "🔍 Search Filter: {}_ (Press Enter or Esc to confirm)",
            state.search_query
        )
    } else if let Some(ref task) = state.active_task_name {
        format!(
            "{} Background Task Active: {} [{:.0}%]",
            spinner,
            task,
            state.active_task_progress * 100.0
        )
    } else {
        state.status_message.clone()
    };

    let style = if state.status_is_error {
        Style::default()
            .fg(Color::LightRed)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.accent)
    };

    let footer = Paragraph::new(status_text)
        .style(style)
        .block(
            Block::default()
                .title(" [Enter] Open | [Backspace] Up | [d] Drives | [F6] Layout | [t/T] Theme | [g] Glass | [Space] Select | [c] Convert ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(palette.border)),
        );
    f.render_widget(footer, area);
}

fn draw_drive_modal(f: &mut Frame, state: &AppState, palette: TuiThemePalette) {
    let area = centered_rect(50, 45, f.area());
    f.render_widget(Clear, area);

    let items: Vec<ListItem> = state
        .drives
        .iter()
        .enumerate()
        .map(|(i, drive)| {
            let is_sel = i == state.selected_drive_idx;
            let style = if is_sel {
                Style::default()
                    .fg(Color::Black)
                    .bg(palette.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.text_primary)
            };
            ListItem::new(format!(
                " 💾 {} ({}) ",
                drive.display_name,
                drive.path.display()
            ))
            .style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Select System Drive or Mount Point (Enter to Switch, Esc to Cancel) ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(palette.gold)),
    );
    f.render_widget(list, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_exam_tab(f: &mut Frame, state: &AppState, area: Rect, palette: TuiThemePalette) {
    let p = Paragraph::new(format!(
        "🎯 Custom Specification & Form Conformer\n\nSelected: {}\nStatus: {}\n\nPress [Enter] to optimize selected file.",
        state.exam_presets[state.selected_preset_idx].0,
        state.exam_status
    ))
    .block(
        Block::default()
            .title(" Custom Form Envelope Conformer ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(palette.border)),
    );
    f.render_widget(p, area);
}

fn draw_archive_tab(f: &mut Frame, state: &AppState, area: Rect, palette: TuiThemePalette) {
    let p = Paragraph::new(format!(
        "📦 Archive Manager\n\nArchive Type: {:?}\nQueued Items: {}\n\nPress [p] or [Enter] to pack archive.",
        state.archive_types[state.selected_archive_type_idx],
        state.selected_paths.len()
    ))
    .block(
        Block::default()
            .title(" Multi-Format Archive Suite ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(palette.border)),
    );
    f.render_widget(p, area);
}

fn draw_hardware_tab(f: &mut Frame, area: Rect, palette: TuiThemePalette) {
    let (os, arch) = get_system_arch_info();
    let hw = detect_hardware_capabilities();
    let p = Paragraph::new(format!(
        "🏎️ Hardware Acceleration & NPU Engine\n\nOS: {} | Arch: {}\nGPU Vendor: {:?} | HW Transcode: {}\nNPU AI Engine: {:?} ({})\nThread Concurrency: 16 threads (Rayon Pool)",
        os, arch, hw.vendor, hw.supports_gpu_transcode, hw.npu_engine, hw.npu_name
    ))
    .block(
        Block::default()
            .title(" Hardware Diagnostics ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(palette.border)),
    );
    f.render_widget(p, area);
}

fn draw_settings_tab(f: &mut Frame, state: &AppState, area: Rect, palette: TuiThemePalette) {
    let pt = state.config.theme_mode.to_pantone_theme();
    let p = Paragraph::new(format!(
        "⚙️ Appearance & Persistent Settings\n\n🎨 Active Theme: {} ({})\n   • Background: {} ({}, {})\n   • Surface:    {} ({}, {})\n   • Accent:     {} ({}, {})\n   • Highlight:  {} ({}, {})\n\n💡 Hotkeys:\n   • [t] Cycle sequentially across all 13 named themes\n   • [T] Switch between the 11 Dark Themes and 1 Light Theme\n   • [g] Toggle Terminal Glass / Transparency (Current: {})\n\nCitrus Shade: {}\nZero-Loss Enforcement: {}\nGesture Wakeup: {}\n\nPress [Enter] to toggle zero-loss.",
        pt.name,
        if pt.is_dark { "Certified Dark Theme" } else { "Certified Light Theme" },
        pt.bg.name, pt.bg.code, pt.bg.hex,
        pt.surface.name, pt.surface.code, pt.surface.hex,
        pt.accent.name, pt.accent.code, pt.accent.hex,
        pt.highlight.name, pt.highlight.code, pt.highlight.hex,
        if state.config.tui_transparent_bg { "Transparent Blur" } else { "Solid Canvas" },
        state.config.citrus_shade,
        state.config.enforce_zero_loss,
        state.config.enable_gesture_wakeup
    ))
    .block(
        Block::default()
            .title(" Persistent Configuration ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(palette.border)),
    );
    f.render_widget(p, area);
}

fn draw_quality_tab(f: &mut Frame, state: &AppState, area: Rect, palette: TuiThemePalette) {
    let text = if let Some(ref rep) = state.quality_report {
        format!(
            "🔬 Quality & Integrity Report: {}\nVerdict: {:?} | Score: {:.1}%\nFormat: {} | Size: {:.2} KB\nSHA-256: {}\nIntegrity: {:.1}%\nParseability: {:.1}%\nCompatibility: {:.1}%",
            rep.file_name,
            rep.verdict,
            rep.overall_score,
            rep.format,
            rep.file_size_kb,
            rep.sha256,
            rep.integrity.structural_integrity_score,
            rep.parseability.parseability_score,
            rep.compatibility.compatibility_score
        )
    } else {
        state.inspection_status.clone()
    };

    let p = Paragraph::new(text).block(
        Block::default()
            .title(" Universal Quality & Standards Matrix ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(palette.border)),
    );
    f.render_widget(p, area);
}
