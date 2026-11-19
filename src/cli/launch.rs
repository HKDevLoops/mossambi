//! GUI / widget / TUI launch handlers and the platform launch-intent pre-pass.
//! Pure ASCII by contract.

use anyhow::{anyhow, Result};
use std::path::PathBuf;

use crate::cli::{result_code, ExitCode};

pub fn handle_gui(initial_path: Option<PathBuf>) -> Result<()> {
    let _ipc_guard = crate::platform::single_instance::start_single_instance_server(None).ok();
    crate::platform::dialog_companion_start_if_enabled();
    crate::gui::run_studio_with_path(initial_path).map_err(|e| anyhow!("Studio GUI error: {}", e))
}

pub fn handle_tui(initial_path: Option<PathBuf>) -> Result<()> {
    crate::tui::run_with_path(initial_path).map_err(|e| anyhow!("TUI execution error: {}", e))
}

pub fn handle_widget(pos: Option<&str>, _active_drag: bool, file: Option<PathBuf>) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        use std::ptr::null_mut;
        use windows_sys::Win32::Foundation::GetLastError;
        use windows_sys::Win32::System::Threading::CreateMutexW;
        use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW;

        let mutex_name: Vec<u16> = "Local\\MossambiWidgetSingleInstance\0"
            .encode_utf16()
            .collect();
        unsafe {
            let handle = CreateMutexW(null_mut(), 1, mutex_name.as_ptr());
            const ERROR_ALREADY_EXISTS: u32 = 183;
            if !handle.is_null() && GetLastError() == ERROR_ALREADY_EXISTS {
                let title: Vec<u16> = "Mossambi Radial Widget\0".encode_utf16().collect();
                let hwnd = FindWindowW(null_mut(), title.as_ptr());
                if !hwnd.is_null() {
                    use windows_sys::Win32::UI::WindowsAndMessaging::{
                        PostMessageW, ShowWindow, SW_RESTORE,
                    };
                    const WM_CLOSE: u32 = 0x0010;
                    ShowWindow(hwnd, SW_RESTORE);
                    let _ = PostMessageW(hwnd, WM_CLOSE, 0, 0);
                }
            }
        }
    }

    let mut initial_pos: Option<eframe::egui::Pos2> = None;
    if let Some(pos_str) = pos {
        let parts: Vec<&str> = pos_str.split(',').collect();
        if parts.len() == 2 {
            if let (Ok(x), Ok(y)) = (
                parts[0].trim().parse::<f32>(),
                parts[1].trim().parse::<f32>(),
            ) {
                initial_pos = Some(eframe::egui::Pos2::new(x - 200.0, y - 200.0));
            }
        }
    }
    crate::widget::run_widget_with_args(initial_pos, file)
        .map_err(|e| anyhow!("Radial Widget error: {}", e))
}

/// Handle OS/shell launch intents that clap cannot model (Explorer shell
/// forms, `mossambi://` deep links, `%1` substitution, no-arg activation).
/// Returns `Some(exit_code)` when the invocation was consumed, or `None` to
/// continue to clap parsing.
pub fn handle_platform_launch(args: &[String]) -> Option<i32> {
    use crate::platform::args::{parse_launch_intent, LaunchIntent};
    use crate::platform::single_instance::try_forward_to_existing_instance;

    let intent = parse_launch_intent(args);
    if intent != LaunchIntent::None {
        if try_forward_to_existing_instance(&intent) {
            log::info!("Forwarded launch intent to existing running instance; exiting.");
            return Some(ExitCode::Ok.code());
        }

        match intent {
            LaunchIntent::Reveal { ref target, .. } => {
                let initial = target
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| target.clone());
                return Some(result_code(handle_gui(Some(initial))));
            }
            LaunchIntent::DeepLink { ref target, .. } => {
                return Some(result_code(handle_gui(Some(target.clone()))));
            }
            LaunchIntent::Navigate(ref target) => {
                if target.to_string_lossy() == "%1" {
                    return Some(result_code(handle_gui(None)));
                }
                if target.is_dir() {
                    return Some(result_code(handle_gui(Some(target.clone()))));
                }
            }
            LaunchIntent::None => {}
        }
    } else if args.len() == 1 && try_forward_to_existing_instance(&LaunchIntent::None) {
        log::info!("Activated existing running instance; exiting.");
        return Some(ExitCode::Ok.code());
    }

    None
}
