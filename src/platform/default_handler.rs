//! Opt-in "Use Mossambi as the default file manager" shell integration.
//!
//! Pure 7-bit ASCII. Windows: writes HKCU shell verbs under a configurable
//! registry root (production `Software\Classes`, tests
//! `Software\MossambiTest\Classes`) so folder/drive double-clicks route to
//! Mossambi, with a reversible backup of the previous default verb. Every
//! operation is idempotent and never touches keys outside the given root.
//!
//! On non-Windows platforms all operations are no-ops (`Ok(())` / `false`),
//! mirroring the platform::notify pattern.

use anyhow::Result;
use std::sync::Mutex;

/// Last error from the production enable/disable path, surfaced inline in
/// Preferences until the next attempt.
static LAST_ERROR: Mutex<Option<String>> = Mutex::new(None);

/// Returns the last error produced by `set_default_folder_handler`, if any.
pub fn last_error() -> Option<String> {
    LAST_ERROR.lock().ok().and_then(|guard| guard.clone())
}

/// Clears the stored inline error.
pub fn clear_last_error() {
    if let Ok(mut guard) = LAST_ERROR.lock() {
        *guard = None;
    }
}

/// Registry root type: base key path under HKCU (e.g. `Software\Classes`).
pub type RegRoot = String;

/// Production root: the per-user file-association classes key.
pub const PROD_ROOT: &str = "Software\\Classes";

/// Test root: an isolated classes key the test suites create and delete.
pub const TEST_ROOT: &str = "Software\\MossambiTest\\Classes";

/// Object classes that receive the default-open verb.
pub const SHELL_CLASSES: &[&str] = &["Directory", "Drive", "Folder"];

/// Backup value name stored on each `<class>\shell` key.
pub const BACKUP_VALUE: &str = "MossambiPrevDefault";

/// Marker stored in the backup when no previous default verb existed.
pub const NO_PREV_DEFAULT: &str = "<none>";

/// Registers the "Open in Mossambi" verb and makes it the default open
/// action for Directory/Drive/Folder. Idempotent: calling enable() again
/// never clobbers the backup with our own verb.
pub fn enable(root: &RegRoot) -> Result<()> {
    #[cfg(windows)]
    {
        win::enable(root)
    }
    #[cfg(not(windows))]
    {
        let _ = root;
        log::debug!("default_handler: not supported on this platform");
        Ok(())
    }
}

/// Restores the previous default verbs, removes the "Open in Mossambi"
/// subtree and the backup values. Idempotent and safe to call when the
/// integration was never enabled.
pub fn disable(root: &RegRoot) -> Result<()> {
    #[cfg(windows)]
    {
        win::disable(root)
    }
    #[cfg(not(windows))]
    {
        let _ = root;
        log::debug!("default_handler: not supported on this platform");
        Ok(())
    }
}

/// True when every tracked class currently has `mossambi` as its default verb.
pub fn is_enabled(root: &RegRoot) -> bool {
    #[cfg(windows)]
    {
        win::is_enabled(root)
    }
    #[cfg(not(windows))]
    {
        let _ = root;
        false
    }
}

/// Sets or clears Mossambi as default folder handler using the live HKCU\Software\Classes path.
/// Clears the inline error on entry and records it on failure so the
/// Preferences toggle can display it.
pub fn set_default_folder_handler(enabled: bool) -> Result<()> {
    clear_last_error();
    let root = PROD_ROOT.to_string();
    let res = if enabled {
        enable(&root)
    } else {
        disable(&root)
    };
    if let Err(e) = &res {
        if let Ok(mut guard) = LAST_ERROR.lock() {
            *guard = Some(format!("{e:#}"));
        }
    }
    res
}

/// Applies or removes default handler setting with customizable root (for isolated tests).
pub fn set_default_folder_handler_with_root(
    root: &str,
    _backup: &str,
    enabled: bool,
    cmd: &str,
) -> Result<()> {
    let root_s = root.to_string();
    #[cfg(windows)]
    {
        if enabled {
            win::enable_with_cmd(&root_s, Some(cmd))
        } else {
            win::disable(&root_s)
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (root_s, enabled, cmd);
        Ok(())
    }
}

/// Queries whether default handler is active under the given root.
pub fn is_default_folder_handler_active(root: &str) -> bool {
    is_enabled(&root.to_string())
}

/// Documentation on Windows Shell remapping constraints.
pub fn get_shell_remapping_notes() -> &'static str {
    "Win+E is registered internally by explorer.exe and cannot be overridden by HKCU alone. \
     Folder navigation via ShellExecute and Folder\\shell routes to Mossambi. \
     Mossambi CLI parses '/select,<path>' to support browser 'Show in folder' invocations."
}

