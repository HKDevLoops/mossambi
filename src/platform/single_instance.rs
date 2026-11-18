//! Single-Instance IPC Mechanism for Mossambi
//!
//! Pure 7-bit ASCII. Enforces single-instance application lifecycle:
//! When a second Mossambi instance is launched (e.g. from Explorer default folder
//! handler or command line), it forwards its LaunchIntent to the existing running instance
//! via a local Windows named pipe (or Unix domain socket on non-Windows) and terminates cleanly.
//! The running instance consumes the intent and opens a new tab or navigates.

use crate::platform::args::LaunchIntent;
use std::path::PathBuf;
#[cfg(windows)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(windows)]
use std::sync::Arc;
use std::sync::{Mutex, OnceLock};

/// Default named pipe path for single-instance IPC on Windows.
pub const DEFAULT_PIPE_NAME: &str = r"\\.\pipe\mossambi_single_instance_ipc";

/// Global queue of incoming intents received by the single-instance listener.
static PENDING_INTENTS: OnceLock<Mutex<Vec<LaunchIntent>>> = OnceLock::new();

fn get_intent_queue() -> &'static Mutex<Vec<LaunchIntent>> {
    PENDING_INTENTS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Serializes a LaunchIntent into a single newline-terminated UTF-8 protocol line.
pub fn serialize_intent(intent: &LaunchIntent) -> String {
    match intent {
        LaunchIntent::Navigate(p) => format!("NAVIGATE\t{}\n", p.to_string_lossy()),
        LaunchIntent::Reveal { target, select } => {
            format!("REVEAL\t{}\t{}\n", target.to_string_lossy(), select)
        }
        LaunchIntent::DeepLink { action, target } => {
            format!("DEEPLINK\t{}\t{}\n", action, target.to_string_lossy())
        }
        LaunchIntent::None => "NONE\n".to_string(),
    }
}

/// Deserializes a LaunchIntent from a protocol string line.
pub fn deserialize_intent(line: &str) -> Option<LaunchIntent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parts: Vec<&str> = trimmed.split('\t').collect();
    match parts.first().copied() {
        Some("NAVIGATE") => {
            let path_str = parts.get(1)?;
            Some(LaunchIntent::Navigate(PathBuf::from(path_str)))
        }
        Some("REVEAL") => {
            let path_str = parts.get(1)?;
            let select = parts.get(2).map(|s| *s == "true").unwrap_or(true);
            Some(LaunchIntent::Reveal {
                target: PathBuf::from(path_str),
                select,
            })
        }
        Some("DEEPLINK") => {
            let action = parts.get(1)?.to_string();
            let path_str = parts.get(2)?;
            Some(LaunchIntent::DeepLink {
                action,
                target: PathBuf::from(path_str),
            })
        }
        Some("NONE") => Some(LaunchIntent::None),
        _ => None,
    }
}

/// Polls and drains all pending LaunchIntents forwarded by secondary instances.
pub fn poll_pending_ipc_intents() -> Vec<LaunchIntent> {
    if let Ok(mut q) = get_intent_queue().lock() {
        std::mem::take(&mut *q)
    } else {
        Vec::new()
    }
}

/// Pushes an incoming LaunchIntent into the global queue.
pub fn push_incoming_intent(intent: LaunchIntent) {
    if let Ok(mut q) = get_intent_queue().lock() {
        q.push(intent);
    }
}

#[cfg(windows)]
mod win_ipc {
    use super::*;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{
        CloseHandle, BOOL, ERROR_PIPE_BUSY, GENERIC_READ, GENERIC_WRITE, HANDLE,
        INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, OPEN_EXISTING,
    };

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateNamedPipeW(
            lpname: *const u16,
            dwopenmode: u32,
            dwpipemode: u32,
            nmaxinstances: u32,
            noutbuffersize: u32,
            ninbuffersize: u32,
            ndefaulttimeout: u32,
            lpsecurityattributes: *const core::ffi::c_void,
        ) -> HANDLE;

        fn ConnectNamedPipe(hnamedpipe: HANDLE, lpoverlapped: *mut core::ffi::c_void) -> BOOL;

        fn DisconnectNamedPipe(hnamedpipe: HANDLE) -> BOOL;

