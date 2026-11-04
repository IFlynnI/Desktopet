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
struct FullscreenWatch {
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

/// When to look at the screen next. The platform runs the timers, and this decides what they do:
/// - Something that may have changed starts the wait before a check, or starts it again, so a burst causes one check.
/// - The regular re-check starts a wait only when none is running, or a return delay longer than its period would
///   never end.
#[derive(Clone, Copy, Debug)]
pub struct FullscreenChecks {
    watch: FullscreenWatch,
    waiting: bool,
}

impl FullscreenChecks {
    pub fn new(return_delay: Duration) -> Self {
        Self {
            watch: FullscreenWatch::new(return_delay),
            waiting: false,
        }
    }

    /// How long to wait before checking, counted from now, replacing any wait that's running.
    pub fn something_changed(&mut self) -> Duration {
        self.waiting = true;
        self.watch.delay()
    }

    /// How long to wait before checking, or `None` to let the wait that's running end as planned.
    pub fn re_check_due(&mut self) -> Option<Duration> {
        (!self.waiting).then(|| self.something_changed())
    }

    /// The wait is over and a check begins. Call this before reading the screen, so a change noticed during the read
    /// keeps the wait it starts.
    pub fn wait_ended(&mut self) {
        self.waiting = false;
    }

    /// Takes the snapshot read when a wait ended, or right at the start. Returns the new answer, if it changed.
    pub fn check(&mut self, snapshot: &Snapshot) -> Option<bool> {
        self.watch.update(snapshot)
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

    fn checking_a_fullscreen_app(return_delay: Duration) -> FullscreenChecks {
        let mut checks = FullscreenChecks::new(return_delay);
        checks.check(&fullscreen_app());
        checks
    }

    #[test]
    fn given_a_fullscreen_app_when_something_may_have_changed_then_the_wait_is_the_return_delay() {
        // Given
        let return_delay = Duration::from_secs(1);
        let mut checks = checking_a_fullscreen_app(return_delay);

        // When
        let wait = checks.something_changed();

        // Then
        assert_eq!(wait, return_delay);
    }

    #[test]
    fn given_a_wait_running_when_something_else_changes_then_the_wait_starts_again_in_full() {
        // Given: the app left, and the wait for the return delay began
        let return_delay = Duration::from_secs(1);
        let mut checks = checking_a_fullscreen_app(return_delay);
        checks.something_changed();

        // When: another window comes to the front before the wait ends
        let wait = checks.something_changed();

        // Then: one check after things settle, not one per change
        assert_eq!(wait, return_delay);
    }

    #[test]
    fn given_nothing_waiting_when_the_re_check_comes_round_then_a_wait_starts() {
        // Given
        let mut checks = FullscreenChecks::new(RETURN_DELAY);

        // When
        let wait = checks.re_check_due();

        // Then
        assert_eq!(wait, Some(HIDE_DELAY));
    }

    #[test]
    fn given_a_wait_running_when_the_re_check_comes_round_then_the_wait_is_left_alone() {
        // Given: a return delay longer than the re-check would never end if each re-check restarted it
        let mut checks = checking_a_fullscreen_app(Duration::from_secs(10));
        checks.something_changed();

        // When
        let wait = checks.re_check_due();

        // Then
        assert_eq!(wait, None);
    }

    #[test]
    fn given_a_wait_that_ended_when_the_re_check_comes_round_then_a_wait_starts_again() {
        // Given
        let mut checks = checking_a_fullscreen_app(RETURN_DELAY);
        checks.something_changed();
        checks.wait_ended();
        checks.check(&fullscreen_app());

        // When
        let wait = checks.re_check_due();

        // Then
        assert_eq!(wait, Some(RETURN_DELAY));
    }

    #[test]
    fn given_a_wait_that_ended_on_a_snapshot_that_can_not_tell_when_the_re_check_comes_round_then_a_wait_starts()
     {
        // Given: the foreground was empty when the wait ended
        let mut checks = checking_a_fullscreen_app(RETURN_DELAY);
        checks.something_changed();
        checks.wait_ended();
        checks.check(&Snapshot {
            foreground: None,
            ..fullscreen_app()
        });

        // When
        let wait = checks.re_check_due();

        // Then: the re-check still gets to look again
        assert_eq!(wait, Some(RETURN_DELAY));
    }

    #[test]
    fn given_the_re_check_just_started_a_wait_when_it_comes_round_again_then_that_wait_is_left_alone()
     {
        // Given
        let mut checks = checking_a_fullscreen_app(Duration::from_secs(10));
        checks.re_check_due();

        // When
        let wait = checks.re_check_due();

        // Then
        assert_eq!(wait, None);
    }

    #[test]
    fn given_a_wait_that_ended_when_something_changes_during_the_read_then_the_new_wait_is_kept() {
        // Given: the wait ended, and the screen is being read
        let mut checks = checking_a_fullscreen_app(Duration::from_secs(10));
        checks.something_changed();
        checks.wait_ended();

        // When: a change is noticed before the snapshot is taken in
        checks.something_changed();
        checks.check(&fullscreen_app());

        // Then: the re-check leaves that new wait running instead of restarting it
        assert_eq!(checks.re_check_due(), None);
    }

    #[test]
    fn given_a_wait_for_the_return_delay_when_it_ends_with_the_app_gone_then_the_change_is_reported()
     {
        // Given
        let mut checks = checking_a_fullscreen_app(RETURN_DELAY);
        checks.something_changed();
        checks.wait_ended();

        // When
        let change = checks.check(&windowed_app());

        // Then
        assert_eq!(change, Some(false));
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
