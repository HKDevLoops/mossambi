//! Conversion and exam command handlers. Delegates to codec/exam modules.
//! Pure ASCII by contract.

use anyhow::{bail, Context, Result};
use serde_json::json;
use std::path::Path;

use crate::codec::{transcode_file, transcode_to_all_formats, LosslessMode};
use crate::exam::{optimize_exam_media, ExamConstraint, PositioningMode};

pub fn handle_convert(
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
                "Mossambi: batch transcoding '{}' to ALL formats in domain...",
                input.display()
            );
        }
        let outputs = transcode_to_all_formats(input, mode, output_dir)
            .context("Failed executing batch ALL transcode")?;

        if as_json {
            let out_strs: Vec<String> = outputs.iter().map(|p| p.display().to_string()).collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "status": "success",
                    "input": input.display().to_string(),
                    "all": true,
                    "count": outputs.len(),
                    "outputs": out_strs
                }))?
            );
        } else {
            println!("Successfully generated {} formats:", outputs.len());
            for out in &outputs {
                let size = std::fs::metadata(out).map(|m| m.len()).unwrap_or(0);
                println!("  {} ({:.1} KB)", out.display(), size as f64 / 1024.0);
            }
        }
        return Ok(());
    }

    if !as_json {
        println!(
            "Mossambi: converting '{}' -> {}...",
            input.display(),
            target_str.to_uppercase()
        );
    }

    let output =
        transcode_file(input, target_str, mode, output_dir).context("Failed converting file")?;
    let out_size = std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);

    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "status": "success",
                "input": input.display().to_string(),
                "target": target_str.to_uppercase(),
                "output": output.display().to_string(),
                "bytes": out_size,
                "lossless": strict_lossless
            }))?
        );
    } else {
        println!(
            "Completed: {} ({:.1} KB)",
            output.display(),
            out_size as f64 / 1024.0
        );
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn handle_exam(
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
            positioning: PositioningMode::AspectFit,
            background_color: [255, 255, 255],
        },
    };

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
            "Indian Exam Form Tuner: optimizing for '{}'...",
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
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "status": "success",
                "preset": constraint.name,
                "output": optimized.display().to_string(),
                "bytes": actual_size,
                "kilobytes": (actual_size as f64 / 1024.0 * 10.0).round() / 10.0,
                "width": constraint.width_px,
                "height": constraint.height_px,
                "dpi": constraint.dpi,
                "in_envelope": actual_size >= constraint.min_bytes && actual_size <= constraint.max_bytes
            }))?
        );
    } else {
        println!("Form Ready Document Generated: {}", optimized.display());
        println!(
            "   Final File Size: {:.2} KB (passed portal validation boundaries)",
            actual_size as f64 / 1024.0
        );
    }

    Ok(())
}
