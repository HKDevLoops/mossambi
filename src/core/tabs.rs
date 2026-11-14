//! Tab management pure decision logic

/// Action to perform when a tab-close request (Ctrl+W / tab close button) is issued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseAction {
    /// Leave non-explorer tool view (Preferences, Transcoder, etc.) and return to previous tab.
    LeaveToolView,
    /// Close an explorer tab at the specified index.
    CloseExplorerTab(usize),
    /// Do nothing (e.g. only 1 explorer tab remains).
    Nothing,
}

/// Determines what action to take when the user requests closing a tab / view.
///
/// - If currently in a non-explorer view (tool/preferences), always returns `LeaveToolView`.
/// - If in `FileExplorer`:
///   - If more than 1 tab exists (`num_explorer_tabs > 1`), returns `CloseExplorerTab(active_tab_index)`.
///   - If only 1 tab exists, returns `Nothing`.
pub fn close_target(
    current_tab_is_explorer: bool,
    num_explorer_tabs: usize,
    active_tab_index: usize,
) -> CloseAction {
    if !current_tab_is_explorer {
        CloseAction::LeaveToolView
    } else if num_explorer_tabs > 1 {
        CloseAction::CloseExplorerTab(active_tab_index)
    } else {
        CloseAction::Nothing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_non_explorer_view_returns_leave_tool_view() {
        assert_eq!(close_target(false, 3, 1), CloseAction::LeaveToolView);
        assert_eq!(close_target(false, 1, 0), CloseAction::LeaveToolView);
        assert_eq!(close_target(false, 0, 0), CloseAction::LeaveToolView);
    }

    #[test]
    fn test_explorer_with_multiple_tabs_closes_active_tab() {
        assert_eq!(close_target(true, 3, 2), CloseAction::CloseExplorerTab(2));
        assert_eq!(close_target(true, 2, 0), CloseAction::CloseExplorerTab(0));
        assert_eq!(close_target(true, 5, 4), CloseAction::CloseExplorerTab(4));
    }

    #[test]
    fn test_explorer_with_single_or_zero_tabs_does_nothing() {
        assert_eq!(close_target(true, 1, 0), CloseAction::Nothing);
        assert_eq!(close_target(true, 0, 0), CloseAction::Nothing);
    }
}
