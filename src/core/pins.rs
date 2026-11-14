//! Pure logic for pinned sidebar folders

use std::path::{Path, PathBuf};

/// Normalizes a path string or reference for uniform storage and comparison.
pub fn normalize_path(path: &Path) -> PathBuf {
    if let Ok(canon) = path.canonicalize() {
        let s = canon.to_string_lossy();
        if let Some(stripped) = s.strip_prefix(r"\\?\") {
            PathBuf::from(stripped)
        } else {
            canon
        }
    } else {
        path.to_path_buf()
    }
}

/// Checks if two path strings are equal (case-insensitive on Windows).
pub fn paths_equal(a: &str, b: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        a.eq_ignore_ascii_case(b)
    }
    #[cfg(not(target_os = "windows"))]
    {
        a == b
    }
}

/// Checks if `list` contains the given path.
pub fn contains(list: &[String], path: &Path) -> bool {
    let norm = normalize_path(path);
    let norm_str = norm.to_string_lossy();
    let raw_str = path.to_string_lossy();
    list.iter()
        .any(|item| paths_equal(item, &norm_str) || paths_equal(item, &raw_str))
}

/// Adds `path` to `list` if it exists, is a directory, and is not already present.
/// Deduplication is case-insensitive on Windows.
/// Returns `true` if added, `false` otherwise.
pub fn add(list: &mut Vec<String>, path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }
    if contains(list, path) {
        return false;
    }
    let norm = normalize_path(path);
    list.push(norm.to_string_lossy().into_owned());
    true
}

/// Removes `path` from `list`. Matches case-insensitively on Windows.
/// Returns `true` if an item was removed, `false` otherwise.
pub fn remove(list: &mut Vec<String>, path: &Path) -> bool {
    let norm = normalize_path(path);
    let norm_str = norm.to_string_lossy();
    let raw_str = path.to_string_lossy();
    let initial_len = list.len();
    list.retain(|item| !paths_equal(item, &norm_str) && !paths_equal(item, &raw_str));
    list.len() < initial_len
}

/// Moves an item from index `from` to index `to` in `list`.
/// Returns `true` on success, `false` if `from` or `to` is out of bounds.
pub fn move_item(list: &mut Vec<String>, from: usize, to: usize) -> bool {
    if from >= list.len() || to >= list.len() {
        return false;
    }
    if from == to {
        return true;
    }
    let item = list.remove(from);
    list.insert(to, item);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_directory_success() {
        let temp_dir = std::env::temp_dir();
        let mut list = Vec::new();
        assert!(add(&mut list, &temp_dir));
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn test_add_non_directory_fails() {
        let temp_file = std::env::temp_dir().join("mossambi_non_dir_test.txt");
        let _ = std::fs::write(&temp_file, "test");
        let mut list = Vec::new();
        assert!(!add(&mut list, &temp_file));
        assert!(list.is_empty());
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_add_deduplication() {
        let temp_dir = std::env::temp_dir();
        let mut list = Vec::new();
        assert!(add(&mut list, &temp_dir));
        assert!(!add(&mut list, &temp_dir));
        assert_eq!(list.len(), 1);

        #[cfg(target_os = "windows")]
        {
            let path_str = temp_dir.to_string_lossy().to_uppercase();
            assert!(!add(&mut list, Path::new(&path_str)));
            assert_eq!(list.len(), 1);
        }
    }

    #[test]
    fn test_remove_item() {
        let temp_dir = std::env::temp_dir();
        let mut list = Vec::new();
        add(&mut list, &temp_dir);
        assert_eq!(list.len(), 1);
        assert!(remove(&mut list, &temp_dir));
        assert!(list.is_empty());
        assert!(!remove(&mut list, &temp_dir));
    }

    #[test]
    fn test_move_item() {
        let mut list = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        // Move from 0 to 2 -> B, C, A
        assert!(move_item(&mut list, 0, 2));
        assert_eq!(list, vec!["B", "C", "A"]);

        // Move same index
        assert!(move_item(&mut list, 1, 1));
        assert_eq!(list, vec!["B", "C", "A"]);

        // Out of bounds
        assert!(!move_item(&mut list, 0, 5));
        assert!(!move_item(&mut list, 5, 0));
    }
}
