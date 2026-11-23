//! Donut Toolbar Profiles and Appearance Configuration
//!
//! Models the 20 domain profiles, custom extension bindings, front/back slice configurations,
//! and donut appearance metrics matching the macOS/Filee visual architecture.
//!
//! Pure 7-bit ASCII.

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

    /// Factory method mapping preset IDs to typed slice definitions.
    pub fn from_id(id: &str) -> Self {
        match id {
            "to-png" => Self::format("PNG"),
            "to-jpg" => Self::format("JPG"),
            "to-webp" => Self::format("WEBP"),
            "to-gif" => Self::format("GIF"),
            "to-pdf" => Self::format("PDF"),
            "merge-pdf" => Self::action("Merge PDF", "Merge PDF", "Merge selected files into one PDF"),
            "trace-svg" => Self::action("Trace SVG", "Trace SVG", "Vector contour tracing to SVG"),
            "tool-crop" => Self::action("Crop", "Crop", "Interactive crop canvas"),
            "rotate" => Self::action("Rotate", "Rotate", "Rotate image in 90 degree increments"),
            "remove-background" => Self::action("Cut out", "Cut out", "Remove background with AI or color mask"),
            "merge-images" => Self::action("Merge images", "Merge images", "Combine pictures horizontally or vertically"),
            "compress-image" => Self::action("Compress image", "Compress image", "Lossy or lossless size reduction"),
            "half-size" => Self::action("Half size", "Half size", "Scale image to 50% width and height"),
            "tool-image-edit" => Self::action("Edit pictures", "Edit pictures", "Adjust brightness, contrast and tone"),
            "zip-all" => Self::action("One ZIP", "One ZIP", "Compress items into one archive"),
            "to-docx" => Self::format("DOCX"),
            "to-xlsx" => Self::format("XLSX"),
            "pdf-slides" => Self::action("PDF Slides", "Slides", "Extract PDF pages to slide presentations"),
            "to-txt" => Self::format("TXT"),
            "to-hwpx" => Self::format("HWPX"),
            "to-hwp" => Self::format("HWP"),
            "pdf-edit" => Self::action("PDF Edit", "Edit", "Lossless page rearrangement and annotations"),
            "pdf-split" => Self::action("PDF Split", "Split", "Separate PDF into individual page files"),
            "compress-pdf" => Self::action("Compress PDF", "Compress PDF", "Downscale PDF images and compress streams"),
            "remove-metadata" => Self::action("Remove Meta", "Strip Meta", "Strip EXIF and tracking metadata"),
            "to-tiff" => Self::format("TIFF"),
            "to-html" => Self::format("HTML"),
            "translate" => Self::action("Translate", "Translate", "Document language translation"),
            "to-md" => Self::format("MD"),
            "pdf-long-image" => Self::action("Long Image", "Long Image", "Stack pages into one vertical strip"),
            "to-csv" => Self::format("CSV"),
            "tool-sheet-studio" => Self::action("Sheet Studio", "Studio", "Advanced tabular dataset editor"),
            "to-vcf" => Self::format("VCF"),
            "to-ics" => Self::format("ICS"),
            "to-pptx" => Self::format("PPTX"),
            "to-odp" => Self::format("ODP"),
            "to-odt" => Self::format("ODT"),
            "to-epub" => Self::format("EPUB"),
            "to-azw3" => Self::format("AZW3"),
            "to-kepub" => Self::format("KEPUB"),
            "to-mp4" => Self::format("MP4"),
            "mp4-720" => Self::action("MP4 720p", "720p", "Export H.264 video at 720p resolution"),
            "to-mov" => Self::format("MOV"),
            "to-mp3" => Self::format("MP3"),
            "caption-video" => Self::action("Captions", "Captions", "Transcribe audio to subtitles"),
            "tool-clip" => Self::action("Clip", "Clip", "Trim start and end timestamps"),
            "crop-video" => Self::action("Crop Video", "Crop Video", "Crop video aspect ratio"),
            "compress-video" => Self::action("Compress Video", "Compress Video", "CRF-based video size reduction"),
            "merge-videos" => Self::action("Merge Videos", "Merge Videos", "Concatenate video streams losslessly"),
            "mp4-av1" => Self::action("MP4 AV1", "AV1", "Encode ultra-efficient AV1 video"),
            "extract-frames-png" => Self::action("Extract Frames", "Frames", "Extract all video frames to PNGs"),
            "mp3-128" => Self::action("MP3 128k", "128k", "Encode MP3 at 128 kbps"),
            "to-m4a" => Self::format("M4A"),
            "to-wav" => Self::format("WAV"),
            "compress-audio" => Self::action("Compress Audio", "Compress Audio", "Audio bitrate optimization"),
            "merge-audio" => Self::action("Merge Audio", "Merge Audio", "Join sound tracks into one"),
            "to-alac" => Self::format("ALAC"),
            "to-srt" => Self::format("SRT"),
            "to-vtt" => Self::format("VTT"),
            "to-svg" => Self::format("SVG"),
            "to-ico" => Self::format("ICO"),
            "to-icns" => Self::format("ICNS"),
            "to-dxf" => Self::format("DXF"),
            "to-dwg" => Self::format("DWG"),
            "extract" => Self::action("Extract", "Extract", "Extract archive contents"),
            "to-zip" => Self::format("ZIP"),
            "to-7z" => Self::format("7Z"),
            "to-iso" => Self::format("ISO"),
            "to-woff2" => Self::format("WOFF2"),
            "to-woff" => Self::format("WOFF"),
            "to-ttf" => Self::format("TTF"),
            "to-otf" => Self::format("OTF"),
            "to-json" => Self::format("JSON"),
            "to-yaml" => Self::format("YAML"),
            "to-xml" => Self::format("XML"),
            "to-ods" => Self::format("ODS"),
            "to-glb" => Self::format("GLB"),
            "to-stl" => Self::format("STL"),
            "to-obj" => Self::format("OBJ"),
            "to-3mf" => Self::format("3MF"),
            "to-geojson" => Self::format("GeoJSON"),
            "to-shp" => Self::format("SHP"),
            "to-kml" => Self::format("KML"),
            "to-gpx" => Self::format("GPX"),
            _ => {
                if let Some(fmt) = id.strip_prefix("to-") {
                    Self::format(&fmt.to_ascii_uppercase())
                } else {
                    Self::action(id, id, id)
                }
            }
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
            front_slices: front.iter().map(|f| DonutSlicePreset::from_id(f)).collect(),
            back_slices: back.iter().map(|b| DonutSlicePreset::from_id(b)).collect(),
            max_front_slices: 12,
            max_back_slices: 12,
        }
    }

    pub fn matches_extension(&self, ext: &str) -> bool {
        let clean = ext.trim_start_matches('.').to_ascii_lowercase();
        self.extensions
            .iter()
            .any(|e| e.trim_start_matches('.').to_ascii_lowercase() == clean)
    }

    pub fn contains_extension(&self, ext: &str) -> bool {
        self.matches_extension(ext)
    }

    /// Formats the front and back item count badge string (e.g. "7-8" or "4").
    pub fn count_badge(&self) -> String {
        if self.back_slices.is_empty() {
            format!("{}", self.front_slices.len())
        } else {
            format!("{}-{}", self.front_slices.len(), self.back_slices.len())
        }
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

/// Complete Donut configuration encompassing all 20 domain profiles and appearance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DonutConfig {
    pub profiles: Vec<DonutProfile>,
    pub selected_profile_index: usize,
    pub appearance: DonutAppearance,
}

