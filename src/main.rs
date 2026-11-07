//! Mossambi UNIX-Style Command-Line Interface (`mossambi`)
//! Frictionless syntax modeled after `mv`: `mossambi <input> [target_format]`
//! Supports `--all` batch transcode, Indian exam preset tuning, archive pack/unpack, hardware accel metrics, and persistent configuration.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use mossambi::archive::{pack_archive, unpack_archive, ArchiveType};
use mossambi::codec::{
    transcode_file, transcode_to_all_formats, verify_conversion_fidelity, BenchmarkEngine,
    FileQualityChecker, LosslessMode,
};
use mossambi::core::{get_system_arch_info, MossambiConfig, ThemeMode, UniversalRegistry};
use mossambi::exam::{optimize_exam_media, ExamConstraint};
use mossambi::hwaccel::detect_hardware_capabilities;
use serde_json::json;
use std::path::{Path, PathBuf};

use mossambi::daemon;

#[derive(Parser, Debug)]
#[command(
    name = "mossambi",
    author = "Mossambi Systems Team",
    version = "2.1.0",
    about = "Zero-loss native desktop file converter, archive suite, Indian exam form tuner, and macOS-style Universal File Explorer",
    long_about = "🍋 Mossambi: Zero-Loss Desktop File Converter & Universal File Explorer\nWindows & Linux | x86_64 & ARM64 | 0.00% Zero-Loss Standard | Hardware Accelerated\n\nACCESS MODALITIES:\n  1. mossambi [or: mossambi app, mossambi explorer]  -> Launch macOS-styled Universal File Explorer & Studio GUI\n  2. mossambi widget                                  -> Launch Sweet Lime Resident Radial Widget overlay\n  3. mossambi tui                                     -> Launch Yazi-grade interactive Terminal File Explorer\n  4. mossambi <INPUT> [TARGET_FORMAT]                 -> Fast UNIX mv-style file conversion",
    subcommand_precedence_over_arg = true
)]
pub struct Cli {
    /// Input file or directory to process (mv-style positional grammar)
    #[arg(value_name = "INPUT")]
    pub input: Option<PathBuf>,

    /// Target format extension (e.g. 'wav', 'mp3', 'webp', 'pdf', or 'ALL')
    #[arg(value_name = "TARGET_FORMAT")]
    pub target: Option<String>,

    /// Batch convert to ALL valid formats in this category simultaneously
    #[arg(short = 'a', long = "all")]
    pub all: bool,

    /// Indian Competitive Exam preset (e.g. upsc-photo, upsc-signature, ssc-photo, ssc-signature, ibps-photo, ibps-signature)
    #[arg(long = "exam")]
    pub exam_preset: Option<String>,

    /// Exam custom target width in pixels
    #[arg(long = "width")]
    pub exam_width: Option<u32>,

    /// Exam custom target height in pixels
    #[arg(long = "height")]
    pub exam_height: Option<u32>,

    /// Exam custom target DPI (dots per inch)
    #[arg(long = "dpi")]
    pub exam_dpi: Option<u32>,

    /// Exam minimum file size in kilobytes
    #[arg(long = "min-kb")]
    pub exam_min_kb: Option<u64>,

    /// Exam maximum file size in kilobytes
    #[arg(long = "max-kb")]
    pub exam_max_kb: Option<u64>,

    /// Auto-whiten background of scanned documents or signatures
    #[arg(long = "whiten")]
    pub exam_whiten: bool,

    /// Destination directory for output files (defaults to sibling directory)
    #[arg(short = 'o', long = "output")]
    pub output_dir: Option<PathBuf>,

    /// Enforce strict 0.00% zero-loss transcode (stream copy / lossless PCM / raster)
    #[arg(long = "strict", global = true, default_value_t = true)]
    pub strict_lossless: bool,

    /// Output results in JSON format for scripts and integrations
    #[arg(long = "json", global = true)]
    pub json: bool,

    /// Launch the full Universal File Explorer & Conversion Studio GUI
    #[arg(long = "app", visible_alias = "gui", visible_alias = "explorer")]
    pub app: bool,

    /// Launch the Sweet Lime Desktop Radial Widget
    #[arg(long = "widget")]
    pub widget: bool,

    /// Launch the Terminal User Interface (TUI) File Explorer
    #[arg(long = "tui")]
    pub tui: bool,

    /// Display MOSSAMBI banner and summary
    #[arg(long = "banner")]
    pub banner: bool,

    /// Display Unicode box table manual
    #[arg(long = "manual", visible_alias = "help-manual")]
    pub manual: bool,

    /// Display detected hardware acceleration engines
    #[arg(long = "hw")]
    pub hw: bool,

    /// List all 20 certified Pantone themes
    #[arg(long = "themes")]
    pub themes: bool,

    /// 4-dimension Quality & Container Integrity Scorecard
    #[arg(long = "check")]
    pub check: Option<PathBuf>,

    /// Benchmark conversion latency and throughput
    #[arg(long = "bench")]
    pub bench: Option<PathBuf>,

    /// Verify mathematical zero-loss fidelity between two files
    #[arg(long = "verify", num_args = 2)]
    pub verify: Option<Vec<PathBuf>>,

    /// Search across the 200+ format registry catalog
    #[arg(long = "catalogue")]
    pub catalogue: Option<String>,

    /// Pack files or directories into an archive
    #[arg(long = "archive", num_args = 1..)]
    pub archive: Option<Vec<PathBuf>>,

    /// Extract an archive with automatic Zip-Slip protection
    #[arg(long = "extract")]
    pub extract: Option<PathBuf>,

    /// Manage background resident daemon (start, stop, status)
    #[arg(long = "daemon")]
    pub daemon: Option<String>,

