//! Polls the cursor and flips click-through, separately from the render cap so clicks at the Pet's edge are
//! never misrouted (REN-05).

use std::ffi::c_void;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use pet_core::geometry::Point;
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

use super::window_styles;
use crate::HitTarget;

const POLL: Duration = Duration::from_millis(8);

/// Current cursor position, or `None` when it can't be read, for example on the lock screen.
fn cursor_position() -> Option<Point> {
    let mut point = POINT::default();
    // SAFETY: `point` is a live local, and Windows writes to it only during the call.
    unsafe { GetCursorPos(&mut point) }.ok()?;
    Some(Point {
        x: point.x,
        y: point.y,
    })
}

pub fn spawn(overlay: isize, target: Arc<Mutex<Option<HitTarget>>>) {
    thread::Builder::new()
        .name("hit-test".into())
        .spawn(move || {
            let hwnd = HWND(overlay as *mut c_void);
            loop {
                thread::sleep(POLL);
                let Some(cursor) = cursor_position() else {
                    continue;
                };
                let on_pet = target
                    .lock()
                    .unwrap()
                    .as_ref()
                    .is_some_and(|t| t.contains(cursor));
                // The lock is released by now, and it has to be: `apply` can wait on the main thread, which
                // can be waiting on a system that's publishing a new target.
                window_styles::apply(hwnd, on_pet);
            }
        })
        .expect("the OS can start a thread");
}
