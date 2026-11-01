//! Launches the real exe and acts on the real desktop through Windows, the way a person would.

use std::io;
use std::os::windows::io::AsRawHandle;
use std::process::{Child, Command, ExitStatus};
use std::thread;
use std::time::{Duration, Instant};

use pet_core::geometry::{Point, Rect};
use windows::Win32::Foundation::{HANDLE, HWND, LPARAM, POINT, RECT, WAIT_OBJECT_0, WPARAM};
use windows::Win32::System::Threading::WaitForSingleObject;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_MOUSE, MOUSE_EVENT_FLAGS, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEINPUT, SendInput,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GA_ROOT, GWL_EXSTYLE, GetAncestor, GetCursorPos, GetForegroundWindow,
    GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId, IsWindowVisible, PostMessageW,
    SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetCursorPos, SystemParametersInfoW,
    WM_CLOSE, WindowFromPoint,
};
use windows::core::BOOL;

/// Long enough for the 8 ms hit-test poll to react to a cursor move.
const SETTLE: Duration = Duration::from_millis(60);

const MIN_OVERLAY_WIDTH: i32 = 100;

const CLOSE_TIMEOUT_MS: u32 = 5_000;

pub fn wait_until(timeout: Duration, mut check: impl FnMut() -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if check() {
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
    check()
}

/// Screen coordinates here must be physical pixels, like the app's.
pub fn make_dpi_aware() {
    // SAFETY: takes a constant and touches no memory. A second call in one process fails harmlessly.
    let _ = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
}

pub fn work_area() -> Rect {
    let mut rect = RECT::default();
    // SAFETY: `rect` is a live local of the type SPI_GETWORKAREA writes.
    unsafe {
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut rect as *mut RECT as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    }
    .expect("read the work area");
    Rect {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    }
}

pub fn foreground_window() -> HWND {
    // SAFETY: takes no arguments.
    unsafe { GetForegroundWindow() }
}

/// The running app. Dropping it closes the process, so a failed test leaves nothing behind.
pub struct PetProcess {
    child: Child,
    overlay: HWND,
}

impl PetProcess {
    pub fn launch() -> Self {
        let child = Command::new(env!("CARGO_BIN_EXE_desktop-pet"))
            .spawn()
            .expect("start the app");
        let mut process = Self {
            child,
            overlay: HWND::default(),
        };
        let pid = process.child.id();
        let appeared = wait_until(Duration::from_secs(30), || match visible_window_of(pid) {
            Some(hwnd) => {
                process.overlay = hwnd;
                true
            }
            None => false,
        });
        assert!(appeared, "the Overlay never became visible");
        process
    }

    pub fn overlay(&self) -> HWND {
        self.overlay
    }

    pub fn ex_style(&self) -> u32 {
        // SAFETY: takes plain values. A window that's gone returns 0, which fails the test's assertions.
        unsafe { GetWindowLongPtrW(self.overlay, GWL_EXSTYLE) as u32 }
    }

    pub fn close(mut self) -> io::Result<ExitStatus> {
        self.ask_to_close()
    }

    /// Asks the app to close the way Windows does, and only kills it if that fails. A killed app can't clean up.
    fn ask_to_close(&mut self) -> io::Result<ExitStatus> {
        // SAFETY: takes plain values. A window that's gone makes this fail, and the wait below still ends.
        let _ = unsafe { PostMessageW(Some(self.overlay), WM_CLOSE, WPARAM(0), LPARAM(0)) };
        let process = HANDLE(self.child.as_raw_handle());
        // SAFETY: `self.child` owns this handle and outlives the call.
        if unsafe { WaitForSingleObject(process, CLOSE_TIMEOUT_MS) } != WAIT_OBJECT_0 {
            self.child.kill()?;
        }
        self.child.wait()
    }
}

impl Drop for PetProcess {
    fn drop(&mut self) {
        let _ = self.ask_to_close();
    }
}

struct Search {
    pid: u32,
    found: Option<HWND>,
}

unsafe extern "system" fn visit_window(hwnd: HWND, search: LPARAM) -> BOOL {
    // SAFETY: `visible_window_of` passes a `Search` that outlives the synchronous enumeration.
    let search = unsafe { &mut *(search.0 as *mut Search) };
    let mut pid = 0;
    // SAFETY: `pid` is a live local that Windows writes during the call.
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    // SAFETY: `hwnd` comes straight from the enumeration.
    if pid == search.pid && unsafe { IsWindowVisible(hwnd) }.as_bool() {
        // winit keeps its own tiny helper window "visible", so the size tells the Overlay apart.
        let mut rect = RECT::default();
        // SAFETY: `rect` is a live local that Windows writes during the call.
        let measured = unsafe { GetWindowRect(hwnd, &mut rect) }.is_ok();
        if measured && rect.right - rect.left > MIN_OVERLAY_WIDTH {
            search.found = Some(hwnd);
            return false.into();
        }
    }
    true.into()
}

fn visible_window_of(pid: u32) -> Option<HWND> {
    let mut search = Search { pid, found: None };
    // SAFETY: `visit_window` matches the expected signature and `search` outlives this call.
    // Stopping early is reported as an error, so the result is ignored.
    let _ = unsafe {
        EnumWindows(
            Some(visit_window),
            LPARAM(&mut search as *mut Search as isize),
        )
    };
    search.found
}

/// Puts the cursor back where the person left it, even when a test fails.
pub struct CursorGuard(POINT);

impl CursorGuard {
    pub fn new() -> Self {
        let mut original = POINT::default();
        // SAFETY: `original` is a live local that Windows writes during the call.
        unsafe { GetCursorPos(&mut original) }.expect("read the cursor");
        Self(original)
    }
}

impl Drop for CursorGuard {
    fn drop(&mut self) {
        // SAFETY: takes plain values.
        let _ = unsafe { SetCursorPos(self.0.x, self.0.y) };
    }
}

fn mouse_button(flags: MOUSE_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dwFlags: flags,
                ..Default::default()
            },
        },
    }
}

/// Moves the real cursor to `point` and asks Windows which top-level window would get a click there.
pub fn window_under_cursor_at(point: Point, settle: Duration) -> HWND {
    // SAFETY: takes plain values.
    unsafe { SetCursorPos(point.x, point.y) }.expect("move the cursor");
    thread::sleep(settle);
    let point = POINT {
        x: point.x,
        y: point.y,
    };
    // SAFETY: both calls take plain values.
    unsafe { GetAncestor(WindowFromPoint(point), GA_ROOT) }
}

/// A real left click. It refuses to fire unless `point` is over one of `allowed`, so a broken test can
/// never click an app the person has open.
pub fn click(point: Point, allowed: &[HWND]) {
    let under = window_under_cursor_at(point, SETTLE);
    assert!(
        allowed.contains(&under),
        "refusing to click {point:?}: not over a test window"
    );
    let inputs = [
        mouse_button(MOUSEEVENTF_LEFTDOWN),
        mouse_button(MOUSEEVENTF_LEFTUP),
    ];
    // SAFETY: `inputs` is a live array and the size is that of its element type.
    let sent = unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
    assert_eq!(sent, 2, "Windows rejected the click");
    thread::sleep(SETTLE);
}
