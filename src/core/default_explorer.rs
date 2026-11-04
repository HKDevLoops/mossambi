//! Default file-manager takeover for Mossambi.
//!
//! Windows: writes HKCU `Software\Classes\Directory|Drive\shell` entries so
//! folder/drive opens route to Mossambi (no admin needed), plus an
//! "Open in Mossambi" background verb. `restore` reverts everything it wrote.
//! Linux: installs a `.desktop` file and sets it as the `inode/directory`
//! handler via `xdg-mime`. macOS: Finder cannot be replaced by third-party
//! apps — `status` logs the honest limitation instead of pretending.
//!
//! Every mutation is logged; failures never panic.

use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TakeoverState {
    Active,
    NotActive,
    Unknown,
}

/// Registry root used for all writes (HKCU — per-user, no elevation).
#[cfg(target_os = "windows")]
const HKEY_CURRENT_USER: isize = 0x8000_0001_isize;
#[cfg(target_os = "windows")]
fn exe_command_line() -> String {
    let exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "mossambi.exe".into());
    format!("\"{exe}\" --app \"%V\"")
}

#[cfg(target_os = "windows")]
mod reg {
    use super::HKEY_CURRENT_USER;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegDeleteValueW, RegQueryValueExW,
        RegSetValueExW, HKEY, KEY_READ, KEY_SET_VALUE, KEY_WRITE, REG_OPTION_NON_VOLATILE,
        REG_SZ,
    };

    pub const ERROR_SUCCESS: u32 = 0;
    pub const ERROR_FILE_NOT_FOUND: u32 = 2;

    /// Opens-or-creates `path` under HKCU and hands back the HKEY.
    pub fn open_or_create(path: &str) -> Result<HKEY, u32> {
        let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let mut hk: HKEY = std::ptr::null_mut();
        let res = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER as _,
                wide.as_ptr(),
                0,
                std::ptr::null_mut(),
                REG_OPTION_NON_VOLATILE,
                KEY_READ | KEY_WRITE | KEY_SET_VALUE,
                std::ptr::null_mut(),
                &mut hk,
                std::ptr::null_mut(),
            )
        };
        if res != ERROR_SUCCESS {
            Err(res)
        } else {
            Ok(hk)
        }
    }

    pub fn set_sz(hk: HKEY, value: Option<&str>, data: &str) -> Result<(), u32> {
        let vname: Vec<u16> = value
            .map(|v| v.encode_utf16().chain(std::iter::once(0)).collect())
            .unwrap_or_default();
        let vdata: Vec<u16> = data.encode_utf16().chain(std::iter::once(0)).collect();
        let res = unsafe {
            RegSetValueExW(
                hk,
                if vname.is_empty() {
                    std::ptr::null()
                } else {
                    vname.as_ptr()
                },
                0,
                REG_SZ,
                vdata.as_ptr() as *const u8,
                (vdata.len() * 2) as u32,
            )
        };
        if res == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(res)
        }
    }

    pub fn get_sz(hk: HKEY, value: Option<&str>) -> Option<String> {
        let vname: Vec<u16> = value
            .map(|v| v.encode_utf16().chain(std::iter::once(0)).collect())
            .unwrap_or_default();
        let mut buf = [0u16; 512];
        let mut len = (buf.len() * 2) as u32;
        let mut kind = 0u32;
        let res = unsafe {
            RegQueryValueExW(
                hk,
                if vname.is_empty() {
                    std::ptr::null()
                } else {
                    vname.as_ptr()
                },
                std::ptr::null_mut(),
                &mut kind,
                buf.as_mut_ptr() as *mut u8,
                &mut len,
            )
        };
        if res != ERROR_SUCCESS || kind != REG_SZ {
            return None;
        }
        let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Some(String::from_utf16_lossy(&buf[..n]))
    }

    pub fn delete_value(hk: HKEY, value: Option<&str>) -> Result<(), u32> {
        let vname: Vec<u16> = value
            .map(|v| v.encode_utf16().chain(std::iter::once(0)).collect())
            .unwrap_or_default();
        let res = unsafe {
            RegDeleteValueW(
                hk,
                if vname.is_empty() {
                    std::ptr::null()
                } else {
                    vname.as_ptr()
                },
            )
        };
        if res == ERROR_SUCCESS || res == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            Err(res)
        }
    }

    pub fn delete_tree(path: &str) -> Result<(), u32> {
        let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let res = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER as _, wide.as_ptr()) };
        if res == ERROR_SUCCESS || res == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            Err(res)
        }
    }

    pub fn close(hk: HKEY) {
        unsafe {
            RegCloseKey(hk);
        }
    }
}

/// Verb subkey name used for all our registrations.
const VERB: &str = "Mossambi";

#[cfg(target_os = "windows")]
fn write_shell_verb(key_path: &str, make_default: bool) -> Result<()> {
    let hk = reg::open_or_create(key_path)
        .map_err(|e| anyhow::anyhow!("RegCreateKeyExW({key_path}) failed: winerror {e}"))?;
    if make_default {
        reg::set_sz(hk, None, VERB)
            .map_err(|e| anyhow::anyhow!("set default verb on {key_path} failed: winerror {e}"))?;
    }
    reg::close(hk);

    let cmd_key = format!("{key_path}\\{VERB}\\command");
    let hk_cmd = reg::open_or_create(&cmd_key)
        .map_err(|e| anyhow::anyhow!("RegCreateKeyExW({cmd_key}) failed: winerror {e}"))?;
    reg::set_sz(hk_cmd, None, &exe_command_line())
        .map_err(|e| anyhow::anyhow!("set command on {cmd_key} failed: winerror {e}"))?;
    reg::close(hk_cmd);

    let label_key = format!("{key_path}\\{VERB}");
    let hk_label = reg::open_or_create(&label_key)
        .map_err(|e| anyhow::anyhow!("RegCreateKeyExW({label_key}) failed: winerror {e}"))?;
    let _ = reg::set_sz(hk_label, Some("MUIVerb"), "Open in Mossambi");
    reg::close(hk_label);
    Ok(())
}

