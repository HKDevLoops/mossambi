//! Extensible Preset Architecture & Import/Export API
//!
//! Pure ASCII. Manages conversion presets, parameters, serialization,
//! and schema compliance (`mossambi-preset.v1.json`).

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PresetOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lossless: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dpi: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strip_metadata: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exam_preset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_kb: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_kb: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whiten_background: Option<bool>,
}

impl Default for PresetOptions {
    fn default() -> Self {
        Self {
            quality: Some(85),
            lossless: None,
            scale: Some(1.0),
            target_width: None,
            target_height: None,
            dpi: Some(300),
            strip_metadata: Some(false),
            effort: Some(6),
            exam_preset: None,
            min_kb: None,
            max_kb: None,
            whiten_background: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PresetDefinition {
    pub schema_version: String,
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub input_formats: Vec<String>,
    pub output_format: String,
    pub options: PresetOptions,
    pub icon: String,
    pub author: String,
    pub tags: Vec<String>,
    #[serde(default)]
    pub is_builtin: bool,
}

impl PresetDefinition {
    pub fn new(
        id: &str,
        name: &str,
        desc: &str,
        category: &str,
        inputs: &[&str],
        output: &str,
    ) -> Self {
        Self {
            schema_version: "1.0".to_string(),
            id: id.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            category: category.to_string(),
            input_formats: inputs.iter().map(|s| s.to_ascii_uppercase()).collect(),
            output_format: output.to_ascii_uppercase(),
            options: PresetOptions::default(),
            icon: "sliders".to_string(),
            author: "User".to_string(),
            tags: Vec::new(),
            is_builtin: false,
        }
    }

    /// True if this preset can run on a file with the given extension.
    pub fn matches_input(&self, ext: &str) -> bool {
        let u = ext.to_ascii_uppercase();
        self.input_formats.iter().any(|f| f == &u || f == "*")
    }

    /// Serializes preset into standardized pretty JSON string.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).context("Failed to serialize preset definition to JSON")
    }

    /// Deserializes preset from a JSON string with validation.
    pub fn from_json(json_str: &str) -> Result<Self> {
        let preset: Self = serde_json::from_str(json_str)
            .context("Failed to deserialize preset JSON; invalid schema")?;
        if preset.id.trim().is_empty() {
            bail!("Preset id cannot be empty");
        }
        if preset.name.trim().is_empty() {
            bail!("Preset name cannot be empty");
        }
        if preset.output_format.trim().is_empty() {
            bail!("Preset output_format cannot be empty");
        }
        Ok(preset)
    }
}

/// Returns the user's preset storage directory (`~/.config/mossambi/presets`).
pub fn get_presets_dir() -> PathBuf {
    let base = crate::core::platform_paths::config_dir();
    base.join("mossambi").join("presets")
}

/// Returns the collection of standard factory built-in presets.
pub fn get_builtin_presets() -> Vec<PresetDefinition> {
    vec![
        PresetDefinition {
            schema_version: "1.0".to_string(),
            id: "web-ultra-webp".to_string(),
            name: "Web Ultra WebP".to_string(),
            description: "High-compression WebP for websites with stripped metadata".to_string(),
            category: "Image".to_string(),
            input_formats: vec![
                "PNG".to_string(),
                "JPG".to_string(),
                "JPEG".to_string(),
                "TIFF".to_string(),
                "BMP".to_string(),
            ],
            output_format: "WEBP".to_string(),
            options: PresetOptions {
                quality: Some(82),
                lossless: Some(false),
                scale: Some(1.0),
                target_width: None,
                target_height: None,
                dpi: Some(72),
                strip_metadata: Some(true),
                effort: Some(6),
                exam_preset: None,
                min_kb: None,
                max_kb: None,
                whiten_background: None,
            },
            icon: "globe".to_string(),
            author: "Mossambi".to_string(),
            tags: vec!["web".to_string(), "compression".to_string()],
            is_builtin: true,
        },
        PresetDefinition {
            schema_version: "1.0".to_string(),
            id: "archive-lossless-png".to_string(),
            name: "Bit-Exact Lossless PNG".to_string(),
            description: "0.00% zero-loss archival PNG transcode with maximum compression"
                .to_string(),
            category: "Image".to_string(),
            input_formats: vec![
                "BMP".to_string(),
                "TIFF".to_string(),
                "QOI".to_string(),
                "TGA".to_string(),
                "PPM".to_string(),
                "PSD".to_string(),
            ],
            output_format: "PNG".to_string(),
            options: PresetOptions {
                quality: Some(100),
                lossless: Some(true),
                scale: Some(1.0),
                target_width: None,
                target_height: None,
                dpi: Some(300),
                strip_metadata: Some(false),
                effort: Some(9),
                exam_preset: None,
                min_kb: None,
                max_kb: None,
                whiten_background: None,
            },
            icon: "shield".to_string(),
            author: "Mossambi".to_string(),
            tags: vec!["lossless".to_string(), "archival".to_string()],
            is_builtin: true,
        },
        PresetDefinition {
            schema_version: "1.0".to_string(),
            id: "upsc-photo".to_string(),
            name: "UPSC Exam Photo".to_string(),
            description:
                "Conforms passport photo to UPSC portal specs (350x450, 20-50 KB, 300 DPI)"
                    .to_string(),
            category: "Exam".to_string(),
            input_formats: vec![
                "JPG".to_string(),
                "JPEG".to_string(),
                "PNG".to_string(),
                "WEBP".to_string(),
            ],
            output_format: "JPG".to_string(),
            options: PresetOptions {
                quality: Some(90),
                lossless: Some(false),
                scale: None,
                target_width: Some(350),
                target_height: Some(450),
                dpi: Some(300),
                strip_metadata: Some(true),
                effort: Some(6),
                exam_preset: Some("UPSC".to_string()),
                min_kb: Some(20),
                max_kb: Some(50),
                whiten_background: Some(true),
            },
            icon: "user-check".to_string(),
            author: "Mossambi Exam Engine".to_string(),
            tags: vec![
                "exam".to_string(),
                "upsc".to_string(),
                "government".to_string(),
            ],
            is_builtin: true,
        },
        PresetDefinition {
            schema_version: "1.0".to_string(),
            id: "upsc-signature".to_string(),
            name: "UPSC Exam Signature".to_string(),
            description:
                "Conforms signature scan to UPSC portal specs (1000x550, 10-20 KB, pure white bg)"
                    .to_string(),
            category: "Exam".to_string(),
            input_formats: vec!["JPG".to_string(), "JPEG".to_string(), "PNG".to_string()],
            output_format: "JPG".to_string(),
            options: PresetOptions {
                quality: Some(85),
                lossless: Some(false),
                scale: None,
                target_width: Some(1000),
                target_height: Some(550),
                dpi: Some(300),
                strip_metadata: Some(true),
                effort: Some(6),
                exam_preset: Some("UPSC".to_string()),
                min_kb: Some(10),
                max_kb: Some(20),
                whiten_background: Some(true),
            },
            icon: "feather".to_string(),
            author: "Mossambi Exam Engine".to_string(),
            tags: vec!["exam".to_string(), "signature".to_string()],
            is_builtin: true,
        },
        PresetDefinition {
            schema_version: "1.0".to_string(),
            id: "doc-archive-pdf".to_string(),
            name: "Archival Vector PDF".to_string(),
            description: "Converts documents and presentations into clean, high-DPI vector PDF"
                .to_string(),
            category: "Document".to_string(),
            input_formats: vec![
                "DOCX".to_string(),
                "HWPX".to_string(),
                "ODT".to_string(),
                "RTF".to_string(),
                "MD".to_string(),
                "HTML".to_string(),
                "EPUB".to_string(),
                "PPTX".to_string(),
            ],
            output_format: "PDF".to_string(),
            options: PresetOptions {
                quality: Some(100),
                lossless: Some(true),
                scale: Some(1.0),
                target_width: None,
                target_height: None,
                dpi: Some(300),
                strip_metadata: Some(false),
                effort: Some(6),
                exam_preset: None,
                min_kb: None,
                max_kb: None,
                whiten_background: None,
            },
            icon: "file-text".to_string(),
            author: "Mossambi".to_string(),
            tags: vec!["document".to_string(), "pdf".to_string()],
            is_builtin: true,
        },
    ]
}

/// Lists all available presets (built-in + user custom presets).
pub fn list_all_presets(category: Option<&str>) -> Vec<PresetDefinition> {
    let mut presets = get_builtin_presets();

    let custom_dir = get_presets_dir();
    if custom_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&custom_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(custom) = PresetDefinition::from_json(&content) {
                            if !presets.iter().any(|p| p.id == custom.id) {
                                presets.push(custom);
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(cat) = category {
        let cat_lower = cat.to_ascii_lowercase();
        presets.retain(|p| p.category.to_ascii_lowercase() == cat_lower);
    }

    presets
}

/// Finds matching presets that can process a file with the given extension.
pub fn find_matching_presets(ext: &str) -> Vec<PresetDefinition> {
    list_all_presets(None)
        .into_iter()
        .filter(|p| p.matches_input(ext))
        .collect()
}

/// Finds a preset by its unique identifier.
pub fn find_preset_by_id(id: &str) -> Option<PresetDefinition> {
    list_all_presets(None).into_iter().find(|p| p.id == id)
}

/// Saves a custom preset definition into the user's preset directory.
pub fn save_custom_preset(preset: &PresetDefinition) -> Result<PathBuf> {
    let dir = get_presets_dir();
    fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create presets directory at {:?}", dir))?;

    let filename = format!("{}.json", preset.id);
    let target = dir.join(filename);

    let json = preset.to_json()?;
    fs::write(&target, json)
        .with_context(|| format!("Failed to write preset file to {:?}", target))?;

    Ok(target)
}

/// Deletes a custom preset by its ID. Returns false if preset was not found or is built-in.
pub fn delete_custom_preset(id: &str) -> Result<bool> {
    let dir = get_presets_dir();
    let target = dir.join(format!("{}.json", id));
    if target.is_file() {
        fs::remove_file(&target)
            .with_context(|| format!("Failed to delete preset file {:?}", target))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Exports a preset to an external path destination.
pub fn export_preset_to_file(id: &str, target_path: &Path) -> Result<()> {
    let preset = match find_preset_by_id(id) {
        Some(p) => p,
        None => bail!("Preset with id '{}' not found", id),
    };

    let json = preset.to_json()?;
    if let Some(parent) = target_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(target_path, json)
        .with_context(|| format!("Failed to export preset to {:?}", target_path))?;

    Ok(())
}

/// Imports a preset from an external file and saves it to the user presets directory.
pub fn import_preset_from_file(source_path: &Path) -> Result<PresetDefinition> {
    let content = fs::read_to_string(source_path)
        .with_context(|| format!("Failed to read preset file at {:?}", source_path))?;

    let preset = PresetDefinition::from_json(&content)?;
    save_custom_preset(&preset)?;

    Ok(preset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_presets_validation() {
        let builtins = get_builtin_presets();
        assert!(!builtins.is_empty());
        for p in &builtins {
            let json = p.to_json().expect("Builtin preset should serialize");
            let round_trip = PresetDefinition::from_json(&json)
                .expect("Round-trip deserialization should succeed");
            assert_eq!(p.id, round_trip.id);
        }
    }

    #[test]
    fn test_matching_presets() {
        let matches_png = find_matching_presets("png");
        assert!(matches_png.iter().any(|p| p.id == "web-ultra-webp"));
        assert!(matches_png.iter().any(|p| p.id == "upsc-photo"));

        let matches_docx = find_matching_presets("docx");
        assert!(matches_docx.iter().any(|p| p.id == "doc-archive-pdf"));
    }
}
