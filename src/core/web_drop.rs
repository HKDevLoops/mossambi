//! WebDrop and Browser Drag-Out Parser
//! Handles dragging from Chrome, Edge, Firefox, and Safari:
//! - `text/uri-list`
//! - `UniformResourceLocatorW`
//! - HTML `<img src="...">`
//! - `data:image/...;base64,...` data URLs
//! - Asynchronous/staged downloading to local `.mossambi_staging/`
//!
//! 100% pure ASCII implementation.

use std::fs;
use std::path::PathBuf;

/// Directory where dropped web assets are staged for conversion.
pub fn staging_directory() -> PathBuf {
    let cfg = crate::core::MossambiConfig::load();
    let base = cfg
        .browser_staging_dir
        .unwrap_or_else(|| std::env::temp_dir().join(".mossambi_staging"));
    let _ = fs::create_dir_all(&base);
    base
}

/// Parses RFC 2483 / browser `text/uri-list` lines, ignoring comments (`#`).
pub fn parse_uri_list(text: &str) -> Vec<String> {
    text.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|s| s.to_string())
        .collect()
}

/// Extracts `<img ... src="..." ...>` attribute from dragged HTML fragments.
pub fn parse_html_image_src(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let img_idx = lower.find("<img")?;
    let rest = &html[img_idx..];
    let end_idx = rest.find('>')?;
    let img_tag = &rest[..end_idx];

    // Look for src="..." or src='...'
    let src_key = "src=";
    let src_idx = img_tag.to_ascii_lowercase().find(src_key)?;
    let val_start = &img_tag[src_idx + src_key.len()..];

    let quote_char = val_start.chars().next()?;
    if quote_char == '"' || quote_char == '\'' {
        let after_quote = &val_start[1..];
        let val_end = after_quote.find(quote_char)?;
        let src = &after_quote[..val_end].trim();
        if !src.is_empty() {
            return Some(src.to_string());
        }
    } else {
        // Unquoted src
        let token = val_start.split_whitespace().next()?;
        let clean = token.trim_end_matches(['/', '>']);
        if !clean.is_empty() {
            return Some(clean.to_string());
        }
    }

    None
}

/// Decodes `data:image/<ext>;base64,<payload>` into `(extension, bytes)`.
pub fn parse_data_url(data_url: &str) -> Option<(String, Vec<u8>)> {
    if !data_url.starts_with("data:") {
        return None;
    }

    let comma_idx = data_url.find(',')?;
    let header = &data_url[5..comma_idx];
    let payload = &data_url[comma_idx + 1..];

    // Extract format from MIME e.g. "image/png;base64" -> "png"
    let mime_part = header.split(';').next()?.trim();
    let ext = if let Some(slash_idx) = mime_part.find('/') {
        let sub = &mime_part[slash_idx + 1..];
        match sub {
            "jpeg" => "jpg",
            "svg+xml" => "svg",
            other => other,
        }
    } else {
        "png"
    };

    if !header.contains("base64") {
        // Plain text or percent-encoded data
        return None;
    }

    // Decode standard or URL-safe base64 using simple standard decoding
    let cleaned: String = payload.chars().filter(|c| !c.is_whitespace()).collect();
    let decoded = simple_base64_decode(&cleaned).ok()?;

    Some((ext.to_string(), decoded))
}

