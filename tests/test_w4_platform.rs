//! Headless Acceptance Tests for W4 Platform Features
//!
//! Pure 7-bit ASCII.
//! Acceptance criteria verified:
//! 1. Registry enable/disable round-trip restores previous values (tested on temporary HKCU subkey)
//! 2. Argument parser handles "%1", "/select,path", "/select, <path>", and mossambi:// URIs
//! 3. Dialog-detection classifier unit-tested on mocked window class trees
//! 4. Single-instance IPC round-trip communication

use mossambi::platform::args::{parse_launch_intent, parse_single_arg, LaunchIntent};
use mossambi::platform::default_handler::{
    is_default_folder_handler_active, set_default_folder_handler_with_root,
};
use mossambi::platform::dialog_companion::{
    calculate_companion_dock_rect, is_common_file_dialog, WindowNode,
};
use std::path::PathBuf;

// ----------------------------------------------------------------------------
// 1. Registry Round-Trip Test on Temporary HKCU Subkey
// ----------------------------------------------------------------------------

#[test]
fn test_registry_default_folder_handler_roundtrip_temp_root() {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Registry::{
            RegCloseKey, RegCreateKeyExW, RegDeleteKeyW, RegOpenKeyExW, RegQueryValueExW,
            RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_ALL_ACCESS, KEY_READ,
            REG_OPTION_NON_VOLATILE, REG_SZ,
        };

        const ERROR_SUCCESS: u32 = 0;

        fn to_wide(s: &str) -> Vec<u16> {
            use std::os::windows::ffi::OsStrExt;
            std::ffi::OsStr::new(s)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect()
        }

        fn set_temp_reg_string(subkey: &str, val: &str) {
            let wide_key = to_wide(subkey);
            let mut hkey: HKEY = std::ptr::null_mut();
            let mut disp = 0u32;
            unsafe {
                let status = RegCreateKeyExW(
                    HKEY_CURRENT_USER,
                    wide_key.as_ptr(),
                    0,
                    std::ptr::null(),
                    REG_OPTION_NON_VOLATILE,
                    KEY_ALL_ACCESS,
                    std::ptr::null(),
                    &mut hkey,
                    &mut disp,
                );
                assert_eq!(status, ERROR_SUCCESS, "RegCreateKeyExW failed for temp key");
                let wide_val = to_wide(val);
                let byte_len = (wide_val.len() * 2) as u32;
                let set_status = RegSetValueExW(
                    hkey,
                    std::ptr::null(),
                    0,
                    REG_SZ,
                    wide_val.as_ptr() as *const u8,
                    byte_len,
                );
                assert_eq!(set_status, ERROR_SUCCESS);
                RegCloseKey(hkey);
            }
        }

        fn read_temp_reg_string(subkey: &str) -> Option<String> {
            let wide_key = to_wide(subkey);
            let mut hkey: HKEY = std::ptr::null_mut();
            unsafe {
                if RegOpenKeyExW(HKEY_CURRENT_USER, wide_key.as_ptr(), 0, KEY_READ, &mut hkey)
                    != ERROR_SUCCESS
                {
                    return None;
                }
                let mut reg_type = 0u32;
                let mut size = 0u32;
                if RegQueryValueExW(
                    hkey,
                    std::ptr::null(),
                    std::ptr::null(),
                    &mut reg_type,
                    std::ptr::null_mut(),
                    &mut size,
                ) != ERROR_SUCCESS
                    || size == 0
                {
                    RegCloseKey(hkey);
                    return None;
                }
                let u16_count = (size as usize).div_ceil(2);
                let mut buf = vec![0u16; u16_count];
                let status = RegQueryValueExW(
                    hkey,
                    std::ptr::null(),
                    std::ptr::null(),
                    &mut reg_type,
                    buf.as_mut_ptr() as *mut u8,
                    &mut size,
                );
                RegCloseKey(hkey);
                if status == ERROR_SUCCESS {
                    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
                    Some(String::from_utf16_lossy(&buf[..len]))
                } else {
                    None
                }
            }
        }

        // Isolated test roots in HKCU
        let test_root = "Software\\MossambiTest\\Classes";
        let test_backup = "Software\\MossambiTest\\Backup";
        let mock_cmd = "\"C:\\Test\\mossambi.exe\" \"%1\"";

        // Step 1: Pre-populate initial state representing existing Explorer values
        set_temp_reg_string(&format!("{test_root}\\Directory\\shell"), "open_explorer");
        set_temp_reg_string(&format!("{test_root}\\Drive\\shell"), "drive_open");
        // Folder\\shell default is intentionally left unset/non-existent
        set_temp_reg_string(
            &format!("{test_root}\\Folder\\shell\\opennewwindow\\command"),
            "explorer.exe /separate",
        );

        assert!(
            !is_default_folder_handler_active(test_root),
            "Handler must not be active initially"
        );

        // Step 2: Enable Mossambi as default folder handler
        let enable_res =
            set_default_folder_handler_with_root(test_root, test_backup, true, mock_cmd);
        assert!(enable_res.is_ok(), "Enable must succeed: {:?}", enable_res);

        // Verify that default verb became "mossambi" and command keys exist
        assert!(
            is_default_folder_handler_active(test_root),
            "Handler must report active after enable"
        );
        assert_eq!(
            read_temp_reg_string(&format!("{test_root}\\Directory\\shell")).as_deref(),
            Some("mossambi")
        );
        assert_eq!(
            read_temp_reg_string(&format!("{test_root}\\Directory\\shell\\mossambi\\command"))
                .as_deref(),
            Some(mock_cmd)
        );
        assert_eq!(
            read_temp_reg_string(&format!("{test_root}\\Drive\\shell")).as_deref(),
            Some("mossambi")
        );
        assert_eq!(
            read_temp_reg_string(&format!("{test_root}\\Folder\\shell")).as_deref(),
            Some("mossambi")
        );
        assert_eq!(
            read_temp_reg_string(&format!(
                "{test_root}\\Folder\\shell\\opennewwindow\\command"
            ))
            .as_deref(),
            Some(mock_cmd)
        );

        // Step 3: Disable Mossambi as default folder handler
        let disable_res =
            set_default_folder_handler_with_root(test_root, test_backup, false, mock_cmd);
        assert!(
            disable_res.is_ok(),
            "Disable must succeed: {:?}",
            disable_res
        );

        // Verify that previous default values were restored exactly
        assert!(
            !is_default_folder_handler_active(test_root),
            "Handler must not be active after disable"
        );
        assert_eq!(
            read_temp_reg_string(&format!("{test_root}\\Directory\\shell")).as_deref(),
            Some("open_explorer"),
            "Directory\\shell must be restored to previous value"
        );
        assert_eq!(
            read_temp_reg_string(&format!("{test_root}\\Drive\\shell")).as_deref(),
            Some("drive_open"),
            "Drive\\shell must be restored to previous value"
        );
        assert_eq!(
            read_temp_reg_string(&format!("{test_root}\\Folder\\shell")).as_deref(),
            None,
            "Folder\\shell must be restored to empty/non-existent"
        );
        assert_eq!(
            read_temp_reg_string(&format!(
                "{test_root}\\Folder\\shell\\opennewwindow\\command"
            ))
            .as_deref(),
            Some("explorer.exe /separate"),
            "opennewwindow command must be restored to previous value"
        );

        // Verify mossambi subkeys were completely removed
        assert!(
            read_temp_reg_string(&format!("{test_root}\\Directory\\shell\\mossambi")).is_none()
        );
        assert!(
            read_temp_reg_string(&format!("{test_root}\\Directory\\shell\\mossambi\\command"))
                .is_none()
        );
        assert!(read_temp_reg_string(&format!("{test_root}\\Drive\\shell\\mossambi")).is_none());
        assert!(read_temp_reg_string(&format!("{test_root}\\Folder\\shell\\mossambi")).is_none());

        // Clean up test HKCU subkey tree
        unsafe {
            for sub in &[
                format!("{test_root}\\Folder\\shell\\opennewwindow\\command"),
                format!("{test_root}\\Folder\\shell\\opennewwindow"),
                format!("{test_root}\\Folder\\shell"),
                format!("{test_root}\\Folder"),
                format!("{test_root}\\Drive\\shell"),
                format!("{test_root}\\Drive"),
                format!("{test_root}\\Directory\\shell"),
                format!("{test_root}\\Directory"),
                test_root.to_string(),
                "Software\\MossambiTest".to_string(),
            ] {
                let w = to_wide(sub);
                RegDeleteKeyW(HKEY_CURRENT_USER, w.as_ptr());
            }
        }
    }
}

