//! Launches the real exe and acts on the real desktop through Windows, the way a person would.

use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::process::{self, Child, Command, ExitStatus};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{env, fs, io, thread};

use pet_core::geometry::{Point, Rect};
use windows::Win32::Foundation::{
    CloseHandle, ERROR_SUCCESS, HANDLE, HWND, LPARAM, POINT, RECT, WAIT_OBJECT_0, WPARAM,
};
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW,
};
use windows::Win32::System::Threading::{
    OpenMutexW, OpenProcess, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE, SYNCHRONIZATION_SYNCHRONIZE,
    TerminateProcess, WaitForSingleObject,
};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP,
    MOD_ALT, MOD_CONTROL, MOUSE_EVENT_FLAGS, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEINPUT,
    RegisterHotKey, SendInput, UnregisterHotKey, VIRTUAL_KEY, VK_CONTROL, VK_MENU, VK_P,
};
use windows::Win32::UI::Shell::{NOTIFYICONIDENTIFIER, Shell_NotifyIconGetRect};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, FindWindowExW, GA_ROOT, GWL_EXSTYLE, GetAncestor, GetClassNameW, GetCursorPos,
    GetForegroundWindow, GetSystemMetrics, GetWindowLongPtrW, GetWindowRect,
    GetWindowThreadProcessId, HWND_MESSAGE, IsWindowVisible, PostMessageW, SM_CXSCREEN,
    SM_CYSCREEN, SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetCursorPos,
    SystemParametersInfoW, WM_CLOSE, WM_COMMAND, WM_LBUTTONUP, WM_RBUTTONUP, WindowFromPoint,
};
use windows::core::{BOOL, PCWSTR, w};

/// Long enough for the 8 ms hit-test poll to react to a cursor move.
const SETTLE: Duration = Duration::from_millis(60);

const MIN_OVERLAY_WIDTH: i32 = 100;

const CLOSE_TIMEOUT_MS: u32 = 5_000;

const PRESENCE_CHANGE: Duration = Duration::from_secs(5);

const LAUNCH_TIMEOUT: Duration = Duration::from_secs(30);

/// Longer than the app takes to go from its message window to the first frame.
const STAYS_HIDDEN: Duration = Duration::from_secs(5);

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

/// Top-left corner of a `width` x `height` box centred along the bottom edge of `area`.
pub fn bottom_centre(area: Rect, width: i32, height: i32) -> Point {
    Point {
        x: area.left + (area.width() - width) / 2,
        y: area.bottom - height,
    }
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

/// The whole primary monitor, which is the Home monitor until settings arrive. It's in physical pixels once
/// `make_dpi_aware` has run.
pub fn monitor_area() -> Rect {
    // SAFETY: both take plain values.
    let (right, bottom) = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
    Rect {
        left: 0,
        top: 0,
        right,
        bottom,
    }
}

pub fn foreground_window() -> HWND {
    // SAFETY: takes no arguments.
    unsafe { GetForegroundWindow() }
}

const RUN_KEY: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");
const RUN_VALUE: PCWSTR = w!("Desktop Pet");

/// The app's entry under `key`, as raw text, or `None` when there isn't one.
pub fn read_startup_entry(key: PCWSTR) -> Option<Vec<u8>> {
    let mut bytes = 0;
    // SAFETY: the caller's name is a static string, and with no buffer Windows only writes the size to `bytes`.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key,
            RUN_VALUE,
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut bytes),
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    let mut value = vec![0; bytes as usize];
    // SAFETY: `value` holds the `bytes` Windows asked for, and both are live for the call.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key,
            RUN_VALUE,
            RRF_RT_REG_SZ,
            None,
            Some(value.as_mut_ptr().cast()),
            Some(&mut bytes),
        )
    };
    assert_eq!(status, ERROR_SUCCESS, "read the startup entry");
    Some(value)
}

