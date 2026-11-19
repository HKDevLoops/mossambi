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

/// Generate shell completions via clap_complete, or a Carapace YAML spec.
pub fn handle_completions(shell_str: Option<&str>, carapace: bool) -> Result<()> {
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
  --exam=: Indian Competitive Exam preset
  --width=: Exam custom target width in pixels
  --height=: Exam custom target height in pixels
  --dpi=: Exam custom target DPI
  --min-kb=: Exam minimum file size in kilobytes
  --max-kb=: Exam maximum file size in kilobytes
  --whiten: Auto-whiten background of scanned documents or signatures
commands:
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
  - name: tui
    description: Launch the interactive Terminal File Explorer
  - name: gui
    description: Launch the Desktop Radial Widget and macOS-Styled GUI
  - name: verify
    description: Verify mathematical zero-loss fidelity between two files
  - name: upload-ready
    description: Prepare image/document for Indian competitive exam portals
  - name: fs
    description: Filesystem operations (copy, move, rename, delete-to-trash)
  - name: tags
    description: File tagging engine (list, add, remove, find)
  - name: queue
    description: Conversion queue control (add, list, run)
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
        _ => {
            bail!("Usage: mossambi completions [bash|zsh|fish|powershell|elvish]");
        }
    };

    let mut cmd = Cli::command();
    clap_complete::generate(shell, &mut cmd, "mossambi", &mut std::io::stdout());
    Ok(())
}

pub fn handle_carapace(_shell: Option<&str>, args: &[String]) -> Result<()> {
    if let Some(first_arg) = args.first() {
        let path = Path::new(first_arg);
        if path.exists() {
            let (category, in_ext) = UniversalRegistry::classify_file(path);
            let targets = UniversalRegistry::get_all_supported_outputs(category);
            let mut completions = Vec::new();
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
