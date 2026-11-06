//! OS File Clipboard for Linux (drag-out fallback)
//!
//! Windows already has real CF_HDROP clipboard support (`context_menu.rs`)
//! and real OLE drag-out (`ole_drag_out.rs`). On Linux, file managers
//! (Nautilus, Dolphin, Nemo, PCManFM...) exchange file lists through the
//! standard `text/uri-list` clipboard target (RFC 2483).
//!
//! The payload bytes are composed by the shared, OS-independent functions
//! in this file root so the wire format is unit-tested on every platform;
//! only the clipboard write itself is Linux-gated and shells out to the
//! session helper (`wl-copy` on Wayland, `xclip` on X11). Plain-text
//! clipboard cannot carry a file list, so a missing helper is reported,
//! not silently degraded.
//!
//! Single-target limitation (verified against `wl-copy(1)` BUGS / issue
//! wl-clipboard#71 and the xclip selection model): one helper process can
//! serve exactly one MIME type, and a second owner takes over the
//! selection. We therefore write the universal `text/uri-list`. Receivers
//! preferring `x-special/gnome-copied-files` (GNOME/Qt) fall back to the
//! uri-list and treat the paste as COPY, which matches this module's
//! COPY-only semantics. `ClipboardOp::Cut` exists for payload symmetry and
//! the Windows CF_HDROP path; the clipboard write itself never signals cut.
//!
//! Keep copy-only behavior: do not perform any removal ourselves.

use std::path::Path;
use std::path::PathBuf;

/// Operation kind advertised to the paste target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardOp {
    Copy,
    Cut,
}

/// Result of a successful clipboard write.
#[derive(Debug, Clone)]
pub struct ClipboardOutcome {
    /// Which helper received the payload (`wl-copy`, `xclip`).
    pub via: String,
    /// True when the *typed* `text/uri-list` target was offered.
    pub typed_uri_list: bool,
}

/// Compose the standard file-clipboard payloads for `paths`.
///
/// Returns `(uri_list, special)`:
/// - `uri_list`: RFC 2483, universally understood, this is what `copy_files`
///   writes.
/// - `special`: GNOME/Qt `x-special/gnome-copied-files` layout (operation
///   word first, then URIs). Kept for completeness since the helper session
///   can serve only one target (see module docs).
pub fn build_payloads(paths: &[PathBuf], op: ClipboardOp) -> (String, String) {
    let verb = match op {
        ClipboardOp::Copy => "copy",
        ClipboardOp::Cut => "cut",
    };
    let uris: Vec<String> = paths.iter().map(|p| path_to_file_uri(p)).collect();

    // RFC 2483 uri-list: CRLF between URIs. Comments start with '#'.
    let uri_list = uris.join("\r\n") + "\r\n";
    // GNOME/Qt special target: operation word + one URI per LF line.
    let special = format!("{verb}\n{}\n", uris.join("\n"));
    (uri_list, special)
}

