//! The Pet's world, as ECS systems over `pet-core`'s rules. It depends on Bevy's ECS, app and time crates but not on
//! rendering, so it can't reach the renderer (ADR-0002). The renderer reads its components, and nothing flows back.
//!
//! A tick runs 30 times a second, in the phases of [`Phase`]. In roadmap item 3 the Pet stands still at the bottom
//! centre of the work area and plays its Form's default Action on a loop.

mod clock;
mod pack;
mod phases;

use bevy_app::{App, Last, Plugin, PreUpdate};
use bevy_ecs::prelude::*;
use pet_core::geometry::{Point, Rect};
use pet_core::placement::Facing;

pub use clock::{AnimationClock, Hidden, ShownFrame};
pub use pack::{CurrentPack, PackReloaded};
pub use phases::{Phase, TICKS_PER_SECOND};

/// Runs the simulation. `work_area` is where the Pet lives, in physical pixels.
pub struct SimPlugin {
    pub work_area: Rect,
}

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        phases::configure(app);
        app.insert_resource(WorkArea(self.work_area))
            .init_resource::<Hidden>()
            .add_message::<PackReloaded>()
            .add_systems(
                PreUpdate,
                pack::spawn_pet.run_if(resource_added::<CurrentPack>),
            )
            .add_systems(bevy_app::FixedUpdate, clock::advance.in_set(Phase::Act))
            .add_systems(
                Last,
                clock::hold_while_hidden.run_if(resource_changed::<Hidden>),
            );
    }
}

/// Where the Pet lives, in physical pixels.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkArea(pub Rect);

/// The Pet, the one Actor in roadmap item 3.
#[derive(Component, Debug)]
pub struct Pet;

/// Where an Actor's Anchor is, in physical pixels. Where an Actor is means where its Anchor is.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position(pub Point);

/// Which way an Actor faces.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Faces(pub Facing);
