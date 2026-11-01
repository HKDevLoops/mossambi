//! Active File Detection under Cursor for Windows & Linux
//! Resolves the path of the file currently being dragged or selected in
//! Windows Explorer (CabinetWClass), Windows Desktop (Progman / WorkerW), or Linux file managers.

use std::path::PathBuf;

/// Attempts to resolve the active file under the cursor or foreground Explorer window.
pub fn get_active_file_under_cursor(cursor_x: i32, cursor_y: i32) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Foundation::POINT;
        let pt = POINT {
            x: cursor_x,
            y: cursor_y,
        };
        let files = mossambi_core::query_active_explorer_files(pt);
        files.first().cloned()
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (cursor_x, cursor_y);
        None
    }
}
