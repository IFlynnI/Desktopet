//! A plain window of one known colour that records the clicks it receives. It stands in for "the app
//! underneath" and gives the Overlay a known background.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Once, mpsc};
use std::thread::{self, JoinHandle};

use pet_core::geometry::Rect;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::CreateSolidBrush;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW,
    HWND_NOTOPMOST, IDC_ARROW, LoadCursorW, MSG, PostThreadMessageW, RegisterClassW,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos, TranslateMessage, WM_LBUTTONDOWN,
    WM_QUIT, WNDCLASSW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};
use windows::core::w;

/// The colour behind the Pet. Nothing in the placeholder art uses it.
pub const BACKGROUND: [u8; 3] = [255, 0, 255];

static CLICKS: AtomicUsize = AtomicUsize::new(0);
static REGISTER_CLASS: Once = Once::new();

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_LBUTTONDOWN {
        CLICKS.fetch_add(1, Ordering::SeqCst);
    }
    // SAFETY: forwards the arguments Windows just gave us.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

pub struct TestWindow {
    hwnd: isize,
    thread_id: u32,
    thread: Option<JoinHandle<()>>,
}

impl TestWindow {
    pub fn open(area: Rect) -> Self {
        let (sender, receiver) = mpsc::channel();
        let thread = thread::spawn(move || {
            let hwnd = create_window(area);
            // SAFETY: takes no arguments.
            let thread_id = unsafe { GetCurrentThreadId() };
            sender
                .send((hwnd.0 as isize, thread_id))
                .expect("report the window");
            pump_messages();
        });
        let (hwnd, thread_id) = receiver.recv().expect("create the test window");
        Self {
            hwnd,
            thread_id,
            thread: Some(thread),
        }
    }

    pub fn hwnd(&self) -> HWND {
        HWND(self.hwnd as *mut _)
    }

    /// Makes this an ordinary window again. It starts topmost because Windows may otherwise open it behind
    /// the app in the foreground, and it has to end up below the topmost Overlay, like a real app.
    pub fn drop_to_normal(&self) {
        // SAFETY: takes plain values, and the window's thread is pumping messages to answer the call.
        unsafe {
            SetWindowPos(
                self.hwnd(),
                Some(HWND_NOTOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )
        }
        .expect("make the test window a normal window");
    }

    pub fn clicks(&self) -> usize {
        CLICKS.load(Ordering::SeqCst)
    }
}

impl Drop for TestWindow {
    fn drop(&mut self) {
        // SAFETY: takes plain values. The thread that owns the window ends its loop on WM_QUIT.
        let _ = unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0)) };
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn create_window(area: Rect) -> HWND {
    let [r, g, b] = BACKGROUND.map(u32::from);
    // SAFETY: passing no module name returns this exe, and the arrow cursor is a stock resource.
    let (instance, cursor) = unsafe {
        (
            GetModuleHandleW(None).expect("get this module"),
            LoadCursorW(None, IDC_ARROW).expect("load the cursor"),
        )
    };
    // A class can only be registered once per process, and `cargo test` shares one process across tests.
    REGISTER_CLASS.call_once(|| {
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: instance.into(),
            hCursor: cursor,
            // SAFETY: takes a plain value. The brush lives as long as the test process.
            hbrBackground: unsafe { CreateSolidBrush(COLORREF(r | (g << 8) | (b << 16))) },
            lpszClassName: w!("DesktopPetE2eTestWindow"),
            ..Default::default()
        };
        // SAFETY: `class` is a fully initialised live local, and its name is a static string.
        let atom = unsafe { RegisterClassW(&class) };
        assert_ne!(atom, 0, "register the window class");
    });
    // SAFETY: the class is registered above, and every other argument is a plain value.
    unsafe {
        CreateWindowExW(
            WS_EX_TOPMOST,
            w!("DesktopPetE2eTestWindow"),
            w!("Desktop Pet e2e"),
            WS_POPUP | WS_VISIBLE,
            area.left,
            area.top,
            area.width(),
            area.height(),
            None,
            None,
            Some(instance.into()),
            None,
        )
    }
    .expect("create the test window")
}

fn pump_messages() {
    let mut msg = MSG::default();
    // SAFETY: `msg` is a live local that each call fills in before the next two read it.
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
