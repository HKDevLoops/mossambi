//! Configuration, completions and Carapace command handlers.
//! Pure ASCII by contract.

use anyhow::{bail, Result};
use serde_json::json;
use std::path::Path;

use crate::cli::Cli;
use crate::core::{MossambiConfig, ThemeMode, UniversalRegistry};

pub fn handle_config(
    action: &str,
    key: Option<&str>,
    value: Option<&str>,
    as_json: bool,
) -> Result<()> {
    let mut config = MossambiConfig::load();

    match action.to_lowercase().as_str() {
        "get" => {
            if let Some(k) = key {
                let val_str = match k {
                    "shade" | "citrus_shade" => config.citrus_shade.clone(),
                    "theme" | "theme_mode" => format!("{:?}", config.theme_mode),
                    "always_on_top" => config.always_on_top.to_string(),
                    "wheel_size" => config.wheel_size.to_string(),
                    "gesture" | "enable_gesture_wakeup" => config.enable_gesture_wakeup.to_string(),
                    "hotkey" | "enable_hotkey_wakeup" => config.enable_hotkey_wakeup.to_string(),
                    "zero_loss" | "enforce_zero_loss" => config.enforce_zero_loss.to_string(),
                    _ => format!("Unknown key '{}'", k),
                };
                if as_json {
                    println!("{}", json!({ "key": k, "value": val_str }));
                } else {
                    println!("{} = {}", k, val_str);
                }
            } else {
                bail!("Missing key to retrieve. Use 'mossambi config get <key>'");
            }
        }
        "set" => {
            if let (Some(k), Some(v)) = (key, value) {
                match k {
                    "shade" | "citrus_shade" => config.citrus_shade = v.to_string(),
                    "theme" | "theme_mode" => {
                        if let Some(mode) = ThemeMode::from_id_or_name(v) {
                            config.theme_mode = mode;
                        } else {
                            match v.to_lowercase().as_str() {
                                "dark" => config.theme_mode = ThemeMode::Rangalipi,
                                "light" => config.theme_mode = ThemeMode::MossambiPunch,
                                _ => bail!(
                                    "Unknown theme: '{}'. Use 'mossambi themes' to see all named themes.",
                                    v
                                ),
                            }
                        }
                    }
                    "always_on_top" => config.always_on_top = v.parse().unwrap_or(true),
                    "wheel_size" => config.wheel_size = v.parse().unwrap_or(440),
                    "gesture" | "enable_gesture_wakeup" => {
                        config.enable_gesture_wakeup = v.parse().unwrap_or(true)
                    }
                    "hotkey" | "enable_hotkey_wakeup" => {
                        config.enable_hotkey_wakeup = v.parse().unwrap_or(true)
                    }
                    "zero_loss" | "enforce_zero_loss" => {
                        config.enforce_zero_loss = v.parse().unwrap_or(true)
                    }
                    _ => bail!("Unknown setting key: {}", k),
                }
                config.save()?;
                if as_json {
                    println!("{}", json!({ "status": "saved", "key": k, "value": v }));
                } else {
                    println!("Saved: {} = {}", k, v);
                }
            } else {
                bail!("Usage: mossambi config set <key> <value>");
            }
        }
        _ => {
            if as_json {
                println!("{}", serde_json::to_string_pretty(&config)?);
            } else {
                println!("Mossambi Persistent Configuration (~/.mossambi/config.json)");
                println!("---------------------------------------------------------------");
                println!("Theme Mode:            {:?}", config.theme_mode);
                println!("Citrus Shade:          {}", config.citrus_shade);
                println!("Always On Top:         {}", config.always_on_top);
                println!("Wheel Size:            {} px", config.wheel_size);
                println!("Win+Ctrl Hotkey:       {}", config.enable_hotkey_wakeup);
                println!("Circle Gesture Wakeup: {}", config.enable_gesture_wakeup);
                println!("Zero-Loss Standard:    {}", config.enforce_zero_loss);
                println!(
                    "Config Path:           {}",
                    MossambiConfig::config_path().display()
                );
                println!("---------------------------------------------------------------");
                println!("To modify: mossambi config set <key> <value>");
            }
        }
    }

    Ok(())
}

