//! What the hit-test thread, the Overlay and the engine share about the pointer: the samples the hit-test thread has
//! read, whether the owner is holding the Pet, and each time the Overlay lost the mouse.

use std::mem;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use pet_core::geometry::Point;

use crate::PointerSample;
use crate::pointer::PointerBuffer;

/// Shared between the engine, the hit-test thread and the Overlay's window. No lock is ever held while calling into
/// Windows.
#[derive(Default)]
pub struct Pointer {
    samples: Mutex<PointerBuffer>,
    /// A plain flag the hit-test thread reads on its next poll, so holding needs no thread or loop of its own.
    holding: AtomicBool,
    /// When the Overlay lost the mouse, and where the cursor was, oldest first, until the engine asks.
    lost: Mutex<Vec<(Instant, Point)>>,
}

impl Pointer {
    pub fn record(&self, sample: PointerSample) {
        self.samples.lock().unwrap().push(sample);
    }

    pub fn take_samples(&self) -> Vec<PointerSample> {
        self.samples.lock().unwrap().drain()
    }

    pub fn set_holding(&self, holding: bool) {
        self.holding.store(holding, Ordering::Relaxed);
    }

    /// The Overlay lost the mouse at `at`, with the cursor at `position`. It happens on every release, when the
    /// Overlay lets the mouse go, and also when Windows takes it away mid-drag, for example for Alt-Tab, the Start
    /// menu or the lock screen, when no release will ever come. Either way the hold is over, so clicks pass through
    /// again at once, before the engine has even heard.
    pub fn mouse_lost(&self, at: Instant, position: Point) {
        self.holding.store(false, Ordering::Relaxed);
        self.lost.lock().unwrap().push((at, position));
    }

    /// Every time the Overlay lost the mouse since the last call, oldest first.
    pub fn take_mouse_lost(&self) -> Vec<(Instant, Point)> {
        mem::take(&mut *self.lost.lock().unwrap())
    }

    /// Whether the Overlay should catch clicks rather than let them through to the window below. While the owner holds
    /// the Pet it does, so the Pet trailing behind a fast drag can't let the mouse fall through to the desktop.
    pub fn catches_clicks(&self, over_pet: bool) -> bool {
        over_pet || self.holding.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn given_the_owner_is_holding_the_pet_when_the_cursor_is_off_it_then_the_overlay_still_catches_clicks()
     {
        // Given
        let pointer = Pointer::default();
        pointer.set_holding(true);

        // When
        let catches = pointer.catches_clicks(false);

        // Then
        assert!(catches);
    }

    #[test]
    fn given_the_owner_is_holding_the_pet_when_the_overlay_loses_the_mouse_then_clicks_pass_through_at_once()
     {
        // Given: Windows takes the mouse away, for example for Alt-Tab during a drag, so no release will come
        let pointer = Pointer::default();
        pointer.set_holding(true);

        // When
        pointer.mouse_lost(Instant::now(), Point { x: 10, y: 20 });

        // Then: clicks pass through before the engine has even heard
        assert!(!pointer.catches_clicks(false));
    }

    #[test]
    fn given_the_mouse_was_lost_twice_when_the_engine_asks_then_it_hears_each_once_in_order() {
        // Given
        let pointer = Pointer::default();
        let start = Instant::now();
        let first = (start, Point { x: 1, y: 1 });
        let second = (start + Duration::from_millis(5), Point { x: 2, y: 2 });
        pointer.mouse_lost(first.0, first.1);
        pointer.mouse_lost(second.0, second.1);

        // When
        let heard = pointer.take_mouse_lost();
        let again = pointer.take_mouse_lost();

        // Then
        assert_eq!(heard, [first, second]);
        assert!(again.is_empty());
    }

    #[test]
    fn given_the_owner_let_go_when_the_cursor_is_off_the_pet_then_clicks_pass_through() {
        // Given
        let pointer = Pointer::default();
        pointer.set_holding(true);
        pointer.set_holding(false);

        // When
        let catches = pointer.catches_clicks(false);

        // Then
        assert!(!catches);
    }

    #[test]
    fn given_nothing_is_held_when_the_cursor_is_on_the_pet_then_the_overlay_catches_clicks() {
        // Given
        let pointer = Pointer::default();

        // When
        let catches = pointer.catches_clicks(true);

        // Then
        assert!(catches);
    }
}
