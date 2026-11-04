//! The Pack in use. The simulation sees only its Definition, never its pixels.

use std::sync::Arc;

use bevy_ecs::prelude::*;
use pet_core::animation_clock::Playback;
use pet_core::pack::PackDefinition;
use pet_core::placement;

use crate::clock::{AnimationClock, ShownFrame};
use crate::emotes::{EmoteClock, ShownEmote};
use crate::{Faces, Pet, Position, WorkArea};

/// The Definition of the Pack in use, shared read-only with whoever else needs it.
#[derive(Resource, Clone, Debug)]
pub struct CurrentPack(pub Arc<PackDefinition>);

/// Sent when an edited Pack has replaced the one in use. Roadmap item 3's slice 5 sends it.
#[derive(Message, Clone, Debug)]
pub struct PackReloaded;

/// When a Pack arrives, the Pet appears at home in the starting Form, playing its default Action on a loop, with no
/// Emote.
pub(crate) fn spawn_pet(mut commands: Commands, pack: Res<CurrentPack>, work_area: Res<WorkArea>) {
    let definition = &pack.0;
    let form = definition.starting_form;
    let animation = definition.form(form).default_animation();
    commands.spawn((
        Pet,
        Position(placement::home(work_area.0)),
        Faces::default(),
        AnimationClock {
            form,
            animation,
            playback: Playback::Looping,
            played_ticks: 0,
        },
        ShownFrame {
            form,
            animation,
            frame: 0,
        },
        EmoteClock::default(),
        ShownEmote::default(),
    ));
}
