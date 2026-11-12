//! Miller lineage model tests: ancestor chain construction and the
//! KeepStack / ShiftTo / ClearStack navigation policy that drives
//! Finder-conformant Miller columns.

use mossambi::core::FileItem;
use mossambi::gui::explorer_view::{ancestor_chain, lineage_policy, MillerLineage};
use std::path::{Path, PathBuf};

/// Builds a fake Miller column stack from directory paths; the entry
/// payloads are empty because lineage_policy never reads them.
fn stack_paths(dirs: &[&str]) -> Vec<(PathBuf, Vec<FileItem>)> {
    dirs.iter()
        .map(|d| (PathBuf::from(d), Vec::new()))
        .collect()
}

#[test]
fn test_ancestor_chain_root_first_excludes_self() {
    let chain = ancestor_chain(Path::new("Z:\\Projects\\mossambi"));
    assert_eq!(
        chain,
        vec![PathBuf::from("Z:\\"), PathBuf::from("Z:\\Projects")]
    );
}

#[test]
fn test_ancestor_chain_single_level() {
    let chain = ancestor_chain(Path::new("Z:\\Users"));
    assert_eq!(chain, vec![PathBuf::from("Z:\\")]);
}

#[test]
fn test_ancestor_chain_at_root_is_empty() {
    assert!(ancestor_chain(Path::new("Z:\\")).is_empty());
}

#[test]
fn test_ancestor_chain_relative_is_empty() {
    assert!(ancestor_chain(Path::new("foo\\bar")).is_empty());
}

#[test]
fn test_policy_same_dir_keeps_stack() {
    let stack = stack_paths(&["Z:\\a\\b", "Z:\\a\\b\\c"]);
    let p = lineage_policy(Path::new("Z:\\a"), Path::new("Z:\\a"), &stack);
    assert_eq!(p, MillerLineage::KeepStack);
}

#[test]
fn test_policy_stack_hit_mid_shifts() {
    let stack = stack_paths(&["Z:\\a\\b", "Z:\\a\\b\\c", "Z:\\a\\b\\c\\d"]);
    let p = lineage_policy(Path::new("Z:\\a\\b\\c"), Path::new("Z:\\a"), &stack);
    assert_eq!(p, MillerLineage::ShiftTo(1));
}

#[test]
fn test_policy_deepest_stack_hit_shifts_to_last() {
    let stack = stack_paths(&["Z:\\a\\b", "Z:\\a\\b\\c"]);
    let p = lineage_policy(Path::new("Z:\\a\\b\\c"), Path::new("Z:\\a"), &stack);
    assert_eq!(p, MillerLineage::ShiftTo(1));
}

#[test]
fn test_policy_up_nav_clears_stack() {
    let stack = stack_paths(&["Z:\\a\\b"]);
    let p = lineage_policy(Path::new("Z:\\a"), Path::new("Z:\\a\\b"), &stack);
    assert_eq!(p, MillerLineage::ClearStack);
}

#[test]
fn test_policy_unrelated_clears_stack() {
    let stack = stack_paths(&["Z:\\a\\b"]);
    let p = lineage_policy(Path::new("Z:\\other"), Path::new("Z:\\a"), &stack);
    assert_eq!(p, MillerLineage::ClearStack);
}

#[test]
fn test_policy_empty_stack_clears() {
    let p = lineage_policy(Path::new("Z:\\x"), Path::new("Z:\\y"), &[]);
    assert_eq!(p, MillerLineage::ClearStack);
}
