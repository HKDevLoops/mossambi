//! Native toast notifications for Mossambi.
//!
//! Pure 7-bit ASCII. All-platform API: `notify()` shows a Windows Action
//! Center toast on Windows and is a logged no-op elsewhere. The pure builders
//! (XML payload, mossambi:// deep-link URLs, gate rule) live here so they are
//! testable headless on every platform.

use std::path::{Path, PathBuf};

/// Broad category of a toast (reserved for per-kind styling).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifyKind {
    /// Job finished successfully.
    Done,
    /// Job failed.
    Error,
    /// Informational message.
    Info,
}

/// Action encoded in a mossambi:// deep link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeepLinkKind {
    /// Navigate to the path.
    Open,
    /// Navigate to the parent folder (and select the file where supported).
    Reveal,
}

/// AUMID used for the unpackaged app identity (toast registration).
pub const AUMID: &str = "Mossambi.Explorer";

/// URL scheme registered for toast protocol activation.
pub const URL_SCHEME: &str = "mossambi";

/// Shows a toast notification. Never blocks the caller for long and never
/// panics: every failure is logged.
pub fn notify(title: &str, body: &str, kind: NotifyKind, open_path: Option<&Path>) {
    #[cfg(windows)]
    {
        win::show_toast(title, body, kind, open_path);
    }
    #[cfg(not(windows))]
    {
        let _ = (title, body, kind, open_path);
        log::debug!("notify: toasts are not supported on this platform");
    }
}

/// Gate rule: notify only when enabled AND (the main window is not focused
/// OR the job took longer than 10 seconds). `window_focused` is the egui
/// viewport focus probe; `None` is treated as "focused" (conservative).
pub fn should_notify(
    enabled: bool,
    window_focused: Option<bool>,
    elapsed: Option<std::time::Duration>,
) -> bool {
    if !enabled {
        return false;
    }
    let focused = window_focused.unwrap_or(true);
    if !focused {
        return true;
    }
    elapsed.is_some_and(|d| d > std::time::Duration::from_secs(10))
}

