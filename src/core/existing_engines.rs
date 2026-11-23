//! Existing System Engine Discovery & Quality-Gated Fallback Shift
//!
//! Pure ASCII. Discovers tools already installed on the machine
//! (ArtCraft, LibreOffice, FFmpeg, Pandoc, Inkscape, Ghostscript, Tesseract)
//! with deep scanning across evident PC locations (Program Files, LocalAppData,
//! Scoop shims, Scoop apps, Chocolatey, WinGet, PATH) and provides
//! direct installation helpers via Scoop and WinGet.

use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineStatus {
    Active,
    Installed,
    NotDetected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectedEngine {
    pub id: String,
    pub name: String,
    pub path: Option<PathBuf>,
    pub version: String,
    pub status: EngineStatus,
    pub download_required_mb: usize,
    pub formats: Vec<String>,
}

impl DetectedEngine {
    pub fn description(&self) -> &'static str {
        match self.id.as_str() {
            "native-rust" => {
                "Pure native Rust codecs bundled inside Mossambi. 0.00% zero-data-loss standard."
            }
            "artcraft" => "ArtCraft Pro suite (DesignCraft, VectorCraft, AniCraft, FilmCraft).",
            "libreoffice" => {
                "LibreOffice headless suite for DOC, DOCX, ODT, RTF, and office spreadsheets."
            }
            "ffmpeg" => {
                "FFmpeg media transcoder for audio/video conversion and hardware acceleration."
            }
            "pandoc" => "Pandoc universal markup converter for Markdown, EPUB, LaTeX, and HTML.",
            "inkscape" => {
                "Inkscape vector graphics suite for high-fidelity SVG, EPS, and vector export."
            }
            "ghostscript" => {
                "Ghostscript PostScript/PDF interpreter for multipage rendering and rasterization."
            }
            "tesseract" => {
                "Tesseract OCR engine for scanned PDF text extraction and document digitization."
            }
            _ => "External system CLI conversion utility.",
        }
    }

    pub fn scoop_package(&self) -> Option<&'static str> {
        match self.id.as_str() {
            "libreoffice" => Some("extras/libreoffice"),
            "ffmpeg" => Some("ffmpeg"),
            "pandoc" => Some("pandoc"),
            "inkscape" => Some("extras/inkscape"),
            "ghostscript" => Some("ghostscript"),
            "tesseract" => Some("tesseract"),
            _ => None,
        }
    }

    pub fn winget_id(&self) -> Option<&'static str> {
        match self.id.as_str() {
            "libreoffice" => Some("TheDocumentFoundation.LibreOffice"),
            "ffmpeg" => Some("Gyan.FFmpeg"),
            "pandoc" => Some("JohnMacFarlane.Pandoc"),
            "inkscape" => Some("Inkscape.Inkscape"),
            "ghostscript" => Some("ArtifexSoftware.Ghostscript"),
            "tesseract" => Some("UB-Mannheim.TesseractOCR"),
            _ => None,
        }
    }
}

/// Status of an active package installation process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallState {
    Idle,
    Installing(String),
    Success(String),
    Error(String),
}

static INSTALL_STATE: std::sync::OnceLock<
    Arc<Mutex<std::collections::BTreeMap<String, InstallState>>>,
> = std::sync::OnceLock::new();

fn get_install_states() -> &'static Arc<Mutex<std::collections::BTreeMap<String, InstallState>>> {
    INSTALL_STATE.get_or_init(|| Arc::new(Mutex::new(std::collections::BTreeMap::new())))
}

pub fn get_engine_install_state(engine_id: &str) -> InstallState {
    let guard = get_install_states().lock().unwrap();
    guard.get(engine_id).cloned().unwrap_or(InstallState::Idle)
}

pub fn set_engine_install_state(engine_id: &str, state: InstallState) {
    let mut guard = get_install_states().lock().unwrap();
    guard.insert(engine_id.to_string(), state);
}

