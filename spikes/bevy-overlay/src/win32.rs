//! Win32 side of the spike: DPI, work area, window styles, per-pixel click-through polling,
//! process metrics, screen capture checks and the cursor self-test.

use std::ffi::c_void;
use std::fs::{self, File};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{COLORREF, FILETIME, HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, CreateCompatibleBitmap,
    CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC,
    SRCCOPY, SelectObject,
};
use windows::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX2};
use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
use windows::Win32::UI::HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext};
use windows::Win32::UI::WindowsAndMessaging::{
    GA_ROOT, GWL_EXSTYLE, GetAncestor, GetCursorPos, GetWindowLongPtrW, LWA_ALPHA, SPI_GETWORKAREA,
    SW_HIDE, SW_SHOWNOACTIVATE, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetCursorPos,
    SetLayeredWindowAttributes, SetWindowLongPtrW, ShowWindow, SystemParametersInfoW,
    WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WindowFromPoint,
};

use crate::hit_mask::{AlphaMask, Placement, hits};

/// State shared between the Bevy app and the Win32 worker threads.
#[derive(Default)]
pub struct Shared {
    pub hwnd: AtomicIsize,
    pub placement: Mutex<Placement>,
    pub frame: AtomicUsize,
    pub dragging: AtomicBool,
    /// Pauses the pet's walking (used by the self-test so the target holds still).
    pub frozen: AtomicBool,
    pub frames_rendered: AtomicU64,
    pub style_toggles: AtomicU64,
    /// Times something other than us (winit) changed our non-hit-test style bits back.
    pub style_heals: AtomicU64,
    pub report: Mutex<Vec<String>>,
}

impl Shared {
    fn hwnd(&self) -> Option<HWND> {
        match self.hwnd.load(Ordering::Acquire) {
            0 => None,
            h => Some(HWND(h as *mut c_void)),
        }
    }

    fn note(&self, line: impl Into<String>) {
        let line = line.into();
        println!("[spike] {line}");
        self.report.lock().unwrap().push(line);
    }
}

pub fn make_dpi_aware() {
    // Must run before winit creates the window; winit's own later call then fails harmlessly.
    let _ = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
}

/// Primary monitor work area (excludes the taskbar), physical pixels.
pub fn primary_work_area() -> RECT {
    let mut rect = RECT::default();
    unsafe {
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut rect as *mut RECT as *mut c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .expect("SPI_GETWORKAREA");
    }
    rect
}

fn ex_style(hwnd: HWND) -> u32 {
    unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 }
}

/// Ex-style we want: layered, tool window (no taskbar / Alt-Tab), never activates on click,
/// topmost, and hit-test-transparent unless the cursor is on the pet.
fn wanted_style(current: u32, cursor_on_pet: bool) -> u32 {
    let mut want = (current | WS_EX_LAYERED.0 | WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0 | WS_EX_TOPMOST.0)
        & !WS_EX_APPWINDOW.0;
    if cursor_on_pet {
        want &= !WS_EX_TRANSPARENT.0;
    } else {
        want |= WS_EX_TRANSPARENT.0;
    }
    want
}

fn cursor_on_pet(shared: &Shared, masks: &[AlphaMask], cursor: (i32, i32)) -> bool {
    if shared.dragging.load(Ordering::Acquire) {
        return true;
    }
    let placement = *shared.placement.lock().unwrap();
    let frame = shared.frame.load(Ordering::Acquire) % masks.len();
    hits(&masks[frame], &placement, cursor)
}

/// Polls the global cursor independently of the render frame cap and flips WS_EX_TRANSPARENT.
pub fn spawn_hit_thread(shared: Arc<Shared>, masks: Arc<Vec<AlphaMask>>, poll: Duration, layered_attrs: bool) {
    thread::Builder::new()
        .name("hit-test".into())
        .spawn(move || {
            let mut attrs_done = false;
            loop {
                thread::sleep(poll);
                let Some(hwnd) = shared.hwnd() else { continue };
                if layered_attrs && !attrs_done {
                    let _ = unsafe { SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA) };
                    attrs_done = true;
                }
                let mut pt = POINT::default();
                if unsafe { GetCursorPos(&mut pt) }.is_err() {
                    continue;
                }
                let on_pet = cursor_on_pet(&shared, &masks, (pt.x, pt.y));
                let current = ex_style(hwnd);
                let want = wanted_style(current, on_pet);
                if want != current {
                    if (current ^ want) & !WS_EX_TRANSPARENT.0 != 0 {
                        shared.style_heals.fetch_add(1, Ordering::Relaxed);
                    }
                    shared.style_toggles.fetch_add(1, Ordering::Relaxed);
                    unsafe { SetWindowLongPtrW(hwnd, GWL_EXSTYLE, want as isize) };
                }
            }
        })
        .expect("spawn hit-test thread");
}

