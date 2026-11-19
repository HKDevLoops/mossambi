//! ArtCraft Apps Ecosystem Integration Bridge
//!
//! Pure ASCII. Discovers and bridges ArtCraft high-performance creative tools
//! (VectorCraft, DesignCraft, DeckCraft, FilmCraft, EffectCraft, AniCraft).
//! Supports both direct CLI execution and stdio MCP (Model Context Protocol) automation.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtcraftApp {
    VectorCraft,
    DesignCraft,
    DeckCraft,
    FilmCraft,
    EffectCraft,
    AniCraft,
}

impl ArtcraftApp {
    pub fn name(&self) -> &'static str {
        match self {
            Self::VectorCraft => "VectorCraft",
            Self::DesignCraft => "DesignCraft",
            Self::DeckCraft => "DeckCraft",
            Self::FilmCraft => "FilmCraft",
            Self::EffectCraft => "EffectCraft",
            Self::AniCraft => "AniCraft",
        }
    }

    pub fn cli_binary_name(&self) -> &'static str {
        #[cfg(target_os = "windows")]
        {
            match self {
                Self::VectorCraft => "vectorcraft-cli.exe",
                Self::DesignCraft => "designcraft-cli.exe",
                Self::DeckCraft => "deckcraft-cli.exe",
                Self::FilmCraft => "filmcraft-cli.exe",
                Self::EffectCraft => "effectcraft-cli.exe",
                Self::AniCraft => "anicraft-cli.exe",
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            match self {
                Self::VectorCraft => "vectorcraft-cli",
                Self::DesignCraft => "designcraft-cli",
                Self::DeckCraft => "deckcraft-cli",
                Self::FilmCraft => "filmcraft-cli",
                Self::EffectCraft => "effectcraft-cli",
                Self::AniCraft => "anicraft-cli",
            }
        }
    }
}

/// Global system detector for installed ArtCraft suite applications.
#[derive(Debug, Clone, Default)]
pub struct ArtcraftDiscovery {
    found_apps: std::collections::HashMap<ArtcraftApp, PathBuf>,
}

impl ArtcraftDiscovery {
    /// Discovers ArtCraft applications across standard OS install paths.
    pub fn detect() -> Self {
        let mut discovery = Self::default();

        let mut candidate_dirs: Vec<PathBuf> = Vec::new();

        // 1. Check ARTCRAFT_PATH environment override if provided
        if let Ok(env_path) = std::env::var("ARTCRAFT_PATH") {
            let p = PathBuf::from(env_path);
            if p.is_dir() {
                candidate_dirs.push(p);
            }
        }

        // 2. Standard OS installation directories
        #[cfg(target_os = "windows")]
        {
            if let Ok(pf) = std::env::var("ProgramFiles") {
                candidate_dirs.push(PathBuf::from(pf).join("ArtCraft"));
            }
            if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
                candidate_dirs.push(PathBuf::from(pf86).join("ArtCraft"));
            }
            if let Ok(la) = std::env::var("LOCALAPPDATA") {
                candidate_dirs.push(PathBuf::from(la).join("Programs").join("ArtCraft"));
            }
            candidate_dirs.push(PathBuf::from("C:\\Program Files\\ArtCraft"));
        }

        #[cfg(target_os = "macos")]
        {
            candidate_dirs.push(PathBuf::from("/Applications/ArtCraft"));
            if let Ok(home) = std::env::var("HOME") {
                candidate_dirs.push(PathBuf::from(home).join("Applications").join("ArtCraft"));
            }
        }

        #[cfg(target_os = "linux")]
        {
            candidate_dirs.push(PathBuf::from("/opt/artcraft"));
            candidate_dirs.push(PathBuf::from("/usr/local/bin"));
            candidate_dirs.push(PathBuf::from("/usr/bin"));
            if let Ok(home) = std::env::var("HOME") {
                candidate_dirs.push(PathBuf::from(home).join(".local").join("bin"));
            }
        }

