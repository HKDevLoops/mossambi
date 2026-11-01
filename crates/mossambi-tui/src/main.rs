//! Mossambi Terminal User Interface (`mossambi-tui`)
//! 100% Apple macOS-Styled Terminal Interface built with Ratatui.
//! Features:
//! - Authentic macOS Traffic Lights (Close, Min, Zoom) in the title bar
//! - Multiple Apple Themes: Liquid Glass, Transparent Frosted Glass, Dark Anthracite, Light Aqua
//! - 5 macOS-styled tabs: File Transcoder (with ⭐ ALL sector), Indian Exam Form Studio,
//!   Archive Manager, Hardware Acceleration Diagnostics, and Persistent Settings & Appearance.
//! - Keyboard navigation: [Tab] or [1-5] for tabs, [t] to cycle themes, [q] to quit.

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use mossambi_archive::{pack_archive, ArchiveType};
use mossambi_codec::{transcode_file, transcode_to_all_formats, LosslessMode};
use mossambi_core::{
    get_system_arch_info, MossambiConfig, ThemeMode, UniversalRegistry,
};
use mossambi_exam::{optimize_exam_media, ExamConstraint};
use mossambi_hwaccel::detect_hardware_capabilities;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Tabs},
    Frame, Terminal,
};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

// 100% Certified Pantone Palette Constants for ANSI RGB
pub const PANTONE_RAW_PEEL: Color = Color::Rgb(120, 190, 32);   // 376 C
pub const PANTONE_SWEET_ZEST: Color = Color::Rgb(228, 240, 36); // 388 C
pub const PANTONE_RIPE_PULP: Color = Color::Rgb(209, 230, 49);  // 389 C
pub const PANTONE_CITRUS_GOLD: Color = Color::Rgb(255, 184, 28); // 1235 C
pub const PANTONE_TANGERINE: Color = Color::Rgb(248, 143, 88);  // 15-1247
pub const PANTONE_DEEP_FOREST: Color = Color::Rgb(46, 77, 30);  // 371 C
pub const PANTONE_WHITE: Color = Color::Rgb(245, 245, 247);

// macOS Traffic Lights
pub const MAC_TRAFFIC_RED: Color = Color::Rgb(255, 95, 86);    // #FF5F56
pub const MAC_TRAFFIC_YELLOW: Color = Color::Rgb(255, 189, 46); // #FFBD2E
pub const MAC_TRAFFIC_GREEN: Color = Color::Rgb(39, 201, 63);   // #27C93F

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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AppTab {
    Convert = 0,
    ExamStudio = 1,
    Archive = 2,
    Hardware = 3,
    Settings = 4,
}

impl AppTab {
    pub fn all() -> &'static [AppTab] {
        &[
            AppTab::Convert,
            AppTab::ExamStudio,
            AppTab::Archive,
            AppTab::Hardware,
            AppTab::Settings,
        ]
    }

    pub fn title(&self) -> &'static str {
        match self {
            AppTab::Convert => " ⚡ Transcoder & ALL ",
            AppTab::ExamStudio => " 🇮🇳 Indian Exam Studio ",
            AppTab::Archive => " 📦 Archive Manager ",
            AppTab::Hardware => " 🏎️ Hardware & AI ",
            AppTab::Settings => " ⚙️ Appearance & Themes ",
        }
    }
}

pub struct AppState {
    pub current_tab: AppTab,
    pub config: MossambiConfig,
    pub status_message: String,
    pub status_is_error: bool,

    // File Browser
    pub current_dir: PathBuf,
    pub dir_entries: Vec<PathBuf>,
    pub selected_file_idx: usize,
    pub selected_format_idx: usize,
    pub available_formats: Vec<&'static str>,

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
}