/// Command line stored on `<class>\shell\mossambi\command`: launch this
/// executable with the clicked path.
pub fn verb_command_line() -> String {
    let exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "mossambi.exe".into());
    format!("\"{}\" \"%1\"", exe)
}

#[cfg(windows)]
mod win {
    use super::{verb_command_line, BACKUP_VALUE, NO_PREV_DEFAULT, SHELL_CLASSES};
    use anyhow::Result;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegDeleteValueW, RegOpenKeyExW,
        RegQueryValueExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE,
        KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ,
    };

    const ERROR_SUCCESS: u32 = 0;
    const ERROR_FILE_NOT_FOUND: u32 = 2;

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn open_or_create(path: &str) -> Result<HKEY, u32> {
        let wide = to_wide(path);
        let mut hk: HKEY = std::ptr::null_mut();
        let res = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                wide.as_ptr(),
                0,
                std::ptr::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_READ | KEY_WRITE | KEY_SET_VALUE,
                std::ptr::null(),
                &mut hk,
                std::ptr::null_mut(),
            )
        };
        if res == ERROR_SUCCESS {
            Ok(hk)
        } else {
            Err(res)
        }
    }

    /// Read-only open: returns `None` when the key does not exist, so
    /// probes never create stray keys.
    fn open_read(path: &str) -> Option<HKEY> {
        let wide = to_wide(path);
        let mut hk: HKEY = std::ptr::null_mut();
        let res = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, wide.as_ptr(), 0, KEY_READ, &mut hk) };
        if res == ERROR_SUCCESS {
            Some(hk)
        } else {
            None
        }
    }

    fn set_sz(hk: HKEY, value: Option<&str>, data: &str) -> Result<(), u32> {
        let vname = value.map(to_wide).unwrap_or_default();
        let mut vdata = data.encode_utf16().collect::<Vec<u16>>();
        vdata.push(0);
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

    /// Reads a REG_SZ value (the key's default when `value` is `None`).
    fn get_sz(hk: HKEY, value: Option<&str>) -> Option<String> {
        let vname = value.map(to_wide).unwrap_or_default();
        let mut buf = [0u16; 1024];
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

    fn delete_value(hk: HKEY, value: Option<&str>) -> Result<(), u32> {
        let vname = value.map(to_wide).unwrap_or_default();
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

    fn close(hk: HKEY) {
        unsafe {
            RegCloseKey(hk);
        }
    }

    fn delete_tree(path: &str) -> Result<(), u32> {
        let wide = to_wide(path);
        let res = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, wide.as_ptr()) };
        if res == ERROR_SUCCESS || res == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            Err(res)
        }
    }

    fn shell_key(root: &str, class: &str) -> String {
        format!("{}\\{}\\shell", root, class)
    }

    /// Reads the current default verb of `<class>\shell` without creating
    /// the key when it is missing.
    fn read_shell_default(key_path: &str) -> Option<String> {
        let hk = open_read(key_path)?;
        let v = get_sz(hk, None);
        close(hk);
        v
    }

    pub(super) fn enable(root: &str) -> Result<()> {
        enable_with_cmd(root, None)
    }

    pub(super) fn enable_with_cmd(root: &str, cmd_override: Option<&str>) -> Result<()> {
        let command = cmd_override
            .map(|s| s.to_string())
            .unwrap_or_else(verb_command_line);
        for class in SHELL_CLASSES {
            let shell = shell_key(root, class);
            // Back up the previous default verb, but only when we do not
            // already own it: a second enable() must never overwrite the
            // saved previous default with "mossambi" itself.
            let current = read_shell_default(&shell);
            if current.as_deref() != Some("mossambi") {
                let hk = open_or_create(&shell)
                    .map_err(|e| anyhow::anyhow!("open {shell} failed: winerror {e}"))?;
                let prev = current
                    .clone()
                    .unwrap_or_else(|| NO_PREV_DEFAULT.to_string());
                set_sz(hk, Some(BACKUP_VALUE), &prev)
                    .map_err(|e| anyhow::anyhow!("backup {shell} failed: winerror {e}"))?;
                close(hk);
            }

            // Verb key: friendly label on the default value.
            let verb_key = format!("{shell}\\mossambi");
            let hk = open_or_create(&verb_key)
                .map_err(|e| anyhow::anyhow!("open {verb_key} failed: winerror {e}"))?;
            set_sz(hk, None, "Open in Mossambi")
                .map_err(|e| anyhow::anyhow!("label {verb_key} failed: winerror {e}"))?;
            close(hk);

            // Command key: launch this executable with the clicked path.
            let cmd_key = format!("{verb_key}\\command");
            let hk = open_or_create(&cmd_key)
                .map_err(|e| anyhow::anyhow!("open {cmd_key} failed: winerror {e}"))?;
            set_sz(hk, None, &command)
                .map_err(|e| anyhow::anyhow!("command {cmd_key} failed: winerror {e}"))?;
            close(hk);

            // Make our verb the default open action.
            let hk = open_or_create(&shell)
                .map_err(|e| anyhow::anyhow!("open {shell} failed: winerror {e}"))?;
            set_sz(hk, None, "mossambi")
                .map_err(|e| anyhow::anyhow!("default {shell} failed: winerror {e}"))?;
            close(hk);
            log::info!("default_handler: {shell} default verb -> mossambi");
        }

        // Support Folder\shell\opennewwindow\command
        let folder_shell = shell_key(root, "Folder");
        let onw_cmd_path = format!("{folder_shell}\\opennewwindow\\command");
        let prev_onw = open_read(&onw_cmd_path).and_then(|hk| {
            let v = get_sz(hk, None);
            close(hk);
            v
        });
        if let Some(prev) = prev_onw {
            if prev != command {
                let hk_fs = open_or_create(&folder_shell)
                    .map_err(|e| anyhow::anyhow!("open {folder_shell} failed: winerror {e}"))?;
                set_sz(hk_fs, Some("MossambiPrevOpenNewWindow"), &prev)
                    .map_err(|e| anyhow::anyhow!("backup onw failed: winerror {e}"))?;
                close(hk_fs);

                let hk_onw = open_or_create(&onw_cmd_path)
                    .map_err(|e| anyhow::anyhow!("open {onw_cmd_path} failed: winerror {e}"))?;
                set_sz(hk_onw, None, &command)
                    .map_err(|e| anyhow::anyhow!("command {onw_cmd_path} failed: winerror {e}"))?;
                close(hk_onw);
            }
        }

        Ok(())
    }

    pub(super) fn disable(root: &str) -> Result<()> {
        for class in SHELL_CLASSES {
            let shell = shell_key(root, class);
            // Restore the previous default from the backup value (read
            // without creating the key when the integration never ran).
            let backup = open_read(&shell).and_then(|hk| {
                let v = get_sz(hk, Some(BACKUP_VALUE));
                close(hk);
                v
            });
            if let Some(prev) = backup {
                let hk = open_or_create(&shell)
                    .map_err(|e| anyhow::anyhow!("open {shell} failed: winerror {e}"))?;
                if prev == NO_PREV_DEFAULT {
                    // Nothing was the default before: clear ours.
                    let _ = delete_value(hk, None);
                } else {
                    let _ = set_sz(hk, None, &prev);
                }
                // The backup has served its purpose.
                let _ = delete_value(hk, Some(BACKUP_VALUE));
                close(hk);
            } else if read_shell_default(&shell).as_deref() == Some("mossambi") {
                // No backup (never enabled through enable()): at least stop
                // owning the default verb.
                let hk = open_or_create(&shell)
                    .map_err(|e| anyhow::anyhow!("open {shell} failed: winerror {e}"))?;
                let _ = delete_value(hk, None);
                close(hk);
            }

            // Remove our verb subtree.
            delete_tree(&format!("{shell}\\mossambi")).map_err(|e| {
                anyhow::anyhow!("delete_tree {shell}\\mossambi failed: winerror {e}")
            })?;
            log::info!("default_handler: {shell} verb removed");
        }

        // Restore Folder\shell\opennewwindow\command if backed up
        let folder_shell = shell_key(root, "Folder");
        let backup_onw = open_read(&folder_shell).and_then(|hk| {
            let v = get_sz(hk, Some("MossambiPrevOpenNewWindow"));
            close(hk);
            v
        });
        if let Some(prev) = backup_onw {
            let onw_cmd_path = format!("{folder_shell}\\opennewwindow\\command");
            if let Ok(hk) = open_or_create(&onw_cmd_path) {
                let _ = set_sz(hk, None, &prev);
                close(hk);
            }
            if let Ok(hk) = open_or_create(&folder_shell) {
                let _ = delete_value(hk, Some("MossambiPrevOpenNewWindow"));
                close(hk);
            }
        }

        Ok(())
    }

    pub(super) fn is_enabled(root: &str) -> bool {
        SHELL_CLASSES
            .iter()
            .all(|class| read_shell_default(&shell_key(root, class)).as_deref() == Some("mossambi"))
    }
}