        let all_apps = [
            ArtcraftApp::VectorCraft,
            ArtcraftApp::DesignCraft,
            ArtcraftApp::DeckCraft,
            ArtcraftApp::FilmCraft,
            ArtcraftApp::EffectCraft,
            ArtcraftApp::AniCraft,
        ];

        for &app in &all_apps {
            let bin_name = app.cli_binary_name();
            let app_folder_name = app.name();

            for dir in &candidate_dirs {
                // Direct file check: <dir>/<bin_name>
                let direct_bin = dir.join(bin_name);
                if direct_bin.is_file() {
                    discovery.found_apps.insert(app, direct_bin);
                    break;
                }

                // Subfolder check: <dir>/<AppName>/<bin_name>
                let subfolder_bin = dir.join(app_folder_name).join(bin_name);
                if subfolder_bin.is_file() {
                    discovery.found_apps.insert(app, subfolder_bin);
                    break;
                }
            }
        }

        discovery
    }

    pub fn is_installed(&self, app: ArtcraftApp) -> bool {
        self.found_apps.contains_key(&app)
    }

    pub fn get_cli_path(&self, app: ArtcraftApp) -> Option<&PathBuf> {
        self.found_apps.get(&app)
    }

    pub fn installed_apps(&self) -> Vec<ArtcraftApp> {
        self.found_apps.keys().copied().collect()
    }
}

/// Executes vector file conversion via VectorCraft CLI.
pub fn vectorcraft_convert(
    in_file: &Path,
    out_file: &Path,
    scale: Option<f32>,
    ppi: Option<u32>,
) -> Result<()> {
    let discovery = ArtcraftDiscovery::detect();
    let cli_path = match discovery.get_cli_path(ArtcraftApp::VectorCraft) {
        Some(p) => p,
        None => bail!("VectorCraft CLI is not installed or not found"),
    };

    let mut cmd = Command::new(cli_path);
    cmd.arg("convert").arg(in_file).arg(out_file);

    if let Some(s) = scale {
        cmd.arg("--scale").arg(s.to_string());
    }
    if let Some(p) = ppi {
        cmd.arg("--ppi").arg(p.to_string());
    }

    let status = cmd.status().with_context(|| {
        format!(
            "Failed to execute vectorcraft-cli convert for {:?}",
            in_file
        )
    })?;

    if !status.success() {
        bail!("vectorcraft-cli convert exited with status {}", status);
    }

    if !out_file.exists() {
        bail!(
            "vectorcraft-cli finished but output file was not created: {:?}",
            out_file
        );
    }

    Ok(())
}

/// Executes presentation file conversion via DeckCraft CLI.
pub fn deckcraft_convert(in_file: &Path, out_file: &Path) -> Result<()> {
    let discovery = ArtcraftDiscovery::detect();
    let cli_path = match discovery.get_cli_path(ArtcraftApp::DeckCraft) {
        Some(p) => p,
        None => bail!("DeckCraft CLI is not installed or not found"),
    };

    let mut cmd = Command::new(cli_path);
    cmd.arg("convert").arg(in_file).arg(out_file);

    let status = cmd
        .status()
        .with_context(|| format!("Failed to execute deckcraft-cli convert for {:?}", in_file))?;

    if !status.success() {
        bail!("deckcraft-cli convert exited with status {}", status);
    }

    if !out_file.exists() {
        bail!(
            "deckcraft-cli finished but output file was not created: {:?}",
            out_file
        );
    }

    Ok(())
}

/// Executes document publishing export via DesignCraft CLI.
pub fn designcraft_export(in_file: &Path, out_file: &Path) -> Result<()> {
    let discovery = ArtcraftDiscovery::detect();
    let cli_path = match discovery.get_cli_path(ArtcraftApp::DesignCraft) {
        Some(p) => p,
        None => bail!("DesignCraft CLI is not installed or not found"),
    };

    let mut cmd = Command::new(cli_path);
    cmd.arg("run")
        .arg("--in")
        .arg(in_file)
        .arg("--export")
        .arg(out_file);

    let status = cmd
        .status()
        .with_context(|| format!("Failed to execute designcraft-cli run for {:?}", in_file))?;

    if !status.success() {
        bail!("designcraft-cli run exited with status {}", status);
    }

    if !out_file.exists() {
        bail!(
            "designcraft-cli finished but output file was not created: {:?}",
            out_file
        );
    }

    Ok(())
}

