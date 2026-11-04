//! Animation clocks: how long each Actor's Animation has played, and so which Frame shows. The simulation owns this,
//! so attack timing, click picking and the Hit mask all agree with the Frame that is drawn.
//!
//! While Hidden the clock holds. The engine stops updating while Hidden, so the first update on waking sees a long gap.
//! Bevy's virtual clock caps that at its `max_delta`, 250 ms by default, and the fixed clock would then run a burst of
//! up to 7 ticks to catch up. Pausing the virtual clock while Hidden means none of that time is added, so there is no
//! burst and the Pet carries on from where it was.

use std::time::Duration;

use bevy_ecs::prelude::*;
use bevy_time::{Time, Virtual};
use pet_core::animation_clock::{self, Playback};
use pet_core::pack::{AnimationId, FormId};

use crate::pack::CurrentPack;

/// Which Animation an Actor is playing, and for how long it has played.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct AnimationClock {
    pub form: FormId,
    pub animation: AnimationId,
    pub playback: Playback,
    pub played: Duration,
}

/// The Frame the clock has picked. It changes only when the Frame does, so readers can react to the change.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShownFrame {
    pub form: FormId,
    pub animation: AnimationId,
    pub frame: usize,
}

/// Whether the Pet is Hidden. The app sets it, and the clock holds while it's `true`.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Hidden(pub bool);

pub(crate) fn advance(
    time: Res<Time>,
    pack: Option<Res<CurrentPack>>,
    mut clocks: Query<(&mut AnimationClock, &mut ShownFrame)>,
) {
    let Some(pack) = pack else { return };
    for (mut clock, mut shown) in &mut clocks {
        clock.played += time.delta();
        let durations = &pack.0.form(clock.form).animation(clock.animation).durations;
        shown.set_if_neq(ShownFrame {
            form: clock.form,
            animation: clock.animation,
            frame: animation_clock::frame_at(durations, clock.playback, clock.played),
        });
    }
}

/// Runs last in the update that learnt of the change, so the next update's time already follows it.
pub(crate) fn hold_while_hidden(hidden: Res<Hidden>, mut time: ResMut<Time<Virtual>>) {
    if hidden.0 {
        time.pause();
    } else {
        time.unpause();
    }
}