// ---------- metrics ----------

fn filetime_100ns(ft: FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | ft.dwLowDateTime as u64
}

fn process_cpu_100ns() -> u64 {
    let (mut c, mut e, mut k, mut u) = Default::default();
    unsafe { GetProcessTimes(GetCurrentProcess(), &mut c, &mut e, &mut k, &mut u) }.expect("GetProcessTimes");
    filetime_100ns(k) + filetime_100ns(u)
}

/// (working set, private bytes, private working set) in bytes. Task Manager's "Memory" column is
/// the private working set.
fn process_memory() -> (u64, u64, u64) {
    let mut pmc = PROCESS_MEMORY_COUNTERS_EX2::default();
    let cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX2>() as u32;
    unsafe {
        let _ = K32GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut pmc as *mut PROCESS_MEMORY_COUNTERS_EX2 as *mut PROCESS_MEMORY_COUNTERS,
            cb,
        );
    }
    (pmc.WorkingSetSize as u64, pmc.PrivateUsage as u64, pmc.PrivateWorkingSetSize as u64)
}

#[derive(Default)]
struct Agg {
    n: u64,
    cpu_sum: f64,
    cpu_max: f64,
    fps_sum: f64,
    ws_max: u64,
    private_max: u64,
    private_ws_max: u64,
}

/// Samples CPU / RAM / fps, runs periodic transparency checks, and exits after `duration`.
pub fn spawn_metrics_thread(shared: Arc<Shared>, out_dir: PathBuf, duration: Duration, sample: Duration, check_every: Duration) {
    thread::Builder::new()
        .name("metrics".into())
        .spawn(move || {
            fs::create_dir_all(&out_dir).expect("create results dir");
            let mut csv = File::create(out_dir.join("metrics.csv")).expect("metrics.csv");
            writeln!(csv, "elapsed_s,cpu_pct_machine,cpu_pct_one_core,working_set_mb,private_mb,private_ws_mb,fps,style_toggles,style_heals").unwrap();
            let cores = thread::available_parallelism().map(|n| n.get()).unwrap_or(1) as f64;

            let start = Instant::now();
            let mut last_t = start;
            let mut last_cpu = process_cpu_100ns();
            let mut last_frames = shared.frames_rendered.load(Ordering::Relaxed);
            let mut last_check = start;
            let mut checks_ok = 0u32;
            let mut checks_bad = 0u32;
            let mut agg = Agg::default();
            // Startup (shader compile, pipeline creation) is not steady state.
            let warmup = Duration::from_secs(20);

            while start.elapsed() < duration {
                thread::sleep(sample);
                let now = Instant::now();
                let cpu = process_cpu_100ns();
                let frames = shared.frames_rendered.load(Ordering::Relaxed);
                let wall_100ns = (now - last_t).as_nanos() as f64 / 100.0;
                let one_core = (cpu - last_cpu) as f64 / wall_100ns * 100.0;
                let machine = one_core / cores;
                let fps = (frames - last_frames) as f64 / (now - last_t).as_secs_f64();
                let (ws, private, private_ws) = process_memory();
                let mb = |b: u64| b as f64 / (1024.0 * 1024.0);
                writeln!(
                    csv,
                    "{:.0},{:.3},{:.2},{:.1},{:.1},{:.1},{:.1},{},{}",
                    start.elapsed().as_secs_f64(),
                    machine,
                    one_core,
                    mb(ws),
                    mb(private),
                    mb(private_ws),
                    fps,
                    shared.style_toggles.load(Ordering::Relaxed),
                    shared.style_heals.load(Ordering::Relaxed)
                )
                .unwrap();
                csv.flush().unwrap();
                if start.elapsed() > warmup {
                    agg.n += 1;
                    agg.cpu_sum += machine;
                    agg.cpu_max = agg.cpu_max.max(machine);
                    agg.fps_sum += fps;
                }
                agg.ws_max = agg.ws_max.max(ws);
                agg.private_max = agg.private_max.max(private);
                agg.private_ws_max = agg.private_ws_max.max(private_ws);
                last_t = now;
                last_cpu = cpu;
                last_frames = frames;

                if now - last_check >= check_every {
                    last_check = now;
                    match transparency_check(&shared, &out_dir, false) {
                        Some(true) => checks_ok += 1,
                        Some(false) => checks_bad += 1,
                        None => {}
                    }
                }
            }

            let final_ok = transparency_check(&shared, &out_dir, true);
            match final_ok {
                Some(true) => checks_ok += 1,
                Some(false) => checks_bad += 1,
                None => {}
            }
            let mb = |b: u64| b as f64 / (1024.0 * 1024.0);
            let n = agg.n.max(1) as f64;
            let mut summary = File::create(out_dir.join("summary.md")).unwrap();
            writeln!(summary, "# Spike run summary\n").unwrap();
            writeln!(summary, "- Duration: {:.1} min", start.elapsed().as_secs_f64() / 60.0).unwrap();
            writeln!(summary, "- CPU (whole machine, {cores} logical cores, after {}s warm-up): avg {:.3}% / max {:.3}%", warmup.as_secs(), agg.cpu_sum / n, agg.cpu_max).unwrap();
            writeln!(summary, "- Avg fps: {:.1}", agg.fps_sum / n).unwrap();
            writeln!(summary, "- Max working set: {:.1} MB", mb(agg.ws_max)).unwrap();
            writeln!(summary, "- Max private bytes (commit): {:.1} MB", mb(agg.private_max)).unwrap();
            writeln!(summary, "- Max private working set (Task Manager \"Memory\"): {:.1} MB", mb(agg.private_ws_max)).unwrap();
            writeln!(summary, "- Hit-test style toggles: {}", shared.style_toggles.load(Ordering::Relaxed)).unwrap();
            writeln!(summary, "- Style heals (someone else reset our ex-style): {}", shared.style_heals.load(Ordering::Relaxed)).unwrap();
            writeln!(summary, "- Transparency checks: {checks_ok} ok / {checks_bad} failed").unwrap();
            writeln!(summary, "\n## Notes\n").unwrap();
            for line in shared.report.lock().unwrap().iter() {
                writeln!(summary, "- {line}").unwrap();
            }
            println!("[spike] done — see {}", out_dir.join("summary.md").display());
            std::process::exit(0);
        })
        .expect("spawn metrics thread");
}

