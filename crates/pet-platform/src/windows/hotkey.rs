//! Ctrl+Alt+P, registered against the message window.
//!
//! Registering against a window, not the thread, means a press isn't lost while the tray menu's own message loop is
//! running. `MOD_NOREPEAT` stops a held key from toggling over and over.

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey, VK_P,
};

/// Only has to be unique within the message window.
const ID: i32 = 1;

/// Holds the hotkey. Dropping it frees the key for other apps.
pub struct Hotkey(HWND);

impl Hotkey {
    /// `None` when another app owns Ctrl+Alt+P.
    pub fn register(window: HWND) -> Option<Self> {
        let modifiers = MOD_CONTROL | MOD_ALT | MOD_NOREPEAT;
        // SAFETY: takes plain values, and `window` belongs to this thread.
        let registered = unsafe { RegisterHotKey(Some(window), ID, modifiers, VK_P.0.into()) };
        registered.ok().map(|()| Self(window))
    }
}

impl Drop for Hotkey {
    fn drop(&mut self) {
        // SAFETY: this value owns the registration, and `drop` runs exactly once.
        let _ = unsafe { UnregisterHotKey(Some(self.0), ID) };
    }
}
