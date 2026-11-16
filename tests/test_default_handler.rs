//! Headless tests for the opt-in default file manager registry handler.
//!
//! Pure 7-bit ASCII. Every registry operation runs under the isolated test
//! root HKCU\Software\MossambiTest\Classes; the real HKCU\Software\Classes
//! tree is never touched.

#![cfg(windows)]

use mossambi::platform::default_handler::{
    disable, enable, is_enabled, BACKUP_VALUE, NO_PREV_DEFAULT, TEST_ROOT,
};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY, HKEY_CURRENT_USER, KEY_ALL_ACCESS, KEY_READ, REG_OPTION_NON_VOLATILE, REG_SZ,
};

const ERROR_SUCCESS: u32 = 0;

fn to_wide(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// Writes a REG_SZ default value on the given subkey.
fn set_default(subkey: &str, value: &str) {
    let wide = to_wide(subkey);
    let mut hk: HKEY = std::ptr::null_mut();
    let mut disp = 0u32;
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            wide.as_ptr(),
            0,
            std::ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_ALL_ACCESS,
            std::ptr::null(),
            &mut hk,
            &mut disp,
        )
    };
    assert_eq!(status, ERROR_SUCCESS, "RegCreateKeyExW failed for {subkey}");
    let wide_val = to_wide(value);
    let status = unsafe {
        RegSetValueExW(
            hk,
            std::ptr::null(),
            0,
            REG_SZ,
            wide_val.as_ptr() as *const u8,
            (wide_val.len() * 2) as u32,
        )
    };
    assert_eq!(status, ERROR_SUCCESS, "RegSetValueExW failed for {subkey}");
    unsafe {
        RegCloseKey(hk);
    }
}

/// Reads a REG_SZ value (the key's default when `name` is empty).
fn query_sz(subkey: &str, name: Option<&str>) -> Option<String> {
    let wide = to_wide(subkey);
    let mut hk: HKEY = std::ptr::null_mut();
    unsafe {
        if RegOpenKeyExW(HKEY_CURRENT_USER, wide.as_ptr(), 0, KEY_READ, &mut hk) != ERROR_SUCCESS {
            return None;
        }
    }
    let wide_name = name.map(to_wide);
    let name_ptr = wide_name
        .as_ref()
        .map(|v| v.as_ptr())
        .unwrap_or(std::ptr::null());
    let mut buf = [0u16; 512];
    let mut len = (buf.len() * 2) as u32;
    let mut kind = 0u32;
    let status = unsafe {
        RegQueryValueExW(
            hk,
            name_ptr,
            std::ptr::null_mut(),
            &mut kind,
            buf.as_mut_ptr() as *mut u8,
            &mut len,
        )
    };
    unsafe {
        RegCloseKey(hk);
    }
    if status != ERROR_SUCCESS || kind != REG_SZ {
        return None;
    }
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..n]))
}

fn read_default(subkey: &str) -> Option<String> {
    query_sz(subkey, None)
}

fn read_backup(subkey: &str) -> Option<String> {
    query_sz(subkey, Some(BACKUP_VALUE))
}

/// Removes the whole isolated test key tree (both halves of this suite).
fn delete_test_tree() {
    let wide = to_wide("Software\\MossambiTest");
    unsafe {
        RegDeleteTreeW(HKEY_CURRENT_USER, wide.as_ptr());
    }
}

#[test]
fn test_default_handler_round_trip_and_idempotency() {
    // Start from a clean isolated tree so state from prior runs cannot leak.
    delete_test_tree();

    let root = TEST_ROOT.to_string();
    let dir_shell = format!("{root}\\Directory\\shell");
    let drive_shell = format!("{root}\\Drive\\shell");
    let folder_shell = format!("{root}\\Folder\\shell");

    // disable() on a never-enabled root is a safe no-op.
    assert!(disable(&root).is_ok(), "fresh disable must succeed");
    assert!(!is_enabled(&root), "fresh root must not report enabled");

    // Pre-set previous defaults: Directory has "open"; Drive and Folder
    // have no default verb at all (the "none" case).
    set_default(&dir_shell, "open");

    // First enable: backs up "open" and "<none>", sets mossambi everywhere.
    assert!(enable(&root).is_ok(), "enable must succeed");
    assert!(
        is_enabled(&root),
        "handler must report enabled after enable"
    );
    assert_eq!(read_default(&dir_shell).as_deref(), Some("mossambi"));
    assert_eq!(
        read_backup(&dir_shell).as_deref(),
        Some("open"),
        "pre-set previous default must be saved in the backup value"
    );
    assert_eq!(
        read_backup(&folder_shell).as_deref(),
        Some(NO_PREV_DEFAULT),
        "absent previous default must be recorded as <none>"
    );
    assert!(
        !read_default(&format!("{dir_shell}\\mossambi\\command"))
            .unwrap_or_default()
            .is_empty(),
        "verb command must be written"
    );

    // Double enable is safe: the backup must never be clobbered with our
    // own verb name.
    assert!(enable(&root).is_ok(), "second enable must succeed");
    assert!(is_enabled(&root));
    assert_eq!(
        read_backup(&dir_shell).as_deref(),
        Some("open"),
        "double enable must not overwrite the backup with mossambi"
    );

    // Disable restores the pre-set default, clears the absent ones and
    // removes the verb subtree plus backup values.
    assert!(disable(&root).is_ok(), "disable must succeed");
    assert!(!is_enabled(&root), "handler must report disabled");
    assert_eq!(
        read_default(&dir_shell).as_deref(),
        Some("open"),
        "pre-set previous default must be restored"
    );
    assert_eq!(
        read_default(&drive_shell).as_deref(),
        None,
        "absent previous default must be cleared, not restored as <none>"
    );
    assert_eq!(read_default(&folder_shell).as_deref(), None);
    assert_eq!(
        read_backup(&dir_shell),
        None,
        "backup value must be removed after disable"
    );
    assert!(
        read_default(&format!("{dir_shell}\\mossambi\\command")).is_none(),
        "mossambi verb subtree must be deleted on disable"
    );

    // Cleanup: remove the whole isolated test key tree.
    delete_test_tree();
}
