//! Pure decision helper for window level / topmost management

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesiredWindowLevel {
    AlwaysOnTop,
    Normal,
}

/// Evaluates whether a new window level command should be sent to the OS viewport.
///
/// Returns `Some(DesiredWindowLevel)` only when the current setting differs from `last_applied`.
/// If `last_applied` already matches `current_setting`, returns `None` to avoid spamming the OS.
pub fn should_apply_window_level(
    current_setting: bool,
    last_applied: Option<bool>,
) -> Option<DesiredWindowLevel> {
    if last_applied == Some(current_setting) {
        None
    } else if current_setting {
        Some(DesiredWindowLevel::AlwaysOnTop)
    } else {
        Some(DesiredWindowLevel::Normal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_application() {
        assert_eq!(
            should_apply_window_level(true, None),
            Some(DesiredWindowLevel::AlwaysOnTop)
        );
        assert_eq!(
            should_apply_window_level(false, None),
            Some(DesiredWindowLevel::Normal)
        );
    }

    #[test]
    fn test_no_change_when_matching() {
        assert_eq!(should_apply_window_level(true, Some(true)), None);
        assert_eq!(should_apply_window_level(false, Some(false)), None);
    }

    #[test]
    fn test_state_transitions() {
        assert_eq!(
            should_apply_window_level(true, Some(false)),
            Some(DesiredWindowLevel::AlwaysOnTop)
        );
        assert_eq!(
            should_apply_window_level(false, Some(true)),
            Some(DesiredWindowLevel::Normal)
        );
    }
}
