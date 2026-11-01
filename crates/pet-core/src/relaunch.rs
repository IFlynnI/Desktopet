//! The limit on Relaunches: at most 3 in 10 minutes, so a machine that can't render doesn't restart the app for ever.

/// How many Relaunches fit in the window.
pub const MAX_RELAUNCHES: usize = 3;

const WINDOW_SECONDS: u64 = 10 * 60;

/// The history to carry into the next process, including this Relaunch, or `None` when the limit is reached. `history`
/// and `now` are times in seconds. A time in the future, from a clock that was changed, counts as recent.
pub fn record_relaunch(history: &[u64], now: u64) -> Option<Vec<u64>> {
    let in_window = |time: &&u64| now.saturating_sub(**time) < WINDOW_SECONDS;
    let mut recent: Vec<u64> = history.iter().filter(in_window).copied().collect();
    if recent.len() >= MAX_RELAUNCHES {
        return None;
    }
    recent.push(now);
    Some(recent)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 100_000;

    #[test]
    fn given_no_relaunches_when_recording_one_then_it_is_allowed_and_remembered() {
        // Given
        let history = [];

        // When
        let recorded = record_relaunch(&history, NOW);

        // Then
        assert_eq!(recorded, Some(vec![NOW]));
    }

    #[test]
    fn given_two_recent_relaunches_when_recording_a_third_then_it_is_allowed() {
        // Given
        let history = [NOW - 120, NOW - 60];

        // When
        let recorded = record_relaunch(&history, NOW);

        // Then
        assert_eq!(recorded, Some(vec![NOW - 120, NOW - 60, NOW]));
    }

    #[test]
    fn given_three_recent_relaunches_when_recording_a_fourth_then_the_limit_stops_it() {
        // Given
        let history = [NOW - 180, NOW - 120, NOW - 60];

        // When
        let recorded = record_relaunch(&history, NOW);

        // Then
        assert_eq!(recorded, None);
    }

    #[test]
    fn given_an_old_relaunch_when_recording_then_it_has_left_the_window_and_is_forgotten() {
        // Given: the first of these is more than 10 minutes ago
        let history = [NOW - 601, NOW - 120, NOW - 60];

        // When
        let recorded = record_relaunch(&history, NOW);

        // Then
        assert_eq!(recorded, Some(vec![NOW - 120, NOW - 60, NOW]));
    }

    #[test]
    fn given_a_relaunch_exactly_ten_minutes_ago_when_recording_then_it_no_longer_counts() {
        // Given
        let history = [NOW - 600, NOW - 120, NOW - 60];

        // When
        let recorded = record_relaunch(&history, NOW);

        // Then
        assert_eq!(recorded, Some(vec![NOW - 120, NOW - 60, NOW]));
    }

    #[test]
    fn given_times_from_the_future_when_recording_then_they_count_as_recent() {
        // Given: the clock went backwards
        let history = [NOW + 500, NOW + 501, NOW + 502];

        // When
        let recorded = record_relaunch(&history, NOW);

        // Then
        assert_eq!(recorded, None);
    }
}
