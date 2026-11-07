//! The springy grab string: while the owner holds the Pet, it pulls the point where they grabbed it towards the cursor.

use crate::step::SUBSTEPS;
use crate::vector::Vec2;

/// The string's feel, from `balance.toml`. It pulls only once it is longer than `length`, harder the more it
/// stretches (`stiffness`, per second squared per pixel of stretch) and against how fast it stretches (`damping`, per
/// second). Damped only along its length, a hanging Pet would swing sideways for ever, so while the string is taut a
/// little drag on how fast the Body moves relative to the cursor (`swing_drag`, per second) settles a swing without
/// spoiling a flick or a sling.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrabString {
    /// In physical pixels.
    pub length: f32,
    pub stiffness: f32,
    pub damping: f32,
    pub swing_drag: f32,
}

/// The share of the linear budget below that a string may use. At its very edge the stretch neither grows nor dies
/// down, so a fifth is kept back.
const BUDGET_SHARE: f32 = 0.8;

/// The most the string may swing the Body round in one substep while taut, in radians: the substep times the
/// string's natural frequency, `h√k`. The sweep that chose it saw a Body hanging from a still cursor first fail to
/// settle at about 1.1, so this keeps a little over half of that back.
const STIFFEST_TURN: f32 = 0.5;

/// The least damping plus swing drag, as a share of the damping that would just stop the string ringing. The sweep
/// saw a string near `STIFFEST_TURN` whirl at thousands of px/s for ever below about a hundredth of it.
const LEAST_DAMPING_RATIO: f32 = 0.05;

/// Whether a string settles, and how stiff or damped it may be. Each substep of `h` seconds adds the pull to the
/// velocity, then moves by the new velocity, which is only stable within limits, so `balance.toml` is checked
/// against them at the app's tick. `k` is the stiffness and `c` the damping plus the swing drag, as both resist the
/// string stretching.
///
/// - Along a taut string a stretch `x` moving at `v` becomes `v' = (1 - hc)v - hkx` and `x' = x + hv'`. That dies
///   down only while both roots of the step stay inside the unit circle: `k >= 0`, `c >= 0` and `h²k + 2hc < 4`. So
///   stiffness and damping share one budget, at 30 ticks a second (`h` = 1/120 s) `k + 240c < 57,600`. A string
///   may use `BUDGET_SHARE` of it.
/// - That budget isn't enough, because the string goes slack. Hanging at rest it is stretched only `g/k`, a fraction
///   of a pixel when stiff, so any bob makes the Body bounce on it, and a catch that lasts only a couple of substeps
///   adds energy. So `h√k` stays under `STIFFEST_TURN`: at 30 ticks a second `k <= 3,600`, nine times the default.
/// - The bounce on the string also needs some damping to take that energy out, at least `LEAST_DAMPING_RATIO` of
///   critical: `c >= 0.1√k`, 2 for the default stiffness.
impl GrabString {
    /// Whether the string settles rather than shaking or whirling for ever, at ticks of `dt` seconds.
    pub fn is_stable(&self, dt: f32) -> bool {
        let resisting = self.damping + self.swing_drag;
        self.stiffness >= 0.0
            && self.stiffness <= self.max_stable_stiffness(dt)
            && resisting >= self.min_stable_damping()
            && resisting <= self.max_stable_damping(dt)
    }

    /// The stiffest this string may be at ticks of `dt` seconds, given its damping and swing drag. 0 if those
    /// already use up the budget.
    pub fn max_stable_stiffness(&self, dt: f32) -> f32 {
        let h = dt / SUBSTEPS as f32;
        let resisting = self.damping + self.swing_drag;
        let budget = (4.0 * BUDGET_SHARE - 2.0 * h * resisting) / (h * h);
        budget.min((STIFFEST_TURN / h).powi(2)).max(0.0)
    }

    /// The least damping plus swing drag this string needs, given its stiffness.
    pub fn min_stable_damping(&self) -> f32 {
        2.0 * LEAST_DAMPING_RATIO * self.stiffness.max(0.0).sqrt()
    }

    /// The most damping plus swing drag this string may have at ticks of `dt` seconds, given its stiffness. 0 if the
    /// stiffness already uses up the budget.
    pub fn max_stable_damping(&self, dt: f32) -> f32 {
        let h = dt / SUBSTEPS as f32;
        ((4.0 * BUDGET_SHARE - h * h * self.stiffness) / (2.0 * h)).max(0.0)
    }
}

/// Where the cursor was, `at` seconds after the tick's start.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CursorSample {
    pub at: f32,
    pub position: Vec2,
}

/// The owner is holding the Body this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Held<'a> {
    /// The grab point relative to the Anchor: the cursor minus the Anchor when the owner grabbed it. It stays fixed
    /// for the whole hold because the Body never turns.
    pub grab: Vec2,
    pub string: GrabString,
    /// The cursor's path through this tick, oldest first, starting with where it was at the tick's start (`at` 0).
    /// It moves in a straight line between samples and stays put after the last.
    pub cursor: &'a [CursorSample],
}