/// Sets the entry under `key` to exactly `value`, or removes it for `None`.
pub fn write_startup_entry(key: PCWSTR, value: Option<&[u8]>) {
    // SAFETY: the caller's name is a static string, and `value` is live for the call.
    let _ = unsafe {
        match value {
            Some(value) => RegSetKeyValueW(
                HKEY_CURRENT_USER,
                key,
                RUN_VALUE,
                REG_SZ.0,
                Some(value.as_ptr().cast()),
                value.len() as u32,
            ),
            None => RegDeleteKeyValueW(HKEY_CURRENT_USER, key, RUN_VALUE),
        }
    };
}

/// The tests can reach the real Launch on startup item, so this makes sure none of them changes the owner's real
/// startup list without anyone noticing: it puts the entry back and fails the test.
pub struct StartupListGuard {
    key: PCWSTR,
    before: Option<Vec<u8>>,
}

impl StartupListGuard {
    fn new() -> Self {
        Self::of(RUN_KEY)
    }

    /// Watches `key`, so this can be tested against a key of its own.
    pub fn of(key: PCWSTR) -> Self {
        let before = read_startup_entry(key);
        Self { key, before }
    }
}

impl Drop for StartupListGuard {
    fn drop(&mut self) {
        if read_startup_entry(self.key) != self.before {
            write_startup_entry(self.key, self.before.as_deref());
            if !thread::panicking() {
                panic!("a test changed the real startup list, so it has been put back");
            }
        }
    }
}

/// What the test's own folder of Packs holds when the app starts.
#[derive(Clone, Copy)]
pub enum Packs {
    /// The placeholder Pack from the repo, which is what most tests want.
    Placeholder,
    /// No Packs at all.
    Empty,
    /// The placeholder with two mistakes put in: see `BROKEN_FIELDS`.
    Broken,
    /// The placeholder in the usual folder, but `DESKTOP_PET_PACKS` names another folder, holding the broken one.
    OverriddenWithBroken,
    /// The placeholder with its Anchor 8 art pixels left of centre, so the Pet stands 16 pixels right of where the
    /// placeholder does. See `write_off_centre_pack`.
    OffCentreAnchor,
    /// The placeholder with `SHADOW` under its Form. See `write_shadowed_pack`.
    WithShadow,
}

/// The shadow `Packs::WithShadow` adds: 30 art pixels wide, so at 2x it reaches 6 pixels past each side of the
/// 48-pixel Pet, and 8 high. Its colour is content/firstborn's.
pub const SHADOW: &str = r##"shadow = { width = 30, colour = "#1a132660" }"##;

/// Where the mistakes in `Packs::Broken` are, as the log names them.
pub const BROKEN_FIELDS: [&str; 2] = [
    "forms.hatchling.default_action",
    "forms.hatchling.animations.idle.durations_ms",
];

fn placeholder_pack() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/placeholder")
}

/// Every colour the placeholder's Frames draw fully opaque, as RGB.
pub fn placeholder_opaque_colours() -> Vec<[u8; 3]> {
    let pack = pet_content::load(&placeholder_pack()).unwrap_or_else(|errors| panic!("{errors:?}"));
    let form = pack.definition.starting_form;
    let mut colours: Vec<[u8; 3]> = pack
        .pixels
        .frames(form, pack.definition.form(form).default_animation())
        .iter()
        .flat_map(|frame| frame.rgba.chunks(4))
        .filter(|pixel| pixel[3] == 255)
        .map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect();
    colours.sort_unstable();
    colours.dedup();
    colours
}

