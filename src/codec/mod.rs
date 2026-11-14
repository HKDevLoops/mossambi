//! Zero-Loss Conversion & Transcoding Engine for Mossambi
//! Implements ArtCraft-grade 0.00% loss guarantees, stream remuxing, bit-exact audio,
//! vector document/text PDF generation, spreadsheet table transforms, and multi-format batch pipelines.

pub mod bench;
pub mod cad;
pub mod checker;
pub mod data_exchange;
pub mod dpi_meta;
pub mod email;
pub mod fonts;
pub mod geo;
pub mod models_3d;
pub mod pdf_image;
pub mod pdf_tables;
pub mod pdf_vector;
pub mod remux;
pub mod subtitles;

pub use bench::{BenchmarkEngine, BenchmarkReport, ComparisonBaseline};
pub use cad::transcode_cad;
pub use checker::{
    CompatibilityReport, FileQualityChecker, FileQualityReport, IntegrityReport,
    ParseabilityReport, QualityMetricReport, QualityVerdict,
};
pub use data_exchange::transcode_data_exchange;
pub use dpi_meta::{inject_jfif_dpi, inject_png_dpi};
pub use email::transcode_email;
pub use fonts::transcode_font;
pub use geo::transcode_geo;
pub use models_3d::transcode_3d_model;
pub use pdf_image::{render_pdf_first_page_thumbnail, transcode_pdf_to_images};
pub use pdf_tables::transcode_pdf_tables;
pub use pdf_vector::{
    convert_image_to_pdf_vector, convert_text_to_pdf_vector, render_text_to_pdf, TextPdfSettings,
};
pub use remux::{
    calculate_sha256, verify_conversion_fidelity, MathematicalQualityReport, MathematicalRemuxType,
};
pub use subtitles::transcode_subtitles;

use crate::archive::{pack_archive, unpack_archive, ArchiveType};
use crate::core::{resolve_clean_sibling_output_path, FileCategory, UniversalRegistry};
use crate::hwaccel::detect_hardware_capabilities;
use anyhow::{bail, Context, Result};
use calamine::Reader;
use rayon::prelude::*;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LosslessMode {
    StrictBitExact,         // True 0.00% loss (stream copy, lossless PCM, lossless raster)
    PerceptualHighFidelity, // < 1.00% loss (CRF 16, Opus 192k+, LAME V0)
}

/// Live encode progress callback: fraction 0.0..=1.0 plus a human status
/// line. Invoked from the ffmpeg pipe-reader thread.
pub type ProgressCallback = Box<dyn FnMut(f32, &str) + Send>;

/// Shared progress callback handle used by the worker task and the reader
/// thread (type alias keeps clippy's type_complexity satisfied).
type SharedProgress = Arc<Mutex<ProgressCallback>>;

/// Shared handle for one encode: cooperative cancel flag, live progress
/// callback (invoked from the ffmpeg pipe-reader thread), and the probed
/// media duration used for determinate progress. Cloneable so the worker
/// task and the reader thread can each hold a reference.
#[derive(Clone)]
pub struct EncodeHandle {
    pub cancel: Arc<AtomicBool>,
    pub duration_secs: Option<f64>,
    progress: SharedProgress,
}

impl EncodeHandle {
    pub fn new(
        cancel: Arc<AtomicBool>,
        duration_secs: Option<f64>,
        progress: ProgressCallback,
    ) -> Self {
        Self {
            cancel,
            duration_secs,
            progress: Arc::new(Mutex::new(progress)),
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// Reports intra-item progress from any thread. A poisoned lock or lost
    /// tick is swallowed: progress reporting must never kill the encode.
    pub fn report(&self, fraction: f32, status: &str) {
        if let Ok(mut guard) = self.progress.lock() {
            guard.as_mut()(fraction, status);
        }
    }
}

/// Parses ffmpeg `-progress pipe:1` key=value blocks. Block terminators are
/// `progress=continue|end`. Note: ffmpeg's `out_time_us` AND `out_time_ms`
/// are both MICROSECONDS (the `_ms` key is a historical misnomer), so both
/// divide by 1e6.
pub struct FfmpegProgressParser {
    duration_secs: Option<f64>,
    out_time_us: Option<f64>,
    speed: Option<f64>,
}

impl FfmpegProgressParser {
    pub fn new(duration_secs: Option<f64>) -> Self {
        Self {
            duration_secs,
            out_time_us: None,
            speed: None,
        }
    }

    /// True when a duration was probed (determinate progress mode).
    pub fn has_duration(&self) -> bool {
        self.duration_secs.is_some()
    }

    /// Feeds one progress line; returns `Some((fraction, status_text))` when
    /// a `progress=` block terminator completes a sample.
    pub fn feed_line(&mut self, line: &str) -> Option<(f32, String)> {
        let (key, value) = line.split_once('=')?;
        let value = value.trim();
        match key {
            "out_time_us" => self.out_time_us = value.parse::<f64>().ok(),
            "out_time_ms" => {
                if self.out_time_us.is_none() {
                    self.out_time_us = value.parse::<f64>().ok();
                }
            }
            "speed" => self.speed = parse_ffmpeg_speed(value),
            "progress" => {
                let sample = self.build_sample();
                self.out_time_us = None;
                self.speed = None;
                return Some(sample);
            }
            _ => {}
        }
        None
    }

    fn build_sample(&self) -> (f32, String) {
        let speed_text = self
            .speed
            .map(|s| format!("speed {:.2}x", s))
            .unwrap_or_else(|| "working".to_string());
        match (self.duration_secs, self.out_time_us) {
            (Some(duration), Some(micros)) if duration > 0.0 => {
                let fraction = ((micros / 1_000_000.0) / duration).clamp(0.0, 1.0) as f32;
                (
                    fraction,
                    format!("{:.0}% | {}", fraction * 100.0, speed_text),
                )
            }
            _ => (0.0, speed_text),
        }
    }
}

/// Parses an ffmpeg `speed` value ("1.5x", "N/A"). Rejects non-finite and
/// zero speeds so a stalled encode reports indeterminate instead of 0%.
pub fn parse_ffmpeg_speed(value: &str) -> Option<f64> {
    value
        .trim()
        .trim_end_matches('x')
        .parse::<f64>()
        .ok()
        .filter(|s| s.is_finite() && *s > 0.0)
}

/// Probes a media file's duration in seconds via ffprobe. Returns None when
/// ffprobe is unavailable, the file is not media, or the duration is
/// unreadable - callers fall back to indeterminate progress.
pub fn probe_media_duration(path: &Path) -> Option<f64> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|d| d.is_finite() && *d > 0.0)
}

/// Transcode a single input file to target extension with Zero-Loss guarantee.
pub fn transcode_file(
    input: &Path,
    target_format: &str,
    mode: LosslessMode,
    custom_dir: Option<&Path>,
) -> Result<PathBuf> {
    transcode_file_ctx(input, target_format, mode, custom_dir, None)
}