// ----------------------------------------------------------------------------
// 2. Argument Parser Acceptance Tests
// ----------------------------------------------------------------------------

#[test]
fn test_arg_parser_percent_one() {
    let intent = parse_single_arg("%1");
    assert_eq!(intent, LaunchIntent::Navigate(PathBuf::from("%1")));

    let quoted_intent = parse_single_arg("\"%1\"");
    assert_eq!(quoted_intent, LaunchIntent::Navigate(PathBuf::from("%1")));

    let cli_args = ["mossambi.exe", "%1"];
    assert_eq!(
        parse_launch_intent(&cli_args),
        LaunchIntent::Navigate(PathBuf::from("%1"))
    );
}

#[test]
fn test_arg_parser_select_syntax() {
    // 1. Single argument: "/select,<path>"
    let arg1 = "/select,C:\\Windows\\System32\\calc.exe";
    assert_eq!(
        parse_single_arg(arg1),
        LaunchIntent::Reveal {
            target: PathBuf::from("C:\\Windows\\System32\\calc.exe"),
            select: true,
        }
    );

    // 2. Single argument with space after comma: "/select, <path>"
    let arg2 = "/select, C:\\Users\\User\\Downloads\\archive.zip";
    assert_eq!(
        parse_single_arg(arg2),
        LaunchIntent::Reveal {
            target: PathBuf::from("C:\\Users\\User\\Downloads\\archive.zip"),
            select: true,
        }
    );

    // 3. Two separate arguments: "/select" "<path>"
    let args3 = ["mossambi", "/select", "D:\\Documents\\Contract.pdf"];
    assert_eq!(
        parse_launch_intent(&args3),
        LaunchIntent::Reveal {
            target: PathBuf::from("D:\\Documents\\Contract.pdf"),
            select: true,
        }
    );

    // 4. Two separate arguments with comma: "/select," "<path>"
    let args4 = ["mossambi", "/select,", "E:\\Music\\song.flac"];
    assert_eq!(
        parse_launch_intent(&args4),
        LaunchIntent::Reveal {
            target: PathBuf::from("E:\\Music\\song.flac"),
            select: true,
        }
    );

    // 5. Quoted target inside single argument: "/select,\"C:\\My Files\\test.txt\""
    let arg5 = "/select,\"C:\\My Files\\test.txt\"";
    assert_eq!(
        parse_single_arg(arg5),
        LaunchIntent::Reveal {
            target: PathBuf::from("C:\\My Files\\test.txt"),
            select: true,
        }
    );
}

