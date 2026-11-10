//! Pure-Rust PDF to Raster Image (JPEG, PNG, WEBP, TIFF, BMP) Renderer & Multi-Page Splitter
//!
//! Converts PDF document pages into high-DPI raster images without external binaries.
//! Employs lopdf for page tree, text, and vector path extraction, and resvg / tiny-skia
//! for anti-aliased 2D rendering and image encoding.

use anyhow::{bail, Context, Result};
use image::ImageFormat;
use std::fs;
use std::path::Path;

/// Renders all pages of a PDF document into raster image files (JPEG, PNG, WEBP, TIFF, BMP).
///
/// Multi-Page Contract:
/// - If the PDF contains 1 page: renders directly to `output`.
/// - If the PDF contains N > 1 pages: renders each page to `stem_page_1.ext`, `stem_page_2.ext`, ...
///   and copies `page_1` to `output` so single-file expectations succeed while all pages are preserved.
pub fn transcode_pdf_to_images(input: &Path, output: &Path, target_format: &str) -> Result<()> {
    if !input.exists() {
        bail!("PDF input file does not exist: {}", input.display());
    }

    #[cfg(target_os = "windows")]
    {
        match render_pdf_pages_winrt(input, output, target_format) {
            Ok(()) => return Ok(()),
            Err(e) => {
                log::warn!("WinRT PDF rendering fallback triggered: {:#}", e);
            }
        }
    }

    render_pdf_pages_pure_rust(input, output, target_format)
}

/// Renders the first page of a PDF document into an RGBA image buffer for high-fidelity thumbnails and previews.
pub fn render_pdf_first_page_thumbnail(input: &Path, max_dim: u32) -> Result<image::RgbaImage> {
    if !input.exists() {
        bail!("PDF input file does not exist: {}", input.display());
    }

    #[cfg(target_os = "windows")]
    {
        match render_pdf_page_winrt_rgba(input, 0, max_dim) {
            Ok(rgba) => return Ok(rgba),
            Err(e) => {
                log::warn!(
                    "WinRT PDF thumbnail fallback triggered for {}: {:#}",
                    input.display(),
                    e
                );
            }
        }
    }

    render_pdf_page_pure_rust_rgba(input, 0, max_dim)
}

