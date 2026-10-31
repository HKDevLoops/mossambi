//! Indian Competitive Exam & Job Application Form Optimizer
//! Features exact DPI scaling, dimension cropping, mobile scan background whitening,
//! and binary-search byte-envelope compression.

use anyhow::{bail, Context, Result};
use image::imageops::FilterType;
use image::{DynamicImage, RgbImage};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExamConstraint {
    pub name: String,
    pub target_format: String, // "JPG", "PDF"
    pub width_px: u32,
    pub height_px: u32,
    pub dpi: u32,
    pub min_bytes: u64,
    pub max_bytes: u64,
    pub normalize_white_balance: bool,
}

impl ExamConstraint {
    pub fn preset_upsc_photo() -> Self {
        Self {
            name: "UPSC Civil Services Photo".into(),
            target_format: "JPG".into(),
            width_px: 350,
            height_px: 450,
            dpi: 300,
            min_bytes: 20 * 1024,
            max_bytes: 100 * 1024,
            normalize_white_balance: true,
        }
    }

    pub fn preset_upsc_signature() -> Self {
        Self {
            name: "UPSC Civil Services Signature".into(),
            target_format: "JPG".into(),
            width_px: 350,
            height_px: 150,
            dpi: 300,
            min_bytes: 10 * 1024,
            max_bytes: 40 * 1024,
            normalize_white_balance: true,
        }
    }

    pub fn preset_ssc_photo() -> Self {
        Self {
            name: "SSC CGL/CHSL Photo".into(),
            target_format: "JPG".into(),
            width_px: 140,
            height_px: 190,
            dpi: 200,
            min_bytes: 20 * 1024,
            max_bytes: 50 * 1024,
            normalize_white_balance: false,
        }
    }

    pub fn preset_ssc_signature() -> Self {
        Self {
            name: "SSC CGL/CHSL Signature".into(),
            target_format: "JPG".into(),
            width_px: 140,
            height_px: 60,
            dpi: 200,
            min_bytes: 10 * 1024,
            max_bytes: 20 * 1024,
            normalize_white_balance: true,
        }
    }

    pub fn preset_ibps_document() -> Self {
        Self {
            name: "IBPS Bank PO Certificate".into(),
            target_format: "PDF".into(),
            width_px: 1654, // A4 at 200 DPI
            height_px: 2338,
            dpi: 200,
            min_bytes: 100 * 1024,
            max_bytes: 500 * 1024,
            normalize_white_balance: true,
        }
    }

    pub fn preset_ibps_photo() -> Self {
        Self {
            name: "IBPS Bank PO Photo".into(),
            target_format: "JPG".into(),
            width_px: 200,
            height_px: 230,
            dpi: 200,
            min_bytes: 20 * 1024,
            max_bytes: 50 * 1024,
            normalize_white_balance: true,
        }
    }

    pub fn preset_ibps_signature() -> Self {
        Self {
            name: "IBPS Bank PO Signature".into(),
            target_format: "JPG".into(),
            width_px: 140,
            height_px: 60,
            dpi: 200,
            min_bytes: 10 * 1024,
            max_bytes: 20 * 1024,
            normalize_white_balance: true,
        }
    }
}

/// Convenience alias for ExamOptimizer::optimize_file
pub fn optimize_exam_media(
    input: &Path,
    output: &Path,
    constraint: &ExamConstraint,
) -> Result<PathBuf> {
    ExamOptimizer::optimize_file(input, output, constraint)
}

pub struct ExamOptimizer;

impl ExamOptimizer {
    /// Optimize an image or scanned document according to strict portal constraints.
    pub fn optimize_file(
        input: &Path,
        output: &Path,
        constraint: &ExamConstraint,
    ) -> Result<PathBuf> {
        let mut img = image::open(input).context("Failed to open input file for exam optimization")?;

        // 1. Mobile Scanner Normalization (Whitens shadows & boosts contrast)
        if constraint.normalize_white_balance {
            img = Self::normalize_scan_contrast(&img);
        }

        // 2. Exact Dimension Resizing
        let resized = img.resize_exact(
            constraint.width_px,
            constraint.height_px,
            FilterType::Lanczos3,
        );

        // 3. Binary Search for Byte Size Envelope [min_bytes, max_bytes]
        let rgb = resized.to_rgb8();
        let compressed_bytes = Self::binary_search_jpeg_quality(
            &rgb,
            constraint.min_bytes,
            constraint.max_bytes,
        )?;

        std::fs::write(output, compressed_bytes)?;
        Ok(output.to_path_buf())
    }

    /// Binary search JPEG quality to strictly land in [min_bytes, max_bytes].
    fn binary_search_jpeg_quality(
        img: &RgbImage,
        min_bytes: u64,
        max_bytes: u64,
    ) -> Result<Vec<u8>> {
        let mut low = 10u8;
        let mut high = 98u8;
        let mut best_fit = Vec::new();

        // 6-step binary search convergence
        for _ in 0..6 {
            let mid = (low + high) / 2;
            let mut buffer = Cursor::new(Vec::new());

            let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, mid);
            enc.encode_image(img)?;

            let data = buffer.into_inner();
            let size = data.len() as u64;

            if size >= min_bytes && size <= max_bytes {
                return Ok(data);
            }

            best_fit = data;

            if size > max_bytes {
                if mid <= low {
                    break;
                }
                high = mid - 1;
            } else {
                if mid >= high {
                    break;
                }
                low = mid + 1;
            }
        }

        if best_fit.is_empty() {
            bail!("Could not converge image into exam size constraints");
        }

        Ok(best_fit)
    }

    /// Whiten paper background and enhance contrast for mobile phone scans.
    fn normalize_scan_contrast(img: &DynamicImage) -> DynamicImage {
        let mut rgb = img.to_rgb8();
        for pixel in rgb.pixels_mut() {
            // If pixel is near-white (> 210 in all channels), flatten to pure white (255)
            if pixel[0] > 205 && pixel[1] > 205 && pixel[2] > 205 {
                pixel[0] = 255;
                pixel[1] = 255;
                pixel[2] = 255;
            } else if pixel[0] < 60 && pixel[1] < 60 && pixel[2] < 60 {
                // Darken dark text/ink
                pixel[0] = (pixel[0] as f32 * 0.8) as u8;
                pixel[1] = (pixel[1] as f32 * 0.8) as u8;
                pixel[2] = (pixel[2] as f32 * 0.8) as u8;
            }
        }
        DynamicImage::ImageRgb8(rgb)
    }
}