/// Percent-encode bytes outside the RFC 3986 allowed set for file URLs.
fn percent_encode_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 8);
    for byte in input.as_bytes() {
        let b = *byte;
        let keep = b.is_ascii_alphanumeric()
            || matches!(
                b,
                b'-' | b'_' | b'.' | b'~' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+'
                    | b',' | b';' | b'=' | b':' | b'@' | b'!'
                    // Path separator: file URLs keep `/` raw so the result
                    // is `file:///abs/path`, not `%2F` soup.
                    | b'/'
            );
        if keep {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// POSIX path -> `file:///abs/path` with UTF-8 percent encoding.
pub fn path_to_file_uri(path: &Path) -> String {
    let text = path.to_string_lossy();
    "file://".to_string() + &percent_encode_component(&text)
}

#[cfg(target_os = "linux")]
pub mod linux {
    //! Session clipboard helpers storing the shared payloads.
    pub use super::{ClipboardOp, ClipboardOutcome};
    use super::build_payloads;
    use std::io::Write;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};

    /// Search PATH for an executable (no `which` dependency).
    fn find_on_path(name: &str) -> Option<PathBuf> {
        let path = std::env::var_os("PATH")?;
        std::env::split_paths(&path)
            .find(|dir| dir.join(name).is_file())
            .map(|dir| dir.join(name))
    }

    /// Locate the session-appropriate clipboard helper.
    ///
    /// Wayland sessions must use `wl-copy` (an X11 helper cannot hand data to
    /// native Wayland clients, and vice versa through most compositors).
    fn resolve_helper() -> (&'static str, String) {
        let wayland = std::env::var("WAYLAND_DISPLAY")
            .map(|v| !v.is_empty())
            .unwrap_or(false)
            || std::env::var("XDG_SESSION_TYPE")
                .map(|v| v == "wayland")
                .unwrap_or(false);

        if wayland {
            if let Some(run) = find_on_path("wl-copy") {
                return ("wl-copy", run.to_string_lossy().into_owned());
            }
        }
        if let Some(run) = find_on_path("xclip") {
            return ("xclip", run.to_string_lossy().into_owned());
        }
        ("none", String::new())
    }

    /// Write `paths` to the OS file clipboard as `text/uri-list` (see the
    /// module docs for why only this single target is served).
    ///
    /// Returns the outcome, or an actionable error string.
    pub fn copy_files(paths: &[PathBuf], op: ClipboardOp) -> Result<ClipboardOutcome, String> {
        if paths.is_empty() {
            return Err("no files given".to_string());
        }

        let (uri_list, _special) = build_payloads(paths, op);
        let (helper, bin) = resolve_helper();
        if helper == "none" {
            return Err(
                "install wl-clipboard (Wayland) or xclip (X11) to share files with other apps"
                    .to_string(),
            );
        }

        match helper {
            "wl-copy" => {
                // wl-copy forks; keep stdin open until payload handed over.
                let mut wr = Command::new(&bin)
                    .arg("--type")
                    .arg("text/uri-list")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .map_err(|e| format!("failed to start wl-copy: {e}"))?;
                if let Some(si) = wr.stdin.as_mut() {
                    si.write_all(uri_list.as_bytes())
                        .map_err(|e| format!("failed to write clipboard payload: {e}"))?;
                }
                drop(wr.stdin.take());
                drop(wr);

                Ok(ClipboardOutcome {
                    via: "wl-copy".to_string(),
                    typed_uri_list: true,
                })
            }
            "xclip" => {
                let mut wr = Command::new(&bin)
                    .arg("-selection")
                    .arg("clipboard")
                    .arg("-t")
                    .arg("text/uri-list")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .map_err(|e| format!("failed to start xclip: {e}"))?;
                if let Some(si) = wr.stdin.as_mut() {
                    si.write_all(uri_list.as_bytes())
                        .map_err(|e| format!("failed to write clipboard payload: {e}"))?;
                }
                drop(wr.stdin.take());
                drop(wr);

                Ok(ClipboardOutcome {
                    via: "xclip".to_string(),
                    typed_uri_list: true,
                })
            }
            _ => unreachable!("resolve_helper never returns other verbs"),
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub mod linux {
    //! Compile-time stub; callers gate themselves with cfg(linux).
    pub use super::{ClipboardOp, ClipboardOutcome};
    use std::path::PathBuf;

    pub fn copy_files(_paths: &[PathBuf], _op: ClipboardOp) -> Result<ClipboardOutcome, String> {
        Err("Linux clipboard is used only on Linux builds".to_string())
    }
}

pub use linux::copy_files;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn uri_list_payload_is_rfc2483() {
        let paths = vec![
            PathBuf::from("/tmp/a file #1.txt"),
            PathBuf::from("/tmp/b.jpg"),
        ];
        let (uri_list, special) = build_payloads(&paths, ClipboardOp::Copy);
        assert_eq!(
            uri_list,
            "file:///tmp/a%20file%20%231.txt\r\nfile:///tmp/b.jpg\r\n"
        );
        assert_eq!(
            special,
            "copy\nfile:///tmp/a%20file%20%231.txt\nfile:///tmp/b.jpg\n"
        );
    }

    #[test]
    fn cut_verb_is_advertised() {
        let (uri_list, special) = build_payloads(&[PathBuf::from("/x")], ClipboardOp::Cut);
        assert!(uri_list.starts_with("file:///x"));
        assert!(special.starts_with("cut\n"));
    }

    #[test]
    fn windows_drive_does_not_get_slash_soup() {
        // Backslashes are percent-encoded, forward path separators are not.
        let uri = path_to_file_uri(std::path::Path::new("C:\\Users\\haris\\a b.png"));
        assert_eq!(uri, "file://C:%5CUsers%5Charis%5Ca%20b.png");
    }
}