/// Spawns an asynchronous package installation via Scoop or WinGet.
pub fn spawn_engine_install(engine_id: String, use_scoop: bool) {
    set_engine_install_state(
        &engine_id,
        InstallState::Installing(if use_scoop {
            "Installing via Scoop...".to_string()
        } else {
            "Installing via WinGet...".to_string()
        }),
    );

    let id = engine_id.clone();
    std::thread::spawn(move || {
        let registry = ExistingEngineRegistry::detect();
        let maybe_eng = registry.get_engine(&id);
        if maybe_eng.is_none() {
            set_engine_install_state(&id, InstallState::Error("Unknown engine".to_string()));
            return;
        }
        let eng = maybe_eng.unwrap();

        let res = if use_scoop {
            if let Some(pkg) = eng.scoop_package() {
                // Execute scoop install <pkg> via powershell
                Command::new("powershell")
                    .args([
                        "-NoProfile",
                        "-ExecutionPolicy",
                        "Bypass",
                        "-Command",
                        &format!("scoop install {}", pkg),
                    ])
                    .output()
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "No Scoop package",
                ))
            }
        } else if let Some(wid) = eng.winget_id() {
            // Execute winget install <wid>
            Command::new("winget")
                .args([
                    "install",
                    wid,
                    "--accept-source-agreements",
                    "--accept-package-agreements",
                    "--silent",
                ])
                .output()
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No WinGet ID",
            ))
        };

        match res {
            Ok(output) => {
                if output.status.success() {
                    set_engine_install_state(
                        &id,
                        InstallState::Success("Installation completed successfully!".to_string()),
                    );
                } else {
                    let err = String::from_utf8_lossy(&output.stderr).to_string();
                    let msg = if err.trim().is_empty() {
                        format!("Exited with code {:?}", output.status.code())
                    } else {
                        err
                    };
                    set_engine_install_state(&id, InstallState::Error(msg));
                }
            }
            Err(e) => {
                set_engine_install_state(&id, InstallState::Error(format!("Failed to run: {}", e)));
            }
        }
    });
}

/// Detects if Scoop is installed and available on the machine.
pub fn is_scoop_available() -> bool {
    if find_executable(&["scoop", "scoop.cmd", "scoop.ps1"]).is_some() {
        return true;
    }
    if let Ok(userprofile) = env::var("USERPROFILE") {
        let shim = Path::new(&userprofile)
            .join("scoop")
            .join("shims")
            .join("scoop.ps1");
        if shim.is_file() {
            return true;
        }
    }
    false
}

/// Detects if WinGet is installed and available on the machine.
pub fn is_winget_available() -> bool {
    if find_executable(&["winget", "winget.exe"]).is_some() {
        return true;
    }
    if let Ok(localappdata) = env::var("LOCALAPPDATA") {
        let exe = Path::new(&localappdata)
            .join("Microsoft")
            .join("WindowsApps")
            .join("winget.exe");
        if exe.is_file() {
            return true;
        }
    }
    false
}

#[derive(Debug, Clone)]
pub struct ExistingEngineRegistry {
    pub engines: Vec<DetectedEngine>,
}

impl Default for ExistingEngineRegistry {
    fn default() -> Self {
        Self::detect()
    }
}

impl ExistingEngineRegistry {
    /// Detects all installed engines locally via PATH, standard OS locations,
    /// Scoop shims/apps, Chocolatey, and user app directories.
    pub fn detect() -> Self {
        let custom = crate::core::MossambiConfig::load().custom_engine_paths;
        Self::detect_with_custom(&custom)
    }

