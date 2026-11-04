//! Notices a fullscreen application entering or leaving the foreground, on the shell thread.
//!
//! Three things can start a check, and they all only restart one timer, so any of them can go without touching the
//! others or the rule in `pet-core`:
//! - The shell's own "fullscreen app" message, which is what catches F11, where the foreground window doesn't change.
//!   It needs an appbar, an API for docked toolbars. Registering one claims no screen space until it asks for some,
//!   which this one never does.
//! - A hook on the foreground window changing. The hook runs in this process, reads no keys and subscribes to nothing
//!   about typing.
//! - A check every 2 s, the one poll here. Windows promises neither of the others fires for every change, a stale Hidden
//!   looks like a crash, and each foreground change measured with no event reached only this check.
//!
//! The first two come from outside this app, through `Signals`, so the tests can stand in their own and stay deaf to
//! the owner switching windows while they run. The rules for the timer are `FullscreenChecks` in `pet-core`.
//!
//! This reads the foreground window's geometry and whether it declares itself never fullscreen, and never its title
//! (SEN-01, SEN-02).

use std::cell::{Cell, OnceCell};
use std::time::Duration;

use pet_core::fullscreen::{ForegroundWindow, FullscreenChecks, Snapshot};
use pet_core::geometry::Rect;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows::Win32::UI::Shell::{
    ABM_NEW, ABM_REMOVE, APPBARDATA, QUNS_RUNNING_D3D_FULL_SCREEN, SHAppBarMessage,
    SHQueryUserNotificationState,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, EVENT_SYSTEM_FOREGROUND, GetClientRect,
    GetForegroundWindow, GetPropW, GetShellWindow, KillTimer, RegisterClassW,
    RegisterWindowMessageW, SetTimer, WINDOW_EX_STYLE, WINEVENT_OUTOFCONTEXT,
    WINEVENT_SKIPOWNPROCESS, WM_APP, WM_TIMER, WNDCLASSW, WS_POPUP,
};
use windows::core::w;

use super::monitor::home_monitor;
use super::window_styles::NOT_FULLSCREEN_PROPERTY;
use crate::ShellEvent;

/// What the shell sends the appbar when a fullscreen app starts or the last one closes. The message's own value only says
/// what the shell thinks, which isn't always about the foreground window, so it only starts a check.
const SHELL_MESSAGE: u32 = WM_APP + 1;

const CHECK_SOON_TIMER: usize = 1;
const RE_CHECK_TIMER: usize = 2;

const RE_CHECK: Duration = Duration::from_secs(2);

/// Where the news that something outside this app may have changed comes from.
trait Signals {
    /// Starts sending it to `window`'s watcher, on this thread. Dropping the result stops it.
    fn listen(window: HWND) -> Self;

    /// Sets up again what a restarted Explorer has forgotten. This takes no `self`, because the window's callback has
    /// no way to reach the value `listen` returned.
    fn listen_again(window: HWND);
}

struct Watcher {
    window: HWND,
    /// Explorer's "I've just restarted" broadcast. A new Explorer has forgotten every appbar.
    taskbar_created: u32,
    listen_again: fn(HWND),
    checks: Cell<FullscreenChecks>,
    read: Box<dyn Fn() -> Snapshot>,
    report: Box<dyn Fn(ShellEvent)>,
}

impl Watcher {
    fn change_checks<T>(&self, change: impl FnOnce(&mut FullscreenChecks) -> T) -> T {
        let mut checks = self.checks.get();
        let result = change(&mut checks);
        self.checks.set(checks);
        result
    }

    /// Starts or restarts the wait before a check, so a burst of changes causes one.
    fn check_soon(&self) {
        let wait = self.change_checks(FullscreenChecks::something_changed);
        self.wait(wait);
    }

    fn wait(&self, wait: Duration) {
        // SAFETY: takes plain values, and `window` belongs to this thread.
        unsafe {
            SetTimer(
                Some(self.window),
                CHECK_SOON_TIMER,
                wait.as_millis() as u32,
                None,
            )
        };
    }

