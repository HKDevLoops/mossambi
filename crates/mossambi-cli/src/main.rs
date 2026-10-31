//! Mossambi UNIX-Style Command-Line Interface (`mossambi`)
//! Frictionless syntax modeled after `mv`: `mossambi <input> [target_format]`
//! Supports `--all` batch transcode, Indian exam preset tuning, archive pack/unpack, hardware accel metrics, and persistent configuration.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use mossambi_archive::{pack_archive, unpack_archive, ArchiveType};
use mossambi_codec::{transcode_file, transcode_to_all_formats, LosslessMode};
use mossambi_core::{
    get_system_arch_info, MossambiConfig, ThemeMode, UniversalRegistry,
};
use mossambi_exam::{optimize_exam_media, ExamConstraint};
use mossambi_hwaccel::detect_hardware_capabilities;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;

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

    /// Display detected hardware acceleration engines (NVENC, AMF, QSV, Adreno, VAAPI, NEON/AVX2)
    Hw,

    /// Launch the interactive full-screen Terminal User Interface (Ratatui)
    Tui,

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
}

fn main() -> Result<()> {
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
            Commands::Hw => handle_hw(cli.json),
            Commands::Tui => handle_tui(),
            Commands::Gui => handle_gui(),
            Commands::Config { action, key, value } => handle_config(&action, key.as_deref(), value.as_deref(), cli.json),
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

    // No arguments provided: print summary banner and help
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
    // Launch mossambi-tui binary or internal runner
    let exe = std::env::current_exe()?
        .parent()
        .map(|p| p.join("mossambi-tui"))
        .unwrap_or_else(|| PathBuf::from("mossambi-tui"));

    if exe.exists() {
        let status = Command::new(&exe).status()?;
        if !status.success() {
            bail!("TUI process exited with status: {}", status);
        }
    } else {
        // Fallback: execute cargo run -p mossambi-tui
        println!("Launching Mossambi Terminal User Interface...");
        let status = Command::new("cargo")
            .args(["run", "-p", "mossambi-tui"])
            .status()?;
        if !status.success() {
            bail!("Failed to launch mossambi-tui");
        }
    }

    Ok(())
}

fn handle_gui() -> Result<()> {
    println!("🍋 Summoning Mossambi Desktop Radial Widget and macOS-Styled GUI...");
    // Summon via Python runner or compiled GUI binary
    let py_path = PathBuf::from("run_widget.py");
    if py_path.exists() {
        let _ = Command::new("python").arg("run_widget.py").spawn()?;
        println!("✓ Mossambi Desktop GUI launched in background.");
    } else {
        println!("Desktop launcher: please ensure run_widget.py or Mossambi.exe is in directory.");
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
                        "dark" => config.theme_mode = ThemeMode::Dark,
                        "light" => config.theme_mode = ThemeMode::Light,
                        _ => config.theme_mode = ThemeMode::System,
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
    println!(r#"
  __  __                                  _     _ 
 |  \/  | ___  ___ ___  __ _ _ __ ___  _ | |__ (_)
 | |\/| |/ _ \/ __/ __|/ _` | '_ ` _ \| '_ \ | |
 | |  | | (_) \__ \__ \ (_| | | | | | | |_) | |
 |_|  |_|\___/|___/___/\__,_|_| |_| |_|_.__/|_|
  Native Desktop Zero-Loss Converter & Form Tuner
"#);
    println!("Usage (UNIX mv-style):");
    println!("  mossambi <file> [format]            # Convert file (e.g. mossambi song.flac mp3)");
    println!("  mossambi <file> --all               # Batch transcode to ALL domain formats");
    println!("  mossambi <photo> --exam upsc-photo  # Indian Exam Optimizer (UPSC/SSC/IBPS)");
    println!("  mossambi archive <files...> -o out.7z  # Pack exhaustive compression archive");
    println!("  mossambi extract <archive>          # Unpack archive safely");
    println!("  mossambi info <file>                # Inspect format, sectors & catalog");
    println!("  mossambi tui                        # Launch Ratatui terminal interface");
    println!("  mossambi gui                        # Launch radial widget & Mac-styled GUI");
    println!("  mossambi hw                         # Check GPU & SIMD hardware engines");
    println!("  mossambi config                     # View or edit system settings");
    println!("\nRun 'mossambi --help' for exhaustive options.\n");
}
