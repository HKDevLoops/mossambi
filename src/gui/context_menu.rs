//! Windows 11 Explorer-parity context menu verbs.
//!
//! These wrap native Windows shell actions (verbs, clipboard, wallpaper,
//! Defender scan...) so Mossambi's right-click flyouts match what Explorer shows.
//! No external crates required - only Win32 via windows-sys.

use crate::core::config::CustomMenuItem;
#[cfg(target_os = "windows")]
use crate::core::config::CustomMenuKind;
use std::path::Path;

/// Whether the Defender scan verb can execute (Windows only).
pub fn win_defender_available() -> bool {
    cfg!(target_os = "windows")
}

/// Execute a registered shell verb on `path` (e.g. "open", "print", "play", "edit")
pub fn shell_verb(path: &Path, verb: &str) -> bool {
    #[cfg(target_os = "windows")]
    unsafe {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let action: Vec<u16> = OsStr::new(verb)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let res = ShellExecuteW(
            std::ptr::null_mut(),
            action.as_ptr(),
            wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
        // returns HINSTANCE > 32 on success
        (res as usize) > 32
    }
    #[cfg(not(target_os = "windows"))]
    {
        // xdg-open covers the "open" verb only; other verbs (print, play,
        // everything custom) intentionally report unsupported on Linux.
        let _ = verb;
        use std::process::Command;
        Command::new("xdg-open").arg(path).spawn().is_ok()
    }
}

/// Open the system "How do you want to open this file?" (openas)
pub fn open_with_choose(path: &Path) -> bool {
    #[cfg(target_os = "windows")]
    unsafe {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        // The openas verb is the modern Win10+ route to the "How do you
        // want to open this file?" picker and is tried first: the classic
        // rundll32 OpenAs_RunDLL spawn below returns success on Win11 but
        // exits without showing any dialog (verified live via pid delta).
        if shell_verb(path, "openas") {
            return true;
        }
        // rundll32 shell32.dll,OpenAs_RunDLL <file> is the classic fallback
        let args = format!("shell32.dll,OpenAs_RunDLL \"{}\"", path.to_string_lossy());
        let wide: Vec<u16> = std::ffi::OsStr::new("rundll32")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let wide_args: Vec<u16> = std::ffi::OsStr::new(&args)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let raw = wide_args.as_ptr();
        let res = ShellExecuteW(
            std::ptr::null_mut(),
            std::ptr::null(),
            wide.as_ptr(),
            raw,
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
        // rundll32 OpenAs_RunDLL is only a fallback: if it spawned but the
        // dialog never appeared, report failure.
        if (res as usize) > 32 {
            return true;
        }
        false
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn().is_ok()
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .is_ok()
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = path;
        false
    }
}

/// One enumerated "Open with" application: (display label, raw
/// shell\open\command line). Mirrors send_to_targets' tuple style.
/// Static-registry route only (research decision D-1) - read-only,
/// no COM, capped at 8, default app first.
pub fn open_with_candidates(path: &Path) -> Vec<(String, String)> {
    #[cfg(target_os = "windows")]
    {
        open_with::candidates(path)
            .into_iter()
            .map(|c| (c.label, c.command))
            .collect()
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        Vec::new()
    }
}

/// Launch `path` with an enumerated Open With command line: %1 / %L are
/// substituted with the quoted path and environment strings expanded
/// before CreateProcessW.
pub fn launch_open_with_cmd(command: &str, path: &Path) -> bool {
    #[cfg(target_os = "windows")]
    {
        open_with::launch(command, path)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (command, path);
        false
    }
}

#[cfg(target_os = "windows")]
mod open_with {
    //! Static-registry Open With enumeration (research D-1).
    //!
    //! Sources, in Explorer's priority order:
    //! 1. HKCU FileExts\<ext>\UserChoice "Progid" (the default app)
    //! 2. HKCU FileExts\<ext>\OpenWithProgids value names
    //! 3. HKCR <.ext>\OpenWithProgids value names
    //! 4. Bounded HKCR\Applications\* scan via SupportedTypes
    //!
    //! Reading only - never writes, never touches UserChoice.

    use std::path::Path;
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegEnumValueW, RegOpenKeyExW, RegQueryValueExW, HKEY, KEY_READ,
    };
    use windows_sys::Win32::System::Threading::{
        CreateProcessW, PROCESS_INFORMATION, STARTUPINFOW,
    };

    // Root handles defined locally (same idiom as core::default_explorer)
    // so we never depend on windows-sys re-exporting them.
    const HKEY_CLASSES_ROOT: isize = 0x8000_0000_isize;
    const HKCU: isize = 0x8000_0001_isize;
    const ERROR_SUCCESS: u32 = 0;
    // Registry value type codes (avoid importing half the feature).
    const REG_SZ: u32 = 1;
    const REG_EXPAND_SZ: u32 = 2;
    const MAX_CANDIDATES: usize = 8;

    /// One resolved candidate.
    pub struct Candidate {
        pub label: String,
        pub command: String,
    }

    fn open_key_read(root: isize, subkey: &str) -> Option<HKEY> {
        let wide: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
        let mut hk: HKEY = std::ptr::null_mut();
        let res = unsafe { RegOpenKeyExW(root as _, wide.as_ptr(), 0, KEY_READ, &mut hk) };
        if res == ERROR_SUCCESS {
            Some(hk)
        } else {
            None
        }
    }

    fn close(hk: HKEY) {
        unsafe { RegCloseKey(hk) };
    }

    /// Reads a string value accepting both REG_SZ and REG_EXPAND_SZ.
    fn get_string(hk: HKEY, value: Option<&str>) -> Option<String> {
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
        if res != ERROR_SUCCESS || (kind != REG_SZ && kind != REG_EXPAND_SZ) {
            return None;
        }
        let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Some(String::from_utf16_lossy(&buf[..n]))
    }

    /// Enumerates VALUE names under `hk`, bounded by `cap`.
    fn enum_value_names(hk: HKEY, cap: usize) -> Vec<String> {
        let mut out = Vec::new();
        for idx in 0..cap as u32 {
            let mut name = [0u16; 128];
            let mut name_len = name.len() as u32;
            let mut kind = 0u32;
            let mut data = [0u8; 8];
            let mut data_len = data.len() as u32;
            let res = unsafe {
                RegEnumValueW(
                    hk,
                    idx,
                    name.as_mut_ptr(),
                    &mut name_len,
                    std::ptr::null_mut(),
                    &mut kind,
                    data.as_mut_ptr(),
                    &mut data_len,
                )
            };
            if res != ERROR_SUCCESS {
                break;
            }
            out.push(String::from_utf16_lossy(&name[..name_len as usize]));
        }
        out
    }

    /// Enumerates SUBKEY names under `hk`, bounded by `cap`.
    fn enum_subkeys(hk: HKEY, cap: usize) -> Vec<String> {
        let mut out = Vec::new();
        for idx in 0..cap as u32 {
            let mut name = [0u16; 128];
            let mut name_len = name.len() as u32;
            let res = unsafe {
                RegEnumKeyExW(
                    hk,
                    idx,
                    name.as_mut_ptr(),
                    &mut name_len,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            if res != ERROR_SUCCESS {
                break;
            }
            out.push(String::from_utf16_lossy(&name[..name_len as usize]));
        }
        out
    }

    fn push_unique(progids: &mut Vec<String>, p: String) {
        if !p.is_empty() && !progids.iter().any(|x| x.eq_ignore_ascii_case(&p)) {
            progids.push(p);
        }
    }

    /// Best-effort display label from a command line's executable basename.
    /// ALL-CAPS stems are title-cased (NOTEPAD -> Notepad) for menu rows.
    fn exe_label_from_command(command: &str) -> Option<String> {
        let s = command.trim();
        let exe = if let Some(rest) = s.strip_prefix('"') {
            rest.split('"').next()?.to_string()
        } else {
            let no_args = s.split('%').next().unwrap_or(s).trim_end();
            match no_args.find(' ') {
                Some(i) => no_args[..i].to_string(),
                None => no_args.to_string(),
            }
        };
        let stem = Path::new(&exe).file_stem()?.to_string_lossy().to_string();
        if stem.is_empty() {
            return None;
        }
        let has_alpha = stem.chars().any(|c| c.is_ascii_alphabetic());
        let all_caps = stem
            .chars()
            .all(|c| !c.is_ascii_alphabetic() || c.is_ascii_uppercase());
        if has_alpha && all_caps {
            let lower = stem.to_ascii_lowercase();
            let mut cs = lower.chars();
            match cs.next() {
                Some(f) => Some(f.to_ascii_uppercase().to_string() + cs.as_str()),
                None => Some(stem),
            }
        } else {
            Some(stem)
        }
    }

    /// Expands %VAR% via the process environment (REG_EXPAND_SZ values).
    /// Unknown variables are left verbatim.
    pub(super) fn expand_env_vars(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut rest = s;
        while let Some(i) = rest.find('%') {
            out.push_str(&rest[..i]);
            let after = &rest[i + 1..];
            match after.find('%') {
                Some(j) => {
                    match std::env::var(&after[..j]) {
                        Ok(v) => out.push_str(&v),
                        Err(_) => out.push_str(&format!("%{}%", &after[..j])),
                    }
                    rest = &after[j + 1..];
                }
                None => {
                    out.push('%');
                    rest = after;
                }
            }
        }
        out.push_str(rest);
        out
    }

    pub fn candidates(path: &Path) -> Vec<Candidate> {
        let ext = path
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy().to_ascii_lowercase()))
            .unwrap_or_default();
        if ext.is_empty() {
            return Vec::new();
        }

        // 1) ProgIDs in priority order (default first, deduped).
        let mut progids: Vec<String> = Vec::new();
        let uc = format!(
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\{}\UserChoice",
            ext
        );
        if let Some(hk) = open_key_read(HKCU, &uc) {
            if let Some(p) = get_string(hk, Some("Progid")) {
                push_unique(&mut progids, p);
            }
            close(hk);
        }
        let owp = format!(
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\{}\OpenWithProgids",
            ext
        );
        if let Some(hk) = open_key_read(HKCU, &owp) {
            for name in enum_value_names(hk, 16) {
                push_unique(&mut progids, name);
            }
            close(hk);
        }
        let hkcr_owp = format!("{}\\OpenWithProgids", ext);
        if let Some(hk) = open_key_read(HKEY_CLASSES_ROOT, &hkcr_owp) {
            for name in enum_value_names(hk, 16) {
                push_unique(&mut progids, name);
            }
            close(hk);
        }

        // 2) Resolve ProgIDs to label + command (cap 8).
        let mut out: Vec<Candidate> = Vec::new();
        for progid in &progids {
            if out.len() >= MAX_CANDIDATES {
                break;
            }
            let cmd_key = format!("{}\\shell\\open\\command", progid);
            let command = open_key_read(HKEY_CLASSES_ROOT, &cmd_key).and_then(|hk| {
                let c = get_string(hk, None);
                close(hk);
                c
            });
            let Some(command) = command.filter(|c| !c.trim().is_empty()) else {
                continue;
            };

            let mut label = open_key_read(HKEY_CLASSES_ROOT, progid).and_then(|hk| {
                let l = get_string(hk, Some("FriendlyTypeName"));
                close(hk);
                l
            });
            // Indirect strings (@dll,-res) are not resolved in v1: fall
            // through to the executable basename from the command line.
            if label.as_deref().map(|l| l.starts_with('@')).unwrap_or(true) {
                label = None;
            }
            let label = label
                .filter(|l| !l.trim().is_empty())
                .or_else(|| exe_label_from_command(&command))
                .unwrap_or_else(|| progid.clone());
            out.push(Candidate { label, command });
        }

        // 3) Supplement with HKCR\Applications whose SupportedTypes lists
        //    the extension (bounded scan, cap 200 subkeys).
        if out.len() < MAX_CANDIDATES {
            if let Some(hk) = open_key_read(HKEY_CLASSES_ROOT, "Applications") {
                for name in enum_subkeys(hk, 200) {
                    if out.len() >= MAX_CANDIDATES {
                        break;
                    }
                    let st_key = format!("{}\\SupportedTypes", name);
                    let hit = open_key_read(HKEY_CLASSES_ROOT, &st_key).map(|sthk| {
                        let hit = enum_value_names(sthk, 64)
                            .iter()
                            .any(|v| v.eq_ignore_ascii_case(&ext));
                        close(sthk);
                        hit
                    });
                    if !hit.unwrap_or(false) {
                        continue;
                    }
                    let cmd_key = format!("{}\\shell\\open\\command", name);
                    let command = open_key_read(HKEY_CLASSES_ROOT, &cmd_key).and_then(|ck| {
                        let c = get_string(ck, None);
                        close(ck);
                        c
                    });
                    let Some(command) = command.filter(|c| !c.trim().is_empty()) else {
                        continue;
                    };
                    if out.iter().any(|c| c.command.eq_ignore_ascii_case(&command)) {
                        continue;
                    }
                    let label = exe_label_from_command(&command).unwrap_or_else(|| name.clone());
                    out.push(Candidate { label, command });
                }
                close(hk);
            }
        }
        out
    }

    pub fn launch(command: &str, path: &Path) -> bool {
        let quoted = format!("\"{}\"", path.to_string_lossy());
        let cmdline = command.replace("%1", &quoted).replace("%L", &quoted);
        let cmdline = if cmdline.contains(&quoted) {
            cmdline
        } else {
            format!("{cmdline} {quoted}")
        };
        let cmdline = expand_env_vars(&cmdline);
        // CreateProcessW may write into the command buffer: keep a mutable copy.
        let mut buf: Vec<u16> = cmdline.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let mut si: STARTUPINFOW = std::mem::zeroed();
            si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
            let mut pi: PROCESS_INFORMATION = std::mem::zeroed();
            let ok = CreateProcessW(
                std::ptr::null(),
                buf.as_mut_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                0,
                std::ptr::null(),
                std::ptr::null(),
                &si,
                &mut pi,
            ) != 0;
            if ok {
                CloseHandle(pi.hProcess);
                CloseHandle(pi.hThread);
            } else {
                log::warn!(
                    target: "mossambi::context_menu",
                    "CreateProcessW failed for Open With command: {}",
                    command
                );
            }
            ok
        }
    }
}

