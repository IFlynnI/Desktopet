//! The simulation with no window and no rendering, driven by time the test sets. `MinimalPlugins` lives in the full
//! `bevy` crate, which `pet-sim` doesn't depend on, so these use the two of its plugins that matter here: the task
//! pools and time.

use std::sync::Arc;
use std::time::Duration;

use bevy_app::{App, FixedUpdate, TaskPoolPlugin, Update};
use bevy_ecs::prelude::*;
use bevy_time::{Fixed, Time, TimePlugin, TimeUpdateStrategy};
use pet_core::geometry::{Point, Rect};
use pet_core::pack::{
    ActionDef, ActionId, Anchor, AnimationDef, AnimationId, Body, CellSize, FormDef, FormId,
    PackDefinition,
};
use pet_core::placement::Facing;
use pet_sim::{
    AnimationClock, CurrentPack, Faces, Hidden, Pet, Phase, Position, ShownFrame, SimPlugin,
};

const WORK_AREA: Rect = Rect {
    left: 100,
    top: 0,
    right: 1100,
    bottom: 800,
};

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

fn form(name: &str, actions: Vec<ActionDef>, animations: Vec<AnimationDef>) -> FormDef {
    FormDef {
        name: name.to_owned(),
        cell: CellSize {
            width: 10,
            height: 6,
        },
        anchor: Anchor { x: 3, y: 6 },
        body: Body {
            x: -3,
            y: -6,
            width: 10,
            height: 6,
        },
        default_action: ActionId(actions.len() - 1),
        shadow: None,
        actions,
        animations,
        situations: Vec::new(),
    }
}

fn action(name: &str, animation: usize) -> ActionDef {
    ActionDef {
        name: name.to_owned(),
        animation: AnimationId(animation),
        body: None,
    }
}

fn animation(name: &str, durations: Vec<Duration>) -> AnimationDef {
    AnimationDef {
        name: name.to_owned(),
        durations,
        markers: Vec::new(),
    }
}

/// Two Forms, starting with the second. Its default Action is its second one, "sleep", which plays 100 ms then
/// 200 ms. Nothing is first, so picking the first of anything shows up.
fn pack() -> PackDefinition {
    let egg = form(
        "egg",
        vec![action("idle", 0)],
        vec![animation("idle", vec![ms(500)])],
    );
    let hatchling = form(
        "hatchling",
        vec![action("idle", 0), action("sleep", 1)],
        vec![
            animation("idle", vec![ms(250), ms(250)]),
            animation("sleep", vec![ms(100), ms(200)]),
        ],
    );
    PackDefinition {
        name: "Test".to_owned(),
        starting_form: FormId(1),
        forms: vec![egg, hatchling],
        emotes: Vec::new(),
        hit_mask_threshold: 128,
    }
}