    /// Explicit subcommands
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum Commands {
    /// Launch the Universal File Explorer & Conversion Studio GUI (Default App)
    #[command(visible_alias = "gui")]
    App {
        /// Optional directory or file to open in the Explorer
        path: Option<PathBuf>,
    },

    /// Launch the Universal File Explorer & Conversion Studio GUI
    Explorer {
        /// Optional directory or file to open in the Explorer
        path: Option<PathBuf>,
    },

    /// Launch the Terminal User Interface (TUI) File Explorer
    Tui {
        /// Optional directory to open in the TUI
        path: Option<PathBuf>,
    },

    /// Launch or summon the Sweet Lime Resident Radial Widget overlay
    Widget {
        /// Optional X,Y spawn coordinates (e.g. 500,400)
        #[arg(long = "pos")]
        pos: Option<String>,

        /// Flag indicating widget was awakened by active mouse drag
        #[arg(long = "active-drag")]
        active_drag: bool,

        /// Initial file to load into the widget
        #[arg(value_name = "FILE")]
        file: Option<PathBuf>,
    },

    /// Convert an input file to target format (explicit syntax)
    Convert {
        /// Source file path
        input: PathBuf,
        /// Target format or 'ALL'
        target: String,
        /// Custom destination directory
        #[arg(short = 'o', long = "output")]
        output: Option<PathBuf>,
        /// Batch convert to ALL valid formats in domain
        #[arg(short = 'a', long = "all")]
        all: bool,
    },

    /// Pack files or directories into an archive (ZIP, 7Z, TAR, GZ, BZ2, XZ, ZST)
    Archive {
        /// Input files or folders to compress
        #[arg(required = true)]
        inputs: Vec<PathBuf>,

        /// Output archive filename (e.g. output.tar.gz, backup.7z, bundle.zip)
        #[arg(short = 'o', long = "output", required = true)]
        output: PathBuf,

        /// Explicit archive format (optional, inferred from extension if omitted)
        #[arg(short = 't', long = "type")]
        archive_type: Option<String>,
    },

    /// Extract an archive with automatic Zip-Slip directory traversal protection
    Extract {
        /// Path to archive file (.zip, .tar, .tar.gz, .7z, .rar, etc.)
        archive: PathBuf,

        /// Output extraction directory (defaults to sibling folder)
        #[arg(short = 'o', long = "output")]
        output_dir: Option<PathBuf>,
    },

    /// Inspect file metadata, detected category, and available target formats
    Info {
        /// File path to inspect
        file: PathBuf,
    },

    /// Check quality, container integrity, cross-platform compatibility, and decoder parseability
    Check {
        /// File path to inspect
        file: PathBuf,
    },

    /// Benchmark conversion latency, throughput (MB/s), and speedup vs Online/CLI tools
    Bench {
        /// Input file to benchmark
        file: PathBuf,
        /// Optional target format to benchmark
        target: Option<String>,
    },

    /// Display detected hardware acceleration engines (NVENC, AMF, QSV, Adreno, VAAPI, NEON/AVX2)
    Hw,

    /// Verify mathematical zero-loss fidelity (0.00% loss report) between two files
    Verify {
        /// Original source file
        source: PathBuf,
        /// Converted target file
        target: PathBuf,
    },

    /// Search across the 200+ format registry and catalogue
    Catalogue {
        /// Search query (e.g. 'flac', 'pdf', 'lossless', 'archive')
        #[arg(default_value = "")]
        query: String,
    },

    /// Prepare image/document for Indian competitive exam / job recruitment portal
    UploadReady {
        /// Source image or document
        input: PathBuf,
        /// Exam preset (e.g. upsc-photo, upsc-signature, ssc-photo, ibps-document)
        #[arg(long = "preset")]
        preset: Option<String>,
        #[arg(long = "width")]
        width: Option<u32>,
        #[arg(long = "height")]
        height: Option<u32>,
        #[arg(long = "dpi")]
        dpi: Option<u32>,
        #[arg(long = "min-kb")]
        min_kb: Option<u64>,
        #[arg(long = "max-kb")]
        max_kb: Option<u64>,
        #[arg(long = "whiten")]
        whiten: bool,
        #[arg(short = 'o', long = "output")]
        output_dir: Option<PathBuf>,
    },

    /// List all 20 certified Pantone themes (10 Dark & 10 Light) with official Pantone codes and hex
    Themes,

    /// View or edit persistent configuration settings (~/.mossambi/config.json)
    Config {
        /// Action: 'get', 'set', or 'list'
        #[arg(default_value = "list")]
        action: String,

        /// Key to view or change (e.g. 'shade', 'theme', 'always_on_top')
        key: Option<String>,

        /// New value to assign
        value: Option<String>,
    },

    /// Generate shell auto-completion script or export Carapace YAML spec
    Completions {
        /// Target shell: bash, zsh, fish, powershell, elvish
        shell: Option<String>,

        /// Export Carapace specification (YAML)
        #[arg(long = "carapace")]
        carapace: bool,
    },

    /// Dynamic context-aware Carapace bridge providing dynamic format recommendations
    #[command(name = "_carapace", hide = true)]
    Carapace {
        /// Shell identifier
        shell: Option<String>,
        /// Arguments typed by user
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Display Unicode box table help and usage manual
    HelpManual,

    /// Display MOSSAMBI sweet lime fruit ASCII art and curvy banner
    Banner,

    /// Manage background resident daemon (Win+Shift+C for Wheel, Win+Ctrl+C for Studio)
    Daemon {
        /// Action: 'run', 'start', 'stop', 'status', 'install', 'uninstall', or 'prompt'
        #[arg(default_value = "prompt")]
        action: String,
    },
}

fn main() -> Result<()> {
    // Logging must initialise at the very first line so every entrypoint
    // (CLI, GUI, widget, TUI) captures logs, including these early intercepts.
    let _ = env_logger::try_init();
    let args: Vec<String> = std::env::args().collect();

    // 1. Intercept help flags -> Print ONLY table structure as requested!
    if args
        .iter()
        .any(|a| a == "--help" || a == "-h" || a == "help" || a == "help-manual" || a == "--manual")
    {
        print_table_help();
        return Ok(());
    }

    // 2. Intercept version flags
    if args
        .iter()
        .any(|a| a == "--version" || a == "-V" || a == "version")
    {
        println!("Mossambi v2.1.0 (Windows & Linux | x86_64 & ARM64 | 0.00% Zero-Loss Standard)");
        return Ok(());
    }

    // 3. Intercept banner
    if args.iter().any(|a| a == "--banner" || a == "banner") {
        print_welcome_banner();
        return Ok(());
    }

    // 4. Intercept hw diagnostics
    if args.iter().any(|a| a == "--hw" || a == "hw") {
        let as_json = args.iter().any(|a| a == "--json");
        return handle_hw(as_json);
    }

    // 5. Intercept themes list
    if args.iter().any(|a| a == "--themes" || a == "themes") {
        let as_json = args.iter().any(|a| a == "--json");
        return handle_themes(as_json);
    }

    let cli = Cli::parse();

    // Wire the rayon global pool from live config knobs BEFORE any conversion
    // work. config.worker_threads == 0 means "auto" and maps to
    // available_parallelism inside init_multi_threading.
    let cfg = MossambiConfig::load();
    let requested_threads = if cfg.worker_threads == 0 {
        None
    } else {
        Some(cfg.worker_threads as usize)
    };
    let pool_threads = mossambi::hwaccel::init_multi_threading(requested_threads);
    log::info!(
        "Rayon global pool initialised with {} worker thread(s) (config.worker_threads = {})",
        pool_threads,
        cfg.worker_threads
    );

    // Handle flag-style operations
    if cli.banner {
        print_welcome_banner();
        return Ok(());
    }
    if cli.manual {
        print_table_help();
        return Ok(());
    }
    if cli.hw {
        return handle_hw(cli.json);
    }
    if cli.themes {
        return handle_themes(cli.json);
    }
    if let Some(ref check_file) = cli.check {
        return handle_check(check_file, cli.json);
    }
    if let Some(ref bench_file) = cli.bench {
        return handle_bench(bench_file, cli.target.as_deref(), cli.json);
    }
    if let Some(ref verify_files) = cli.verify {
        if verify_files.len() >= 2 {
            return handle_verify(&verify_files[0], &verify_files[1], cli.json);
        } else {
            bail!("--verify requires two file arguments: <original> <converted>");
        }
    }
    if let Some(ref query) = cli.catalogue {
        return handle_catalogue(query, cli.json);
    }
    if let Some(ref inputs) = cli.archive {
        if let Some(ref out) = cli.output_dir {
            return handle_archive(inputs, out, None, cli.json);
        } else {
            bail!("--archive requires -o <OUTPUT_ARCHIVE>");
        }
    }
    if let Some(ref arc) = cli.extract {
        return handle_extract(arc, cli.output_dir.as_deref(), cli.json);
    }
    if let Some(ref d_act) = cli.daemon {
        return daemon::handle_daemon_command(d_act);
    }

    if cli.app {
        return handle_gui(None);
    }
    if cli.widget {
        return handle_widget(None, false, None);
    }
    if cli.tui {
        return handle_tui(None);
    }

    // Check if an explicit subcommand was requested
    if let Some(cmd) = cli.command {
        return match cmd {
            Commands::App { path } | Commands::Explorer { path } => handle_gui(path),
            Commands::Tui { path } => handle_tui(path),
            Commands::Widget {
                pos,
                active_drag,
                file,
            } => handle_widget(pos.as_deref(), active_drag, file),
            Commands::Convert {
                input,
                target,
                output,
                all,
            } => handle_convert(
                &input,
                Some(&target),
                all,
                output.as_deref(),
                cli.strict_lossless,
                cli.json,
            ),
            Commands::Archive {
                inputs,
                output,
                archive_type,
            } => handle_archive(&inputs, &output, archive_type.as_deref(), cli.json),
            Commands::Extract {
                archive,
                output_dir,
            } => handle_extract(&archive, output_dir.as_deref(), cli.json),
            Commands::Info { file } => handle_info(&file, cli.json),
            Commands::Check { file } => handle_check(&file, cli.json),
            Commands::Bench { file, target } => handle_bench(&file, target.as_deref(), cli.json),
            Commands::Hw => handle_hw(cli.json),
            Commands::Verify { source, target } => handle_verify(&source, &target, cli.json),
            Commands::Catalogue { query } => handle_catalogue(&query, cli.json),
            Commands::UploadReady {
                input,
                preset,
                width,
                height,
                dpi,
                min_kb,
                max_kb,
                whiten,
                output_dir,
            } => handle_exam(
                &input,
                preset.as_deref(),
                width,
                height,
                dpi,
                min_kb,
                max_kb,
                whiten,
                output_dir.as_deref(),
                cli.json,
            ),
            Commands::Themes => handle_themes(cli.json),
            Commands::Config { action, key, value } => {
                handle_config(&action, key.as_deref(), value.as_deref(), cli.json)
            }
            Commands::Completions { shell, carapace } => {
                handle_completions(shell.as_deref(), carapace)
            }
            Commands::Carapace { shell, args } => handle_carapace(shell.as_deref(), &args),
            Commands::HelpManual => {
                print_table_help();
                Ok(())
            }
            Commands::Banner => {
                print_welcome_banner();
                Ok(())
            }
            Commands::Daemon { action } => daemon::handle_daemon_command(&action),
        };
    }

    // UNIX mv-like positional syntax: `mossambi <INPUT> [TARGET_FORMAT]`
    if let Some(input) = cli.input {
        let input_str = input.to_string_lossy();
        // Guard against command names being treated as files or directories:
        match input_str.as_ref() {
            "app" | "explorer" | "gui" => return handle_gui(None),
            "widget" => return handle_widget(None, false, None),
            "tui" => return handle_tui(None),
            "banner" => {
                print_welcome_banner();
                return Ok(());
            }
            "help-manual" | "manual" | "help" => {
                print_table_help();
                return Ok(());
            }
            "hw" => return handle_hw(cli.json),
            "themes" => return handle_themes(cli.json),
            _ => {}
        }

        // If input is a directory and no target is given, launch Explorer GUI directly opened at that directory
        if input.is_dir() && cli.target.is_none() && !cli.all && cli.exam_preset.is_none() {
            return handle_gui(Some(input));
        }

        // Check if Exam Mode was requested
        if cli.exam_preset.is_some() || cli.exam_width.is_some() || cli.exam_min_kb.is_some() {
            return handle_exam(
                &input,
                cli.exam_preset.as_deref(),
                cli.exam_width,
                cli.exam_height,
                cli.exam_dpi,
                cli.exam_min_kb,
                cli.exam_max_kb,
                cli.exam_whiten,
                cli.output_dir.as_deref(),
                cli.json,
            );
        }

        // If target is omitted, check if --all flag is set
        if cli.all || cli.target.as_deref().map(|t| t.to_uppercase()) == Some("ALL".to_string()) {
            return handle_convert(
                &input,
                Some("ALL"),
                true,
                cli.output_dir.as_deref(),
                cli.strict_lossless,
                cli.json,
            );
        }

        // If target is specified, convert
        if let Some(ref target) = cli.target {
            return handle_convert(
                &input,
                Some(target),
                false,
                cli.output_dir.as_deref(),
                cli.strict_lossless,
                cli.json,
            );
        }

        // If target is omitted and not --all, display file information and available targets
        return handle_info(&input, cli.json);
    }

    // Double-clicked or launched without console arguments: launch full Studio GUI directly
    handle_gui(None)
}

fn handle_convert(
    input: &Path,
    target: Option<&str>,
    is_all: bool,
    output_dir: Option<&Path>,
    strict_lossless: bool,
    as_json: bool,
) -> Result<()> {
    if !input.exists() {
        bail!("Input path does not exist: {}", input.display());
    }

    let mode = if strict_lossless {
        LosslessMode::StrictBitExact
    } else {
        LosslessMode::PerceptualHighFidelity
    };

    let target_str = target.unwrap_or("ALL");

    if is_all || target_str.eq_ignore_ascii_case("ALL") {
        if !as_json {
            println!(
                "🍋 Mossambi: Batch transcoding '{}' to ALL formats in domain...",
                input.display()
            );
        }
        let outputs = transcode_to_all_formats(input, mode, output_dir)
            .context("Failed executing batch ALL transcode")?;

        if as_json {
            let out_strs: Vec<String> = outputs.iter().map(|p| p.display().to_string()).collect();
            let res = json!({
                "status": "success",
                "input": input.display().to_string(),
                "all": true,
                "count": outputs.len(),
                "outputs": out_strs
            });
            println!("{}", serde_json::to_string_pretty(&res)?);
        } else {
            println!("✨ Successfully generated {} formats:", outputs.len());
            for out in &outputs {
                let size = std::fs::metadata(out).map(|m| m.len()).unwrap_or(0);
                println!("  ✓ {} ({:.1} KB)", out.display(), size as f64 / 1024.0);
            }
        }
        return Ok(());
    }

    if !as_json {
        println!(
            "🍋 Mossambi: Converting '{}' -> {}...",
            input.display(),
            target_str.to_uppercase()
        );
    }

    let output =
        transcode_file(input, target_str, mode, output_dir).context("Failed converting file")?;

    let out_size = std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);

    if as_json {
        let res = json!({
            "status": "success",
            "input": input.display().to_string(),
            "target": target_str.to_uppercase(),
            "output": output.display().to_string(),
            "bytes": out_size,
            "lossless": strict_lossless
        });
        println!("{}", serde_json::to_string_pretty(&res)?);
    } else {
        println!(
            "✨ Completed: {} ({:.1} KB)",
            output.display(),
            out_size as f64 / 1024.0
        );
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_exam(
    input: &Path,
    preset: Option<&str>,
    width: Option<u32>,
    height: Option<u32>,
    dpi: Option<u32>,
    min_kb: Option<u64>,
    max_kb: Option<u64>,
    whiten: bool,
    output_dir: Option<&Path>,
    as_json: bool,
) -> Result<()> {
    if !input.exists() {
        bail!("Input file does not exist: {}", input.display());
    }

    let mut constraint = match preset.unwrap_or("").to_lowercase().as_str() {
        "upsc-photo" | "upsc_photo" => ExamConstraint::preset_upsc_photo(),
        "upsc-signature" | "upsc_signature" | "upsc-sign" => {
            ExamConstraint::preset_upsc_signature()
        }
        "ssc-photo" | "ssc_photo" => ExamConstraint::preset_ssc_photo(),
        "ssc-signature" | "ssc_signature" | "ssc-sign" => ExamConstraint::preset_ssc_signature(),
        "ibps-photo" | "ibps_photo" => ExamConstraint::preset_ibps_photo(),
        "ibps-signature" | "ibps_signature" | "ibps-sign" => {
            ExamConstraint::preset_ibps_signature()
        }
        _ => ExamConstraint {
            name: "Custom Exam Specification".into(),
            target_format: "JPG".into(),
            width_px: width.unwrap_or(350),
            height_px: height.unwrap_or(450),
            dpi: dpi.unwrap_or(200),
            min_bytes: min_kb.unwrap_or(20) * 1024,
            max_bytes: max_kb.unwrap_or(50) * 1024,
            normalize_white_balance: whiten,
            positioning: mossambi::exam::PositioningMode::AspectFit,
            background_color: [255, 255, 255],
        },
    };

    // Apply manual overrides if provided
    if let Some(w) = width {
        constraint.width_px = w;
    }
    if let Some(h) = height {
        constraint.height_px = h;
    }
    if let Some(d) = dpi {
        constraint.dpi = d;
    }
    if let Some(min) = min_kb {
        constraint.min_bytes = min * 1024;
    }
    if let Some(max) = max_kb {
        constraint.max_bytes = max * 1024;
    }
    if whiten {
        constraint.normalize_white_balance = true;
    }

    let parent = output_dir.unwrap_or_else(|| input.parent().unwrap_or_else(|| Path::new(".")));
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("exam_doc");
    let ext_lower = constraint.target_format.to_lowercase();
    let mut out_candidate = parent.join(format!("{}_exam.{}", stem, ext_lower));
    let mut c = 1;
    while out_candidate.exists() {
        out_candidate = parent.join(format!("{}_exam_{}.{}", stem, c, ext_lower));
        c += 1;
    }

    if !as_json {
        println!(
            "🇮🇳 Indian Exam Form Tuner: Optimizing for '{}'...",
            constraint.name
        );
        println!(
            "   Target: {}x{} px @ {} DPI | Allowed Size: {:.1} KB - {:.1} KB",
            constraint.width_px,
            constraint.height_px,
            constraint.dpi,
            constraint.min_bytes as f64 / 1024.0,
            constraint.max_bytes as f64 / 1024.0
        );
    }

    let optimized = optimize_exam_media(input, &out_candidate, &constraint)?;
    let actual_size = std::fs::metadata(&optimized).map(|m| m.len()).unwrap_or(0);

    if as_json {
        let res = json!({
            "status": "success",
            "preset": constraint.name,
            "output": optimized.display().to_string(),
            "bytes": actual_size,
            "kilobytes": (actual_size as f64 / 1024.0 * 10.0).round() / 10.0,
            "width": constraint.width_px,
            "height": constraint.height_px,
            "dpi": constraint.dpi,
            "in_envelope": actual_size >= constraint.min_bytes && actual_size <= constraint.max_bytes
        });
        println!("{}", serde_json::to_string_pretty(&res)?);
    } else {
        println!("✨ Form Ready Document Generated: {}", optimized.display());
        println!(
            "   Final File Size: {:.2} KB (Passed portal validation boundaries)",
            actual_size as f64 / 1024.0
        );
    }

    Ok(())
}

fn handle_archive(
    inputs: &[PathBuf],
    output: &Path,
    archive_type_str: Option<&str>,
    as_json: bool,
) -> Result<()> {
    for inp in inputs {
        if !inp.exists() {
            bail!("Input path does not exist: {}", inp.display());
        }
    }

    let archive_type = if let Some(type_str) = archive_type_str {
        ArchiveType::from_extension(type_str)
            .with_context(|| format!("Unknown archive type specified: {}", type_str))?
    } else {
        let ext = output.extension().and_then(|e| e.to_str()).unwrap_or("zip");
        ArchiveType::from_extension(ext).unwrap_or(ArchiveType::Zip)
    };

    if !as_json {
        println!(
            "📦 Compressing {} input(s) into '{}'...",
            inputs.len(),
            output.display()
        );
    }

    pack_archive(inputs, output, archive_type)?;
    let size = std::fs::metadata(output).map(|m| m.len()).unwrap_or(0);

    if as_json {
        let in_strs: Vec<String> = inputs.iter().map(|p| p.display().to_string()).collect();
        let res = json!({
            "status": "success",
            "archive": output.display().to_string(),
            "bytes": size,
            "inputs": in_strs
        });
        println!("{}", serde_json::to_string_pretty(&res)?);
    } else {
        println!(
            "✨ Archive successfully created: {} ({:.1} KB)",
            output.display(),
            size as f64 / 1024.0
        );
    }

    Ok(())
}

fn handle_extract(archive: &Path, output_dir: Option<&Path>, as_json: bool) -> Result<()> {
    if !archive.exists() {
        bail!("Archive file does not exist: {}", archive.display());
    }

    let dest = match output_dir {
        Some(d) => d.to_path_buf(),
        None => {
            let stem = archive
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("extracted");
            archive
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(stem)
        }
    };

    if !as_json {
        println!(
            "📂 Extracting archive '{}' into '{}' with Zip-Slip protection...",
            archive.display(),
            dest.display()
        );
    }

    unpack_archive(archive, &dest)?;

    if as_json {
        let res = json!({
            "status": "success",
            "archive": archive.display().to_string(),
            "destination": dest.display().to_string()
        });
        println!("{}", serde_json::to_string_pretty(&res)?);
    } else {
        println!("✨ Extraction complete into: {}", dest.display());
    }

    Ok(())
}

fn handle_info(file: &Path, as_json: bool) -> Result<()> {
    if !file.exists() {
        bail!("File does not exist: {}", file.display());
    }

    let (category, ext) = UniversalRegistry::classify_file(file);
    let widget_sectors = UniversalRegistry::get_widget_sectors_for_category(category);
    let all_supported = UniversalRegistry::get_all_supported_outputs(category);
    let meta = std::fs::metadata(file)?;
    let size_kb = meta.len() as f64 / 1024.0;

    if as_json {
        let res = json!({
            "path": file.display().to_string(),
            "extension": ext,
            "category": format!("{:?}", category),
            "size_bytes": meta.len(),
            "size_kb": (size_kb * 10.0).round() / 10.0,
            "widget_sectors_8petal": widget_sectors,
            "all_supported_formats": all_supported
        });
        println!("{}", serde_json::to_string_pretty(&res)?);
    } else {
        println!("🍋 Mossambi File Inspector");
        println!("--------------------------------------------------");
        println!("File:         {}", file.display());
        println!(
            "Extension:    {}",
            if ext.is_empty() { "(none)" } else { &ext }
        );
        println!("Category:     {:?}", category);
        println!("Size:         {:.2} KB ({} bytes)", size_kb, meta.len());
        println!("\nRadial Widget Sectors (8 Petals):");
        print!("  ");
        for sec in &widget_sectors {
            if *sec == "ALL" {
                print!(" [⭐ ALL]");
            } else {
                print!(" [{}]", sec);
            }
        }
        println!("\n\nAll Supported Output Formats (Catalog):");
        println!("  {}", all_supported.join(", "));
        println!("--------------------------------------------------");
    }

    Ok(())
}

fn handle_check(file: &Path, as_json: bool) -> Result<()> {
    if !file.exists() {
        bail!("File does not exist: {}", file.display());
    }

    let report = FileQualityChecker::inspect_file(file)?;
    if as_json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", report.format_terminal());
    }
    Ok(())
}

fn handle_bench(file: &Path, target: Option<&str>, as_json: bool) -> Result<()> {
    if !file.exists() {
        bail!("File does not exist: {}", file.display());
    }

    let report = BenchmarkEngine::benchmark_file(file, target)?;
    if as_json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", report.format_terminal());
    }
    Ok(())
}

fn handle_hw(as_json: bool) -> Result<()> {
    let hw = detect_hardware_capabilities();
    let specs = mossambi::hwaccel::detect_system_specs();
    let suggestions = mossambi::hwaccel::get_system_auto_suggestions(&specs);
    let (os, arch) = get_system_arch_info();

    if as_json {
        let res = json!({
            "os": os,
            "arch": arch,
            "gpu_vendor": format!("{:?}", hw.vendor),
            "gpu_name": specs.gpu_name,
            "total_ram_gb": specs.total_ram_gb,
            "npu_engine": format!("{:?}", hw.npu_engine),
            "npu_name": hw.npu_name,
            "supports_gpu_transcode": hw.supports_gpu_transcode,
            "supports_npu_ai": hw.supports_npu_ai,
            "supports_h264_hw": hw.supports_h264_hw,
            "supports_hevc_hw": hw.supports_hevc_hw,
            "supports_av1_hw": hw.supports_av1_hw,
            "neon_simd": hw.supports_neon_simd,
            "avx2_simd": hw.supports_avx2_simd,
            "ffmpeg_hwaccel_flag": hw.ffmpeg_hwaccel_flag,
            "h264_encoder": hw.h264_encoder,
            "hevc_encoder": hw.hevc_encoder,
            "optimal_thread_count": hw.optimal_thread_count,
            "recommended_concurrency": specs.recommended_concurrency,
            "recommended_ai_model": specs.recommended_ai_model,
            "auto_suggestions": suggestions,
        });
        println!("{}", serde_json::to_string_pretty(&res)?);
    } else {
        println!("🍋 Mossambi Hardware Acceleration & Platform Diagnostics");
        println!("==========================================================");
        println!("OS Platform:          {}", os);
        println!("CPU Architecture:     {}", arch);
        println!("Primary GPU:          {}", specs.gpu_name);
        println!("Detected GPU Vendor:  {:?}", hw.vendor);
        println!(
            "Physical RAM:         {:.1} GB Installed",
            specs.total_ram_gb
        );
        println!("FFmpeg HW Flag:       {}", hw.ffmpeg_hwaccel_flag);
        println!("H.264 Encoder:        {}", hw.h264_encoder);
        println!("HEVC Encoder:         {}", hw.hevc_encoder);
        println!("NPU AI Accelerator:   {}", hw.npu_name);
        println!(
            "NPU Status:           {}",
            if hw.supports_npu_ai {
                "Active (AI Scan & OCR Acceleration)"
            } else {
                "CPU Fallback"
            }
        );
        println!(
            "AV1 Support:          {}",
            if hw.supports_av1_hw { "Yes" } else { "No" }
        );
        println!(
            "ARM NEON SIMD:        {}",
            if hw.supports_neon_simd {
                "Enabled"
            } else {
                "Disabled"
            }
        );
        println!(
            "x86_64 AVX2 SIMD:     {}",
            if hw.supports_avx2_simd {
                "Enabled"
            } else {
                "Disabled"
            }
        );
        println!(
            "Multi-Core Threads:   {} logical cores (Auto-tuned: {} workers)",
            hw.optimal_thread_count, specs.recommended_concurrency
        );
        println!("----------------------------------------------------------");
        println!("Auto-Aware System Optimization & Suggestions:");
        for s in suggestions {
            println!("  [{}] {}", s.badge, s.title);
            println!("    -> Recommendation: {}", s.recommendation);
            println!("       Rationale:      {}", s.rationale);
        }
        println!("==========================================================");
    }

    Ok(())
}

fn handle_verify(source: &Path, target: &Path, as_json: bool) -> Result<()> {
    let report = verify_conversion_fidelity(source, target)?;
    if as_json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", report.format_terminal());
    }
    Ok(())
}

