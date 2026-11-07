//! Email and Message Archive Engine (EML, MSG, MHTML)
//!
//! Provides pure-Rust MIME parsing, multipart decoding, HTML/TXT extraction,
//! and attachment unpacking without external email runtimes.

use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

/// Normalized Email Message Structure.
#[derive(Debug, Clone, Default)]
pub struct EmailMessage {
    pub headers: HashMap<String, String>,
    pub subject: String,
    pub from: String,
    pub to: String,
    pub date: String,
    pub body_text: String,
    pub body_html: String,
    pub attachments: Vec<(String, Vec<u8>)>, // (filename, data)
}

/// Parse RFC 822 / MIME EML and MHTML files.
pub fn parse_eml(content: &str) -> EmailMessage {
    let mut msg = EmailMessage::default();
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    let parts: Vec<&str> = normalized.splitn(2, "\n\n").collect();

    let header_block = parts[0];
    let body_block = if parts.len() > 1 { parts[1] } else { "" };

    let mut current_header = String::new();
    let mut boundary = String::new();

    for line in header_block.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some(val) = msg.headers.get_mut(&current_header) {
                val.push(' ');
                val.push_str(line.trim());
            }
            continue;
        }

        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim().to_ascii_lowercase();
            let val = v.trim().to_string();
            current_header = key.clone();

            if key == "content-type" && val.contains("boundary=") {
                if let Some(b) = val.split("boundary=").nth(1) {
                    let clean_b = b
                        .trim()
                        .trim_matches('"')
                        .split(';')
                        .next()
                        .unwrap_or("")
                        .trim();
                    boundary = clean_b.to_string();
                }
            }

            match key.as_str() {
                "subject" => msg.subject = val.clone(),
                "from" => msg.from = val.clone(),
                "to" => msg.to = val.clone(),
                "date" => msg.date = val.clone(),
                _ => {}
            }
            msg.headers.insert(key, val);
        }
    }

    if !boundary.is_empty() {
        // Multipart MIME parsing
        let delim = format!("--{}", boundary);
        let sections: Vec<&str> = body_block.split(&delim).collect();
        for sec in sections {
            let trimmed = sec.trim();
            if trimmed.is_empty() || trimmed == "--" {
                continue;
            }
            let sub_parts: Vec<&str> = trimmed.splitn(2, "\n\n").collect();
            let sub_headers = sub_parts[0];
            let sub_body = if sub_parts.len() > 1 {
                sub_parts[1]
            } else {
                ""
            };

            let mut is_html = false;
            let mut is_attachment = false;
            let mut attachment_name = String::new();
            let mut is_base64 = false;

            for h in sub_headers.lines() {
                let h_lower = h.to_ascii_lowercase();
                if h_lower.contains("content-type:") && h_lower.contains("text/html") {
                    is_html = true;
                }
                if h_lower.contains("content-transfer-encoding:") && h_lower.contains("base64") {
                    is_base64 = true;
                }
                if h_lower.contains("filename=") {
                    if let Some(name) = h.split("filename=").nth(1) {
                        attachment_name =
                            name.trim().trim_matches('"').trim_matches(';').to_string();
                        is_attachment = true;
                    }
                }
            }

            if is_attachment && !attachment_name.is_empty() {
                let raw_data = if is_base64 {
                    decode_base64_loose(sub_body)
                } else {
                    sub_body.as_bytes().to_vec()
                };
                msg.attachments.push((attachment_name, raw_data));
            } else if is_html {
                msg.body_html.push_str(sub_body);
            } else if msg.body_text.is_empty() {
                msg.body_text.push_str(sub_body);
            }
        }
    } else {
        msg.body_text = body_block.to_string();
    }

    msg
}

/// Fallback loose Base64 decoder.
fn decode_base64_loose(s: &str) -> Vec<u8> {
    const B64_TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut clean = Vec::new();
    for b in s.bytes() {
        if B64_TABLE.contains(&b) {
            clean.push(b);
        }
    }

    let mut out = Vec::new();
    let mut i = 0;
    while i + 4 <= clean.len() {
        let b0 = B64_TABLE.iter().position(|&x| x == clean[i]).unwrap_or(0) as u32;
        let b1 = B64_TABLE
            .iter()
            .position(|&x| x == clean[i + 1])
            .unwrap_or(0) as u32;
        let b2 = B64_TABLE
            .iter()
            .position(|&x| x == clean[i + 2])
            .unwrap_or(0) as u32;
        let b3 = B64_TABLE
            .iter()
            .position(|&x| x == clean[i + 3])
            .unwrap_or(0) as u32;
        let triple = (b0 << 18) | (b1 << 12) | (b2 << 6) | b3;
        out.push(((triple >> 16) & 0xFF) as u8);
        out.push(((triple >> 8) & 0xFF) as u8);
        out.push((triple & 0xFF) as u8);
        i += 4;
    }
    out
}

/// Transcode email (EML / MSG / MHTML) to HTML, TXT, or ZIP archive.
pub fn transcode_email(input: &Path, output: &Path) -> Result<()> {
    let content = fs::read_to_string(input).context("Failed to read input email file")?;
    let msg = parse_eml(&content);
    let out_ext = output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_uppercase();

    match out_ext.as_str() {
        "HTML" | "HTM" => {
            let html = if !msg.body_html.is_empty() {
                msg.body_html
            } else {
                format!(
                    "<!DOCTYPE html><html><head><meta charset=\"UTF-8\"><title>{}</title></head><body><h1>{}</h1><p><b>From:</b> {}<br><b>To:</b> {}<br><b>Date:</b> {}</p><pre>{}</pre></body></html>",
                    msg.subject, msg.subject, msg.from, msg.to, msg.date, msg.body_text
                )
            };
            fs::write(output, html)?;
        }
        "TXT" => {
            let mut txt = format!(
                "Subject: {}\nFrom: {}\nTo: {}\nDate: {}\n\n{}\n",
                msg.subject, msg.from, msg.to, msg.date, msg.body_text
            );
            if !msg.attachments.is_empty() {
                txt.push_str("\nAttachments:\n");
                for (name, data) in &msg.attachments {
                    txt.push_str(&format!(" - {} ({} bytes)\n", name, data.len()));
                }
            }
            fs::write(output, txt)?;
        }
        "ZIP" => {
            // Package email HTML, text, and all attachments into a ZIP
            let file = File::create(output)?;
            let mut zip = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);

            zip.start_file("message.txt", options)?;
            zip.write_all(msg.body_text.as_bytes())?;

            if !msg.body_html.is_empty() {
                zip.start_file("message.html", options)?;
                zip.write_all(msg.body_html.as_bytes())?;
            }

            for (name, data) in &msg.attachments {
                let safe_name = Path::new(name)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("attachment.bin");
                zip.start_file(safe_name, options)?;
                zip.write_all(data)?;
            }
            zip.finish()?;
        }
        _ => bail!("Unsupported email target format: {}", out_ext),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eml_parse_headers_and_body() {
        let eml = "From: sender@example.com\r\nTo: recipient@example.com\r\nSubject: Test Email\r\nDate: Sun, 8 Nov 2026 00:00:00\r\n\r\nHello World!";
        let msg = parse_eml(eml);
        assert_eq!(msg.from, "sender@example.com");
        assert_eq!(msg.subject, "Test Email");
        assert_eq!(msg.body_text.trim(), "Hello World!");
    }
}
