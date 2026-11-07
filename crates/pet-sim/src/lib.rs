//! The Pet's world, as ECS systems over `pet-core`'s rules. It depends on Bevy's ECS, app and time crates but not on
//! rendering, so it can't reach the renderer (ADR-0002). The renderer reads its components, and nothing flows back.
//!
//! A tick runs 30 times a second, in the phases of [`Phase`]. The Pet appears at the bottom centre of the work area,
//! standing on the Ground plane, and plays its Form's default Action on a loop. The owner can grab it, swing it and
//! throw it, and it falls, bounces off the screen edges and Obstacles, and lands, playing its held, falling and
//! landing Situations as it goes. It shows the Emotes it's asked to with [`EmoteRequest`], though nothing asks yet.

mod clock;
mod emotes;
mod gestures;
mod input;
mod motion;
mod pack;
mod phases;

use bevy_app::{App, Last, Plugin, PreUpdate};
use bevy_ecs::prelude::*;
use pet_core::geometry::{Point, Rect};
use pet_core::physics::{Obstacle, Tuning};
use pet_core::placement::Facing;

pub use clock::{AnimationClock, Hidden, ShownFrame};
pub use emotes::{EmoteClock, EmoteFrame, EmoteRequest, PlayingEmote, ShownEmote};
pub use gestures::GestureMade;
pub use input::{ButtonChanged, MouseLost, PointerMoved, PointerThisTick};
pub use motion::{Held, InSituation, Landed, Motion, Obstacles, Scale};
pub use pack::{CurrentPack, PackReloaded, replace_pack};
pub use phases::Phase;

/// Runs the simulation. `work_area` is where the Pet lives, in physical pixels: its bottom is the Ground plane, and
/// its other sides are the screen edges. `scale` is how many physical pixels each art pixel takes up. `obstacles` are
/// on screen, in physical pixels.
pub struct SimPlugin {
    pub work_area: Rect,
    pub scale: u32,
    pub physics: Tuning,
    pub obstacles: Vec<Obstacle>,
}

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        phases::configure(app);
        app.insert_resource(WorkArea(self.work_area))
            .insert_resource(Scale(self.scale))
            .insert_resource(motion::PhysicsInUse(self.physics))
            .insert_resource(Obstacles(self.obstacles.clone()))
            .init_resource::<Hidden>()
            .init_resource::<input::LastCursor>()
            .init_resource::<PointerThisTick>()
            .init_resource::<gestures::Recognising>()
            .add_message::<PackReloaded>()
            .add_message::<EmoteRequest>()
            .add_message::<PointerMoved>()
            .add_message::<ButtonChanged>()
            .add_message::<MouseLost>()
            .add_message::<GestureMade>()
            .add_message::<Landed>()
            .add_systems(
                PreUpdate,
                pack::spawn_pet.run_if(resource_added::<CurrentPack>),
            )
            .add_systems(
                bevy_app::FixedUpdate,
                (
                    input::sense.in_set(Phase::Sense),
                    gestures::interpret.in_set(Phase::Interpret),
                    (emotes::receive, motion::take_hold).in_set(Phase::React),
                    (clock::advance, emotes::advance).in_set(Phase::Act),
                    motion::resolve.in_set(Phase::Resolve),
                ),
            )
            .add_systems(
                Last,
                (clock::hold_while_hidden, motion::let_go_when_hidden)
                    .run_if(resource_changed::<Hidden>),
            );
    }
}

/// Where the Pet lives, in physical pixels. The app changes it when the taskbar moves or resizes, and the Pet then
/// lands on, or is pushed up onto, the new Ground plane.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkArea(pub Rect);

/// The Pet, the one Actor so far.
#[derive(Component, Debug)]
pub struct Pet;

/// Where an Actor's Anchor is, in physical pixels. Where an Actor is means where its Anchor is.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position(pub Point);

/// Which way an Actor faces.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Faces(pub Facing);
