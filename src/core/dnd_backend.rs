//! Windowing Backend Policy for Cross-Platform Drag & Drop
//!
//! Receive-capability matrix for external file drops *into* Mossambi:
//!
//! | Host backend            | File-drop receive | Why                                   |
//! |-------------------------|-------------------|---------------------------------------|
//! | Windows (winit/WM_DROP) | Yes               | winit implements OLE drop natively    |
//! | Linux X11               | Yes               | winit implements XDND receive         |
//! | Linux Wayland           | No                | winit has no `wl_data_device` receive |
//!
//! On Wayland sessions (GNOME 40+, KDE Plasma 5.24+, WSLg) winit auto-selects
//! the Wayland backend whenever `WAYLAND_DISPLAY` exists, which silently
//! kills external drag & drop. Almost every modern compositor, however,
//! bridges Wayland-initiated drags onto XWayland (X11) targets, where winit's
//! XDND receive works. So when we detect a Wayland session with a usable
//! XWayland socket we force the X11 backend through eframe's
//! `NativeOptions::event_loop_builder` hook
//! (`EventLoopBuilderExtX11::with_x11`).
//!
//! Overrides (highest precedence first):
//! 1. `MOSSAMBI_BACKEND=x11|wayland|auto`
//! 2. Auto policy (below)
//!
//! If XWayland cannot be found we leave winit's auto selection in place
//! (pure-Wayland window: no external drop receive, everything else works)
//! and log an actionable hint.

use eframe::NativeOptions;

/// Windowing backend we resolved for this process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowBackend {
    /// Let winit pick per its own precedence (Wayland first on Wayland sessions).
    Auto,
    /// Force the X11 backend (includes XWayland on Wayland sessions).
    ForceX11,
    /// Prefer native Wayland explicitly (no external file-drop receive).
    ForceWayland,
}

impl WindowBackend {
    pub fn describe(self) -> &'static str {
        match self {
            WindowBackend::Auto => "winit auto (Wayland preferred on Wayland sessions)",
            WindowBackend::ForceX11 => "X11 (forced; includes XWayland under Wayland)",
            WindowBackend::ForceWayland => "Wayland (forced)",
        }
    }
}

/// Result of applying policy: which backend `options` now carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppliedPolicy {
    pub backend: WindowBackend,
}

/// Parses `MOSSAMBI_BACKEND` if present.
fn env_override() -> Option<WindowBackend> {
    let raw = std::env::var("MOSSAMBI_BACKEND").ok()?;
    match raw.trim().to_ascii_lowercase().as_str() {
        "x11" | "x" => Some(WindowBackend::ForceX11),
        "wayland" | "wl" => Some(WindowBackend::ForceWayland),
        "auto" => Some(WindowBackend::Auto),
        _ => None,
    }
}

/// Best-effort Wayland session detection.
#[cfg(target_os = "linux")]
fn is_wayland_session() -> bool {
    match std::env::var("XDG_SESSION_TYPE").as_deref() {
        Ok("wayland") => true,
        // No XDG info (common under `ssh` or custom sessions): treat a live
        // Wayland socket as evidence of a Wayland session.
        _ => {
            std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty())
                || std::env::var_os("WAYLAND_SOCKET").is_some()
        }
    }
}

/// True when an XWayland (or plain Xorg) display socket is reachable.
///
/// XWayland always creates sockets under `/tmp/.X11-unix/X<n>` for
/// `DISPLAY=:[<screen>][.<display>]`.
#[cfg(target_os = "linux")]
fn xwayland_socket_available() -> bool {
    let display = match std::env::var("DISPLAY") {
        Ok(d) if !d.is_empty() => d,
        _ => return false,
    };
    let num: String = display
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if num.is_empty() {
        return false;
    }
    std::path::Path::new("/tmp/.X11-unix")
        .join(format!("X{num}"))
        .exists()
}

#[cfg(not(target_os = "linux"))]
fn is_wayland_session() -> bool {
    false
}

#[cfg(not(target_os = "linux"))]
fn xwayland_socket_available() -> bool {
    false
}

fn resolve_backend() -> WindowBackend {
    if let Some(forced) = env_override() {
        return forced;
    }
    #[cfg(target_os = "linux")]
    if is_wayland_session() && xwayland_socket_available() {
        return WindowBackend::ForceX11;
    }
    WindowBackend::Auto
}

/// Resolve which backend should host GUI windows and inject the matching
/// forcing hook into `options` (called right before `eframe::run_native`).
///
/// Off-Linux builds keep winit's auto behavior (no Wayland does not exist
/// there; the arms below are compiled out with their winit paths).
pub fn apply_backend_policy(options: &mut NativeOptions) -> AppliedPolicy {
    let backend = resolve_backend();
    log::info!("Drag & drop backend policy: {}", backend.describe());

    match backend {
        WindowBackend::Auto => {
            options.event_loop_builder = None;
        }
        #[cfg(target_os = "linux")]
        WindowBackend::ForceX11 => {
            options.event_loop_builder = Some(Box::new(|builder| {
                winit::platform::x11::EventLoopBuilderExtX11::with_x11(builder);
            }));
        }
        #[cfg(target_os = "linux")]
        WindowBackend::ForceWayland => {
            options.event_loop_builder = Some(Box::new(|builder| {
                winit::platform::wayland::EventLoopBuilderExtWayland::with_wayland(builder);
            }));
        }
        #[cfg(not(target_os = "linux"))]
        WindowBackend::ForceX11 | WindowBackend::ForceWayland => {
            log::warn!("MOSSAMBI_BACKEND override is a Linux knob; keeping the default backend");
            options.event_loop_builder = None;
        }
    }

    if is_wayland_session() && !xwayland_socket_available() {
        log::warn!(
            "Wayland session without XWayland: external file drag & drop into Mossambi \
             will not receive files on native Wayland (winit limitation). Everything \
             inside the app still works; an XWayland-capable compositor is needed for \
             OS-level drops."
        );
    }

    AppliedPolicy { backend }
}
