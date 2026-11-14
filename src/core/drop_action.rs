//! Pure decision logic for drag-and-drop actions.
//!
//! Follows Windows Explorer / macOS Finder conventions for modifiers, volume
//! boundaries, right-click drags, and user preferences.
//! 100% pure ASCII.

use std::path::{Component, Path, PathBuf};

use crate::core::config::DropActionPreference;

/// The specific file operation to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropActionKind {
    Move,
    Copy,
    Link,
}

impl DropActionKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Move => "Move",
            Self::Copy => "Copy",
            Self::Link => "Link",
        }
    }
}

/// Keyboard modifiers active during a drop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DropModifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl DropModifiers {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn ctrl() -> Self {
        Self {
            ctrl: true,
            shift: false,
            alt: false,
        }
    }

    pub fn shift() -> Self {
        Self {
            ctrl: false,
            shift: true,
            alt: false,
        }
    }

    pub fn alt() -> Self {
        Self {
            ctrl: false,
            shift: false,
            alt: true,
        }
    }

    pub fn ctrl_shift() -> Self {
        Self {
            ctrl: true,
            shift: true,
            alt: false,
        }
    }
}

/// Pointer mouse button used for the drag-and-drop gesture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DropMouseButton {
    #[default]
    Left,
    Right,
}

/// Reason why a drop operation is rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DropRejectReason {
    EmptySources,
    EmptyTargetDir,
    SelfDrop,
    DescendantDrop,
    SameDirectoryMove,
}

impl DropRejectReason {
    pub fn message(&self) -> &'static str {
        match self {
            Self::EmptySources => "No source items specified",
            Self::EmptyTargetDir => "Target directory is invalid",
            Self::SelfDrop => "Cannot drop a folder into itself",
            Self::DescendantDrop => "Cannot drop a folder into one of its subfolders",
            Self::SameDirectoryMove => "Source and destination are the same folder",
        }
    }
}

/// Decision output from the pure decision function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DropDecision {
    Immediate(DropActionKind),
    ShowMenu { default: DropActionKind },
    Reject(DropRejectReason),
}

/// Decides what action should occur when items are dropped onto a target directory.
///
/// Rules:
/// - Shift forces Move
/// - Ctrl forces Copy
/// - Ctrl+Shift or Alt forces Link (shortcut)
/// - Without modifiers, user preference (AlwaysCopy / AlwaysMove) is honored
/// - Under SmartDefault (or AlwaysAsk without modifiers): same volume defaults to Move, cross-volume to Copy
/// - Right-button drag or setting "Always ask" displays the drop action menu
/// - Reject dropping a folder into itself or a descendant
/// - Reject same-directory move (no-op)
pub fn decide_drop_action(
    sources: &[PathBuf],
    target_dir: &Path,
    modifiers: DropModifiers,
    mouse_button: DropMouseButton,
    same_volume: bool,
    setting: DropActionPreference,
) -> DropDecision {
    if sources.is_empty() {
        return DropDecision::Reject(DropRejectReason::EmptySources);
    }
    if target_dir.as_os_str().is_empty() {
        return DropDecision::Reject(DropRejectReason::EmptyTargetDir);
    }

    // Check for dropping an item into itself or a descendant
    for src in sources {
        if src == target_dir {
            return DropDecision::Reject(DropRejectReason::SelfDrop);
        }
        if target_dir.starts_with(src) {
            return DropDecision::Reject(DropRejectReason::DescendantDrop);
        }
    }

    // Determine the base intended action according to Windows convention
    let intended_action = if modifiers.alt || (modifiers.ctrl && modifiers.shift) {
        DropActionKind::Link
    } else if modifiers.shift {
        DropActionKind::Move
    } else if modifiers.ctrl {
        DropActionKind::Copy
    } else {
        match setting {
            DropActionPreference::AlwaysCopy => DropActionKind::Copy,
            DropActionPreference::AlwaysMove => DropActionKind::Move,
            DropActionPreference::SmartDefault | DropActionPreference::AlwaysAsk => {
                if same_volume {
                    DropActionKind::Move
                } else {
                    DropActionKind::Copy
                }
            }
        }
    };

    let all_in_target = sources.iter().all(|p| {
        p.parent()
            .map(|parent| parent == target_dir)
            .unwrap_or(false)
    });

    let show_menu =
        mouse_button == DropMouseButton::Right || setting == DropActionPreference::AlwaysAsk;

    if show_menu {
        DropDecision::ShowMenu {
            default: intended_action,
        }
    } else {
        if intended_action == DropActionKind::Move && all_in_target {
            return DropDecision::Reject(DropRejectReason::SameDirectoryMove);
        }
        DropDecision::Immediate(intended_action)
    }
}

