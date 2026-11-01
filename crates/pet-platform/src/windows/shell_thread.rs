//! The one place the shell's messages are waited for. The thread sits in `GetMessageW`, using no CPU, and it keeps
//! running while the engine is stopped, which is how the engine gets woken again.

use std::cell::OnceCell;
use std::ffi::c_void;
use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, EndMenu, GetMessageW,
    HWND_MESSAGE, MSG, PostMessageW, PostQuitMessage, RegisterClassW, WINDOW_EX_STYLE,
    WINDOW_STYLE, WM_APP, WM_CLOSE, WM_HOTKEY, WNDCLASSW,
};
use windows::core::{PCWSTR, w};

use super::foreground::{self, Foreground};
use super::hotkey::Hotkey;
use super::tray;
use crate::{ShellEvent, TrayIconImage, Wake};

/// How a second launch finds the message window.
pub const CLASS: PCWSTR = w!("DesktopPetMessageWindow");

/// What a second launch sends it.
pub const PLEASE_SHOW: u32 = WM_APP + 1;

/// Sends an event, then wakes the engine so it reads it.
struct Notifier {
    events: Sender<ShellEvent>,
    wake: Wake,
}

impl Notifier {
    fn notify(&self, event: ShellEvent) {
        // Sending fails only once the engine has gone, and then nobody needs telling.
        let _ = self.events.send(event);
        (self.wake)();
    }
}

thread_local! {
    /// The window callback can't take arguments of ours, so it finds this here. It's set before the window exists.
    static NOTIFIER: OnceCell<Notifier> = const { OnceCell::new() };
}

fn notify(event: ShellEvent) {
    NOTIFIER.with(|notifier| {
        if let Some(notifier) = notifier.get() {
            notifier.notify(event);
        }
    });
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_HOTKEY => notify(ShellEvent::Toggle),
        PLEASE_SHOW => notify(ShellEvent::Show),
        WM_CLOSE => {
            // An open tray menu runs its own message loop, and the thread would never get out of it. This does nothing
            // when no menu is open, and the error that reports that doesn't matter.
            // SAFETY: takes no arguments.
            let _ = unsafe { EndMenu() };
            // SAFETY: takes a plain value.
            unsafe { PostQuitMessage(0) };
        }
        // SAFETY: passes on exactly what Windows gave this callback.
        _ => return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
    LRESULT(0)
}

/// A window that's never visible and only receives messages. It's the address other things send to.
struct MessageWindow(HWND);

impl MessageWindow {
    fn create() -> Self {
        // SAFETY: passing no name asks for this program's own handle.
        let module = unsafe { GetModuleHandleW(None) }.expect("this program has a module handle");
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: module.into(),
            lpszClassName: CLASS,
            ..Default::default()
        };
        // SAFETY: `class` is a live local, and Windows copies it during the call.
        unsafe { RegisterClassW(&class) };
        // SAFETY: the class was registered above, and every other argument is a plain value.
        let window = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                CLASS,
                None,
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                Some(module.into()),
                None,
            )
        };
        Self(window.expect("Windows can create a message window"))
    }
}

impl Drop for MessageWindow {
    fn drop(&mut self) {
        // SAFETY: this value owns the window, and it's dropped on the thread that created it.
        let _ = unsafe { DestroyWindow(self.0) };
    }
}

/// Whatever owns a window or a registration drops here on the way out, in reverse order: the tray icon first, and the
/// message window last.
fn run(
    notifier: Notifier,
    tray_icon: TrayIconImage,
    return_delay: Duration,
    started: Sender<isize>,
) {
    NOTIFIER.with(|cell| cell.set(notifier).ok());
    let window = MessageWindow::create();
    let hotkey = Hotkey::register(window.0);
    let _foreground = Foreground::start(foreground::read_snapshot, notify, return_delay);
    // The Overlay shows after this, so by then the hotkey works, and a fullscreen app already in front has been reported.
    // If the caller has gone, there's nobody to tell.
    let _ = started.send(window.0.0 as isize);
    if hotkey.is_none() {
        log::warn!("Ctrl+Alt+P is taken by another app, so the hotkey is off");
    }
    let _tray = tray::add(tray_icon, hotkey.is_some(), notify);

    let mut message = MSG::default();
    // SAFETY: `message` is a live local. GetMessageW returns 0 on quit and -1 on an error, and both end the loop.
    while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
        // SAFETY: `message` was just filled in by GetMessageW.
        unsafe { DispatchMessageW(&message) };
    }
}

/// The running shell thread. Dropping this stops and joins it.
pub struct ShellThread {
    window: isize,
    thread: Option<JoinHandle<()>>,
}

impl ShellThread {
    pub fn spawn(
        events: Sender<ShellEvent>,
        wake: Wake,
        tray_icon: TrayIconImage,
        return_delay: Duration,
    ) -> Self {
        let (started, window) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("shell".into())
            .spawn(move || run(Notifier { events, wake }, tray_icon, return_delay, started))
            .expect("the OS can start a thread");
        Self {
            window: window.recv().expect("the shell thread starts"),
            thread: Some(thread),
        }
    }
}

impl Drop for ShellThread {
    fn drop(&mut self) {
        let window = HWND(self.window as *mut c_void);
        // SAFETY: takes plain values. A window that's gone makes this fail, and then the thread has already stopped.
        let _ = unsafe { PostMessageW(Some(window), WM_CLOSE, WPARAM(0), LPARAM(0)) };
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