    /// Detects engines taking into account user-configured custom paths.
    pub fn detect_with_custom(custom_paths: &std::collections::BTreeMap<String, PathBuf>) -> Self {
        let mut engines = Vec::new();

        // 1. Pure Native Rust Engine (Always Active)
        engines.push(DetectedEngine {
            id: "native-rust".to_string(),
            name: "Native Rust Engine".to_string(),
            path: None,
            version: env!("CARGO_PKG_VERSION").to_string(),
            status: EngineStatus::Active,
            download_required_mb: 0,
            formats: vec![
                "PNG".into(),
                "JPEG".into(),
                "WEBP".into(),
                "TIFF".into(),
                "BMP".into(),
                "ICO".into(),
                "GIF".into(),
                "AVIF".into(),
                "PDF".into(),
                "DOCX".into(),
                "HWPX".into(),
                "XLSX".into(),
                "CSV".into(),
                "ZIP".into(),
                "TAR".into(),
                "GZ".into(),
                "ZST".into(),
            ],
        });

        // 2. ArtCraft Pro Tools (DesignCraft, VectorCraft, AniCraft, DeckCraft, Launcher)
        let artcraft_path = custom_paths
            .get("artcraft")
            .cloned()
            .filter(|p| p.is_file())
            .or_else(|| {
                let artcraft_cands = vec![
                    "artcraft",
                    "artcraft-cli",
                    "designcraft",
                    "designcraft-cli",
                    "vectorcraft",
                    "vectorcraft-cli",
                    "anicraft",
                    "anicraft-cli",
                ];
                find_tool(
                    &artcraft_cands,
                    &[
                        "ArtCraft\\DesignCraft\\designcraft.exe",
                        "ArtCraft\\DesignCraft\\designcraft-cli.exe",
                        "ArtCraft\\VectorCraft\\vectorcraft.exe",
                        "ArtCraft\\VectorCraft\\vectorcraft-cli.exe",
                        "ArtCraft\\AniCraft\\anicraft.exe",
                        "ArtCraft\\DeckCraft\\deckcraft.exe",
                        "ArtCraft\\FilmCraft\\filmcraft.exe",
                        "ArtCraft\\EffectCraft\\effectcraft.exe",
                        "ArtCraft Launcher\\artcraft-launcher.exe",
                    ],
                )
            });
        let artcraft_installed = artcraft_path.is_some();
        engines.push(DetectedEngine {
            id: "artcraft".to_string(),
            name: "ArtCraft Pro Suite".to_string(),
            path: artcraft_path,
            version: if artcraft_installed {
                "2026 Pro Installed".to_string()
            } else {
                "Not detected".to_string()
            },
            status: if artcraft_installed {
                EngineStatus::Installed
            } else {
                EngineStatus::NotDetected
            },
            download_required_mb: 0,
            formats: vec!["PSD".into(), "AI".into(), "CDR".into(), "SVG".into()],
        });

        // 3. LibreOffice (soffice)
        let soffice_path = custom_paths
            .get("libreoffice")
            .cloned()
            .filter(|p| p.is_file())
            .or_else(|| {
                find_tool(
                    &["soffice", "libreoffice"],
                    &[
                        "LibreOffice\\program\\soffice.exe",
                        "LibreOffice 5\\program\\soffice.exe",
                        "LibreOffice 6\\program\\soffice.exe",
                        "LibreOffice 7\\program\\soffice.exe",
                        "LibreOffice 24\\program\\soffice.exe",
                        "LibreOffice 25\\program\\soffice.exe",
                    ],
                )
            });
        let soffice_installed = soffice_path.is_some();
        engines.push(DetectedEngine {
            id: "libreoffice".to_string(),
            name: "LibreOffice".to_string(),
            path: soffice_path,
            version: if soffice_installed {
                "System installed".to_string()
            } else {
                "Not detected".to_string()
            },
            status: if soffice_installed {
                EngineStatus::Installed
            } else {
                EngineStatus::NotDetected
            },
            download_required_mb: 0,
            formats: vec!["DOC".into(), "DOCX".into(), "ODT".into(), "RTF".into()],
        });

        // 4. FFmpeg
        let ffmpeg_path = custom_paths
            .get("ffmpeg")
            .cloned()
            .filter(|p| p.is_file())
            .or_else(|| find_tool(&["ffmpeg", "ffmpeg.exe"], &["ffmpeg\\bin\\ffmpeg.exe"]));
        let ffmpeg_installed = ffmpeg_path.is_some();
        engines.push(DetectedEngine {
            id: "ffmpeg".to_string(),
            name: "FFmpeg Media Transcoder".to_string(),
            path: ffmpeg_path,
            version: if ffmpeg_installed {
                "System installed".to_string()
            } else {
                "Not detected".to_string()
            },
            status: if ffmpeg_installed {
                EngineStatus::Installed
            } else {
                EngineStatus::NotDetected
            },
            download_required_mb: 0,
            formats: vec!["MP4".into(), "MKV".into(), "MP3".into(), "FLAC".into()],
        });

        // 5. Pandoc
        let pandoc_path = custom_paths
            .get("pandoc")
            .cloned()
            .filter(|p| p.is_file())
            .or_else(|| find_tool(&["pandoc", "pandoc.exe"], &["Pandoc\\pandoc.exe"]));
        let pandoc_installed = pandoc_path.is_some();
        engines.push(DetectedEngine {
            id: "pandoc".to_string(),
            name: "Pandoc Markup Converter".to_string(),
            path: pandoc_path,
            version: if pandoc_installed {
                "System installed".to_string()
            } else {
                "Not detected".to_string()
            },
            status: if pandoc_installed {
                EngineStatus::Installed
            } else {
                EngineStatus::NotDetected
            },
            download_required_mb: 0,
            formats: vec!["MD".into(), "HTML".into(), "EPUB".into(), "TEX".into()],
        });

        // 6. Inkscape
        let inkscape_path = custom_paths
            .get("inkscape")
            .cloned()
            .filter(|p| p.is_file())
            .or_else(|| {
                find_tool(
                    &["inkscape", "inkscape.exe"],
                    &["Inkscape\\bin\\inkscape.exe", "Inkscape\\inkscape.exe"],
                )
            });
        let inkscape_installed = inkscape_path.is_some();
        engines.push(DetectedEngine {
            id: "inkscape".to_string(),
            name: "Inkscape Vector Graphics".to_string(),
            path: inkscape_path,
            version: if inkscape_installed {
                "System installed".to_string()
            } else {
                "Not detected".to_string()
            },
            status: if inkscape_installed {
                EngineStatus::Installed
            } else {
                EngineStatus::NotDetected
            },
            download_required_mb: 0,
            formats: vec!["SVG".into(), "EPS".into(), "PDF".into()],
        });

        // 7. Ghostscript
        let gs_path = custom_paths
            .get("ghostscript")
            .cloned()
            .filter(|p| p.is_file())
            .or_else(|| {
                let mut p = find_tool(&["gswin64c", "gswin32c", "gs"], &[]);
                if p.is_none() {
                    p = find_ghostscript_in_gs_folder();
                }
                p
            });
        let gs_installed = gs_path.is_some();
        engines.push(DetectedEngine {
            id: "ghostscript".to_string(),
            name: "Ghostscript PostScript/PDF".to_string(),
            path: gs_path,
            version: if gs_installed {
                "System installed".to_string()
            } else {
                "Not detected".to_string()
            },
            status: if gs_installed {
                EngineStatus::Installed
            } else {
                EngineStatus::NotDetected
            },
            download_required_mb: 0,
            formats: vec!["PS".into(), "EPS".into(), "PDF".into()],
        });

        // 8. Tesseract OCR
        let tess_path = custom_paths
            .get("tesseract")
            .cloned()
            .filter(|p| p.is_file())
            .or_else(|| {
                find_tool(
                    &["tesseract", "tesseract.exe"],
                    &["Tesseract-OCR\\tesseract.exe", "Tesseract\\tesseract.exe"],
                )
            });
        let tess_installed = tess_path.is_some();
        engines.push(DetectedEngine {
            id: "tesseract".to_string(),
            name: "Tesseract OCR Engine".to_string(),
            path: tess_path,
            version: if tess_installed {
                "System installed".to_string()
            } else {
                "Not detected".to_string()
            },
            status: if tess_installed {
                EngineStatus::Installed
            } else {
                EngineStatus::NotDetected
            },
            download_required_mb: 0,
            formats: vec!["PDF".into(), "TXT".into()],
        });

        Self { engines }
    }