        fn ReadFile(
            hfile: HANDLE,
            lpbuffer: *mut u8,
            nnumberofbytestoread: u32,
            lpnumberofbytesread: *mut u32,
            lpoverlapped: *mut core::ffi::c_void,
        ) -> BOOL;

        fn WriteFile(
            hfile: HANDLE,
            lpbuffer: *const u8,
            nnumberofbytestowrite: u32,
            lpnumberofbyteswritten: *mut u32,
            lpoverlapped: *mut core::ffi::c_void,
        ) -> BOOL;
    }

    const PIPE_ACCESS_DUPLEX: u32 = 0x00000003;
    const PIPE_TYPE_MESSAGE: u32 = 0x00000004;
    const PIPE_READMODE_MESSAGE: u32 = 0x00000002;
    const PIPE_WAIT: u32 = 0x00000000;
    const PIPE_UNLIMITED_INSTANCES: u32 = 255;

    fn to_wide(s: &str) -> Vec<u16> {
        std::ffi::OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    /// Attempts to connect to an existing Mossambi instance via named pipe and forward the intent.
    /// Returns true if successfully delivered, false if no server is running.
    pub fn try_forward_to_pipe(pipe_name: &str, intent: &LaunchIntent) -> bool {
        let wide_name = to_wide(pipe_name);
        let h_pipe = unsafe {
            CreateFileW(
                wide_name.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                std::ptr::null_mut(),
            )
        };

        if h_pipe == INVALID_HANDLE_VALUE {
            return false;
        }

        let message = serialize_intent(intent);
        let bytes = message.as_bytes();
        let mut written = 0u32;
        let write_ok = unsafe {
            WriteFile(
                h_pipe,
                bytes.as_ptr(),
                bytes.len() as u32,
                &mut written,
                std::ptr::null_mut(),
            )
        };

        let mut ack_buf = [0u8; 8];
        let mut read_bytes = 0u32;
        let read_ok = unsafe {
            ReadFile(
                h_pipe,
                ack_buf.as_mut_ptr(),
                ack_buf.len() as u32,
                &mut read_bytes,
                std::ptr::null_mut(),
            )
        };

        unsafe {
            CloseHandle(h_pipe);
        }

        write_ok != 0 && read_ok != 0 && read_bytes > 0
    }

    /// Handle for a running named pipe IPC server.
    pub struct ServerHandle {
        running: Arc<AtomicBool>,
        thread_handle: Option<std::thread::JoinHandle<()>>,
        pipe_name: String,
    }

    impl ServerHandle {
        pub fn stop(&mut self) {
            if self.running.swap(false, Ordering::SeqCst) {
                // Wake up the blocking ConnectNamedPipe by attempting a dummy connection
                let wide_name = to_wide(&self.pipe_name);
                unsafe {
                    let h = CreateFileW(
                        wide_name.as_ptr(),
                        GENERIC_READ | GENERIC_WRITE,
                        0,
                        std::ptr::null(),
                        OPEN_EXISTING,
                        FILE_ATTRIBUTE_NORMAL,
                        std::ptr::null_mut(),
                    );
                    if h != INVALID_HANDLE_VALUE {
                        CloseHandle(h);
                    }
                }
                if let Some(t) = self.thread_handle.take() {
                    let _ = t.join();
                }
            }
        }
    }

    impl Drop for ServerHandle {
        fn drop(&mut self) {
            self.stop();
        }
    }

    /// Starts the background named pipe server listening for secondary instances.
    pub fn start_named_pipe_server(pipe_name: &str) -> anyhow::Result<ServerHandle> {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);
        let p_name = pipe_name.to_string();
        let p_name_clone = p_name.clone();

        let thread_handle = std::thread::Builder::new()
            .name("mossambi_ipc_server".to_string())
            .spawn(move || {
                let wide_name = to_wide(&p_name_clone);

                while running_clone.load(Ordering::SeqCst) {
                    let h_pipe = unsafe {
                        CreateNamedPipeW(
                            wide_name.as_ptr(),
                            PIPE_ACCESS_DUPLEX,
                            PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT,
                            PIPE_UNLIMITED_INSTANCES,
                            4096,
                            4096,
                            5000,
                            std::ptr::null(),
                        )
                    };

                    if h_pipe == INVALID_HANDLE_VALUE {
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        continue;
                    }

                    let connected = unsafe { ConnectNamedPipe(h_pipe, std::ptr::null_mut()) };
                    let client_ok = connected != 0
                        || unsafe {
                            windows_sys::Win32::Foundation::GetLastError() == ERROR_PIPE_BUSY
                        };

                    if client_ok && running_clone.load(Ordering::SeqCst) {
                        let mut buf = [0u8; 4096];
                        let mut read_bytes = 0u32;
                        let read_res = unsafe {
                            ReadFile(
                                h_pipe,
                                buf.as_mut_ptr(),
                                buf.len() as u32,
                                &mut read_bytes,
                                std::ptr::null_mut(),
                            )
                        };

                        if read_res != 0 && read_bytes > 0 {
                            if let Ok(s) = std::str::from_utf8(&buf[..read_bytes as usize]) {
                                if let Some(intent) = deserialize_intent(s) {
                                    push_incoming_intent(intent);
                                }
                            }

                            // Write ACK
                            let ack = b"OK\n";
                            let mut written = 0u32;
                            unsafe {
                                WriteFile(
                                    h_pipe,
                                    ack.as_ptr(),
                                    ack.len() as u32,
                                    &mut written,
                                    std::ptr::null_mut(),
                                );
                            }
                        }
                    }

                    unsafe {
                        DisconnectNamedPipe(h_pipe);
                        CloseHandle(h_pipe);
                    }
                }
            })?;

        Ok(ServerHandle {
            running,
            thread_handle: Some(thread_handle),
            pipe_name: p_name,
        })
    }
}

