//! Pure-Rust peek/preview engine for the Mossambi explorer.

use std::fs;
use std::io::Read;
use std::path::Path;

pub fn preview_kind(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "ico" | "tiff" | "tif" | "avif" => {
            "image"
        }
        "pdf" => "pdf",
        "doc" | "docx" | "odt" | "rtf" => "document",
        "xls" | "xlsx" | "ods" | "csv" | "tsv" => "spreadsheet",
        "ppt" | "pptx" | "odp" => "presentation",
        "zip" | "tar" | "gz" | "7z" | "xz" | "zst" | "bz2" | "rar" => "archive",
        "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "hpp" | "java" | "go" => "code",
        "txt" | "md" | "log" | "json" | "xml" | "yaml" | "yml" | "toml" | "ini" | "cfg" => "text",
        _ => "binary",
    }
}

pub fn extract_preview_text(path: &Path) -> anyhow::Result<String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_uppercase();
    match ext.as_str() {
        "PDF" => extract_pdf(path),
        "DOCX" | "ODT" => extract_docx(path),
        "XLSX" | "XLS" | "ODS" | "CSV" | "TSV" => extract_spreadsheet(path, &ext),
        "PPTX" | "ODP" => extract_pptx(path),
        "RTF" => extract_rtf(path),
        "PNG" | "JPG" | "JPEG" | "WEBP" | "GIF" | "BMP" | "ICO" | "TIFF" | "TIF" => {
            extract_image_meta(path)
        }
        "ZIP" | "TAR" | "GZ" | "XZ" | "ZST" | "BZ2" | "7Z" | "RAR" => extract_archive(path),
        _ => extract_text_lossy(path),
    }
}

fn extract_pdf(path: &Path) -> anyhow::Result<String> {
    let doc = lopdf::Document::load(path)?;
    let pages: Vec<u32> = doc.get_pages().keys().cloned().collect();
    let text = doc.extract_text(&pages)?;
    Ok(truncate(&text, 16384))
}

fn extract_docx(path: &Path) -> anyhow::Result<String> {
    let file = fs::File::open(path)?;
    let mut zip = zip::ZipArchive::new(file)?;
    let mut doc_xml = String::new();
    zip.by_name("word/document.xml")?
        .take(50 * 1024 * 1024)
        .read_to_string(&mut doc_xml)?;
    let p_re = regex::Regex::new(r"</w:p>").unwrap();
    let t_re = regex::Regex::new(r"<w:t[^>]*>(.*?)</w:t>").unwrap();
    let mut result = String::new();
    for para in p_re.split(&doc_xml) {
        let mut para_text = String::new();
        for cap in t_re.captures_iter(para) {
            if let Some(m) = cap.get(1) {
                para_text.push_str(
                    &m.as_str()
                        .replace("&amp;", "&")
                        .replace("&lt;", "<")
                        .replace("&gt;", ">")
                        .replace("&quot;", "\"")
                        .replace("&apos;", "'"),
                );
            }
        }
        if !para_text.is_empty() {
            result.push_str(&para_text);
            result.push_str("\n\n");
        }
    }
    Ok(truncate(&result, 16384))
}

use calamine::Reader;
fn extract_spreadsheet(path: &Path, ext: &str) -> anyhow::Result<String> {
    if ext == "CSV" || ext == "TSV" {
        let bytes = fs::read(path)?;
        let text = String::from_utf8_lossy(&bytes);
        return Ok(truncate(&text, 16384));
    }
    let mut wb = calamine::open_workbook_auto(path)?;
    let sheets = wb.sheet_names().to_vec();
    let mut out = String::new();
    if let Some(first) = sheets.first() {
        let range = wb.worksheet_range(first)?;
        for row in range.rows() {
            let rec: Vec<String> = row.iter().map(|c| c.to_string()).collect();
            out.push_str(&rec.join("\t"));
            out.push('\n');
        }
    }
    Ok(truncate(&out, 16384))
}

fn extract_pptx(path: &Path) -> anyhow::Result<String> {
    let file = fs::File::open(path)?;
    let mut zip = zip::ZipArchive::new(file)?;
    let mut slide_names: Vec<String> = (0..zip.len())
        .filter_map(|i| zip.by_index(i).ok().map(|e| e.name().to_string()))
        .filter(|n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))
        .collect();
    slide_names.sort();
    let t_re = regex::Regex::new(r"<a:t[^>]*>(.*?)</a:t>").unwrap();
    let mut out = String::new();
    for name in slide_names {
        if let Ok(entry) = zip.by_name(&name) {
            let mut xml = String::new();
            let _ = entry.take(10 * 1024 * 1024).read_to_string(&mut xml);
            for cap in t_re.captures_iter(&xml) {
                if let Some(m) = cap.get(1) {
                    out.push_str(m.as_str());
                    out.push(' ');
                }
            }
            out.push('\n');
        }
    }
    Ok(truncate(&out, 16384))
}

fn extract_rtf(path: &Path) -> anyhow::Result<String> {
    let bytes = fs::read(path)?;
    let text = String::from_utf8_lossy(&bytes);
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            while let Some(&n) = chars.peek() {
                if n.is_alphabetic() {
                    chars.next();
                } else {
                    break;
                }
            }
            while let Some(&n) = chars.peek() {
                if n.is_ascii_digit() || n == '-' {
                    chars.next();
                } else {
                    break;
                }
            }
            if let Some(&' ') = chars.peek() {
                chars.next();
            }
        } else if c == '{' || c == '}' {
        } else {
            out.push(c);
        }
    }
    Ok(truncate(&out, 16384))
}

fn extract_image_meta(path: &Path) -> anyhow::Result<String> {
    let img = image::open(path)?;
    let (w, h) = (img.width(), img.height());
    Ok(format!(
        "Image: {}x{} pixels\nFormat: {}\nFile size: {} bytes",
        w,
        h,
        path.extension().and_then(|e| e.to_str()).unwrap_or(""),
        fs::metadata(path).map(|m| m.len()).unwrap_or(0)
    ))
}

fn extract_archive(path: &Path) -> anyhow::Result<String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ext == "zip" {
        let file = fs::File::open(path)?;
        let mut zip = zip::ZipArchive::new(file)?;
        let mut out = String::new();
        for i in 0..zip.len() {
            if let Ok(e) = zip.by_index(i) {
                out.push_str(e.name());
                out.push('\n');
            }
        }
        return Ok(truncate(&out, 16384));
    }
    Ok(format!("Archive: {}", path.display()))
}

fn extract_text_lossy(path: &Path) -> anyhow::Result<String> {
    let bytes = fs::read(path)?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err(anyhow::anyhow!("file too large for text preview"));
    }
    Ok(truncate(&String::from_utf8_lossy(&bytes), 16384))
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}