    fn check(&self) {
        // Before the read, so a change that arrives while reading keeps the wait it starts.
        self.change_checks(FullscreenChecks::wait_ended);
        let snapshot = (self.read)();
        if let Some(fullscreen) = self.change_checks(|checks| checks.check(&snapshot)) {
            (self.report)(ShellEvent::FullscreenChanged(fullscreen));
        }
    }

    fn on_timer(&self, timer: usize) {
        if timer == CHECK_SOON_TIMER {
            // Windows timers repeat, so this one has to be stopped after its first tick.
            // SAFETY: takes plain values, and `window` belongs to this thread.
            let _ = unsafe { KillTimer(Some(self.window), CHECK_SOON_TIMER) };
            self.check();
        } else if let Some(wait) = self.change_checks(FullscreenChecks::re_check_due) {
            self.wait(wait);
        }
    }
}

thread_local! {
    /// Windows calls the callbacks below with no arguments of ours, so they find this here.
    static WATCHER: OnceCell<Watcher> = const { OnceCell::new() };
}

fn with_watcher(act: impl FnOnce(&Watcher)) {
    WATCHER.with(|watcher| {
        if let Some(watcher) = watcher.get() {
            act(watcher);
        }
    });
}

fn is_taskbar_created(message: u32) -> bool {
    WATCHER.with(|watcher| {
        watcher
            .get()
            .is_some_and(|watcher| watcher.taskbar_created == message)
    })
}

unsafe extern "system" fn on_foreground_change(
    _hook: HWINEVENTHOOK,
    _event: u32,
    _window: HWND,
    _object: i32,
    _child: i32,
    _thread: u32,
    _time: u32,
) {
    with_watcher(Watcher::check_soon);
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        SHELL_MESSAGE => with_watcher(Watcher::check_soon),
        WM_TIMER => with_watcher(|watcher| watcher.on_timer(wparam.0)),
        message if is_taskbar_created(message) => with_watcher(|watcher| {
            (watcher.listen_again)(hwnd);
            watcher.check_soon();
        }),
        // SAFETY: passes on exactly what Windows gave this callback.
        _ => return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
    LRESULT(0)
}

/// Registering twice does nothing, so this is safe to repeat after an Explorer restart.
fn register_appbar(window: HWND) {
    let mut appbar = APPBARDATA {
        cbSize: size_of::<APPBARDATA>() as u32,
        hWnd: window,
        uCallbackMessage: SHELL_MESSAGE,
        ..Default::default()
    };
    // SAFETY: `appbar` is a live local that Windows reads during the call.
    unsafe { SHAppBarMessage(ABM_NEW, &mut appbar) };
}

/// The appbar's registration. Without it, nothing is left on the shell's list after this app exits.
struct AppBar(HWND);

impl Drop for AppBar {
    fn drop(&mut self) {
        let mut appbar = APPBARDATA {
            cbSize: size_of::<APPBARDATA>() as u32,
            hWnd: self.0,
            ..Default::default()
        };
        // SAFETY: `appbar` is a live local that Windows reads during the call.
        unsafe { SHAppBarMessage(ABM_REMOVE, &mut appbar) };
    }
}

/// Unhooks on drop, which has to happen on the thread that hooked.
struct ForegroundHook(HWINEVENTHOOK);

impl ForegroundHook {
    fn install() -> Self {
        // SAFETY: the callback matches the type Windows expects and stays valid for the whole program. With
        // WINEVENT_OUTOFCONTEXT nothing loads into other apps, and events arrive on this thread's message loop.
        Self(unsafe {
            SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(on_foreground_change),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            )
        })
    }
}

impl Drop for ForegroundHook {
    fn drop(&mut self) {
        // SAFETY: this value owns the hook, and it's dropped on the thread that installed it.
        let _ = unsafe { UnhookWinEvent(self.0) };
    }
}

/// What the shell and the desktop say: the appbar hears the shell's message, and the hook hears the foreground window
/// change anywhere on the desktop. Dropping this unhooks first.
pub struct DesktopSignals {
    _hook: ForegroundHook,
    _appbar: AppBar,
}

impl Signals for DesktopSignals {
    fn listen(window: HWND) -> Self {
        register_appbar(window);
        Self {
            _hook: ForegroundHook::install(),
            _appbar: AppBar(window),
        }
    }

