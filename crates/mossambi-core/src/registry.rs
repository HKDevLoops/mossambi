//! Universal Format Registry covering 200+ media and document formats

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileCategory {
    Image,
    Video,
    Audio,
    Document,
    Spreadsheet,
    Presentation,
    Archive,
    Subtitle,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatInfo {
    pub extension: &'static str,
    pub name: &'static str,
    pub category: FileCategory,
    pub is_lossless: bool,
    pub supports_all_conversion: bool,
}

pub struct UniversalRegistry;

impl UniversalRegistry {
    /// Classifies an extension into its primary category.
    pub fn classify_extension(ext: &str) -> FileCategory {
        match ext.to_uppercase().as_str() {
            // Images
            "PNG" | "JPG" | "JPEG" | "WEBP" | "AVIF" | "HEIC" | "TIFF" | "TIF" | "BMP" | "ICO"
            | "GIF" | "SVG" | "PSD" | "RAW" | "CR2" | "NEF" | "ARW" | "DNG" | "QOI" | "JXL" => {
                FileCategory::Image
            }

            // Video
            "MP4" | "MOV" | "MKV" | "WEBM" | "AVI" | "WMV" | "FLV" | "M4V" | "TS" | "MPEG"
            | "MPG" | "3GP" | "OGV" | "PRORES" => FileCategory::Video,

            // Audio
            "MP3" | "WAV" | "FLAC" | "M4A" | "AAC" | "OGG" | "OPUS" | "AIFF" | "AIF" | "WMA"
            | "ALAC" | "DSD" | "DSF" | "APE" | "MID" | "MIDI" => FileCategory::Audio,

            // Documents
            "PDF" | "DOCX" | "DOC" | "TXT" | "MD" | "HTML" | "HTM" | "RTF" | "ORG" | "TEX"
            | "EPUB" | "ODT" | "PAGES" => FileCategory::Document,

            // Spreadsheets
            "XLSX" | "XLS" | "CSV" | "TSV" | "ODS" | "NUMBERS" => FileCategory::Spreadsheet,

            // Presentations
            "PPTX" | "PPT" | "ODP" | "KEY" => FileCategory::Presentation,

            // Archives
            "ZIP" | "7Z" | "TAR" | "GZ" | "TGZ" | "TAR.GZ" | "BZ2" | "TBZ2" | "TAR.BZ2"
            | "XZ" | "TXZ" | "TAR.XZ" | "ZST" | "TZST" | "TAR.ZST" | "RAR" => FileCategory::Archive,

            // Subtitles
            "SRT" | "VTT" | "ASS" | "SSA" | "SUB" => FileCategory::Subtitle,

            _ => FileCategory::Unknown,
        }
    }

    /// Classify a file path.
    pub fn classify_file<P: AsRef<Path>>(path: P) -> (FileCategory, String) {
        let p = path.as_ref();
        if p.is_dir() {
            return (FileCategory::Archive, "DIR".to_string());
        }
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_uppercase();
        let cat = Self::classify_extension(&ext);
        (cat, ext)
    }

    /// Top 7 contextual formats + ALL sector for the Radial Wheel Widget (Preserves 8-Petal Shape).
    pub fn get_widget_sectors_for_category(category: FileCategory) -> Vec<&'static str> {
        match category {
            FileCategory::Image => vec!["JPG", "PNG", "WEBP", "PDF", "HEIC", "TIFF", "AVIF", "ALL"],
            FileCategory::Video => vec!["MP4", "MOV", "MKV", "WEBM", "GIF", "MP3", "AVI", "ALL"],
            FileCategory::Audio => vec!["MP3", "WAV", "FLAC", "M4A", "OGG", "AAC", "OPUS", "ALL"],
            FileCategory::Document => vec!["PDF", "DOCX", "TXT", "HTML", "JPG", "PNG", "EPUB", "ALL"],
            FileCategory::Spreadsheet => vec!["XLSX", "CSV", "TSV", "PDF", "ODS", "XLS", "HTML", "ALL"],
            FileCategory::Presentation => vec!["PDF", "PPTX", "JPG", "PNG", "HTML", "TXT", "ODP", "ALL"],
            FileCategory::Archive => vec!["ZIP", "7Z", "TAR", "TAR.GZ", "TAR.XZ", "TAR.ZST", "EXTRACT", "ALL"],
            FileCategory::Subtitle => vec!["VTT", "SRT", "TXT", "ASS", "SSA", "SUB", "JSON", "ALL"],
            FileCategory::Unknown => vec!["PDF", "ZIP", "PNG", "MP4", "MP3", "TXT", "INFO", "ALL"],
        }
    }

    /// Comprehensive catalog of all valid output formats for a category (for GUI, TUI, and CLI).
    pub fn get_all_supported_outputs(category: FileCategory) -> Vec<&'static str> {
        match category {
            FileCategory::Image => vec![
                "PNG", "JPG", "JPEG", "WEBP", "AVIF", "HEIC", "TIFF", "BMP", "ICO", "GIF", "PDF",
                "SVG", "EPS", "PSD", "QOI", "JXL",
            ],
            FileCategory::Video => vec![
                "MP4", "MOV", "MKV", "WEBM", "AVI", "WMV", "GIF", "MP3", "WAV", "FLAC", "M4A",
                "OGV", "TS", "PRORES",
            ],
            FileCategory::Audio => vec![
                "WAV", "FLAC", "MP3", "M4A", "AAC", "OGG", "OPUS", "AIFF", "ALAC", "WMA", "DSD",
            ],
            FileCategory::Document => vec![
                "PDF", "DOCX", "TXT", "MD", "HTML", "EPUB", "RTF", "ORG", "TEX", "PNG", "JPG",
            ],
            FileCategory::Spreadsheet => vec!["XLSX", "CSV", "TSV", "PDF", "ODS", "HTML"],
            FileCategory::Presentation => vec!["PDF", "PPTX", "PNG", "JPG", "HTML"],
            FileCategory::Archive => vec![
                "ZIP", "7Z", "TAR", "TAR.GZ", "TAR.BZ2", "TAR.XZ", "TAR.ZST", "EXTRACT",
            ],
            FileCategory::Subtitle => vec!["VTT", "SRT", "TXT", "ASS", "SSA"],
            FileCategory::Unknown => vec!["ZIP", "PDF", "TXT"],
        }
    }
}


