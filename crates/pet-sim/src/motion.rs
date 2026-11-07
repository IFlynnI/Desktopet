//! The Pet's body in the world: gravity, landing, the screen edges, Obstacles, and the owner's grab string, by
//! `pet_physics`'s rules (ADR-0003). It also picks the Situation the Pet is in, held, falling or landing, which decides
//! the Animation it plays.
//!
//! The physics keeps its own exact position in fractions of a pixel. `Position` is that position rounded, and it only
//! changes when the rounded value does, so the renderer and the hit test redo their work only then.

use std::time::Instant;

use bevy_ecs::prelude::*;
use bevy_time::{Fixed, Time};
use pet_core::animation_clock::{self, Playback};
use pet_core::geometry::Point;
use pet_core::gesture::Gesture;
use pet_core::pack::{FormDef, Situation};
use pet_core::physics::{self, CursorSample, Extent, Obstacle, Tuning, Vec2, World};

use crate::clock::{AnimationClock, ShownFrame};
use crate::gestures::{GestureMade, Recognising};
use crate::input::PointerThisTick;
use crate::pack::CurrentPack;
use crate::{Pet, Position, WorkArea};

/// How many physical pixels each art pixel takes up on screen.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scale(pub u32);

/// The app-wide feel numbers for physics in use, from `balance.toml`.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub(crate) struct PhysicsInUse(pub(crate) Tuning);

/// The Obstacles in the world, in physical pixels on screen.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct Obstacles(pub Vec<Obstacle>);

/// An Actor's body as the physics sees it: its exact position, its velocity, its box and its Material.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Motion(pub physics::Body);

/// The owner is holding the Actor by the point `grab` from its Anchor, until `let_go_at`, if they have let go.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Held {
    pub grab: Vec2,
    pub let_go_at: Option<Instant>,
}

/// The Situation an Actor is in, if any.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InSituation(pub Option<Situation>);

/// An Actor landed on the Ground plane or an Obstacle, coming down at `speed` px/s. Memory and XP will hear of it.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct Landed {
    pub actor: Entity,
    pub speed: f32,
}

/// The physics body of an Actor in `form` standing with its Anchor at `anchor`, so its first tick isn't a landing.
pub(crate) fn body_of(form: &FormDef, anchor: Point, scale: Scale) -> physics::Body {
    let mut body = physics::Body::new(
        Vec2::new(anchor.x as f32, anchor.y as f32),
        extent(form, scale),
        form.material,
    );
    body.standing = true;
    body
}

/// The box of `form`'s Body on screen, relative to its Anchor.
pub(crate) fn extent(form: &FormDef, scale: Scale) -> Extent {
    let body = form.body;
    Extent::scaled(body.x, body.y, body.width, body.height, scale.0)
}

/// React: a drag takes hold of the Pet where the cursor grabbed it, and ends when the owner lets go. A tick can hear
/// both, as with a tap shorter than a tick, so each Pet's hold is worked out from all of them in order and written
/// once: a hold inserted through `Commands` only appears after this system, too late for a release in the same tick.
pub(crate) fn take_hold(
    mut commands: Commands,
    mut gestures: MessageReader<GestureMade>,
    pets: Query<(Entity, &Motion, Option<&Held>), With<Pet>>,
) {
    let made: Vec<GestureMade> = gestures.read().copied().collect();
    if made.is_empty() {
        return;
    }
    for (pet, motion, held) in &pets {
        let mut hold = held.copied();
        for made in &made {
            match made.gesture {
                Gesture::DragStart { at } => {
                    hold = Some(Held {
                        grab: Vec2::new(at.x as f32, at.y as f32) - motion.0.anchor,
                        let_go_at: None,
                    });
                }
                Gesture::DragEnd { .. } => {
                    if let Some(hold) = hold.as_mut() {
                        hold.let_go_at = Some(made.at);
                    }
                }
            }
        }
        if hold != held.copied() {
            match hold {
                Some(hold) => commands.entity(pet).insert(hold),
                None => commands.entity(pet).remove::<Held>(),
            };
        }
    }
}

/// What the physics moves of an Actor, and what follows from where it ends up.
type Moving = (
    Entity,
    &'static mut Motion,
    Option<&'static Held>,
    &'static mut Position,
    &'static mut InSituation,
    &'static mut AnimationClock,
    &'static mut ShownFrame,
);