/// Print a document with the associated app (verb "print")
pub fn print_file(path: &Path) -> bool {
    shell_verb(path, "print")
}

/// Play with the associated media player (verb "play")
pub fn play_file(path: &Path) -> bool {
    shell_verb(path, "play")
}

/// Substitute `%p`/`%1` in a template with the quoted target path. The
/// substitution is an empty string when the entry is invoked from the
/// background menu (no target path).
#[cfg(target_os = "windows")]
fn substitute_target(template: &str, path: Option<&Path>) -> String {
    let quoted = path
        .map(|p| format!("\"{}\"", p.to_string_lossy()))
        .unwrap_or_default();
    template.replace("%p", &quoted).replace("%1", &quoted)
}

/// Run a raw command line through CreateProcessW. Unlike
/// [`open_with::launch`] no path is appended automatically: the command
/// line is executed exactly as configured.
#[cfg(target_os = "windows")]
pub fn run_command_line(cmdline: &str) -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        CreateProcessW, PROCESS_INFORMATION, STARTUPINFOW,
    };

    // CreateProcessW may write into the command buffer: keep a mutable copy.
    let mut buf: Vec<u16> = cmdline.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let mut si: STARTUPINFOW = std::mem::zeroed();
        si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        let mut pi: PROCESS_INFORMATION = std::mem::zeroed();
        let ok = CreateProcessW(
            std::ptr::null(),
            buf.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            0,
            std::ptr::null(),
            std::ptr::null(),
            &si,
            &mut pi,
        ) != 0;
        if ok {
            CloseHandle(pi.hProcess);
            CloseHandle(pi.hThread);
        } else {
            log::warn!(
                target: "mossambi::context_menu",
                "CreateProcessW failed for custom command line"
            );
        }
        ok
    }
}

