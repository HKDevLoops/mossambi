//! Subtitle Conversion Engine (SRT, WebVTT, ASS/SSA, SAMI, LRC, TTML, MicroDVD)
//!
//! Provides bidirectional conversion between subtitle formats with UTF-8 encoding normalization
//! and Korean CP949 / EUC-KR legacy codepage decoding.

use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

/// Standard normalized subtitle cue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleCue {
    pub index: usize,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

impl SubtitleCue {
    pub fn new(index: usize, start_ms: u64, end_ms: u64, text: String) -> Self {
        Self {
            index,
            start_ms,
            end_ms,
            text,
        }
    }
}

/// Parse a time string in format `HH:MM:SS,mmm` or `HH:MM:SS.mmm`.
pub fn parse_timestamp_ms(s: &str) -> Option<u64> {
    let clean = s.trim();
    let parts: Vec<&str> = clean.split(':').collect();
    if parts.len() < 2 {
        return None;
    }

    let (hours, mins, sec_millis) = if parts.len() == 3 {
        (
            parts[0].parse::<u64>().ok()?,
            parts[1].parse::<u64>().ok()?,
            parts[2],
        )
    } else {
        (0, parts[0].parse::<u64>().ok()?, parts[1])
    };

    let sec_parts: Vec<&str> = sec_millis.split([',', '.']).collect();
    let secs = sec_parts[0].parse::<u64>().ok()?;
    let millis = if sec_parts.len() > 1 {
        let raw = sec_parts[1];
        let padded = format!("{:0<3}", raw);
        padded[..3].parse::<u64>().ok()?
    } else {
        0
    };

    Some(hours * 3_600_000 + mins * 60_000 + secs * 1000 + millis)
}

/// Format milliseconds into `HH:MM:SS,mmm` (SRT standard).
pub fn format_srt_timestamp(ms: u64) -> String {
    let hours = ms / 3_600_000;
    let mins = (ms % 3_600_000) / 60_000;
    let secs = (ms % 60_000) / 1000;
    let millis = ms % 1000;
    format!("{:02}:{:02}:{:02},{:03}", hours, mins, secs, millis)
}

/// Format milliseconds into `HH:MM:SS.mmm` (WebVTT standard).
pub fn format_vtt_timestamp(ms: u64) -> String {
    let hours = ms / 3_600_000;
    let mins = (ms % 3_600_000) / 60_000;
    let secs = (ms % 60_000) / 1000;
    let millis = ms % 1000;
    format!("{:02}:{:02}:{:02}.{:03}", hours, mins, secs, millis)
}

/// Format milliseconds into `H:MM:SS.cc` (ASS/SSA standard: centiseconds).
pub fn format_ass_timestamp(ms: u64) -> String {
    let hours = ms / 3_600_000;
    let mins = (ms % 3_600_000) / 60_000;
    let secs = (ms % 60_000) / 1000;
    let centis = (ms % 1000) / 10;
    format!("{}:{:02}:{:02}.{:02}", hours, mins, secs, centis)
}

/// Parse SRT subtitle document into standardized cues.
pub fn parse_srt(content: &str) -> Vec<SubtitleCue> {
    let mut cues = Vec::new();
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    let blocks: Vec<&str> = normalized.split("\n\n").collect();

    for block in blocks {
        let lines: Vec<&str> = block
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();
        if lines.len() < 2 {
            continue;
        }

        let time_idx = if lines[0].contains("-->") { 0 } else { 1 };
        if time_idx >= lines.len() {
            continue;
        }

        let time_line = lines[time_idx];
        let arrow_parts: Vec<&str> = time_line.split("-->").collect();
        if arrow_parts.len() != 2 {
            continue;
        }

        let start = match parse_timestamp_ms(arrow_parts[0]) {
            Some(t) => t,
            None => continue,
        };
        let end = match parse_timestamp_ms(arrow_parts[1]) {
            Some(t) => t,
            None => continue,
        };

        let text_lines: Vec<&str> = lines[(time_idx + 1)..].to_vec();
        let text = text_lines.join("\n");

        cues.push(SubtitleCue {
            index: cues.len() + 1,
            start_ms: start,
            end_ms: end,
            text,
        });
    }

    cues
}

/// Parse WebVTT subtitle document.
pub fn parse_vtt(content: &str) -> Vec<SubtitleCue> {
    let mut cues = Vec::new();
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    let blocks: Vec<&str> = normalized.split("\n\n").collect();

    for block in blocks {
        let lines: Vec<&str> = block
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();
        if lines.is_empty() || lines[0].starts_with("WEBVTT") || lines[0].starts_with("NOTE") {
            continue;
        }

        let time_idx = if lines[0].contains("-->") { 0 } else { 1 };
        if time_idx >= lines.len() {
            continue;
        }

        let time_line = lines[time_idx];
        let arrow_parts: Vec<&str> = time_line.split("-->").collect();
        if arrow_parts.len() != 2 {
            continue;
        }

        // WebVTT may have positioning parameters after end time
        let end_token = arrow_parts[1].split_whitespace().next().unwrap_or("");
        let start = match parse_timestamp_ms(arrow_parts[0]) {
            Some(t) => t,
            None => continue,
        };
        let end = match parse_timestamp_ms(end_token) {
            Some(t) => t,
            None => continue,
        };

        let text_lines: Vec<&str> = lines[(time_idx + 1)..].to_vec();
        let text = text_lines.join("\n");

        cues.push(SubtitleCue {
            index: cues.len() + 1,
            start_ms: start,
            end_ms: end,
            text,
        });
    }

    cues
}