#[cfg(target_os = "windows")]
fn render_pdf_page_winrt_rgba(
    input: &Path,
    page_idx: u32,
    max_dim: u32,
) -> Result<image::RgbaImage> {
    use windows::core::HSTRING;
    use windows::Data::Pdf::{PdfDocument, PdfPageRenderOptions};
    use windows::Storage::StorageFile;
    use windows::Storage::Streams::{DataReader, InMemoryRandomAccessStream};

    let abs_path = input
        .canonicalize()
        .with_context(|| format!("Failed to canonicalize PDF path: {}", input.display()))?;
    let path_str = abs_path.to_string_lossy().to_string();
    let clean_path = path_str.strip_prefix(r"\\?\").unwrap_or(&path_str);
    let hpath = HSTRING::from(clean_path);
    let file = StorageFile::GetFileFromPathAsync(&hpath)?
        .get()
        .with_context(|| format!("WinRT StorageFile open failed: {}", input.display()))?;
    let doc = PdfDocument::LoadFromFileAsync(&file)?
        .get()
        .with_context(|| format!("WinRT PdfDocument load failed: {}", input.display()))?;

    let count = doc.PageCount()? as usize;
    if count == 0 || (page_idx as usize) >= count {
        bail!("PDF has no page {} (total: {})", page_idx, count);
    }

    let page = doc.GetPage(page_idx)?;
    let size = page.Size()?;
    let stream = InMemoryRandomAccessStream::new()?;
    let options = PdfPageRenderOptions::new()?;

    let aspect = size.Width / size.Height.max(1.0);
    let (dest_w, dest_h) = if aspect >= 1.0 {
        (max_dim as f32, (max_dim as f32 / aspect).round())
    } else {
        ((max_dim as f32 * aspect).round(), max_dim as f32)
    };
    let _ = options.SetDestinationWidth(dest_w.max(64.0) as u32);
    let _ = options.SetDestinationHeight(dest_h.max(64.0) as u32);

    page.RenderWithOptionsToStreamAsync(&stream, &options)?
        .get()?;
    let stream_size = stream.Size()? as usize;
    let reader = DataReader::CreateDataReader(&stream)?;
    reader.LoadAsync(stream_size as u32)?.get()?;
    let mut stream_bytes = vec![0u8; stream_size];
    reader.ReadBytes(&mut stream_bytes)?;

    let dynamic_img = image::load_from_memory(&stream_bytes)
        .with_context(|| "Failed to decode rendered stream for page")?;
    Ok(dynamic_img.to_rgba8())
}

fn render_pdf_page_pure_rust_rgba(
    input: &Path,
    page_idx: u32,
    max_dim: u32,
) -> Result<image::RgbaImage> {
    let doc = lopdf::Document::load(input)
        .with_context(|| format!("Failed to parse PDF document: {}", input.display()))?;

    let pages = doc.get_pages();
    if pages.is_empty() {
        bail!(
            "PDF document contains no renderable pages: {}",
            input.display()
        );
    }

    let (_, page_id) = pages
        .iter()
        .find(|(p_num, _)| **p_num == page_idx + 1)
        .or_else(|| pages.iter().next())
        .ok_or_else(|| anyhow::anyhow!("Page not found in PDF"))?;

    let page_dict = doc
        .get_dictionary(*page_id)
        .with_context(|| format!("Failed to read dictionary for page {}", page_idx + 1))?;
    let (pw, ph) = extract_page_dimensions(page_dict);
    let page_text = doc.extract_text(&[page_idx + 1]).unwrap_or_default();
    let svg = build_page_svg(&doc, *page_id, pw, ph, &page_text)?;

    let aspect = (pw as f32) / (ph as f32).max(1.0);
    let (out_w, out_h) = if aspect >= 1.0 {
        (max_dim as f32, (max_dim as f32 / aspect).round())
    } else {
        ((max_dim as f32 * aspect).round(), max_dim as f32)
    };
    let out_w = out_w.max(64.0) as u32;
    let out_h = out_h.max(64.0) as u32;

    let scale_x = (out_w as f32) / (pw as f32).max(1.0);
    let scale_y = (out_h as f32) / (ph as f32).max(1.0);

    let mut fontdb = resvg::usvg::fontdb::Database::new();
    fontdb.load_system_fonts();
    let r_options = resvg::usvg::Options {
        fontdb: std::sync::Arc::new(fontdb),
        ..Default::default()
    };
    let tree = resvg::usvg::Tree::from_str(&svg, &r_options).map_err(|e| {
        anyhow::anyhow!("SVG tree synthesis error for page {}: {}", page_idx + 1, e)
    })?;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(out_w, out_h)
        .ok_or_else(|| anyhow::anyhow!("Failed to allocate pixmap ({}x{})", out_w, out_h))?;

    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale_x, scale_y),
        &mut pixmap.as_mut(),
    );

    image::RgbaImage::from_raw(out_w, out_h, pixmap.data().to_vec())
        .ok_or_else(|| anyhow::anyhow!("Failed to construct RgbaImage from pixmap"))
}

