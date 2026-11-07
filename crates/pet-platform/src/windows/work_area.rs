//! Tells the engine when the Home monitor's work area changes: the taskbar is resized, moved or switches auto-hide, or
//! the display's resolution or scale changes.
//!
//! Windows announces these with broadcast messages, which it sends to every top-level window. The shell thread's
//! message window can't hear them: Microsoft's "Window Features" page says a message-only window "does not receive
//! broadcast messages" (https://learn.microsoft.com/en-us/windows/win32/winmsg/window-features#message-only-windows).
//! So this owns a real top-level window of its own that's never shown. `WM_SETTINGCHANGE` is "sent to all top-level
//! windows" (https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-settingchange), and `WM_DISPLAYCHANGE` is "only
//! sent to top-level windows" (https://learn.microsoft.com/en-us/windows/win32/gdi/wm-displaychange). Both arrive on the
//! shell thread's message loop, so nothing here polls.

use std::cell::{Cell, RefCell};

use pet_core::geometry::Rect;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassW, SPI_SETWORKAREA,
    WM_DISPLAYCHANGE, WM_SETTINGCHANGE, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP,
};
use windows::core::{PCWSTR, w};

use crate::ShellEvent;

const CLASS: PCWSTR = w!("DesktopPetWorkAreaWatcher");

/// Whether a message means the work area may have changed, so it's worth reading again.
///
/// - `WM_SETTINGCHANGE` comes after any system setting changes, and its `wParam` says which. Explorer sets
///   `SPI_SETWORKAREA` whenever the taskbar is resized, moved or switches auto-hide, which includes after a scale change.
/// - `WM_DISPLAYCHANGE` comes after the resolution changes. It can arrive before Explorer has moved the taskbar, so the
///   read may still find the old work area. That's harmless: an unchanged work area isn't reported, and the
///   `SPI_SETWORKAREA` that follows brings the new one.
fn may_have_changed_work_area(message: u32, wparam: WPARAM) -> bool {
    match message {
        WM_SETTINGCHANGE => wparam.0 == SPI_SETWORKAREA.0 as usize,
        WM_DISPLAYCHANGE => true,
        _ => false,
    }
}

/// The work area the engine was last told about, so only a real change is reported.
#[derive(Clone, Copy, Debug)]
struct LastWorkArea(Rect);

impl LastWorkArea {
    /// The new work area if it differs from the last one, which it then becomes.
    fn changed_to(&mut self, now: Rect) -> Option<Rect> {
        (now != self.0).then(|| {
            self.0 = now;
            now
        })
    }
}

struct Watcher {
    last: Cell<LastWorkArea>,
    read: Box<dyn Fn() -> Option<Rect>>,
    report: Box<dyn Fn(ShellEvent)>,
}

impl Watcher {
    /// A work area that can't be read is skipped, not reported, and the next message that passes the filter tries again.
    /// This runs inside a window's callback, where a panic would end the whole program.
    fn check(&self) {
        let Some(now) = (self.read)() else {
            return;
        };
        let mut last = self.last.get();
        let changed = last.changed_to(now);
        self.last.set(last);
        if let Some(work_area) = changed {
            (self.report)(ShellEvent::WorkAreaChanged(work_area));
        }
    }
}

