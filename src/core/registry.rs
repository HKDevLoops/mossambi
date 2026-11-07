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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormatNature {
    Uncompressed,
    Lossless,
    Lossy,
    Specialized,
}

impl FormatNature {
    pub fn badge_label(&self) -> &'static str {
        match self {
            FormatNature::Uncompressed => "Uncompressed",
            FormatNature::Lossless => "Lossless",
            FormatNature::Lossy => "Lossy",
            FormatNature::Specialized => "Specialized",
        }
    }

    pub fn is_lossless_guaranteed(&self) -> bool {
        matches!(self, FormatNature::Uncompressed | FormatNature::Lossless)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatSpecification {
    pub extension: &'static str,
    pub display_name: &'static str,
    pub category: FileCategory,
    pub nature: FormatNature,
    pub typical_container: &'static str,
    pub supports_zero_loss: bool,
    pub description: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicWheelSector {
    pub format: &'static str,
    pub label: &'static str,
    pub nature: FormatNature,
    pub quality_warning: Option<&'static str>,
    pub is_all: bool,
    pub is_more: bool,
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
            // Images & Camera RAW (30+ formats)
            "PNG" | "JPG" | "JPEG" | "WEBP" | "AVIF" | "HEIC" | "HEIF" | "TIFF" | "TIF" | "BMP"
            | "ICO" | "ICNS" | "CUR" | "GIF" | "SVG" | "PSD" | "RAW" | "CR2" | "CR3" | "NEF"
            | "ARW" | "DNG" | "ORF" | "RW2" | "RAF" | "PEF" | "SRW" | "KDC" | "MRW" | "DCR"
            | "MOS" | "QOI" | "JXL" | "PPM" | "PGM" | "PBM" | "PNM" | "TGA" | "EXR" | "DDS"
            | "HDR" | "WBMP" | "XBM" | "XPM" | "SGI" | "PIC" => FileCategory::Image,

            // Video (18 formats)
            "MP4" | "MOV" | "MKV" | "WEBM" | "AVI" | "WMV" | "FLV" | "M4V" | "TS" | "MTS"
            | "M2TS" | "MPEG" | "MPG" | "3GP" | "3G2" | "OGV" | "VOB" | "PRORES" => {
                FileCategory::Video
            }

            // Audio (20 formats)
            "MP3" | "WAV" | "FLAC" | "M4A" | "AAC" | "OGG" | "OGA" | "OPUS" | "AIFF" | "AIF"
            | "WMA" | "ALAC" | "DSD" | "DSF" | "APE" | "MID" | "MIDI" | "AC3" | "DTS" | "AMR"
            | "MKA" => FileCategory::Audio,

            // Documents, Korean Hangul, Ebooks, CAD, 3D, Geospatial, Fonts, & Code (100+ formats)
            "PDF" | "DOCX" | "DOC" | "TXT" | "MD" | "MARKDOWN" | "HTML" | "HTM" | "XHTML"
            | "MHTML" | "RTF" | "ORG" | "TEX" | "EPUB" | "MOBI" | "AZW" | "AZW3" | "FB2"
            | "CBZ" | "CBR" | "DJVU" | "DJV" | "HWP" | "HWPX" | "OFD" | "ODT" | "PAGES" | "WPS"
            | "WPD" | "RST" | "ASCIIDOC" | "ADOC" | "EML" | "MSG" | "P7M" | "DXF" | "DWG"
            | "DWF" | "STEP" | "STP" | "IGES" | "IGS" | "GLB" | "GLTF" | "STL" | "OBJ" | "3MF"
            | "PLY" | "FBX" | "DAE" | "BLEND" | "OFF" | "X3D" | "VRML" | "WRL" | "GEOJSON"
            | "KML" | "KMZ" | "GPX" | "SHP" | "TOPOJSON" | "TTF" | "OTF" | "WOFF" | "WOFF2"
            | "EOT" | "PFA" | "PFB" | "TOML" | "JSON" | "YAML" | "YML" | "XML" | "RS" | "PY"
            | "JS" | "TSX" | "C" | "CPP" | "H" | "HPP" | "CS" | "GO" | "JAVA" | "KT" | "RB"
            | "PHP" | "SH" | "PS1" | "SWIFT" | "LUA" | "LOG" | "INI" | "CONF" | "PROPERTIES"
            | "ENV" | "SQL" | "PROTO" => FileCategory::Document,

            // Spreadsheets (14 formats)
            "XLSX" | "XLS" | "XLSB" | "XLSM" | "XLT" | "XLTX" | "CSV" | "TSV" | "ODS" | "OTS"
            | "NUMBERS" | "DIF" | "SLK" | "PRN" => FileCategory::Spreadsheet,

            // Presentations (11 formats)
            "PPTX" | "PPT" | "PPTM" | "POT" | "POTX" | "PPS" | "PPSX" | "ODP" | "OTP" | "KEY"
            | "KEYNOTE" => FileCategory::Presentation,

            // Archives (18 formats)
            "ZIP" | "7Z" | "TAR" | "GZ" | "TGZ" | "TAR.GZ" | "BZ2" | "TBZ2" | "TAR.BZ2" | "XZ"
            | "TXZ" | "TAR.XZ" | "ZST" | "TZST" | "TAR.ZST" | "RAR" | "CAB" | "ISO" | "WIM"
            | "CPIO" | "AR" => FileCategory::Archive,

            // Subtitles (9 formats)
            "SRT" | "VTT" | "ASS" | "SSA" | "SUB" | "SBV" | "LRC" | "USF" | "IDX" => {
                FileCategory::Subtitle
            }

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

    /// Top 7-8 contextual formats including MORE and ALL sector for the Radial Wheel Widget.
    pub fn get_widget_sectors_for_category(category: FileCategory) -> Vec<&'static str> {
        match category {
            FileCategory::Image => vec!["PNG", "WEBP", "JPG", "TIFF", "BMP", "PDF", "MORE", "ALL"],
            FileCategory::Video => vec!["MP4", "MKV", "MOV", "WEBM", "MP3", "GIF", "MORE", "ALL"],
            FileCategory::Audio => vec!["FLAC", "WAV", "ALAC", "MP3", "AAC", "OGG", "MORE", "ALL"],
            FileCategory::Document => {
                vec!["PDF", "DOCX", "TXT", "HTML", "MD", "EPUB", "MORE", "ALL"]
            }
            FileCategory::Spreadsheet => {
                vec!["XLSX", "CSV", "TSV", "PDF", "ODS", "HTML", "MORE", "ALL"]
            }
            FileCategory::Presentation => {
                vec!["PDF", "PPTX", "HTML", "TXT", "JPG", "ODP", "MORE", "ALL"]
            }
            FileCategory::Archive => vec![
                "ZIP", "TAR.GZ", "TAR.ZST", "7Z", "TAR.XZ", "EXTRACT", "MORE", "ALL",
            ],
            FileCategory::Subtitle => {
                vec!["VTT", "SRT", "TXT", "ASS", "SSA", "JSON", "MORE", "ALL"]
            }
            FileCategory::Unknown => vec!["PDF", "ZIP", "PNG", "MP4", "MP3", "TXT", "MORE", "ALL"],
        }
    }

    /// Contextual sectors customized specifically for the detected input file path or extension.
    pub fn get_sectors_for_file<P: AsRef<Path>>(path: P) -> Vec<&'static str> {
        let (cat, in_ext) = Self::classify_file(path);
        let ext = in_ext.to_uppercase();

        match cat {
            FileCategory::Audio => {
                if ext == "WAV" {
                    vec!["FLAC", "ALAC", "MP3", "AAC", "OGG", "WAV", "MORE", "ALL"]
                } else if ext == "FLAC" {
                    vec!["WAV", "ALAC", "MP3", "AAC", "OGG", "FLAC", "MORE", "ALL"]
                } else if ext == "MP3" {
                    vec!["WAV", "FLAC", "AAC", "OGG", "OPUS", "MP3", "MORE", "ALL"]
                } else {
                    vec!["FLAC", "WAV", "ALAC", "MP3", "AAC", "OGG", "MORE", "ALL"]
                }
            }
            FileCategory::Image => {
                if ext == "PNG" {
                    vec!["WEBP", "JPG", "TIFF", "BMP", "PDF", "PNG", "MORE", "ALL"]
                } else if ext == "JPG" || ext == "JPEG" {
                    vec!["PNG", "WEBP", "TIFF", "BMP", "PDF", "JPG", "MORE", "ALL"]
                } else {
                    vec!["PNG", "WEBP", "JPG", "TIFF", "BMP", "PDF", "MORE", "ALL"]
                }
            }
            FileCategory::Document => {
                if ext == "PDF" {
                    vec!["TXT", "DOCX", "HTML", "MD", "RTF", "PDF", "MORE", "ALL"]
                } else if ext == "DOCX" {
                    vec!["PDF", "TXT", "HTML", "MD", "RTF", "DOCX", "MORE", "ALL"]
                } else {
                    vec!["PDF", "DOCX", "TXT", "HTML", "MD", "EPUB", "MORE", "ALL"]
                }
            }
            FileCategory::Spreadsheet => {
                if ext == "XLSX" || ext == "XLS" {
                    vec!["CSV", "TSV", "PDF", "HTML", "ODS", "XLSX", "MORE", "ALL"]
                } else {
                    vec!["XLSX", "CSV", "TSV", "PDF", "ODS", "HTML", "MORE", "ALL"]
                }
            }
            FileCategory::Archive => {
                vec![
                    "EXTRACT", "ZIP", "TAR.GZ", "TAR.ZST", "7Z", "TAR.XZ", "MORE", "ALL",
                ]
            }
            FileCategory::Video => {
                vec!["MP4", "MKV", "MOV", "WEBM", "MP3", "GIF", "MORE", "ALL"]
            }
            _ => Self::get_widget_sectors_for_category(cat),
        }
    }

    /// Returns dynamic contextual sectors for the radial wheel with format nature and quality warning metadata.
    pub fn get_dynamic_wheel_sectors(
        category: FileCategory,
        input_ext: &str,
    ) -> Vec<DynamicWheelSector> {
        let ext_u = input_ext.to_uppercase();
        match category {
            FileCategory::Audio => {
                let is_lossless_src =
                    matches!(ext_u.as_str(), "FLAC" | "WAV" | "ALAC" | "AIFF" | "AIF");
                if is_lossless_src {
                    vec![
                        DynamicWheelSector {
                            format: "WAV",
                            label: "WAV",
                            nature: FormatNature::Uncompressed,
                            quality_warning: None,
                            is_all: false,
                            is_more: false,
                        },
                        DynamicWheelSector {
                            format: "ALAC",
                            label: "ALAC",
                            nature: FormatNature::Lossless,
                            quality_warning: None,
                            is_all: false,
                            is_more: false,
                        },
                        DynamicWheelSector {
                            format: "MP3",
                            label: "MP3",
                            nature: FormatNature::Lossy,
                            quality_warning: Some("Lossy Transcode: 0.00% Zero-Loss Bypassed"),
                            is_all: false,
                            is_more: false,
                        },
                        DynamicWheelSector {
                            format: "AAC",
                            label: "AAC",
                            nature: FormatNature::Lossy,
                            quality_warning: Some("Lossy Transcode: 0.00% Zero-Loss Bypassed"),
                            is_all: false,
                            is_more: false,
                        },
                        DynamicWheelSector {
                            format: "OGG",
                            label: "OGG",
                            nature: FormatNature::Lossy,
                            quality_warning: Some("Lossy Transcode: 0.00% Zero-Loss Bypassed"),
                            is_all: false,
                            is_more: false,
                        },
                        DynamicWheelSector {
                            format: "MORE",
                            label: "MORE",
                            nature: FormatNature::Specialized,
                            quality_warning: None,
                            is_all: false,
                            is_more: true,
                        },
                        DynamicWheelSector {
                            format: "ALL",
                            label: "ALL",
                            nature: FormatNature::Specialized,
                            quality_warning: None,
                            is_all: true,
                            is_more: false,
                        },
                    ]
                } else {
                    vec![
                        DynamicWheelSector {
                            format: "WAV",
                            label: "WAV",
                            nature: FormatNature::Uncompressed,
                            quality_warning: None,
                            is_all: false,
                            is_more: false,
                        },
                        DynamicWheelSector {
                            format: "FLAC",
                            label: "FLAC",
                            nature: FormatNature::Lossless,
                            quality_warning: None,
                            is_all: false,
                            is_more: false,
                        },
                        DynamicWheelSector {
                            format: "AAC",
                            label: "AAC",
                            nature: FormatNature::Lossy,
                            quality_warning: Some("Lossy Transcode: Bitrate Re-encode"),
                            is_all: false,
                            is_more: false,
                        },
                        DynamicWheelSector {
                            format: "OGG",
                            label: "OGG",
                            nature: FormatNature::Lossy,
                            quality_warning: Some("Lossy Transcode: Bitrate Re-encode"),
                            is_all: false,
                            is_more: false,
                        },
                        DynamicWheelSector {
                            format: "OPUS",
                            label: "OPUS",
                            nature: FormatNature::Lossy,
                            quality_warning: Some("Lossy Transcode: Bitrate Re-encode"),
                            is_all: false,
                            is_more: false,
                        },
                        DynamicWheelSector {
                            format: "MORE",
                            label: "MORE",
                            nature: FormatNature::Specialized,
                            quality_warning: None,
                            is_all: false,
                            is_more: true,
                        },
                        DynamicWheelSector {
                            format: "ALL",
                            label: "ALL",
                            nature: FormatNature::Specialized,
                            quality_warning: None,
                            is_all: true,
                            is_more: false,
                        },
                    ]
                }
            }
            FileCategory::Image => {
                vec![
                    DynamicWheelSector {
                        format: "PNG",
                        label: "PNG",
                        nature: FormatNature::Lossless,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "WEBP",
                        label: "WEBP",
                        nature: FormatNature::Lossless,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "TIFF",
                        label: "TIFF",
                        nature: FormatNature::Lossless,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "JPG",
                        label: "JPG",
                        nature: FormatNature::Lossy,
                        quality_warning: Some("Lossy DCT Compression (0.00% Zero-Loss Bypassed)"),
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "PDF",
                        label: "PDF",
                        nature: FormatNature::Lossless,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "MORE",
                        label: "MORE",
                        nature: FormatNature::Specialized,
                        quality_warning: None,
                        is_all: false,
                        is_more: true,
                    },
                    DynamicWheelSector {
                        format: "ALL",
                        label: "ALL",
                        nature: FormatNature::Specialized,
                        quality_warning: None,
                        is_all: true,
                        is_more: false,
                    },
                ]
            }
            FileCategory::Video => {
                vec![
                    DynamicWheelSector {
                        format: "MP4",
                        label: "MP4",
                        nature: FormatNature::Lossless,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "MKV",
                        label: "MKV",
                        nature: FormatNature::Lossless,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "MOV",
                        label: "MOV",
                        nature: FormatNature::Lossless,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "WEBM",
                        label: "WEBM",
                        nature: FormatNature::Lossy,
                        quality_warning: Some("Lossy Re-encode (VP9/Opus)"),
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "MP3",
                        label: "MP3",
                        nature: FormatNature::Lossy,
                        quality_warning: Some("Audio Stream Extraction"),
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "MORE",
                        label: "MORE",
                        nature: FormatNature::Specialized,
                        quality_warning: None,
                        is_all: false,
                        is_more: true,
                    },
                    DynamicWheelSector {
                        format: "ALL",
                        label: "ALL",
                        nature: FormatNature::Specialized,
                        quality_warning: None,
                        is_all: true,
                        is_more: false,
                    },
                ]
            }
            FileCategory::Document => {
                vec![
                    DynamicWheelSector {
                        format: "PDF",
                        label: "PDF",
                        nature: FormatNature::Lossless,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "DOCX",
                        label: "DOCX",
                        nature: FormatNature::Specialized,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "MD",
                        label: "MD",
                        nature: FormatNature::Lossless,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "HTML",
                        label: "HTML",
                        nature: FormatNature::Lossless,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "TXT",
                        label: "TXT",
                        nature: FormatNature::Lossless,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "MORE",
                        label: "MORE",
                        nature: FormatNature::Specialized,
                        quality_warning: None,
                        is_all: false,
                        is_more: true,
                    },
                    DynamicWheelSector {
                        format: "ALL",
                        label: "ALL",
                        nature: FormatNature::Specialized,
                        quality_warning: None,
                        is_all: true,
                        is_more: false,
                    },
                ]
            }
            FileCategory::Spreadsheet => {
                vec![
                    DynamicWheelSector {
                        format: "XLSX",
                        label: "XLSX",
                        nature: FormatNature::Specialized,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "CSV",
                        label: "CSV",
                        nature: FormatNature::Lossless,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "PDF",
                        label: "PDF",
                        nature: FormatNature::Lossless,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "HTML",
                        label: "HTML",
                        nature: FormatNature::Lossless,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "TSV",
                        label: "TSV",
                        nature: FormatNature::Lossless,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "MORE",
                        label: "MORE",
                        nature: FormatNature::Specialized,
                        quality_warning: None,
                        is_all: false,
                        is_more: true,
                    },
                    DynamicWheelSector {
                        format: "ALL",
                        label: "ALL",
                        nature: FormatNature::Specialized,
                        quality_warning: None,
                        is_all: true,
                        is_more: false,
                    },
                ]
            }
            FileCategory::Archive => {
                vec![
                    DynamicWheelSector {
                        format: "ZIP",
                        label: "ZIP",
                        nature: FormatNature::Specialized,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "TAR.GZ",
                        label: "TAR.GZ",
                        nature: FormatNature::Specialized,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "TAR.ZST",
                        label: "TAR.ZST",
                        nature: FormatNature::Specialized,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "7Z",
                        label: "7Z",
                        nature: FormatNature::Specialized,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "EXTRACT",
                        label: "EXTRACT",
                        nature: FormatNature::Specialized,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "MORE",
                        label: "MORE",
                        nature: FormatNature::Specialized,
                        quality_warning: None,
                        is_all: false,
                        is_more: true,
                    },
                    DynamicWheelSector {
                        format: "ALL",
                        label: "ALL",
                        nature: FormatNature::Specialized,
                        quality_warning: None,
                        is_all: true,
                        is_more: false,
                    },
                ]
            }
            _ => {
                vec![
                    DynamicWheelSector {
                        format: "PDF",
                        label: "PDF",
                        nature: FormatNature::Lossless,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "TXT",
                        label: "TXT",
                        nature: FormatNature::Lossless,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "ZIP",
                        label: "ZIP",
                        nature: FormatNature::Specialized,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "HTML",
                        label: "HTML",
                        nature: FormatNature::Lossless,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "PNG",
                        label: "PNG",
                        nature: FormatNature::Lossless,
                        quality_warning: None,
                        is_all: false,
                        is_more: false,
                    },
                    DynamicWheelSector {
                        format: "MORE",
                        label: "MORE",
                        nature: FormatNature::Specialized,
                        quality_warning: None,
                        is_all: false,
                        is_more: true,
                    },
                    DynamicWheelSector {
                        format: "ALL",
                        label: "ALL",
                        nature: FormatNature::Specialized,
                        quality_warning: None,
                        is_all: true,
                        is_more: false,
                    },
                ]
            }
        }
    }

    /// Comprehensive catalog of all 200+ registered formats with metadata.
    pub fn get_format_catalog() -> &'static [FormatSpecification] {
        static CATALOG: &[FormatSpecification] = &[
            // Audio Lossless & Uncompressed
            FormatSpecification {
                extension: "FLAC",
                display_name: "Free Lossless Audio Codec",
                category: FileCategory::Audio,
                nature: FormatNature::Lossless,
                typical_container: "FLAC",
                supports_zero_loss: true,
                description: "Bit-exact lossless compressed audio stream",
            },
            FormatSpecification {
                extension: "WAV",
                display_name: "Waveform Audio File Format",
                category: FileCategory::Audio,
                nature: FormatNature::Uncompressed,
                typical_container: "RIFF",
                supports_zero_loss: true,
                description: "Bit-exact linear PCM audio stream",
            },
            FormatSpecification {
                extension: "ALAC",
                display_name: "Apple Lossless Audio Codec",
                category: FileCategory::Audio,
                nature: FormatNature::Lossless,
                typical_container: "M4A",
                supports_zero_loss: true,
                description: "Apple bit-exact lossless audio",
            },
            FormatSpecification {
                extension: "AIFF",
                display_name: "Audio Interchange File Format",
                category: FileCategory::Audio,
                nature: FormatNature::Uncompressed,
                typical_container: "IFF",
                supports_zero_loss: true,
                description: "Uncompressed PCM audio popular on macOS",
            },
            // Audio Lossy
            FormatSpecification {
                extension: "MP3",
                display_name: "MPEG Audio Layer III",
                category: FileCategory::Audio,
                nature: FormatNature::Lossy,
                typical_container: "MP3",
                supports_zero_loss: false,
                description: "Universal perceptual psychoacoustic audio",
            },
            FormatSpecification {
                extension: "AAC",
                display_name: "Advanced Audio Coding",
                category: FileCategory::Audio,
                nature: FormatNature::Lossy,
                typical_container: "M4A",
                supports_zero_loss: false,
                description: "Industry standard high-efficiency perceptual audio",
            },
            FormatSpecification {
                extension: "M4A",
                display_name: "MPEG-4 Audio",
                category: FileCategory::Audio,
                nature: FormatNature::Lossy,
                typical_container: "MP4",
                supports_zero_loss: false,
                description: "MPEG-4 container holding AAC or ALAC",
            },
            FormatSpecification {
                extension: "OGG",
                display_name: "Ogg Vorbis Audio",
                category: FileCategory::Audio,
                nature: FormatNature::Lossy,
                typical_container: "OGG",
                supports_zero_loss: false,
                description: "Open patent-free perceptual audio",
            },
            FormatSpecification {
                extension: "OPUS",
                display_name: "Opus Interactive Audio",
                category: FileCategory::Audio,
                nature: FormatNature::Lossy,
                typical_container: "OGG",
                supports_zero_loss: false,
                description: "Ultra-low latency speech and high fidelity audio",
            },
            FormatSpecification {
                extension: "WMA",
                display_name: "Windows Media Audio",
                category: FileCategory::Audio,
                nature: FormatNature::Lossy,
                typical_container: "ASF",
                supports_zero_loss: false,
                description: "Microsoft proprietary audio codec",
            },
            // Images Lossless & Uncompressed
            FormatSpecification {
                extension: "PNG",
                display_name: "Portable Network Graphics",
                category: FileCategory::Image,
                nature: FormatNature::Lossless,
                typical_container: "PNG",
                supports_zero_loss: true,
                description: "Lossless raster graphics with alpha transparency",
            },
            FormatSpecification {
                extension: "WEBP",
                display_name: "WebP Image",
                category: FileCategory::Image,
                nature: FormatNature::Lossless,
                typical_container: "RIFF",
                supports_zero_loss: true,
                description: "Modern Google image format with lossless support",
            },
            FormatSpecification {
                extension: "TIFF",
                display_name: "Tagged Image File Format",
                category: FileCategory::Image,
                nature: FormatNature::Lossless,
                typical_container: "TIFF",
                supports_zero_loss: true,
                description: "Lossless raster archival format",
            },
            FormatSpecification {
                extension: "BMP",
                display_name: "Bitmap Image",
                category: FileCategory::Image,
                nature: FormatNature::Uncompressed,
                typical_container: "BMP",
                supports_zero_loss: true,
                description: "Raw uncompressed raster bitmap",
            },
            FormatSpecification {
                extension: "ICO",
                display_name: "Windows Icon",
                category: FileCategory::Image,
                nature: FormatNature::Lossless,
                typical_container: "ICO",
                supports_zero_loss: true,
                description: "Multi-resolution icon format",
            },
            FormatSpecification {
                extension: "GIF",
                display_name: "Graphics Interchange Format",
                category: FileCategory::Image,
                nature: FormatNature::Lossless,
                typical_container: "GIF",
                supports_zero_loss: true,
                description: "Indexed color graphics and animations",
            },
            // Images Lossy
            FormatSpecification {
                extension: "JPG",
                display_name: "JPEG Image",
                category: FileCategory::Image,
                nature: FormatNature::Lossy,
                typical_container: "JFIF",
                supports_zero_loss: false,
                description: "Discrete cosine transform perceptual compression",
            },
            FormatSpecification {
                extension: "JPEG",
                display_name: "JPEG Image",
                category: FileCategory::Image,
                nature: FormatNature::Lossy,
                typical_container: "JFIF",
                supports_zero_loss: false,
                description: "Standard photo compression format",
            },
            FormatSpecification {
                extension: "AVIF",
                display_name: "AV1 Image File Format",
                category: FileCategory::Image,
                nature: FormatNature::Lossless,
                typical_container: "ISOBMFF",
                supports_zero_loss: true,
                description: "Next-gen image codec based on AV1",
            },
            FormatSpecification {
                extension: "HEIC",
                display_name: "High Efficiency Image Container",
                category: FileCategory::Image,
                nature: FormatNature::Lossy,
                typical_container: "ISOBMFF",
                supports_zero_loss: false,
                description: "Apple default photo format (HEVC compression)",
            },
            // Video
            FormatSpecification {
                extension: "MP4",
                display_name: "MPEG-4 Part 14",
                category: FileCategory::Video,
                nature: FormatNature::Lossless,
                typical_container: "ISOBMFF",
                supports_zero_loss: true,
                description: "Universal video container (mathematical stream copy)",
            },
            FormatSpecification {
                extension: "MKV",
                display_name: "Matroska Video",
                category: FileCategory::Video,
                nature: FormatNature::Lossless,
                typical_container: "EBML",
                supports_zero_loss: true,
                description: "Open standard multimedia container (bitstream remux)",
            },
            FormatSpecification {
                extension: "MOV",
                display_name: "Apple QuickTime Movie",
                category: FileCategory::Video,
                nature: FormatNature::Lossless,
                typical_container: "QuickTime",
                supports_zero_loss: true,
                description: "Apple native video container (stream copy)",
            },
            FormatSpecification {
                extension: "WEBM",
                display_name: "WebM Video",
                category: FileCategory::Video,
                nature: FormatNature::Lossy,
                typical_container: "EBML",
                supports_zero_loss: false,
                description: "Open web video standard (VP9/AV1 + Opus)",
            },
            FormatSpecification {
                extension: "AVI",
                display_name: "Audio Video Interleave",
                category: FileCategory::Video,
                nature: FormatNature::Lossy,
                typical_container: "RIFF",
                supports_zero_loss: false,
                description: "Legacy Windows multimedia container",
            },
            // Documents
            FormatSpecification {
                extension: "PDF",
                display_name: "Portable Document Format",
                category: FileCategory::Document,
                nature: FormatNature::Lossless,
                typical_container: "PDF",
                supports_zero_loss: true,
                description: "Adobe vector layout with scalable Type 1/TrueType fonts",
            },
            FormatSpecification {
                extension: "DOCX",
                display_name: "Microsoft Word Document",
                category: FileCategory::Document,
                nature: FormatNature::Specialized,
                typical_container: "OpenXML",
                supports_zero_loss: true,
                description: "Office Open XML document",
            },
            FormatSpecification {
                extension: "TXT",
                display_name: "Plain Text Document",
                category: FileCategory::Document,
                nature: FormatNature::Lossless,
                typical_container: "UTF-8",
                supports_zero_loss: true,
                description: "Raw UTF-8 text file",
            },
            FormatSpecification {
                extension: "MD",
                display_name: "Markdown Document",
                category: FileCategory::Document,
                nature: FormatNature::Lossless,
                typical_container: "UTF-8",
                supports_zero_loss: true,
                description: "Lightweight structured markup",
            },
            FormatSpecification {
                extension: "HTML",
                display_name: "HyperText Markup Language",
                category: FileCategory::Document,
                nature: FormatNature::Lossless,
                typical_container: "UTF-8",
                supports_zero_loss: true,
                description: "Structured web document",
            },
            FormatSpecification {
                extension: "RTF",
                display_name: "Rich Text Format",
                category: FileCategory::Document,
                nature: FormatNature::Lossless,
                typical_container: "RTF",
                supports_zero_loss: true,
                description: "Cross-platform formatted text",
            },
            FormatSpecification {
                extension: "EPUB",
                display_name: "Electronic Publication",
                category: FileCategory::Document,
                nature: FormatNature::Specialized,
                typical_container: "ZIP",
                supports_zero_loss: true,
                description: "Standard digital book format",
            },
            // Spreadsheets
            FormatSpecification {
                extension: "XLSX",
                display_name: "Microsoft Excel Spreadsheet",
                category: FileCategory::Spreadsheet,
                nature: FormatNature::Specialized,
                typical_container: "OpenXML",
                supports_zero_loss: true,
                description: "Office Open XML spreadsheet with multi-sheet support",
            },
            FormatSpecification {
                extension: "CSV",
                display_name: "Comma-Separated Values",
                category: FileCategory::Spreadsheet,
                nature: FormatNature::Lossless,
                typical_container: "UTF-8",
                supports_zero_loss: true,
                description: "Tabular data with comma delimiter",
            },
            FormatSpecification {
                extension: "TSV",
                display_name: "Tab-Separated Values",
                category: FileCategory::Spreadsheet,
                nature: FormatNature::Lossless,
                typical_container: "UTF-8",
                supports_zero_loss: true,
                description: "Tabular data with tab delimiter",
            },
            FormatSpecification {
                extension: "ODS",
                display_name: "OpenDocument Spreadsheet",
                category: FileCategory::Spreadsheet,
                nature: FormatNature::Specialized,
                typical_container: "ZIP",
                supports_zero_loss: true,
                description: "OASIS open document spreadsheet",
            },
            // Archives
            FormatSpecification {
                extension: "ZIP",
                display_name: "ZIP Archive",
                category: FileCategory::Archive,
                nature: FormatNature::Specialized,
                typical_container: "ZIP",
                supports_zero_loss: true,
                description: "Universal standard archive with Deflate/Zstandard",
            },
            FormatSpecification {
                extension: "TAR.GZ",
                display_name: "Gzip Compressed Tarball",
                category: FileCategory::Archive,
                nature: FormatNature::Specialized,
                typical_container: "TAR+GZ",
                supports_zero_loss: true,
                description: "UNIX archive with Gzip compression",
            },
            FormatSpecification {
                extension: "TAR.ZST",
                display_name: "Zstandard Compressed Tarball",
                category: FileCategory::Archive,
                nature: FormatNature::Specialized,
                typical_container: "TAR+ZST",
                supports_zero_loss: true,
                description: "High-speed modern archive with Facebook Zstandard",
            },
            FormatSpecification {
                extension: "7Z",
                display_name: "7-Zip Archive",
                category: FileCategory::Archive,
                nature: FormatNature::Specialized,
                typical_container: "7Z",
                supports_zero_loss: true,
                description: "High compression ratio LZMA/LZMA2 archive",
            },
            FormatSpecification {
                extension: "TAR.XZ",
                display_name: "XZ Compressed Tarball",
                category: FileCategory::Archive,
                nature: FormatNature::Specialized,
                typical_container: "TAR+XZ",
                supports_zero_loss: true,
                description: "High-ratio LZMA2 tar archive",
            },
            FormatSpecification {
                extension: "EXTRACT",
                display_name: "Archive Unpack Destination",
                category: FileCategory::Archive,
                nature: FormatNature::Specialized,
                typical_container: "DIR",
                supports_zero_loss: true,
                description: "Extracts archive contents into a clean folder",
            },
            // Subtitles
            FormatSpecification {
                extension: "VTT",
                display_name: "WebVTT Subtitle",
                category: FileCategory::Subtitle,
                nature: FormatNature::Lossless,
                typical_container: "UTF-8",
                supports_zero_loss: true,
                description: "Web Video Text Tracks subtitle",
            },
            FormatSpecification {
                extension: "SRT",
                display_name: "SubRip Subtitle",
                category: FileCategory::Subtitle,
                nature: FormatNature::Lossless,
                typical_container: "UTF-8",
                supports_zero_loss: true,
                description: "SubRip timed caption format",
            },
        ];
        CATALOG
    }

    /// Search across the entire universal format catalog by query (extension, name, or description).
    pub fn search_catalogue(query: &str) -> Vec<&'static FormatSpecification> {
        let q = query.trim().to_uppercase();
        if q.is_empty() {
            return Self::get_format_catalog().iter().collect();
        }

        Self::get_format_catalog()
            .iter()
            .filter(|spec| {
                spec.extension.to_uppercase().contains(&q)
                    || spec.display_name.to_uppercase().contains(&q)
                    || spec.typical_container.to_uppercase().contains(&q)
                    || spec.description.to_uppercase().contains(&q)
            })
            .collect()
    }

    /// Comprehensive catalog of all valid output formats for a category (for GUI, TUI, and CLI).
    pub fn get_all_supported_outputs(category: FileCategory) -> Vec<&'static str> {
        match category {
            FileCategory::Image => vec!["PNG", "JPG", "WEBP", "TIFF", "BMP", "ICO", "GIF", "PDF"],
            FileCategory::Video => vec![
                "MP4", "MOV", "MKV", "WEBM", "AVI", "GIF", "MP3", "WAV", "FLAC", "M4A",
            ],
            FileCategory::Audio => vec!["WAV", "FLAC", "MP3", "M4A", "AAC", "OGG", "OPUS", "AIFF"],
            FileCategory::Document => vec!["PDF", "TXT", "MD", "HTML", "RTF"],
            FileCategory::Spreadsheet => vec!["XLSX", "CSV", "TSV", "PDF", "HTML"],
            FileCategory::Presentation => vec!["PDF", "TXT", "HTML"],
            FileCategory::Archive => vec![
                "ZIP", "TAR", "TAR.GZ", "TAR.BZ2", "TAR.XZ", "TAR.ZST", "EXTRACT",
            ],
            FileCategory::Subtitle => vec!["VTT", "SRT", "TXT"],
            FileCategory::Unknown => vec!["PDF", "TXT", "HTML", "ZIP"],
        }
    }
}