impl Default for DonutConfig {
    fn default() -> Self {
        let profiles = vec![
            // 1. Images (7 Front, 8 Back)
            DonutProfile::new(
                "images",
                "Images",
                &[
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
                ],
                &["to-png", "to-jpg", "to-webp", "to-gif", "to-pdf", "merge-pdf", "trace-svg"],
                &["tool-crop", "rotate", "remove-background", "merge-images", "compress-image", "half-size", "tool-image-edit", "zip-all"],
            ),

            // 2. PDF (8 Front, 8 Back)
            DonutProfile::new(
                "pdf",
                "PDF",
                &[".pdf", ".ai"],
                &["to-png", "to-jpg", "to-docx", "to-xlsx", "pdf-slides", "to-txt", "to-hwpx", "to-hwp"],
                &["pdf-edit", "pdf-split", "merge-pdf", "compress-pdf", "remove-metadata", "to-tiff", "to-html", "zip-all"],
            ),

            // 3. Documents (Office) (7 Front, 4 Back)
            DonutProfile::new(
                "office",
                "Documents",
                &[
                    ".docx", ".doc", ".dotx", ".dotm", ".docm", ".odt", ".rtf", ".wps", ".wpd",
                    ".pages", ".oxps", ".xps",
                ],
                &["to-pdf", "to-hwpx", "to-hwp", "to-docx", "to-txt", "to-png", "merge-pdf"],
                &["translate", "to-md", "pdf-long-image", "zip-all"],
            ),

            // 4. Spreadsheets (6 Front, 5 Back)
            DonutProfile::new(
                "spreadsheets",
                "Spreadsheets",
                &[
                    ".xlsx", ".xls", ".xltx", ".xlsm", ".xlsb", ".ods", ".ots", ".csv", ".tsv",
                    ".numbers",
                ],
                &["to-pdf", "to-xlsx", "to-csv", "to-hwpx", "to-hwp", "merge-pdf"],
                &["tool-sheet-studio", "to-docx", "to-vcf", "to-ics", "zip-all"],
            ),

            // 5. Presentations (7 Front, 4 Back)
            DonutProfile::new(
                "presentations",
                "Presentations",
                &[".pptx", ".ppt", ".potx", ".potm", ".pptm", ".ppsx", ".pps", ".odp", ".otp", ".key"],
                &["to-pdf", "to-png", "to-jpg", "to-pptx", "to-hwpx", "to-hwp", "merge-pdf"],
                &["to-odp", "pdf-long-image", "to-docx", "zip-all"],
            ),

            // 6. HWP / HWPX (7 Front, 4 Back)
            DonutProfile::new(
                "hwp",
                "HWP / HWPX",
                &[".hwp", ".hwpx"],
                &["to-pdf", "to-docx", "to-hwpx", "to-hwp", "to-png", "to-txt", "merge-pdf"],
                &["to-html", "to-odt", "pdf-long-image", "zip-all"],
            ),

            // 7. Text & Markdown (6 Front, 5 Back)
            DonutProfile::new(
                "text",
                "Text & Markdown",
                &[
                    ".md", ".markdown", ".txt", ".rst", ".adoc", ".asciidoc", ".tex", ".latex",
                    ".typ", ".typst", ".org", ".wiki", ".ipynb", ".log", ".bat", ".cmd", ".sh",
                    ".bash", ".ps1", ".py", ".rb", ".js", ".ts", ".rs", ".c", ".cpp", ".h",
                    ".hpp", ".cs", ".go", ".java", ".lua", ".sql", ".html", ".htm", ".css",
                ],
                &["to-pdf", "to-hwpx", "to-hwp", "to-docx", "to-html", "to-txt"],
                &["translate", "to-md", "to-odt", "merge-pdf", "zip-all"],
            ),

            // 8. E-books (6 Front, 3 Back)
            DonutProfile::new(
                "ebooks",
                "E-books",
                &[
                    ".epub", ".mobi", ".azw", ".azw3", ".kepub", ".fb2", ".cbz", ".cbr", ".cb7",
                    ".cbt", ".cbc", ".lit", ".lrf", ".chm", ".pdb", ".pml", ".rb", ".snb", ".tcr",
                    ".oeb", ".azw4",
                ],
                &["to-pdf", "to-epub", "to-docx", "to-txt", "to-azw3", "to-kepub"],
                &["translate", "to-md", "zip-all"],
            ),

            // 9. Video (6 Front, 8 Back)
            DonutProfile::new(
                "video",
                "Video",
                &[
                    ".mp4", ".mov", ".mkv", ".webm", ".avi", ".wmv", ".flv", ".m4v", ".ts",
                    ".mts", ".m2ts", ".vob", ".ogv", ".3gp", ".3g2", ".f4v", ".asf", ".rm",
                    ".rmvb", ".dv", ".mxf",
                ],
                &["to-mp4", "mp4-720", "to-mov", "to-gif", "to-mp3", "caption-video"],
                &["tool-clip", "crop-video", "compress-video", "merge-videos", "remove-metadata", "mp4-av1", "extract-frames-png", "zip-all"],
            ),

            // 10. Audio (5 Front, 5 Back)
            DonutProfile::new(
                "audio",
                "Audio",
                &[
                    ".mp3", ".flac", ".wav", ".m4a", ".aac", ".ogg", ".opus", ".wma", ".aiff",
                    ".aif", ".alac", ".caf", ".wv", ".ape", ".dsd", ".dsf", ".dff", ".dts",
                    ".ac3", ".eac3", ".tta",
                ],
                &["to-mp3", "mp3-128", "to-m4a", "to-wav", "caption-video"],
                &["compress-audio", "merge-audio", "remove-metadata", "to-alac", "zip-all"],
            ),

            // 11. Subtitles (2 Front, 1 Back)
            DonutProfile::new(
                "subtitles",
                "Subtitles",
                &[
                    ".srt", ".vtt", ".ass", ".ssa", ".smi", ".sami", ".lrc", ".ttml", ".sub",
                    ".scc", ".krc",
                ],
                &["to-srt", "to-vtt"],
                &["zip-all"],
            ),

            // 12. Vector Graphics (5 Front, 5 Back)
            DonutProfile::new(
                "vector",
                "Vector graphics",
                &[".svg", ".svgz", ".eps", ".ps"],
                &["to-png", "to-pdf", "to-svg", "to-jpg", "to-webp"],
                &["to-tiff", "to-ico", "to-icns", "to-dxf", "zip-all"],
            ),

            // 13. Archives (4 Front, 0 Back)
            DonutProfile::new(
                "archives",
                "Archives",
                &[
                    ".zip", ".tar", ".gz", ".tgz", ".zst", ".tzst", ".7z", ".rar", ".bz2",
                    ".tbz2", ".xz", ".txz", ".iso", ".cab", ".dmg", ".lz", ".wbpz",
                ],
                &["extract", "zip-all", "to-zip", "to-7z"],
                &[],
            ),

            // 14. Folders (3 Front, 0 Back)
            DonutProfile::new(
                "folders",
                "Folders",
                &[],
                &["to-iso", "to-zip", "zip-all"],
                &[],
            ),

            // 15. CAD Drawings (5 Front, 3 Back)
            DonutProfile::new(
                "cad",
                "CAD drawings",
                &[".dwg", ".dxf", ".plt"],
                &["to-pdf", "to-png", "to-svg", "to-dxf", "to-dwg"],
                &["to-jpg", "merge-pdf", "zip-all"],
            ),

            // 16. Fonts (4 Front, 1 Back)
            DonutProfile::new(
                "fonts",
                "Fonts",
                &[".ttf", ".otf", ".woff", ".woff2", ".eot", ".fon", ".fnt"],
                &["to-woff2", "to-woff", "to-ttf", "to-otf"],
                &["zip-all"],
            ),

            // 17. Data (5 Front, 4 Back)
            DonutProfile::new(
                "data",
                "Data",
                &[
                    ".json", ".jsonl", ".yaml", ".yml", ".toml", ".xml", ".vcf", ".ics",
                    ".plist",
                ],
                &["to-json", "to-yaml", "to-xml", "to-csv", "to-xlsx"],
                &["to-ods", "to-html", "to-md", "zip-all"],
            ),

            // 18. 3D Models (5 Front, 1 Back)
            DonutProfile::new(
                "models",
                "3D models",
                &[".glb", ".gltf", ".stl", ".obj", ".3mf", ".ply", ".fbx"],
                &["to-glb", "to-stl", "to-obj", "to-3mf", "to-png"],
                &["zip-all"],
            ),

            // 19. Geospatial (5 Front, 1 Back)
            DonutProfile::new(
                "geo",
                "Geospatial",
                &[".geojson", ".kml", ".kmz", ".gpx", ".shp"],
                &["to-geojson", "to-shp", "to-kml", "to-csv", "to-gpx"],
                &["zip-all"],
            ),

            // 20. Mixed Files (5 Front, 2 Back)
            DonutProfile::new(
                "mixed",
                "Mixed files",
                &[],
                &["to-pdf", "merge-pdf", "zip-all", "to-png", "to-txt"],
                &["to-jpg", "to-docx"],
            ),
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

    pub fn find_profile_for_filename(&self, filename: &str) -> Option<&DonutProfile> {
        let path = std::path::Path::new(filename);
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext.is_empty() {
            return None;
        }
        self.find_profile_for_extension(ext)
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
                    "models" | "models3d" => "models_3d",
                    "geo" | "geospatial" => "maps",
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

    pub fn select_profile_by_id(&mut self, id: &str) {
        if let Some(pos) = self.profiles.iter().position(|p| p.id == id || (p.id == "office" && id == "documents")) {
            self.selected_profile_index = pos;
        }
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
        assert_eq!(cfg.profiles.len(), 20);
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