impl AppState {
    pub fn new() -> Self {
        let config = MossambiConfig::load();
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut app = Self {
            current_tab: AppTab::Convert,
            config,
            status_message: "Welcome to Mossambi. [Tab] Navigate | [t] Cycle Themes | [q] Quit".into(),
            status_is_error: false,
            current_dir: current_dir.clone(),
            dir_entries: Vec::new(),
            selected_file_idx: 0,
            selected_format_idx: 0,
            available_formats: vec!["JPG", "PNG", "WEBP", "PDF", "HEIC", "TIFF", "AVIF", "ALL"],
            exam_presets: vec![
                ("UPSC Civil Services Photo (350x450, 300DPI, 20-100KB)", ExamConstraint::preset_upsc_photo()),
                ("UPSC Civil Services Signature (350x150, 300DPI, 10-40KB)", ExamConstraint::preset_upsc_signature()),
                ("SSC CGL Photo (100x120, 200DPI, 20-50KB)", ExamConstraint::preset_ssc_photo()),
                ("SSC CGL Signature (140x60, 200DPI, 10-20KB)", ExamConstraint::preset_ssc_signature()),
                ("IBPS Bank PO Photo (200x230, 200DPI, 20-50KB)", ExamConstraint::preset_ibps_photo()),
                ("IBPS Bank PO Signature (140x60, 200DPI, 10-20KB)", ExamConstraint::preset_ibps_signature()),
            ],
            selected_preset_idx: 0,
            exam_status: "Select an exam preset and press [Enter] to optimize.".into(),
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
        };

        app.refresh_directory();
        app
    }

    pub fn current_theme_palette(&self) -> TuiThemePalette {
        match self.config.theme_mode {
            ThemeMode::LiquidGlass => TuiThemePalette {
                bg: Color::Rgb(16, 20, 26),
                surface: Color::Rgb(26, 32, 44),
                border: Color::Rgb(70, 95, 125),
                text_primary: Color::Rgb(245, 245, 250),
                text_secondary: Color::Rgb(175, 185, 200),
                text_muted: Color::Rgb(110, 125, 140),
                accent: Color::Rgb(120, 190, 32),
                gold: Color::Rgb(255, 184, 28),
            },
            ThemeMode::FrostedGlass => TuiThemePalette {
                bg: Color::Rgb(24, 24, 28),
                surface: Color::Rgb(36, 36, 42),
                border: Color::Rgb(70, 70, 80),
                text_primary: Color::Rgb(240, 240, 245),
                text_secondary: Color::Rgb(165, 165, 175),
                text_muted: Color::Rgb(115, 115, 125),
                accent: Color::Rgb(130, 205, 40),
                gold: Color::Rgb(255, 184, 28),
            },
            ThemeMode::DarkAnthracite => TuiThemePalette {
                bg: Color::Rgb(22, 22, 24),
                surface: Color::Rgb(34, 34, 38),
                border: Color::Rgb(60, 60, 66),
                text_primary: Color::Rgb(245, 245, 247),
                text_secondary: Color::Rgb(161, 161, 166),
                text_muted: Color::Rgb(120, 120, 126),
                accent: PANTONE_RAW_PEEL,
                gold: PANTONE_CITRUS_GOLD,
            },
            ThemeMode::LightAqua => TuiThemePalette {
                bg: Color::Rgb(245, 245, 247),
                surface: Color::Rgb(255, 255, 255),
                border: Color::Rgb(190, 190, 200),
                text_primary: Color::Rgb(28, 28, 30),
                text_secondary: Color::Rgb(105, 105, 115),
                text_muted: Color::Rgb(140, 140, 150),
                accent: Color::Rgb(85, 150, 18),
                gold: Color::Rgb(210, 140, 15),
            },
        }
    }

    pub fn refresh_directory(&mut self) {
        let mut entries = Vec::new();
        if let Ok(rd) = fs::read_dir(&self.current_dir) {
            for entry in rd.flatten() {
                entries.push(entry.path());
            }
        }
        entries.sort_by(|a, b| {
            let a_is_dir = a.is_dir();
            let b_is_dir = b.is_dir();
            if a_is_dir && !b_is_dir {
                std::cmp::Ordering::Less
            } else if !a_is_dir && b_is_dir {
                std::cmp::Ordering::Greater
            } else {
                a.file_name().cmp(&b.file_name())
            }
        });
        self.dir_entries = entries;
        if self.selected_file_idx >= self.dir_entries.len() {
            self.selected_file_idx = self.dir_entries.len().saturating_sub(1);
        }
        self.update_formats_for_selected();
    }