thread_local! {
    /// Windows calls the window's callback with no arguments of ours, so it finds this here.
    static WATCHER: RefCell<Option<Watcher>> = const { RefCell::new(None) };
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if may_have_changed_work_area(message, wparam) {
        WATCHER.with_borrow(|watcher| {
            if let Some(watcher) = watcher {
                watcher.check();
            }
        });
    }
    // Both messages are only news, so Windows' default handling is still right for them.
    // SAFETY: passes on exactly what Windows gave this callback.
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

/// Watches for work-area changes on this thread, which must run a message loop. Dropping it stops the watching, and it
/// must be dropped on the thread that started it.
pub struct WorkAreaWatch {
    window: HWND,
}

impl WorkAreaWatch {
    /// `known` is the work area the engine already has, so only a change from it is reported. `read` gives the current
    /// one, or `None` when it can't be read. `report` is given each `ShellEvent::WorkAreaChanged`.
    pub fn start(
        known: Rect,
        read: impl Fn() -> Option<Rect> + 'static,
        report: impl Fn(ShellEvent) + 'static,
    ) -> Self {
        WATCHER.set(Some(Watcher {
            last: Cell::new(LastWorkArea(known)),
            read: Box::new(read),
            report: Box::new(report),
        }));
        Self {
            window: create_hidden_window(),
        }
    }
}

impl Drop for WorkAreaWatch {
    fn drop(&mut self) {
        // SAFETY: this value owns the window, and it's dropped on the thread that created it.
        let _ = unsafe { DestroyWindow(self.window) };
        WATCHER.set(None);
    }
}

/// A top-level window that's never shown. Nothing ever calls `ShowWindow` on it, and it's created without `WS_VISIBLE`.
/// It's a tool window, so even if it were shown it would get no taskbar button, and it never takes focus. It has no
/// owner, because broadcasts reach unowned top-level windows, hidden ones included.
fn create_hidden_window() -> HWND {
    // SAFETY: passing no name asks for this program's own handle.
    let module = unsafe { GetModuleHandleW(None) }.expect("this program has a module handle");
    let class = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        hInstance: module.into(),
        lpszClassName: CLASS,
        ..Default::default()
    };
    // SAFETY: `class` is a live local, and Windows copies it during the call. A second registration fails harmlessly.
    unsafe { RegisterClassW(&class) };
    // SAFETY: the class was registered above, and every other argument is a plain value.
    let window = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            CLASS,
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
    };
    window.expect("Windows can create a hidden window")
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use windows::Win32::UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GetWindowLongPtrW, IsWindowVisible, SPI_SETDESKWALLPAPER, SendMessageW,
        WM_TIMER,
    };

    use super::*;

    const START: Rect = Rect {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1032,
    };

    const TASKBAR_GREW: Rect = Rect {
        bottom: 1000,
        ..START
    };

    #[test]
    fn given_a_work_area_setting_change_when_filtering_then_it_may_have_changed_the_work_area() {
        // Given
        let wparam = WPARAM(SPI_SETWORKAREA.0 as usize);

        // When
        let may_have_changed = may_have_changed_work_area(WM_SETTINGCHANGE, wparam);

        // Then
        assert!(may_have_changed);
    }

    #[test]
    fn given_a_display_change_when_filtering_then_it_may_have_changed_the_work_area() {
        // Given: the bits per pixel, which don't matter here
        let wparam = WPARAM(32);

        // When
        let may_have_changed = may_have_changed_work_area(WM_DISPLAYCHANGE, wparam);

        // Then
        assert!(may_have_changed);
    }

    #[test]
    fn given_some_other_setting_change_when_filtering_then_the_work_area_is_left_alone() {
        // Given
        let wparam = WPARAM(SPI_SETDESKWALLPAPER.0 as usize);

        // When
        let may_have_changed = may_have_changed_work_area(WM_SETTINGCHANGE, wparam);

        // Then
        assert!(!may_have_changed);
    }

    #[test]
    fn given_an_unrelated_message_when_filtering_then_the_work_area_is_left_alone() {
        // Given: a message whose wParam happens to match
        let wparam = WPARAM(SPI_SETWORKAREA.0 as usize);

        // When
        let may_have_changed = may_have_changed_work_area(WM_TIMER, wparam);

        // Then
        assert!(!may_have_changed);
    }

    #[test]
    fn given_a_known_work_area_when_a_different_one_is_read_then_it_is_the_change() {
        // Given
        let mut last = LastWorkArea(START);

        // When
        let changed = last.changed_to(TASKBAR_GREW);

        // Then
        assert_eq!(changed, Some(TASKBAR_GREW));
    }

    #[test]
    fn given_a_known_work_area_when_the_same_one_is_read_then_there_is_no_change() {
        // Given
        let mut last = LastWorkArea(START);

        // When
        let changed = last.changed_to(START);

        // Then
        assert_eq!(changed, None);
    }

    #[test]
    fn given_a_change_was_reported_when_the_same_work_area_is_read_again_then_there_is_no_change() {
        // Given
        let mut last = LastWorkArea(START);
        last.changed_to(TASKBAR_GREW);

        // When
        let changed = last.changed_to(TASKBAR_GREW);

        // Then
        assert_eq!(changed, None);
    }

    /// A watch on this thread, with a made-up work area the test changes, and the reports and reads it saw.
    struct Watched {
        watch: WorkAreaWatch,
        work_area: Rc<Cell<Option<Rect>>>,
        reads: Rc<Cell<usize>>,
        reports: Rc<RefCell<Vec<ShellEvent>>>,
    }

    impl Watched {
        fn start() -> Self {
            let work_area = Rc::new(Cell::new(Some(START)));
            let reads = Rc::new(Cell::new(0));
            let reports = Rc::new(RefCell::new(Vec::new()));
            let (read_work_area, count_reads, keep_reports) =
                (work_area.clone(), reads.clone(), reports.clone());
            let watch = WorkAreaWatch::start(
                START,
                move || {
                    count_reads.set(count_reads.get() + 1);
                    read_work_area.get()
                },
                move |event| keep_reports.borrow_mut().push(event),
            );
            Self {
                watch,
                work_area,
                reads,
                reports,
            }
        }

        /// Sends the message straight to this one window, never as a broadcast. From the thread that owns the window,
        /// `SendMessageW` calls its callback before returning, so no message loop is needed.
        fn send(&self, message: u32, wparam: usize) {
            // SAFETY: takes plain values, and the window lives as long as `self`.
            unsafe {
                SendMessageW(
                    self.watch.window,
                    message,
                    Some(WPARAM(wparam)),
                    Some(LPARAM(0)),
                )
            };
        }
    }

    #[test]
    fn given_the_taskbar_grew_when_windows_says_the_work_area_changed_then_the_new_one_is_reported_once()
     {
        // Given
        let watched = Watched::start();
        watched.work_area.set(Some(TASKBAR_GREW));

        // When
        watched.send(WM_SETTINGCHANGE, SPI_SETWORKAREA.0 as usize);
        watched.send(WM_SETTINGCHANGE, SPI_SETWORKAREA.0 as usize);

        // Then
        assert_eq!(
            *watched.reports.borrow(),
            [ShellEvent::WorkAreaChanged(TASKBAR_GREW)]
        );
    }

    #[test]
    fn given_the_work_area_did_not_change_when_the_display_changes_then_nothing_is_reported() {
        // Given
        let watched = Watched::start();

        // When
        watched.send(WM_DISPLAYCHANGE, 32);

        // Then
        assert_eq!(watched.reads.get(), 1);
        assert!(watched.reports.borrow().is_empty());
    }

    #[test]
    fn given_some_other_setting_changed_when_windows_says_so_then_the_work_area_is_not_read() {
        // Given
        let watched = Watched::start();
        watched.work_area.set(Some(TASKBAR_GREW));

        // When
        watched.send(WM_SETTINGCHANGE, SPI_SETDESKWALLPAPER.0 as usize);

        // Then
        assert_eq!(watched.reads.get(), 0);
        assert!(watched.reports.borrow().is_empty());
    }

    #[test]
    fn given_the_work_area_cannot_be_read_when_windows_says_it_changed_then_nothing_is_reported_until_it_can()
     {
        // Given: mid-way through a display change, say
        let watched = Watched::start();
        watched.work_area.set(None);

        // When
        watched.send(WM_DISPLAYCHANGE, 32);

        // Then
        assert!(watched.reports.borrow().is_empty());

        // When
        watched.work_area.set(Some(TASKBAR_GREW));
        watched.send(WM_SETTINGCHANGE, SPI_SETWORKAREA.0 as usize);

        // Then
        assert_eq!(
            *watched.reports.borrow(),
            [ShellEvent::WorkAreaChanged(TASKBAR_GREW)]
        );
    }

    #[test]
    fn given_a_watch_when_it_starts_then_its_window_is_hidden_and_never_takes_focus_or_a_taskbar_button()
     {
        // Given
        let watched = Watched::start();

        // When
        // SAFETY: takes plain values, and the window lives as long as `watched`.
        let (visible, ex_style) = unsafe {
            (
                IsWindowVisible(watched.watch.window),
                GetWindowLongPtrW(watched.watch.window, GWL_EXSTYLE) as u32,
            )
        };

        // Then
        assert!(!visible.as_bool());
        assert_ne!(ex_style & WS_EX_TOOLWINDOW.0, 0);
        assert_ne!(ex_style & WS_EX_NOACTIVATE.0, 0);
    }
}
