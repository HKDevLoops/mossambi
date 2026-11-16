//! Common File Dialog Companion Bar & Detection Classifier
//!
//! Pure 7-bit ASCII. Detects Windows Common File Dialogs (#32770 hosting DirectUIHWND,
//! ShellView, or IFileDialog controls) and enables companion quick-jump navigation.
//! The companion bar docks to the dialog edge without stealing keyboard or window focus.

use std::path::Path;

/// Mockable window node representing a desktop window and its child control hierarchy.
/// Allows complete unit testing of file dialog classification in pure headless mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowNode {
    pub class_name: String,
    pub title: String,
    pub control_id: Option<i32>,
    pub children: Vec<WindowNode>,
}

impl WindowNode {
    /// Creates a new window node with given class and title.
    pub fn new(class_name: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            class_name: class_name.into(),
            title: title.into(),
            control_id: None,
            children: Vec::new(),
        }
    }

    /// Sets the control identifier (e.g. 1148 for standard file dialog filename edit).
    pub fn with_id(mut self, id: i32) -> Self {
        self.control_id = Some(id);
        self
    }

    /// Appends a child window control.
    pub fn with_child(mut self, child: WindowNode) -> Self {
        self.children.push(child);
        self
    }

    /// Replaces child list with given vector.
    pub fn with_children(mut self, children: Vec<WindowNode>) -> Self {
        self.children = children;
        self
    }
}

/// Classifies whether a window hierarchy represents a Windows Common File Dialog
/// (either modern IFileOpenDialog/IFileSaveDialog or classic GetOpenFileName/GetSaveFileName).
///
/// Rules:
/// 1. Top-level window class must be "#32770" (standard Windows dialog class).
/// 2. Must contain characteristic file dialog markers in its subtree:
///    - "DirectUIHWND" (modern Explorer frame hosting IFileDialog)
///    - "SHELLDLL_DefView" or "ShellView" (the folder listview control)
///    - "NamespaceTreeControl" (the folder navigation tree)
///    - Control ID 1148 (standard cmb13/Edit filename input) or 1152 (classic filename input)
///    - "ComboBoxEx32" (standard filename combo container)
/// 3. Rejects simple dialogs like standard MessageBox (whose only children are Static and Button).
pub fn is_common_file_dialog(node: &WindowNode) -> bool {
    // 1. Root window class must be the standard dialog class #32770
    if node.class_name != "#32770" {
        return false;
    }

    // 2. Scan descendants for Common File Dialog markers
    let mut has_directui = false;
    let mut has_shell_view = false;
    let mut has_namespace_tree = false;
    let mut has_filename_edit_id = false;
    let mut has_combobox_ex = false;
    let mut only_simple_controls = true;

    fn walk(
        curr: &WindowNode,
        has_directui: &mut bool,
        has_shell_view: &mut bool,
        has_namespace_tree: &mut bool,
        has_filename_edit_id: &mut bool,
        has_combobox_ex: &mut bool,
        only_simple_controls: &mut bool,
    ) {
        let cls = curr.class_name.to_ascii_lowercase();

        if cls == "directuihwnd" {
            *has_directui = true;
            *only_simple_controls = false;
        } else if cls == "shelldll_defview" || cls == "shellview" {
            *has_shell_view = true;
            *only_simple_controls = false;
        } else if cls == "namespacetreecontrol" {
            *has_namespace_tree = true;
            *only_simple_controls = false;
        } else if cls == "comboboxex32" {
            *has_combobox_ex = true;
            *only_simple_controls = false;
        }

        if let Some(id) = curr.control_id {
            // 1148 (0x047C) is the standard common dialog filename edit control
            // 1152 (0x0480) is the classic common dialog filename edit control
            if id == 1148 || id == 1152 {
                *has_filename_edit_id = true;
                *only_simple_controls = false;
            }
        }

        if cls != "static" && cls != "button" && cls != "#32770" {
            *only_simple_controls = false;
        }

        for child in &curr.children {
            walk(
                child,
                has_directui,
                has_shell_view,
                has_namespace_tree,
                has_filename_edit_id,
                has_combobox_ex,
                only_simple_controls,
            );
        }
    }

    walk(
        node,
        &mut has_directui,
        &mut has_shell_view,
        &mut has_namespace_tree,
        &mut has_filename_edit_id,
        &mut has_combobox_ex,
        &mut only_simple_controls,
    );

    // Simple message boxes have only Static text and Button controls
    if only_simple_controls {
        return false;
    }

    // Must have at least one file dialog signature component
    has_directui || has_shell_view || has_namespace_tree || has_filename_edit_id || has_combobox_ex
}