/// Intelligently detect the active user shell based on environment variables and OS conventions.
pub fn detect_shell() -> clap_complete::Shell {
    use clap_complete::Shell;

    if let Ok(sh) = std::env::var("SHELL") {
        let lower = sh.to_lowercase();
        if lower.ends_with("zsh") {
            return Shell::Zsh;
        } else if lower.ends_with("bash") {
            return Shell::Bash;
        } else if lower.ends_with("fish") {
            return Shell::Fish;
        } else if lower.ends_with("elvish") {
            return Shell::Elvish;
        }
    }

    if std::env::var("ZSH_VERSION").is_ok() {
        return Shell::Zsh;
    }
    if std::env::var("BASH_VERSION").is_ok() {
        return Shell::Bash;
    }
    if std::env::var("FISH_VERSION").is_ok() {
        return Shell::Fish;
    }

    if std::env::var("PSModulePath").is_ok()
        || std::env::var("POWERSHELL_DISTRIBUTION_CHANNEL").is_ok()
        || std::env::var("PWSH").is_ok()
    {
        return Shell::PowerShell;
    }

    #[cfg(target_os = "windows")]
    {
        Shell::PowerShell
    }
    #[cfg(target_os = "macos")]
    {
        Shell::Zsh
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Shell::Bash
    }
}

/// Install completion script automatically to the target shell's configuration directory.
pub fn install_completions(shell: clap_complete::Shell, script: &str) -> Result<()> {
    use clap_complete::Shell;
    let home = crate::core::platform_paths::home_dir();

    match shell {
        Shell::PowerShell => {
            let comp_dir = home.join(".mossambi").join("completions");
            std::fs::create_dir_all(&comp_dir)?;
            let script_path = comp_dir.join("mossambi.ps1");
            std::fs::write(&script_path, script.as_bytes())?;

            println!("[mossambi] Shell autocompletion script generated:");
            println!("  Path: {}", script_path.display());

            let source_line = format!(
                "\n# Mossambi shell autocompletion\nif (Test-Path \"{}\") {{ . \"{}\" }}\n",
                script_path.display(),
                script_path.display()
            );

            let candidates = [
                home.join("Documents")
                    .join("PowerShell")
                    .join("Microsoft.PowerShell_profile.ps1"),
                home.join("Documents")
                    .join("WindowsPowerShell")
                    .join("Microsoft.PowerShell_profile.ps1"),
                home.join(".config")
                    .join("powershell")
                    .join("Microsoft.PowerShell_profile.ps1"),
            ];

            let mut configured = Vec::new();
            for prof in &candidates {
                if let Some(parent) = prof.parent() {
                    if parent.exists() {
                        let content = std::fs::read_to_string(prof).unwrap_or_default();
                        if !content.contains("mossambi.ps1") {
                            use std::io::Write;
                            if let Ok(mut f) = std::fs::OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(prof)
                            {
                                let _ = f.write_all(source_line.as_bytes());
                                configured.push(prof.clone());
                            }
                        } else {
                            configured.push(prof.clone());
                        }
                    }
                }
            }

            if configured.is_empty() {
                let default_prof = candidates[0].clone();
                if let Some(parent) = default_prof.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                use std::io::Write;
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&default_prof)
                {
                    let _ = f.write_all(source_line.as_bytes());
                    configured.push(default_prof);
                }
            }

            for p in &configured {
                println!("  Profile configured: {}", p.display());
            }
            println!("[mossambi] To activate in the current session, run:");
            if let Some(first) = configured.first() {
                println!("  . \"{}\"", first.display());
            }
        }
        Shell::Fish => {
            let comp_dir = home.join(".config").join("fish").join("completions");
            std::fs::create_dir_all(&comp_dir)?;
            let script_path = comp_dir.join("mossambi.fish");
            std::fs::write(&script_path, script.as_bytes())?;
            println!("[mossambi] Shell autocompletion installed for Fish:");
            println!("  Path: {}", script_path.display());
            println!("  Fish automatically loads completions on demand in new sessions.");
        }
        Shell::Bash => {
            let comp_dir = home
                .join(".local")
                .join("share")
                .join("bash-completion")
                .join("completions");
            std::fs::create_dir_all(&comp_dir)?;
            let script_path = comp_dir.join("mossambi");
            std::fs::write(&script_path, script.as_bytes())?;

            println!("[mossambi] Shell autocompletion installed for Bash:");
            println!("  Path: {}", script_path.display());

            let bash_comp = home.join(".bash_completion");
            let source_cmd = format!("\nsource \"{}\"\n", script_path.display());
            let current = std::fs::read_to_string(&bash_comp).unwrap_or_default();
            if !current.contains("bash-completion/completions/mossambi") {
                use std::io::Write;
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&bash_comp)
                {
                    let _ = f.write_all(source_cmd.as_bytes());
                    println!("  Sourced in: {}", bash_comp.display());
                }
            }
            println!("[mossambi] To activate in the current session, run:");
            println!("  source \"{}\"", script_path.display());
        }
        Shell::Zsh => {
            let comp_dir = home.join(".zsh").join("completion");
            std::fs::create_dir_all(&comp_dir)?;
            let script_path = comp_dir.join("_mossambi");
            std::fs::write(&script_path, script.as_bytes())?;

            println!("[mossambi] Shell autocompletion installed for Zsh:");
            println!("  Path: {}", script_path.display());
            println!("  Ensure ~/.zsh/completion is in your $fpath in ~/.zshrc:");
            println!("    fpath=(~/.zsh/completion $fpath)");
            println!("    autoload -Uz compinit && compinit");
        }
        Shell::Elvish => {
            let comp_dir = home.join(".config").join("elvish").join("lib");
            std::fs::create_dir_all(&comp_dir)?;
            let script_path = comp_dir.join("mossambi.elv");
            std::fs::write(&script_path, script.as_bytes())?;

            println!("[mossambi] Shell autocompletion installed for Elvish:");
            println!("  Path: {}", script_path.display());
            println!("  Add 'use mossambi' to your ~/.config/elvish/rc.elv to activate.");
        }
        _ => {}
    }
    Ok(())
}