/// The simulation, before its first update. Time moves only when a test says so.
fn unstarted(pack: Option<PackDefinition>, add: impl FnOnce(&mut App)) -> App {
    let mut app = App::new();
    app.add_plugins((TaskPoolPlugin::default(), TimePlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
        .add_plugins(SimPlugin {
            work_area: WORK_AREA,
        });
    if let Some(pack) = pack {
        app.insert_resource(CurrentPack(Arc::new(pack)));
    }
    add(&mut app);
    app
}

/// The simulation, after its first update, which always sees no time pass.
fn sim_with(pack: Option<PackDefinition>, add: impl FnOnce(&mut App)) -> App {
    let mut app = unstarted(pack, add);
    app.update();
    app
}

fn sim() -> App {
    sim_with(Some(pack()), |_| {})
}

fn tick_length(app: &App) -> Duration {
    app.world().resource::<Time<Fixed>>().timestep()
}

/// One update that sees `by` pass.
fn step(app: &mut App, by: Duration) {
    app.insert_resource(TimeUpdateStrategy::ManualDuration(by));
    app.update();
}

fn ticks(app: &mut App, count: u32) {
    let tick = tick_length(app);
    for _ in 0..count {
        step(app, tick);
    }
}

fn pet<T: Component + Clone>(app: &mut App) -> T {
    let world = app.world_mut();
    world
        .query_filtered::<&T, With<Pet>>()
        .single(world)
        .expect("one Pet")
        .clone()
}

/// The update in which the app learns of the hide, as `apply_presence` does.
fn hide(app: &mut App) {
    app.world_mut().resource_mut::<Hidden>().0 = true;
    step(app, Duration::ZERO);
}

#[derive(Resource)]
struct Log<T>(Vec<T>);

impl<T> Default for Log<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

fn record_frame(frames: Query<&ShownFrame, With<Pet>>, mut log: ResMut<Log<ShownFrame>>) {
    log.0.extend(frames.iter().copied());
}

#[test]
fn given_a_pack_when_the_simulation_starts_then_the_pet_stands_at_home_on_the_first_frame_of_its_default_action()
 {
    // Given
    let mut app = unstarted(Some(pack()), |_| {});

    // When
    app.update();

    // Then: home is the bottom centre of the work area
    assert_eq!(
        pet::<Position>(&mut app),
        Position(Point { x: 600, y: 800 })
    );
    assert_eq!(pet::<Faces>(&mut app), Faces(Facing::AsDrawn));
    assert_eq!(
        pet::<ShownFrame>(&mut app),
        ShownFrame {
            form: FormId(1),
            animation: AnimationId(1),
            frame: 0
        }
    );
    assert_eq!(pet::<AnimationClock>(&mut app).played, Duration::ZERO);
}

#[test]
fn given_no_pack_when_the_simulation_runs_then_there_is_no_pet() {
    // Given
    let mut app = sim_with(None, |_| {});

    // When
    ticks(&mut app, 3);

    // Then
    let world = app.world_mut();
    assert_eq!(world.query::<&Pet>().iter(world).count(), 0);
}

#[test]
fn given_one_second_passes_in_tenths_when_the_simulation_runs_then_it_ticks_thirty_times() {
    // Given
    #[derive(Resource, Default)]
    struct Ticks(u32);
    let mut app = sim_with(Some(pack()), |app| {
        app.init_resource::<Ticks>().add_systems(
            FixedUpdate,
            (|mut ticks: ResMut<Ticks>| ticks.0 += 1).in_set(Phase::Sense),
        );
    });

    // When
    for _ in 0..10 {
        step(&mut app, ms(100));
    }

    // Then
    assert_eq!(app.world().resource::<Ticks>().0, 30);
}

#[test]
fn given_a_system_in_each_phase_when_a_tick_runs_then_the_phases_run_in_order() {
    // Given: added last phase first, so only the phases' own order can put them right
    fn note(phase: Phase) -> impl FnMut(ResMut<Log<Phase>>) {
        move |mut log: ResMut<Log<Phase>>| log.0.push(phase)
    }
    let order = [
        Phase::Sense,
        Phase::Interpret,
        Phase::React,
        Phase::Decide,
        Phase::Act,
        Phase::Resolve,
        Phase::Present,
    ];
    let mut app = sim_with(Some(pack()), |app| {
        app.init_resource::<Log<Phase>>();
        for phase in order.into_iter().rev() {
            app.add_systems(FixedUpdate, note(phase).in_set(phase));
        }
    });

    // When
    ticks(&mut app, 1);

    // Then
    assert_eq!(app.world().resource::<Log<Phase>>().0, order);
}

#[test]
#[should_panic(expected = "Error when initializing schedule")]
fn given_two_systems_in_one_phase_writing_the_same_thing_in_no_stated_order_when_a_tick_runs_then_it_refuses_to_run()
 {
    // Given
    #[derive(Resource, Default)]
    struct Shared(u32);
    let mut app = sim_with(Some(pack()), |app| {
        app.init_resource::<Shared>().add_systems(
            FixedUpdate,
            (
                |mut shared: ResMut<Shared>| shared.0 += 1,
                |mut shared: ResMut<Shared>| shared.0 *= 2,
            )
                .in_set(Phase::Decide),
        );
    });

    // When / Then
    ticks(&mut app, 1);
}

#[test]
fn given_a_running_pet_when_ticks_pass_then_each_frame_shows_for_its_own_duration_and_it_loops() {
    // Given: Present reads the Frame the clock picked in Act during the same tick. A tick is 33.3 ms, so 100 ms ends
    // just after the third tick and the 300 ms loop just after the ninth.
    let mut app = sim_with(Some(pack()), |app| {
        app.init_resource::<Log<ShownFrame>>()
            .add_systems(FixedUpdate, record_frame.in_set(Phase::Present));
    });

    // When
    ticks(&mut app, 10);

    // Then
    let frames: Vec<usize> = app
        .world()
        .resource::<Log<ShownFrame>>()
        .0
        .iter()
        .map(|shown| shown.frame)
        .collect();
    assert_eq!(frames, [0, 0, 0, 1, 1, 1, 1, 1, 1, 0]);
    assert_eq!(
        pet::<AnimationClock>(&mut app).played,
        tick_length(&app) * 10
    );
}

#[test]
fn given_a_hidden_pet_when_time_passes_then_its_clock_holds() {
    // Given
    let mut app = sim();
    ticks(&mut app, 4);
    hide(&mut app);
    let before = pet::<AnimationClock>(&mut app);
    let shown = pet::<ShownFrame>(&mut app);

    // When
    for _ in 0..10 {
        step(&mut app, Duration::from_secs(10));
    }

    // Then
    assert_eq!(pet::<AnimationClock>(&mut app), before);
    assert_eq!(pet::<ShownFrame>(&mut app), shown);
}

#[test]
fn given_a_pet_hidden_for_a_long_time_when_the_update_that_wakes_it_sees_all_that_time_then_it_moves_on_by_one_tick_after()
 {
    // Given: the app learns of the wake during the update itself, after the fixed ticks have run
    #[derive(Resource, Default)]
    struct WakeDuringUpdate(bool);
    let mut app = sim_with(Some(pack()), |app| {
        app.init_resource::<WakeDuringUpdate>().add_systems(
            Update,
            |mut wake: ResMut<WakeDuringUpdate>, mut hidden: ResMut<Hidden>| {
                if std::mem::take(&mut wake.0) {
                    hidden.0 = false;
                }
            },
        );
    });
    ticks(&mut app, 4);
    hide(&mut app);
    let before = pet::<AnimationClock>(&mut app).played;

    // When: an hour hidden, then one more tick shown
    app.world_mut().resource_mut::<WakeDuringUpdate>().0 = true;
    step(&mut app, Duration::from_secs(3600));
    let on_waking = pet::<AnimationClock>(&mut app).played;
    ticks(&mut app, 1);

    // Then
    assert_eq!(on_waking, before);
    assert_eq!(
        pet::<AnimationClock>(&mut app).played,
        before + tick_length(&app)
    );
}

#[test]
fn given_a_pet_shown_again_just_before_an_update_when_that_update_sees_the_hidden_time_then_it_moves_on_by_one_tick_after()
 {
    // Given
    let mut app = sim();
    ticks(&mut app, 4);
    hide(&mut app);
    let before = pet::<AnimationClock>(&mut app).played;

    // When: shown between updates, then the next update sees the hour that passed
    app.world_mut().resource_mut::<Hidden>().0 = false;
    step(&mut app, Duration::from_secs(3600));
    let on_waking = pet::<AnimationClock>(&mut app).played;
    ticks(&mut app, 1);

    // Then
    assert_eq!(on_waking, before);
    assert_eq!(
        pet::<AnimationClock>(&mut app).played,
        before + tick_length(&app)
    );
}