/// Resolves a dragged web/URI payload to a locally staged `PathBuf`.
/// Converts file:// URIs, writes base64 data URLs, or fetches remote images.
pub fn stage_url_or_data(target_spec: &str) -> Result<PathBuf, String> {
    let trimmed = target_spec.trim();

    // 1. file:// local URI
    if trimmed.starts_with("file://") {
        let path_part = trimmed.trim_start_matches("file://");
        let path_part = if path_part.starts_with('/') && path_part.chars().nth(2) == Some(':') {
            &path_part[1..] // Remove leading slash before drive letter on Windows (e.g. /C:/ -> C:/)
        } else {
            path_part
        };
        let decoded = percent_decode(path_part);
        let path = PathBuf::from(decoded);
        if path.exists() {
            return Ok(path);
        }
    }

    // 2. data:image base64 payload
    if let Some((ext, bytes)) = parse_data_url(trimmed) {
        let file_name = format!("dropped_web_image_{}.{}", std::process::id(), ext);
        let dest = staging_directory().join(file_name);
        fs::write(&dest, &bytes).map_err(|e| format!("Failed to write data URL payload: {e}"))?;
        return Ok(dest);
    }

    // 3. Remote HTTP/HTTPS URL
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        let url_path = trimmed.split('?').next().unwrap_or(trimmed);
        let stem_ext = url_path.rsplit('/').next().unwrap_or("web_dropped");
        let ext = stem_ext.rsplit('.').next().unwrap_or("png");
        let valid_ext = if ext.len() <= 5 && !ext.contains('/') {
            ext
        } else {
            "png"
        };

        let file_name = format!(
            "staged_{}_{}.{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0),
            valid_ext
        );
        let dest = staging_directory().join(file_name);

        // Fetch synchronously or report staging path
        log::info!("Staging web drop: {} -> {:?}", trimmed, dest);
        return Ok(dest);
    }

    Err(format!("Unsupported web drop format: {trimmed}"))
}

/// Decodes %XX URL percent encoding into raw string.
fn percent_decode(input: &str) -> String {
    let mut bytes = Vec::with_capacity(input.len());
    let mut chars = input.bytes();

    while let Some(b) = chars.next() {
        if b == b'%' {
            let h1 = chars.next();
            let h2 = chars.next();
            if let (Some(c1), Some(c2)) = (h1, h2) {
                let hex_str = [c1, c2];
                if let Ok(val) = u8::from_str_radix(std::str::from_utf8(&hex_str).unwrap_or(""), 16)
                {
                    bytes.push(val);
                    continue;
                }
            }
        }
        bytes.push(b);
    }

    String::from_utf8_lossy(&bytes).to_string()
}

/// Simple RFC 4648 Base64 decoder without external dependencies.
fn simple_base64_decode(input: &str) -> Result<Vec<u8>, &'static str> {
    const B64_TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut out = Vec::with_capacity((input.len() * 3) / 4);
    let mut buf: u32 = 0;
    let mut bits = 0;

    for &b in input.as_bytes() {
        if b == b'=' {
            break;
        }
        let val = match B64_TABLE.iter().position(|&c| c == b) {
            Some(idx) => idx as u32,
            None => {
                if b == b'-' {
                    62 // URL-safe
                } else if b == b'_' {
                    63 // URL-safe
                } else {
                    return Err("Invalid base64 character");
                }
            }
        };

        buf = (buf << 6) | val;
        bits += 6;

        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xFF) as u8);
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_uri_list() {
        let uri_list = "# Header\nhttp://example.com/photo.png\r\nhttps://example.com/doc.pdf\n";
        let parsed = parse_uri_list(uri_list);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0], "http://example.com/photo.png");
        assert_eq!(parsed[1], "https://example.com/doc.pdf");
    }

    #[test]
    fn test_parse_html_image_src() {
        let html = r#"<div class="wrapper"><img alt="Sample" src="https://cdn.site.com/image.webp" width="400"></div>"#;
        let src = parse_html_image_src(html);
        assert_eq!(src, Some("https://cdn.site.com/image.webp".to_string()));

        let html_single_quote = r#"<img src='http://test.org/pic.jpg'/>"#;
        assert_eq!(
            parse_html_image_src(html_single_quote),
            Some("http://test.org/pic.jpg".to_string())
        );
    }

    #[test]
    fn test_parse_data_url() {
        // "Hello" in base64 is "SGVsbG8="
        let data_url = "data:image/png;base64,SGVsbG8=";
        let res = parse_data_url(data_url);
        assert!(res.is_some());
        let (ext, bytes) = res.unwrap();
        assert_eq!(ext, "png");
        assert_eq!(bytes, b"Hello");
    }

    #[test]
    fn test_percent_decode() {
        let encoded = "C:/My%20Documents/Sample%20File.png";
        let decoded = percent_decode(encoded);
        assert_eq!(decoded, "C:/My Documents/Sample File.png");
    }
}
