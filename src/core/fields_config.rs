//! Fields Configuration
//!
//! Controls domain activation switches matching the Filee Fields architecture.
//! Disabling a field removes its specialized presets from the radial donut slices,
//! while retaining CLI conversion and drag reception capability.

use serde::{Deserialize, Serialize};

/// 12 Toggleable domain fields matching Filee Fields architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldsConfig {
    #[serde(default = "default_true")]
    pub cad: bool,
    #[serde(default = "default_true")]
    pub fonts: bool,
    #[serde(default = "default_true")]
    pub models_3d: bool,
    #[serde(default = "default_true")]
    pub maps: bool,
    #[serde(default = "default_true")]
    pub games: bool,
    #[serde(default = "default_true")]
    pub design_print: bool,
    #[serde(default = "default_true")]
    pub science_medical: bool,
    #[serde(default = "default_true")]
    pub developer: bool,
    #[serde(default = "default_true")]
    pub technical_writing: bool,
    #[serde(default = "default_true")]
    pub video_broadcast: bool,
    #[serde(default = "default_true")]
    pub pro_audio: bool,
    #[serde(default = "default_true")]
    pub old_formats: bool,
}

fn default_true() -> bool {
    true
}

impl Default for FieldsConfig {
    fn default() -> Self {
        Self {
            cad: true,
            fonts: true,
            models_3d: true,
            maps: true,
            games: true,
            design_print: true,
            science_medical: true,
            developer: true,
            technical_writing: true,
            video_broadcast: true,
            pro_audio: true,
            old_formats: true,
        }
    }
}

/// Metadata descriptor for rendering domain toggles in the Fields settings subpage.
#[derive(Debug, Clone)]
pub struct FieldItemSpec {
    pub key: &'static str,
    pub title: &'static str,
    pub formats: &'static str,
}

impl FieldsConfig {
    pub const SPECS: &'static [FieldItemSpec] = &[
        FieldItemSpec {
            key: "cad",
            title: "CAD",
            formats: "DWG, STL, OBJ, 3MF",
        },
        FieldItemSpec {
            key: "fonts",
            title: "Fonts",
            formats: "TTF, OTF, WOFF, WOFF2, EOT",
        },
        FieldItemSpec {
            key: "models_3d",
            title: "3D models",
            formats: "GLB, glTF, STL, OBJ, 3MF, PLY",
        },
        FieldItemSpec {
            key: "maps",
            title: "Maps",
            formats: "GeoJSON, KML, KMZ, GPX, Shapefile",
        },
        FieldItemSpec {
            key: "games",
            title: "Games",
            formats: "DDS, Minecraft pack, Minecraft add-on, Minecraft world, Minecraft world template, Modrinth modpack",
        },
        FieldItemSpec {
            key: "design_print",
            title: "Design & print",
            formats: "JPEG 2000, PSD, PSB, EPS, PS",
        },
        FieldItemSpec {
            key: "science_medical",
            title: "Science & medical",
            formats: "JPEG 2000, Jupyter",
        },
        FieldItemSpec {
            key: "developer",
            title: "Developer",
            formats: "PPM, ICNS, QOI, CUR, XPM, XBM...",
        },
        FieldItemSpec {
            key: "technical_writing",
            title: "Technical writing & publishing",
            formats: "reStructuredText, LaTeX, Org, AsciiDoc, Typst, MediaWiki...",
        },
        FieldItemSpec {
            key: "video_broadcast",
            title: "Video production & broadcast",
            formats: "DV, MXF, AC3, MP2, E-AC3, ASS...",
        },
        FieldItemSpec {
            key: "pro_audio",
            title: "Pro audio",
            formats: "CAF, WavPack, TTA, W64",
        },
        FieldItemSpec {
            key: "old_formats",
            title: "Old formats",
            formats: "MNG, PCX, WBMP, SGI, Sun Raster, WMF...",
        },
    ];

    pub fn is_field_enabled(&self, key: &str) -> bool {
        match key {
            "cad" => self.cad,
            "fonts" => self.fonts,
            "models_3d" => self.models_3d,
            "maps" => self.maps,
            "games" => self.games,
            "design_print" => self.design_print,
            "science_medical" => self.science_medical,
            "developer" => self.developer,
            "technical_writing" => self.technical_writing,
            "video_broadcast" => self.video_broadcast,
            "pro_audio" => self.pro_audio,
            "old_formats" => self.old_formats,
            _ => true,
        }
    }

    pub fn set_field_enabled(&mut self, key: &str, enabled: bool) {
        match key {
            "cad" => self.cad = enabled,
            "fonts" => self.fonts = enabled,
            "models_3d" => self.models_3d = enabled,
            "maps" => self.maps = enabled,
            "games" => self.games = enabled,
            "design_print" => self.design_print = enabled,
            "science_medical" => self.science_medical = enabled,
            "developer" => self.developer = enabled,
            "technical_writing" => self.technical_writing = enabled,
            "video_broadcast" => self.video_broadcast = enabled,
            "pro_audio" => self.pro_audio = enabled,
            "old_formats" => self.old_formats = enabled,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fields_config_defaults() {
        let f = FieldsConfig::default();
        assert!(f.cad);
        assert!(f.fonts);
        assert!(f.models_3d);
        assert!(f.maps);
        assert_eq!(FieldsConfig::SPECS.len(), 12);
    }
}