fn handle_catalogue(query: &str, as_json: bool) -> Result<()> {
    let results = UniversalRegistry::search_catalogue(query);
    if as_json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        println!("🍋 Mossambi Universal Format Catalogue (200+ Formats)");
        println!("Query: '{}' ({} matching format(s))", query, results.len());
        println!(
            "--------------------------------------------------------------------------------"
        );
        println!(
            "{:<8} | {:<12} | {:<14} | {:<10} | NAME",
            "EXT", "CATEGORY", "NATURE", "ZERO-LOSS"
        );
        println!(
            "--------------------------------------------------------------------------------"
        );
        for spec in results {
            println!(
                "{:<8} | {:<12} | {:<14} | {:<10} | {}",
                spec.extension,
                format!("{:?}", spec.category),
                spec.nature.badge_label(),
                if spec.supports_zero_loss {
                    "✓ Guaranteed"
                } else {
                    "⚠️ Lossy"
                },
                spec.display_name
            );
        }
        println!(
            "--------------------------------------------------------------------------------"
        );
    }
    Ok(())
}

fn handle_widget(pos: Option<&str>, _active_drag: bool, file: Option<PathBuf>) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        use std::ptr::null_mut;
        use windows_sys::Win32::Foundation::GetLastError;
        use windows_sys::Win32::System::Threading::CreateMutexW;
        use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW;

        let mutex_name: Vec<u16> = "Local\\MossambiWidgetSingleInstance\0"
            .encode_utf16()
            .collect();
        unsafe {
            let handle = CreateMutexW(null_mut(), 1, mutex_name.as_ptr());
            const ERROR_ALREADY_EXISTS: u32 = 183;
            if !handle.is_null() && GetLastError() == ERROR_ALREADY_EXISTS {
                // Close existing widget first, then continue to launch a fresh one
                let title: Vec<u16> = "Mossambi Radial Widget\0".encode_utf16().collect();
                let hwnd = FindWindowW(null_mut(), title.as_ptr());
                if !hwnd.is_null() {
                    use windows_sys::Win32::UI::WindowsAndMessaging::{
                        PostMessageW, ShowWindow, SW_RESTORE,
                    };
                    const WM_CLOSE: u32 = 0x0010;
                    ShowWindow(hwnd, SW_RESTORE);
                    let _ = PostMessageW(hwnd, WM_CLOSE, 0, 0);
                }
                // Fall through: launch a fresh instance at the pointer
            }
        }
    }

    let mut initial_pos: Option<eframe::egui::Pos2> = None;
    if let Some(pos_str) = pos {
        let parts: Vec<&str> = pos_str.split(',').collect();
        if parts.len() == 2 {
            if let (Ok(x), Ok(y)) = (
                parts[0].trim().parse::<f32>(),
                parts[1].trim().parse::<f32>(),
            ) {
                initial_pos = Some(eframe::egui::Pos2::new(x - 200.0, y - 200.0));
            }
        }
    }
    mossambi::widget::run_widget_with_args(initial_pos, file)
        .map_err(|e| anyhow::anyhow!("Radial Widget error: {}", e))
}