/// Returns true if two paths belong to the same filesystem volume or drive.
pub fn is_same_volume(a: &Path, b: &Path) -> bool {
    let a_root = a.components().next();
    let b_root = b.components().next();
    match (a_root, b_root) {
        (Some(Component::Prefix(p1)), Some(Component::Prefix(p2))) => {
            p1.as_os_str().eq_ignore_ascii_case(p2.as_os_str())
        }
        _ => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if let (Ok(m1), Ok(m2)) = (a.metadata(), b.metadata()) {
                    return m1.dev() == m2.dev();
                }
            }
            true
        }
    }
}

/// Returns true if all source paths are on the same volume as the target directory.
pub fn paths_on_same_volume(sources: &[PathBuf], target_dir: &Path) -> bool {
    sources.iter().all(|src| is_same_volume(src, target_dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_sources_rejected() {
        let res = decide_drop_action(
            &[],
            Path::new("C:/dest"),
            DropModifiers::none(),
            DropMouseButton::Left,
            true,
            DropActionPreference::SmartDefault,
        );
        assert_eq!(res, DropDecision::Reject(DropRejectReason::EmptySources));
    }

    #[test]
    fn test_empty_target_dir_rejected() {
        let sources = vec![PathBuf::from("C:/src/file.txt")];
        let res = decide_drop_action(
            &sources,
            Path::new(""),
            DropModifiers::none(),
            DropMouseButton::Left,
            true,
            DropActionPreference::SmartDefault,
        );
        assert_eq!(res, DropDecision::Reject(DropRejectReason::EmptyTargetDir));
    }

    #[test]
    fn test_self_drop_rejected() {
        let folder = PathBuf::from("C:/my_folder");
        let sources = vec![folder.clone()];
        let res = decide_drop_action(
            &sources,
            &folder,
            DropModifiers::none(),
            DropMouseButton::Left,
            true,
            DropActionPreference::SmartDefault,
        );
        assert_eq!(res, DropDecision::Reject(DropRejectReason::SelfDrop));
    }

    #[test]
    fn test_descendant_drop_rejected() {
        let parent = PathBuf::from("C:/parent");
        let child = PathBuf::from("C:/parent/child/deep");
        let sources = vec![parent];
        let res = decide_drop_action(
            &sources,
            &child,
            DropModifiers::none(),
            DropMouseButton::Left,
            true,
            DropActionPreference::SmartDefault,
        );
        assert_eq!(res, DropDecision::Reject(DropRejectReason::DescendantDrop));
    }

    #[test]
    fn test_shift_modifier_forces_move() {
        let sources = vec![PathBuf::from("C:/folder_a/file.txt")];
        let dest = PathBuf::from("C:/folder_b");
        let res = decide_drop_action(
            &sources,
            &dest,
            DropModifiers::shift(),
            DropMouseButton::Left,
            false,                            // cross-volume
            DropActionPreference::AlwaysCopy, // even if preference says AlwaysCopy
        );
        assert_eq!(res, DropDecision::Immediate(DropActionKind::Move));
    }

    #[test]
    fn test_ctrl_modifier_forces_copy() {
        let sources = vec![PathBuf::from("C:/folder_a/file.txt")];
        let dest = PathBuf::from("C:/folder_b");
        let res = decide_drop_action(
            &sources,
            &dest,
            DropModifiers::ctrl(),
            DropMouseButton::Left,
            true,                             // same volume
            DropActionPreference::AlwaysMove, // even if preference says AlwaysMove
        );
        assert_eq!(res, DropDecision::Immediate(DropActionKind::Copy));
    }

    #[test]
    fn test_alt_modifier_forces_link() {
        let sources = vec![PathBuf::from("C:/folder_a/file.txt")];
        let dest = PathBuf::from("C:/folder_b");
        let res = decide_drop_action(
            &sources,
            &dest,
            DropModifiers::alt(),
            DropMouseButton::Left,
            true,
            DropActionPreference::SmartDefault,
        );
        assert_eq!(res, DropDecision::Immediate(DropActionKind::Link));
    }

    #[test]
    fn test_ctrl_shift_modifier_forces_link() {
        let sources = vec![PathBuf::from("C:/folder_a/file.txt")];
        let dest = PathBuf::from("C:/folder_b");
        let res = decide_drop_action(
            &sources,
            &dest,
            DropModifiers::ctrl_shift(),
            DropMouseButton::Left,
            true,
            DropActionPreference::SmartDefault,
        );
        assert_eq!(res, DropDecision::Immediate(DropActionKind::Link));
    }

    #[test]
    fn test_smart_default_same_volume() {
        let sources = vec![PathBuf::from("C:/folder_a/file.txt")];
        let dest = PathBuf::from("C:/folder_b");
        let res = decide_drop_action(
            &sources,
            &dest,
            DropModifiers::none(),
            DropMouseButton::Left,
            true, // same volume -> Move
            DropActionPreference::SmartDefault,
        );
        assert_eq!(res, DropDecision::Immediate(DropActionKind::Move));
    }

    #[test]
    fn test_smart_default_cross_volume() {
        let sources = vec![PathBuf::from("C:/folder_a/file.txt")];
        let dest = PathBuf::from("D:/folder_b");
        let res = decide_drop_action(
            &sources,
            &dest,
            DropModifiers::none(),
            DropMouseButton::Left,
            false, // cross-volume -> Copy
            DropActionPreference::SmartDefault,
        );
        assert_eq!(res, DropDecision::Immediate(DropActionKind::Copy));
    }

    #[test]
    fn test_always_copy_preference() {
        let sources = vec![PathBuf::from("C:/folder_a/file.txt")];
        let dest = PathBuf::from("C:/folder_b");
        let res = decide_drop_action(
            &sources,
            &dest,
            DropModifiers::none(),
            DropMouseButton::Left,
            true, // same volume, but preference is AlwaysCopy
            DropActionPreference::AlwaysCopy,
        );
        assert_eq!(res, DropDecision::Immediate(DropActionKind::Copy));
    }

    #[test]
    fn test_always_move_preference() {
        let sources = vec![PathBuf::from("C:/folder_a/file.txt")];
        let dest = PathBuf::from("D:/folder_b");
        let res = decide_drop_action(
            &sources,
            &dest,
            DropModifiers::none(),
            DropMouseButton::Left,
            false, // cross-volume, but preference is AlwaysMove
            DropActionPreference::AlwaysMove,
        );
        assert_eq!(res, DropDecision::Immediate(DropActionKind::Move));
    }

    #[test]
    fn test_right_click_shows_menu() {
        let sources = vec![PathBuf::from("C:/folder_a/file.txt")];
        let dest = PathBuf::from("C:/folder_b");
        let res = decide_drop_action(
            &sources,
            &dest,
            DropModifiers::none(),
            DropMouseButton::Right, // Right mouse button drag
            true,
            DropActionPreference::SmartDefault,
        );
        assert_eq!(
            res,
            DropDecision::ShowMenu {
                default: DropActionKind::Move
            }
        );
    }

    #[test]
    fn test_always_ask_preference_shows_menu() {
        let sources = vec![PathBuf::from("C:/folder_a/file.txt")];
        let dest = PathBuf::from("D:/folder_b");
        let res = decide_drop_action(
            &sources,
            &dest,
            DropModifiers::none(),
            DropMouseButton::Left,
            false,
            DropActionPreference::AlwaysAsk, // Always ask preference
        );
        assert_eq!(
            res,
            DropDecision::ShowMenu {
                default: DropActionKind::Copy
            }
        );
    }

    #[test]
    fn test_same_directory_move_rejected() {
        let dest = PathBuf::from("C:/folder_a");
        let sources = vec![PathBuf::from("C:/folder_a/file.txt")];
        let res = decide_drop_action(
            &sources,
            &dest,
            DropModifiers::none(),
            DropMouseButton::Left,
            true,
            DropActionPreference::SmartDefault,
        );
        assert_eq!(
            res,
            DropDecision::Reject(DropRejectReason::SameDirectoryMove)
        );
    }

    #[test]
    fn test_same_directory_ctrl_copy_allowed() {
        let dest = PathBuf::from("C:/folder_a");
        let sources = vec![PathBuf::from("C:/folder_a/file.txt")];
        let res = decide_drop_action(
            &sources,
            &dest,
            DropModifiers::ctrl(),
            DropMouseButton::Left,
            true,
            DropActionPreference::SmartDefault,
        );
        assert_eq!(res, DropDecision::Immediate(DropActionKind::Copy));
    }

    #[test]
    fn test_same_directory_alt_link_allowed() {
        let dest = PathBuf::from("C:/folder_a");
        let sources = vec![PathBuf::from("C:/folder_a/file.txt")];
        let res = decide_drop_action(
            &sources,
            &dest,
            DropModifiers::alt(),
            DropMouseButton::Left,
            true,
            DropActionPreference::SmartDefault,
        );
        assert_eq!(res, DropDecision::Immediate(DropActionKind::Link));
    }

    #[test]
    fn test_same_directory_right_click_shows_menu() {
        let dest = PathBuf::from("C:/folder_a");
        let sources = vec![PathBuf::from("C:/folder_a/file.txt")];
        let res = decide_drop_action(
            &sources,
            &dest,
            DropModifiers::none(),
            DropMouseButton::Right,
            true,
            DropActionPreference::SmartDefault,
        );
        assert_eq!(
            res,
            DropDecision::ShowMenu {
                default: DropActionKind::Move
            }
        );
    }

    #[test]
    fn test_is_same_volume_windows_prefixes() {
        let p1 = Path::new("C:/Users/test/file.txt");
        let p2 = Path::new("c:/Program Files/test");
        let p3 = Path::new("D:/Data/file.txt");

        #[cfg(target_os = "windows")]
        {
            assert!(is_same_volume(p1, p2));
            assert!(!is_same_volume(p1, p3));
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = (p1, p2, p3);
        }
    }
}
