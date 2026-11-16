//! Unit tests for the W4b dialog companion bar pure functions.
//!
//! Pure 7-bit ASCII.
//! - is_file_dialog: accepts the modern common file dialog signature,
//!   rejects non-dialog classes and plain message boxes.
//! - dock_rect: docks right of the dialog when it fits, falls back left.

#[cfg(windows)]
use mossambi::platform::windows::dialog_companion::{dock_rect, is_file_dialog};

#[test]
fn test_is_file_dialog_accepts_modern_file_dialog() {
    #[cfg(windows)]
    {
        let class = "#32770";
        let descendants = [
            "DirectUIHWND",
            "CtrlNotifySink",
            "ComboBoxEx32",
            "Edit",
            "Static",
            "Button",
        ];
        assert!(is_file_dialog(class, &descendants));
    }
}

#[test]
fn test_is_file_dialog_rejects_wrong_top_class() {
    #[cfg(windows)]
    {
        let class = "Notepad";
        let descendants = ["DirectUIHWND", "ComboBoxEx32", "Edit"];
        assert!(!is_file_dialog(class, &descendants));
    }
}

#[test]
fn test_is_file_dialog_rejects_message_box() {
    #[cfg(windows)]
    {
        // A MessageBox is "#32770" too, but carries no file dialog markers.
        let class = "#32770";
        let descendants = ["Static", "Button"];
        assert!(!is_file_dialog(class, &descendants));
    }
}

#[test]
fn test_is_file_dialog_rejects_dialog_without_filename_edit() {
    #[cfg(windows)]
    {
        let class = "#32770";
        let descendants = ["DirectUIHWND"];
        assert!(!is_file_dialog(class, &descendants));
    }
}

#[test]
fn test_is_file_dialog_is_case_insensitive_on_descendants() {
    #[cfg(windows)]
    {
        let class = "#32770";
        let descendants = ["directuihwnd", "comboboxex32", "EDIT"];
        assert!(is_file_dialog(class, &descendants));
    }
}

#[test]
fn test_dock_rect_docks_right_when_it_fits() {
    #[cfg(windows)]
    {
        let dialog = (100, 100, 400, 300);
        let screen = (0, 0, 1920, 1080);
        assert_eq!(dock_rect(dialog, 180, screen), (500, 100, 180, 300));
    }
}

#[test]
fn test_dock_rect_falls_back_to_left_when_right_overflows() {
    #[cfg(windows)]
    {
        let dialog = (1000, 100, 900, 300);
        let screen = (0, 0, 1920, 1080);
        // Right dock would end at 1900 + 180 = 2080 > 1920; dock left instead.
        assert_eq!(dock_rect(dialog, 180, screen), (820, 100, 180, 300));
    }
}