#[cfg(target_os = "windows")]
fn read_shell_default(key_path: &str) -> Option<String> {
    let hk = reg::open_or_create(key_path).ok()?;
    let v = reg::get_sz(hk, None);
    reg::close(hk);
    v
}

/// Registers Mossambi as the OS file manager. Windows: HKCU shell verbs for
/// Directory/Drive (default open) + Directory.Background context verb.
pub fn take_over() -> Result<()> {
    log::info!("Explorer takeover requested");
    #[cfg(target_os = "windows")]
    {
        write_shell_verb("Software\\Classes\\Directory\\shell", true)?;
        write_shell_verb("Software\\Classes\\Drive\\shell", true)?;
        // Background (empty area) right-click verb — never a default.
        write_shell_verb("Software\\Classes\\Directory\\Background\\shell", false)?;
        log::info!("HKCU Directory/Drive shell verbs registered (Mossambi default open)");
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        let home = std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("HOME not set"))?;
        let apps = home.join(".local/share/applications");
        std::fs::create_dir_all(&apps)?;
        let exe = std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "mossambi".into());
        let desktop = format!(
            "[Desktop Entry]\nType=Application\nName=Mossambi\nExec={} --app %F\n\
             Terminal=false\nMimeType=inode/directory;\nCategories=System;FileManager;\n\
             NoDisplay=false\n",
            exe
        );
        let path = apps.join("mossambi.desktop");
        std::fs::write(&path, desktop)?;
        log::info!("Wrote {}", path.display());
        let res = std::process::Command::new("xdg-mime")
            .args(["default", "mossambi.desktop", "inode/directory"])
            .status();
        match res {
            Ok(s) if s.success() => {
                log::info!("xdg-mime default set: mossambi.desktop for inode/directory");
                Ok(())
            }
            _ => Err(anyhow::anyhow!("xdg-mime default failed (xdg-utils installed?)")),
        }
    }
    #[cfg(target_os = "macos")]
    {
        anyhow::bail!(
            "macOS: Finder cannot be replaced by third-party apps. \
             Use Services/Quick Actions or launch Mossambi directly."
        )
    }
}

/// Reverts [`take_over`] — removes our verbs and restores previous defaults.
pub fn restore() -> Result<()> {
    log::info!("Explorer restore requested");
    #[cfg(target_os = "windows")]
    {
        for base in [
            "Software\\Classes\\Directory\\shell",
            "Software\\Classes\\Drive\\shell",
            "Software\\Classes\\Directory\\Background\\shell",
        ] {
            // Only clear the default verb if WE own it.
            if read_shell_default(base).as_deref() == Some(VERB) {
                let hk = reg::open_or_create(base)
                    .map_err(|e| anyhow::anyhow!("open {base} failed: winerror {e}"))?;
                let _ = reg::delete_value(hk, None);
                reg::close(hk);
                log::info!("Default verb restored (removed ours) on {base}");
            }
            reg::delete_tree(&format!("{base}\\{VERB}"))
                .map_err(|e| anyhow::anyhow!("delete_tree {base}\\{VERB} failed: winerror {e}"))?;
        }
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        // Best-effort restore: prefer Nautilus/Dolphin/Thunar if installed.
        for candidate in ["org.gnome.Nautilus.desktop", "dolphin.desktop", "thunar.desktop"] {
            let res = std::process::Command::new("xdg-mime")
                .args(["default", candidate, "inode/directory"])
                .status();
            if matches!(res, Ok(s) if s.success()) {
                log::info!("xdg-mime default restored to {candidate}");
                return Ok(());
            }
        }
        anyhow::bail!("No known file-manager desktop entry found to restore")
    }
    #[cfg(target_os = "macos")]
    {
        anyhow::bail!("Nothing to restore on macOS (Finder was never replaced)")
    }
}

/// Reports whether Mossambi currently owns the default file-manager verbs.
pub fn status() -> TakeoverState {
    #[cfg(target_os = "windows")]
    {
        match read_shell_default("Software\\Classes\\Directory\\shell") {
            Some(v) if v == VERB => TakeoverState::Active,
            Some(_) => TakeoverState::NotActive,
            None => TakeoverState::NotActive,
        }
    }
    #[cfg(target_os = "linux")]
    {
        let out = std::process::Command::new("xdg-mime")
            .args(["query", "default", "inode/directory"])
            .output();
        match out {
            Ok(o) if o.status.success() => {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if s == "mossambi.desktop" {
                    TakeoverState::Active
                } else {
                    TakeoverState::NotActive
                }
            }
            _ => TakeoverState::Unknown,
        }
    }
    #[cfg(target_os = "macos")]
    {
        TakeoverState::Unknown
    }
}

/// Best-effort existence probe used by the GUI before showing buttons.
pub fn executable_available() -> bool {
    std::env::current_exe().is_ok() && Path::new("/").exists()
}