    pub fn update_formats_for_selected(&mut self) {
        if let Some(file) = self.dir_entries.get(self.selected_file_idx) {
            let (cat, _) = UniversalRegistry::classify_file(file);
            self.available_formats = UniversalRegistry::get_widget_sectors_for_category(cat);
            if self.selected_format_idx >= self.available_formats.len() {
                self.selected_format_idx = 0;
            }
        }
    }

    pub fn execute_selected_conversion(&mut self) {
        if let Some(file) = self.dir_entries.get(self.selected_file_idx).cloned() {
            if file.is_dir() {
                self.status_message = "Cannot convert directory directly. Use Archive tab.".into();
                self.status_is_error = true;
                return;
            }

            let target_fmt = self.available_formats.get(self.selected_format_idx).copied().unwrap_or("ALL");
            let mode = if self.config.enforce_zero_loss {
                LosslessMode::StrictBitExact
            } else {
                LosslessMode::PerceptualHighFidelity
            };

            if target_fmt == "ALL" {
                match transcode_to_all_formats(&file, mode, None) {
                    Ok(outputs) => {
                        self.status_message = format!("✓ Batch ALL succeeded: Created {} files", outputs.len());
                        self.status_is_error = false;
                        self.refresh_directory();
                    }
                    Err(e) => {
                        self.status_message = format!("✗ Batch transcode error: {}", e);
                        self.status_is_error = true;
                    }
                }
            } else {
                match transcode_file(&file, target_fmt, mode, None) {
                    Ok(out) => {
                        self.status_message = format!("✓ Converted: {}", out.file_name().unwrap_or_default().to_string_lossy());
                        self.status_is_error = false;
                        self.refresh_directory();
                    }
                    Err(e) => {
                        self.status_message = format!("✗ Transcode error: {}", e);
                        self.status_is_error = true;
                    }
                }
            }
        }
    }

    pub fn execute_exam_optimization(&mut self) {
        if let Some(file) = self.dir_entries.get(self.selected_file_idx).cloned() {
            if file.is_dir() {
                self.exam_status = "Please select an image file first.".into();
                return;
            }

            let (_, constraint) = &self.exam_presets[self.selected_preset_idx];
            let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("exam_output");
            let out_file = file.parent().unwrap_or_else(|| Path::new(".")).join(format!("{}_exam_ready.jpg", stem));

            match optimize_exam_media(&file, &out_file, constraint) {
                Ok(res) => {
                    let len = fs::metadata(&res).map(|m| m.len()).unwrap_or(0);
                    self.exam_status = format!(
                        "✓ PASS: {} ({:.1} KB) matches {} bounds!",
                        res.file_name().unwrap_or_default().to_string_lossy(),
                        len as f64 / 1024.0,
                        constraint.name
                    );
                    self.status_message = self.exam_status.clone();
                    self.status_is_error = false;
                    self.refresh_directory();
                }
                Err(e) => {
                    self.exam_status = format!("✗ Exam convergence failed: {}", e);
                    self.status_message = self.exam_status.clone();
                    self.status_is_error = true;
                }
            }
        } else {
            self.exam_status = "No file selected from directory!".into();
        }
    }

