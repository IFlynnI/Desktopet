//! Whether a fullscreen application is in the foreground, decided from plain facts the platform reads for us.

use std::time::Duration;

use crate::geometry::Rect;

/// How long the Pet waits before hiding, so a window has reached its final size by the time it's measured.
pub const HIDE_DELAY: Duration = Duration::from_millis(100);

/// How long the Pet waits before coming back. It's separate from `HIDE_DELAY`, so a quick Alt-Tab can be made not to
/// show the Pet without making it slower to hide. Settings replace this value in roadmap item 6.
pub const RETURN_DELAY: Duration = Duration::from_millis(100);

/// The window the owner is working in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ForegroundWindow {
    /// The part the app draws itself, without a title bar or borders.
    pub client_area: Rect,
    /// The desktop covers the monitor and can take the foreground, but it isn't a fullscreen app.
    pub is_desktop: bool,
    /// The window has told the OS it's never fullscreen, as the Overlay does.
    pub declares_not_fullscreen: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    /// `None` when there's no foreground window, which happens during a change, or it couldn't be read.
    pub foreground: Option<ForegroundWindow>,
    pub home_monitor: Rect,
    /// The OS reports a game that has taken over the display directly.
    pub exclusive_fullscreen: bool,
}

impl Snapshot {
    /// `None` when this snapshot can't tell, so the answer stays as it was.
    fn is_fullscreen(&self) -> Option<bool> {
        if self.exclusive_fullscreen {
            return Some(true);
        }
        let window = self.foreground?;
        Some(
            !window.is_desktop
                && !window.declares_not_fullscreen
                && window.client_area.covers(self.home_monitor),
        )
    }
}

/// Remembers the last answer, so the engine hears only about changes.
#[derive(Clone, Copy, Debug)]
pub struct FullscreenWatch {
    fullscreen: bool,
    return_delay: Duration,
}

impl FullscreenWatch {
    pub fn new(return_delay: Duration) -> Self {
        Self {
            fullscreen: false,
            return_delay,
        }
    }

    /// How long to wait after something may have changed before taking a snapshot.
    pub fn delay(&self) -> Duration {
        if self.fullscreen {
            self.return_delay
        } else {
            HIDE_DELAY
        }
    }

