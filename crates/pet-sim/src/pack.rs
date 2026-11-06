//! The Pack in use. The simulation sees only its Definition, never its pixels.

use std::sync::Arc;

use bevy_ecs::prelude::*;
use pet_core::animation_clock::Playback;
use pet_core::pack::{FormId, PackDefinition};
use pet_core::placement;

use crate::clock::{AnimationClock, ShownFrame};
use crate::emotes::{EmoteClock, EmoteRequest, ShownEmote};
use crate::{Faces, Pet, Position, WorkArea};

/// The Definition of the Pack in use, shared read-only with whoever else needs it.
#[derive(Resource, Clone, Debug)]
pub struct CurrentPack(pub Arc<PackDefinition>);

/// Sent when an edited Pack has replaced the one in use, by `replace_pack`. It's sent outside the ticks, so while
/// Hidden it waits, as every message does, and the first tick after waking reads it.
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

/// Swaps in an edited Pack's Definition. It all happens here, at once, so nothing can read the new Pack with the old
/// one's numbers: call it from an exclusive system that runs before the ticks and the renderer.
///
/// Each Actor stays where it is and faces the same way. The Pet, while it's in the starting Form, takes the new Pack's
/// starting Form, so an author who changes `starting_form` sees it. Any other Actor keeps its Form if the new Pack has
/// one of that name, since numbers come from a Pack's order and can change with any edit, or else takes the new
/// starting Form. Its clock starts again on that Form's default Action, showing its first Frame before any time
/// passes, as a Pet that has just appeared does. Its Emote ends: the Emote may have changed or gone, and whoever
/// showed it can show it again. Emote requests not yet read go too, because they name Emotes by the old Pack's
/// numbers.
///
/// Every Actor's `ShownFrame` and `ShownEmote` count as changed afterwards, even when the numbers are the same,
/// because the Frame they name is the new Pack's. So whatever draws them, or hit-tests them, does it again.
///
/// With no Pack in use yet, because the one at startup was broken, this one is used, and the Pet appears.
pub fn replace_pack(world: &mut World, definition: Arc<PackDefinition>) {
    let old = world
        .get_resource::<CurrentPack>()
        .map(|pack| pack.0.clone());
    match old {
        Some(old) => {
            let mut actors = world.query::<(
                &mut ShownFrame,
                Option<&mut AnimationClock>,
                Option<&mut EmoteClock>,
                Option<&mut ShownEmote>,
                Has<Pet>,
            )>();
            for (mut shown, clock, emote_clock, shown_emote, is_pet) in actors.iter_mut(world) {
                let form = if is_pet {
                    pet_form_after_edit(&old, &definition, shown.form)
                } else {
                    same_form(&old, &definition, shown.form)
                };
                let animation = definition.form(form).default_animation();
                *shown = ShownFrame {
                    form,
                    animation,
                    frame: 0,
                };
                if let Some(mut clock) = clock {
                    *clock = AnimationClock {
                        form,
                        animation,
                        playback: Playback::Looping,
                        played_ticks: 0,
                    };
                }
                if let Some(mut emote_clock) = emote_clock {
                    *emote_clock = EmoteClock(None);
                }
                if let Some(mut shown_emote) = shown_emote {
                    *shown_emote = ShownEmote(None);
                }
            }
            world.resource_mut::<CurrentPack>().0 = definition;
        }
        None => world.insert_resource(CurrentPack(definition)),
    }
    if let Some(mut requests) = world.get_resource_mut::<Messages<EmoteRequest>>() {
        requests.clear();
    }
    world.write_message(PackReloaded);
}

/// The Form of `new` the Pet in `form` of `old` takes. In the starting Form, it takes the new starting Form: the same
/// Form, if only its number changed, or the one the author now starts with. In any other Form it keeps it by name, so
/// an edit that only changes `starting_form` doesn't throw a grown Pet back. The starting Form is the Pet's alone, so
/// other Actors are always kept by name.
///
/// The rule has no memory: it looks only at the Form the Pet is in now. So two edits can still move a grown Pet: one
/// that makes its current Form the starting one, then one that changes `starting_form` back. Telling a Pet that has
/// grown from one that never left its starting Form needs state of its own, its Progress, which roadmap item 4 brings.
fn pet_form_after_edit(old: &PackDefinition, new: &PackDefinition, form: FormId) -> FormId {
    if form == old.starting_form {
        new.starting_form
    } else {
        same_form(old, new, form)
    }
}

/// The Form of `new` with the name `form` has in `old`, or else `new`'s starting Form.
fn same_form(old: &PackDefinition, new: &PackDefinition, form: FormId) -> FormId {
    let name = &old.form(form).name;
    new.forms
        .iter()
        .position(|candidate| candidate.name == *name)
        .map_or(new.starting_form, FormId)
}