#[test]
fn test_arg_parser_mossambi_uris() {
    // 1. Open folder deep link
    let uri_open = "mossambi://open?path=C%3A%5CUsers%5CAlice%5CDocuments";
    assert_eq!(
        parse_single_arg(uri_open),
        LaunchIntent::Navigate(PathBuf::from("C:\\Users\\Alice\\Documents"))
    );

    // 2. Reveal file deep link
    let uri_reveal = "mossambi://reveal?path=C%3A%5CUsers%5CAlice%5CDesktop%5Cphoto.jpg";
    assert_eq!(
        parse_single_arg(uri_reveal),
        LaunchIntent::Reveal {
            target: PathBuf::from("C:\\Users\\Alice\\Desktop\\photo.jpg"),
            select: true,
        }
    );

    // 3. Custom deep link action
    let uri_custom = "mossambi://view?path=D%3A%5CData%5Cschema.sql";
    assert_eq!(
        parse_single_arg(uri_custom),
        LaunchIntent::DeepLink {
            action: "view".to_string(),
            target: PathBuf::from("D:\\Data\\schema.sql"),
        }
    );
}

// ----------------------------------------------------------------------------
// 3. Dialog Detection Classifier on Mocked Window Class Trees
// ----------------------------------------------------------------------------

#[test]
fn test_classifier_modern_ifiledialog_detection() {
    // Construct simulated modern IFileOpenDialog window tree
    let dialog_tree = WindowNode::new("#32770", "Open")
        .with_child(
            WindowNode::new("DirectUIHWND", "")
                .with_child(
                    WindowNode::new("CtrlNotifySink", "")
                        .with_child(WindowNode::new("NamespaceTreeControl", "")),
                )
                .with_child(
                    WindowNode::new("CtrlNotifySink", "")
                        .with_child(WindowNode::new("SHELLDLL_DefView", "ShellView")),
                ),
        )
        .with_child(
            WindowNode::new("ComboBoxEx32", "")
                .with_child(WindowNode::new("Edit", "").with_id(1148)),
        )
        .with_child(WindowNode::new("Button", "Open").with_id(1))
        .with_child(WindowNode::new("Button", "Cancel").with_id(2));

    assert!(
        is_common_file_dialog(&dialog_tree),
        "Modern IFileDialog tree must be positively classified"
    );
}