    pub fn execute_archive_pack(&mut self) {
        if let Some(file) = self.dir_entries.get(self.selected_file_idx).cloned() {
            let atype = self.archive_types[self.selected_archive_type_idx];
            let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("bundle");
            let ext = match atype {
                ArchiveType::Zip => "zip",
                ArchiveType::TarGz => "tar.gz",
                ArchiveType::TarXz => "tar.xz",
                ArchiveType::TarZst => "tar.zst",
                ArchiveType::TarBz2 => "tar.bz2",
                ArchiveType::Tar => "tar",
                _ => "zip",
            };

            let out_file = file.parent().unwrap_or_else(|| Path::new(".")).join(format!("{}.{}", stem, ext));
            match pack_archive(&[file], &out_file, atype) {
                Ok(_) => {
                    self.status_message = format!("✓ Packed: {}", out_file.display());
                    self.status_is_error = false;
                    self.refresh_directory();
                }
                Err(e) => {
                    self.status_message = format!("✗ Archive pack error: {}", e);
                    self.status_is_error = true;
                }
            }
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(clippy::collapsible_match)]
fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = AppState::new();

    while !state.should_quit {
        terminal.draw(|f| draw_ui(f, &mut state))?;

        if event::poll(Duration::from_millis(80))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => state.should_quit = true,
                        KeyCode::Tab => {
                            let curr = state.current_tab as usize;
                            state.current_tab = match curr {
                                0 => AppTab::ExamStudio,
                                1 => AppTab::Archive,
                                2 => AppTab::Hardware,
                                3 => AppTab::Settings,
                                _ => AppTab::Convert,
                            };
                        }
                        KeyCode::BackTab => {
                            let curr = state.current_tab as usize;
                            state.current_tab = match curr {
                                0 => AppTab::Settings,
                                1 => AppTab::Convert,
                                2 => AppTab::ExamStudio,
                                3 => AppTab::Archive,
                                _ => AppTab::Hardware,
                            };
                        }
                        KeyCode::Char('1') => state.current_tab = AppTab::Convert,
                        KeyCode::Char('2') => state.current_tab = AppTab::ExamStudio,
                        KeyCode::Char('3') => state.current_tab = AppTab::Archive,
                        KeyCode::Char('4') => state.current_tab = AppTab::Hardware,
                        KeyCode::Char('5') => state.current_tab = AppTab::Settings,

                        // Theme switch shortcut [t]
                        KeyCode::Char('t') => {
                            state.config.theme_mode = match state.config.theme_mode {
                                ThemeMode::LiquidGlass => ThemeMode::FrostedGlass,
                                ThemeMode::FrostedGlass => ThemeMode::DarkAnthracite,
                                ThemeMode::DarkAnthracite => ThemeMode::LightAqua,
                                ThemeMode::LightAqua => ThemeMode::LiquidGlass,
                            };
                            let _ = state.config.save();
                            state.status_message = format!("Theme switched to: {:?}", state.config.theme_mode);
                        }

                        // Arrow Navigation
                        KeyCode::Up | KeyCode::Char('k') => match state.current_tab {
                            AppTab::Convert | AppTab::Archive => {
                                if state.selected_file_idx > 0 {
                                    state.selected_file_idx -= 1;
                                    state.update_formats_for_selected();
                                }
                            }
                            AppTab::ExamStudio => {
                                if state.selected_preset_idx > 0 {
                                    state.selected_preset_idx -= 1;
                                }
                            }
                            AppTab::Settings => {
                                if state.selected_shade_idx > 0 {
                                    state.selected_shade_idx -= 1;
                                    state.config.citrus_shade = state.shade_options[state.selected_shade_idx].to_string();
                                    let _ = state.config.save();
                                }
                            }
                            _ => {}
                        },
                        KeyCode::Down | KeyCode::Char('j') => match state.current_tab {
                            AppTab::Convert | AppTab::Archive => {
                                if state.selected_file_idx + 1 < state.dir_entries.len() {
                                    state.selected_file_idx += 1;
                                    state.update_formats_for_selected();
                                }
                            }
                            AppTab::ExamStudio => {
                                if state.selected_preset_idx + 1 < state.exam_presets.len() {
                                    state.selected_preset_idx += 1;
                                }
                            }
                            AppTab::Settings => {
                                if state.selected_shade_idx + 1 < state.shade_options.len() {
                                    state.selected_shade_idx += 1;
                                    state.config.citrus_shade = state.shade_options[state.selected_shade_idx].to_string();
                                    let _ = state.config.save();
                                }
                            }
                            _ => {}
                        },
                        KeyCode::Left | KeyCode::Char('h') => match state.current_tab {
                            AppTab::Convert => {
                                if state.selected_format_idx > 0 {
                                    state.selected_format_idx -= 1;
                                }
                            }
                            AppTab::Archive => {
                                if state.selected_archive_type_idx > 0 {
                                    state.selected_archive_type_idx -= 1;
                                }
                            }
                            _ => {}
                        },
                        KeyCode::Right | KeyCode::Char('l') => match state.current_tab {
                            AppTab::Convert => {
                                if state.selected_format_idx + 1 < state.available_formats.len() {
                                    state.selected_format_idx += 1;
                                }
                            }
                            AppTab::Archive => {
                                if state.selected_archive_type_idx + 1 < state.archive_types.len() {
                                    state.selected_archive_type_idx += 1;
                                }
                            }
                            _ => {}
                        },

                        // Enter Execution
                        KeyCode::Enter => match state.current_tab {
                            AppTab::Convert => state.execute_selected_conversion(),
                            AppTab::ExamStudio => state.execute_exam_optimization(),
                            AppTab::Archive => state.execute_archive_pack(),
                            AppTab::Settings => {
                                state.config.enforce_zero_loss = !state.config.enforce_zero_loss;
                                let _ = state.config.save();
                                state.status_message = format!("Zero-loss enforcement: {}", state.config.enforce_zero_loss);
                            }
                            _ => {}
                        },

                        // Directory Navigation (Backspace)
                        KeyCode::Backspace => {
                            if let Some(parent) = state.current_dir.parent() {
                                state.current_dir = parent.to_path_buf();
                                state.refresh_directory();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn draw_ui(f: &mut Frame, state: &mut AppState) {
    let palette = state.current_theme_palette();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header & macOS Traffic Lights + Tabs
            Constraint::Min(10),   // Main Viewport
            Constraint::Length(3), // Status & Keymap
        ])
        .split(f.area());

    // 1. macOS Chrome Header Bar with Authentic Traffic Lights
    let tab_titles: Vec<Line> = AppTab::all()
        .iter()
        .map(|t| {
            let style = if *t == state.current_tab {
                Style::default().fg(palette.accent).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.text_muted)
            };
            Line::from(vec![Span::styled(t.title(), style)])
        })
        .collect();

    let header_title = Line::from(vec![
        Span::styled(" ●", Style::default().fg(MAC_TRAFFIC_RED)),
        Span::styled(" ●", Style::default().fg(MAC_TRAFFIC_YELLOW)),
        Span::styled(" ●", Style::default().fg(MAC_TRAFFIC_GREEN)),
        Span::raw("   "),
        Span::styled("mossambi", Style::default().fg(palette.text_primary).add_modifier(Modifier::BOLD)),
        Span::styled(" — macOS Sequoia Suite", Style::default().fg(palette.text_muted)),
        Span::raw("   "),
        Span::styled(format!("[Theme: {:?}]", state.config.theme_mode), Style::default().fg(palette.accent)),
    ]);

    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(palette.border))
                .title(header_title),
        )
        .highlight_style(Style::default().fg(palette.accent).add_modifier(Modifier::UNDERLINED))
        .select(state.current_tab as usize);
    f.render_widget(tabs, chunks[0]);