/// Generate shell completions via clap_complete, or a Carapace YAML spec.
pub fn handle_completions(shell_str: Option<&str>, carapace: bool, install: bool) -> Result<()> {
    use clap::CommandFactory;
    use clap_complete::Shell;

    if carapace {
        let yaml = r#"name: mossambi
description: Zero-loss native desktop file converter, archive suite, and Indian exam form tuner
flags:
  -a, --all: Batch convert to ALL valid formats in domain
  -o, --output=: Destination directory for output files
  --strict: Enforce strict 0.00% zero-loss transcode
  --json: Output results in JSON format
  --exam=: Indian Competitive Exam preset (e.g. upsc-photo, ssc-photo, ibps-signature)
  --width=: Exam custom target width in pixels
  --height=: Exam custom target height in pixels
  --dpi=: Exam custom target DPI
  --min-kb=: Exam minimum file size in kilobytes
  --max-kb=: Exam maximum file size in kilobytes
  --whiten: Auto-whiten background of scanned documents or signatures
  --app: Launch the full Universal File Explorer & Conversion Studio GUI
  --gui: Launch the full Universal File Explorer & Conversion Studio GUI
  --widget: Launch the Sweet Lime Desktop Radial Widget
  --tui: Launch the Terminal User Interface (TUI) File Explorer
  --banner: Display the MOSSAMBI banner and summary
  --manual: Display the Unicode box table manual
  --hw: Display detected hardware acceleration engines
  --themes: List all 26 named Pantone themes
  --check=: 4-dimension Quality & Container Integrity Scorecard
  --bench=: Benchmark conversion latency and throughput
  --verify=: Verify mathematical zero-loss fidelity between two files
  --catalogue=: Search across the 200+ format registry catalog
  --archive=: Pack files or directories into an archive
  --extract=: Extract an archive with automatic Zip-Slip protection
  --daemon=: Manage background resident daemon (start, stop, status)
commands:
  - name: app
    description: Launch Universal File Explorer & Conversion Studio GUI
  - name: explorer
    description: Launch Universal File Explorer & Conversion Studio GUI
  - name: tui
    description: Launch the Terminal User Interface (TUI) File Explorer
  - name: widget
    description: Launch the Sweet Lime Desktop Radial Widget
  - name: convert
    description: Convert an input file to target format
  - name: archive
    description: Pack files or directories into an archive
  - name: extract
    description: Extract an archive with Zip-Slip protection
  - name: info
    description: Inspect file metadata and detected category
  - name: check
    description: Check quality, container integrity, and decoder parseability
  - name: bench
    description: Benchmark conversion latency and throughput
  - name: hw
    description: Display detected GPU/NPU hardware acceleration engines
  - name: verify
    description: Verify mathematical zero-loss fidelity between two files
  - name: catalogue
    description: Search across the 200+ format registry catalog
  - name: upload-ready
    description: Prepare image/document for Indian competitive exam portals
  - name: themes
    description: List all 26 named Pantone themes with codes and hex values
  - name: config
    description: View or edit persistent configuration settings
  - name: completions
    description: Generate shell auto-completion script or export Carapace YAML spec
  - name: help-manual
    description: Display Unicode box table help and usage manual
  - name: banner
    description: Display MOSSAMBI sweet lime fruit ASCII art and banner
  - name: daemon
    description: Manage background resident daemon and global hotkeys
  - name: fs
    description: Filesystem operations (copy, move, rename, delete)
  - name: tags
    description: File tagging engine (list, add, remove, find)
  - name: queue
    description: Conversion queue control (add, list, run)
  - name: open-with
    description: Open With handlers (list, run, custom-list, custom-run)
  - name: preview
    description: Print text preview and metadata for a file
  - name: crop
    description: Crop an image to a normalized region
  - name: scripts
    description: List or run user scripts (Rhai / Lua)
  - name: route
    description: Show Dijkstra conversion route between two formats
  - name: search
    description: Fuzzy filename search under a root directory
  - name: batch-rename
    description: Pattern-based batch rename with preview
  - name: undo
    description: Undo the last journaled action
  - name: settings
    description: Configuration fields control (list, get, set)
"#;
        println!("{}", yaml);
        return Ok(());
    }

    let shell = match shell_str.map(|s| s.to_lowercase()).as_deref() {
        Some("bash") => Shell::Bash,
        Some("zsh") => Shell::Zsh,
        Some("fish") => Shell::Fish,
        Some("powershell") | Some("pwsh") => Shell::PowerShell,
        Some("elvish") => Shell::Elvish,
        Some(other) => {
            bail!(
                "Unknown shell '{}'. Supported shells: bash, zsh, fish, powershell, elvish",
                other
            );
        }
        None => detect_shell(),
    };

    let mut cmd = Cli::command();
    if install {
        let mut buf = Vec::new();
        clap_complete::generate(shell, &mut cmd, "mossambi", &mut buf);
        let script = String::from_utf8_lossy(&buf);
        install_completions(shell, &script)?;
    } else {
        clap_complete::generate(shell, &mut cmd, "mossambi", &mut std::io::stdout());
    }
    Ok(())
}

