//! Miller lineage model tests: ancestor chain construction and the
//! KeepStack / ShiftTo / ClearStack navigation policy that drives
//! Finder-conformant Miller columns.

use mossambi::core::FileItem;
use mossambi::gui::explorer_view::{ancestor_chain, lineage_policy, MillerLineage};
use std::path::{Path, PathBuf};

fn make_path(parts: &[&str]) -> PathBuf {
    #[cfg(windows)]
    let mut p = PathBuf::from("C:\\");
    #[cfg(not(windows))]
    let mut p = PathBuf::from("/");
    for part in parts {
        p.push(part);
    }
    p
}

/// Builds a fake Miller column stack from directory paths; the entry
/// payloads are empty because lineage_policy never reads them.
fn stack_paths(dirs: &[PathBuf]) -> Vec<(PathBuf, Vec<FileItem>)> {
    dirs.iter().map(|d| (d.clone(), Vec::new())).collect()
}

#[test]
fn test_ancestor_chain_root_first_excludes_self() {
    let path = make_path(&["Projects", "mossambi"]);
    let chain = ancestor_chain(&path);
    assert_eq!(chain, vec![make_path(&[]), make_path(&["Projects"])]);
}

#[test]
fn test_ancestor_chain_single_level() {
    let path = make_path(&["Users"]);
    let chain = ancestor_chain(&path);
    assert_eq!(chain, vec![make_path(&[])]);
}

#[test]
fn test_ancestor_chain_at_root_is_empty() {
    assert!(ancestor_chain(&make_path(&[])).is_empty());
}

#[test]
fn test_ancestor_chain_relative_is_empty() {
    let rel = Path::new("foo").join("bar");
    assert!(ancestor_chain(&rel).is_empty());
}

#[test]
fn test_policy_same_dir_keeps_stack() {
    let a = make_path(&["a"]);
    let ab = make_path(&["a", "b"]);
    let abc = make_path(&["a", "b", "c"]);
    let stack = stack_paths(&[ab, abc]);
    let p = lineage_policy(&a, &a, &stack);
    assert_eq!(p, MillerLineage::KeepStack);
}

#[test]
fn test_policy_stack_hit_mid_shifts() {
    let a = make_path(&["a"]);
    let ab = make_path(&["a", "b"]);
    let abc = make_path(&["a", "b", "c"]);
    let abcd = make_path(&["a", "b", "c", "d"]);
    let stack = stack_paths(&[ab, abc.clone(), abcd]);
    let p = lineage_policy(&abc, &a, &stack);
    assert_eq!(p, MillerLineage::ShiftTo(1));
}

#[test]
fn test_policy_deepest_stack_hit_shifts_to_last() {
    let a = make_path(&["a"]);
    let ab = make_path(&["a", "b"]);
    let abc = make_path(&["a", "b", "c"]);
    let stack = stack_paths(&[ab, abc.clone()]);
    let p = lineage_policy(&abc, &a, &stack);
    assert_eq!(p, MillerLineage::ShiftTo(1));
}

#[test]
fn test_policy_up_nav_clears_stack() {
    let a = make_path(&["a"]);
    let ab = make_path(&["a", "b"]);
    let stack = stack_paths(std::slice::from_ref(&ab));
    let p = lineage_policy(&a, &ab, &stack);
    assert_eq!(p, MillerLineage::ClearStack);
}

#[test]
fn test_policy_unrelated_clears_stack() {
    let a = make_path(&["a"]);
    let ab = make_path(&["a", "b"]);
    let other = make_path(&["other"]);
    let stack = stack_paths(&[ab]);
    let p = lineage_policy(&other, &a, &stack);
    assert_eq!(p, MillerLineage::ClearStack);
}

#[test]
fn test_policy_empty_stack_clears() {
    let x = make_path(&["x"]);
    let y = make_path(&["y"]);
    let p = lineage_policy(&x, &y, &[]);
    assert_eq!(p, MillerLineage::ClearStack);
}