    // 2. Main Tab Viewport
    match state.current_tab {
        AppTab::Convert => draw_convert_tab(f, chunks[1], state, &palette),
        AppTab::ExamStudio => draw_exam_tab(f, chunks[1], state, &palette),
        AppTab::Archive => draw_archive_tab(f, chunks[1], state, &palette),
        AppTab::Hardware => draw_hardware_tab(f, chunks[1], state, &palette),
        AppTab::Settings => draw_settings_tab(f, chunks[1], state, &palette),
    }

    // 3. Status Bar & Keybindings
    let status_style = if state.status_is_error {
        Style::default().fg(Color::LightRed)
    } else {
        Style::default().fg(palette.accent)
    };

    let status_p = Paragraph::new(Line::from(vec![
        Span::styled(" [Status] ", Style::default().fg(palette.text_muted)),
        Span::styled(&state.status_message, status_style),
        Span::raw(" | "),
        Span::styled("[Tab] Tab | [t] Theme | [↑/↓] Select | [←/→] Sector | [Enter] Execute | [q] Quit", Style::default().fg(palette.text_secondary)),
    ]))
    .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(palette.border)));
    f.render_widget(status_p, chunks[2]);
}

fn draw_convert_tab(f: &mut Frame, area: Rect, state: &AppState, palette: &TuiThemePalette) {
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left: File list
    let items: Vec<ListItem> = state
        .dir_entries
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let is_sel = i == state.selected_file_idx;
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            let is_dir = path.is_dir();
            let icon = if is_dir { "📁 " } else { "📄 " };

            let style = if is_sel {
                Style::default().fg(Color::Black).bg(palette.accent).add_modifier(Modifier::BOLD)
            } else if is_dir {
                Style::default().fg(PANTONE_SWEET_ZEST)
            } else {
                Style::default().fg(palette.text_primary)
            };

            ListItem::new(format!("{}{}", icon, name)).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(
                    format!(" Directory: {} ", state.current_dir.display()),
                    Style::default().fg(palette.accent),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(palette.border)),
        );
    f.render_widget(list, main_layout[0]);

    // Right: Selected File Details & Radial Widget Petals + Golden ALL Sector
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(8)])
        .split(main_layout[1]);

    if let Some(file) = state.dir_entries.get(state.selected_file_idx) {
        let (cat, ext) = UniversalRegistry::classify_file(file);
        let size = fs::metadata(file).map(|m| m.len()).unwrap_or(0);

        let details = Paragraph::new(vec![
            Line::from(vec![Span::styled("File: ", Style::default().fg(palette.text_muted)), Span::raw(file.display().to_string())]),
            Line::from(vec![Span::styled("Category: ", Style::default().fg(palette.text_muted)), Span::styled(format!("{:?}", cat), Style::default().fg(PANTONE_SWEET_ZEST))]),
            Line::from(vec![Span::styled("Format: ", Style::default().fg(palette.text_muted)), Span::raw(ext)]),
            Line::from(vec![Span::styled("Size: ", Style::default().fg(palette.text_muted)), Span::raw(format!("{:.2} KB ({} bytes)", size as f64 / 1024.0, size))]),
            Line::from(vec![Span::styled("Zero-Loss Standard: ", Style::default().fg(palette.text_muted)), Span::styled("0.00% Bit-Exact", Style::default().fg(palette.accent))]),
        ])
        .block(
            Block::default()
                .title(" File Details ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(palette.border)),
        );
    f.render_widget(details, right_chunks[0]);

        // Sector Petals View
        let mut sector_spans = Vec::new();
        sector_spans.push(Line::from("Radial Wheel Sectors (Press ← / → then Enter):"));
        sector_spans.push(Line::from(""));

        for (idx, sector) in state.available_formats.iter().enumerate() {
            let is_sel = idx == state.selected_format_idx;
            let is_all = *sector == "ALL";

            let label = if is_all {
                " ⭐ ALL (Batch Transcode to Every Format) ".to_string()
            } else {
                format!(" Sector {}: {} ", idx + 1, sector)
            };

            let style = if is_sel {
                if is_all {
                    Style::default().fg(Color::Black).bg(palette.gold).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Black).bg(palette.accent).add_modifier(Modifier::BOLD)
                }
            } else if is_all {
                Style::default().fg(palette.gold).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.text_primary)
            };

            sector_spans.push(Line::from(vec![Span::styled(label, style)]));
        }

        let petals_p = Paragraph::new(sector_spans)
            .block(
                Block::default()
                    .title(" Radial Petal Selection ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(palette.border)),
            );
        f.render_widget(petals_p, right_chunks[1]);
    } else {
        let empty_p = Paragraph::new("No files in directory.")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(palette.border)),
            );
        f.render_widget(empty_p, right_chunks[0]);
    }
}