    /// The new answer, if it differs from the last one.
    pub fn update(&mut self, snapshot: &Snapshot) -> Option<bool> {
        let answer = snapshot.is_fullscreen()?;
        let changed = answer != self.fullscreen;
        self.fullscreen = answer;
        changed.then_some(answer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MONITOR: Rect = Rect {
        left: 0,
        top: 0,
        right: 3840,
        bottom: 2160,
    };

    fn app_covering(client_area: Rect) -> Snapshot {
        Snapshot {
            foreground: Some(ForegroundWindow {
                client_area,
                ..Default::default()
            }),
            home_monitor: MONITOR,
            exclusive_fullscreen: false,
        }
    }

    fn fullscreen_app() -> Snapshot {
        app_covering(MONITOR)
    }

    fn windowed_app() -> Snapshot {
        app_covering(Rect {
            right: 1920,
            bottom: 1080,
            ..MONITOR
        })
    }

    fn watching_a_fullscreen_app() -> FullscreenWatch {
        let mut watch = FullscreenWatch::new(RETURN_DELAY);
        watch.update(&fullscreen_app());
        watch
    }

    #[test]
    fn given_a_borderless_app_filling_the_home_monitor_when_checked_then_it_is_fullscreen() {
        // Given
        let snapshot = fullscreen_app();

        // When
        let answer = snapshot.is_fullscreen();

        // Then
        assert_eq!(answer, Some(true));
    }

    #[test]
    fn given_an_app_larger_than_the_home_monitor_when_checked_then_it_is_fullscreen() {
        // Given: covering the monitor counts, an exact match isn't needed
        let snapshot = app_covering(Rect {
            left: -8,
            top: -8,
            right: 3848,
            bottom: 2168,
        });

        // When
        let answer = snapshot.is_fullscreen();

        // Then
        assert_eq!(answer, Some(true));
    }

    #[test]
    fn given_a_maximized_app_that_stops_at_the_taskbar_when_checked_then_it_is_not_fullscreen() {
        // Given: measured from a maximized Task Manager
        let snapshot = app_covering(Rect {
            left: 0,
            top: -11,
            right: 3840,
            bottom: 2088,
        });

        // When
        let answer = snapshot.is_fullscreen();

        // Then
        assert_eq!(answer, Some(false));
    }

    #[test]
    fn given_a_minimized_app_when_checked_then_it_is_not_fullscreen() {
        // Given: Windows parks a minimized window far off screen with an empty client area
        let snapshot = app_covering(Rect {
            left: -32000,
            top: -32000,
            right: -32000,
            bottom: -32000,
        });

        // When
        let answer = snapshot.is_fullscreen();

        // Then
        assert_eq!(answer, Some(false));
    }

    #[test]
    fn given_the_desktop_in_front_when_checked_then_it_is_not_fullscreen() {
        // Given
        let snapshot = Snapshot {
            foreground: Some(ForegroundWindow {
                client_area: MONITOR,
                is_desktop: true,
                declares_not_fullscreen: false,
            }),
            ..fullscreen_app()
        };

        // When
        let answer = snapshot.is_fullscreen();

        // Then
        assert_eq!(answer, Some(false));
    }

    #[test]
    fn given_a_window_that_declares_it_is_not_fullscreen_when_checked_then_it_is_not() {
        // Given
        let snapshot = Snapshot {
            foreground: Some(ForegroundWindow {
                client_area: MONITOR,
                is_desktop: false,
                declares_not_fullscreen: true,
            }),
            ..fullscreen_app()
        };

        // When
        let answer = snapshot.is_fullscreen();

        // Then
        assert_eq!(answer, Some(false));
    }

    #[test]
    fn given_the_os_reports_exclusive_fullscreen_when_checked_then_it_is_fullscreen_whatever_the_window_says()
     {
        // Given
        let snapshot = Snapshot {
            exclusive_fullscreen: true,
            ..windowed_app()
        };

        // When
        let answer = snapshot.is_fullscreen();

        // Then
        assert_eq!(answer, Some(true));
    }

    #[test]
    fn given_no_foreground_window_when_checked_then_it_can_not_tell() {
        // Given
        let snapshot = Snapshot {
            foreground: None,
            ..fullscreen_app()
        };

        // When
        let answer = snapshot.is_fullscreen();

        // Then
        assert_eq!(answer, None);
    }

    #[test]
    fn given_no_foreground_window_but_exclusive_fullscreen_when_checked_then_it_is_fullscreen() {
        // Given
        let snapshot = Snapshot {
            foreground: None,
            exclusive_fullscreen: true,
            ..fullscreen_app()
        };

        // When
        let answer = snapshot.is_fullscreen();

        // Then
        assert_eq!(answer, Some(true));
    }

    #[test]
    fn given_a_new_watch_when_a_fullscreen_app_is_seen_then_the_answer_is_reported() {
        // Given
        let mut watch = FullscreenWatch::new(RETURN_DELAY);

        // When
        let change = watch.update(&fullscreen_app());

        // Then
        assert_eq!(change, Some(true));
    }

    #[test]
    fn given_a_new_watch_when_a_windowed_app_is_seen_then_nothing_is_reported() {
        // Given: the Pet starts with no Auto-hide, so this changes nothing
        let mut watch = FullscreenWatch::new(RETURN_DELAY);

        // When
        let change = watch.update(&windowed_app());

        // Then
        assert_eq!(change, None);
    }

    #[test]
    fn given_a_fullscreen_app_when_it_is_seen_again_then_nothing_is_reported() {
        // Given
        let mut watch = watching_a_fullscreen_app();

        // When
        let change = watch.update(&fullscreen_app());

        // Then
        assert_eq!(change, None);
    }

    #[test]
    fn given_a_fullscreen_app_when_it_leaves_then_the_change_is_reported() {
        // Given
        let mut watch = watching_a_fullscreen_app();

        // When
        let change = watch.update(&windowed_app());

        // Then
        assert_eq!(change, Some(false));
    }

    #[test]
    fn given_a_fullscreen_app_when_the_foreground_is_briefly_empty_then_the_answer_stays() {
        // Given
        let mut watch = watching_a_fullscreen_app();
        let empty = Snapshot {
            foreground: None,
            ..fullscreen_app()
        };

        // When
        let change = watch.update(&empty);

        // Then: the Pet must not flash over the app, and the next snapshot still reports a real exit
        assert_eq!(change, None);
        assert_eq!(watch.update(&windowed_app()), Some(false));
    }

    #[test]
    fn given_nothing_fullscreen_when_asked_how_long_to_wait_then_it_is_the_hide_delay() {
        // Given
        let watch = FullscreenWatch::new(Duration::from_secs(1));

        // When
        let delay = watch.delay();

        // Then
        assert_eq!(delay, HIDE_DELAY);
    }

    #[test]
    fn given_a_fullscreen_app_when_asked_how_long_to_wait_then_it_is_the_chosen_return_delay() {
        // Given
        let return_delay = Duration::from_secs(1);
        let mut watch = FullscreenWatch::new(return_delay);
        watch.update(&fullscreen_app());

        // When
        let delay = watch.delay();

        // Then
        assert_eq!(delay, return_delay);
    }
}
