//! The real app, plus a system that exits with the render-failure code. A Relaunch starts another copy of this program,
//! so the limit can be tested with no help from the real app. This tests our path, not a real GPU failure.
//!
//! `FAILING_APP_WHEN` picks when it fails: `shown` soon after the Overlay appears, or `hidden` once it's been shown and
//! then hidden. Each copy adds lines to the file named by `FAILING_APP_RECORD`.

use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::process;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use pet_app::RENDER_FAILURE_EXIT;
use windows::Win32::UI::WindowsAndMessaging::{FindWindowExW, GetWindowThreadProcessId};
use windows::core::w;

/// Long enough for the tray icon to be up, so the check at the end means something.
const SECONDS_SHOWN_BEFORE_FAILING: f32 = 0.5;

fn record(line: &str) {
    let path = env::var_os("FAILING_APP_RECORD").expect("the test names a record file");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("open the record file");
    writeln!(file, "{line}").expect("write to the record file");
}

fn fail() -> AppExit {
    AppExit::from_code(RENDER_FAILURE_EXIT)
}

fn fail_when_shown(
    time: Res<Time>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut seconds_shown: Local<f32>,
    mut exit: MessageWriter<AppExit>,
) {
    if windows.single().is_ok_and(|window| window.visible) {
        *seconds_shown += time.delta_secs();
    }
    if *seconds_shown > SECONDS_SHOWN_BEFORE_FAILING {
        exit.write(fail());
    }
}

fn fail_when_hidden(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut was_shown: Local<bool>,
    mut exit: MessageWriter<AppExit>,
) {
    let visible = windows.single().is_ok_and(|window| window.visible);
    if *was_shown && !visible {
        exit.write(fail());
    }
    *was_shown |= visible;
}

/// Does this process still have the window `tray-icon` makes? Dropping `Platform` stops the shell thread, which removes
/// it, so it must be gone by the time `pet_app::run` returns.
fn tray_window_remains() -> bool {
    let mut after = None;
    // SAFETY: the class name is a static string and `after` is a window from the previous call.
    while let Ok(window) = unsafe { FindWindowExW(None, after, w!("tray_icon_app"), None) } {
        let mut pid = 0;
        // SAFETY: `pid` is a live local that Windows writes during the call.
        unsafe { GetWindowThreadProcessId(window, Some(&mut pid)) };
        if pid == process::id() {
            return true;
        }
        after = Some(window);
    }
    false
}

fn main() -> AppExit {
    let exit = pet_app::run(|app| {
        record("started");
        // After Update, because a Hidden engine runs nothing once that update has finished.
        match env::var("FAILING_APP_WHEN").as_deref() {
            Ok("hidden") => app.add_systems(PostUpdate, fail_when_hidden),
            _ => app.add_systems(PostUpdate, fail_when_shown),
        };
    });
    record(if tray_window_remains() {
        "stopped with the tray window still there"
    } else {
        "stopped"
    });
    exit
}
