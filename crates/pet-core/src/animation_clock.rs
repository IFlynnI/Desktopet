//! Which Frame of an Animation shows after it has played for a while. Each Frame shows for its own duration, and the
//! next one starts the moment that duration has fully passed.
//!
//! Time is counted in whole simulation ticks, and tick `n` is exactly `n / TICKS_PER_SECOND` seconds. A tick at 30 a
//! second is a third of a nanosecond more than 33,333,333 ns, so counting it as a rounded `Duration` would drift: 3
//! ticks would come to just under 100 ms, and a 100 ms Frame would show for a tick too many.
//!
//! The Pack's validator makes every Frame last at least `SHORTEST_FRAME_MS`. A Frame at least a tick long always has
//! a tick inside it, so with exact ticks no Frame is ever skipped.

use std::time::Duration;

/// How often the simulation ticks. Every clock counts in these ticks.
pub const TICKS_PER_SECOND: u32 = 30;

/// The shortest a Frame may last, in milliseconds: one tick, rounded up to a whole millisecond, so 34 at 30 ticks a
/// second. A Frame at least a tick long always has a tick inside it, so it's always shown.
pub const SHORTEST_FRAME_MS: u32 = 1000_u32.div_ceil(TICKS_PER_SECOND);

/// What an Animation does once its last Frame has had its time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Playback {
    /// Starts again from the first Frame.
    Looping,
    /// Stays on the last Frame.
    OneShot,
}

/// The Frame showing once the Animation has played for `ticks` ticks. `durations` has one entry per Frame, and each is
/// above zero, as the Pack's validator makes sure.
pub fn frame_at(durations: &[Duration], playback: Playback, ticks: u64) -> usize {
    let played = at(ticks);
    let length = scaled(durations.iter().sum());
    let last = durations.len() - 1;
    let into_loop = match playback {
        // Whole loops are skipped in one step, so a Pet that has been up for days costs no more than a fresh one.
        Playback::Looping => played % length,
        Playback::OneShot if played >= length => return last,
        Playback::OneShot => played,
    };
    let mut ends = 0;
    for (frame, duration) in durations.iter().enumerate() {
        ends += scaled(*duration);
        if into_loop < ends {
            return frame;
        }
    }
    last
}

/// Whether a one-shot Animation is over once it has played for `ticks` ticks: its last Frame has had its whole
/// duration.
pub fn has_ended(durations: &[Duration], ticks: u64) -> bool {
    at(ticks) >= scaled(durations.iter().sum())
}

/// Exact time, in nanoseconds times `TICKS_PER_SECOND`, so that a tick is a whole number of these: 1,000,000,000.
fn scaled(duration: Duration) -> u128 {
    duration.as_nanos() * u128::from(TICKS_PER_SECOND)
}

/// The exact time `ticks` ticks in, scaled as `scaled` does.
fn at(ticks: u64) -> u128 {
    u128::from(ticks) * 1_000_000_000
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    #[test]
    fn given_the_tick_rate_when_finding_the_shortest_frame_then_it_is_one_tick_rounded_up_to_a_whole_millisecond()
     {
        // Given: a tick is 33.3 ms
        let rate = TICKS_PER_SECOND;

        // When
        let shortest = SHORTEST_FRAME_MS;

        // Then
        assert_eq!((rate, shortest), (30, 34));
    }

    #[test]
    fn given_a_looping_animation_when_less_than_the_first_duration_has_played_then_the_first_frame_shows()
     {
        // Given: 7 ticks is 233 ms
        let durations = [ms(250), ms(250)];

        // When
        let frames = [0, 7].map(|ticks| frame_at(&durations, Playback::Looping, ticks));

        // Then
        assert_eq!(frames, [0, 0]);
    }

    #[test]
    fn given_a_first_frame_a_whole_number_of_ticks_long_when_exactly_that_many_ticks_have_played_then_the_next_frame_shows()
     {
        // Given: 100 ms is exactly 3 ticks
        let durations = [ms(100), ms(200)];

        // When
        let frames = [2, 3].map(|ticks| frame_at(&durations, Playback::Looping, ticks));

        // Then
        assert_eq!(frames, [0, 1]);
    }

    #[test]
    fn given_frames_of_different_durations_when_ticks_pass_then_each_shows_for_its_own_duration() {
        // Given: 100 ms, then 300 ms, so the second Frame shows from 3 ticks to 11, 367 ms
        let durations = [ms(100), ms(300)];

        // When
        let frames = [2, 3, 11].map(|ticks| frame_at(&durations, Playback::Looping, ticks));

        // Then
        assert_eq!(frames, [0, 1, 1]);
    }

    #[test]
    fn given_a_looping_animation_when_its_last_frame_has_played_then_it_wraps_around_to_the_first()
    {
        // Given: one loop is 400 ms, exactly 12 ticks
        let durations = [ms(100), ms(300)];

        // When
        let frames = [12, 14, 15].map(|ticks| frame_at(&durations, Playback::Looping, ticks));

        // Then
        assert_eq!(frames, [0, 0, 1]);
    }

    #[test]
    fn given_a_loop_that_is_not_a_whole_number_of_ticks_when_many_loops_have_played_then_each_frame_starts_exactly_on_time()
     {
        // Given: 133 ms then 200 ms, a 333 ms loop. 1000 loops is 333 s, exactly 9990 ticks.
        let durations = [ms(133), ms(200)];

        // When: 133 ms in is just under 4 ticks, so it changes Frame at the 4th tick of the loop
        let frames = [9990, 9993, 9994].map(|ticks| frame_at(&durations, Playback::Looping, ticks));

        // Then
        assert_eq!(frames, [0, 0, 1]);
    }

    #[test]
    fn given_a_looping_animation_when_it_has_played_for_days_then_the_frame_is_still_exact() {
        // Given: three days is a whole number of 400 ms loops
        let durations = [ms(100), ms(300)];
        let three_days = 3 * 24 * 60 * 60 * u64::from(TICKS_PER_SECOND);

        // When
        let frames = [three_days + 2, three_days + 3]
            .map(|ticks| frame_at(&durations, Playback::Looping, ticks));

        // Then
        assert_eq!(frames, [0, 1]);
    }

    #[test]
    fn given_a_one_shot_animation_when_it_has_played_past_its_end_then_the_last_frame_stays() {
        // Given: 800 ms is exactly 24 ticks, and its last Frame starts at 400 ms, 12 ticks
        let durations = [ms(200), ms(200), ms(400)];

        // When
        let frames = [11, 12, 24, 1800].map(|ticks| frame_at(&durations, Playback::OneShot, ticks));

        // Then
        assert_eq!(frames, [1, 2, 2, 2]);
    }

    #[test]
    fn given_a_one_shot_animation_when_its_last_frame_has_had_its_whole_duration_then_it_has_ended()
    {
        // Given: 800 ms in all, exactly 24 ticks
        let durations = [ms(200), ms(200), ms(400)];

        // When
        let ended = [0, 23, 24, 1800].map(|ticks| has_ended(&durations, ticks));

        // Then
        assert_eq!(ended, [false, false, true, true]);
    }
}