// ---------- screen capture ----------

/// Captures a screen rectangle (physical px) as RGBA8.
pub fn capture(left: i32, top: i32, width: i32, height: i32) -> Vec<u8> {
    unsafe {
        let screen = GetDC(None);
        let mem = CreateCompatibleDC(Some(screen));
        let bmp = CreateCompatibleBitmap(screen, width, height);
        let old = SelectObject(mem, bmp.into());
        let _ = BitBlt(mem, 0, 0, width, height, Some(screen), left, top, SRCCOPY | CAPTUREBLT);
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bgra = vec![0u8; (width * height * 4) as usize];
        GetDIBits(mem, bmp, 0, height as u32, Some(bgra.as_mut_ptr() as *mut c_void), &mut info, DIB_RGB_COLORS);
        SelectObject(mem, old);
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);
        for px in bgra.chunks_exact_mut(4) {
            px.swap(0, 2);
            px[3] = 255;
        }
        bgra
    }
}

pub fn save_png(path: &Path, width: i32, height: i32, rgba: &[u8]) {
    let _ = image::save_buffer(path, rgba, width as u32, height as u32, image::ExtendedColorType::Rgba8);
}

fn black_fraction(rgba: &[u8]) -> f64 {
    let px = rgba.len() / 4;
    let black = rgba.chunks_exact(4).filter(|p| p[0] < 8 && p[1] < 8 && p[2] < 8).count();
    black as f64 / px.max(1) as f64
}

fn similarity(a: &[u8], b: &[u8]) -> f64 {
    let px = a.len() / 4;
    let same = a
        .chunks_exact(4)
        .zip(b.chunks_exact(4))
        .filter(|(p, q)| (0..3).all(|i| p[i].abs_diff(q[i]) <= 8))
        .count();
    same as f64 / px.max(1) as f64
}

/// Compares a patch of the overlay (away from the pet) with the window hidden vs shown.
/// A transparent overlay looks identical; the NVIDIA bug shows up as a black patch.
/// Also saves a screenshot around the pet so a human can eyeball it.
fn transparency_check(shared: &Shared, out_dir: &Path, save_shots: bool) -> Option<bool> {
    let hwnd = shared.hwnd()?;
    let work = primary_work_area();
    let p = *shared.placement.lock().unwrap();
    let sprite_w = 24 * p.scale as i32;
    // Patch: 200x120 in the window, on the opposite half of the screen from the pet.
    let mid = (work.left + work.right) / 2;
    let patch_left = if p.left > mid { work.left + 100 } else { work.right - 300 };
    let patch_top = work.bottom - 160;
    let (pw, ph) = (200, 120);

    unsafe {
        let _ = ShowWindow(hwnd, SW_HIDE);
    }
    thread::sleep(Duration::from_millis(250));
    let hidden = capture(patch_left, patch_top, pw, ph);
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
    thread::sleep(Duration::from_millis(250));
    let shown = capture(patch_left, patch_top, pw, ph);

    let sim = similarity(&hidden, &shown);
    let black_hidden = black_fraction(&hidden);
    let black_shown = black_fraction(&shown);
    // Fail on the bug signature: the overlay turns the patch black where the desktop isn't.
    let ok = !(black_shown > 0.9 && black_hidden < 0.5) && sim > 0.6;
    shared.note(format!(
        "transparency check @ patch ({patch_left},{patch_top}): similarity {:.3}, black hidden {:.2} / shown {:.2} → {}",
        sim,
        black_hidden,
        black_shown,
        if ok { "OK" } else { "FAIL" }
    ));

    if save_shots {
        let (sw, sh) = (sprite_w + 400, 400);
        let left = (p.left - 200).max(work.left);
        let top = work.bottom - sh;
        let around_pet = capture(left, top, sw, sh);
        save_png(&out_dir.join("shot_pet.png"), sw, sh, &around_pet);
        save_png(&out_dir.join("shot_patch_hidden.png"), pw, ph, &hidden);
        save_png(&out_dir.join("shot_patch_shown.png"), pw, ph, &shown);
    }
    Some(ok)
}