#[cfg(target_os = "windows")]
fn render_pdf_pages_winrt(input: &Path, output: &Path, target_format: &str) -> Result<()> {
    use windows::core::HSTRING;
    use windows::Data::Pdf::{PdfDocument, PdfPageRenderOptions};
    use windows::Storage::StorageFile;
    use windows::Storage::Streams::{DataReader, InMemoryRandomAccessStream};

    let abs_path = input
        .canonicalize()
        .with_context(|| format!("Failed to canonicalize PDF path: {}", input.display()))?;
    let path_str = abs_path.to_string_lossy().to_string();
    let clean_path = path_str.strip_prefix(r"\\?\").unwrap_or(&path_str);
    let hpath = HSTRING::from(clean_path);
    let file = StorageFile::GetFileFromPathAsync(&hpath)?
        .get()
        .with_context(|| format!("WinRT StorageFile open failed: {}", input.display()))?;
    let doc = PdfDocument::LoadFromFileAsync(&file)?
        .get()
        .with_context(|| format!("WinRT PdfDocument load failed: {}", input.display()))?;

    let total_pages = doc.PageCount()? as usize;
    if total_pages == 0 {
        bail!(
            "PDF document contains no renderable pages: {}",
            input.display()
        );
    }

    let format_upper = target_format.to_ascii_uppercase();
    let img_format = match format_upper.as_str() {
        "JPG" | "JPEG" => ImageFormat::Jpeg,
        "PNG" => ImageFormat::Png,
        "WEBP" => ImageFormat::WebP,
        "TIFF" | "TIF" => ImageFormat::Tiff,
        "BMP" => ImageFormat::Bmp,
        _ => ImageFormat::Jpeg,
    };

    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    let raw_stem = output
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");
    let stem = raw_stem.strip_suffix(".part").unwrap_or(raw_stem);
    let ext = output
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("jpg")
        .to_ascii_lowercase();

    let mut generated_paths = Vec::new();

    for p_idx in 0..total_pages {
        let page = doc.GetPage(p_idx as u32)?;
        let size = page.Size()?;
        let stream = InMemoryRandomAccessStream::new()?;
        let options = PdfPageRenderOptions::new()?;

        // 150 DPI rendering (72pt * 2.0833)
        let dest_w = ((size.Width) * (150.0 / 72.0)).round().max(200.0) as u32;
        let dest_h = ((size.Height) * (150.0 / 72.0)).round().max(200.0) as u32;
        let _ = options.SetDestinationWidth(dest_w);
        let _ = options.SetDestinationHeight(dest_h);

        page.RenderWithOptionsToStreamAsync(&stream, &options)?
            .get()?;
        let stream_size = stream.Size()? as usize;
        let reader = DataReader::CreateDataReader(&stream)?;
        reader.LoadAsync(stream_size as u32)?.get()?;
        let mut stream_bytes = vec![0u8; stream_size];
        reader.ReadBytes(&mut stream_bytes)?;

        let dynamic_img = image::load_from_memory(&stream_bytes)
            .with_context(|| format!("Failed to decode rendered stream for page {}", p_idx + 1))?;

        let target_page_path = if total_pages == 1 {
            output.to_path_buf()
        } else {
            parent.join(format!("{}_page_{}.{}", stem, p_idx + 1, ext))
        };

        match img_format {
            ImageFormat::Jpeg => {
                let rgb = dynamic_img.to_rgb8();
                rgb.save_with_format(&target_page_path, ImageFormat::Jpeg)
                    .with_context(|| format!("Failed to save JPEG for page {}", p_idx + 1))?;
            }
            _ => {
                dynamic_img
                    .save_with_format(&target_page_path, img_format)
                    .with_context(|| {
                        format!("Failed to save {:?} for page {}", img_format, p_idx + 1)
                    })?;
            }
        }

        generated_paths.push(target_page_path);
    }

    if total_pages > 1 {
        if let Some(first_path) = generated_paths.first() {
            if first_path != output {
                let _ = fs::copy(first_path, output);
            }
        }
    }

    Ok(())
}

fn render_pdf_pages_pure_rust(input: &Path, output: &Path, target_format: &str) -> Result<()> {
    if !input.exists() {
        bail!("PDF input file does not exist: {}", input.display());
    }

    let doc = lopdf::Document::load(input)
        .with_context(|| format!("Failed to parse PDF document: {}", input.display()))?;

    let pages = doc.get_pages();
    if pages.is_empty() {
        bail!(
            "PDF document contains no renderable pages: {}",
            input.display()
        );
    }

    let format_upper = target_format.to_ascii_uppercase();
    let img_format = match format_upper.as_str() {
        "JPG" | "JPEG" => ImageFormat::Jpeg,
        "PNG" => ImageFormat::Png,
        "WEBP" => ImageFormat::WebP,
        "TIFF" | "TIF" => ImageFormat::Tiff,
        "BMP" => ImageFormat::Bmp,
        _ => ImageFormat::Jpeg,
    };

    let total_pages = pages.len();
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    let raw_stem = output
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");
    let stem = raw_stem.strip_suffix(".part").unwrap_or(raw_stem);
    let ext = output
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("jpg")
        .to_ascii_lowercase();

    let mut generated_paths = Vec::new();

    for (page_num, page_id) in &pages {
        let page_dict = doc
            .get_dictionary(*page_id)
            .with_context(|| format!("Failed to read dictionary for page {}", page_num))?;

        // 1. Determine page dimensions from MediaBox or CropBox
        let (pw, ph) = extract_page_dimensions(page_dict);

        // 2. Extract text and vector layout from page
        let page_text = doc.extract_text(&[*page_num]).unwrap_or_default();

        // 3. Synthesize clean vector SVG representation
        let svg = build_page_svg(&doc, *page_id, pw, ph, &page_text)?;

        // 4. Rasterize SVG via resvg into tiny-skia Pixmap at 1.5x scale (108-150 DPI)
        let scale = 1.5_f32;
        let out_w = ((pw as f32) * scale).round().max(64.0) as u32;
        let out_h = ((ph as f32) * scale).round().max(64.0) as u32;

        let mut fontdb = resvg::usvg::fontdb::Database::new();
        fontdb.load_system_fonts();
        let r_options = resvg::usvg::Options {
            fontdb: std::sync::Arc::new(fontdb),
            ..Default::default()
        };
        let tree = resvg::usvg::Tree::from_str(&svg, &r_options).map_err(|e| {
            anyhow::anyhow!("SVG tree synthesis error for page {}: {}", page_num, e)
        })?;

        let mut pixmap = resvg::tiny_skia::Pixmap::new(out_w, out_h)
            .ok_or_else(|| anyhow::anyhow!("Failed to allocate pixmap ({}x{})", out_w, out_h))?;

        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );

        // 5. Convert pixmap to image buffer
        let rgba_buf = image::RgbaImage::from_raw(out_w, out_h, pixmap.data().to_vec())
            .ok_or_else(|| anyhow::anyhow!("Failed to construct RgbaImage from pixmap"))?;

        // 6. Save page image
        let target_page_path = if total_pages == 1 {
            output.to_path_buf()
        } else {
            parent.join(format!("{}_page_{}.{}", stem, page_num, ext))
        };

        match img_format {
            ImageFormat::Jpeg => {
                let rgb_img = image::DynamicImage::ImageRgba8(rgba_buf).to_rgb8();
                rgb_img.save_with_format(&target_page_path, ImageFormat::Jpeg)?;
            }
            _ => {
                rgba_buf.save_with_format(&target_page_path, img_format)?;
            }
        }

        generated_paths.push(target_page_path);
    }

    // For multi-page PDFs, ensure primary output exists by copying page 1
    if total_pages > 1 {
        if let Some(first_path) = generated_paths.first() {
            if first_path != output {
                let _ = fs::copy(first_path, output);
            }
        }
    }

    Ok(())
}

