//! Donut Toolbar Profiles and Appearance Configuration
//!
//! Models the 19 domain profiles, custom extension bindings, front/back slice configurations,
//! and donut appearance metrics matching the macOS/Filee visual architecture.

use serde::{Deserialize, Serialize};

/// Visual appearance controls for the Sweet Lime radial donut widget.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DonutAppearance {
    /// Overall diameter of the radial donut in pixels. Default 360.0. Range 280.0..480.0.
    #[serde(default = "default_donut_size")]
    pub size: f32,
    /// Center columella hole radius in pixels. Default 52.0. Range 30.0..80.0.
    #[serde(default = "default_donut_hole")]
    pub hole: f32,
    /// Container opacity / translucency alpha (0.20 to 1.00). Default 0.95.
    #[serde(default = "default_donut_opacity")]
    pub opacity: f32,
}

pub const DEFAULT_DONUT_DIAMETER: f32 = 360.0;
pub const MIN_DONUT_DIAMETER: f32 = 280.0;
pub const MAX_DONUT_DIAMETER: f32 = 480.0;

/// Clamps donut diameter to the supported range (280.0..=480.0).
pub fn clamp_donut_diameter(d: f32) -> f32 {
    d.clamp(MIN_DONUT_DIAMETER, MAX_DONUT_DIAMETER)
}

fn default_donut_size() -> f32 {
    DEFAULT_DONUT_DIAMETER
}

fn default_donut_hole() -> f32 {
    52.0
}

fn default_donut_opacity() -> f32 {
    0.95
}

impl Default for DonutAppearance {
    fn default() -> Self {
        Self {
            size: default_donut_size(),
            hole: default_donut_hole(),
            opacity: default_donut_opacity(),
        }
    }
}

/// A single preset slice on a donut face (e.g. "PNG", "JPG", "CROP", "PDF").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DonutSlicePreset {
    pub name: String,
    pub label: String,
    pub description: String,
    #[serde(default)]
    pub is_action: bool,
}

impl DonutSlicePreset {
    pub fn format(fmt: &str) -> Self {
        Self {
            name: fmt.to_string(),
            label: fmt.to_string(),
            description: format!("Convert to {}", fmt),
            is_action: false,
        }
    }

    pub fn action(name: &str, label: &str, desc: &str) -> Self {
        Self {
            name: name.to_string(),
            label: label.to_string(),
            description: desc.to_string(),
            is_action: true,
        }
    }
}

/// Profile specifying the active slices and extension rules for a domain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DonutProfile {
    pub id: String,
    pub name: String,
    pub is_mixed_selection: bool,
    pub extensions: Vec<String>,
    pub front_slices: Vec<DonutSlicePreset>,
    pub back_slices: Vec<DonutSlicePreset>,
    pub max_front_slices: usize,
    pub max_back_slices: usize,
}

impl DonutProfile {
    pub fn new(id: &str, name: &str, exts: &[&str], front: &[&str], back: &[&str]) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            is_mixed_selection: id == "mixed",
            extensions: exts.iter().map(|s| s.to_string()).collect(),
            front_slices: front.iter().map(|f| DonutSlicePreset::format(f)).collect(),
            back_slices: back
                .iter()
                .map(|b| DonutSlicePreset::action(b, b, b))
                .collect(),
            max_front_slices: 8,
            max_back_slices: 8,
        }
    }

    pub fn matches_extension(&self, ext: &str) -> bool {
        let clean = ext.trim_start_matches('.').to_ascii_lowercase();
        self.extensions
            .iter()
            .any(|e| e.trim_start_matches('.').to_ascii_lowercase() == clean)
    }
}

