//! Polls the cursor and flips click-through, separately from the render cap so clicks at the Pet's edge are
//! never misrouted (REN-05). Each reading is also kept as a `PointerSample` for the engine.

use std::ffi::c_void;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use pet_core::geometry::Point;
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

use super::pointer::Pointer;
use super::window_styles;
use crate::{HitTarget, PointerSample};

const POLL: Duration = Duration::from_millis(8);

/// Current cursor position, or `None` when it can't be read, for example on the lock screen.
pub(super) fn cursor_position() -> Option<Point> {
    let mut point = POINT::default();
    // SAFETY: `point` is a live local, and Windows writes to it only during the call.
    unsafe { GetCursorPos(&mut point) }.ok()?;
    Some(Point {
        x: point.x,
        y: point.y,
    })
}

/// Lets the hit-test thread sleep while the Pet is Hidden. It waits to be told, so nothing polls.
#[derive(Default)]
pub struct HitTestPause {
    hidden: Mutex<bool>,
    changed: Condvar,
}

impl HitTestPause {
    pub fn set_hidden(&self, hidden: bool) {
        *self.hidden.lock().unwrap() = hidden;
        self.changed.notify_all();
    }

    fn wait_while_hidden(&self) {
        let hidden = self.hidden.lock().unwrap();
        drop(self.changed.wait_while(hidden, |hidden| *hidden).unwrap());
    }
}

pub fn spawn(
    overlay: isize,
    target: Arc<Mutex<Option<HitTarget>>>,
    pause: Arc<HitTestPause>,
    pointer: Arc<Pointer>,
) {
    thread::Builder::new()
        .name("hit-test".into())
        .spawn(move || {
            let hwnd = HWND(overlay as *mut c_void);
            loop {
                pause.wait_while_hidden();
                thread::sleep(POLL);
                let Some(cursor) = cursor_position() else {
                    continue;
                };
                let at = Instant::now();
                let over_pet = target
                    .lock()
                    .unwrap()
                    .as_ref()
                    .is_some_and(|t| t.contains(cursor));
                pointer.record(PointerSample {
                    at,
                    position: cursor,
                    over_pet,
                });
                // Both locks are released by now, and they have to be: `apply` can wait on the main thread, which
                // can be waiting on a system that's publishing a new target or taking the samples.
                window_styles::apply(hwnd, pointer.catches_clicks(over_pet));
            }
        })
        .expect("the OS can start a thread");
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    const LONG_ENOUGH_TO_NOTICE: Duration = Duration::from_millis(50);

    /// A thread that reports each time it gets past the pause.
    fn waiter(pause: Arc<HitTestPause>) -> mpsc::Receiver<()> {
        let (passed, passes) = mpsc::channel();
        thread::spawn(move || {
            loop {
                pause.wait_while_hidden();
                if passed.send(()).is_err() {
                    return;
                }
                thread::sleep(POLL);
            }
        });
        passes
    }

    #[test]
    fn given_the_pet_is_shown_when_waiting_then_the_thread_carries_on() {
        // Given
        let pause = Arc::new(HitTestPause::default());

        // When
        let passes = waiter(pause);

        // Then
        assert!(passes.recv_timeout(LONG_ENOUGH_TO_NOTICE).is_ok());
    }

    #[test]
    fn given_the_pet_is_hidden_when_waiting_then_the_thread_stays_asleep_until_it_is_shown() {
        // Given
        let pause = Arc::new(HitTestPause::default());
        pause.set_hidden(true);

        // When
        let passes = waiter(pause.clone());

        // Then
        assert!(passes.recv_timeout(LONG_ENOUGH_TO_NOTICE).is_err());

        // When
        pause.set_hidden(false);

        // Then
        assert!(passes.recv_timeout(LONG_ENOUGH_TO_NOTICE).is_ok());
    }
}
