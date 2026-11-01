//! Mossambi UNIX-Style Command-Line Interface (`mossambi`)
//! Frictionless syntax modeled after `mv`: `mossambi <input> [target_format]`
//! Supports `--all` batch transcode, Indian exam preset tuning, archive pack/unpack, hardware accel metrics, and persistent configuration.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use crossterm::tty::IsTty;
use mossambi_archive::{pack_archive, unpack_archive, ArchiveType};
use mossambi_codec::{
    transcode_file, transcode_to_all_formats, verify_conversion_fidelity, BenchmarkEngine,
    FileQualityChecker, LosslessMode,
};
use mossambi_core::{
    get_system_arch_info, MossambiConfig, ThemeMode, UniversalRegistry,
};
use mossambi_exam::{optimize_exam_media, ExamConstraint};
use mossambi_hwaccel::detect_hardware_capabilities;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;

mod daemon;

#[derive(Parser, Debug)]
#[command(
    name = "mossambi",
    author = "Mossambi Systems Team",
    version = "2.1.0",
    about = "Zero-loss native desktop file converter, archive suite, and Indian exam form tuner",
    long_about = "Mossambi: Native high-performance file conversion modeled after `mv`.\nWindows & Linux | x86_64 & ARM64 | 0.00% Loss Standard | Hardware Accelerated"
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

    /// Explicit subcommands
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
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

    /// Launch the interactive full-screen Terminal User Interface (Ratatui)
    Tui,

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

    /// Launch or summon the Sweet Lime Resident Radial Widget overlay
    Widget,

    /// Launch or summon the Desktop Radial Widget and macOS-Styled GUI
    Gui,

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
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--table-help" || a == "help-manual") {
        print_table_help();
        return Ok(());
    }

    if args.iter().any(|a| a == "--banner" || a == "banner") {
        print_welcome_banner();
        return Ok(());
    }

    let cli = Cli::parse();

    // Check if an explicit subcommand was requested
    if let Some(cmd) = cli.command {
        return match cmd {
            Commands::Convert {
                input,
                target,
                output,
                all,
            } => handle_convert(&input, Some(&target), all, output.as_deref(), cli.strict_lossless, cli.json),
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
            Commands::Tui => handle_tui(),
            Commands::Gui => handle_gui(),
            Commands::Widget => handle_widget(),
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
            Commands::Config { action, key, value } => handle_config(&action, key.as_deref(), value.as_deref(), cli.json),
            Commands::Completions { shell, carapace } => handle_completions(shell.as_deref(), carapace),
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
            return handle_convert(&input, Some("ALL"), true, cli.output_dir.as_deref(), cli.strict_lossless, cli.json);
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

    // No arguments provided:
    // If run interactively in a terminal, launch the full-screen Apple TUI directly!
    if std::io::stdout().is_tty() {
        return handle_tui();
    }

    print_welcome_banner();
    Ok(())
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

    let output = transcode_file(input, target_str, mode, output_dir)
        .context("Failed converting file")?;

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
        println!("✨ Completed: {} ({:.1} KB)", output.display(), out_size as f64 / 1024.0);
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
        "upsc-signature" | "upsc_signature" | "upsc-sign" => ExamConstraint::preset_upsc_signature(),
        "ssc-photo" | "ssc_photo" => ExamConstraint::preset_ssc_photo(),
        "ssc-signature" | "ssc_signature" | "ssc-sign" => ExamConstraint::preset_ssc_signature(),
        "ibps-photo" | "ibps_photo" => ExamConstraint::preset_ibps_photo(),
        "ibps-signature" | "ibps_signature" | "ibps-sign" => ExamConstraint::preset_ibps_signature(),
        _ => ExamConstraint {
            name: "Custom Exam Specification".into(),
            target_format: "JPG".into(),
            width_px: width.unwrap_or(350),
            height_px: height.unwrap_or(450),
            dpi: dpi.unwrap_or(200),
            min_bytes: min_kb.unwrap_or(20) * 1024,
            max_bytes: max_kb.unwrap_or(50) * 1024,
            normalize_white_balance: whiten,
            positioning: mossambi_exam::PositioningMode::AspectFit,
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
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("exam_doc");
    let ext_lower = constraint.target_format.to_lowercase();
    let mut out_candidate = parent.join(format!("{}_exam.{}", stem, ext_lower));
    let mut c = 1;
    while out_candidate.exists() {
        out_candidate = parent.join(format!("{}_exam_{}.{}", stem, c, ext_lower));
        c += 1;
    }

    if !as_json {
        println!("🇮🇳 Indian Exam Form Tuner: Optimizing for '{}'...", constraint.name);
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
        println!("   Final File Size: {:.2} KB (Passed portal validation boundaries)", actual_size as f64 / 1024.0);
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
        let ext = output
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("zip");
        ArchiveType::from_extension(ext)
            .unwrap_or(ArchiveType::Zip)
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
        println!("✨ Archive successfully created: {} ({:.1} KB)", output.display(), size as f64 / 1024.0);
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
            let stem = archive.file_stem().and_then(|s| s.to_str()).unwrap_or("extracted");
            archive.parent().unwrap_or_else(|| Path::new(".")).join(stem)
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
        println!("Extension:    {}", if ext.is_empty() { "(none)" } else { &ext });
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
    let (os, arch) = get_system_arch_info();

    if as_json {
        let res = json!({
            "os": os,
            "arch": arch,
            "gpu_vendor": format!("{:?}", hw.vendor),
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
        });
        println!("{}", serde_json::to_string_pretty(&res)?);
    } else {
        println!("🍋 Mossambi Hardware Acceleration & Platform Diagnostics");
        println!("==========================================================");
        println!("OS Platform:          {}", os);
        println!("CPU Architecture:     {}", arch);
        println!("Detected GPU Vendor:  {:?}", hw.vendor);
        println!("FFmpeg HW Flag:       {}", hw.ffmpeg_hwaccel_flag);
        println!("H.264 Encoder:        {}", hw.h264_encoder);
        println!("HEVC Encoder:         {}", hw.hevc_encoder);
        println!("NPU AI Accelerator:   {}", hw.npu_name);
        println!("NPU Status:           {}", if hw.supports_npu_ai { "Active (AI Scan & OCR Acceleration)" } else { "CPU Fallback" });
        println!("AV1 Support:          {}", if hw.supports_av1_hw { "Yes" } else { "No" });
        println!("ARM NEON SIMD:        {}", if hw.supports_neon_simd { "Enabled" } else { "Disabled" });
        println!("x86_64 AVX2 SIMD:     {}", if hw.supports_avx2_simd { "Enabled" } else { "Disabled" });
        println!("Multi-Core Threads:   {} concurrent worker threads", hw.optimal_thread_count);
        println!("==========================================================");
    }

    Ok(())
}

fn handle_tui() -> Result<()> {
    // Launch directly via internal pure Rust runner
    mossambi_tui::run()
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
        println!("--------------------------------------------------------------------------------");
        println!("{:<8} | {:<12} | {:<14} | {:<10} | NAME", "EXT", "CATEGORY", "NATURE", "ZERO-LOSS");
        println!("--------------------------------------------------------------------------------");
        for spec in results {
            println!(
                "{:<8} | {:<12} | {:<14} | {:<10} | {}",
                spec.extension,
                format!("{:?}", spec.category),
                spec.nature.badge_label(),
                if spec.supports_zero_loss { "✓ Guaranteed" } else { "⚠️ Lossy" },
                spec.display_name
            );
        }
        println!("--------------------------------------------------------------------------------");
    }
    Ok(())
}

fn handle_widget() -> Result<()> {
    println!("🍊 Summoning Mossambi Sweet Lime Resident Radial Widget overlay...");
    let current_exe = std::env::current_exe()?;
    let current_dir = current_exe.parent().unwrap_or_else(|| Path::new("."));
    let candidates = vec![
        current_dir.join("mossambi-widget.exe"),
        current_dir.join("mossambi-widget"),
        PathBuf::from("mossambi-widget.exe"),
        PathBuf::from("target/release/mossambi-widget.exe"),
        PathBuf::from("target/debug/mossambi-widget.exe"),
    ];

    let found = candidates.into_iter().find(|p| p.exists());

    if let Some(widget_path) = found {
        Command::new(&widget_path).spawn()?;
        println!("✓ Native Mossambi Radial Fruit Widget launched in background ({})", widget_path.display());
    } else {
        println!("Launching Mossambi Radial Widget via cargo run -p mossambi-widget...");
        Command::new("cargo")
            .args(["run", "-p", "mossambi-widget"])
            .spawn()?;
        println!("✓ Mossambi Radial Fruit Widget spawned.");
    }
    Ok(())
}

fn handle_gui() -> Result<()> {
    println!("🍋 Launching Mossambi Universal File Conversion Studio...");
    // Launch native pure Rust mossambi-gui binary
    let current_exe = std::env::current_exe()?;
    let current_dir = current_exe.parent().unwrap_or_else(|| Path::new("."));
    let candidates = vec![
        current_dir.join("mossambi-gui.exe"),
        current_dir.join("mossambi-gui"),
        PathBuf::from("mossambi-gui.exe"),
        PathBuf::from("target/release/mossambi-gui.exe"),
        PathBuf::from("target/debug/mossambi-gui.exe"),
    ];

    let found = candidates.into_iter().find(|p| p.exists());

    if let Some(gui_path) = found {
        Command::new(&gui_path).spawn()?;
        println!("✓ Native Mossambi Desktop Studio launched in background ({})", gui_path.display());
    } else {
        println!("Launching Mossambi Desktop Studio via cargo run -p mossambi-gui...");
        Command::new("cargo")
            .args(["run", "-p", "mossambi-gui"])
            .spawn()?;
        println!("✓ Mossambi Desktop Studio spawned.");
    }
    Ok(())
}

fn handle_config(action: &str, key: Option<&str>, value: Option<&str>, as_json: bool) -> Result<()> {
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
                    "theme" | "theme_mode" => match v.to_lowercase().as_str() {
                        "frosted" | "frosted_glass" | "sonoma" => config.theme_mode = ThemeMode::FrostedGlass,
                        "dark" | "dark_anthracite" | "pro" => config.theme_mode = ThemeMode::DarkAnthracite,
                        "light" | "light_aqua" | "aqua" => config.theme_mode = ThemeMode::LightAqua,
                        _ => config.theme_mode = ThemeMode::LiquidGlass,
                    },
                    "always_on_top" => config.always_on_top = v.parse().unwrap_or(true),
                    "wheel_size" => config.wheel_size = v.parse().unwrap_or(440),
                    "gesture" | "enable_gesture_wakeup" => config.enable_gesture_wakeup = v.parse().unwrap_or(true),
                    "hotkey" | "enable_hotkey_wakeup" => config.enable_hotkey_wakeup = v.parse().unwrap_or(true),
                    "zero_loss" | "enforce_zero_loss" => config.enforce_zero_loss = v.parse().unwrap_or(true),
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
                println!("Config Path:           {}", MossambiConfig::config_path().display());
                println!("---------------------------------------------------------------");
                println!("To modify: mossambi config set <key> <value>");
            }
        }
    }

    Ok(())
}