/// Escapes `& < > " '` for XML text and attribute values.
pub fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// Percent-encodes a string for use as a URL query component (UTF-8 aware).
/// Unreserved characters (RFC 3986) are left as-is; everything else,
/// including spaces and all non-ASCII bytes, is `%XX`-escaped.
pub fn url_encode_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// Decodes a percent-encoded URL query component. `+` decodes as a space.
/// Returns `None` on malformed escapes or invalid UTF-8.
pub fn url_decode_component(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                if i + 2 >= bytes.len() {
                    return None;
                }
                let hi = (bytes[i + 1] as char).to_digit(16)?;
                let lo = (bytes[i + 2] as char).to_digit(16)?;
                out.push((hi * 16 + lo) as u8);
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

/// Builds a mossambi:// deep-link URL for `path` (`open` or `reveal` verb).
pub fn build_mossambi_url(kind: DeepLinkKind, path: &Path) -> String {
    let verb = match kind {
        DeepLinkKind::Open => "open",
        DeepLinkKind::Reveal => "reveal",
    };
    format!(
        "{}://{}?path={}",
        URL_SCHEME,
        verb,
        url_encode_component(&path.to_string_lossy())
    )
}

/// Parses a mossambi:// deep-link argument into its action and target path.
/// Returns `None` for foreign URLs, unknown verbs, or missing/malformed
/// `path` parameters.
pub fn parse_mossambi_url(arg: &str) -> Option<(DeepLinkKind, PathBuf)> {
    let rest = arg.strip_prefix("mossambi://")?;
    let (verb, query) = match rest.split_once('?') {
        Some((v, q)) => (v, q),
        None => (rest, ""),
    };
    let kind = match verb {
        "open" => DeepLinkKind::Open,
        "reveal" => DeepLinkKind::Reveal,
        _ => return None,
    };
    let mut raw = None;
    for pair in query.split('&') {
        if let Some(v) = pair.strip_prefix("path=") {
            raw = Some(v);
        }
    }
    let decoded = url_decode_component(raw?)?;
    if decoded.is_empty() {
        return None;
    }
    Some((kind, PathBuf::from(decoded)))
}

/// GUI navigation target for a deep link: Open navigates to the path (its
/// parent folder when the path is a file); Reveal navigates to the parent
/// folder so the file can be selected.
pub fn navigate_target_for(kind: DeepLinkKind, path: &Path) -> PathBuf {
    let parent = || path.parent().map(Path::to_path_buf);
    match kind {
        DeepLinkKind::Open => {
            if path.is_file() {
                parent().unwrap_or_else(|| path.to_path_buf())
            } else {
                path.to_path_buf()
            }
        }
        DeepLinkKind::Reveal => parent().unwrap_or_else(|| path.to_path_buf()),
    }
}

/// Builds the Windows toast XML payload per the notification spec:
/// protocol-activated launch plus (when a path is given) the "Open folder"
/// and "Show in Mossambi" actions.
pub fn build_toast_xml(title: &str, body: &str, open_path: Option<&Path>) -> String {
    let launch = match open_path {
        Some(p) => format!(
            " launch=\"{}\"",
            xml_escape(&build_mossambi_url(DeepLinkKind::Open, p))
        ),
        None => String::new(),
    };
    let actions = match open_path {
        Some(p) => format!(
            "<actions><action content=\"Open folder\" activationType=\"protocol\" arguments=\"{}\"/><action content=\"Show in Mossambi\" activationType=\"protocol\" arguments=\"{}\"/></actions>",
            xml_escape(&build_mossambi_url(DeepLinkKind::Open, p)),
            xml_escape(&build_mossambi_url(DeepLinkKind::Reveal, p))
        ),
        None => String::new(),
    };
    format!(
        "<toast activationType=\"protocol\"{}><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual>{}</toast>",
        launch,
        xml_escape(title),
        xml_escape(body),
        actions
    )
}

// -- Windows implementation ------------------------------------------------

#[cfg(windows)]
mod win {
    use super::{build_toast_xml, NotifyKind, AUMID};
    use std::path::Path;
    use std::sync::Once;

    /// One-shot HKCU identity registration (idempotent by construction).
    static REGISTER: Once = Once::new();

    pub(super) fn show_toast(title: &str, body: &str, kind: NotifyKind, open_path: Option<&Path>) {
        let title = title.to_string();
        let body = body.to_string();
        let open_path = open_path.map(Path::to_path_buf);
        // Toasts run off-thread: WinRT activation must never stall the UI
        // frame or panic the app (spec: log errors, never panic).
        let spawned = std::thread::Builder::new()
            .name("mossambi-toast".into())
            .spawn(move || show_toast_blocking(&title, &body, kind, open_path.as_deref()));
        if let Err(e) = spawned {
            log::warn!("could not spawn toast thread: {}", e);
        }
    }

    fn show_toast_blocking(title: &str, body: &str, kind: NotifyKind, open_path: Option<&Path>) {
        use windows::core::HSTRING;
        use windows::Data::Xml::Dom::XmlDocument;
        use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};

        let _ = kind; // ToastGeneric template; kind is reserved for styling

        ensure_registered();

        // A fresh thread needs an initialized COM apartment for WinRT calls.
        // An Err (e.g. RPC_E_CHANGED_MODE) is not fatal; keep going.
        let com_ok = unsafe {
            use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok()
        };

        let xml = build_toast_xml(title, body, open_path);
        let result = (|| -> Result<(), windows::core::Error> {
            let doc = XmlDocument::new()?;
            doc.LoadXml(&HSTRING::from(xml.as_str()))?;
            let toast = ToastNotification::CreateToastNotification(&doc)?;
            let notifier =
                ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(AUMID))?;
            notifier.Show(&toast)?;
            Ok(())
        })();
        if let Err(e) = result {
            log::warn!("toast notification failed: {}", e);
        }

        if com_ok {
            unsafe {
                use windows::Win32::System::Com::CoUninitialize;
                CoUninitialize();
            }
        }
    }

    /// Registers the unpackaged app identity (AUMID) and the mossambi://
    /// URL protocol under HKCU, once per process. Only HKCU writes; every
    /// write is idempotent (create-or-overwrite).
    fn ensure_registered() {
        REGISTER.call_once(|| {
            if let Err(e) = register_unpackaged_identity() {
                log::warn!("toast identity registration failed: {}", e);
            }
        });
    }

    fn register_unpackaged_identity() -> Result<(), String> {
        let exe = std::env::current_exe().map_err(|e| format!("current_exe: {}", e))?;
        let exe_str = exe.to_string_lossy().to_string();

        // AUMID identity: display name + icon (only when the asset exists).
        let aumid_key = format!("Software\\Classes\\AppUserModelId\\{}", AUMID);
        write_reg_string(&aumid_key, "DisplayName", "Mossambi")?;
        let icon = exe
            .parent()
            .map(|d| d.join("assets").join("icon.png"))
            .filter(|p| p.is_file());
        if let Some(icon_path) = icon {
            write_reg_string(&aumid_key, "IconUri", &icon_path.to_string_lossy())?;
        }

        // mossambi:// URL protocol -> re-launches the exe with the URI.
        write_reg_string("Software\\Classes\\mossambi", "", "URL:Mossambi")?;
        write_reg_empty("Software\\Classes\\mossambi", "URL Protocol")?;
        write_reg_string(
            "Software\\Classes\\mossambi\\shell\\open\\command",
            "",
            &format!("\"{}\" \"%1\"", exe_str),
        )?;
        Ok(())
    }

    fn to_wide(s: &str) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        let mut wide: Vec<u16> = std::ffi::OsStr::new(s).encode_wide().collect();
        wide.push(0);
        wide
    }

    fn write_reg_string(subkey: &str, value_name: &str, data: &str) -> Result<(), String> {
        let mut wide: Vec<u16> = data.encode_utf16().collect();
        wide.push(0);
        write_reg_value(subkey, value_name, &wide)
    }

    fn write_reg_empty(subkey: &str, value_name: &str) -> Result<(), String> {
        write_reg_value(subkey, value_name, &[0])
    }

    fn write_reg_value(subkey: &str, value_name: &str, data: &[u16]) -> Result<(), String> {
        use windows_sys::Win32::System::Registry::{
            RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_WRITE,
            REG_OPTION_NON_VOLATILE, REG_SZ,
        };

        unsafe {
            let subkey_w = to_wide(subkey);
            let value_w = to_wide(value_name);
            let mut hkey: HKEY = std::ptr::null_mut();
            let res = RegCreateKeyExW(
                HKEY_CURRENT_USER,
                subkey_w.as_ptr(),
                0,
                std::ptr::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE,
                std::ptr::null(),
                &mut hkey,
                std::ptr::null_mut(),
            );
            if res != 0 {
                return Err(format!("RegCreateKeyExW failed: 0x{:08X}", res));
            }
            let res2 = RegSetValueExW(
                hkey,
                value_w.as_ptr(),
                0,
                REG_SZ,
                data.as_ptr() as *const u8,
                (data.len() * 2) as u32,
            );
            RegCloseKey(hkey);
            if res2 != 0 {
                return Err(format!("RegSetValueExW failed: 0x{:08X}", res2));
            }
        }
        Ok(())
    }
}