pub fn handle_carapace(_shell: Option<&str>, args: &[String]) -> Result<()> {
    let mut completions = Vec::new();

    // Contextual match 1: Indian competitive exam presets
    let has_exam_flag = args.iter().any(|a| a == "--exam" || a == "--preset")
        || args.last().map(|s| s.as_str()) == Some("--exam")
        || args.last().map(|s| s.as_str()) == Some("--preset");

    if has_exam_flag {
        let presets = [
            (
                "upsc-photo",
                "UPSC Civil Services Photo (350x450, 300 DPI, 20-100 KB)",
            ),
            (
                "upsc-signature",
                "UPSC Civil Services Signature (350x150, 300 DPI, 10-40 KB)",
            ),
            (
                "ssc-photo",
                "SSC CGL/CHSL Photo (140x190, 200 DPI, 20-50 KB)",
            ),
            (
                "ssc-signature",
                "SSC CGL/CHSL Signature (140x60, 200 DPI, 10-20 KB)",
            ),
            (
                "ibps-photo",
                "IBPS Bank PO Photo (200x230, 200 DPI, 20-50 KB)",
            ),
            (
                "ibps-signature",
                "IBPS Bank PO Signature (140x60, 200 DPI, 10-20 KB)",
            ),
            (
                "ibps-document",
                "IBPS Bank PO Document (A4 1654x2338, 200 DPI, 100-500 KB)",
            ),
            (
                "gate-photo",
                "GATE / IIT Photo (240x320, 200 DPI, 20-50 KB)",
            ),
        ];
        for (val, desc) in presets {
            completions.push(json!({
                "value": val,
                "display": val,
                "description": desc
            }));
        }
        println!("{}", serde_json::to_string(&completions)?);
        return Ok(());
    }

    // Contextual match 2: 26 Pantone themes
    let is_theme_context = args
        .iter()
        .any(|a| a == "theme" || a == "theme_mode" || a == "themes");
    if is_theme_context {
        for theme in crate::core::pantone::all_26_pantone_themes() {
            completions.push(json!({
                "value": theme.id,
                "display": theme.name,
                "description": format!(
                    "Pantone: {} ({}) - {}",
                    theme.accent.code,
                    if theme.is_dark { "Dark" } else { "Light" },
                    theme.accent.hex
                )
            }));
        }
        println!("{}", serde_json::to_string(&completions)?);
        return Ok(());
    }

    // Contextual match 3: Route command formats
    if args.first().map(|s| s.as_str()) == Some("route") {
        let formats = [
            "png", "jpg", "jpeg", "webp", "avif", "bmp", "tiff", "ico", "svg", "pdf", "docx",
            "xlsx", "csv", "json", "yaml", "xml", "txt", "md", "mp4", "mkv", "mov", "avi", "webm",
            "flac", "wav", "mp3", "aac", "ogg", "zip", "tar", "tar.gz", "tar.bz2", "tar.xz",
            "tar.zst", "7z",
        ];
        for fmt in formats {
            completions.push(json!({
                "value": fmt,
                "display": fmt.to_uppercase(),
                "description": format!("Format: {}", fmt.to_uppercase())
            }));
        }
        println!("{}", serde_json::to_string(&completions)?);
        return Ok(());
    }

    // Contextual match 4: Target format recommendations for existing file paths
    for arg in args {
        let path = Path::new(arg);
        if path.exists() && path.is_file() {
            let (category, in_ext) = UniversalRegistry::classify_file(path);
            let targets = UniversalRegistry::get_all_supported_outputs(category);
            for tgt in targets {
                if tgt != in_ext.as_str() {
                    completions.push(json!({
                        "value": tgt.to_lowercase(),
                        "display": tgt.to_uppercase(),
                        "description": format!("Convert to {} ({:?})", tgt, category)
                    }));
                }
            }
            println!("{}", serde_json::to_string(&completions)?);
            return Ok(());
        }
    }

    println!("[]");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_shell_returns_valid_shell() {
        let shell = detect_shell();
        let name = format!("{:?}", shell);
        assert!(!name.is_empty());
    }

    #[test]
    fn test_handle_completions_carapace_yaml() {
        let res = handle_completions(None, true, false);
        assert!(res.is_ok());
    }

    #[test]
    fn test_carapace_theme_completions() {
        let args = vec!["config".to_string(), "set".to_string(), "theme".to_string()];
        let res = handle_carapace(None, &args);
        assert!(res.is_ok());
    }

    #[test]
    fn test_carapace_exam_completions() {
        let args = vec!["upload-ready".to_string(), "--preset".to_string()];
        let res = handle_carapace(None, &args);
        assert!(res.is_ok());
    }
}