/// Startup screenshot of the area around the pet (proves it renders and that the background is see-through).
pub fn spawn_startup_shot(shared: Arc<Shared>, out_dir: PathBuf, after: Duration) {
    thread::spawn(move || {
        thread::sleep(after);
        let _ = fs::create_dir_all(&out_dir);
        if transparency_check(&shared, &out_dir, true).is_some() {
            let _ = fs::rename(out_dir.join("shot_pet.png"), out_dir.join("shot_pet_startup.png"));
        }
        report_styles(&shared);
    });
}

fn report_styles(shared: &Shared) {
    let Some(hwnd) = shared.hwnd() else { return };
    let ex = ex_style(hwnd);
    let has = |flag: u32| ex & flag != 0;
    shared.note(format!(
        "ex-style: TOOLWINDOW={} APPWINDOW={} NOACTIVATE={} TOPMOST={} LAYERED={} (TOOLWINDOW && !APPWINDOW ⇒ hidden from taskbar and Alt-Tab)",
        has(WS_EX_TOOLWINDOW.0),
        has(WS_EX_APPWINDOW.0),
        has(WS_EX_NOACTIVATE.0),
        has(WS_EX_TOPMOST.0),
        has(WS_EX_LAYERED.0)
    ));
}

// ---------- self-test ----------

/// Sweeps the real cursor across the pet and asks Windows which window is under each point.
/// Expected: our window exactly on opaque pixels, something else everywhere else (holes included).
pub fn spawn_cursor_selftest(shared: Arc<Shared>, masks: Arc<Vec<AlphaMask>>, after: Duration, settle: Duration) {
    thread::spawn(move || {
        thread::sleep(after);
        let Some(hwnd) = shared.hwnd() else {
            shared.note("self-test skipped: no window handle");
            return;
        };
        shared.frozen.store(true, Ordering::Release);
        thread::sleep(Duration::from_millis(200));
        let mut original = POINT::default();
        let _ = unsafe { GetCursorPos(&mut original) };

        let p = *shared.placement.lock().unwrap();
        let mask = &masks[shared.frame.load(Ordering::Acquire) % masks.len()];
        let s = p.scale as i32;
        let (w, h) = (mask.width() as i32 * s, mask.height() as i32 * s);
        let (mut checked, mut wrong) = (0u32, 0u32);
        let mut failures = Vec::new();
        // Rows through the eyes, the belly/tail gap and the legs (hole between them).
        for &sprite_row in &[5, 8, 12, 16] {
            let y = p.top + sprite_row * s + s / 2;
            for x in (p.left - 2 * s)..(p.left + w + 2 * s) {
                if x % 2 != 0 {
                    continue;
                }
                let _ = unsafe { SetCursorPos(x, y) };
                thread::sleep(settle);
                let expected = cursor_on_pet(&shared, &masks, (x, y));
                let under = unsafe { GetAncestor(WindowFromPoint(POINT { x, y }), GA_ROOT) };
                let actual = under == hwnd;
                checked += 1;
                if expected != actual {
                    wrong += 1;
                    if failures.len() < 10 {
                        failures.push(format!("({x},{y}) expected {expected} got {actual}"));
                    }
                }
            }
        }
        let _ = unsafe { SetCursorPos(original.x, original.y) };
        shared.frozen.store(false, Ordering::Release);
        shared.note(format!(
            "cursor self-test: {checked} points across sprite rows 5/8/12/16 (incl. holes) at {s}x, settle {}ms → {wrong} misrouted{}",
            settle.as_millis(),
            if failures.is_empty() { String::new() } else { format!(": {}", failures.join("; ")) }
        ));
        let _ = h;
    });
}
