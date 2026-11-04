//! Which Frame of an Animation shows after it has played for a while. Each Frame shows for its own duration, and the
//! next one starts the moment that duration has fully passed.

use std::time::Duration;

/// What an Animation does once its last Frame has had its time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Playback {
    /// Starts again from the first Frame.
    Looping,
    /// Stays on the last Frame.
    OneShot,
}

/// The Frame showing once `played` has passed since the Animation started. `durations` has one entry per Frame, and
/// each is above zero, as the Pack's validator makes sure.
pub fn frame_at(durations: &[Duration], playback: Playback, played: Duration) -> usize {
    let length: Duration = durations.iter().sum();
    let last = durations.len() - 1;
    let into_loop = match playback {
        // Whole loops are skipped in one step, so a Pet that has been up for days costs no more than a fresh one.
        Playback::Looping => Duration::from_nanos((played.as_nanos() % length.as_nanos()) as u64),
        Playback::OneShot if played >= length => return last,
        Playback::OneShot => played,
    };
    let mut ends = Duration::ZERO;
    for (frame, duration) in durations.iter().enumerate() {
        ends += *duration;
        if into_loop < ends {
            return frame;
        }
    }
    last
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    #[test]
    fn given_a_looping_animation_when_less_than_the_first_duration_has_played_then_the_first_frame_shows()
     {
        // Given
        let durations = [ms(250), ms(250)];

        // When
        let frames = [ms(0), ms(249)].map(|played| frame_at(&durations, Playback::Looping, played));

        // Then
        assert_eq!(frames, [0, 0]);
    }

    #[test]
    fn given_a_looping_animation_when_the_first_duration_has_fully_played_then_the_next_frame_shows()
     {
        // Given
        let durations = [ms(250), ms(250)];

        // When
        let frame = frame_at(&durations, Playback::Looping, ms(250));

        // Then
        assert_eq!(frame, 1);
    }

    #[test]
    fn given_frames_of_different_durations_when_time_passes_then_each_shows_for_its_own_duration() {
        // Given: 100 ms, then 300 ms
        let durations = [ms(100), ms(300)];

        // When
        let frames = [ms(99), ms(100), ms(399)]
            .map(|played| frame_at(&durations, Playback::Looping, played));

        // Then
        assert_eq!(frames, [0, 1, 1]);
    }

    #[test]
    fn given_a_looping_animation_when_its_last_frame_has_played_then_it_wraps_around_to_the_first()
    {
        // Given: one loop is 400 ms
        let durations = [ms(100), ms(300)];

        // When
        let frames = [ms(400), ms(499), ms(500)]
            .map(|played| frame_at(&durations, Playback::Looping, played));

        // Then
        assert_eq!(frames, [0, 0, 1]);
    }

    #[test]
    fn given_a_looping_animation_when_it_has_played_for_days_then_the_frame_is_still_exact() {
        // Given: three days is a whole number of 400 ms loops
        let durations = [ms(100), ms(300)];
        let three_days = Duration::from_secs(3 * 24 * 60 * 60);

        // When
        let frames = [three_days + ms(99), three_days + ms(100)]
            .map(|played| frame_at(&durations, Playback::Looping, played));

        // Then
        assert_eq!(frames, [0, 1]);
    }

    #[test]
    fn given_a_one_shot_animation_when_it_has_played_past_its_end_then_the_last_frame_stays() {
        // Given
        let durations = [ms(200), ms(200), ms(400)];

        // When
        let frames = [ms(399), ms(400), ms(800), ms(60_000)]
            .map(|played| frame_at(&durations, Playback::OneShot, played));

        // Then
        assert_eq!(frames, [1, 2, 2, 2]);
    }
}