/// Spawns an in-process MCP (Model Context Protocol) automation server for an ArtCraft application.
pub fn spawn_artcraft_mcp_server(
    app: ArtcraftApp,
    read_root: &Path,
    write_root: &Path,
) -> Result<Child> {
    let discovery = ArtcraftDiscovery::detect();
    let cli_path = match discovery.get_cli_path(app) {
        Some(p) => p,
        None => bail!("ArtCraft app {:?} CLI is not installed", app),
    };

    let child = Command::new(cli_path)
        .arg("mcp")
        .arg("--headless")
        .arg("--automation-read-root")
        .arg(read_root)
        .arg("--automation-write-root")
        .arg(write_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to spawn ArtCraft MCP server for {:?}", app))?;

    Ok(child)
}

/// Returns true if the file extension is supported as input by VectorCraft.
pub fn is_vectorcraft_readable(ext: &str) -> bool {
    matches!(
        ext.to_ascii_uppercase().as_str(),
        "VECTORCRAFT"
            | "DRAWCRAFT"
            | "SVG"
            | "SVGZ"
            | "PDF"
            | "AI"
            | "AIT"
            | "PNG"
            | "PNS"
            | "JPG"
            | "JPEG"
            | "GIF"
            | "WEBP"
            | "TIF"
            | "TIFF"
            | "BMP"
            | "DIB"
            | "TGA"
            | "PCX"
            | "JP2"
            | "HEIC"
            | "HEIF"
            | "PSD"
            | "PSB"
            | "JXL"
            | "DXF"
            | "EMF"
            | "WMF"
            | "EPS"
            | "PS"
            | "AFDESIGN"
            | "AFPUB"
    )
}

/// Returns true if the file extension is supported as output by VectorCraft.
pub fn is_vectorcraft_writable(ext: &str) -> bool {
    matches!(
        ext.to_ascii_uppercase().as_str(),
        "VECTORCRAFT"
            | "SVG"
            | "SVGZ"
            | "PDF"
            | "AI"
            | "PNG"
            | "JPG"
            | "JPEG"
            | "GIF"
            | "WEBP"
            | "TIF"
            | "TIFF"
            | "BMP"
            | "TXT"
            | "DXF"
            | "EPS"
            | "EMF"
            | "WMF"
            | "TGA"
            | "PSD"
    )
}

/// Returns true if the file extension is supported as input by DeckCraft.
pub fn is_deckcraft_readable(ext: &str) -> bool {
    matches!(ext.to_ascii_uppercase().as_str(), "DECKCRAFT" | "PPTX")
}

/// Returns true if the file extension is supported as input by DesignCraft.
pub fn is_designcraft_readable(ext: &str) -> bool {
    matches!(
        ext.to_ascii_uppercase().as_str(),
        "DESIGNCRAFT" | "IDML" | "EPUB" | "PDF"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artcraft_discovery() {
        let disc = ArtcraftDiscovery::detect();
        // Check if VectorCraft is detected on this dev system
        if disc.is_installed(ArtcraftApp::VectorCraft) {
            let p = disc.get_cli_path(ArtcraftApp::VectorCraft).unwrap();
            assert!(p.exists());
        }
    }

    #[test]
    fn test_artcraft_format_support() {
        assert!(is_vectorcraft_readable("afdesign"));
        assert!(is_vectorcraft_readable("ai"));
        assert!(is_vectorcraft_readable("dxf"));
        assert!(is_vectorcraft_writable("svg"));
        assert!(is_vectorcraft_writable("pdf"));
        assert!(is_deckcraft_readable("pptx"));
        assert!(is_designcraft_readable("idml"));
    }
}