    /// Registering twice does nothing, so this is safe whatever Explorer remembers.
    fn listen_again(window: HWND) {
        register_appbar(window);
    }
}

/// A window that's never shown. It has to be a real top-level window, because a message-only window hears no broadcasts,
/// and the one that says Explorer has restarted is a broadcast.
struct HiddenWindow(HWND);

impl HiddenWindow {
    fn create() -> Self {
        // SAFETY: passing no name asks for this program's own handle.
        let module = unsafe { GetModuleHandleW(None) }.expect("this program has a module handle");
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: module.into(),
            lpszClassName: w!("DesktopPetForegroundWatcher"),
            ..Default::default()
        };
        // SAFETY: `class` is a live local, and Windows copies it during the call. A second registration fails harmlessly.
        unsafe { RegisterClassW(&class) };
        // SAFETY: the class was registered above, and every other argument is a plain value.
        let window = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("DesktopPetForegroundWatcher"),
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
        Self(window.expect("Windows can create a hidden window"))
    }
}

impl Drop for HiddenWindow {
    fn drop(&mut self) {
        // SAFETY: this value owns the window, and it's dropped on the thread that created it. Its timers go with it.
        let _ = unsafe { DestroyWindow(self.0) };
    }
}

/// Everything that keeps the watching going. Dropping this on the thread that started it stops it, signals first.
pub struct Foreground<S = DesktopSignals> {
    _signals: S,
    _window: HiddenWindow,
}

impl Foreground {
    /// Starts watching on this thread, which must run a message loop. The first check happens before this returns, so a
    /// fullscreen app already in front is reported before anything can show.
    pub fn start(
        read: impl Fn() -> Snapshot + 'static,
        report: impl Fn(ShellEvent) + 'static,
        return_delay: Duration,
    ) -> Self {
        Self::start_with(read, report, return_delay)
    }
}

impl<S> Foreground<S> {
    fn start_with(
        read: impl Fn() -> Snapshot + 'static,
        report: impl Fn(ShellEvent) + 'static,
        return_delay: Duration,
    ) -> Self
    where
        S: Signals,
    {
        let window = HiddenWindow::create();
        // SAFETY: the name is a static string.
        let taskbar_created = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
        WATCHER
            .with(|watcher| {
                watcher.set(Watcher {
                    window: window.0,
                    taskbar_created,
                    listen_again: S::listen_again,
                    checks: Cell::new(FullscreenChecks::new(return_delay)),
                    read: Box::new(read),
                    report: Box::new(report),
                })
            })
            .ok()
            .expect("one thread runs one watcher");
        let signals = S::listen(window.0);
        let re_check = RE_CHECK.as_millis() as u32;
        // SAFETY: takes plain values, and the window belongs to this thread.
        unsafe { SetTimer(Some(window.0), RE_CHECK_TIMER, re_check, None) };
        // Everything that can notice a change is in place by now, so none falls between this check and the next.
        with_watcher(Watcher::check);
        Self {
            _signals: signals,
            _window: window,
        }
    }
}

fn client_area(window: HWND) -> Option<Rect> {
    let mut client = RECT::default();
    // SAFETY: `client` is a live local that Windows writes during the call. A window that's gone makes this fail.
    unsafe { GetClientRect(window, &mut client) }.ok()?;
    let mut top_left = POINT {
        x: client.left,
        y: client.top,
    };
    let mut bottom_right = POINT {
        x: client.right,
        y: client.bottom,
    };
    // SAFETY: `top_left` is a live local that Windows updates during the call.
    let converted = unsafe { ClientToScreen(window, &mut top_left) }.as_bool();
    // SAFETY: as above, for `bottom_right`.
    let converted = converted && unsafe { ClientToScreen(window, &mut bottom_right) }.as_bool();
    converted.then_some(Rect {
        left: top_left.x,
        top: top_left.y,
        right: bottom_right.x,
        bottom: bottom_right.y,
    })
}