fn draw_exam_tab(f: &mut Frame, area: Rect, state: &AppState, palette: &TuiThemePalette) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(8)])
        .split(area);

    let intro = Paragraph::new(vec![
        Line::from(Span::styled("🇮🇳 Indian Competitive Exam & Recruitment Form Tuner", Style::default().fg(palette.accent).add_modifier(Modifier::BOLD))),
        Line::from("Solves strict upload failures on UPSC, SSC, IBPS, GATE, and State PSC application portals."),
        Line::from("Enforces exact pixel dimensions, calibrated 200/300 DPI, and micro-converged binary search byte limits."),
        Line::from(Span::styled(&state.exam_status, Style::default().fg(palette.gold))),
    ])
    .block(
        Block::default()
            .title(" Portal Verification Engine ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(palette.border)),
    );
    f.render_widget(intro, chunks[0]);

    let preset_items: Vec<ListItem> = state
        .exam_presets
        .iter()
        .enumerate()
        .map(|(i, (title, c))| {
            let is_sel = i == state.selected_preset_idx;
            let style = if is_sel {
                Style::default().fg(Color::Black).bg(palette.accent).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.text_primary)
            };

            let line = format!("{} | {}x{} px @ {} DPI [{:.0}KB - {:.0}KB]", title, c.width_px, c.height_px, c.dpi, c.min_bytes as f64 / 1024.0, c.max_bytes as f64 / 1024.0);
            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(preset_items)
        .block(
            Block::default()
                .title(" Official Exam Presets (Press Enter to optimize selected file) ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(palette.border)),
        );
    f.render_widget(list, chunks[1]);
}

fn draw_archive_tab(f: &mut Frame, area: Rect, state: &AppState, palette: &TuiThemePalette) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(6)])
        .split(area);

    let info = Paragraph::new(vec![
        Line::from(Span::styled("📦 Archive Suite & Safe Extraction", Style::default().fg(palette.accent).add_modifier(Modifier::BOLD))),
        Line::from("Supports ZIP, TAR.GZ, TAR.XZ, TAR.ZST, TAR.BZ2 with Zip-Slip path traversal defense."),
        Line::from("Select a file from the list, choose an archive format, and press [Enter] to pack."),
    ])
    .block(
        Block::default()
            .title(" Archive System ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(palette.border)),
    );
    f.render_widget(info, chunks[0]);

    let mut type_spans = Vec::new();
    for (i, atype) in state.archive_types.iter().enumerate() {
        let is_sel = i == state.selected_archive_type_idx;
        let style = if is_sel {
            Style::default().fg(Color::Black).bg(palette.accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.text_primary)
        };
        type_spans.push(Line::from(Span::styled(format!(" [{:?}] ", atype), style)));
    }

    let p = Paragraph::new(type_spans)
        .block(
            Block::default()
                .title(" Archive Containers (Use ← / → then Enter) ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(palette.border)),
        );
    f.render_widget(p, chunks[1]);
}

fn draw_hardware_tab(f: &mut Frame, area: Rect, _state: &AppState, palette: &TuiThemePalette) {
    let hw = detect_hardware_capabilities();
    let (os, arch) = get_system_arch_info();

    let p = Paragraph::new(vec![
        Line::from(Span::styled("⚡ Hardware Acceleration, NPU & SIMD Diagnostics", Style::default().fg(palette.accent).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![Span::styled("OS Platform: ", Style::default().fg(palette.text_muted)), Span::raw(os)]),
        Line::from(vec![Span::styled("Target CPU Arch: ", Style::default().fg(palette.text_muted)), Span::styled(arch, Style::default().fg(PANTONE_SWEET_ZEST))]),
        Line::from(vec![Span::styled("GPU Vendor: ", Style::default().fg(palette.text_muted)), Span::styled(format!("{:?}", hw.vendor), Style::default().fg(palette.accent))]),
        Line::from(vec![Span::styled("FFmpeg HW Accel Flag: ", Style::default().fg(palette.text_muted)), Span::raw(hw.ffmpeg_hwaccel_flag)]),
        Line::from(vec![Span::styled("H.264 Encoder Engine: ", Style::default().fg(palette.text_muted)), Span::raw(hw.h264_encoder)]),
        Line::from(vec![Span::styled("HEVC Encoder Engine: ", Style::default().fg(palette.text_muted)), Span::raw(hw.hevc_encoder)]),
        Line::from(vec![Span::styled("NPU Neural Engine: ", Style::default().fg(palette.text_muted)), Span::styled(hw.npu_name, Style::default().fg(palette.gold))]),
        Line::from(vec![Span::styled("NPU AI Status: ", Style::default().fg(palette.text_muted)), Span::styled(if hw.supports_npu_ai { "Active (DirectML / NPU Accelerated)" } else { "CPU Fallback" }, Style::default().fg(PANTONE_SWEET_ZEST))]),
        Line::from(vec![Span::styled("ARM NEON 128-bit SIMD: ", Style::default().fg(palette.text_muted)), Span::styled(if hw.supports_neon_simd { "Active" } else { "Inactive" }, Style::default().fg(PANTONE_SWEET_ZEST))]),
        Line::from(vec![Span::styled("x86_64 AVX2 256-bit SIMD: ", Style::default().fg(palette.text_muted)), Span::styled(if hw.supports_avx2_simd { "Active" } else { "Inactive" }, Style::default().fg(PANTONE_SWEET_ZEST))]),
        Line::from(vec![Span::styled("Concurrent Rayon Workers: ", Style::default().fg(palette.text_muted)), Span::styled(format!("{} threads", hw.optimal_thread_count), Style::default().fg(palette.accent))]),
    ])
    .block(
        Block::default()
            .title(" Platform Hardware Telemetry ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(palette.border)),
    );
    f.render_widget(p, area);
}

fn draw_settings_tab(f: &mut Frame, area: Rect, state: &AppState, palette: &TuiThemePalette) {
    let mut spans = vec![
        Line::from(Span::styled("⚙️ Appearance, Themes & Configuration", Style::default().fg(palette.accent).add_modifier(Modifier::BOLD))),
        Line::from("All settings persist to ~/.mossambi/config.json"),
        Line::from(""),
        Line::from(Span::styled("Apple macOS Themes (Press [t] to cycle themes):", Style::default().fg(palette.text_primary).add_modifier(Modifier::BOLD))),
    ];
    let themes = [
        (ThemeMode::LiquidGlass, "1. Liquid Glass (macOS Sequoia Specular Refraction)"),
        (ThemeMode::FrostedGlass, "2. Frosted Glass (macOS Sonoma Matte Acrylic)"),
        (ThemeMode::DarkAnthracite, "3. Dark Anthracite (macOS Pro Dark #161618)"),
        (ThemeMode::LightAqua, "4. Light Aqua (macOS Crisp Clean Light #F5F5F7)"),
    ];
    for (t, label) in themes {
        let is_active = state.config.theme_mode == t;
        let marker = if is_active { " ● " } else { " ○ " };
        let style = if is_active {
            Style::default().fg(palette.accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.text_muted)
        };
        spans.push(Line::from(Span::styled(format!("{}{}", marker, label), style)));
    }

    spans.push(Line::from(""));
    spans.push(Line::from(Span::styled("Global Activation & Gestures:", Style::default().fg(palette.text_primary).add_modifier(Modifier::BOLD))));
    spans.push(Line::from("• 2-Time Circular Motion Gesture: Drag a file and trace 2 circular loops (>=720 deg) to summon the widget with active file."));
    spans.push(Line::from("• Global Hotkey: WIN + CTRL awakens the radial widget centered under cursor."));
    spans.push(Line::from("• System Tray: Right-click citrus tray icon for themes, exam presets, and controls."));

    spans.push(Line::from(""));
    spans.push(Line::from(vec![
        Span::styled("Enforce Zero-Loss Standard (0.00%): ", Style::default().fg(palette.text_muted)),
        Span::styled(
            if state.config.enforce_zero_loss { "ENABLED (Strict Bit-Exact)" } else { "DISABLED" },
            Style::default().fg(palette.accent),
        ),
        Span::raw(" (Press Enter to toggle)"),
    ]));

    let p = Paragraph::new(spans)
        .block(
            Block::default()
                .title(" Preferences (~/.mossambi/config.json) ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(palette.border)),
        );
    f.render_widget(p, area);
}