#[test]
fn test_classifier_classic_file_dialog_detection() {
    // Construct simulated classic GetOpenFileName window tree
    let classic_tree = WindowNode::new("#32770", "Save As")
        .with_child(WindowNode::new("SHELLDLL_DefView", ""))
        .with_child(WindowNode::new("Edit", "default.txt").with_id(1152))
        .with_child(WindowNode::new("Button", "Save").with_id(1));

    assert!(
        is_common_file_dialog(&classic_tree),
        "Classic common file dialog tree must be positively classified"
    );
}

#[test]
fn test_classifier_messagebox_rejection() {
    // Standard Windows MessageBox is #32770 but contains only Static text and Buttons
    let msg_box = WindowNode::new("#32770", "Warning")
        .with_child(WindowNode::new("Static", "File already exists. Overwrite?").with_id(65535))
        .with_child(WindowNode::new("Button", "Yes").with_id(6))
        .with_child(WindowNode::new("Button", "No").with_id(7));

    assert!(
        !is_common_file_dialog(&msg_box),
        "Standard MessageBox must NOT be classified as a file dialog"
    );
}

#[test]
fn test_classifier_unrelated_window_rejection() {
    // Notepad main window
    let notepad = WindowNode::new("Notepad", "notes.txt - Notepad")
        .with_child(WindowNode::new("Edit", "").with_id(15));
    assert!(
        !is_common_file_dialog(&notepad),
        "Notepad window must not be classified as a file dialog"
    );

    // Explorer main shell window
    let explorer = WindowNode::new("CabinetWClass", "Documents")
        .with_child(WindowNode::new("ShellTabWindowClass", ""));
    assert!(
        !is_common_file_dialog(&explorer),
        "Explorer window must not be classified as a common file dialog"
    );
}

#[test]
fn test_companion_dock_rect_calculations() {
    // Standard dialog 800x600 located at (200, 150)
    let rect = calculate_companion_dock_rect(200, 150, 800, 600, 36);
    assert_eq!(rect.x, 200);
    assert_eq!(rect.y, 114); // 150 - 36
    assert_eq!(rect.width, 800);
    assert_eq!(rect.height, 36);

    // Clamping to top of screen
    let clamped = calculate_companion_dock_rect(50, 20, 700, 500, 30);
    assert_eq!(clamped.y, 0);
}

// ----------------------------------------------------------------------------
// 4. Single-Instance IPC Communication Test
// ----------------------------------------------------------------------------

#[test]
#[cfg(windows)]
fn test_single_instance_ipc_delivery() {
    let test_pipe = r"\\.\pipe\mossambi_acceptance_test_pipe_w4";
    let server_guard =
        mossambi::platform::single_instance::start_single_instance_server(Some(test_pipe));
    assert!(server_guard.is_ok(), "Named pipe server must start cleanly");

    std::thread::sleep(std::time::Duration::from_millis(80));

    let target_path = PathBuf::from("C:\\Users\\User\\Documents\\TestFolder");
    let intent = LaunchIntent::Navigate(target_path.clone());

    let forwarded =
        mossambi::platform::single_instance::try_forward_to_pipe_custom(test_pipe, &intent);
    assert!(forwarded, "Client must forward intent to server");

    std::thread::sleep(std::time::Duration::from_millis(80));

    let pending = mossambi::platform::single_instance::poll_pending_ipc_intents();
    assert!(
        pending.iter().any(|it| it == &intent),
        "Server must have received forwarded intent in pending queue"
    );
}