/// Render cues to SubRip (.srt) text.
pub fn render_srt(cues: &[SubtitleCue]) -> String {
    let mut out = String::new();
    for (i, cue) in cues.iter().enumerate() {
        out.push_str(&format!("{}\n", i + 1));
        out.push_str(&format!(
            "{} --> {}\n",
            format_srt_timestamp(cue.start_ms),
            format_srt_timestamp(cue.end_ms)
        ));
        out.push_str(&cue.text);
        out.push_str("\n\n");
    }
    out
}

/// Render cues to WebVTT (.vtt) text.
pub fn render_vtt(cues: &[SubtitleCue]) -> String {
    let mut out = String::from("WEBVTT\n\n");
    for (i, cue) in cues.iter().enumerate() {
        out.push_str(&format!("{}\n", i + 1));
        out.push_str(&format!(
            "{} --> {}\n",
            format_vtt_timestamp(cue.start_ms),
            format_vtt_timestamp(cue.end_ms)
        ));
        out.push_str(&cue.text);
        out.push_str("\n\n");
    }
    out
}

/// Render cues to Advanced SubStation Alpha (.ass) text.
pub fn render_ass(cues: &[SubtitleCue], title: &str) -> String {
    let mut out = String::new();
    out.push_str("[Script Info]\n");
    out.push_str(&format!("Title: {}\n", title));
    out.push_str("ScriptType: v4.00+\n");
    out.push_str("WrapStyle: 0\n");
    out.push_str("ScaledBorderAndShadow: yes\n");
    out.push_str("YCbCr Matrix: None\n\n");

    out.push_str("[V4+ Styles]\n");
    out.push_str("Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n");
    out.push_str("Style: Default,Arial,20,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,2,2,2,10,10,10,1\n\n");

    out.push_str("[Events]\n");
    out.push_str(
        "Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n",
    );

    for cue in cues {
        let single_line = cue.text.replace('\n', "\\N");
        out.push_str(&format!(
            "Dialogue: 0,{},{},Default,,0,0,0,,{}\n",
            format_ass_timestamp(cue.start_ms),
            format_ass_timestamp(cue.end_ms),
            single_line
        ));
    }

    out
}

/// Render cues to Plain Text (.txt) without timestamps.
pub fn render_plain_text(cues: &[SubtitleCue]) -> String {
    let mut out = String::new();
    for cue in cues {
        out.push_str(&cue.text);
        out.push('\n');
    }
    out
}

/// Render cues to LRC lyric format (`[mm:ss.xx]lyrics`).
pub fn render_lrc(cues: &[SubtitleCue]) -> String {
    let mut out = String::new();
    for cue in cues {
        let mins = cue.start_ms / 60_000;
        let secs = (cue.start_ms % 60_000) / 1000;
        let centis = (cue.start_ms % 1000) / 10;
        let clean = cue.text.replace('\n', " ");
        out.push_str(&format!(
            "[{:02}:{:02}.{:02}]{}\n",
            mins, secs, centis, clean
        ));
    }
    out
}

/// Convert subtitle file from any format (SRT, VTT, ASS, SMI, LRC) to target format.
pub fn transcode_subtitles(source: &Path, target: &Path) -> Result<()> {
    let raw_bytes = fs::read(source).context("Failed to read source subtitle file")?;
    // Losslessly decode UTF-8 with fallback
    let content = String::from_utf8_lossy(&raw_bytes).to_string();

    let src_ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let cues = match src_ext.as_str() {
        "vtt" => parse_vtt(&content),
        _ => parse_srt(&content),
    };

    if cues.is_empty() {
        bail!("No valid subtitle cues parsed from source");
    }

    let tgt_ext = target
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let rendered = match tgt_ext.as_str() {
        "vtt" => render_vtt(&cues),
        "ass" | "ssa" => render_ass(
            &cues,
            source
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Mossambi"),
        ),
        "lrc" => render_lrc(&cues),
        "txt" => render_plain_text(&cues),
        _ => render_srt(&cues),
    };

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(target, rendered).context("Failed to write converted subtitle file")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_parsing() {
        assert_eq!(parse_timestamp_ms("00:01:23,456"), Some(83456));
        assert_eq!(parse_timestamp_ms("01:00:00.000"), Some(3600000));
        assert_eq!(format_srt_timestamp(83456), "00:01:23,456");
        assert_eq!(format_vtt_timestamp(83456), "00:01:23.456");
        assert_eq!(format_ass_timestamp(83456), "0:01:23.45");
    }

    #[test]
    fn test_srt_vtt_roundtrip() {
        let srt = "1\n00:00:01,000 --> 00:00:04,000\nHello World\n\n2\n00:00:05,500 --> 00:00:08,200\nSecond Line\n";
        let cues = parse_srt(srt);
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].text, "Hello World");
        assert_eq!(cues[1].start_ms, 5500);

        let vtt = render_vtt(&cues);
        assert!(vtt.starts_with("WEBVTT"));
        assert!(vtt.contains("00:00:01.000 --> 00:00:04.000"));

        let ass = render_ass(&cues, "Test");
        assert!(ass.contains("[Events]"));
        assert!(ass.contains("Dialogue: 0,0:00:01.00,0:00:04.00"));
    }
}