#[cfg(not(target_os = "windows"))]
pub fn run_command_line(_cmdline: &str) -> bool {
    false
}

/// Execute a user-defined custom context-menu entry (Phase 4
/// customization). `path` is `None` when the entry is invoked from the
/// background menu.
pub fn run_custom_item(item: &CustomMenuItem, path: Option<&Path>) -> bool {
    #[cfg(target_os = "windows")]
    {
        let raw_args = substitute_target(&item.args, path);
        let raw_args = raw_args.trim().to_string();
        let args = if raw_args.is_empty() {
            String::new()
        } else {
            format!(" {raw_args}")
        };
        match item.kind {
            CustomMenuKind::Command => {
                let cmdline = open_with::expand_env_vars(&format!(
                    "cmd.exe /C {}{}",
                    item.value.trim(),
                    args
                ));
                run_command_line(&cmdline)
            }
            CustomMenuKind::App => {
                let cmdline = open_with::expand_env_vars(&format!("{}{}", item.value.trim(), args));
                run_command_line(&cmdline)
            }
            CustomMenuKind::Script => {
                let target = open_with::expand_env_vars(&substitute_target(&item.value, path));
                let params = if raw_args.is_empty() {
                    None
                } else {
                    Some(raw_args.as_str())
                };
                shell_execute_open(target.trim(), params)
            }
            CustomMenuKind::Link => {
                let target = open_with::expand_env_vars(&substitute_target(&item.value, path));
                shell_execute_open(target.trim(), None)
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (item, path);
        false
    }
}

/// Open a target (URL, script, document) with the shell "open" verb.
pub fn shell_execute_open(target: &str, params: Option<&str>) -> bool {
    #[cfg(target_os = "windows")]
    unsafe {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let file: Vec<u16> = OsStr::new(target)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let action: Vec<u16> = OsStr::new("open")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let param_wide: Vec<u16> = match params {
            Some(p) if !p.trim().is_empty() => OsStr::new(p)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect(),
            _ => vec![0],
        };
        let res = ShellExecuteW(
            std::ptr::null_mut(),
            action.as_ptr(),
            file.as_ptr(),
            param_wide.as_ptr(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
        (res as usize) > 32
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (target, params);
        false
    }
}

/// Copy a list of files to the OLE clipboard (CF_HDROP + PreferredDropEffect) so
/// Explorer can paste them with the intended verb (copy or move).
pub fn set_clipboard_files(paths: &[std::path::PathBuf]) -> bool {
    set_clipboard_files_with_effect(paths, false)
}

pub fn set_clipboard_files_with_effect(paths: &[std::path::PathBuf], cut: bool) -> bool {
    #[cfg(target_os = "windows")]
    unsafe {
        use std::mem::size_of;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Foundation::HANDLE;
        use windows_sys::Win32::System::DataExchange::{
            CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
        };
        use windows_sys::Win32::UI::Shell::DROPFILES;

        const CF_HDROP: u32 = 15;

        let joined: Vec<u16> = paths
            .iter()
            .flat_map(|p| p.as_os_str().encode_wide().chain(std::iter::once(0)))
            .chain(std::iter::once(0))
            .collect();
        if joined.is_empty() {
            return false;
        }
        let bytes = std::mem::size_of::<u16>() * joined.len();
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return false;
        }
        EmptyClipboard();

        let mem_bytes = size_of::<DROPFILES>() + bytes;
        let mem = windows_sys::Win32::System::Memory::GlobalAlloc(
            windows_sys::Win32::System::Memory::GMEM_MOVEABLE,
            mem_bytes,
        );
        if mem.is_null() {
            let _ = CloseClipboard();
            return false;
        }

        let p_mem = windows_sys::Win32::System::Memory::GlobalLock(mem);
        if p_mem.is_null() {
            let _ = CloseClipboard();
            return false;
        }

        let dropfiles = DROPFILES {
            pFiles: size_of::<DROPFILES>() as u32,
            pt: windows_sys::Win32::Foundation::POINT { x: 0, y: 0 },
            fNC: 0,
            fWide: 1,
        };
        std::ptr::write_bytes(p_mem, 0, mem_bytes);
        std::ptr::copy_nonoverlapping(
            &dropfiles as *const DROPFILES as *const u8,
            p_mem as *mut u8,
            size_of::<DROPFILES>(),
        );
        std::ptr::copy_nonoverlapping(
            joined.as_ptr() as *const u8,
            (p_mem as *mut u8).add(size_of::<DROPFILES>()),
            bytes,
        );
        let _ = windows_sys::Win32::System::Memory::GlobalUnlock(mem);

        if SetClipboardData(CF_HDROP, mem as HANDLE).is_null() {
            let _ = CloseClipboard();
            // On failure we still own hmem (system did not take ownership).
            // Avoid relying on GlobalFree feature availability by exiting safely.
            return false;
        }

        // FileNameW + PreferredDropEffect: tell Explorer whether to Move or Copy
        const DROPEFFECT_COPY: u32 = 1;
        const DROPEFFECT_MOVE: u32 = 2;
        let effect = if cut {
            DROPEFFECT_MOVE
        } else {
            DROPEFFECT_COPY
        };
        let pref_name: Vec<u16> = "Preferred DropEffect"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let pref_id =
            windows_sys::Win32::System::DataExchange::RegisterClipboardFormatW(pref_name.as_ptr());

        let pref_mem = windows_sys::Win32::System::Memory::GlobalAlloc(
            windows_sys::Win32::System::Memory::GMEM_MOVEABLE,
            4,
        );
        if !pref_mem.is_null() {
            let pref_ptr = windows_sys::Win32::System::Memory::GlobalLock(pref_mem);
            if !pref_ptr.is_null() {
                std::ptr::write_bytes(pref_ptr as *mut u32, 0, 1);
                std::ptr::write_unaligned(pref_ptr as *mut u32, effect);
                let _ = windows_sys::Win32::System::Memory::GlobalUnlock(pref_mem);
                let _ = SetClipboardData(pref_id, pref_mem as HANDLE);
            }
            // On success the system takes ownership; on lock failure we simply abandon the allocation (tiny leak, app continues)
        }

        // On success the system takes ownership of hmem
        let _ = CloseClipboard();
        true
    }
    #[cfg(not(target_os = "windows"))]
    {
        // CF_HDROP is a Windows construct; Linux receivers take uri-lists
        // via `linux_clipboard_files` instead. Report unsupported.
        let _ = (paths, cut);
        false
    }
}

/// Set the given image path as the desktop wallpaper (SPI_SETDESKWALLPAPER)
pub fn set_desktop_wallpaper(path: &Path) -> bool {
    #[cfg(target_os = "windows")]
    unsafe {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SystemParametersInfoW, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE,
        };

        const SPI_SETDESKWALLPAPER: u32 = 0x0014;
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let res = SystemParametersInfoW(
            SPI_SETDESKWALLPAPER,
            0,
            wide.as_ptr() as *mut _,
            SPIF_SENDCHANGE | SPIF_UPDATEINIFILE,
        );
        res != 0
    }
    #[cfg(not(target_os = "windows"))]
    {
        // No SystemParametersInfoW equivalent on Linux in scope here.
        let _ = path;
        false
    }
}

/// Create a `.lnk` shortcut to `target` alongside it via PowerShell COM (WScript.Shell)
pub fn create_shortcut(target: &Path) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let shortcut = target.with_extension("lnk");
        // Use PowerShell hidden window for a simple, reliable shortcut creation
        // Escape single quotes in every value interpolated into the PowerShell
        // single-quoted strings: NTFS paths may legally contain a quote, and an
        // unescaped one would break out and run arbitrary PowerShell (CI-01).
        let cmd = format!(
            "$ws=New-Object -ComObject WScript.Shell;$s=$ws.CreateShortcut('{}');$s.TargetPath='{}';$s.WorkingDirectory='{}';$s.Save()",
            shortcut.to_string_lossy().replace('\'', "''"),
            target.to_string_lossy().replace('\'', "''"),
            target.parent().map(|p| p.to_string_lossy().replace('\'', "''")).unwrap_or_default(),
        );
        std::process::Command::new("powershell")
            .arg("-NoProfile")
            .arg("-WindowStyle")
            .arg("Hidden")
            .arg("-Command")
            .arg(cmd)
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
            .spawn()
            .is_ok()
    }
    #[cfg(not(target_os = "windows"))]
    {
        // .lnk shortcuts are Windows-only; report unsupported elsewhere.
        let _ = target;
        false
    }
}