fn read_foreground() -> Option<ForegroundWindow> {
    // SAFETY: takes no arguments.
    let window = unsafe { GetForegroundWindow() };
    if window.0.is_null() {
        return None;
    }
    // SAFETY: takes no arguments.
    let desktop = unsafe { GetShellWindow() };
    // SAFETY: the name is a static string, and a window that's gone reads as having no such property.
    let flag = unsafe { GetPropW(window, NOT_FULLSCREEN_PROPERTY) };
    Some(ForegroundWindow {
        client_area: client_area(window)?,
        is_desktop: window == desktop,
        declares_not_fullscreen: !flag.0.is_null(),
    })
}

/// The plain facts the rule in `pet-core` needs, read fresh each time because a game can change the display mode.
pub fn read_snapshot() -> Snapshot {
    // This runs inside a Windows callback, where a panic would end the app, so a failed read only means "can't tell".
    let Some(home_monitor) = home_monitor() else {
        return Snapshot::default();
    };
    // SAFETY: takes no arguments.
    let shell_state = unsafe { SHQueryUserNotificationState() };
    Snapshot {
        foreground: read_foreground(),
        home_monitor,
        exclusive_fullscreen: shell_state == Ok(QUNS_RUNNING_D3D_FULL_SCREEN),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
    use std::sync::{Arc, Mutex};
    use std::thread::{self, JoinHandle};
    use std::time::Instant;

    use pet_core::fullscreen::RETURN_DELAY;
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, MSG, PostMessageW, PostThreadMessageW, SendMessageW, WM_QUIT,
    };

    use super::*;

    /// Shorter than `RE_CHECK`, so only the trigger under test can have done it.
    const SOON: Duration = Duration::from_secs(1);

    /// A Windows timer can land one tick of the system clock early.
    const TIMER_TICK: Duration = Duration::from_millis(16);

    const MONITOR: Rect = Rect {
        left: 0,
        top: 0,
        right: 100,
        bottom: 100,
    };

    fn snapshot(fullscreen: bool) -> Snapshot {
        let right = if fullscreen { 100 } else { 50 };
        Snapshot {
            foreground: Some(ForegroundWindow {
                client_area: Rect { right, ..MONITOR },
                ..Default::default()
            }),
            home_monitor: MONITOR,
            exclusive_fullscreen: false,
        }
    }

    /// Signals from the test alone. It posts the shell's message itself, and with no hook and no appbar, the owner
    /// switching windows or going fullscreen can't move its timing. Only an Explorer restart still reaches the window.
    struct TestSignals;

    impl Signals for TestSignals {
        fn listen(_window: HWND) -> Self {
            Self
        }

        fn listen_again(_window: HWND) {}
    }

    /// A `Foreground` on a message loop of its own, with a made-up foreground window.
    struct Running {
        window: isize,
        thread_id: u32,
        fullscreen: Arc<AtomicBool>,
        reads: Arc<AtomicUsize>,
        events: Receiver<ShellEvent>,
        thread: Option<JoinHandle<()>>,
    }

    impl Running {
        fn start(fullscreen: bool, return_delay: Duration) -> Self {
            Self::start_with::<TestSignals>(fullscreen, return_delay)
        }

        /// For the few tests of the real signals. They only check what something unrelated can't undo.
        fn start_with_desktop_signals(fullscreen: bool, return_delay: Duration) -> Self {
            Self::start_with::<DesktopSignals>(fullscreen, return_delay)
        }

        fn start_with<S: Signals + 'static>(fullscreen: bool, return_delay: Duration) -> Self {
            let fullscreen = Arc::new(AtomicBool::new(fullscreen));
            let reads = Arc::new(AtomicUsize::new(0));
            let (sender, events) = mpsc::channel();
            let (started, ids) = mpsc::channel();
            let (read_fullscreen, count_reads) = (fullscreen.clone(), reads.clone());
            let thread = thread::spawn(move || {
                let read = move || {
                    count_reads.fetch_add(1, Ordering::SeqCst);
                    snapshot(read_fullscreen.load(Ordering::SeqCst))
                };
                let report = move |event| sender.send(event).expect("the test is listening");
                let foreground = Foreground::<S>::start_with(read, report, return_delay);
                let window = foreground._window.0.0 as isize;
                // SAFETY: takes no arguments.
                let thread_id = unsafe { GetCurrentThreadId() };
                started
                    .send((window, thread_id))
                    .expect("the test is waiting");
                let mut message = MSG::default();
                // SAFETY: `message` is a live local, and GetMessageW fills it in before it's read.
                while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
                    // SAFETY: `message` was just filled in by GetMessageW.
                    unsafe { DispatchMessageW(&message) };
                }
            });
            let (window, thread_id) = ids.recv().expect("the watcher starts");
            Self {
                window,
                thread_id,
                fullscreen,
                reads,
                events,
                thread: Some(thread),
            }
        }

        fn window(&self) -> HWND {
            HWND(self.window as *mut _)
        }

        fn app_goes_fullscreen(&self) {
            self.fullscreen.store(true, Ordering::SeqCst);
        }

        fn app_leaves_fullscreen(&self) {
            self.fullscreen.store(false, Ordering::SeqCst);
        }

        fn shell_says_something_changed(&self) {
            // SAFETY: takes plain values, and the window lives as long as `self`.
            unsafe { PostMessageW(Some(self.window()), SHELL_MESSAGE, WPARAM(0), LPARAM(0)) }
                .expect("reach the watcher");
        }
    }

    impl Drop for Running {
        fn drop(&mut self) {
            // SAFETY: takes plain values, and the loop above ends on WM_QUIT.
            let _ = unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0)) };
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    #[test]
    fn given_a_fullscreen_app_in_front_when_starting_then_it_is_reported_before_start_returns() {
        // Given
        let reported = Arc::new(Mutex::new(Vec::new()));
        let record = reported.clone();

        // When
        let _foreground = Foreground::start(
            || snapshot(true),
            move |event| record.lock().unwrap().push(event),
            RETURN_DELAY,
        );

        // Then: this thread has no message loop, so nothing could have run later
        assert_eq!(
            *reported.lock().unwrap(),
            [ShellEvent::FullscreenChanged(true)]
        );
    }

    #[test]
    fn given_nothing_fullscreen_when_starting_then_nothing_is_reported() {
        // Given / When
        let running = Running::start(false, RETURN_DELAY);

        // Then
        assert_eq!(
            running.events.recv_timeout(Duration::from_millis(300)),
            Err(RecvTimeoutError::Timeout)
        );
    }

    #[test]
    fn given_an_app_went_fullscreen_when_the_shell_says_so_then_the_change_is_reported_straight_away()
     {
        // Given
        let running = Running::start(false, RETURN_DELAY);
        running.app_goes_fullscreen();

        // When
        running.shell_says_something_changed();

        // Then
        assert_eq!(
            running.events.recv_timeout(SOON),
            Ok(ShellEvent::FullscreenChanged(true))
        );
    }

    #[test]
    fn given_an_app_went_fullscreen_when_nothing_signals_it_then_the_re_check_finds_it() {
        // Given
        let running = Running::start(false, RETURN_DELAY);

        // When
        running.app_goes_fullscreen();

        // Then
        assert_eq!(
            running.events.recv_timeout(RE_CHECK * 2),
            Ok(ShellEvent::FullscreenChanged(true))
        );
    }

    #[test]
    fn given_a_burst_of_changes_when_they_settle_then_the_window_is_read_once() {
        // Given: one read at the start
        let running = Running::start(false, RETURN_DELAY);

        // When
        for _ in 0..5 {
            running.shell_says_something_changed();
        }
        thread::sleep(Duration::from_millis(500));

        // Then
        assert_eq!(running.reads.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn given_the_fullscreen_app_comes_back_within_the_return_delay_when_it_settles_then_nothing_is_reported()
     {
        // Given
        let running = Running::start(true, Duration::from_millis(400));
        running.events.recv_timeout(SOON).expect("the first answer");

        // When: a quick Alt-Tab away and back
        running.app_leaves_fullscreen();
        running.shell_says_something_changed();
        thread::sleep(Duration::from_millis(100));
        running.app_goes_fullscreen();
        running.shell_says_something_changed();

        // Then
        assert_eq!(
            running.events.recv_timeout(Duration::from_millis(700)),
            Err(RecvTimeoutError::Timeout)
        );
    }

    #[test]
    fn given_a_return_delay_when_the_fullscreen_app_leaves_then_the_pet_waits_for_all_of_it() {
        // Given
        let return_delay = Duration::from_millis(500);
        let running = Running::start(true, return_delay);
        running.events.recv_timeout(SOON).expect("the first answer");

        // When
        let left = Instant::now();
        running.app_leaves_fullscreen();
        running.shell_says_something_changed();

        // Then
        assert_eq!(
            running.events.recv_timeout(SOON),
            Ok(ShellEvent::FullscreenChanged(false))
        );
        assert!(left.elapsed() >= return_delay - TIMER_TICK);
    }

    #[test]
    fn given_a_return_delay_longer_than_the_re_check_when_the_app_leaves_then_the_re_check_does_not_hold_it_back()
     {
        // Given: if each re-check restarted the wait, this delay would never end
        let return_delay = RE_CHECK + Duration::from_millis(500);
        let running = Running::start(true, return_delay);
        running.events.recv_timeout(SOON).expect("the first answer");

        // When
        running.app_leaves_fullscreen();
        running.shell_says_something_changed();

        // Then
        assert_eq!(
            running.events.recv_timeout(return_delay + SOON),
            Ok(ShellEvent::FullscreenChanged(false))
        );
    }

    #[test]
    fn given_the_desktop_signals_when_started_on_this_thread_then_windows_accepts_the_foreground_hook_and_the_appbar()
     {
        // Given / When: whether Windows then delivers the hook's events is left to the e2e tests
        let foreground = Foreground::start(|| snapshot(false), |_| {}, RETURN_DELAY);

        // Then: registering again fails only for a window that's already registered
        assert!(!foreground._signals._hook.0.is_invalid());
        let mut appbar = APPBARDATA {
            cbSize: size_of::<APPBARDATA>() as u32,
            hWnd: foreground._window.0,
            ..Default::default()
        };
        // SAFETY: `appbar` is a live local that Windows reads during the call.
        assert_eq!(unsafe { SHAppBarMessage(ABM_NEW, &mut appbar) }, 0);
    }

    #[test]
    fn given_the_test_signals_when_explorer_announces_a_restart_then_no_real_appbar_is_registered()
    {
        // Given
        let running = Running::start(false, RETURN_DELAY);

        // When
        // SAFETY: the name is a static string.
        let taskbar_created = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
        // SAFETY: takes plain values, and this returns once the watcher's thread has handled it.
        unsafe { SendMessageW(running.window(), taskbar_created, None, None) };

        // Then: registering succeeds only for a window that isn't registered yet, and this one is removed again
        let mut appbar = APPBARDATA {
            cbSize: size_of::<APPBARDATA>() as u32,
            hWnd: running.window(),
            ..Default::default()
        };
        // SAFETY: `appbar` is a live local that Windows reads during the call.
        let registered_now = unsafe { SHAppBarMessage(ABM_NEW, &mut appbar) };
        // SAFETY: as above.
        unsafe { SHAppBarMessage(ABM_REMOVE, &mut appbar) };
        assert_ne!(registered_now, 0);
    }

    #[test]
    fn given_explorer_restarted_when_it_announces_that_then_the_appbar_is_registered_again() {
        // Given: Explorer has forgotten the appbar
        let running = Running::start_with_desktop_signals(false, RETURN_DELAY);
        let mut appbar = APPBARDATA {
            cbSize: size_of::<APPBARDATA>() as u32,
            hWnd: running.window(),
            ..Default::default()
        };
        // SAFETY: `appbar` is a live local that Windows reads during the call.
        unsafe { SHAppBarMessage(ABM_REMOVE, &mut appbar) };

        // When
        // SAFETY: the name is a static string.
        let taskbar_created = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
        // SAFETY: takes plain values, and this returns once the watcher's thread has handled it.
        unsafe { SendMessageW(running.window(), taskbar_created, None, None) };

        // Then: registering again fails only for a window that's already registered
        // SAFETY: `appbar` is a live local that Windows reads during the call.
        assert_eq!(unsafe { SHAppBarMessage(ABM_NEW, &mut appbar) }, 0);
    }
}