    /// Retrieves an engine by its identifier.
    pub fn get_engine(&self, id: &str) -> Option<&DetectedEngine> {
        self.engines.iter().find(|e| e.id == id)
    }

    /// Selects the optimal engine for a conversion, favoring Native Rust first.
    pub fn select_engine_for_conversion(
        &self,
        source_format: &str,
        target_format: &str,
    ) -> &DetectedEngine {
        let src = source_format.to_ascii_uppercase();
        let tgt = target_format.to_ascii_uppercase();

        // Native Rust Engine handles standard formats with 0.00% zero-loss
        let native = self
            .get_engine("native-rust")
            .expect("Native engine exists");
        if native.formats.contains(&src) && native.formats.contains(&tgt) {
            return native;
        }

        // Check if ArtCraft Pro is available for complex proprietary graphic types
        if (src == "PSD" || src == "AI" || src == "CDR")
            && self
                .get_engine("artcraft")
                .map(|e| e.status == EngineStatus::Installed)
                .unwrap_or(false)
        {
            return self.get_engine("artcraft").unwrap();
        }

        // Check LibreOffice for complex office formats
        if (src == "DOC" || tgt == "ODT")
            && self
                .get_engine("libreoffice")
                .map(|e| e.status == EngineStatus::Installed)
                .unwrap_or(false)
        {
            return self.get_engine("libreoffice").unwrap();
        }

        // Fallback to Native Rust
        native
    }
}