/// Resolve: one step of the physics for every Actor, then its Position and its Situation.
#[allow(clippy::too_many_arguments)]
pub(crate) fn resolve(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    tuning: Res<PhysicsInUse>,
    work_area: Res<WorkArea>,
    obstacles: Res<Obstacles>,
    pointer: Res<PointerThisTick>,
    pack: Option<Res<CurrentPack>>,
    mut landings: MessageWriter<Landed>,
    mut actors: Query<Moving>,
) {
    let Some(pack) = pack else { return };
    let world = World {
        bounds: work_area.0,
        surface: tuning.0.surface(),
        gravity: tuning.0.gravity,
        obstacles: obstacles.0.clone(),
    };
    let dt = time.timestep().as_secs_f32();
    for (actor, mut motion, held, mut position, mut situation, mut clock, mut shown) in &mut actors
    {
        let path = held.map(|held| cursor_path(&pointer.path, held.let_go_at, dt));
        let step = physics::step(
            &world,
            &mut motion.0,
            held.zip(path.as_deref())
                .map(|(held, cursor)| physics::Held {
                    grab: held.grab,
                    string: tuning.0.grab,
                    cursor,
                }),
            dt,
        );
        if held.is_some_and(|held| held.let_go_at.is_some()) {
            physics::release(&mut motion.0, tuning.0.max_release_speed);
            commands.entity(actor).remove::<Held>();
        }
        let still_held = held.is_some_and(|held| held.let_go_at.is_none());
        // Set down in the owner's hand is no landing: nothing fell. That includes the tick the owner lets go in, which
        // the physics steps as held; a fall after letting go lands in a later tick.
        if let Some(landing) = step.landing.filter(|_| held.is_none()) {
            landings.write(Landed {
                actor,
                speed: landing.speed,
            });
        }
        position.set_if_neq(Position(rounded(motion.0.anchor)));

        let form = pack.0.form(clock.form);
        let next = next_situation(situation.0, still_held, &step, form, &clock);
        if next != situation.0 || step.landing.is_some() && next == Some(Situation::Landing) {
            situation.0 = next;
            play(form, next, &mut clock, &mut shown);
        }
    }
}

/// The cursor's path through this tick for the physics: times in seconds from the tick's start, ending when the owner
/// let go. The first point is where the cursor was last seen before the tick, at its start.
fn cursor_path(path: &[(Instant, Point)], until: Option<Instant>, dt: f32) -> Vec<CursorSample> {
    let Some(&(start, _)) = path.first() else {
        return Vec::new();
    };
    path.iter()
        .take_while(|(at, _)| until.is_none_or(|until| *at <= until))
        .map(|&(at, position)| CursorSample {
            at: at.duration_since(start).as_secs_f32().min(dt),
            position: Vec2::new(position.x as f32, position.y as f32),
        })
        .collect()
}

fn rounded(anchor: Vec2) -> Point {
    Point {
        x: anchor.x.round() as i32,
        y: anchor.y.round() as i32,
    }
}

/// Held while held, falling while in the air, landing from the tick it lands until its landing Animation ends, and
/// otherwise none. A Situation the Form has no Animation for plays the default one, so landing, which ends by
/// itself, only lasts when there is an Animation to end.
fn next_situation(
    now: Option<Situation>,
    held: bool,
    step: &physics::Step,
    form: &FormDef,
    clock: &AnimationClock,
) -> Option<Situation> {
    if held {
        Some(Situation::Held)
    } else if !step.standing {
        Some(Situation::Falling)
    } else if step.landing.is_some()
        || now == Some(Situation::Landing) && !landing_ended(form, clock)
    {
        animation_for(form, Situation::Landing).map(|_| Situation::Landing)
    } else {
        None
    }
}

fn landing_ended(form: &FormDef, clock: &AnimationClock) -> bool {
    let durations = &form.animation(clock.animation).durations;
    animation_clock::has_ended(durations, clock.played_ticks)
}

fn animation_for(form: &FormDef, situation: Situation) -> Option<pet_core::pack::AnimationId> {
    form.situations
        .iter()
        .find(|(mapped, _)| *mapped == situation)
        .map(|(_, animation)| *animation)
}

/// Starts the Animation for `situation` from its first Frame, as a Pet that has just appeared does. Landing plays
/// once, and the rest loop.
fn play(
    form: &FormDef,
    situation: Option<Situation>,
    clock: &mut AnimationClock,
    shown: &mut ShownFrame,
) {
    let animation = situation
        .and_then(|situation| animation_for(form, situation))
        .unwrap_or_else(|| form.default_animation());
    let playback = if situation == Some(Situation::Landing) {
        Playback::OneShot
    } else {
        Playback::Looping
    };
    *clock = AnimationClock {
        form: clock.form,
        animation,
        playback,
        played_ticks: 0,
    };
    *shown = ShownFrame {
        form: clock.form,
        animation,
        frame: 0,
    };
}

/// While Hidden no tick runs and the Overlay can't hear the button come up, so a hide ends any drag. The Pet keeps
/// the speed it had, and falls when it's shown again.
pub(crate) fn let_go_when_hidden(
    mut commands: Commands,
    hidden: Res<crate::Hidden>,
    tuning: Res<PhysicsInUse>,
    mut recognising: ResMut<Recognising>,
    mut held: Query<(Entity, &mut Motion), With<Held>>,
) {
    if !hidden.0 {
        return;
    }
    recognising.0 = Default::default();
    for (actor, mut motion) in &mut held {
        physics::release(&mut motion.0, tuning.0.max_release_speed);
        commands.entity(actor).remove::<Held>();
    }
}
