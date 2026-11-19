//! Inspection command handlers: info, check, bench, hw, verify, catalogue, themes.
//! Pure ASCII by contract.

use anyhow::{bail, Result};
use serde_json::json;
use std::path::Path;

use crate::codec::{verify_conversion_fidelity, BenchmarkEngine, FileQualityChecker};
use crate::core::{get_system_arch_info, UniversalRegistry, THEMES};
use crate::hwaccel::detect_hardware_capabilities;

pub fn handle_info(file: &Path, as_json: bool) -> Result<()> {
    if !file.exists() {
        bail!("File does not exist: {}", file.display());
    }

    let (category, ext) = UniversalRegistry::classify_file(file);
    let widget_sectors = UniversalRegistry::get_widget_sectors_for_category(category);
    let all_supported = UniversalRegistry::get_all_supported_outputs(category);
    let meta = std::fs::metadata(file)?;
    let size_kb = meta.len() as f64 / 1024.0;

    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "path": file.display().to_string(),
                "extension": ext,
                "category": format!("{:?}", category),
                "size_bytes": meta.len(),
                "size_kb": (size_kb * 10.0).round() / 10.0,
                "widget_sectors_8petal": widget_sectors,
                "all_supported_formats": all_supported
            }))?
        );
    } else {
        println!("Mossambi File Inspector");
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
                print!(" [ALL]");
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

pub fn handle_check(file: &Path, as_json: bool) -> Result<()> {
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

pub fn handle_bench(file: &Path, target: Option<&str>, as_json: bool) -> Result<()> {
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

pub fn handle_hw(as_json: bool) -> Result<()> {
    let hw = detect_hardware_capabilities();
    let specs = crate::hwaccel::detect_system_specs();
    let suggestions = crate::hwaccel::get_system_auto_suggestions(&specs);
    let (os, arch) = get_system_arch_info();

    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
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
            }))?
        );
    } else {
        println!("Mossambi Hardware Acceleration & Platform Diagnostics");
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

pub fn handle_verify(source: &Path, target: &Path, as_json: bool) -> Result<()> {
    let report = verify_conversion_fidelity(source, target)?;
    if as_json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", report.format_terminal());
    }
    Ok(())
}

pub fn handle_catalogue(query: &str, as_json: bool) -> Result<()> {
    let results = UniversalRegistry::search_catalogue(query);
    if as_json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        println!("Mossambi Universal Format Catalogue (200+ Formats)");
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
                    "Guaranteed"
                } else {
                    "Lossy"
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

pub fn handle_themes(as_json: bool) -> Result<()> {
    if as_json {
        let all = crate::core::all_pantone_themes();
        println!("{}", serde_json::to_string_pretty(all)?);
    } else {
        println!("13 NAMED THEMES (11 DARK & 1 LIGHT)");
        println!("Catalog order; each color records its source provenance.\n");

        println!("11 DARK THEMES:");
        println!("--------------------------------------------------------------");
        for (idx, theme) in THEMES.iter().take(12).enumerate() {
            println!("  {:2}. {:<24} [ID: {:<16}]", idx + 1, theme.name, theme.id);
            println!(
                "      BG:      {:<18} ({:<24}) [{}]",
                theme.bg.name, theme.bg.code, theme.bg.hex
            );
            println!(
                "      Surface: {:<18} ({:<24}) [{}]",
                theme.surface.name, theme.surface.code, theme.surface.hex
            );
            println!(
                "      Accent:  {:<18} ({:<24}) [{}]",
                theme.accent.name, theme.accent.code, theme.accent.hex
            );
            println!();
        }

        println!("1 LIGHT THEME:");
        println!("--------------------------------------------------------------");
        for (idx, theme) in THEMES.iter().skip(12).enumerate() {
            println!("  {:2}. {:<24} [ID: {:<16}]", idx + 1, theme.name, theme.id);
            println!(
                "      BG:      {:<18} ({:<24}) [{}]",
                theme.bg.name, theme.bg.code, theme.bg.hex
            );
            println!(
                "      Surface: {:<18} ({:<24}) [{}]",
                theme.surface.name, theme.surface.code, theme.surface.hex
            );
            println!(
                "      Accent:  {:<18} ({:<24}) [{}]",
                theme.accent.name, theme.accent.code, theme.accent.hex
            );
            println!();
        }
        println!("Tip: Switch themes in CLI using 'mossambi config set theme <id_or_name>'");
        println!("     or press [t] in TUI / select from Preferences tab in Desktop Studio.\n");
    }
    Ok(())
}
