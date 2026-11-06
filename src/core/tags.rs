//! File Tagging (Finder-style colored tags) stored in NTFS Alternate Streams.
//!
//! Mossambi tags a file by writing a UTF-8, comma-separated label list into
//! the file's own NTFS alternate data stream `file:tags`. Nothing outside the
//! tagged object is touched: no sidecar files, no database, and the tag survives
//! Explorer moves on the same volume.
//!
//! The four preset tag labels are also the sidebar smart-list names
//! (Home / Work / Photos / Important) and the context-menu presets.
//!
//! Row rendering reads tags once per session per path through a small cache;
//! [`read_tags_cache`] is the UI-facing accessor, [`set_tags`] invalidates the
//! cached entry for that path, and navigation refreshes call [`clear_cache`].

use std::path::Path;
#[cfg(target_os = "windows")]
use std::path::PathBuf;
use std::sync::Mutex;

/// The preset tag labels, in menu order (Finder uses free-form tags - we ship
/// four and treat them as a closed set so the UI stays clean).
pub const TAG_PRESETS: [&str; 4] = ["Home", "Work", "Photos", "Important"];

/// Display colors for the presets (macOS system palette).
pub fn tag_color(name: &str) -> eframe::egui::Color32 {
    match name {
        "Home" => eframe::egui::Color32::from_rgb(0, 122, 255),
        "Work" => eframe::egui::Color32::from_rgb(255, 149, 0),
        "Photos" => eframe::egui::Color32::from_rgb(255, 45, 85),
        "Important" => eframe::egui::Color32::from_rgb(52, 199, 89),
        _ => eframe::egui::Color32::from_rgb(160, 160, 160),
    }
}

/// Parse the on-disk tag payload ("Home,Work") into a list.
pub fn parse_tags(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

/// Serialize a tag list back to the on-disk payload.
pub fn join_tags(tags: &[String]) -> String {
    tags.join(",")
}

#[cfg(target_os = "windows")]
fn tags_stream_path(path: &Path) -> PathBuf {
    // NTFS alternate data stream: `file/dir :tags`
    let mut s = path.as_os_str().to_os_string();
    s.push(":tags");
    PathBuf::from(s)
}

/// Read the raw tag payload (Windows IO; other platforms yield empty).
#[cfg(target_os = "windows")]
fn read_raw(path: &Path) -> Vec<String> {
    let p = tags_stream_path(path);
    match std::fs::read(&p) {
        Ok(bytes) => parse_tags(&String::from_utf8_lossy(&bytes)),
        Err(_) => Vec::new(), // no stream yet = untagged (never an error in UI)
    }
}

#[cfg(not(target_os = "windows"))]
fn read_raw(_path: &Path) -> Vec<String> {
    Vec::new()
}

/// Write/overwrite the tag payload. Empty list removes the stream.
#[cfg(target_os = "windows")]
fn write_raw(path: &Path, payload: &str) -> Result<(), String> {
    if payload.is_empty() {
        match std::fs::remove_file(tags_stream_path(path)) {
            Ok(()) => return Ok(()),
            // Removing a stream that never existed is success for our purposes.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(format!("could not clear tags stream: {e}")),
        }
    }
    std::fs::write(tags_stream_path(path), payload.as_bytes())
        .map_err(|e| format!("could not write tags stream: {e}"))
}

#[cfg(not(target_os = "windows"))]
fn write_raw(_path: &Path, _payload: &str) -> Result<(), String> {
    Ok(())
}

/// Tag list for `path` (empty when untagged or the platform cannot store ADS).
pub fn get_tags(path: &Path) -> Vec<String> {
    read_raw(path)
}

/// Cache/lookup wrapper for UI row rendering: avoids per-frame ADS reads.
static TAG_CACHE: Mutex<Option<std::collections::HashMap<PathBuf, Vec<String>>>> = Mutex::new(None);

/// Cached read path used by the GUI each frame.
pub fn read_tags_cache(path: &Path) -> Vec<String> {
    if let Ok(mut guard) = TAG_CACHE.lock() {
        let map = guard.get_or_insert_with(Default::default);
        if let Some(hit) = map.get(path) {
            return hit.clone();
        }
        let tags = read_raw(path);
        map.insert(path.to_path_buf(), tags.clone());
        return tags;
    }
    read_raw(path)
}

/// Forget the cache for one path (called after a successful tag write).
pub fn invalidate_cache(path: &Path) {
    if let Ok(mut guard) = TAG_CACHE.lock() {
        if let Some(map) = guard.as_mut() {
            map.remove(path);
        }
    }
}

/// Forget the whole cache (navigation refresh).
pub fn clear_cache() {
    if let Ok(mut guard) = TAG_CACHE.lock() {
        *guard = None;
    }
}

/// Assign a tag to `path`, returning an error string suitable for status notes.
pub fn set_tag(path: &Path, tag: &str) -> Result<(), String> {
    if !TAG_PRESETS.contains(&tag) {
        return Err(format!("unknown tag preset: {tag}"));
    }
    let mut tags = get_tags(path);
    if tags.iter().any(|t| t == tag) {
        return Ok(()); // idempotent
    }
    tags.push(tag.to_string());
    write_raw(path, &join_tags(&tags))?;
    invalidate_cache(path);
    Ok(())
}

/// Remove one tag (no-op friendly when absent).
pub fn clear_tag(path: &Path, tag: &str) -> Result<(), String> {
    let mut tags = get_tags(path);
    tags.retain(|t| t != tag);
    write_raw(path, &join_tags(&tags))?;
    invalidate_cache(path);
    Ok(())
}

/// Overwrite the tag list exactly (undo support for the action journal).
pub fn restore_tags(path: &Path, tags: &[String]) -> Result<(), String> {
    let clean: Vec<String> = tags
        .iter()
        .filter(|t| TAG_PRESETS.contains(&t.as_str()))
        .map(|t| t.to_string())
        .collect();
    write_raw(path, &join_tags(&clean))?;
    invalidate_cache(path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_join_roundtrip() {
        let raw = "Home, Work,,Important";
        let tags = parse_tags(raw);
        assert_eq!(tags, vec!["Home", "Work", "Important"]);
        assert_eq!(join_tags(&tags), "Home,Work,Important");
        assert!(parse_tags("  ").is_empty());
    }

    #[test]
    fn presets_and_colors() {
        assert_eq!(TAG_PRESETS.len(), 4);
        // All presets resolve to a color (gray fallback never used).
        for t in TAG_PRESETS {
            let c = tag_color(t);
            assert_ne!(c, tag_color("unknown-tag"));
        }
    }
}