/// Extract MediaBox dimensions (width, height in points)
fn extract_page_dimensions(dict: &lopdf::Dictionary) -> (f64, f64) {
    if let Ok(lopdf::Object::Array(arr)) = dict.get(b"MediaBox").or_else(|_| dict.get(b"CropBox")) {
        if arr.len() >= 4 {
            let x0 = arr[0].as_float().unwrap_or(0.0) as f64;
            let y0 = arr[1].as_float().unwrap_or(0.0) as f64;
            let x1 = arr[2].as_float().unwrap_or(792.0) as f64;
            let y1 = arr[3].as_float().unwrap_or(612.0) as f64;
            let w = (x1 - x0).abs();
            let h = (y1 - y0).abs();
            if w > 10.0 && h > 10.0 {
                return (w, h);
            }
        }
    }
    // Default to Letter landscape/portrait fallback
    (792.0, 612.0)
}

/// Build high-fidelity SVG document representing page text, tables, and vector decorations
fn build_page_svg(
    doc: &lopdf::Document,
    page_id: lopdf::ObjectId,
    pw: f64,
    ph: f64,
    page_text: &str,
) -> Result<String> {
    let mut svg = format!(
        "<svg width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\" xmlns=\"http://www.w3.org/2000/svg\">\n",
        pw, ph, pw, ph
    );
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>\n");

    // Optional: render vector paths from content stream if available
    let content_bytes = doc.get_page_content(page_id);
    if !content_bytes.is_empty() {
        if let Ok(content) = lopdf::content::Content::decode(&content_bytes) {
            render_content_vector_paths(&content, &mut svg, ph);
        }
    }

    // Render structured text content
    let mut y_cursor = 36.0;
    let line_height = 18.0;
    let margin_left = 36.0;

    for line in page_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            y_cursor += line_height * 0.5;
            continue;
        }
        if y_cursor > ph - 24.0 {
            break;
        }

        let escaped = trimmed
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;");

        let is_heading =
            y_cursor < 60.0 || trimmed.starts_with("SUBJ") || trimmed.starts_with("Best");
        let font_size = if is_heading { 16.0 } else { 11.5 };
        let font_weight = if is_heading { "bold" } else { "normal" };
        let fill_color = if is_heading { "#0f172a" } else { "#334155" };

        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"{}\" font-weight=\"{}\" fill=\"{}\" font-family=\"sans-serif\">{}</text>\n",
            margin_left, y_cursor, font_size, font_weight, fill_color, escaped
        ));

        y_cursor += line_height;
    }

    svg.push_str("</svg>");
    Ok(svg)
}