/// Create a shortcut (.lnk on Windows, symlink on Unix) to `target` inside `dest_dir`.
pub fn create_shortcut_in(target: &Path, dest_dir: &Path) -> bool {
    let name = match target.file_name() {
        Some(n) => n,
        None => return false,
    };
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let file_stem = name.to_string_lossy();
        let candidate = dest_dir.join(format!("{}.lnk", file_stem));
        let shortcut = crate::core::fs_utils::resolve_unique_paste_path(&candidate);
        let cmd = format!(
            "$ws=New-Object -ComObject WScript.Shell;$s=$ws.CreateShortcut('{}');$s.TargetPath='{}';$s.WorkingDirectory='{}';$s.Save()",
            shortcut.to_string_lossy().replace('\'', "''"),
            target.to_string_lossy().replace('\'', "''"),
            target.parent().map(|p| p.to_string_lossy().replace('\'', "''")).unwrap_or_default(),
        );
        let success = std::process::Command::new("powershell")
            .arg("-NoProfile")
            .arg("-WindowStyle")
            .arg("Hidden")
            .arg("-Command")
            .arg(cmd)
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
            .spawn()
            .is_ok();
        if success {
            crate::core::action_journal::record_copy(target.to_path_buf(), shortcut);
        }
        success
    }
    #[cfg(unix)]
    {
        let link_path = crate::core::fs_utils::resolve_unique_paste_path(&dest_dir.join(name));
        let success = std::os::unix::fs::symlink(target, &link_path).is_ok();
        if success {
            crate::core::action_journal::record_copy(target.to_path_buf(), link_path);
        }
        success
    }
    #[cfg(not(any(target_os = "windows", unix)))]
    {
        let _ = (target, dest_dir);
        false
    }
}