/// Default Power Tools assigned to the Back face (Face 2) of every profile.
pub fn default_back_tools() -> Vec<DonutSlicePreset> {
    vec![
        DonutSlicePreset::action("CROP", "Crop", "Interactive Crop Canvas"),
        DonutSlicePreset::action("EXAM", "Exam", "Exam Conformer Preset Tuning"),
        DonutSlicePreset::action("META", "Meta", "EXIF and Metadata Inspector"),
        DonutSlicePreset::action("RENAME", "Rename", "Batch Sequence Renamer"),
        DonutSlicePreset::action("AI OCR", "AI OCR", "DirectML Neural Scan & OCR"),
        DonutSlicePreset::action("PDF", "PDF Pack", "Vector PDF Document Creator"),
        DonutSlicePreset::action("ZIP", "Archive", "Ultra-Deflate ZIP Archive Pack"),
        DonutSlicePreset::action("EXPLORE", "Explore", "Locate Sibling Directory in Explorer"),
    ]
}

/// Complete Donut configuration encompassing all 19 domain profiles and appearance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DonutConfig {
    pub profiles: Vec<DonutProfile>,
    pub selected_profile_index: usize,
    pub appearance: DonutAppearance,
}

impl Default for DonutConfig {
    fn default() -> Self {
        let back = default_back_tools();

        let profiles = vec![
            DonutProfile {
                id: "images".to_string(),
                name: "Images".to_string(),
                is_mixed_selection: false,
                extensions: vec![
                    ".png", ".jpg", ".jpeg", ".jpe", ".jfif", ".webp", ".tiff", ".tif", ".bmp",
                    ".dib", ".gif", ".ico", ".avif", ".heic", ".heif", ".hif", ".jxl", ".jp2",
                    ".j2k", ".jpf", ".jpx", ".psd", ".psb", ".tga", ".icb", ".vda", ".vst", ".ppm",
                    ".pgm", ".pbm", ".pnm", ".icns", ".apng", ".mng", ".qoi", ".dds", ".pcx",
                    ".cur", ".xpm", ".xbm", ".wbmp", ".pam", ".pfm", ".sgi", ".rgb", ".rgba",
                    ".bw", ".ras", ".sun", ".miff", ".dcm", ".dicom", ".pcd", ".exr", ".hdr",
                    ".fits", ".fts", ".jxr", ".wdp", ".hdp", ".xcf", ".kra", ".livp", ".dng",
                    ".cr2", ".cr3", ".crw", ".nef", ".nrw", ".arw", ".srf", ".sr2", ".orf", ".rw2",
                    ".raf", ".pef", ".dcr", ".kdc", ".erf", ".mos", ".mrw", ".x3f", ".3fr", ".fff",
                    ".iiq", ".mef", ".rwl", ".srw", ".raw",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
                front_slices: vec![
                    DonutSlicePreset::format("PNG"),
                    DonutSlicePreset::format("JPG"),
                    DonutSlicePreset::format("WEBP"),
                    DonutSlicePreset::format("GIF"),
                    DonutSlicePreset::format("PDF"),
                    DonutSlicePreset::action(
                        "Trace SVG",
                        "Trace SVG",
                        "Vector contour tracing to SVG",
                    ),
                ],
                back_slices: back.clone(),
                max_front_slices: 6,
                max_back_slices: 8,
            },
            DonutProfile {
                id: "pdf".to_string(),
                name: "PDF".to_string(),
                is_mixed_selection: false,
                extensions: vec![".pdf", ".ai"].into_iter().map(String::from).collect(),
                front_slices: vec![
                    DonutSlicePreset::format("PNG"),
                    DonutSlicePreset::format("JPG"),
                    DonutSlicePreset::format("DOCX"),
                    DonutSlicePreset::format("TXT"),
                    DonutSlicePreset::format("PPTX"),
                    DonutSlicePreset::format("XLSX"),
                    DonutSlicePreset::action("Compress", "Compress", "Downscale PDF images"),
                ],
                back_slices: back.clone(),
                max_front_slices: 7,
                max_back_slices: 8,
            },
            DonutProfile {
                id: "documents".to_string(),
                name: "Documents".to_string(),
                is_mixed_selection: false,
                extensions: vec![
                    ".docx", ".doc", ".odt", ".rtf", ".wps", ".wpd", ".pages", ".oxps", ".xps",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
                front_slices: vec![
                    DonutSlicePreset::format("PDF"),
                    DonutSlicePreset::format("DOCX"),
                    DonutSlicePreset::format("ODT"),
                    DonutSlicePreset::format("TXT"),
                    DonutSlicePreset::format("HTML"),
                    DonutSlicePreset::format("MD"),
                    DonutSlicePreset::format("EPUB"),
                ],
                back_slices: back.clone(),
                max_front_slices: 7,
                max_back_slices: 8,
            },
            DonutProfile {
                id: "spreadsheets".to_string(),
                name: "Spreadsheets".to_string(),
                is_mixed_selection: false,
                extensions: vec![
                    ".xlsx", ".xls", ".ods", ".csv", ".tsv", ".numbers", ".xlsm", ".xlsb",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
                front_slices: vec![
                    DonutSlicePreset::format("CSV"),
                    DonutSlicePreset::format("XLSX"),
                    DonutSlicePreset::format("ODS"),
                    DonutSlicePreset::format("PDF"),
                    DonutSlicePreset::format("HTML"),
                    DonutSlicePreset::format("JSON"),
                ],
                back_slices: back.clone(),
                max_front_slices: 6,
                max_back_slices: 8,
            },
            DonutProfile {
                id: "presentations".to_string(),
                name: "Presentations".to_string(),
                is_mixed_selection: false,
                extensions: vec![".pptx", ".ppt", ".odp", ".key", ".ppsx", ".pps"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                front_slices: vec![
                    DonutSlicePreset::format("PDF"),
                    DonutSlicePreset::format("PPTX"),
                    DonutSlicePreset::format("ODP"),
                    DonutSlicePreset::format("PNG"),
                    DonutSlicePreset::format("JPG"),
                    DonutSlicePreset::format("HTML"),
                    DonutSlicePreset::format("TXT"),
                ],
                back_slices: back.clone(),
                max_front_slices: 7,
                max_back_slices: 8,
            },
            DonutProfile {
                id: "hwp".to_string(),
                name: "HWP / HWPX".to_string(),
                is_mixed_selection: false,
                extensions: vec![".hwp", ".hwpx"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                front_slices: vec![
                    DonutSlicePreset::format("HWPX"),
                    DonutSlicePreset::format("PDF"),
                    DonutSlicePreset::format("DOCX"),
                    DonutSlicePreset::format("MD"),
                    DonutSlicePreset::format("HTML"),
                    DonutSlicePreset::format("TXT"),
                    DonutSlicePreset::format("PNG"),
                ],
                back_slices: back.clone(),
                max_front_slices: 7,
                max_back_slices: 8,
            },
            DonutProfile {
                id: "text".to_string(),
                name: "Text & Markdown".to_string(),
                is_mixed_selection: false,
                extensions: vec![
                    ".md", ".txt", ".rst", ".adoc", ".tex", ".typ", ".org", ".wiki", ".ipynb",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
                front_slices: vec![
                    DonutSlicePreset::format("HTML"),
                    DonutSlicePreset::format("PDF"),
                    DonutSlicePreset::format("DOCX"),
                    DonutSlicePreset::format("TXT"),
                    DonutSlicePreset::format("ODT"),
                    DonutSlicePreset::format("RTF"),
                ],
                back_slices: back.clone(),
                max_front_slices: 6,
                max_back_slices: 8,
            },
            DonutProfile {
                id: "ebooks".to_string(),
                name: "E-books".to_string(),
                is_mixed_selection: false,
                extensions: vec![
                    ".epub", ".mobi", ".azw", ".azw3", ".fb2", ".cbz", ".cbr", ".lit", ".lrf",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
                front_slices: vec![
                    DonutSlicePreset::format("EPUB"),
                    DonutSlicePreset::format("PDF"),
                    DonutSlicePreset::format("DOCX"),
                    DonutSlicePreset::format("TXT"),
                    DonutSlicePreset::format("MOBI"),
                    DonutSlicePreset::format("AZW3"),
                ],
                back_slices: back.clone(),
                max_front_slices: 6,
                max_back_slices: 8,
            },
            DonutProfile {
                id: "video".to_string(),
                name: "Video".to_string(),
                is_mixed_selection: false,
                extensions: vec![
                    ".mp4", ".mov", ".mkv", ".webm", ".avi", ".wmv", ".flv", ".m4v", ".ts", ".mts",
                    ".m2ts", ".vob", ".ogv",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
                front_slices: vec![
                    DonutSlicePreset::format("MP4"),
                    DonutSlicePreset::format("WEBM"),
                    DonutSlicePreset::format("MKV"),
                    DonutSlicePreset::format("GIF"),
                    DonutSlicePreset::format("MP3"),
                    DonutSlicePreset::action(
                        "Extract Audio",
                        "Audio",
                        "Lossless audio stream extraction",
                    ),
                ],
                back_slices: back.clone(),
                max_front_slices: 6,
                max_back_slices: 8,
            },
            DonutProfile {
                id: "audio".to_string(),
                name: "Audio".to_string(),
                is_mixed_selection: false,
                extensions: vec![
                    ".mp3", ".flac", ".wav", ".m4a", ".aac", ".ogg", ".opus", ".wma", ".aiff",
                    ".alac", ".caf", ".wv",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
                front_slices: vec![
                    DonutSlicePreset::format("MP3"),
                    DonutSlicePreset::format("FLAC"),
                    DonutSlicePreset::format("WAV"),
                    DonutSlicePreset::format("M4A"),
                    DonutSlicePreset::format("OGG"),
                ],
                back_slices: back.clone(),
                max_front_slices: 5,
                max_back_slices: 8,
            },
            DonutProfile {
                id: "subtitles".to_string(),
                name: "Subtitles".to_string(),
                is_mixed_selection: false,
                extensions: vec![
                    ".srt", ".vtt", ".ass", ".ssa", ".smi", ".lrc", ".ttml", ".sub",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
                front_slices: vec![
                    DonutSlicePreset::format("SRT"),
                    DonutSlicePreset::format("VTT"),
                    DonutSlicePreset::format("ASS"),
                    DonutSlicePreset::format("TXT"),
                ],
                back_slices: back.clone(),
                max_front_slices: 4,
                max_back_slices: 8,
            },
            DonutProfile {
                id: "vector".to_string(),
                name: "Vector graphics".to_string(),
                is_mixed_selection: false,
                extensions: vec![".svg", ".svgz", ".eps", ".ps"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                front_slices: vec![
                    DonutSlicePreset::format("PDF"),
                    DonutSlicePreset::format("PNG"),
                    DonutSlicePreset::format("SVG"),
                    DonutSlicePreset::format("DXF"),
                    DonutSlicePreset::format("WEBP"),
                ],
                back_slices: back.clone(),
                max_front_slices: 5,
                max_back_slices: 8,
            },
            DonutProfile {
                id: "archives".to_string(),
                name: "Archives".to_string(),
                is_mixed_selection: false,
                extensions: vec![
                    ".zip", ".tar", ".gz", ".tgz", ".zst", ".7z", ".rar", ".bz2", ".xz", ".iso",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
                front_slices: vec![
                    DonutSlicePreset::action(
                        "Unpack",
                        "Unpack",
                        "Safe extraction preserving directory tree",
                    ),
                    DonutSlicePreset::format("ZIP"),
                    DonutSlicePreset::format("TAR.GZ"),
                    DonutSlicePreset::format("TAR.ZST"),
                ],
                back_slices: back.clone(),
                max_front_slices: 4,
                max_back_slices: 8,
            },
            DonutProfile {
                id: "cad".to_string(),
                name: "CAD drawings".to_string(),
                is_mixed_selection: false,
                extensions: vec![".dwg", ".dxf"].into_iter().map(String::from).collect(),
                front_slices: vec![
                    DonutSlicePreset::format("DXF"),
                    DonutSlicePreset::format("PDF"),
                    DonutSlicePreset::format("SVG"),
                    DonutSlicePreset::format("PNG"),
                    DonutSlicePreset::format("DWG"),
                ],
                back_slices: back.clone(),
                max_front_slices: 5,
                max_back_slices: 8,
            },
            DonutProfile {
                id: "fonts".to_string(),
                name: "Fonts".to_string(),
                is_mixed_selection: false,
                extensions: vec![".ttf", ".otf", ".woff", ".woff2", ".eot", ".fon"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                front_slices: vec![
                    DonutSlicePreset::format("TTF"),
                    DonutSlicePreset::format("OTF"),
                    DonutSlicePreset::format("WOFF"),
                    DonutSlicePreset::format("WOFF2"),
                ],
                back_slices: back.clone(),
                max_front_slices: 4,
                max_back_slices: 8,
            },
            DonutProfile {
                id: "data".to_string(),
                name: "Data".to_string(),
                is_mixed_selection: false,
                extensions: vec![
                    ".json", ".jsonl", ".yaml", ".yml", ".toml", ".xml", ".vcf", ".ics",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
                front_slices: vec![
                    DonutSlicePreset::format("JSON"),
                    DonutSlicePreset::format("YAML"),
                    DonutSlicePreset::format("TOML"),
                    DonutSlicePreset::format("XML"),
                    DonutSlicePreset::format("CSV"),
                ],
                back_slices: back.clone(),
                max_front_slices: 5,
                max_back_slices: 8,
            },
            DonutProfile {
                id: "models3d".to_string(),
                name: "3D models".to_string(),
                is_mixed_selection: false,
                extensions: vec![".glb", ".gltf", ".stl", ".obj", ".3mf", ".ply", ".fbx"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                front_slices: vec![
                    DonutSlicePreset::format("GLB"),
                    DonutSlicePreset::format("STL"),
                    DonutSlicePreset::format("OBJ"),
                    DonutSlicePreset::format("3MF"),
                    DonutSlicePreset::format("PNG"),
                ],
                back_slices: back.clone(),
                max_front_slices: 5,
                max_back_slices: 8,
            },
            DonutProfile {
                id: "geospatial".to_string(),
                name: "Geospatial".to_string(),
                is_mixed_selection: false,
                extensions: vec![".geojson", ".kml", ".kmz", ".gpx", ".shp"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                front_slices: vec![
                    DonutSlicePreset::format("GeoJSON"),
                    DonutSlicePreset::format("KML"),
                    DonutSlicePreset::format("KMZ"),
                    DonutSlicePreset::format("GPX"),
                    DonutSlicePreset::format("CSV"),
                ],
                back_slices: back.clone(),
                max_front_slices: 5,
                max_back_slices: 8,
            },
            DonutProfile {
                id: "mixed".to_string(),
                name: "Mixed files".to_string(),
                is_mixed_selection: true,
                extensions: Vec::new(),
                front_slices: vec![
                    DonutSlicePreset::action(
                        "Compress ZIP",
                        "ZIP",
                        "Bundle all files into one ZIP",
                    ),
                    DonutSlicePreset::action(
                        "PDF Portfolio",
                        "PDF",
                        "Combine into unified PDF document",
                    ),
                    DonutSlicePreset::action(
                        "Batch Transcode",
                        "Batch",
                        "Transcode each to default format",
                    ),
                    DonutSlicePreset::action(
                        "Batch Rename",
                        "Rename",
                        "Batch rename with numeric sequences",
                    ),
                    DonutSlicePreset::action(
                        "Sibling Copy",
                        "Copy",
                        "Duplicate with non-destructive suffix",
                    ),
                ],
                back_slices: back,
                max_front_slices: 5,
                max_back_slices: 8,
            },
        ];

        Self {
            profiles,
            selected_profile_index: 0,
            appearance: DonutAppearance::default(),
        }
    }
}

impl DonutConfig {
    pub fn find_profile_for_extension(&self, ext: &str) -> Option<&DonutProfile> {
        self.find_profile_for_extension_filtered(ext, None)
    }

    pub fn find_profile_for_extension_filtered(
        &self,
        ext: &str,
        fields: Option<&crate::core::fields_config::FieldsConfig>,
    ) -> Option<&DonutProfile> {
        let clean = ext.trim_start_matches('.').to_ascii_lowercase();
        self.profiles.iter().find(|p| {
            if p.is_mixed_selection || !p.matches_extension(&clean) {
                return false;
            }
            if let Some(f) = fields {
                let domain_key = match p.id.as_str() {
                    "cad" => "cad",
                    "fonts" => "fonts",
                    "models3d" => "models_3d",
                    "geospatial" => "maps",
                    "audio" => "pro_audio",
                    "video" => "video_broadcast",
                    _ => "",
                };
                if !domain_key.is_empty() && !f.is_field_enabled(domain_key) {
                    return false;
                }
            }
            true
        })
    }

    pub fn active_profile(&self) -> &DonutProfile {
        self.profiles
            .get(self.selected_profile_index)
            .unwrap_or(&self.profiles[0])
    }

    pub fn active_profile_mut(&mut self) -> &mut DonutProfile {
        if self.selected_profile_index >= self.profiles.len() {
            self.selected_profile_index = 0;
        }
        &mut self.profiles[self.selected_profile_index]
    }

    pub fn move_profile_up(&mut self, index: usize) {
        if index > 0 && index < self.profiles.len() {
            self.profiles.swap(index, index - 1);
            if self.selected_profile_index == index {
                self.selected_profile_index -= 1;
            } else if self.selected_profile_index == index - 1 {
                self.selected_profile_index += 1;
            }
        }
    }

    pub fn move_profile_down(&mut self, index: usize) {
        if index + 1 < self.profiles.len() {
            self.profiles.swap(index, index + 1);
            if self.selected_profile_index == index {
                self.selected_profile_index += 1;
            } else if self.selected_profile_index == index + 1 {
                self.selected_profile_index -= 1;
            }
        }
    }

    pub fn add_profile(&mut self, profile: DonutProfile) {
        self.profiles.push(profile);
        self.selected_profile_index = self.profiles.len() - 1;
    }

    pub fn delete_profile(&mut self, index: usize) {
        if self.profiles.len() > 1 && index < self.profiles.len() {
            self.profiles.remove(index);
            if self.selected_profile_index >= self.profiles.len() {
                self.selected_profile_index = self.profiles.len() - 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_donut_config_default_profiles() {
        let cfg = DonutConfig::default();
        assert_eq!(cfg.profiles.len(), 19);
        assert_eq!(cfg.profiles[0].name, "Images");
        assert!(cfg.profiles[0].matches_extension("png"));
        assert!(cfg.profiles[0].matches_extension(".webp"));
        assert!(cfg.profiles[0].matches_extension("JPEG"));
    }

    #[test]
    fn test_find_profile_for_extension() {
        let cfg = DonutConfig::default();
        let prof = cfg.find_profile_for_extension("epub").unwrap();
        assert_eq!(prof.id, "ebooks");

        let prof_cad = cfg.find_profile_for_extension("dwg").unwrap();
        assert_eq!(prof_cad.id, "cad");
    }

    #[test]
    fn test_appearance_defaults() {
        let app = DonutAppearance::default();
        assert_eq!(app.size, DEFAULT_DONUT_DIAMETER);
        assert_eq!(app.hole, 52.0);
        assert_eq!(app.opacity, 0.95);
    }
}