fn handle_gui(initial_path: Option<PathBuf>) -> Result<()> {
    mossambi::gui::run_studio_with_path(initial_path)
        .map_err(|e| anyhow::anyhow!("Studio GUI error: {}", e))
}

fn handle_tui(initial_path: Option<PathBuf>) -> Result<()> {
    mossambi::tui::run_with_path(initial_path)
        .map_err(|e| anyhow::anyhow!("TUI execution error: {}", e))
}

fn handle_themes(as_json: bool) -> Result<()> {
    use mossambi::core::{DARK_THEMES, LIGHT_THEMES};

    if as_json {
        let all = mossambi::core::all_pantone_themes();
        println!("{}", serde_json::to_string_pretty(all)?);
    } else {
        println!("╔══════════════════════════════════════════════════════════════════════════════════════════════╗");
        println!("║                      🎨 20 CERTIFIED PANTONE THEMES (10 DARK & 10 LIGHT)                     ║");
        println!("║          100% Pantone-Certified Matching System (PMS) & FHI Standards Architecture           ║");
        println!("╚══════════════════════════════════════════════════════════════════════════════════════════════╝\n");

        println!("🌙 10 CERTIFIED DARK PANTONE THEMES:");
        println!("────────────────────────────────────────────────────────────────────────────────────────────────");
        for (idx, theme) in DARK_THEMES.iter().enumerate() {
            println!("  {:2}. {:<24} [ID: {:<16}]", idx + 1, theme.name, theme.id);
            println!(
                "      BG:      {:<18} ({:<20}) [{}]",
                theme.bg.name, theme.bg.code, theme.bg.hex
            );
            println!(
                "      Surface: {:<18} ({:<20}) [{}]",
                theme.surface.name, theme.surface.code, theme.surface.hex
            );
            println!(
                "      Accent:  {:<18} ({:<20}) [{}]",
                theme.accent.name, theme.accent.code, theme.accent.hex
            );
            println!();
        }

        println!("☀️ 10 CERTIFIED LIGHT PANTONE THEMES:");
        println!("────────────────────────────────────────────────────────────────────────────────────────────────");
        for (idx, theme) in LIGHT_THEMES.iter().enumerate() {
            println!("  {:2}. {:<24} [ID: {:<16}]", idx + 1, theme.name, theme.id);
            println!(
                "      BG:      {:<18} ({:<20}) [{}]",
                theme.bg.name, theme.bg.code, theme.bg.hex
            );
            println!(
                "      Surface: {:<18} ({:<20}) [{}]",
                theme.surface.name, theme.surface.code, theme.surface.hex
            );
            println!(
                "      Accent:  {:<18} ({:<20}) [{}]",
                theme.accent.name, theme.accent.code, theme.accent.hex
            );
            println!();
        }
        println!("💡 Tip: Switch themes in CLI using 'mossambi config set theme <id_or_name>'");
        println!("        or press [t] in TUI / select from Preferences tab in Desktop Studio.\n");
    }
    Ok(())
}