/// Run a Microsoft Defender custom scan on `path` (fire and forget, hidden)
pub fn scan_with_defender(path: &Path) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let cmd = format!(
            "Start-MpScan -ScanPath '{}'",
            path.to_string_lossy().replace('\'', "''")
        );
        std::process::Command::new("powershell")
            .arg("-NoProfile")
            .arg("-WindowStyle")
            .arg("Hidden")
            .arg("-Command")
            .arg(cmd)
            .creation_flags(0x0800_0000)
            .spawn()
            .is_ok()
    }
    #[cfg(not(target_os = "windows"))]
    {
        // Defender is Windows-only; report unsupported elsewhere.
        let _ = path;
        false
    }
}

/// Rotate an image by 90 deg without touching EXIF orientation flags
pub fn rotate_image_90(path: &Path) -> bool {
    if let Some(parent) = path.parent() {
        let _ = parent;
    }
    let img = match image::open(path) {
        Ok(i) => i,
        Err(_) => return false,
    };
    let rotated = img.rotate90();
    rotated.save(path).is_ok()
}

/// Deletes `paths` to the OS trash/recycle.
///
/// Windows: `SHFileOperationW` with `FOF_ALLOWUNDO` (true Recycle Bin).
/// macOS: hidden `.Trash` move; Linux: `gio trash` when available.
/// Returns `false` (caller may fall back to a permanent, logged delete).
pub fn delete_to_trash(paths: &[std::path::PathBuf]) -> bool {
    crate::core::fs_utils::delete_to_recycle_bin(paths).is_ok()
}