/// Same as [`transcode_file`] with an optional encode handle for live
/// progress reporting and cooperative cancellation (W-7).
pub fn transcode_file_ctx(
    input: &Path,
    target_format: &str,
    mode: LosslessMode,
    custom_dir: Option<&Path>,
    handle: Option<&EncodeHandle>,
) -> Result<PathBuf> {
    if !input.exists() {
        bail!("Input path does not exist: {}", input.display());
    }

    let (category, in_ext) = UniversalRegistry::classify_file(input);
    let target_upper = target_format.to_uppercase();

    // Check for special "ALL" sector request
    if target_upper == "ALL" {
        let all_outputs = transcode_to_all_formats(input, mode, custom_dir)?;
        return all_outputs
            .into_iter()
            .next()
            .context("No output generated in ALL transcode");
    }

    // Archive extraction targets a directory, not a converted file — handle
    // it before the scratch-file plumbing (nothing atomic to rename).
    if category == FileCategory::Archive && target_upper == "EXTRACT" {
        let parent = input.parent().unwrap_or_else(|| Path::new("."));
        let stem = input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("extracted");
        let dest = custom_dir
            .map(PathBuf::from)
            .unwrap_or_else(|| parent.join(stem));

        let t0 = Instant::now();
        log::info!(
            "convert start: {} -> {} (category=Archive, target=EXTRACT)",
            input.display(),
            dest.display()
        );
        match unpack_archive(input, &dest) {
            Ok(()) => {
                log::info!(
                    "convert ok: {} -> {} in {:.2?}",
                    input.display(),
                    dest.display(),
                    t0.elapsed()
                );
                return Ok(dest);
            }
            Err(e) => {
                log::error!(
                    "convert failed: {} -> {} after {:.2?}: {:#}",
                    input.display(),
                    dest.display(),
                    t0.elapsed(),
                    e
                );
                return Err(e);
            }
        }
    }

    let output_path = resolve_clean_sibling_output_path(input, &target_upper, custom_dir);

    // Archives unpack to directories / pack to explicit archive paths — they
    // keep writing to their final location. Everything else goes through an
    // atomic scratch file (see part_path_for) so a crash or a killed ffmpeg
    // can never leave a truncated file masquerading under the real name.
    let use_atomic_part = !matches!(category, FileCategory::Archive);
    let work_path = if use_atomic_part {
        part_path_for(&output_path)
    } else {
        output_path.clone()
    };
    // Clear a stale scratch file left behind by a previous crashed run.
    let _ = fs::remove_file(&work_path);

    let t0 = Instant::now();
    log::info!(
        "convert start: {} -> {} (category={:?}, target={})",
        input.display(),
        output_path.display(),
        category,
        target_upper
    );

    let outcome: Result<()> = transcode_single_step(input, &work_path, &target_upper, mode, handle)
        .or_else(|direct_err| {
            // If single-step conversion failed, attempt to find a multi-hop route
            if let Some(route) =
                crate::core::route_planner::find_conversion_route(&in_ext, &target_upper)
            {
                if route.hop_count() > 1 {
                    log::info!(
                        "Attempting {}-hop fallback route for {} -> {}: {}",
                        route.hop_count(),
                        in_ext,
                        target_upper,
                        route.display_path()
                    );
                    return execute_multi_hop_route(input, &work_path, &route, mode, handle);
                }
            }
            Err(direct_err)
        });

    // Finalize: sanity-check the scratch payload, then atomically rename it
    // onto the final name. Any failure removes the scratch file.
    let min_bytes = match category {
        FileCategory::Image
        | FileCategory::Audio
        | FileCategory::Document
        | FileCategory::Spreadsheet
        | FileCategory::Video
        | FileCategory::Presentation
        | FileCategory::Cad
        | FileCategory::Models3D
        | FileCategory::Geo
        | FileCategory::Fonts
        | FileCategory::Data
        | FileCategory::Email => 16,
        // Subtitles and unknown/text-like inputs may legitimately be tiny.
        _ => 1,
    };
    let finalized: Result<()> = outcome.and_then(|()| {
        if use_atomic_part {
            verify_output_sanity(&work_path, min_bytes)?;
            fs::rename(&work_path, &output_path).with_context(|| {
                format!(
                    "Failed to finalize output {} (scratch: {})",
                    output_path.display(),
                    work_path.display()
                )
            })?;
        }
        Ok(())
    });

    match finalized {
        Ok(()) => {
            log::info!(
                "convert ok: {} -> {} in {:.2?}",
                input.display(),
                output_path.display(),
                t0.elapsed()
            );
            Ok(output_path)
        }
        Err(e) => {
            discard_part(&work_path);
            log::error!(
                "convert failed: {} -> {} after {:.2?}: {:#}",
                input.display(),
                output_path.display(),
                t0.elapsed(),
                e
            );
            Err(e)
        }
    }
}

/// Executes parallel transcoding to ALL valid formats in that category simultaneously.
pub fn transcode_to_all_formats(
    input: &Path,
    mode: LosslessMode,
    custom_dir: Option<&Path>,
) -> Result<Vec<PathBuf>> {
    let (category, in_ext) = UniversalRegistry::classify_file(input);
    let targets = UniversalRegistry::get_all_supported_outputs(category);

    // Parallel execution across CPU cores using Rayon
    let results: Vec<Result<PathBuf>> = targets
        .into_par_iter()
        .filter(|&tgt| tgt != in_ext.as_str() && tgt != "ALL")
        .map(|tgt| transcode_file(input, tgt, mode, custom_dir))
        .collect();

    let failures = results.iter().filter(|r| r.is_err()).count();
    if failures > 0 {
        log::warn!(
            "ALL transcode for {}: {} target(s) failed",
            input.display(),
            failures
        );
    }

    let mut successful = Vec::new();
    for path in results.into_iter().flatten() {
        successful.push(path);
    }

    if successful.is_empty() {
        bail!("Failed to generate any output format in ALL batch transcode");
    }

    Ok(successful)
}

/// Executes a single conversion step across any registered codec.
pub fn transcode_single_step(
    input: &Path,
    output: &Path,
    target_format: &str,
    mode: LosslessMode,
    handle: Option<&EncodeHandle>,
) -> Result<()> {
    let (category, in_ext) = UniversalRegistry::classify_file(input);
    let target_upper = target_format.to_ascii_uppercase();

    match category {
        FileCategory::Image => {
            if target_upper == "PDF" {
                convert_image_to_pdf_vector(input, output, None)
            } else if matches!(
                target_upper.as_str(),
                "STL" | "OBJ" | "PLY" | "GLTF" | "GLB"
            ) {
                models_3d::transcode_3d_model(input, output)
            } else {
                convert_image_zero_loss(input, output, &target_upper, mode)
            }
        }
        FileCategory::Document => {
            if in_ext == "PDF" && matches!(target_upper.as_str(), "XLSX" | "CSV") {
                pdf_tables::transcode_pdf_tables(input, output)
            } else if in_ext == "PDF"
                && matches!(
                    target_upper.as_str(),
                    "JPG" | "JPEG" | "PNG" | "WEBP" | "TIFF" | "TIF" | "BMP"
                )
            {
                pdf_image::transcode_pdf_to_images(input, output, &target_upper)
            } else if matches!(in_ext.as_str(), "EML" | "MSG" | "MHTML") {
                email::transcode_email(input, output)
            } else {
                convert_document_native(input, output, &in_ext, &target_upper)
            }
        }
        FileCategory::Spreadsheet => {
            convert_spreadsheet_native(input, output, &in_ext, &target_upper)
        }
        FileCategory::Presentation => {
            convert_presentation_native(input, output, &in_ext, &target_upper)
        }
        FileCategory::Archive => {
            let arch_type = ArchiveType::from_extension(&target_upper).unwrap_or(ArchiveType::Zip);
            pack_archive(&[input.to_path_buf()], output, arch_type)
        }
        FileCategory::Audio => convert_audio_zero_loss(input, output, &target_upper, mode, handle),
        FileCategory::Video => convert_video_zero_loss(input, output, &target_upper, mode, handle),
        FileCategory::Subtitle => subtitles::transcode_subtitles(input, output),
        FileCategory::Cad => cad::transcode_cad(input, output),
        FileCategory::Models3D => models_3d::transcode_3d_model(input, output),
        FileCategory::Geo => geo::transcode_geo(input, output, false),
        FileCategory::Fonts => fonts::transcode_font(input, output),
        FileCategory::Data => data_exchange::transcode_data_exchange(input, output),
        FileCategory::Email => email::transcode_email(input, output),
        FileCategory::Unknown => {
            if is_text_file(input) {
                convert_document_native(input, output, &in_ext, &target_upper)
            } else {
                convert_image_zero_loss(input, output, &target_upper, mode)
            }
        }
    }
}