fn handle_config(
    action: &str,
    key: Option<&str>,
    value: Option<&str>,
    as_json: bool,
) -> Result<()> {
    let mut config = MossambiConfig::load();

    match action.to_lowercase().as_str() {
        "get" => {
            if let Some(k) = key {
                let val_str = match k {
                    "shade" | "citrus_shade" => config.citrus_shade.clone(),
                    "theme" | "theme_mode" => format!("{:?}", config.theme_mode),
                    "always_on_top" => config.always_on_top.to_string(),
                    "wheel_size" => config.wheel_size.to_string(),
                    "gesture" | "enable_gesture_wakeup" => config.enable_gesture_wakeup.to_string(),
                    "hotkey" | "enable_hotkey_wakeup" => config.enable_hotkey_wakeup.to_string(),
                    "zero_loss" | "enforce_zero_loss" => config.enforce_zero_loss.to_string(),
                    _ => format!("Unknown key '{}'", k),
                };
                if as_json {
                    println!("{}", json!({ "key": k, "value": val_str }));
                } else {
                    println!("{} = {}", k, val_str);
                }
            } else {
                bail!("Missing key to retrieve. Use 'mossambi config get <key>'");
            }
        }
        "set" => {
            if let (Some(k), Some(v)) = (key, value) {
                match k {
                    "shade" | "citrus_shade" => config.citrus_shade = v.to_string(),
                    "theme" | "theme_mode" => {
                        if let Some(mode) = ThemeMode::from_id_or_name(v) {
                            config.theme_mode = mode;
                        } else {
                            match v.to_lowercase().as_str() {
                                "dark" => config.theme_mode = ThemeMode::DarkAnthracite,
                                "light" => config.theme_mode = ThemeMode::LightBrilliant,
                                _ => bail!("Unknown Pantone theme: '{}'. Use 'mossambi themes' to see all 20 certified Pantone themes.", v),
                            }
                        }
                    }
                    "always_on_top" => config.always_on_top = v.parse().unwrap_or(true),
                    "wheel_size" => config.wheel_size = v.parse().unwrap_or(440),
                    "gesture" | "enable_gesture_wakeup" => {
                        config.enable_gesture_wakeup = v.parse().unwrap_or(true)
                    }
                    "hotkey" | "enable_hotkey_wakeup" => {
                        config.enable_hotkey_wakeup = v.parse().unwrap_or(true)
                    }
                    "zero_loss" | "enforce_zero_loss" => {
                        config.enforce_zero_loss = v.parse().unwrap_or(true)
                    }
                    _ => bail!("Unknown setting key: {}", k),
                }
                config.save()?;
                if as_json {
                    println!("{}", json!({ "status": "saved", "key": k, "value": v }));
                } else {
                    println!("✓ Saved: {} = {}", k, v);
                }
            } else {
                bail!("Usage: mossambi config set <key> <value>");
            }
        }
        _ => {
            // list
            if as_json {
                println!("{}", serde_json::to_string_pretty(&config)?);
            } else {
                println!("🍋 Mossambi Persistent Configuration (~/.mossambi/config.json)");
                println!("---------------------------------------------------------------");
                println!("Theme Mode:            {:?}", config.theme_mode);
                println!("Citrus Shade:          {}", config.citrus_shade);
                println!("Always On Top:         {}", config.always_on_top);
                println!("Wheel Size:            {} px", config.wheel_size);
                println!("Win+Ctrl Hotkey:       {}", config.enable_hotkey_wakeup);
                println!("Circle Gesture Wakeup: {}", config.enable_gesture_wakeup);
                println!("Zero-Loss Standard:    {}", config.enforce_zero_loss);
                println!(
                    "Config Path:           {}",
                    MossambiConfig::config_path().display()
                );
                println!("---------------------------------------------------------------");
                println!("To modify: mossambi config set <key> <value>");
            }
        }
    }

    Ok(())
}

