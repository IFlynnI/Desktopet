//! Waiting for a burst of changes to settle before acting on them once. An editor saves a file in several steps, such
//! as writing a temporary file, renaming it and writing again, and each step is its own change. Acting only once no
//! change has come for a while turns the whole burst into one action.
//!
//! This is a one-shot timer that each change restarts. Whoever owns it sleeps until a change comes or the time left is
//! up, so nothing polls. A tool that writes more often than the quiet time would hold it back for ever, so a burst
//! also settles once it has gone on for a longest wait, counted from its first change.

use std::time::{Duration, Instant};

/// Whether a burst of changes is waiting to settle, and until when.
#[derive(Clone, Copy, Debug)]
pub struct Settle {
    quiet_for: Duration,
    at_most: Duration,
    /// The first change of the burst waiting to settle, if there is one.
    first: Option<Instant>,
    due: Option<Instant>,
}

impl Settle {
    /// Settles once no change has come for `quiet_for`, or `at_most` after the burst's first change, whichever is
    /// sooner.
    pub fn new(quiet_for: Duration, at_most: Duration) -> Self {
        Self {
            quiet_for,
            at_most,
            first: None,
            due: None,
        }
    }

    /// Something changed at `now`, so the wait starts again from there, but never past the longest wait.
    pub fn changed(&mut self, now: Instant) {
        let first = *self.first.get_or_insert(now);
        self.due = Some((now + self.quiet_for).min(first + self.at_most));
    }

    /// How long to sleep from `now` before checking again, or `None` to sleep until the next change.
    pub fn wait(&self, now: Instant) -> Option<Duration> {
        self.due.map(|due| due.saturating_duration_since(now))
    }

    /// Whether the burst has settled by `now`. It says so once per burst.
    pub fn take_settled(&mut self, now: Instant) -> bool {
        let settled = self.due.is_some_and(|due| due <= now);
        if settled {
            self.first = None;
            self.due = None;
        }
        settled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const QUIET: Duration = Duration::from_millis(250);

    const AT_MOST: Duration = Duration::from_secs(2);

    fn settle() -> Settle {
        Settle::new(QUIET, AT_MOST)
    }

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    #[test]
    fn given_no_change_when_asking_how_long_to_wait_then_it_is_until_the_next_change() {
        // Given
        let settle = settle();

        // When
        let wait = settle.wait(Instant::now());

        // Then
        assert_eq!(wait, None);
    }

    #[test]
    fn given_no_change_when_any_time_passes_then_nothing_has_settled() {
        // Given
        let mut settle = settle();
        let start = Instant::now();

        // When
        let settled = settle.take_settled(start + ms(10_000));

        // Then
        assert!(!settled);
    }

    #[test]
    fn given_one_change_when_the_quiet_time_has_passed_then_it_has_settled_once() {
        // Given
        let mut settle = settle();
        let start = Instant::now();
        settle.changed(start);

        // When
        let before = settle.take_settled(start + ms(249));
        let at = settle.take_settled(start + QUIET);
        let after = settle.take_settled(start + ms(1_000));

        // Then
        assert_eq!((before, at, after), (false, true, false));
        assert_eq!(settle.wait(start + ms(1_000)), None);
    }

    #[test]
    fn given_a_change_when_asking_how_long_to_wait_then_it_is_the_quiet_time_left() {
        // Given
        let mut settle = settle();
        let start = Instant::now();
        settle.changed(start);

        // When
        let waits = [ms(0), ms(100), ms(250), ms(400)].map(|at| settle.wait(start + at));

        // Then: once it's due, there's no time left to wait
        assert_eq!(
            waits,
            [Some(QUIET), Some(ms(150)), Some(ms(0)), Some(ms(0))]
        );
    }

    #[test]
    fn given_a_burst_of_changes_closer_together_than_the_quiet_time_when_it_ends_then_it_settles_once_after_the_last()
     {
        // Given: an editor's save, five steps 100 ms apart
        let mut settle = settle();
        let start = Instant::now();
        let mut settled = Vec::new();

        // When: checked every 50 ms from the first change until well after the last
        for step in 0..30 {
            let now = start + ms(50 * step);
            if step % 2 == 0 && step <= 8 {
                settle.changed(now);
            }
            if settle.take_settled(now) {
                settled.push(50 * step);
            }
        }

        // Then: the last change was at 400 ms
        assert_eq!(settled, [650]);
    }

    #[test]
    fn given_two_bursts_further_apart_than_the_quiet_time_when_both_end_then_each_settles_once() {
        // Given
        let mut settle = settle();
        let start = Instant::now();

        // When
        settle.changed(start);
        let first = settle.take_settled(start + QUIET);
        settle.changed(start + ms(1_000));
        let between = settle.take_settled(start + ms(1_100));
        let second = settle.take_settled(start + ms(1_250));

        // Then
        assert_eq!((first, between, second), (true, false, true));
    }

    #[test]
    fn given_changes_closer_together_than_the_quiet_time_that_go_on_when_the_longest_wait_has_passed_then_it_settles_anyway()
     {
        // Given: a tool writing every 100 ms for 5 s
        let mut settle = settle();
        let start = Instant::now();
        let mut settled = Vec::new();

        // When: checked every 50 ms until a second after the writing stops
        for step in 0..120 {
            let now = start + ms(50 * step);
            if step % 2 == 0 && step < 100 {
                settle.changed(now);
            }
            if settle.take_settled(now) {
                settled.push(50 * step);
            }
        }

        // Then: 2 s after each burst's first change, then the quiet time after the last change at 4,900 ms
        assert_eq!(settled, [2_000, 4_100, 5_150]);
    }

    #[test]
    fn given_a_burst_near_its_longest_wait_when_another_change_comes_then_the_wait_is_only_what_is_left_of_it()
     {
        // Given
        let mut settle = settle();
        let start = Instant::now();
        settle.changed(start);

        // When
        settle.changed(start + ms(1_900));

        // Then
        assert_eq!(settle.wait(start + ms(1_900)), Some(ms(100)));
    }
}