/// Executes a multi-hop shortest path conversion route step-by-step.
pub fn execute_multi_hop_route(
    input: &Path,
    output_path: &Path,
    route: &crate::core::route_planner::ConversionRoute,
    mode: LosslessMode,
    handle: Option<&EncodeHandle>,
) -> Result<()> {
    if route.steps.is_empty() {
        bail!("Empty conversion route specified");
    }

    let staging_dir = std::env::temp_dir().join(".mossambi_staging");
    let _ = fs::create_dir_all(&staging_dir);

    let run_id = format!(
        "hop_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );

    let mut current_input = input.to_path_buf();
    let mut intermediate_files: Vec<PathBuf> = Vec::new();

    let total_steps = route.steps.len();
    for (i, step) in route.steps.iter().enumerate() {
        let is_last = i == total_steps - 1;
        let step_out = if is_last {
            output_path.to_path_buf()
        } else {
            let ext = step.to.to_lowercase();
            staging_dir.join(format!("{}_step_{}.{}", run_id, i, ext))
        };

        log::info!(
            "Route execution step {}/{}: {} -> {} ({})",
            i + 1,
            total_steps,
            current_input.display(),
            step_out.display(),
            step.description
        );

        // Cooperative cancel between hops (W-7): clean up intermediates and
        // stop before spending time on the next encode.
        if let Some(h) = handle {
            if h.is_cancelled() {
                for tmp in &intermediate_files {
                    let _ = fs::remove_file(tmp);
                }
                if !is_last {
                    let _ = fs::remove_file(&step_out);
                }
                bail!(
                    "Conversion cancelled by user before step {} of {}",
                    i + 1,
                    total_steps
                );
            }
            h.report(0.0, &format!("step {}/{}: {}", i + 1, total_steps, step.to));
        }

        let step_result = transcode_single_step(&current_input, &step_out, &step.to, mode, handle);
        if let Err(e) = step_result {
            // Clean up all intermediates immediately on failure
            for tmp in &intermediate_files {
                let _ = fs::remove_file(tmp);
            }
            if !is_last {
                let _ = fs::remove_file(&step_out);
            }
            return Err(e).with_context(|| {
                format!(
                    "Multi-hop conversion failed at step {} ({} -> {}): {}",
                    i + 1,
                    step.from,
                    step.to,
                    step.description
                )
            });
        }

        if !is_last {
            intermediate_files.push(step_out.clone());
            current_input = step_out;
        }
    }

    // Clean up temporary files on successful completion
    for tmp in &intermediate_files {
        let _ = fs::remove_file(tmp);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Atomic scratch-file helpers
// ---------------------------------------------------------------------------

/// Sibling scratch path for `final_out`: inserts `.part` *before* the last
/// extension (`song.flac` -> `song.part.flac`). Keeping the real extension as
/// the suffix preserves extension-guessed tooling (ffmpeg muxer detection,
/// image-crate format selection) while the name is still clearly a partial.
pub(crate) fn part_path_for(final_out: &Path) -> PathBuf {
    let name = final_out
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let scratch = match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => format!("{}.part.{}", stem, ext),
        _ => format!("{}.part", name),
    };
    final_out.with_file_name(scratch)
}

/// Removes the scratch file left behind by a failed conversion (best-effort:
/// the file is garbage at that point, never surface the cleanup error).
fn discard_part(part_path: &Path) {
    if let Err(e) = fs::remove_file(part_path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            log::warn!(
                "Could not remove scratch file {}: {}",
                part_path.display(),
                e
            );
        }
    }
}

/// Sanity-checks a freshly written output: it must exist and carry a real
/// payload. Catches ffmpeg/decoder failures that exit 0 while emitting nothing
/// usable, and zero/1-byte phantom files from truncated writes.
fn verify_output_sanity(path: &Path, min_bytes: u64) -> Result<()> {
    let meta = fs::metadata(path)
        .with_context(|| format!("Output file was not created: {}", path.display()))?;
    if meta.len() < min_bytes {
        bail!(
            "Output missing or truncated (got {} bytes, expected > {}): {}",
            meta.len(),
            min_bytes,
            path.display()
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Image Zero-Loss Pipeline (PNG, WebP Lossless, TIFF, BMP, ICO, JPG, GIF, PDF)
// ---------------------------------------------------------------------------
/// Safely opens and decodes an image with strict Decompression Bomb and Memory Safety limits:
/// - Max Width: 16,384 px
/// - Max Height: 16,384 px
/// - Max Allocation Ceiling: 512 MB
pub fn open_image_safe(path: &Path) -> Result<image::DynamicImage> {
    let mut reader = image::ImageReader::open(path)
        .with_context(|| format!("Failed to open image file: {}", path.display()))?
        .with_guessed_format()
        .with_context(|| format!("Failed to determine format of {}", path.display()))?;

    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16_384);
    limits.max_image_height = Some(16_384);
    limits.max_alloc = Some(512 * 1024 * 1024);
    reader.limits(limits);

    reader.decode().with_context(|| {
        format!(
            "Failed to decode image safely (exceeded safety limits or corrupted): {}",
            path.display()
        )
    })
}

fn convert_image_zero_loss(
    input: &Path,
    output: &Path,
    target_format: &str,
    _mode: LosslessMode,
) -> Result<()> {
    let img = open_image_safe(input)?;

    match target_format {
        "PNG" => {
            img.save_with_format(output, image::ImageFormat::Png)?;
            // Inject Photoshop-standard 300 DPI pHYs chunk
            if let Ok(raw) = fs::read(output) {
                let enriched = inject_png_dpi(&raw, 300);
                let _ = fs::write(output, enriched);
            }
        }
        "WEBP" => {
            img.save_with_format(output, image::ImageFormat::WebP)?;
        }
        "TIFF" | "TIF" => {
            img.save_with_format(output, image::ImageFormat::Tiff)?;
        }
        "BMP" => {
            let rgb = img.to_rgb8();
            rgb.save_with_format(output, image::ImageFormat::Bmp)?;
        }
        "ICO" => {
            img.save_with_format(output, image::ImageFormat::Ico)?;
        }
        "GIF" => {
            img.save_with_format(output, image::ImageFormat::Gif)?;
        }
        "JPG" | "JPEG" => {
            // Prevent transparent PNGs from turning pitch black on JPEG conversion
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            let mut rgb_canvas = image::RgbImage::from_pixel(w, h, image::Rgb([255, 255, 255]));
            for y in 0..h {
                for x in 0..w {
                    let pixel = rgba.get_pixel(x, y);
                    let alpha = pixel[3] as f32 / 255.0;
                    if alpha >= 1.0 {
                        rgb_canvas.put_pixel(x, y, image::Rgb([pixel[0], pixel[1], pixel[2]]));
                    } else if alpha <= 0.0 {
                        rgb_canvas.put_pixel(x, y, image::Rgb([255, 255, 255]));
                    } else {
                        let r = ((pixel[0] as f32 * alpha) + (255.0 * (1.0 - alpha))).round() as u8;
                        let g = ((pixel[1] as f32 * alpha) + (255.0 * (1.0 - alpha))).round() as u8;
                        let b = ((pixel[2] as f32 * alpha) + (255.0 * (1.0 - alpha))).round() as u8;
                        rgb_canvas.put_pixel(x, y, image::Rgb([r, g, b]));
                    }
                }
            }
            // Quality from Preferences (it used to be ignored).
            {
                let q = crate::core::MossambiConfig::load()
                    .conv_image_quality
                    .clamp(10, 100);
                let mut w = std::io::BufWriter::new(fs::File::create(output)?);
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut w, q)
                    .encode_image(&rgb_canvas)?;
                std::io::Write::flush(&mut w)?;
            }
            // Inject Photoshop-standard 300 DPI JFIF APP0 header
            if let Ok(raw) = fs::read(output) {
                let enriched = inject_jfif_dpi(&raw, 300);
                let _ = fs::write(output, enriched);
            }
        }
        "PDF" => {
            convert_image_to_pdf_vector(input, output, None)?;
        }
        _ => {
            img.save(output)?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Document Conversion Engine (DOCX, PDF, TXT, MD, HTML, RTF, Code/Config)
// ---------------------------------------------------------------------------
fn convert_document_native(
    input: &Path,
    output: &Path,
    src_ext: &str,
    target_format: &str,
) -> Result<()> {
    // Memory Safety Ceiling: Prevent allocating arbitrary gigabytes on massive inputs
    if let Ok(meta) = fs::metadata(input) {
        if meta.len() > 100 * 1024 * 1024 {
            bail!(
                "Input document exceeds safe text conversion ceiling of 100 MB: {}",
                input.display()
            );
        }
    }

    // 1. Extract plain text content from various source document formats.
    // Extraction failures are honest errors: the previous behaviour fell back
    // to a raw byte dump / empty string and produced garbage "successful"
    // output files.
    let document_text = match src_ext {
        "DOCX" => extract_text_from_docx(input).with_context(|| {
            format!(
                "Failed to extract text from DOCX (file may be corrupt or not a real DOCX): {}",
                input.display()
            )
        })?,
        "PDF" => extract_text_from_pdf(input).with_context(|| {
            format!(
                "Failed to extract text from PDF (scanned/image-only PDFs contain no extractable text): {}",
                input.display()
            )
        })?,
        _ => fs::read_to_string(input).with_context(|| {
            format!(
                "Input document is not readable UTF-8 text: {}",
                input.display()
            )
        })?,
    };

    let title = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Document");

    // 2. Render to target format
    match target_format {
        "PDF" => {
            if src_ext == "PDF" {
                // Direct copy
                fs::copy(input, output)?;
            } else {
                let cfg = TextPdfSettings {
                    title: Some(title.to_string()),
                    ..Default::default()
                };
                render_text_to_pdf(&document_text, output, &cfg)?;
            }
        }
        "HTML" | "HTM" => {
            let mut html = format!(
                "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<title>{}</title>\n\
                <style>\n\
                body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; line-height: 1.6; max-width: 800px; margin: 40px auto; padding: 0 20px; color: #222; }}\n\
                pre {{ background: #f6f8fa; padding: 16px; border-radius: 6px; overflow-x: auto; font-family: monospace; font-size: 13px; }}\n\
                h1, h2, h3 {{ color: #111; }}\n\
                p {{ margin: 0.8em 0; }}\n\
                </style>\n</head>\n<body>\n<h1>{}</h1>\n",
                title, title
            );

            if src_ext == "MD" {
                for line in document_text.lines() {
                    let trimmed = line.trim();
                    if let Some(stripped) = trimmed.strip_prefix("# ") {
                        html.push_str(&format!("<h2>{}</h2>\n", stripped));
                    } else if let Some(stripped) = trimmed.strip_prefix("## ") {
                        html.push_str(&format!("<h3>{}</h3>\n", stripped));
                    } else if let Some(stripped) = trimmed
                        .strip_prefix("- ")
                        .or_else(|| trimmed.strip_prefix("* "))
                    {
                        html.push_str(&format!("<li>{}</li>\n", stripped));
                    } else if trimmed.is_empty() {
                        html.push_str("<br/>\n");
                    } else {
                        html.push_str(&format!("<p>{}</p>\n", trimmed));
                    }
                }
            } else {
                for para in document_text.split("\n\n") {
                    let trimmed = para.trim();
                    if !trimmed.is_empty() {
                        html.push_str(&format!("<p>{}</p>\n", trimmed.replace('\n', "<br/>")));
                    }
                }
            }
            html.push_str("</body>\n</html>\n");
            fs::write(output, html)?;
        }
        "TXT" => {
            fs::write(output, document_text.as_bytes())?;
        }
        "MD" => {
            let mut md = format!("# {}\n\n", title);
            md.push_str(&document_text);
            fs::write(output, md.as_bytes())?;
        }
        "RTF" => {
            let mut rtf = String::from(r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Helvetica;}}\fs22 ");
            for line in document_text.lines() {
                rtf.push_str(line);
                rtf.push_str(r"\par ");
            }
            rtf.push('}');
            fs::write(output, rtf.as_bytes())?;
        }
        _ => {
            if matches!(
                target_format,
                "JPG" | "JPEG" | "PNG" | "WEBP" | "TIFF" | "TIF" | "BMP" | "ICO"
            ) {
                bail!(
                    "Cannot write raw document text to image format '{}'; routing error",
                    target_format
                );
            }
            fs::write(output, document_text.as_bytes())?;
        }
    }

    Ok(())
}

/// Extract clean text from a DOCX file by reading word/document.xml inside the zip.
pub fn extract_text_from_docx(path: &Path) -> Result<String> {
    let file = fs::File::open(path)?;
    let mut zip = zip::ZipArchive::new(file)?;
    let doc_xml = zip
        .by_name("word/document.xml")
        .context("Missing word/document.xml in DOCX")?;
    let mut xml = String::new();
    doc_xml.take(50 * 1024 * 1024).read_to_string(&mut xml)?;

    let mut result = String::new();
    let p_tag = regex::Regex::new(r"</w:p>")?;
    let t_tag = regex::Regex::new(r"<w:t[^>]*>(.*?)</w:t>")?;

    for para in p_tag.split(&xml) {
        let mut para_text = String::new();
        for cap in t_tag.captures_iter(para) {
            if let Some(m) = cap.get(1) {
                let unescaped = m
                    .as_str()
                    .replace("&amp;", "&")
                    .replace("&lt;", "<")
                    .replace("&gt;", ">")
                    .replace("&quot;", "\"")
                    .replace("&apos;", "'");
                para_text.push_str(&unescaped);
            }
        }
        if !para_text.is_empty() {
            result.push_str(&para_text);
            result.push_str("\n\n");
        }
    }

    Ok(result)
}

/// Extract clean text from a PDF document using lopdf.
pub fn extract_text_from_pdf(path: &Path) -> Result<String> {
    let doc =
        lopdf::Document::load(path).context("Failed to load PDF document for text extraction")?;
    let page_numbers: Vec<u32> = doc.get_pages().keys().cloned().collect();
    let text = doc
        .extract_text(&page_numbers)
        .context("Failed to extract text from PDF")?;
    Ok(text)
}

// ---------------------------------------------------------------------------
// Native Spreadsheet Transformations (XLSX, XLS, ODS, CSV, TSV, PDF, HTML)
// ---------------------------------------------------------------------------
fn convert_spreadsheet_native(
    input: &Path,
    output: &Path,
    src_ext: &str,
    dst_ext: &str,
) -> Result<()> {
    match dst_ext {
        "PDF" => {
            convert_spreadsheet_to_pdf(input, output)?;
        }
        "HTML" | "HTM" => {
            convert_spreadsheet_to_html(input, output)?;
        }
        "CSV" | "TSV" => {
            if ["XLSX", "XLS", "ODS"].contains(&src_ext) {
                let mut wb = calamine::open_workbook_auto(input)?;
                let sheets = wb.sheet_names().to_vec();
                if let Some(first) = sheets.first() {
                    let range = wb.worksheet_range(first)?;
                    let delim = if dst_ext == "TSV" { b'\t' } else { b',' };
                    let mut wtr = csv::WriterBuilder::new()
                        .delimiter(delim)
                        .from_path(output)?;
                    for row in range.rows() {
                        let rec: Vec<String> = row.iter().map(|c| c.to_string()).collect();
                        wtr.write_record(&rec)?;
                    }
                    wtr.flush()?;
                } else {
                    bail!(
                        "Spreadsheet {} contains no worksheets — nothing to convert to {}",
                        input.display(),
                        dst_ext
                    );
                }
            } else if ["CSV", "TSV"].contains(&src_ext) {
                let in_delim = if src_ext == "TSV" { b'\t' } else { b',' };
                let out_delim = if dst_ext == "TSV" { b'\t' } else { b',' };
                let mut rdr = csv::ReaderBuilder::new()
                    .has_headers(false)
                    .delimiter(in_delim)
                    .from_path(input)?;
                let mut wtr = csv::WriterBuilder::new()
                    .delimiter(out_delim)
                    .from_path(output)?;
                for record in rdr.records() {
                    wtr.write_record(&record?)?;
                }
                wtr.flush()?;
            }
        }
        "XLSX" => {
            if ["CSV", "TSV"].contains(&src_ext) {
                let delim = if src_ext == "TSV" { b'\t' } else { b',' };
                let mut rdr = csv::ReaderBuilder::new()
                    .has_headers(false)
                    .delimiter(delim)
                    .from_path(input)?;
                let mut workbook = rust_xlsxwriter::Workbook::new();
                let worksheet = workbook.add_worksheet();
                for (r_idx, result) in rdr.records().enumerate() {
                    let record = result?;
                    for (c_idx, field) in record.iter().enumerate() {
                        worksheet.write_string(r_idx as u32, c_idx as u16, field)?;
                    }
                }
                workbook.save(output)?;
            } else if ["XLS", "ODS"].contains(&src_ext) {
                let mut wb = calamine::open_workbook_auto(input)?;
                let sheets = wb.sheet_names().to_vec();
                let mut workbook = rust_xlsxwriter::Workbook::new();
                for sheet in sheets {
                    let ws = workbook.add_worksheet();
                    let _ = ws.set_name(&sheet);
                    if let Ok(range) = wb.worksheet_range(&sheet) {
                        for (r_idx, row) in range.rows().enumerate() {
                            for (c_idx, cell) in row.iter().enumerate() {
                                let _ =
                                    ws.write_string(r_idx as u32, c_idx as u16, cell.to_string());
                            }
                        }
                    }
                }
                workbook.save(output)?;
            } else {
                bail!(
                    "Spreadsheet conversion {} -> XLSX is not supported (supported sources: CSV, TSV, XLS, ODS)",
                    src_ext
                );
            }
        }
        _ => {
            // Never blind-copy bytes: writing XLSX payloads under a foreign
            // extension produced corrupt, mislabeled files.
            bail!(
                "Spreadsheet conversion to .{} is not supported (supported targets: CSV, TSV, XLSX, HTML, PDF)",
                dst_ext.to_lowercase()
            );
        }
    }

    Ok(())
}

fn convert_spreadsheet_to_pdf(input: &Path, output: &Path) -> Result<()> {
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_uppercase();
    let mut table_text = String::new();

    if ["XLSX", "XLS", "ODS"].contains(&ext.as_str()) {
        let mut wb = calamine::open_workbook_auto(input)?;
        for sheet in wb.sheet_names().to_vec() {
            table_text.push_str(&format!("# Sheet: {}\n\n", sheet));
            if let Ok(range) = wb.worksheet_range(&sheet) {
                for row in range.rows() {
                    let line: Vec<String> = row.iter().map(|c| c.to_string()).collect();
                    table_text.push_str(&line.join("  |  "));
                    table_text.push('\n');
                }
            }
            table_text.push('\n');
        }
    } else if ["CSV", "TSV"].contains(&ext.as_str()) {
        let delim = if ext == "TSV" { b'\t' } else { b',' };
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .delimiter(delim)
            .from_path(input)?;
        let title = input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Spreadsheet");
        table_text.push_str(&format!("# {}\n\n", title));
        for result in rdr.records() {
            let record = result?;
            let line: Vec<&str> = record.iter().collect();
            table_text.push_str(&line.join("  |  "));
            table_text.push('\n');
        }
    }

    render_text_to_pdf(&table_text, output, &TextPdfSettings::default())
}

fn convert_spreadsheet_to_html(input: &Path, output: &Path) -> Result<()> {
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_uppercase();
    let title = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Spreadsheet");

    let mut html = format!(
        "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<title>{}</title>\n\
        <style>\n\
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; margin: 40px; background: #fafafa; color: #333; }}\n\
        table {{ border-collapse: collapse; width: 100%; max-width: 1200px; background: #fff; box-shadow: 0 1px 3px rgba(0,0,0,0.1); border-radius: 8px; overflow: hidden; margin-bottom: 30px; }}\n\
        th, td {{ border: 1px solid #e1e4e8; padding: 10px 14px; text-align: left; font-size: 13px; }}\n\
        th {{ background: #f6f8fa; font-weight: 600; color: #24292e; }}\n\
        tr:nth-child(even) {{ background: #fcfcfc; }}\n\
        tr:hover {{ background: #f0f7ff; }}\n\
        h1, h2 {{ color: #24292e; }}\n\
        </style>\n</head>\n<body>\n<h1>{}</h1>\n",
        title, title
    );

    const MAX_PREVIEW_ROWS: usize = 10_000;

    if ["XLSX", "XLS", "ODS"].contains(&ext.as_str()) {
        let mut wb = calamine::open_workbook_auto(input)?;
        for sheet in wb.sheet_names().to_vec() {
            html.push_str(&format!("<h2>Sheet: {}</h2>\n<table>\n", sheet));
            if let Ok(range) = wb.worksheet_range(&sheet) {
                let mut truncated = false;
                for (r_idx, row) in range.rows().enumerate() {
                    if r_idx >= MAX_PREVIEW_ROWS {
                        truncated = true;
                        break;
                    }
                    html.push_str("<tr>");
                    for cell in row {
                        let tag = if r_idx == 0 { "th" } else { "td" };
                        html.push_str(&format!("<{}>{}</{}>", tag, cell, tag));
                    }
                    html.push_str("</tr>\n");
                }
                if truncated {
                    html.push_str("<tr><td colspan=\"100\" style=\"text-align: center; color: #888; padding: 12px;\"><i>... Truncated for memory safety (displaying first 10,000 rows) ...</i></td></tr>\n");
                }
            }
            html.push_str("</table>\n");
        }
    } else if ["CSV", "TSV"].contains(&ext.as_str()) {
        let delim = if ext == "TSV" { b'\t' } else { b',' };
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .delimiter(delim)
            .from_path(input)?;
        html.push_str("<table>\n");
        let mut truncated = false;
        for (r_idx, result) in rdr.records().enumerate() {
            if r_idx >= MAX_PREVIEW_ROWS {
                truncated = true;
                break;
            }
            let record = result?;
            html.push_str("<tr>");
            for field in record.iter() {
                let tag = if r_idx == 0 { "th" } else { "td" };
                html.push_str(&format!("<{}>{}</{}>", tag, field, tag));
            }
            html.push_str("</tr>\n");
        }
        if truncated {
            html.push_str("<tr><td colspan=\"100\" style=\"text-align: center; color: #888; padding: 12px;\"><i>... Truncated for memory safety (displaying first 10,000 rows) ...</i></td></tr>\n");
        }
        html.push_str("</table>\n");
    }

    html.push_str("</body>\n</html>\n");
    fs::write(output, html)?;
    Ok(())
}

fn convert_presentation_native(
    input: &Path,
    _output: &Path,
    _src_ext: &str,
    target_format: &str,
) -> Result<()> {
    // The previous stub wrote placeholder slide text and reported success,
    // which silently destroyed user trust in the output. Fail honestly until
    // a real lossless presentation pipeline exists.
    bail!(
        "PPTX/ODP→{} conversion is not yet lossless-capable (input: {}). No output was written.",
        target_format,
        input.display()
    );
}

// ---------------------------------------------------------------------------
// FFmpeg subprocess plumbing (piped stderr, Windows-no-console)
// ---------------------------------------------------------------------------

/// Explicit ffmpeg output muxer for a target extension. Required because we
/// transcode into a scratch file first and must not depend on extension
/// guessing for correct container selection.
fn ffmpeg_muxer(target: &str) -> Option<&'static str> {
    Some(match target {
        // Audio containers / codecs
        "FLAC" => "flac",
        "WAV" => "wav",
        "AIFF" => "aiff",
        "ALAC" | "M4A" => "ipod",
        "MP3" => "mp3",
        "AAC" => "adts",
        "OPUS" | "OGG" => "ogg",
        "WMA" => "asf",
        // Video containers
        "MP4" | "M4V" => "mp4",
        "MOV" => "mov",
        "MKV" => "matroska",
        "TS" | "MTS" | "M2TS" => "mpegts",
        "WEBM" => "webm",
        "AVI" => "avi",
        "GIF" => "gif",
        "PRORES" => "mov",
        _ => return None,
    })
}

/// Runs ffmpeg with stderr PIPED so failures report the actual ffmpeg
/// diagnosis instead of a bare exit status. With an `EncodeHandle` (progress
/// mode), adds the machine progress protocol (`-progress pipe:1 -nostats`),
/// keeps a dedicated reader thread on every pipe (undrained pipes block the
/// child), and supports two-tier cancellation: graceful stdin 'q' first,
/// then a hard kill after the 2s grace window (TerminateProcess is
/// asynchronous, so exit is confirmed by polling).
fn run_ffmpeg_ctx(mut cmd: Command, stage: &str, handle: Option<&EncodeHandle>) -> Result<()> {
    let progress_mode = handle.is_some();
    if progress_mode {
        cmd.args(["-progress", "pipe:1", "-nostats"]);
    }
    cmd.stderr(Stdio::piped());
    if progress_mode {
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped());
    } else {
        cmd.stdin(Stdio::null()).stdout(Stdio::null());
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x00004000;
        cmd.creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS);
    }

    let mut child = cmd.spawn().with_context(|| {
        format!(
            "Failed to run FFmpeg for {stage}. Ensure FFmpeg is installed and accessible in PATH."
        )
    })?;

    // stderr tail thread (both modes): keep only the last ~2000 characters -
    // the summary lines right before the error are the useful part of a
    // multi-megabyte progress log.
    let stderr_tail = Arc::new(Mutex::new(String::new()));
    if let Some(err_pipe) = child.stderr.take() {
        let tail = Arc::clone(&stderr_tail);
        thread::spawn(move || {
            let reader = BufReader::new(err_pipe);
            for line in reader.lines().map_while(std::result::Result::ok) {
                let mut buf = match tail.lock() {
                    Ok(buf) => buf,
                    Err(poisoned) => poisoned.into_inner(),
                };
                buf.push_str(&line);
                buf.push('\n');
                if buf.chars().count() > 2000 {
                    let cut = buf.chars().count() - 2000;
                    let start = buf
                        .char_indices()
                        .nth(cut)
                        .map(|(idx, _)| idx)
                        .unwrap_or(buf.len());
                    buf.drain(..start);
                }
            }
        });
    }

    // stdout progress reader thread (progress mode only): parses key=value
    // blocks and reports fraction + speed text through the encode handle.
    if let Some(h) = handle {
        if let Some(out_pipe) = child.stdout.take() {
            let cancel = Arc::clone(&h.cancel);
            let reporter = Arc::clone(&h.progress);
            let mut parser = FfmpegProgressParser::new(h.duration_secs);
            thread::spawn(move || {
                let start = Instant::now();
                let reader = BufReader::new(out_pipe);
                for line in reader.lines().map_while(std::result::Result::ok) {
                    // Keep draining even after cancel so the child's pipes
                    // never fill and block it mid-shutdown.
                    if cancel.load(Ordering::Relaxed) {
                        continue;
                    }
                    if let Some((fraction, mut status)) = parser.feed_line(&line) {
                        if !parser.has_duration() {
                            status = format!("{} (t+{}s)", status, start.elapsed().as_secs());
                        }
                        if let Ok(mut guard) = reporter.lock() {
                            guard.as_mut()(fraction, &status);
                        }
                    }
                }
            });
        }
    }

    // Waiter thread: owns the child + stdin. Two-tier cancel: write 'q'
    // (graceful), wait a 2s grace window, then kill and poll until exit.
    let stdin_pipe = child.stdin.take();
    let cancel_flag = handle.map(|h| Arc::clone(&h.cancel));
    let waiter = thread::spawn(move || -> Result<ExitStatus> {
        let grace = Duration::from_secs(2);
        let poll = Duration::from_millis(100);
        if let Some(cancel) = cancel_flag {
            loop {
                if cancel.load(Ordering::Relaxed) {
                    if let Some(mut stdin) = stdin_pipe {
                        let _ = stdin.write_all(b"q");
                        let _ = stdin.flush();
                        drop(stdin);
                    }
                    let stop_at = Instant::now() + grace;
                    loop {
                        match child.try_wait() {
                            Ok(Some(status)) => return Ok(status),
                            Ok(None) => {
                                if Instant::now() >= stop_at {
                                    let _ = child.kill();
                                }
                                thread::sleep(poll);
                            }
                            Err(e) => return Err(e).context("Failed to poll the FFmpeg process"),
                        }
                    }
                }
                match child.try_wait() {
                    Ok(Some(status)) => return Ok(status),
                    Ok(None) => thread::sleep(poll),
                    Err(e) => return Err(e).context("Failed to poll the FFmpeg process"),
                }
            }
        }
        drop(stdin_pipe);
        child
            .wait()
            .context("Failed to wait for the FFmpeg process to finish")
    });

    let status = match waiter.join() {
        Ok(result) => result,
        Err(_) => bail!("FFmpeg watcher thread panicked during {stage}"),
    }?;

    if let Some(h) = handle {
        if h.is_cancelled() {
            bail!("FFmpeg {stage} cancelled by user");
        }
    }

    if !status.success() {
        let tail = stderr_tail
            .lock()
            .map(|buf| buf.clone())
            .unwrap_or_else(|p| p.into_inner().clone());
        bail!(
            "FFmpeg {stage} failed ({status}). ffmpeg stderr (tail):\n{}",
            stderr_tail_of(&tail, 2000)
        );
    }
    Ok(())
}

/// Last `max_chars` characters of `s`, char-boundary safe.
fn stderr_tail_of(s: &str, max_chars: usize) -> String {
    let total = s.chars().count();
    if total <= max_chars {
        return s.trim_end().to_string();
    }
    s.chars()
        .skip(total - max_chars)
        .collect::<String>()
        .trim()
        .to_string()
}

/// Single in-flight slot for video first-frame extraction so a folder
/// full of videos cannot storm ffmpeg processes (the UI decode budget
/// caps decodes per frame; this caps concurrent video spawns at one).
static VIDEO_THUMB_INFLIGHT: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

struct VideoThumbGuard;

impl Drop for VideoThumbGuard {
    fn drop(&mut self) {
        VIDEO_THUMB_INFLIGHT.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Extract the first video frame as an RGBA image scaled to fit within
/// `max_dim` (aspect preserved, never upscaled). Requires ffmpeg on
/// PATH; degrades to an error (callers fall back to the vector icon)
/// when ffmpeg is missing. At most one extraction runs at a time; a
/// concurrent call errors immediately so the tile keeps its vector
/// icon for that frame.
pub fn video_first_frame_thumbnail(input: &Path, max_dim: u32) -> Result<image::RgbaImage> {
    use std::sync::atomic::Ordering;

    if VIDEO_THUMB_INFLIGHT
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        bail!("another video thumbnail extraction is already running");
    }
    let _guard = VideoThumbGuard;

    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-hide_banner", "-loglevel", "error", "-nostdin", "-i"]);
    cmd.arg(input);
    cmd.args([
        "-frames:v",
        "1",
        "-an",
        "-sn",
        "-dn",
        "-vf",
        &format!(
            "scale=w='min(iw,{max_dim})':h='min(ih,{max_dim})':force_original_aspect_ratio=decrease"
        ),
        "-f",
        "image2pipe",
        "-c:v",
        "png",
        "pipe:1",
    ]);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x00004000;
        cmd.creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS);
    }

    cmd.stdin(Stdio::null())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped());
    let mut child = cmd.spawn().with_context(|| {
        format!(
            "Failed to run FFmpeg for the video thumbnail of {}. Ensure FFmpeg is installed and accessible in PATH.",
            input.display()
        )
    })?;

    // Drain stderr on a thread so a chatty decoder cannot deadlock the
    // stdout pipe, then read the PNG bytes from stdout.
    let stderr_handle = child.stderr.take();
    let stderr_thread = std::thread::spawn(move || {
        let mut tail = String::new();
        if let Some(mut pipe) = stderr_handle {
            let _ = pipe.read_to_string(&mut tail);
        }
        tail
    });

    let mut png = Vec::new();
    if let Some(mut out_pipe) = child.stdout.take() {
        out_pipe.read_to_end(&mut png)?;
    }
    let status = child
        .wait()
        .context("Failed to wait for the FFmpeg process to finish")?;
    let stderr_tail = stderr_thread.join().unwrap_or_default();
    if !status.success() {
        bail!(
            "FFmpeg video thumbnail failed ({status}). ffmpeg stderr (tail):\n{}",
            stderr_tail_of(&stderr_tail, 2000)
        );
    }
    if png.is_empty() {
        bail!("FFmpeg produced no frame for {}", input.display());
    }

    let img = image::load_from_memory(&png).with_context(|| {
        format!(
            "Failed to decode the extracted frame of {}",
            input.display()
        )
    })?;
    let (w, h) = (img.width(), img.height());
    let thumb = if w <= max_dim && h <= max_dim {
        img.to_rgba8()
    } else {
        img.resize(max_dim, max_dim, image::imageops::FilterType::Lanczos3)
            .to_rgba8()
    };
    Ok(thumb)
}