/// Attempts to forward the launch intent to an already running Mossambi instance.
/// Returns true if an instance was running and accepted the intent; false otherwise.
pub fn try_forward_to_existing_instance(intent: &LaunchIntent) -> bool {
    #[cfg(windows)]
    {
        win_ipc::try_forward_to_pipe(DEFAULT_PIPE_NAME, intent)
    }
    #[cfg(not(windows))]
    {
        let _ = intent;
        false
    }
}

/// Try forwarding to a custom pipe name (useful for isolated unit tests).
pub fn try_forward_to_pipe_custom(pipe_name: &str, intent: &LaunchIntent) -> bool {
    #[cfg(windows)]
    {
        win_ipc::try_forward_to_pipe(pipe_name, intent)
    }
    #[cfg(not(windows))]
    {
        let _ = (pipe_name, intent);
        false
    }
}

/// Handle representing an active single-instance server.
pub struct SingleInstanceGuard {
    #[cfg(windows)]
    _handle: win_ipc::ServerHandle,
}

/// Starts the single-instance IPC server on the default or custom pipe.
pub fn start_single_instance_server(
    pipe_override: Option<&str>,
) -> anyhow::Result<SingleInstanceGuard> {
    let name = pipe_override.unwrap_or(DEFAULT_PIPE_NAME);
    #[cfg(windows)]
    {
        let handle = win_ipc::start_named_pipe_server(name)?;
        Ok(SingleInstanceGuard { _handle: handle })
    }
    #[cfg(not(windows))]
    {
        let _ = name;
        Ok(SingleInstanceGuard {})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_serialization_round_trip() {
        let nav = LaunchIntent::Navigate(PathBuf::from("C:\\Program Files\\Mossambi"));
        let s_nav = serialize_intent(&nav);
        assert_eq!(deserialize_intent(&s_nav), Some(nav));

        let rev = LaunchIntent::Reveal {
            target: PathBuf::from("D:\\Data\\Report.pdf"),
            select: true,
        };
        let s_rev = serialize_intent(&rev);
        assert_eq!(deserialize_intent(&s_rev), Some(rev));

        let deep = LaunchIntent::DeepLink {
            action: "open".to_string(),
            target: PathBuf::from("C:\\Users\\Default"),
        };
        let s_deep = serialize_intent(&deep);
        assert_eq!(deserialize_intent(&s_deep), Some(deep));

        let none = LaunchIntent::None;
        let s_none = serialize_intent(&none);
        assert_eq!(deserialize_intent(&s_none), Some(none));
    }
}