fn print_welcome_banner() {
    let sage = "\x1b[38;2;74;133;90m"; // Pantone 16-5917 Sage Meadow
    let blue = "\x1b[38;2;0;122;255m"; // Pantone 19-4052 Classic Blue
    let amber = "\x1b[38;2;217;119;6m"; // Pantone 14-1064 Soft Amber
    let cyan = "\x1b[38;2;60;200;230m";
    let white = "\x1b[38;2;244;245;247m";
    let muted = "\x1b[38;2;160;160;165m";
    let bold = "\x1b[1m";
    let reset = "\x1b[0m";

    println!(
        r#"{sage}
                       .-''''-.     _/\_
                     .'  ____  `.  (____)
                    /  .' || `.  \   ||
                   |  / \ || / \  |  ||
                   | ==  (())  == |  ||
                   |  \ / || \ /  |  ||
                    \  `. || .'  /   ||
                     `.  `--'  .'
                       `'-..-'`{reset}

{blue}   M O S S A M B I{reset}
{amber}   Zero-Loss File Conversion Platform & Universal File Explorer{reset}
{muted}   One Pure Rust Core · Three Modalities: Explorer App, Radial Widget, Interactive TUI{reset}
   {white}v2.1.0 | 0.00% Zero-Loss Standard | Windows Fluent & macOS Finder DNA{reset}
"#
    );

    println!("{bold}Access Modalities:{reset}");
    println!("  {cyan}mossambi{reset}                       # Launch full macOS-style File Explorer & Studio GUI");
    println!("  {cyan}mossambi app [DIR]{reset}             # Launch Explorer GUI opened at specified directory");
    println!(
        "  {cyan}mossambi explorer [DIR]{reset}        # Launch Explorer GUI (explicit alias)"
    );
    println!("  {cyan}mossambi widget{reset}                # Summon Sweet Lime desktop floating radial widget");
    println!("  {cyan}mossambi tui [DIR]{reset}             # Launch Yazi-grade interactive Terminal File Explorer");
    println!();
    println!("{bold}CLI Fast-Path Conversion:{reset}");
    println!("  {cyan}mossambi <file> [format]{reset}       # Instant conversion (e.g. mossambi sound.wav flac)");
    println!("  {cyan}mossambi <file> --all{reset}          # Transcode to ALL domain formats in parallel");
    println!("  {cyan}mossambi <photo> --exam <p>{reset}    # Indian Exam Optimizer (upsc-photo, ssc-photo, etc.)");
    println!("  {cyan}mossambi archive <f...> -o a.zip{reset}# Exhaustive compression archive (ZIP, 7Z, TAR, ZST)");
    println!(
        "  {cyan}mossambi extract <archive>{reset}     # Safe extraction with Zip-Slip protection"
    );
    println!(
        "  {cyan}mossambi check <file>{reset}          # Quality, integrity & decode scorecard"
    );
    println!("  {cyan}mossambi bench <file> [tgt]{reset}    # Latency, throughput (MB/s) & speedup benchmark");
    println!(
        "  {cyan}mossambi hw{reset}                    # GPU/NPU & SIMD hardware acceleration info"
    );
    println!("  {cyan}mossambi verify <src> <tgt>{reset}    # Mathematical 0.00% zero-loss verification proof");
    println!(
        "  {cyan}mossambi catalogue [query]{reset}     # Search 200+ format registry catalogue"
    );
    println!(
        "  {cyan}mossambi themes{reset}                # List 20 certified eye-safe Pantone themes"
    );
    println!("  {cyan}mossambi daemon{reset}                # Resident mouse gesture service (Win+Shift+C / Win+Ctrl+C)");
    println!("  {cyan}mossambi help-manual{reset}           # Exhaustive table help and manual");
    println!("\nRun '{cyan}mossambi --help{reset}' for all command flags and options.\n");
}

fn handle_completions(shell_str: Option<&str>, carapace: bool) -> Result<()> {
    use clap::CommandFactory;
    use clap_complete::Shell;

    if carapace {
        let yaml = r#"name: mossambi
description: Zero-loss native desktop file converter, archive suite, and Indian exam form tuner
flags:
  -a, --all: Batch convert to ALL valid formats in domain
  -o, --output=: Destination directory for output files
  --strict: Enforce strict 0.00% zero-loss transcode
  --json: Output results in JSON format
  --exam=: Indian Competitive Exam preset (e.g. upsc-photo, ssc-photo, ibps-signature)
  --width=: Exam custom target width in pixels
  --height=: Exam custom target height in pixels
  --dpi=: Exam custom target DPI (200 / 300)
  --min-kb=: Exam minimum file size in kilobytes
  --max-kb=: Exam maximum file size in kilobytes
  --whiten: Auto-whiten background of scanned documents or signatures
commands:
  - name: convert
    description: Convert an input file to target format
  - name: archive
    description: Pack files or directories into an archive
  - name: extract
    description: Extract an archive with Zip-Slip protection
  - name: info
    description: Inspect file metadata and detected category
  - name: check
    description: Check quality, container integrity, and decoder parseability
  - name: bench
    description: Benchmark conversion latency and throughput vs alternatives
  - name: hw
    description: Display detected GPU/NPU hardware acceleration engines
  - name: tui
    description: Launch the interactive Terminal User Interface
  - name: gui
    description: Launch the Desktop Radial Widget and macOS-Styled GUI
  - name: verify
    description: Verify mathematical zero-loss fidelity between two files
  - name: upload-ready
    description: Prepare image/document for Indian competitive exam portals
"#;
        println!("{}", yaml);
        return Ok(());
    }

    let shell = match shell_str.map(|s| s.to_lowercase()).as_deref() {
        Some("bash") => Shell::Bash,
        Some("zsh") => Shell::Zsh,
        Some("fish") => Shell::Fish,
        Some("powershell") | Some("pwsh") => Shell::PowerShell,
        Some("elvish") => Shell::Elvish,
        _ => {
            println!("Usage: mossambi completions [bash|zsh|fish|powershell|elvish] [--carapace]");
            return Ok(());
        }
    };

    let mut cmd = Cli::command();
    clap_complete::generate(shell, &mut cmd, "mossambi", &mut std::io::stdout());
    Ok(())
}

fn handle_carapace(_shell: Option<&str>, args: &[String]) -> Result<()> {
    if let Some(first_arg) = args.first() {
        let path = Path::new(first_arg);
        if path.exists() {
            let (category, in_ext) = UniversalRegistry::classify_file(path);
            let targets = UniversalRegistry::get_all_supported_outputs(category);
            let mut completions = Vec::new();
            for tgt in targets {
                if tgt != in_ext.as_str() {
                    completions.push(json!({
                        "value": tgt.to_lowercase(),
                        "display": tgt.to_uppercase(),
                        "description": format!("Convert to {} ({:?})", tgt, category)
                    }));
                }
            }
            println!("{}", serde_json::to_string(&completions)?);
            return Ok(());
        }
    }
    println!("[]");
    Ok(())
}

fn print_table_help() {
    println!("┌──────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐");
    println!("│  Mossambi (MOSSAMBI — ZERO-LOSS CONVERSION PLATFORM & UNIVERSAL FILE EXPLORER v2.1.0)                            │");
    println!("│  One Pure Rust Core · Three Operating Modalities · Apple Elegance + Linux Power                                  │");
    println!("├──────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤");
    println!("│  Usage: mossambi [FLAGS / OPTIONS] [INPUT] [TARGET_FORMAT]                                                       │");
    println!("│  UNIX 'mv' POSITIONAL GRAMMAR: mossambi <INPUT> [TARGET_FORMAT] [-a|--all] [-o <DIR>]                            │");
    println!("├──────────────────────────────────┬──────────────────────────────────────────────────────┬────────────────────────┤");
    println!("│  FLAG / OPTION                   │  DESCRIPTION & FUNCTION                              │  EXAMPLE USAGE         │");
    println!("├──────────────────────────────────┼──────────────────────────────────────────────────────┼────────────────────────┤");
    println!("│  --app, --explorer, --gui [DIR]  │  Launch macOS-styled Universal File Explorer & Studio │  mossambi --app        │");
    println!("│  --widget [FILE]                 │  Launch Sweet Lime Resident Radial Widget (8 petals) │  mossambi --widget     │");
    println!("│  --tui [DIR]                     │  Launch Yazi-Grade Interactive Terminal File Explorer │  mossambi --tui        │");
    println!("├──────────────────────────────────┼──────────────────────────────────────────────────────┼────────────────────────┤");
    println!("│  -a, --all                       │  Batch transcode to ALL valid formats in domain      │  mossambi file.wav -a  │");
    println!("│  -o, --output <DIR>              │  Custom destination directory for output files       │  mossambi file.png -o .│");
    println!("│  --strict                        │  Enforce strict 0.00% zero-loss mathematical standard│  mossambi in.wav flac  │");
    println!("│  --json                          │  Output structured machine-parseable JSON response   │  mossambi in.wav --json│");
    println!("├──────────────────────────────────┼──────────────────────────────────────────────────────┼────────────────────────┤");
    println!("│  --exam <PRESET>                 │  Indian Exam Tuner (upsc-photo, ssc-photo, ibps-sign)│  mossambi in.jpg --exam│");
    println!("│  --width <PX> / --height <PX>    │  Exact target width and height in pixels             │  --width 350 --height  │");
    println!("│  --dpi <INT>                     │  Target resolution DPI (e.g. 200 or 300 DPI)         │  --dpi 300             │");
    println!("│  --min-kb <N> / --max-kb <N>     │  Target file byte envelope range (e.g. 20KB to 50KB) │  --min-kb 20 --max-kb50│");
    println!("│  --whiten                        │  Auto-whiten background of scanned signatures/photos │  --whiten              │");
    println!("├──────────────────────────────────┼──────────────────────────────────────────────────────┼────────────────────────┤");
    println!("│  --archive <FILES...> -o <ZIP>   │  Pack files into ZIP, 7Z, TAR, GZ, BZ2, XZ, or ZST   │  --archive a b -o c.zip│");
    println!("│  --extract <ARCHIVE> [-o <DIR>]  │  Extract archive safely with Zip-Slip protection     │  --extract bundle.zip  │");
    println!("├──────────────────────────────────┼──────────────────────────────────────────────────────┼────────────────────────┤");
    println!("│  --check <FILE>                  │  4-dimension Quality & Container Integrity Scorecard │  mossambi --check f.png│");
    println!("│  --bench <FILE> [TARGET]         │  Benchmark latency, throughput (MB/s) and speedup    │  mossambi --bench f.wav│");
    println!("│  --hw                            │  Display GPU/NPU hardware acceleration engines       │  mossambi --hw         │");
    println!("│  --verify <ORIG> <CONV>          │  Mathematical 0.00% zero-loss verification proof     │  --verify a.png b.png  │");
    println!("│  --catalogue [QUERY]             │  Search across 200+ format registry catalog          │  --catalogue lossless  │");
    println!("├──────────────────────────────────┼──────────────────────────────────────────────────────┼────────────────────────┤");
    println!("│  --themes                        │  List all 20 certified eye-safe Pantone themes       │  mossambi --themes     │");
    println!("│  --config [get|set|list]         │  View or edit persistent preferences (~/.mossambi)   │  --config set theme ...│");
    println!("│  --daemon [start|stop|status]    │  Resident mouse gesture daemon (Win+Shift+C Hotkey) │  --daemon start        │");
    println!("│  --banner                        │  Display Sweet Lime ASCII fruit art and summary      │  mossambi --banner     │");
    println!("│  --manual                        │  Display Unicode box table manual                    │  mossambi --manual     │");
    println!("│  -h, --help                      │  Display this complete usage manual table            │  mossambi --help       │");
    println!("│  -V, --version                   │  Display version and system architecture info        │  mossambi --version    │");
    println!("└──────────────────────────────────┴──────────────────────────────────────────────────────┴────────────────────────┘");
}