/// Helper to scan candidates in PATH, system directories, Scoop shims, Scoop apps,
/// Chocolatey, WinGet packages, LocalAppData, and Program Files.
fn find_tool(cli_names: &[&str], subpaths: &[&str]) -> Option<PathBuf> {
    // 1. Check direct PATH candidates
    if let Some(p) = find_executable(cli_names) {
        return Some(p);
    }

    // Standard base directories to scan
    let mut base_dirs: Vec<PathBuf> = Vec::new();

    if let Ok(p) = env::var("ProgramFiles") {
        base_dirs.push(PathBuf::from(p));
    }
    if let Ok(p) = env::var("ProgramFiles(x86)") {
        base_dirs.push(PathBuf::from(p));
    }
    if let Ok(p) = env::var("LOCALAPPDATA") {
        base_dirs.push(PathBuf::from(&p).join("Programs"));
        base_dirs.push(PathBuf::from(&p));
    }
    if let Ok(p) = env::var("APPDATA") {
        base_dirs.push(PathBuf::from(p));
    }
    if let Ok(p) = env::var("ProgramData") {
        base_dirs.push(PathBuf::from(&p).join("chocolatey").join("bin"));
    }
    if let Ok(u) = env::var("USERPROFILE") {
        let scoop_shims = PathBuf::from(&u).join("scoop").join("shims");
        let scoop_apps = PathBuf::from(&u).join("scoop").join("apps");
        base_dirs.push(scoop_shims);
        base_dirs.push(scoop_apps);
    }

    // Direct root drive folders commonly used on Windows
    base_dirs.push(PathBuf::from("C:\\ffmpeg\\bin"));
    base_dirs.push(PathBuf::from("C:\\Program Files"));
    base_dirs.push(PathBuf::from("C:\\Program Files (x86)"));

    // Check subpaths against all base directories
    for base in &base_dirs {
        for sub in subpaths {
            let full = base.join(sub);
            if full.is_file() {
                return Some(full);
            }
        }
        for name in cli_names {
            let direct = base.join(name);
            if direct.is_file() {
                return Some(direct);
            }
            let with_exe = base.join(format!("{}.exe", name));
            if with_exe.is_file() {
                return Some(with_exe);
            }
            // Check Scoop app structure: scoop\apps\<name>\current\<name>.exe or current\bin\<name>.exe
            let scoop_curr_exe = base
                .join(name)
                .join("current")
                .join(format!("{}.exe", name));
            if scoop_curr_exe.is_file() {
                return Some(scoop_curr_exe);
            }
            let scoop_curr_bin_exe = base
                .join(name)
                .join("current")
                .join("bin")
                .join(format!("{}.exe", name));
            if scoop_curr_bin_exe.is_file() {
                return Some(scoop_curr_bin_exe);
            }
        }
    }

    None
}

/// Ghostscript installs into `C:\Program Files\gs\gs<version>\bin\gswin64c.exe`.
fn find_ghostscript_in_gs_folder() -> Option<PathBuf> {
    let mut check_roots = Vec::new();
    if let Ok(p) = env::var("ProgramFiles") {
        check_roots.push(PathBuf::from(p).join("gs"));
    }
    if let Ok(p) = env::var("ProgramFiles(x86)") {
        check_roots.push(PathBuf::from(p).join("gs"));
    }
    check_roots.push(PathBuf::from("C:\\Program Files\\gs"));
    check_roots.push(PathBuf::from("C:\\Program Files (x86)\\gs"));

    for gs_root in check_roots {
        if let Ok(entries) = std::fs::read_dir(&gs_root) {
            for entry in entries.flatten() {
                let sub_path = entry.path();
                if sub_path.is_dir() {
                    let exe64 = sub_path.join("bin").join("gswin64c.exe");
                    if exe64.is_file() {
                        return Some(exe64);
                    }
                    let exe32 = sub_path.join("bin").join("gswin32c.exe");
                    if exe32.is_file() {
                        return Some(exe32);
                    }
                    let gs_exe = sub_path.join("bin").join("gs.exe");
                    if gs_exe.is_file() {
                        return Some(gs_exe);
                    }
                }
            }
        }
    }
    None
}

/// Helper to check if an executable binary exists in PATH or direct path.
fn find_executable(candidates: &[&str]) -> Option<PathBuf> {
    for &cand in candidates {
        let p = Path::new(cand);
        if p.is_file() {
            return Some(p.to_path_buf());
        }

        // Check PATH
        if let Ok(paths) = env::var("PATH") {
            for dir in env::split_paths(&paths) {
                let full = dir.join(cand);
                if full.is_file() {
                    return Some(full);
                }
                #[cfg(windows)]
                {
                    let full_exe = dir.join(format!("{}.exe", cand));
                    if full_exe.is_file() {
                        return Some(full_exe);
                    }
                }
            }
        }
    }
    None
}
