//! Audio notification system
//!
//! Plays alert sounds for completed operations and errors when `play_sound`
//! is enabled in configuration.
//!
//! All content is pure ASCII.

#[cfg(target_os = "windows")]
extern "system" {
    fn MessageBeep(uType: u32) -> i32;
}

pub fn play_success_sound() {
    let cfg = crate::core::MossambiConfig::load();
    if !cfg.play_sound {
        return;
    }
    #[cfg(target_os = "windows")]
    unsafe {
        // MB_OK (0x00000000) plays standard Windows default notification sound
        MessageBeep(0);
    }
    #[cfg(not(target_os = "windows"))]
    {
        print!("\x07");
    }
}

pub fn play_error_sound() {
    let cfg = crate::core::MossambiConfig::load();
    if !cfg.play_sound {
        return;
    }
    #[cfg(target_os = "windows")]
    unsafe {
        // MB_ICONHAND (0x00000010) plays Windows error alert sound
        MessageBeep(0x10);
    }
    #[cfg(not(target_os = "windows"))]
    {
        print!("\x07");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_functions_callable_safely() {
        // When play_sound is disabled or enabled, calling these should not crash
        play_success_sound();
        play_error_sound();
    }
}