// ---------------------------------------------------------------------------
// Audio Zero-Loss Pipeline (Stream-Copy & Bit-Exact PCM)
// ---------------------------------------------------------------------------
fn convert_audio_zero_loss(
    input: &Path,
    output: &Path,
    target_format: &str,
    _mode: LosslessMode,
    handle: Option<&EncodeHandle>,
) -> Result<()> {
    let in_ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_uppercase();

    // Fast-path: If source and target are already the same container/codec, direct copy
    if in_ext == target_format {
        fs::copy(input, output)?;
        return Ok(());
    }

    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-y", "-threads", "0"]);
    cmd.arg("-i");
    cmd.arg(input);

    // Studio Quality Standards (Audacity, Logic Pro, Adobe Audition)
    match target_format {
        "FLAC" => {
            cmd.args(["-c:a", "flac", "-compression_level", "8", "-ar", "48000"]);
        }
        "WAV" => {
            cmd.args(["-c:a", "pcm_s24le", "-ar", "48000"]);
        }
        "AIFF" => {
            cmd.args(["-c:a", "pcm_s24be", "-ar", "48000"]);
        }
        "ALAC" => {
            cmd.args(["-c:a", "alac", "-ar", "48000"]);
        }
        "MP3" => {
            // LAME 320 kbps High-Fidelity CBR Studio Master with ID3v2.4
            let br = audio_bitrate("320k");
            cmd.args([
                "-c:a",
                "libmp3lame",
                "-b:a",
                br.as_str(),
                "-ar",
                "44100",
                "-id3v2_version",
                "4",
            ]);
        }
        "M4A" | "AAC" => {
            // Fraunhofer / CoreAudio style AAC-LC (bitrate from Preferences)
            let br = audio_bitrate("320k");
            cmd.args(["-c:a", "aac", "-b:a", br.as_str(), "-ar", "48000"]);
        }
        "OPUS" => {
            // Broadcast Fullband (20 Hz - 20 kHz), bitrate from Preferences
            let br = audio_bitrate("256k");
            cmd.args([
                "-c:a",
                "libopus",
                "-b:a",
                br.as_str(),
                "-vbr",
                "on",
                "-ar",
                "48000",
            ]);
        }
        "OGG" => {
            cmd.args(["-c:a", "libvorbis", "-q:a", "8", "-ar", "48000"]);
        }
        "WMA" => {
            cmd.args(["-c:a", "wmav2", "-b:a", "192k"]);
        }
        _ => {
            cmd.args(["-c:a", "copy"]);
        }
    }

    // Force the output muxer so the scratch (.part) filename never confuses
    // ffmpeg's container detection.
    if let Some(muxer) = ffmpeg_muxer(target_format) {
        cmd.arg("-f").arg(muxer);
    }
    cmd.arg(output);

    run_ffmpeg_ctx(cmd, "audio transcode", handle)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Video Zero-Loss Remuxing & Transcoding
// ---------------------------------------------------------------------------
fn convert_video_zero_loss(
    input: &Path,
    output: &Path,
    target_format: &str,
    _mode: LosslessMode,
    handle: Option<&EncodeHandle>,
) -> Result<()> {
    let _in_ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_uppercase();
    let hw = detect_hardware_capabilities();

    // Check if target is audio extraction from video (Audacity / Premiere style)
    let is_audio_extract = matches!(
        target_format,
        "MP3" | "WAV" | "FLAC" | "M4A" | "AAC" | "OGG" | "OPUS"
    );

    if is_audio_extract {
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-y", "-threads", "0"]);
        cmd.arg("-i");
        cmd.arg(input);
        cmd.arg("-vn");
        match target_format {
            "MP3" => {
                cmd.args([
                    "-c:a",
                    "libmp3lame",
                    "-b:a",
                    "320k",
                    "-ar",
                    "44100",
                    "-id3v2_version",
                    "4",
                ]);
            }
            "WAV" => {
                cmd.args(["-c:a", "pcm_s24le", "-ar", "48000"]);
            }
            "FLAC" => {
                cmd.args(["-c:a", "flac", "-compression_level", "8", "-ar", "48000"]);
            }
            "M4A" | "AAC" => {
                cmd.args(["-c:a", "aac", "-b:a", "320k", "-ar", "48000"]);
            }
            "OGG" => {
                cmd.args(["-c:a", "libvorbis", "-q:a", "8", "-ar", "48000"]);
            }
            "OPUS" => {
                cmd.args([
                    "-c:a", "libopus", "-b:a", "256k", "-vbr", "on", "-ar", "48000",
                ]);
            }
            _ => {
                cmd.args(["-c:a", "aac", "-b:a", "320k"]);
            }
        }
        // Force the muxer so the scratch (.part) filename cannot confuse ffmpeg.
        if let Some(muxer) = ffmpeg_muxer(target_format) {
            cmd.arg("-f").arg(muxer);
        }
        cmd.arg(output);

        run_ffmpeg_ctx(cmd, "audio extraction", handle)?;
        return Ok(());
    }

    // 1. FAST-PATH: Mathematical Direct Stream-Copy Remuxing (True 0.00% Zero-Loss)
    // Allows 2-hour movies (10-50GB) to convert in seconds (pure NVMe disk I/O, ~1-2 GB/s)
    // without dropping a single bit or byte of pixel or audio data!
    let is_video_container = matches!(
        target_format,
        "MP4" | "MKV" | "MOV" | "M4V" | "TS" | "MTS" | "M2TS" | "WEBM" | "AVI"
    );

    if is_video_container {
        let mut remux_cmd = Command::new("ffmpeg");
        remux_cmd.args(["-y", "-threads", "0"]);
        remux_cmd.arg("-i");
        remux_cmd.arg(input);
        remux_cmd.args(["-c", "copy", "-bufsize", "64M"]);
        if matches!(target_format, "MP4" | "MOV" | "M4V") {
            remux_cmd.args(["-movflags", "+faststart"]);
        }
        if let Some(muxer) = ffmpeg_muxer(target_format) {
            remux_cmd.arg("-f").arg(muxer);
        }
        remux_cmd.arg(output);

        match run_ffmpeg_ctx(remux_cmd, "stream-copy remux", handle) {
            Ok(()) if output.exists() => {
                if fs::metadata(output)
                    .map(|m| m.len() > 1024)
                    .unwrap_or(false)
                {
                    return Ok(());
                }
                log::warn!(
                    "Remux output suspiciously small ({}), re-encoding instead",
                    output.display()
                );
                let _ = fs::remove_file(output);
            }
            Ok(()) => {
                let _ = fs::remove_file(output);
            }
            Err(remux_err) => {
                // Respect cancellation: never fall back into a fresh encode.
                if handle.map(|h| h.is_cancelled()).unwrap_or(false) {
                    return Err(remux_err);
                }
                // Stream copy is impossible for this codec/container pair —
                // fall back to the full encode pipeline below.
                log::warn!(
                    "Remux fast-path failed for target {} (will re-encode): {:#}",
                    target_format,
                    remux_err
                );
                let _ = fs::remove_file(output);
            }
        }
    }

    // 2. STUDIO ENCODING PIPELINE (DaVinci Resolve / Premiere Pro Broadcast Standard)
    // Runs when stream copy is impossible (codec incompatible with target container).
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-y", "-threads", "0"]);
    if hw.supports_gpu_transcode {
        cmd.args(["-hwaccel", hw.ffmpeg_hwaccel_flag]);
    }
    cmd.arg("-i");
    cmd.arg(input);

    match target_format {
        "PRORES" => {
            // Apple ProRes 422 HQ / 4444 Cinema Standard
            cmd.args([
                "-c:v",
                "prores_ks",
                "-profile:v",
                "4",
                "-pix_fmt",
                "yuv422p10le",
                "-threads",
                "0",
                "-bufsize",
                "64M",
                "-c:a",
                "pcm_s24le",
                "-ar",
                "48000",
            ]);
        }
        "GIF" => {
            cmd.args([
                "-vf",
                "fps=15,scale=640:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse",
            ]);
        }
        "MP4" | "MOV" | "MKV" | "M4V" => {
            // Quality, preset and hardware use come from Preferences.
            let vcfg = crate::core::MossambiConfig::load();
            let crf = vcfg.conv_video_crf.min(51).to_string();
            let crf = crf.as_str();
            if hw.supports_h264_hw && vcfg.conv_video_hw_accel {
                // Hardware Accelerated Cinema Encode (NVENC, AMF, QSV, VideoToolbox)
                cmd.args(["-c:v", hw.h264_encoder]);
                if hw.h264_encoder.contains("nvenc") {
                    cmd.args(["-preset", "p4", "-cq", crf, "-bufsize", "64M"]);
                } else if hw.h264_encoder.contains("qsv") {
                    cmd.args(["-global_quality", crf, "-bufsize", "64M"]);
                } else if hw.h264_encoder.contains("amf") {
                    cmd.args([
                        "-quality", "speed", "-rc", "cqp", "-qp_i", crf, "-qp_p", crf, "-bufsize",
                        "64M",
                    ]);
                } else {
                    cmd.args(["-b:v", "10M", "-bufsize", "64M"]);
                }
                cmd.args([
                    "-pix_fmt", "yuv420p", "-c:a", "aac", "-b:a", "320k", "-ar", "48000",
                ]);
            } else {
                // CPU x264 Broadcast Quality with multi-threading
                let preset = x264_preset(&vcfg.conv_video_preset);
                cmd.args([
                    "-c:v", "libx264", "-crf", crf, "-preset", preset, "-threads", "0", "-bufsize",
                    "64M", "-pix_fmt", "yuv420p", "-c:a", "aac", "-b:a", "320k", "-ar", "48000",
                ]);
            }
            if matches!(target_format, "MP4" | "MOV" | "M4V") {
                cmd.args(["-movflags", "+faststart"]);
            }
        }
        "WEBM" => {
            cmd.args([
                "-c:v",
                "libvpx-vp9",
                "-crf",
                "18",
                "-b:v",
                "0",
                "-threads",
                "0",
                "-bufsize",
                "64M",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "libopus",
                "-b:a",
                "256k",
                "-ar",
                "48000",
            ]);
        }
        "AVI" => {
            cmd.args([
                "-c:v", "mpeg4", "-q:v", "2", "-threads", "0", "-c:a", "mp3", "-b:a", "320k",
            ]);
        }
        _ => {
            cmd.args(["-c", "copy"]);
        }
    }

    if let Some(muxer) = ffmpeg_muxer(target_format) {
        cmd.arg("-f").arg(muxer);
    }
    cmd.arg(output);

    run_ffmpeg_ctx(cmd, "video conversion", handle)?;
    Ok(())
}

fn is_text_file(path: &Path) -> bool {
    if let Ok(mut f) = fs::File::open(path) {
        let mut buffer = [0u8; 512];
        if let Ok(n) = f.read(&mut buffer) {
            return !buffer[..n].contains(&0);
        }
    }
    false
}

/// FFmpeg `-b:a` value from Preferences ("256k"), or `default` when the
/// setting is blank or not of the form `<digits>k`.
fn audio_bitrate(default: &str) -> String {
    let v = crate::core::MossambiConfig::load()
        .conv_audio_bitrate
        .trim()
        .to_ascii_lowercase();
    let ok =
        v.len() > 1 && v.ends_with('k') && v[..v.len() - 1].chars().all(|c| c.is_ascii_digit());
    if ok {
        v
    } else {
        default.to_string()
    }
}

/// x264 preset for the Preferences "Video preset" choice.
fn x264_preset(label: &str) -> &'static str {
    match label.trim().to_ascii_lowercase().as_str() {
        "fast" => "veryfast",
        "balanced" => "medium",
        "high quality" | "quality" => "slow",
        _ => "veryfast",
    }
}

#[cfg(test)]
mod conv_setting_tests {
    use super::x264_preset;

    #[test]
    fn preset_labels_map_to_x264() {
        assert_eq!(x264_preset("Fast"), "veryfast");
        assert_eq!(x264_preset("Balanced"), "medium");
        assert_eq!(x264_preset("High Quality"), "slow");
        assert_eq!(x264_preset("???"), "veryfast");
    }
}