fn copy_folder(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("make a Pack folder");
    for entry in fs::read_dir(from).expect("read the placeholder Pack") {
        let entry = entry.expect("read the placeholder Pack");
        let target = to.join(entry.file_name());
        if entry
            .file_type()
            .expect("read the placeholder Pack")
            .is_dir()
        {
            copy_folder(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("copy the placeholder Pack");
        }
    }
}

/// The placeholder Pack with a Form whose default Action doesn't exist, and a Frame of 0 ms.
fn write_broken_pack(folder: &Path) {
    copy_folder(&placeholder_pack(), folder);
    break_manifest(folder);
}

/// Puts the mistakes `BROKEN_FIELDS` names into the placeholder manifest in `folder`.
pub fn break_manifest(folder: &Path) {
    let manifest = folder.join("manifest.toml");
    let text = fs::read_to_string(&manifest).expect("read the manifest");
    let broken = text
        .replace("default_action = \"idle\"", "default_action = \"walk\"")
        .replace("durations_ms = [250, 250]", "durations_ms = [250, 0]");
    assert_ne!(
        broken, text,
        "the placeholder manifest changed, so update this"
    );
    fs::write(manifest, broken).expect("write the broken manifest");
}

/// The placeholder Pack with its Anchor moved from the middle of the cell, x = 12, to x = 4.
fn write_off_centre_pack(folder: &Path) {
    copy_folder(&placeholder_pack(), folder);
    let manifest = folder.join("manifest.toml");
    let text = fs::read_to_string(&manifest).expect("read the manifest");
    let moved = text.replace("anchor = { x = 12, y = 18 }", "anchor = { x = 4, y = 18 }");
    assert_ne!(
        moved, text,
        "the placeholder manifest changed, so update this"
    );
    fs::write(manifest, moved).expect("write the moved manifest");
}

/// The placeholder Pack with `SHADOW` under its Form.
fn write_shadowed_pack(folder: &Path) {
    copy_folder(&placeholder_pack(), folder);
    let manifest = folder.join("manifest.toml");
    let text = fs::read_to_string(&manifest).expect("read the manifest");
    let shadowed = text.replace(
        "default_action = \"idle\"",
        &format!(
            "default_action = \"idle\"
{SHADOW}"
        ),
    );
    assert_ne!(
        shadowed, text,
        "the placeholder manifest changed, so update this"
    );
    fs::write(manifest, shadowed).expect("write the shadowed manifest");
}

/// A folder of its own for the app's files, so tests never touch the owner's log or Packs. It goes when the test ends.
/// A test is its own process, so the process ID makes this unique.
pub struct TestData(PathBuf);

impl TestData {
    fn new() -> Self {
        let folder = env::temp_dir().join(format!("desktop-pet-e2e-{}", process::id()));
        fs::create_dir_all(&folder).expect("make a test folder");
        Self(folder)
    }

    pub fn log(&self) -> String {
        let log = self.0.join("DesktopPet").join("desktop-pet.log");
        fs::read_to_string(log).unwrap_or_default()
    }

    /// Where the app looks for Packs, unless `DESKTOP_PET_PACKS` says otherwise.
    fn packs_folder(&self) -> PathBuf {
        self.0.join("DesktopPet").join("packs")
    }

    /// The Pack the app uses, for every `Packs` but `Empty` and `OverriddenWithBroken`.
    pub fn pack_folder(&self) -> PathBuf {
        self.packs_folder().join("placeholder")
    }

    /// Fills the Packs folders, and says which one the app should look in. `None` means the usual one.
    fn add_packs(&self, packs: Packs) -> Option<PathBuf> {
        let usual = self.packs_folder();
        fs::create_dir_all(&usual).expect("make the Packs folder");
        match packs {
            Packs::Placeholder => copy_folder(&placeholder_pack(), &usual.join("placeholder")),
            Packs::Empty => {}
            Packs::Broken => write_broken_pack(&usual.join("placeholder")),
            Packs::OffCentreAnchor => write_off_centre_pack(&usual.join("placeholder")),
            Packs::WithShadow => write_shadowed_pack(&usual.join("placeholder")),
            Packs::OverriddenWithBroken => {
                copy_folder(&placeholder_pack(), &usual.join("placeholder"));
                let elsewhere = self.0.join("other-packs");
                write_broken_pack(&elsewhere.join("broken"));
                return Some(elsewhere);
            }
        }
        None
    }

    fn record_path(&self) -> PathBuf {
        self.0.join("record.txt")
    }

    /// What `failing-app` has written, one line per entry.
    pub fn record(&self) -> Vec<String> {
        fs::read_to_string(self.record_path())
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

impl Drop for TestData {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// When `failing-app` should fail.
#[derive(Clone, Copy)]
pub enum FailWhen {
    Shown,
    Hidden,
}

/// The running app. Dropping it closes the process, so a failed test leaves nothing behind.
pub struct PetProcess {
    child: Child,
    overlay: HWND,
    data: Arc<TestData>,
    _startup_list: StartupListGuard,
}

impl PetProcess {
    /// The app with the placeholder Pack.
    pub fn launch() -> Self {
        Self::launch_with(Packs::Placeholder)
    }

    /// The app with `packs` in its folders. With no usable Pack the Overlay still shows, with nothing on it.
    pub fn launch_with(packs: Packs) -> Self {
        Self::launch_from(Command::new(env!("CARGO_BIN_EXE_desktop-pet")), packs)
    }

    /// `failing-app`, which is the real app with a failure built in.
    pub fn launch_failing(when: FailWhen) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_failing-app"));
        let when = match when {
            FailWhen::Shown => "shown",
            FailWhen::Hidden => "hidden",
        };
        command.env("FAILING_APP_WHEN", when);
        Self::launch_from(command, Packs::Placeholder)
    }

    /// For a launch that should stay Hidden: this returns once the app is running, and fails if the Overlay ever shows.
    pub fn launch_kept_hidden() -> Self {
        let command = Command::new(env!("CARGO_BIN_EXE_desktop-pet"));
        let process = Self::start_from(command, Packs::Placeholder);
        let pid = process.child.id();
        let start = Instant::now();
        let mut running_since = None;
        while running_since.is_none_or(|since: Instant| since.elapsed() < STAYS_HIDDEN) {
            assert!(start.elapsed() < LAUNCH_TIMEOUT, "the app never started");
            assert!(visible_window_of(pid).is_none(), "the Overlay showed");
            if running_since.is_none() && message_window().is_some() {
                running_since = Some(Instant::now());
            }
            thread::sleep(Duration::from_millis(1));
        }
        process
    }

    /// Waits for an Overlay that started Hidden to show.
    pub fn wait_for_overlay(&mut self) -> bool {
        self.find_overlay(PRESENCE_CHANGE)
    }

    fn find_overlay(&mut self, timeout: Duration) -> bool {
        let pid = self.child.id();
        wait_until(timeout, || match visible_window_of(pid) {
            Some(hwnd) => {
                self.overlay = hwnd;
                true
            }
            None => false,
        })
    }

    fn launch_from(command: Command, packs: Packs) -> Self {
        let mut process = Self::start_from(command, packs);
        let appeared = process.find_overlay(LAUNCH_TIMEOUT);
        assert!(appeared, "the Overlay never became visible");
        process
    }

    fn start_from(mut command: Command, packs: Packs) -> Self {
        assert_no_pet_is_running();
        let startup_list = StartupListGuard::new();
        let data = Arc::new(TestData::new());
        // The owner's own override mustn't reach the app under test.
        command.env_remove("DESKTOP_PET_PACKS");
        if let Some(packs_folder) = data.add_packs(packs) {
            command.env("DESKTOP_PET_PACKS", packs_folder);
        }
        let child = command
            .env("LOCALAPPDATA", &data.0)
            .env("FAILING_APP_RECORD", data.record_path())
            .spawn()
            .expect("start the app");
        Self {
            child,
            overlay: HWND::default(),
            data,
            _startup_list: startup_list,
        }
    }

    pub fn overlay(&self) -> HWND {
        self.overlay
    }

    pub fn ex_style(&self) -> u32 {
        // SAFETY: takes plain values. A window that's gone returns 0, which fails the test's assertions.
        unsafe { GetWindowLongPtrW(self.overlay, GWL_EXSTYLE) as u32 }
    }

    /// What Windows reports for the Overlay. It's about the window, not what the screen shows.
    pub fn is_shown(&self) -> bool {
        // SAFETY: takes plain values. A window that's gone reads as not visible.
        unsafe { IsWindowVisible(self.overlay) }.as_bool()
    }

    pub fn wait_until_hidden(&self) -> bool {
        wait_until(PRESENCE_CHANGE, || !self.is_shown())
    }

    pub fn wait_until_shown(&self) -> bool {
        wait_until(PRESENCE_CHANGE, || self.is_shown())
    }

    pub fn child(&self) -> &Child {
        &self.child
    }

    /// The app's files, which outlive this value if a Relaunch is still using them.
    pub fn data(&self) -> Arc<TestData> {
        self.data.clone()
    }

    /// The app's tray icon. It's added just after the Overlay appears, so this waits for it.
    pub fn tray(&self) -> Tray {
        Tray::of(self.child.id())
    }

    /// Waits for the app to exit by itself, and kills it if it doesn't.
    pub fn wait_for_exit(mut self) -> io::Result<ExitStatus> {
        wait_or_kill(&mut self.child)
    }

    /// Starts the app again while this one runs, and waits for that second copy to exit.
    pub fn launch_again(&self) -> io::Result<ExitStatus> {
        wait_or_kill(&mut Command::new(env!("CARGO_BIN_EXE_desktop-pet")).spawn()?)
    }

    pub fn close(mut self) -> io::Result<ExitStatus> {
        self.ask_to_close()
    }

    /// Asks the app to close the way Windows does, and only kills it if that fails. A killed app can't clean up.
    fn ask_to_close(&mut self) -> io::Result<ExitStatus> {
        // SAFETY: takes plain values. A window that's gone makes this fail, and the wait below still ends.
        let _ = unsafe { PostMessageW(Some(self.overlay), WM_CLOSE, WPARAM(0), LPARAM(0)) };
        wait_or_kill(&mut self.child)
    }
}

/// A copy of the app started by a Relaunch, not by the test. Dropping it closes that copy.
pub struct Relaunched {
    pid: u32,
    process: HANDLE,
}

impl Relaunched {
    /// Waits for the new copy to make its message window, which is what a second launch looks for too.
    pub fn wait_for_it() -> Self {
        let mut window = None;
        let appeared = wait_until(PRESENCE_CHANGE, || {
            window = message_window();
            window.is_some()
        });
        assert!(appeared, "the app never relaunched");
        let mut pid = 0;
        // SAFETY: `pid` is a live local that Windows writes during the call.
        unsafe { GetWindowThreadProcessId(window.expect("it appeared"), Some(&mut pid)) };
        // SAFETY: takes plain values.
        let process = unsafe { OpenProcess(PROCESS_SYNCHRONIZE | PROCESS_TERMINATE, false, pid) }
            .expect("open the relaunched app");
        Self { pid, process }
    }

    pub fn overlay(&self) -> Option<HWND> {
        visible_window_of(self.pid)
    }

    pub fn tray(&self) -> Tray {
        Tray::of(self.pid)
    }
}

impl Drop for Relaunched {
    fn drop(&mut self) {
        if let Some(overlay) = self.overlay() {
            // SAFETY: takes plain values. A window that's gone makes this fail, and the wait below still ends.
            let _ = unsafe { PostMessageW(Some(overlay), WM_CLOSE, WPARAM(0), LPARAM(0)) };
            // SAFETY: this value keeps the handle open for the whole call.
            unsafe { WaitForSingleObject(self.process, CLOSE_TIMEOUT_MS) };
        }
        // SAFETY: this value owns the handle, and `drop` runs exactly once. Terminating an app that's gone does nothing.
        let _ = unsafe { TerminateProcess(self.process, 1) };
        // SAFETY: as above.
        let _ = unsafe { CloseHandle(self.process) };
    }
}

/// The window a second launch looks for, which the app makes before it can show the Overlay.
fn message_window() -> Option<HWND> {
    // SAFETY: the class name is a static string and the rest are plain values.
    let found = unsafe {
        FindWindowExW(
            Some(HWND_MESSAGE),
            None,
            w!("DesktopPetMessageWindow"),
            None,
        )
    };
    found.ok()
}

fn wait_or_kill(child: &mut Child) -> io::Result<ExitStatus> {
    let process = HANDLE(child.as_raw_handle());
    // SAFETY: `child` owns this handle and outlives the call.
    if unsafe { WaitForSingleObject(process, CLOSE_TIMEOUT_MS) } != WAIT_OBJECT_0 {
        child.kill()?;
    }
    child.wait()
}

/// The owner's own Pet would turn every launch here into a second launch, which exits, so stop with a clear reason.
fn assert_no_pet_is_running() {
    // SAFETY: the name is a static string and the rest are plain values.
    let opened = unsafe { OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, false, w!("Local\\DesktopPet")) };
    if let Ok(mutex) = opened {
        // SAFETY: this handle came from the call above and nothing else uses it.
        let _ = unsafe { CloseHandle(mutex) };
        panic!("quit the running Pet first");
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

/// The window class `tray-icon` registers.
const TRAY_CLASS: &str = "tray_icon_app";

/// The ID `tray-icon` gives the first icon: its counter starts at 1, and the first number goes on `TrayIconId`.
const TRAY_ICON_ID: u32 = 2;

/// What the shell sends the icon's window when something happens to it. It's `tray-icon`'s own private number, so a
/// new version could change it, and these tests would then fail.
const TRAY_CALLBACK_MESSAGE: u32 = 6002;

/// What `muda` numbers the menu's items, in the order they're made.
const SHOW_HIDE_ITEM: usize = 1000;
const QUIT_ITEM: usize = 1002;

struct TraySearch {
    pid: u32,
    found: Option<HWND>,
}

unsafe extern "system" fn visit_tray_window(hwnd: HWND, search: LPARAM) -> BOOL {
    // SAFETY: `tray_window_of` passes a `TraySearch` that outlives the synchronous enumeration.
    let search = unsafe { &mut *(search.0 as *mut TraySearch) };
    let mut pid = 0;
    // SAFETY: `pid` is a live local that Windows writes during the call.
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    let mut class = [0; 64];
    // SAFETY: `hwnd` comes straight from the enumeration, and `class` is a live local.
    let length = unsafe { GetClassNameW(hwnd, &mut class) } as usize;
    if pid == search.pid && String::from_utf16_lossy(&class[..length]) == TRAY_CLASS {
        search.found = Some(hwnd);
        return false.into();
    }
    true.into()
}

fn tray_window_of(pid: u32) -> Option<HWND> {
    let mut search = TraySearch { pid, found: None };
    // SAFETY: `visit_tray_window` matches the expected signature and `search` outlives this call.
    // Stopping early is reported as an error, so the result is ignored.
    let _ = unsafe {
        EnumWindows(
            Some(visit_tray_window),
            LPARAM(&mut search as *mut TraySearch as isize),
        )
    };
    search.found
}

/// The app's tray icon, driven with the same messages Windows sends it, so no mouse is needed. These can't prove what a
/// real click on the taskbar does, which is why that's a manual check.
#[derive(Clone, Copy)]
pub struct Tray {
    window: HWND,
}

impl Tray {
    /// The tray icon of process `pid`, waiting for it to appear.
    fn of(pid: u32) -> Self {
        let mut window = None;
        let found = wait_until(Duration::from_secs(5), || {
            window = tray_window_of(pid);
            window.is_some_and(|window| Tray { window }.icon_exists())
        });
        assert!(found, "the tray icon never appeared");
        Tray {
            window: window.expect("the search found it"),
        }
    }

    /// Whether the taskbar has the icon, for an app that's running. Don't use this to show an icon has gone: it fails as
    /// soon as the app's window has gone, even when a dead icon stays behind on the taskbar.
    pub fn icon_exists(&self) -> bool {
        let icon = NOTIFYICONIDENTIFIER {
            cbSize: size_of::<NOTIFYICONIDENTIFIER>() as u32,
            hWnd: self.window,
            uID: TRAY_ICON_ID,
            ..Default::default()
        };
        // SAFETY: `icon` is a live local that Windows reads during the call.
        unsafe { Shell_NotifyIconGetRect(&icon) }.is_ok()
    }

    pub fn left_click(&self) {
        let up = LPARAM(WM_LBUTTONUP as isize);
        // SAFETY: takes plain values.
        unsafe { PostMessageW(Some(self.window), TRAY_CALLBACK_MESSAGE, WPARAM(0), up) }
            .expect("reach the tray window");
    }

    pub fn right_click(&self) {
        let up = LPARAM(WM_RBUTTONUP as isize);
        // SAFETY: takes plain values.
        unsafe { PostMessageW(Some(self.window), TRAY_CALLBACK_MESSAGE, WPARAM(0), up) }
            .expect("reach the tray window");
    }

    fn choose(&self, item: usize) {
        // SAFETY: takes plain values.
        unsafe { PostMessageW(Some(self.window), WM_COMMAND, WPARAM(item), LPARAM(0)) }
            .expect("reach the tray window");
    }

    pub fn choose_show_hide(&self) {
        self.choose(SHOW_HIDE_ITEM);
    }

    pub fn choose_quit(&self) {
        self.choose(QUIT_ITEM);
    }
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

fn key(key: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                dwFlags: flags,
                ..Default::default()
            },
        },
    }
}

/// Presses Ctrl+Alt+P on the real keyboard path, the way a person does.
pub fn press_hotkey() {
    let inputs = [
        key(VK_CONTROL, KEYBD_EVENT_FLAGS(0)),
        key(VK_MENU, KEYBD_EVENT_FLAGS(0)),
        key(VK_P, KEYBD_EVENT_FLAGS(0)),
        key(VK_P, KEYEVENTF_KEYUP),
        key(VK_MENU, KEYEVENTF_KEYUP),
        key(VK_CONTROL, KEYEVENTF_KEYUP),
    ];
    // SAFETY: `inputs` is a live array and the size is that of its element type. The whole batch goes in as one, so
    // the modifier keys can't be left held down.
    let sent = unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
    assert_eq!(sent as usize, inputs.len(), "Windows rejected the keys");
}

/// Holds Ctrl+Alt+P for this thread, as another app would, until dropped.
pub struct HotkeyTaken;

impl HotkeyTaken {
    pub fn by_this_test() -> Self {
        // SAFETY: takes plain values.
        unsafe { RegisterHotKey(None, 1, MOD_CONTROL | MOD_ALT, u32::from(VK_P.0)) }
            .expect("Ctrl+Alt+P is free");
        Self
    }
}

impl Drop for HotkeyTaken {
    fn drop(&mut self) {
        // SAFETY: takes plain values, and this thread registered the key.
        let _ = unsafe { UnregisterHotKey(None, 1) };
    }
}

pub fn cursor_position() -> Point {
    let mut point = POINT::default();
    // SAFETY: `point` is a live local that Windows writes during the call.
    unsafe { GetCursorPos(&mut point) }.expect("read the cursor");
    Point {
        x: point.x,
        y: point.y,
    }
}

/// A person moved the mouse if it isn't where this test put it, so the results can't be trusted and the run should end.
pub fn assert_cursor_is_at(point: Point) {
    assert_eq!(
        cursor_position(),
        point,
        "the mouse moved during the run, so it stopped: run it again and keep your hands off the mouse"
    );
}

/// Moves the real cursor to `point` and asks Windows which top-level window would get a click there.
pub fn window_under_cursor_at(point: Point, settle: Duration) -> HWND {
    // SAFETY: takes plain values.
    unsafe { SetCursorPos(point.x, point.y) }.expect("move the cursor");
    thread::sleep(settle);
    assert_cursor_is_at(point);
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
