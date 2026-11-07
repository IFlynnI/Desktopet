//! Hears when the Overlay loses the mouse.
//!
//! While a button is held on the Pet, winit gives the Overlay the mouse ("capture"), so every move and the release
//! come to it, wherever the cursor goes. Windows says when that ends with `WM_CAPTURECHANGED`, sent to the window that
//! loses the mouse: on every release, when winit lets it go, and also when something takes it away mid-drag, such as
//! Alt-Tab, the Start menu or the lock screen. In that second case no release ever comes, so this message is the only
//! way to know the hold is over. winit receives it but tells the app nothing (winit 0.30, `event_loop.rs`,
//! `WM_CAPTURECHANGED`).
//!
//! So this puts a window procedure of its own in front of winit's, a long-standing Win32 technique called subclassing.
//! It notes `WM_CAPTURECHANGED`, and hands every message, that one included, on to winit unchanged. Nothing polls,
//! and nothing reads the mouse buttons: an earlier version read them every 8 ms, but Windows reports them all up
//! while an app running as administrator is in front, such as Task Manager, which ended every drag at once.

use std::mem;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, GWLP_WNDPROC, GetWindowLongPtrW, SetWindowLongPtrW, WM_CAPTURECHANGED, WNDPROC,
};

use super::hit_thread::cursor_position;
use super::pointer::Pointer;

/// The one Overlay's watch. A process has one Overlay, so one watch, and it lasts as long as the window.
static WATCH: OnceLock<Watch> = OnceLock::new();

struct Watch {
    /// winit's window procedure, which every message is handed on to.
    original: isize,
    pointer: Arc<Pointer>,
}

/// Starts telling `pointer` each time `overlay` loses the mouse. Call it on the Overlay's own thread, before the owner
/// can press on the Pet. It does nothing if a watch has already started in this process.
pub fn watch(overlay: HWND, pointer: Arc<Pointer>) {
    // SAFETY: takes plain values, and `overlay` is a window of this process.
    let original = unsafe { GetWindowLongPtrW(overlay, GWLP_WNDPROC) };
    // The watch is in place before the procedure that reads it, so no message can arrive to find it missing.
    if WATCH.set(Watch { original, pointer }).is_err() {
        return;
    }
    let procedure: unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT = overlay_proc;
    // SAFETY: `overlay` is a window of this process, which is all that changing its procedure needs, and
    // `procedure` has the signature Windows calls a window procedure with.
    unsafe { SetWindowLongPtrW(overlay, GWLP_WNDPROC, procedure as usize as isize) };
}

unsafe extern "system" fn overlay_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let Some(watch) = WATCH.get() else {
        // Never happens: the watch is set before this procedure is installed.
        return LRESULT(0);
    };
    // `lparam` is the window gaining the mouse, or none. If it's the Overlay itself, it kept it.
    if message == WM_CAPTURECHANGED && lparam.0 != hwnd.0 as isize {
        // Where the cursor is can't be read on the lock screen, and it doesn't matter there: the drag just ends.
        let position = cursor_position().unwrap_or_default();
        watch.pointer.mouse_lost(Instant::now(), position);
    }
    // SAFETY: `original` was read from this window as its procedure, and an address and a procedure are the same
    // size, so it turns back into what it was.
    let original = unsafe { mem::transmute::<isize, WNDPROC>(watch.original) };
    // SAFETY: hands on exactly what Windows gave this procedure, to the procedure the window had.
    unsafe { CallWindowProcW(original, hwnd, message, wparam, lparam) }
}

#[cfg(test)]
mod tests {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassW, SendMessageW,
        WINDOW_EX_STYLE, WM_APP, WNDCLASSW, WS_POPUP,
    };
    use windows::core::w;

    use super::*;

    /// What the stand-in for winit answers to `WM_APP`, so a test can tell it was reached.
    const ANSWER: isize = 42;

    unsafe extern "system" fn stand_in_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if message == WM_APP {
            return LRESULT(ANSWER);
        }
        // SAFETY: passes on exactly what Windows gave this callback.
        unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
    }

    /// A window of this thread that's never shown, standing in for the Overlay with winit's procedure.
    fn stand_in_overlay() -> HWND {
        // SAFETY: passing no name asks for this program's own handle.
        let module = unsafe { GetModuleHandleW(None) }.expect("this program has a module handle");
        let class = WNDCLASSW {
            lpfnWndProc: Some(stand_in_proc),
            hInstance: module.into(),
            lpszClassName: w!("DesktopPetCaptureTest"),
            ..Default::default()
        };
        // SAFETY: `class` is a live local, and Windows copies it during the call.
        unsafe { RegisterClassW(&class) };
        // SAFETY: the class was registered above, and every other argument is a plain value. It's never shown.
        unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("DesktopPetCaptureTest"),
                None,
                WS_POPUP,
                0,
                0,
                0,
                0,
                None,
                None,
                Some(module.into()),
                None,
            )
        }
        .expect("Windows can create a hidden window")
    }

    fn send(hwnd: HWND, message: u32, lparam: isize) -> LRESULT {
        // SAFETY: the window belongs to this thread, so this runs its procedure before returning.
        unsafe { SendMessageW(hwnd, message, Some(WPARAM(0)), Some(LPARAM(lparam))) }
    }

    /// One test, because a process has one watch.
    #[test]
    fn given_a_watched_overlay_when_it_loses_the_mouse_or_keeps_it_then_only_losing_it_is_heard_and_winit_gets_every_message()
     {
        // Given
        let overlay = stand_in_overlay();
        let pointer = Arc::new(Pointer::default());
        watch(overlay, pointer.clone());
        let someone_else = 0x1234_isize;

        // When: let go of with no new owner, taken by another window, kept by the Overlay itself
        send(overlay, WM_CAPTURECHANGED, 0);
        send(overlay, WM_CAPTURECHANGED, someone_else);
        send(overlay, WM_CAPTURECHANGED, overlay.0 as isize);
        let answer = send(overlay, WM_APP, 0);

        // Then
        assert_eq!(pointer.take_mouse_lost().len(), 2);
        assert_eq!(
            answer,
            LRESULT(ANSWER),
            "winit's procedure still gets every message"
        );

        // SAFETY: the window belongs to this thread, and nothing uses it afterwards.
        let _ = unsafe { DestroyWindow(overlay) };
    }
}