impl Held<'_> {
    /// Where the cursor is `time` seconds after the tick's start, if it has a path at all.
    fn cursor_at(&self, time: f32) -> Option<Vec2> {
        let mut at = self.cursor.first()?.position;
        for pair in self.cursor.windows(2) {
            let (from, to) = (pair[0], pair[1]);
            if time <= from.at {
                break;
            }
            if time >= to.at {
                at = to.position;
                continue;
            }
            let share = (time - from.at) / (to.at - from.at);
            return Some(from.position + (to.position - from.position) * share);
        }
        Some(at)
    }

    /// The string's pull on the Body, as an acceleration, over the time from `start` to `end` seconds into the tick.
    /// It pulls the grab point towards where the cursor is at `end` once it is longer than its length, and pulls
    /// harder the faster it is stretching. It never pushes, and with no cursor path it doesn't pull.
    pub(crate) fn pull(&self, anchor: Vec2, velocity: Vec2, start: f32, end: f32) -> Vec2 {
        let (Some(was), Some(cursor)) = (self.cursor_at(start), self.cursor_at(end)) else {
            return Vec2::ZERO;
        };
        let cursor_velocity = (cursor - was) * (1.0 / (end - start));
        let towards = cursor - (anchor + self.grab);
        let distance = towards.length();
        if distance <= self.string.length {
            return Vec2::ZERO;
        }
        let along = towards * (1.0 / distance);
        let relative = cursor_velocity - velocity;
        let stretch = distance - self.string.length;
        let tension =
            (self.string.stiffness * stretch + self.string.damping * relative.dot(along)).max(0.0);
        along * tension + relative * self.string.swing_drag
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STRING: GrabString = GrabString {
        length: 24.0,
        stiffness: 400.0,
        damping: 30.0,
        swing_drag: 4.0,
    };

    fn sample(at: f32, x: f32) -> CursorSample {
        CursorSample {
            at,
            position: Vec2::new(x, 0.0),
        }
    }

    #[test]
    fn given_a_cursor_path_when_asked_between_two_samples_then_the_cursor_is_on_the_straight_line_between_them()
     {
        // Given
        let path = [sample(0.0, 0.0), sample(0.01, 100.0), sample(0.02, 100.0)];
        let held = Held {
            grab: Vec2::ZERO,
            string: STRING,
            cursor: &path,
        };

        // When
        let at = held.cursor_at(0.0025);

        // Then
        assert_eq!(at, Some(Vec2::new(25.0, 0.0)));
    }

    #[test]
    fn given_a_cursor_path_when_asked_after_its_last_sample_then_the_cursor_stays_there() {
        // Given
        let path = [sample(0.0, 0.0), sample(0.01, 100.0)];
        let held = Held {
            grab: Vec2::ZERO,
            string: STRING,
            cursor: &path,
        };

        // When
        let at = held.cursor_at(0.03);

        // Then
        assert_eq!(at, Some(Vec2::new(100.0, 0.0)));
    }

    #[test]
    fn given_the_cursor_within_the_strings_length_when_pulling_then_the_string_is_slack() {
        // Given
        let path = [sample(0.0, STRING.length - 1.0)];
        let held = Held {
            grab: Vec2::ZERO,
            string: STRING,
            cursor: &path,
        };

        // When
        let pull = held.pull(Vec2::ZERO, Vec2::new(0.0, 500.0), 0.0, 0.01);

        // Then
        assert_eq!(pull, Vec2::ZERO);
    }

    #[test]
    fn given_the_cursor_beyond_the_strings_length_when_pulling_then_it_pulls_towards_the_cursor() {
        // Given
        let path = [sample(0.0, STRING.length + 10.0)];
        let held = Held {
            grab: Vec2::ZERO,
            string: STRING,
            cursor: &path,
        };

        // When
        let pull = held.pull(Vec2::ZERO, Vec2::ZERO, 0.0, 0.01);

        // Then
        assert!(pull.x > 0.0);
        assert_eq!(pull.y, 0.0);
    }

    const DT: f32 = 1.0 / 30.0;

    #[test]
    fn given_the_balance_defaults_when_checked_at_30_ticks_a_second_then_the_string_is_stable() {
        // Given
        let string = STRING;

        // When
        let stable = string.is_stable(DT);

        // Then
        assert!(stable);
    }

    #[test]
    fn given_a_string_past_any_of_its_limits_when_checked_then_it_is_unstable() {
        // Given
        let stiff = GrabString {
            stiffness: STRING.max_stable_stiffness(DT) * 1.01,
            ..STRING
        };
        let overdamped = GrabString {
            damping: STRING.max_stable_damping(DT) * 1.01 - STRING.swing_drag,
            ..STRING
        };
        let underdamped = GrabString {
            damping: 0.0,
            swing_drag: STRING.min_stable_damping() * 0.99,
            ..STRING
        };

        // When
        let stable = [stiff, overdamped, underdamped].map(|string| string.is_stable(DT));

        // Then
        assert_eq!(stable, [false; 3]);
    }

    #[test]
    fn given_damping_near_its_limit_when_asking_for_the_stiffness_limit_then_it_is_lower_than_with_light_damping()
     {
        // Given: stiffness and damping share one budget, so heavy damping leaves less room for stiffness
        let heavy = GrabString {
            damping: STRING.max_stable_damping(DT) * 0.97 - STRING.swing_drag,
            ..STRING
        };

        // When
        let (light_limit, heavy_limit) = (
            STRING.max_stable_stiffness(DT),
            heavy.max_stable_stiffness(DT),
        );

        // Then
        assert!(
            heavy_limit < light_limit,
            "{heavy_limit} with heavy damping, {light_limit} with light"
        );
    }

    #[test]
    fn given_no_cursor_samples_when_pulling_then_the_string_does_not_pull() {
        // Given: a far-away grab point, which any cursor position would pull on
        let held = Held {
            grab: Vec2::new(5000.0, 5000.0),
            string: STRING,
            cursor: &[],
        };

        // When
        let pull = held.pull(Vec2::ZERO, Vec2::ZERO, 0.0, 0.01);

        // Then
        assert_eq!(pull, Vec2::ZERO);
    }
}
