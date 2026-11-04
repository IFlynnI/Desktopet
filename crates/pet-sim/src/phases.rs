//! The phases of a tick. Each tick runs every phase once, in this order, so a system only has to say which phase it
//! belongs to. Inside a phase, systems must not touch the same data in an unstated order: ambiguity detection is on and
//! turns that into an error, because parallel systems can otherwise run in any order (ADR-0002).

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{LogLevel, ScheduleBuildSettings};
use bevy_time::{Fixed, Time};

/// How often the simulation ticks. Bevy's fixed clock runs the ticks, however often the engine updates.
pub const TICKS_PER_SECOND: f64 = 30.0;

/// One phase of a tick, in the order they run.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Phase {
    /// Reads what happened outside: the cursor, the time, OS events.
    Sense,
    /// Turns what was sensed into meaning, such as a poke.
    Interpret,
    /// Quick reactions that can't wait for a decision.
    React,
    /// The Mind picks the next Action.
    Decide,
    /// Actions run, and Animation clocks move on.
    Act,
    /// Physics and hits settle.
    Resolve,
    /// The results are made ready for others to read.
    Present,
}

pub(crate) fn configure(app: &mut App) {
    app.insert_resource(Time::<Fixed>::from_hz(TICKS_PER_SECOND))
        .configure_sets(
            FixedUpdate,
            (
                Phase::Sense,
                Phase::Interpret,
                Phase::React,
                Phase::Decide,
                Phase::Act,
                Phase::Resolve,
                Phase::Present,
            )
                .chain(),
        )
        .edit_schedule(FixedUpdate, |schedule| {
            schedule.set_build_settings(ScheduleBuildSettings {
                ambiguity_detection: LogLevel::Error,
                ..Default::default()
            });
        });
}