/// Enumerates "Send to" destinations from the Windows SendTo folder.
/// Label pairs are (display label, .lnk path). Non-Windows yields nothing.
pub fn send_to_targets() -> Vec<(String, std::path::PathBuf)> {
    #[cfg(target_os = "windows")]
    {
        let dir = std::env::var_os("APPDATA")
            .map(|a| std::path::PathBuf::from(a).join(r"Microsoft\Windows\SendTo"));

        let Some(dir) = dir else { return Vec::new() };
        let Ok(rd) = std::fs::read_dir(&dir) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for entry in rd.flatten() {
            let p = entry.path();
            if !matches!(
                p.extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_ascii_lowercase())
                    .as_deref(),
                Some("lnk"),
            ) {
                continue;
            }
            let label = p
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Destination".into());
            out.push((label, p));
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }
    #[cfg(not(target_os = "windows"))]
    {
        Vec::new()
    }
}

/// Invokes a SendTo destination (.lnk) with the given files as arguments.
pub fn send_to(paths: &[std::path::PathBuf], destination: &std::path::Path) -> bool {
    if paths.is_empty() {
        return false;
    }
    let joined = paths
        .iter()
        .map(|p| format!("\"{}\"", p.to_string_lossy()))
        .collect::<Vec<_>>()
        .join(" ");
    #[cfg(target_os = "windows")]
    unsafe {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let wide: Vec<u16> = destination
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let params: Vec<u16> = OsStr::new(&joined)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let verb_open: Vec<u16> = OsStr::new("open")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let res = ShellExecuteW(
            std::ptr::null_mut(),
            verb_open.as_ptr(),
            wide.as_ptr(),
            params.as_ptr(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
        (res as usize) > 32
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (destination, joined);
        false
    }
}