fn print_welcome_banner() {
    let green = "\x1b[38;2;120;190;32m";  // Pantone 376 C
    let zest  = "\x1b[38;2;228;240;36m";  // Pantone 388 C
    let gold  = "\x1b[38;2;255;184;28m";  // Pantone 1235 C
    let cyan  = "\x1b[38;2;60;200;230m";
    let white = "\x1b[38;2;245;245;247m";
    let muted = "\x1b[38;2;160;160;165m";
    let bold  = "\x1b[1m";
    let reset = "\x1b[0m";

    println!(
        r#"{green}
                       .-''''-.     _/\_
                     .'  ____  `.  (____)
                    /  .' || `.  \   ||
                   |  / \ || / \  |  ||
                   | ==  (())  == |  ||
                   |  \ / || \ /  |  ||
                    \  `. || .'  /   ||
                     `.  `--'  .'
                       `'-..-'`{reset}

{zest}   ╭╮╭╮╭╮ ╭───╮ ╭────╮ ╭────╮ ╭───╮ ╭╮╭╮╭╮ ╭────╮ ╭─╮
   ││││││ │╭─╮│ │╭───╯ │╭───╯ │╭─╮│ ││││││ │╭─╮│ │ │
   ││││││ ││ ││ ╰───╮  ╰───╮  ││ ││ ││││││ │╰─╯│ │ │
   ││││││ │╰─╯│ ╭───╯  ╭───╯  │╰─╯│ ││││││ │╭─╮│ │ │
   ╰╯╰╯╰╯ ╰───╯ ╰────╯ ╰────╯ ╰─╰─╯ ╰╯╰╯╰╯ ╰───╯ ╰─╯{reset}

{gold}   Universal File Conversion Platform · Indian Exam Form Studio{reset}
{muted}   One Rust Core · Three Interfaces: CLI, TUI, and Floating Radial Widget{reset}
   {white}v2.1.0 | 0.00% Zero-Loss Standard | Hardware Accelerated{reset}
"#
    );

    println!("{bold}Usage (UNIX mv-style):{reset}");
    println!("  {cyan}mossambi <file> [format]{reset}            # Convert file (e.g. mossambi song.flac mp3)");
    println!("  {cyan}mossambi <file> --all{reset}               # Batch transcode to ALL domain formats");
    println!("  {cyan}mossambi <photo> --exam upsc-photo{reset}  # Indian Exam Optimizer (UPSC/SSC/IBPS)");
    println!("  {cyan}mossambi archive <files...> -o out.7z{reset}  # Pack exhaustive compression archive");
    println!("  {cyan}mossambi extract <archive>{reset}          # Unpack archive safely with Zip-Slip protection");
    println!("  {cyan}mossambi info <file>{reset}                # Inspect format, sectors & catalog");
    println!("  {cyan}mossambi check <file>{reset}               # Universal Quality, Integrity & Compatibility Inspector");
    println!("  {cyan}mossambi bench <file> [tgt]{reset}         # Benchmark speed & compare vs Online/CLI tools");
    println!("  {cyan}mossambi tui{reset}                        # Launch Ratatui terminal interface");
    println!("  {cyan}mossambi gui{reset}                        # Launch radial widget & Mac-styled GUI");
    println!("  {cyan}mossambi hw{reset}                         # Check GPU & SIMD hardware engines");
    println!("  {cyan}mossambi config{reset}                     # View or edit system settings");
    println!("  {cyan}mossambi help-manual{reset}                # Display full Unicode box table manual");
    println!("  {cyan}mossambi banner{reset}                     # Print Sweet Lime fruit ASCII art & banner");
    println!("\nRun '{cyan}mossambi --help{reset}' or '{cyan}mossambi help-manual{reset}' for exhaustive options.\n");
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
    println!("┌──────────────────────────────────────────────────────────────────────────────────────────────────┐");
    println!("│  MOSSAMBI — ZERO-LOSS CONVERSION PLATFORM & DESKTOP SUITE (v2.1.0)                               │");
    println!("│  One Pure Rust Core · Three Operating Modalities · Apple Elegance + Linux Power                  │");
    println!("├──────────────────────────────────────────────────────────────────────────────────────────────────┤");
    println!("│  UNIX 'mv' POSITIONAL GRAMMAR:                                                                   │");
    println!("│    mossambi <INPUT> [TARGET_FORMAT] [OPTIONS]                                                    │");
    println!("│                                                                                                  │");
    println!("│  EXAMPLES:                                                                                       │");
    println!("│    mossambi input.wav flac                      Convert WAV to FLAC with 24-bit 0.00% Zero-Loss │");
    println!("│    mossambi photo.png webp                      Lossless WebP raster transcoding                 │");
    println!("│    mossambi movie.mkv mp4                       Fast-path NVMe stream remux (2hr movie in ~10s)  │");
    println!("│    mossambi notes.txt pdf                       Vector typography PDF with embedded CMaps        │");
    println!("│    mossambi sound.flac ALL                      Batch transcode to ALL audio formats in parallel │");
    println!("│    mossambi check test_vault/image/photo.png    4-dimension Quality & Integrity Scorecard        │");
    println!("│    mossambi bench test_vault/audio/sound.wav    Throughput (MB/s) & Speedup vs Online/CLI tools  │");
    println!("│    mossambi --exam upsc-photo sign.png          Indian competitive exam 100% scorecard optimizer │");
    println!("├──────────────────────────┬──────────────────────────────────────────┬────────────────────────────┤");
    println!("│  COMMAND                 │  DESCRIPTION                             │  KEY USAGE / EXAMPLE       │");
    println!("├──────────────────────────┼──────────────────────────────────────────┼────────────────────────────┤");
    println!("│  convert                 │  Explicit file conversion                │  mossambi convert in.wav mp3│");
    println!("│  check                   │  Quality & container integrity scorecard │  mossambi check file.png   │");
    println!("│  bench                   │  Benchmark latency, throughput & speedup │  mossambi bench file.wav   │");
    println!("│  archive                 │  Pack files into ZIP, 7Z, TAR, GZ, XZ, ZST│ mossambi archive -o out.zip│");
    println!("│  extract                 │  Extract archive with Zip-Slip safety    │  mossambi extract bundle.zip│");
    println!("│  upload-ready            │  Indian Exam Portal Optimizer            │  mossambi upload-ready in.jpg│");
    println!("│  verify                  │  Mathematical 0.00% loss verification    │  mossambi verify a.png b.png│");
    println!("│  catalogue               │  Search 200+ format registry             │  mossambi catalogue lossless│");
    println!("│  hw                      │  Display GPU/NPU hardware telemetry      │  mossambi hw               │");
    println!("│  tui                     │  Full-screen Apple-styled interactive TUI│  mossambi tui              │");
    println!("│  gui                     │  Launch macOS-styled studio workspace    │  mossambi gui              │");
    println!("│  widget                  │  Summon Sweet Lime radial hero overlay   │  mossambi widget           │");
    println!("│  daemon                  │  Background service (Whisper Flow style) │  mossambi daemon           │");
    println!("│                          │  Hotkeys: Win+Shift+C / Win+Ctrl+C       │  mossambi daemon install   │");
    println!("│  config                  │  View or edit persistent preferences     │  mossambi config list      │");
    println!("│  completions             │  Generate shell completions & Carapace   │  mossambi completions zsh  │");
    println!("├──────────────────────────┼──────────────────────────────────────────┼────────────────────────────┤");
    println!("│  FLAG / OPTION           │  SHORT / ARG                             │  FUNCTION                  │");
    println!("├──────────────────────────┼──────────────────────────────────────────┼────────────────────────────┤");
    println!("│  --all                   │  -a                                      │  Batch convert to ALL valid│");
    println!("│  --output                │  -o <DIR>                                │  Custom destination folder │");
    println!("│  --strict                │  (default: true)                         │  Enforce 0.00% Zero-Loss   │");
    println!("│  --json                  │  --json                                  │  Emit script-ready JSON    │");
    println!("│  --exam                  │  --exam <PRESET>                         │  upsc/ssc/ibps photo/sign  │");
    println!("│  --width / --height      │  --width <PX> / --height <PX>            │  Exact pixel dimensions    │");
    println!("│  --dpi                   │  --dpi <INT>                             │  Target DPI (200 / 300)    │");
    println!("│  --min-kb / --max-kb     │  --min-kb <N> / --max-kb <N>             │  Target byte envelope range│");
    println!("│  --whiten                │  --whiten                                │  Background illumination   │");
    println!("│  --help                  │  -h                                      │  Show this command table   │");
    println!("└──────────────────────────┴──────────────────────────────────────────┴────────────────────────────┘");
}