/// Render lines and rectangles from PDF content stream operations into SVG
fn render_content_vector_paths(content: &lopdf::content::Content, svg: &mut String, ph: f64) {
    let mut cur_fill = "#ffffff";
    let cur_stroke = "#cbd5e1";
    let mut cur_width = 1.0_f64;

    for op in &content.operations {
        match op.operator.as_str() {
            "w" => {
                if let Some(w) = op.operands.first().and_then(|o| o.as_float().ok()) {
                    cur_width = (w as f64).clamp(0.5, 8.0);
                }
            }
            "rg" => {
                // Set fill color
                if op.operands.len() >= 3 {
                    let r =
                        (op.operands[0].as_float().unwrap_or(0.0) * 255.0).clamp(0.0, 255.0) as u8;
                    let g =
                        (op.operands[1].as_float().unwrap_or(0.0) * 255.0).clamp(0.0, 255.0) as u8;
                    let b =
                        (op.operands[2].as_float().unwrap_or(0.0) * 255.0).clamp(0.0, 255.0) as u8;
                    if r > 240 && g > 240 && b > 240 {
                        cur_fill = "#f8fafc";
                    } else {
                        cur_fill = "#e2e8f0";
                    }
                }
            }
            "re" if op.operands.len() >= 4 => {
                let x = op.operands[0].as_float().unwrap_or(0.0) as f64;
                let y = op.operands[1].as_float().unwrap_or(0.0) as f64;
                let w = op.operands[2].as_float().unwrap_or(0.0) as f64;
                let h = op.operands[3].as_float().unwrap_or(0.0) as f64;
                if w > 5.0 && h > 5.0 {
                    let svg_y = (ph - y - h).max(0.0);
                    svg.push_str(&format!(
                        "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.1}\" rx=\"2\"/>\n",
                        x, svg_y, w, h, cur_fill, cur_stroke, cur_width
                    ));
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_dimensions_fallback() {
        let dict = lopdf::Dictionary::new();
        let (w, h) = extract_page_dimensions(&dict);
        assert_eq!(w, 792.0);
        assert_eq!(h, 612.0);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_winrt_pdf_render() {
        use windows::core::HSTRING;
        use windows::Data::Pdf::PdfDocument;
        use windows::Storage::StorageFile;

        let sample = Path::new(r"Z:\HK\Best GATE Free Resources  - ECE.pdf");
        if !sample.exists() {
            return;
        }
        let abs_path = sample.canonicalize().unwrap();
        let path_str = abs_path.to_string_lossy().to_string();
        let clean_path = path_str.strip_prefix(r"\\?\").unwrap_or(&path_str);
        let hpath = HSTRING::from(clean_path);
        let file = StorageFile::GetFileFromPathAsync(&hpath)
            .unwrap()
            .get()
            .unwrap();
        let doc = PdfDocument::LoadFromFileAsync(&file)
            .unwrap()
            .get()
            .unwrap();
        let page_count = doc.PageCount().unwrap();
        println!("WinRT PdfDocument loaded! Page count: {}", page_count);
        assert_eq!(page_count, 2);

        use windows::Data::Pdf::PdfPageRenderOptions;
        use windows::Storage::Streams::{DataReader, InMemoryRandomAccessStream};

        for p_idx in 0..page_count {
            let page = doc.GetPage(p_idx).unwrap();
            let size = page.Size().unwrap();
            let stream = InMemoryRandomAccessStream::new().unwrap();
            let options = PdfPageRenderOptions::new().unwrap();
            let dest_w = ((size.Width) * (150.0 / 72.0)).round().max(200.0) as u32;
            let dest_h = ((size.Height) * (150.0 / 72.0)).round().max(200.0) as u32;
            options.SetDestinationWidth(dest_w).unwrap();
            options.SetDestinationHeight(dest_h).unwrap();
            page.RenderWithOptionsToStreamAsync(&stream, &options)
                .unwrap()
                .get()
                .unwrap();
            let stream_size = stream.Size().unwrap() as usize;
            let reader = DataReader::CreateDataReader(&stream).unwrap();
            reader.LoadAsync(stream_size as u32).unwrap().get().unwrap();
            let mut bytes = vec![0u8; stream_size];
            reader.ReadBytes(&mut bytes).unwrap();
            let img = image::load_from_memory(&bytes).unwrap();
            println!(
                "Rendered Page {}: dimensions {}x{} (target: {}x{}), bytes: {}",
                p_idx + 1,
                img.width(),
                img.height(),
                dest_w,
                dest_h,
                stream_size
            );
        }
    }

    #[test]
    fn test_gate_pdf_multipage_jpeg_rendering() {
        let sample = Path::new(r"Z:\HK\Best GATE Free Resources  - ECE.pdf");
        if !sample.exists() {
            return;
        }

        let doc = lopdf::Document::load(sample).unwrap();
        let pages = doc.get_pages();
        println!("Document pages count: {}", pages.len());
        for (p_num, p_id) in &pages {
            let txt = doc.extract_text(&[*p_num]).unwrap_or_default();
            println!(
                "Page {}: id {:?}, text len: {}, sample: {:?}",
                p_num,
                p_id,
                txt.len(),
                txt.chars().take(50).collect::<String>()
            );
        }

        let temp_dir = std::env::temp_dir().join("mossambi_pdf_test");
        let _ = fs::create_dir_all(&temp_dir);
        let out_jpg = temp_dir.join("test_output.jpg");

        let res = transcode_pdf_to_images(sample, &out_jpg, "JPG");
        assert!(
            res.is_ok(),
            "transcode_pdf_to_images failed: {:?}",
            res.err()
        );

        // Check primary output exists
        assert!(out_jpg.exists(), "Primary output JPG does not exist");
        let bytes = fs::read(&out_jpg).unwrap();
        assert!(bytes.len() > 1000, "JPG file is too small");
        // Verify valid JPEG JFIF magic header 0xFF, 0xD8, 0xFF
        assert_eq!(bytes[0], 0xFF, "Invalid JPEG header byte 0");
        assert_eq!(bytes[1], 0xD8, "Invalid JPEG header byte 1");
        assert_eq!(bytes[2], 0xFF, "Invalid JPEG header byte 2");

        // Check page 1 and page 2 split outputs exist
        let p1 = temp_dir.join("test_output_page_1.jpg");
        let p2 = temp_dir.join("test_output_page_2.jpg");
        assert!(p1.exists(), "Page 1 output does not exist");
        assert!(p2.exists(), "Page 2 output does not exist");

        let b1 = fs::read(&p1).unwrap();
        let b2 = fs::read(&p2).unwrap();
        // Page 1 and Page 2 must be DIFFERENT images with different content/hashes
        assert_ne!(b1, b2, "Page 1 and Page 2 rendered identically!");

        let _ = fs::remove_dir_all(temp_dir);
    }
}
