//! The Overlay's Win32 styles.

use windows::Win32::Foundation::{HANDLE, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetWindowLongPtrW, SetPropW, SetWindowLongPtrW, WS_EX_APPWINDOW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
};
use windows::core::{PCWSTR, w};

/// The window property the shell reads to learn that a window is never fullscreen.
pub const NOT_FULLSCREEN_PROPERTY: PCWSTR = w!("NonRudeHWND");

/// Layered, a tool window (no taskbar or Alt-Tab) that never takes focus, and transparent to clicks unless it
/// should catch them: when the cursor is on the Pet, or while the owner holds it.
///
/// Topmost isn't here: Windows only changes it through `SetWindowPos`, which winit calls for us.
fn wanted_ex_style(current: u32, catch_clicks: bool) -> u32 {
    let wanted =
        (current | WS_EX_LAYERED.0 | WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0) & !WS_EX_APPWINDOW.0;
    if catch_clicks {
        wanted & !WS_EX_TRANSPARENT.0
    } else {
        wanted | WS_EX_TRANSPARENT.0
    }
}

/// Brings the Overlay's styles in line. winit overwrites them at startup, so this runs on every poll.
pub fn apply(hwnd: HWND, catch_clicks: bool) {
    // SAFETY: takes plain values. A window that's gone returns 0 and the write below fails harmlessly.
    let current = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
    let wanted = wanted_ex_style(current, catch_clicks);
    if wanted != current {
        // SAFETY: takes plain values, and these are style bits rather than a pointer.
        unsafe { SetWindowLongPtrW(hwnd, GWL_EXSTYLE, wanted as isize) };
    }
}

/// Tells the shell the Overlay isn't a fullscreen app, even when it covers the whole monitor. Without this
/// an auto-hide taskbar could lose its always-on-top status. It must be set before the first show.
pub fn declare_not_fullscreen(hwnd: HWND) {
    // Any non-null value counts, and Windows never dereferences it.
    let set = HANDLE(std::ptr::dangling_mut());
    // SAFETY: the name is a static string and the value is only a flag.
    let _ = unsafe { SetPropW(hwnd, NOT_FULLSCREEN_PROPERTY, Some(set)) };
}

#[cfg(test)]
mod tests {
    use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOPMOST;

    use super::*;

    #[test]
    fn given_any_current_style_when_computing_the_wanted_style_then_the_overlay_stays_a_layered_tool_window_that_never_activates()
     {
        // Given
        let current = WS_EX_APPWINDOW.0;

        // When
        let wanted = wanted_ex_style(current, false);

        // Then
        assert_eq!(wanted & WS_EX_APPWINDOW.0, 0);
        assert_ne!(wanted & WS_EX_TOOLWINDOW.0, 0);
        assert_ne!(wanted & WS_EX_NOACTIVATE.0, 0);
        assert_ne!(wanted & WS_EX_LAYERED.0, 0);
    }

    #[test]
    fn given_the_cursor_is_off_the_pet_when_computing_the_wanted_style_then_clicks_pass_through() {
        // Given
        let current = 0;

        // When
        let wanted = wanted_ex_style(current, false);

        // Then
        assert_ne!(wanted & WS_EX_TRANSPARENT.0, 0);
    }

    #[test]
    fn given_the_cursor_is_on_the_pet_when_computing_the_wanted_style_then_the_overlay_catches_clicks()
     {
        // Given
        let current = WS_EX_TRANSPARENT.0;

        // When
        let wanted = wanted_ex_style(current, true);

        // Then
        assert_eq!(wanted & WS_EX_TRANSPARENT.0, 0);
    }

    #[test]
    fn given_topmost_status_when_computing_the_wanted_style_then_it_is_left_alone() {
        // Given: Windows only changes topmost through SetWindowPos, so writing the bit would do nothing
        let (topmost, normal) = (WS_EX_TOPMOST.0, 0);

        // When
        let with_topmost = wanted_ex_style(topmost, false);
        let without_topmost = wanted_ex_style(normal, false);

        // Then
        assert_ne!(with_topmost & WS_EX_TOPMOST.0, 0);
        assert_eq!(without_topmost & WS_EX_TOPMOST.0, 0);
    }
}
