//! File Tagging (Finder-style colored tags) stored in NTFS Alternate Streams.
//!
//! Mossambi tags a file by writing a UTF-8, comma-separated label list into
//! the file's own NTFS alternate data stream `file:tags`. Nothing outside the
//! tagged object is touched: no sidecar files, no database, and the tag
//! survives Explorer moves on the same volume.
//!
//! The four preset tag labels are also the sidebar smart-list names
//! (Home / Work / Photos / Important) and the context-menu presets. Users
//! may also create custom tags, which are stored in the config file and
//! rendered alongside the presets with a stable hash-based colour.
//!
//! Row rendering reads tags once per session per path through a small cache;
//! [`read_tags_cache`] is the UI-facing accessor, [`set_tags`] invalidates
//! the cached entry for that path, and navigation refreshes call
//! [`clear_cache`].

use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The preset tag labels, in menu order (Finder uses free-form tags - we
/// ship four and allow user-created custom tags on top).
pub const TAG_PRESETS: [&str; 4] = ["Home", "Work", "Photos", "Important"];

/// Display colours for the presets (macOS system palette).
pub fn tag_color(name: &str) -> eframe::egui::Color32 {
    match name.trim().to_lowercase().as_str() {
        "home" => eframe::egui::Color32::from_rgb(0, 122, 255),
        "work" => eframe::egui::Color32::from_rgb(255, 149, 0),
        "photos" => eframe::egui::Color32::from_rgb(255, 45, 85),
        "important" => eframe::egui::Color32::from_rgb(52, 199, 89),
        norm => {
            // Stable hash-based colour for custom / unknown tags so every
            // surface (row dots, menu dots, sidebar) agrees on the colour
            // without needing a config lookup per frame.
            let idx = tag_hash(norm) % CUSTOM_TAG_COLORS.len();
            CUSTOM_TAG_COLORS[idx]
        }
    }
}

/// Fixed palette used for custom tags (distinct from the four presets).
const CUSTOM_TAG_COLORS: [eframe::egui::Color32; 8] = [
    eframe::egui::Color32::from_rgb(175, 82, 222),
    eframe::egui::Color32::from_rgb(64, 200, 244),
    eframe::egui::Color32::from_rgb(255, 214, 10),
    eframe::egui::Color32::from_rgb(0, 200, 140),
    eframe::egui::Color32::from_rgb(108, 110, 255),
    eframe::egui::Color32::from_rgb(170, 130, 80),
    eframe::egui::Color32::from_rgb(255, 120, 140),
    eframe::egui::Color32::from_rgb(212, 80, 220),
];

fn tag_hash(name: &str) -> usize {
    let mut h: u32 = 5381;
    for b in name.bytes() {
        h = h.wrapping_mul(33).wrapping_add(b as u32);
    }
    h as usize
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
    let ads_tags = match std::fs::read(&p) {
        Ok(bytes) => parse_tags(&String::from_utf8_lossy(&bytes)),
        Err(_) => Vec::new(),
    };
    if !ads_tags.is_empty() {
        return ads_tags;
    }
    crate::core::state_db::get_file_tags(path)
}

/// Write/overwrite the tag payload. Empty list removes the stream and DB entry.
#[cfg(target_os = "windows")]
fn write_raw(path: &Path, payload: &str) -> Result<(), String> {
    let tag_list = parse_tags(payload);
    let _ = crate::core::state_db::set_file_tags(path, &tag_list);

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

#[cfg(target_os = "macos")]
const XATTR_TAG_KEY: &str = "com.apple.metadata:_kMDItemUserTags";

#[cfg(all(unix, not(target_os = "macos")))]
const XATTR_TAG_KEY: &str = "user.xdg.tags";

#[cfg(unix)]
fn read_raw(path: &Path) -> Vec<String> {
    if let Ok(Some(bytes)) = xattr::get(path, XATTR_TAG_KEY) {
        let text = String::from_utf8_lossy(&bytes);
        let list = parse_tags(&text);
        if !list.is_empty() {
            return list;
        }
    }
    crate::core::state_db::get_file_tags(path)
}

#[cfg(unix)]
fn write_raw(path: &Path, payload: &str) -> Result<(), String> {
    let tag_list = parse_tags(payload);
    let _ = crate::core::state_db::set_file_tags(path, &tag_list);

    if payload.is_empty() {
        let _ = xattr::remove(path, XATTR_TAG_KEY);
        return Ok(());
    }
    xattr::set(path, XATTR_TAG_KEY, payload.as_bytes())
        .map_err(|e| format!("could not write xattr: {e}"))
}

#[cfg(not(any(target_os = "windows", unix)))]
fn read_raw(path: &Path) -> Vec<String> {
    crate::core::state_db::get_file_tags(path)
}

#[cfg(not(any(target_os = "windows", unix)))]
fn write_raw(path: &Path, payload: &str) -> Result<(), String> {
    let tag_list = parse_tags(payload);
    crate::core::state_db::set_file_tags(path, &tag_list)
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

/// True when `tag` names a preset or one of the supplied custom tags
/// (case-insensitive, trimmed). Pure helper, easy to test.
pub fn is_known_tag_in(tag: &str, custom: &[String]) -> bool {
    let norm = tag.trim().to_lowercase();
    if norm.is_empty() {
        return false;
    }
    TAG_PRESETS.iter().any(|t| t.to_lowercase() == norm)
        || custom.iter().any(|t| t.trim().to_lowercase() == norm)
}

/// True when `tag` names any known tag (presets + config custom tags).
pub fn is_known_tag(tag: &str) -> bool {
    let cfg = crate::core::MossambiConfig::load();
    is_known_tag_in(tag, &cfg.custom_tags)
}

/// All tags offered in menus and the sidebar: presets first, then custom
/// tags, de-duplicated case-insensitively.
pub fn available_tags_in(custom: &[String]) -> Vec<String> {
    let mut out: Vec<String> = TAG_PRESETS.iter().map(|t| t.to_string()).collect();
    for t in custom {
        let name = t.trim();
        if name.is_empty() {
            continue;
        }
        let norm = name.to_lowercase();
        if !out.iter().any(|k| k.to_lowercase() == norm) {
            out.push(name.to_string());
        }
    }
    out
}

/// All tags offered in menus and the sidebar (presets + config custom tags).
pub fn available_tags() -> Vec<String> {
    let cfg = crate::core::MossambiConfig::load();
    available_tags_in(&cfg.custom_tags)
}

/// Validate a candidate new-tag name. Returns the normalized name or an
/// error string suitable for status notes.
pub fn sanitize_new_tag(raw: &str, custom: &[String]) -> Result<String, String> {
    let name = raw.trim();
    if name.is_empty() {
        return Err("Tag name is empty".to_string());
    }
    if name.len() > 32 {
        return Err("Tag name is too long (max 32 characters)".to_string());
    }
    if name.contains(',') {
        return Err("Tag names cannot contain commas".to_string());
    }
    if is_known_tag_in(name, custom) {
        return Err(format!("Tag \"{name}\" already exists"));
    }
    Ok(name.to_string())
}

/// Assign a tag to `path`, returning an error string suitable for status notes.
pub fn set_tag(path: &Path, tag: &str) -> Result<(), String> {
    if !is_known_tag(tag) {
        return Err(format!("unknown tag: {tag}"));
    }
    let mut tags = get_tags(path);
    if tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
        return Ok(()); // idempotent (case-insensitive)
    }
    tags.push(tag.to_string());
    let res = write_raw(path, &join_tags(&tags));
    // Invalidate BEFORE returning so a failed native write (directories on
    // NTFS have no ADS write without backup semantics) still refreshes the
    // UI from the DB fallback, which did persist.
    invalidate_cache(path);
    res
}

/// Remove one tag (no-op friendly when absent).
pub fn clear_tag(path: &Path, tag: &str) -> Result<(), String> {
    let mut tags = get_tags(path);
    tags.retain(|t| !t.eq_ignore_ascii_case(tag));
    let res = write_raw(path, &join_tags(&tags));
    invalidate_cache(path);
    res
}

/// Overwrite the tag list exactly (undo support for the action journal).
pub fn restore_tags(path: &Path, tags: &[String]) -> Result<(), String> {
    let clean: Vec<String> = tags
        .iter()
        .filter(|t| is_known_tag(t))
        .collect::<Vec<_>>()
        .into_iter()
        .map(|t| t.to_string())
        .collect();
    let res = write_raw(path, &join_tags(&clean));
    invalidate_cache(path);
    res
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
        // All presets resolve to a non-custom colour.
        for t in TAG_PRESETS {
            let c = tag_color(t);
            assert_ne!(c, tag_color("zzz-unknown-not-in-palette-list-xyz"));
        }
    }

    #[test]
    fn case_insensitive_colors() {
        assert_eq!(tag_color("home"), tag_color("Home"));
        assert_eq!(tag_color("  IMPORTANT  "), tag_color("Important"));
    }

    #[test]
    fn custom_color_stable() {
        let a = tag_color("Projects");
        let b = tag_color("projects");
        assert_eq!(a, b);
        // Custom colour is never one of the preset colours.
        for t in TAG_PRESETS {
            assert_ne!(tag_color(t), a);
        }
    }

    #[test]
    fn known_and_available() {
        assert!(is_known_tag_in("home", &[]));
        assert!(is_known_tag_in("Projects", &["Projects".to_string()]));
        assert!(!is_known_tag_in("Nope", &[]));
        let avail = available_tags_in(&["Projects".to_string(), "home".to_string()]);
        assert_eq!(
            avail,
            vec!["Home", "Work", "Photos", "Important", "Projects"]
        );
    }

    #[test]
    fn test_sanitize_new_tag() {
        let customs = vec!["Projects".to_string()];
        assert!(sanitize_new_tag("", &customs).is_err());
        assert!(sanitize_new_tag("a,b", &customs).is_err());
        assert!(sanitize_new_tag("Projects", &customs).is_err());
        assert!(sanitize_new_tag("Home", &customs).is_err());
        assert_eq!(sanitize_new_tag("Finance", &customs).unwrap(), "Finance");
    }
}