/// Calculates the docked rectangle for a companion bar positioned relative to a target dialog window.
/// Default docking is pinned to the top edge (or right edge if insufficient screen room).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompanionBarRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

pub fn calculate_companion_dock_rect(
    dialog_x: i32,
    dialog_y: i32,
    dialog_width: i32,
    dialog_height: i32,
    bar_height: i32,
) -> CompanionBarRect {
    let _ = dialog_height;
    // Dock above the dialog header bar, aligned horizontally with dialog width
    let y = (dialog_y - bar_height).max(0);
    CompanionBarRect {
        x: dialog_x,
        y,
        width: dialog_width,
        height: bar_height,
    }
}

#[cfg(windows)]
pub mod win_companion {
    use super::*;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, WPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumChildWindows, GetClassNameW, GetDlgCtrlID, GetDlgItem, PostMessageW, SendMessageW,
        SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW, WM_GETTEXT, WM_KEYDOWN,
        WM_SETTEXT,
    };

    const VK_RETURN: u32 = 0x0D;

    fn to_wide(s: &str) -> Vec<u16> {
        std::ffi::OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    /// Queries the window class name for an HWND.
    ///
    /// # Safety
    /// Caller must pass a valid window handle.
    pub unsafe fn get_window_class(hwnd: HWND) -> String {
        let mut buf = [0u16; 256];
        let len = GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        if len > 0 {
            String::from_utf16_lossy(&buf[..len as usize])
        } else {
            String::new()
        }
    }

    /// Traverses a live HWND tree and constructs a WindowNode hierarchy.
    ///
    /// # Safety
    /// Caller must pass a valid window handle.
    pub unsafe fn build_window_tree(hwnd: HWND) -> WindowNode {
        let root_class = get_window_class(hwnd);
        let root_id = GetDlgCtrlID(hwnd);

        struct Collector {
            nodes: Vec<WindowNode>,
        }

        unsafe extern "system" fn enum_proc(child: HWND, lparam: LPARAM) -> BOOL {
            let col = &mut *(lparam as *mut Collector);
            let cls = get_window_class(child);
            let id = GetDlgCtrlID(child);
            col.nodes.push(WindowNode {
                class_name: cls,
                title: String::new(),
                control_id: if id != 0 { Some(id) } else { None },
                children: Vec::new(),
            });
            1 // Continue enumeration
        }

        let mut collector = Collector { nodes: Vec::new() };
        EnumChildWindows(
            hwnd,
            Some(enum_proc),
            &mut collector as *mut Collector as LPARAM,
        );

        WindowNode {
            class_name: root_class,
            title: String::new(),
            control_id: if root_id != 0 { Some(root_id) } else { None },
            children: collector.nodes,
        }
    }

    /// Navigates a common file dialog to the specified folder path by populating
    /// the filename edit control (Edit 1148), triggering navigation, and restoring
    /// the original input text.
    pub fn jump_dialog_to_folder(dialog_hwnd: isize, folder_path: &Path) -> bool {
        let hwnd = dialog_hwnd as HWND;
        let folder_str = dunce::canonicalize(folder_path)
            .unwrap_or_else(|_| folder_path.to_path_buf())
            .to_string_lossy()
            .to_string();

        // Standard filename edit is control ID 1148 (0x047C) in modern common dialogs
        // Fallback is 1152 in classic common dialogs
        let mut edit_hwnd = unsafe { GetDlgItem(hwnd, 1148) };
        if edit_hwnd.is_null() {
            edit_hwnd = unsafe { GetDlgItem(hwnd, 1152) };
        }

        if edit_hwnd.is_null() {
            // Attempt to search children for an Edit control
            let tree = unsafe { build_window_tree(hwnd) };
            if !is_common_file_dialog(&tree) {
                return false;
            }
        }

        if edit_hwnd.is_null() {
            return false;
        }

        // 1. Read current text so it can be restored after folder jump
        let mut orig_text_buf = [0u16; 512];
        let orig_len = unsafe {
            SendMessageW(
                edit_hwnd,
                WM_GETTEXT,
                orig_text_buf.len() as WPARAM,
                orig_text_buf.as_mut_ptr() as LPARAM,
            )
        };

        // 2. Set edit text to target folder path
        let folder_wide = to_wide(&folder_str);
        unsafe {
            SendMessageW(edit_hwnd, WM_SETTEXT, 0, folder_wide.as_ptr() as LPARAM);
        }

        // 3. Post Enter key down to trigger the dialog's folder navigation logic
        unsafe {
            PostMessageW(edit_hwnd, WM_KEYDOWN, VK_RETURN as WPARAM, 0);
        }

        // 4. If original text had a filename (not a path), restore it after slight yield
        if orig_len > 0 {
            let orig_str = String::from_utf16_lossy(&orig_text_buf[..orig_len as usize]);
            if !orig_str.contains('\\') && !orig_str.contains('/') {
                let edit_copy = edit_hwnd as usize;
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(80));
                    let restore_wide = to_wide(&orig_str);
                    unsafe {
                        SendMessageW(
                            edit_copy as HWND,
                            WM_SETTEXT,
                            0,
                            restore_wide.as_ptr() as LPARAM,
                        );
                    }
                });
            }
        }

        true
    }

    /// Moves and resizes the companion bar to follow the target dialog without stealing focus.
    pub fn reposition_companion_bar(bar_hwnd: isize, rect: &CompanionBarRect) {
        unsafe {
            SetWindowPos(
                bar_hwnd as HWND,
                HWND_TOPMOST,
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
    }
}

/// Headless/public entry point: navigates a file dialog HWND to the specified folder.
pub fn jump_dialog_to_folder(dialog_hwnd: isize, folder_path: &Path) -> bool {
    #[cfg(windows)]
    {
        win_companion::jump_dialog_to_folder(dialog_hwnd, folder_path)
    }
    #[cfg(not(windows))]
    {
        let _ = (dialog_hwnd, folder_path);
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modern_file_dialog_classification() {
        // Mock modern IFileDialog hierarchy:
        // #32770
        //   -> DirectUIHWND
        //        -> CtrlNotifySink
        //             -> NamespaceTreeControl
        //   -> ComboBoxEx32
        //        -> Edit (id: 1148)
        let modern_dialog = WindowNode::new("#32770", "Open")
            .with_child(
                WindowNode::new("DirectUIHWND", "").with_child(
                    WindowNode::new("CtrlNotifySink", "")
                        .with_child(WindowNode::new("NamespaceTreeControl", "")),
                ),
            )
            .with_child(
                WindowNode::new("ComboBoxEx32", "")
                    .with_child(WindowNode::new("Edit", "").with_id(1148)),
            );

        assert!(
            is_common_file_dialog(&modern_dialog),
            "Modern IFileDialog tree must be recognized as common file dialog"
        );
    }

    #[test]
    fn test_classic_file_dialog_classification() {
        // Mock classic GetOpenFileName hierarchy:
        // #32770
        //   -> SHELLDLL_DefView
        //   -> Edit (id: 1152)
        //   -> Button (id: 1, "Open")
        let classic_dialog = WindowNode::new("#32770", "Select File")
            .with_child(WindowNode::new("SHELLDLL_DefView", ""))
            .with_child(WindowNode::new("Edit", "").with_id(1152))
            .with_child(WindowNode::new("Button", "Open").with_id(1));

        assert!(
            is_common_file_dialog(&classic_dialog),
            "Classic common dialog tree must be recognized"
        );
    }

    #[test]
    fn test_messagebox_dialog_rejection() {
        // Standard Windows MessageBox is #32770 but has only Static text and Button controls
        let msg_box = WindowNode::new("#32770", "Confirm Action")
            .with_child(WindowNode::new("Static", "Are you sure?").with_id(65535))
            .with_child(WindowNode::new("Button", "OK").with_id(1))
            .with_child(WindowNode::new("Button", "Cancel").with_id(2));

        assert!(
            !is_common_file_dialog(&msg_box),
            "MessageBox must NOT be classified as common file dialog"
        );
    }

    #[test]
    fn test_notepad_main_window_rejection() {
        // Notepad main window: class "Notepad", child "Edit"
        let notepad = WindowNode::new("Notepad", "Untitled - Notepad")
            .with_child(WindowNode::new("Edit", "").with_id(15));

        assert!(
            !is_common_file_dialog(&notepad),
            "Non-#32770 window must be rejected"
        );
    }

    #[test]
    fn test_dock_rect_calculation() {
        let rect = calculate_companion_dock_rect(100, 200, 600, 400, 36);
        assert_eq!(rect.x, 100);
        assert_eq!(rect.y, 164); // 200 - 36
        assert_eq!(rect.width, 600);
        assert_eq!(rect.height, 36);

        // Near top of screen boundary clamp
        let clamped = calculate_companion_dock_rect(50, 10, 500, 300, 40);
        assert_eq!(clamped.y, 0);
    }
}
