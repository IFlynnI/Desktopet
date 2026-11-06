//! The simulation with no window and no rendering, driven by time the test sets. `MinimalPlugins` lives in the full
//! `bevy` crate, which `pet-sim` doesn't depend on, so these use the two of its plugins that matter here: the task
//! pools and time.

use std::sync::Arc;
use std::time::Duration;

use bevy_app::{App, FixedUpdate, TaskPoolPlugin, Update};
use bevy_ecs::prelude::*;
use bevy_time::{Fixed, Time, TimePlugin, TimeUpdateStrategy};
use pet_core::animation_clock::Playback;
use pet_core::geometry::{Point, Rect};
use pet_core::pack::{
    ActionDef, ActionId, Anchor, AnimationDef, AnimationId, Body, CellSize, EmoteDef, EmoteId,
    EmoteMode, FormDef, FormId, PackDefinition,
};
use pet_core::placement::Facing;
use pet_sim::{
    AnimationClock, CurrentPack, EmoteClock, EmoteFrame, EmoteRequest, Faces, Hidden, PackReloaded,
    Pet, Phase, PlayingEmote, Position, ShownEmote, ShownFrame, SimPlugin, replace_pack,
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
    assert_eq!(pet::<AnimationClock>(&mut app).played_ticks, 0);
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

/// The test Pack, with its starting Form's default Animation, "sleep", playing `durations` instead.
fn pack_playing(durations: Vec<Duration>) -> PackDefinition {
    let mut pack = pack();
    pack.forms[1].animations[1].durations = durations;
    pack
}

/// Each item of `items`, with how many times in a row it comes.
fn runs<T: PartialEq>(items: impl IntoIterator<Item = T>) -> Vec<(T, usize)> {
    let mut runs: Vec<(T, usize)> = Vec::new();
    for item in items {
        match runs.last_mut() {
            Some((last, count)) if *last == item => *count += 1,
            _ => runs.push((item, 1)),
        }
    }
    runs
}

/// `loop_runs` again and again, `loops` times.
fn repeated<T: Copy>(loop_runs: &[T], loops: usize) -> Vec<T> {
    loop_runs.repeat(loops)
}

/// Each Frame the Pet shows while it plays `durations` on a loop, with how many ticks in a row it shows for. The
/// update the Pet appears in shows its first Frame before any tick, so that counts as the first, then `count` ticks.
fn pet_runs(durations: Vec<Duration>, count: u32) -> Vec<(usize, usize)> {
    let mut app = sim_with(Some(pack_playing(durations)), |app| {
        app.init_resource::<Log<ShownFrame>>()
            .add_systems(FixedUpdate, record_frame.in_set(Phase::Present));
    });
    let first = pet::<ShownFrame>(&mut app);
    ticks(&mut app, count);
    assert_eq!(
        pet::<AnimationClock>(&mut app).played_ticks,
        u64::from(count)
    );
    let log = &app.world().resource::<Log<ShownFrame>>().0;
    runs(std::iter::once(&first).chain(log).map(|shown| shown.frame))
}

#[test]
fn given_a_running_pet_when_ticks_pass_then_each_frame_shows_for_exactly_its_own_number_of_ticks_and_it_loops()
 {
    // Given: 100 ms then 200 ms, exactly 3 and 6 ticks
    let durations = vec![ms(100), ms(200)];

    // When: four loops
    let runs = pet_runs(durations, 4 * 9 - 1);

    // Then
    assert_eq!(runs, repeated(&[(0, 3), (1, 6)], 4));
}

#[test]
fn given_a_pet_playing_brims_tired_timings_when_four_loops_pass_then_each_frame_shows_for_exactly_ten_ticks()
 {
    // Given: 333, 333, 334 and 333 ms, starting at 0, 333, 666 and 1000 ms. Each start falls within the tick it
    // would start on if the loop were exactly 40 ticks, so none runs a tick long or short.
    let durations = vec![ms(333), ms(333), ms(334), ms(333)];

    // When
    let runs = pet_runs(durations, 4 * 40 - 1);

    // Then
    assert_eq!(runs, repeated(&[(0, 10), (1, 10), (2, 10), (3, 10)], 4));
}

#[test]
fn given_a_pet_playing_brims_annoyed_timings_when_three_loops_pass_then_each_frame_shows_for_its_exact_ticks()
 {
    // Given: 200, 133 and 200 ms, starting at 0, 200 and 333 ms in a 533 ms loop
    let durations = vec![ms(200), ms(133), ms(200)];

    // When
    let runs = pet_runs(durations, 3 * 16 - 1);

    // Then: 200 ms is exactly 6 ticks
    assert_eq!(runs, repeated(&[(0, 6), (1, 4), (2, 6)], 3));
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
    let before = pet::<AnimationClock>(&mut app).played_ticks;

    // When: an hour hidden, then one more tick shown
    app.world_mut().resource_mut::<WakeDuringUpdate>().0 = true;
    step(&mut app, Duration::from_secs(3600));
    let on_waking = pet::<AnimationClock>(&mut app).played_ticks;
    ticks(&mut app, 1);

    // Then
    assert_eq!(on_waking, before);
    assert_eq!(pet::<AnimationClock>(&mut app).played_ticks, before + 1);
}

#[test]
fn given_a_pet_shown_again_just_before_an_update_when_that_update_sees_the_hidden_time_then_it_moves_on_by_one_tick_after()
 {
    // Given
    let mut app = sim();
    ticks(&mut app, 4);
    hide(&mut app);
    let before = pet::<AnimationClock>(&mut app).played_ticks;

    // When: shown between updates, then the next update sees the hour that passed
    app.world_mut().resource_mut::<Hidden>().0 = false;
    step(&mut app, Duration::from_secs(3600));
    let on_waking = pet::<AnimationClock>(&mut app).played_ticks;
    ticks(&mut app, 1);

    // Then
    assert_eq!(on_waking, before);
    assert_eq!(pet::<AnimationClock>(&mut app).played_ticks, before + 1);
}

fn emote(name: &str, durations: Vec<Duration>, mode: EmoteMode) -> EmoteDef {
    EmoteDef {
        name: name.to_owned(),
        cell: CellSize {
            width: 8,
            height: 8,
        },
        durations,
        mode,
    }
}

/// The test Pack with Emotes, none of them first, so picking the first Emote shows up:
/// - "tired" loops, 100 ms then 200 ms;
/// - "love" plays once with Brim's timings, none of them a whole number of ticks;
/// - "blink" plays once, for 34 ms, the shortest Frame a Pack may have;
/// - "glance" plays once, with a 34 ms Frame in the middle.
fn pack_with_emotes() -> PackDefinition {
    PackDefinition {
        emotes: vec![
            emote("spare", vec![ms(500)], EmoteMode::Looping),
            emote("tired", vec![ms(100), ms(200)], EmoteMode::Looping),
            emote(
                "love",
                [66, 67, 100, 400, 200, 133].map(ms).to_vec(),
                EmoteMode::OneShot,
            ),
            emote("blink", vec![ms(34)], EmoteMode::OneShot),
            emote("glance", vec![ms(50), ms(34), ms(100)], EmoteMode::OneShot),
        ],
        ..pack()
    }
}

const TIRED: EmoteId = EmoteId(1);
const LOVE: EmoteId = EmoteId(2);
const BLINK: EmoteId = EmoteId(3);
const GLANCE: EmoteId = EmoteId(4);

/// The simulation with Emotes, logging the Emote shown at the end of every tick.
fn emoting() -> App {
    sim_with(Some(pack_with_emotes()), |app| {
        app.init_resource::<Log<ShownEmote>>()
            .add_systems(FixedUpdate, record_emote.in_set(Phase::Present));
    })
}

fn record_emote(emotes: Query<&ShownEmote, With<Pet>>, mut log: ResMut<Log<ShownEmote>>) {
    log.0.extend(emotes.iter().copied());
}

/// The Frame of `emote` shown at the end of each tick, or `None` when no Emote shows or another one does.
fn emote_frames(app: &App, emote: EmoteId) -> Vec<Option<usize>> {
    app.world()
        .resource::<Log<ShownEmote>>()
        .0
        .iter()
        .map(|shown| {
            shown
                .0
                .filter(|shown| shown.emote == emote)
                .map(|shown| shown.frame)
        })
        .collect()
}

/// Each Frame of `emote` shown, or `None`, with how many ticks in a row it showed for.
fn emote_runs(app: &App, emote: EmoteId) -> Vec<(Option<usize>, usize)> {
    runs(emote_frames(app, emote))
}

/// Each Frame shown by an Emote of `durations` in `mode`, or `None`, with how many ticks in a row it showed for, from
/// the tick that reads the request and for `count` ticks in all.
fn emote_played(
    durations: Vec<Duration>,
    mode: EmoteMode,
    count: u32,
) -> Vec<(Option<usize>, usize)> {
    let timed = EmoteId(1);
    let pack = PackDefinition {
        emotes: vec![
            emote("spare", vec![ms(500)], EmoteMode::Looping),
            emote("timed", durations, mode),
        ],
        ..pack()
    };
    let mut app = sim_with(Some(pack), |app| {
        app.init_resource::<Log<ShownEmote>>()
            .add_systems(FixedUpdate, record_emote.in_set(Phase::Present));
    });
    show_emote(&mut app, timed);
    ticks(&mut app, count);
    emote_runs(&app, timed)
}

/// `runs` of Frames that all show, as an Emote's runs.
fn all_shown(runs: Vec<(usize, usize)>) -> Vec<(Option<usize>, usize)> {
    runs.into_iter()
        .map(|(frame, count)| (Some(frame), count))
        .collect()
}

fn request(app: &mut App, request: EmoteRequest) {
    app.world_mut().write_message(request);
}

fn show_emote(app: &mut App, emote: EmoteId) {
    request(app, EmoteRequest::Show(emote));
}

fn hide_emote(app: &mut App, emote: EmoteId) {
    request(app, EmoteRequest::Hide(emote));
}

fn shown(emote: EmoteId, frame: usize) -> ShownEmote {
    ShownEmote(Some(EmoteFrame { emote, frame }))
}

fn playing(emote: EmoteId, played_ticks: u64) -> EmoteClock {
    EmoteClock(Some(PlayingEmote {
        emote,
        played_ticks,
        begun: true,
    }))
}

/// Brim's "love" played once from the tick that reads the request: 66, 67, 100, 400, 200 and 133 ms. The tick that
/// starts it shows its first Frame with no time played, and each later tick adds 33.3 ms. A Frame shows from the
/// first tick whose time has reached its start, so 66 ms takes 2 ticks, 67 ms 2, 100 ms 3, 400 ms 12, 200 ms 6 and
/// 133 ms 4. It goes on the tick its 966 ms have passed.
const LOVE_PLAYED_ONCE: [(Option<usize>, usize); 6] = [
    (Some(0), 2),
    (Some(1), 2),
    (Some(2), 3),
    (Some(3), 12),
    (Some(4), 6),
    (Some(5), 4),
];

#[test]
fn given_a_one_shot_emote_when_it_is_shown_then_each_frame_shows_for_its_own_duration_and_then_it_is_gone()
 {
    // Given
    let mut app = emoting();

    // When: 29 ticks of it, then 3 more
    show_emote(&mut app, LOVE);
    ticks(&mut app, 32);

    // Then
    let mut want = LOVE_PLAYED_ONCE.to_vec();
    want.push((None, 3));
    assert_eq!(emote_runs(&app, LOVE), want);
    assert_eq!(pet::<EmoteClock>(&mut app), EmoteClock(None));
    assert_eq!(pet::<ShownEmote>(&mut app), ShownEmote(None));
}

#[test]
fn given_an_emote_requested_when_the_tick_that_reads_the_request_runs_then_its_first_frame_shows_with_no_time_played()
 {
    // Given
    let mut app = emoting();
    show_emote(&mut app, TIRED);

    // When
    ticks(&mut app, 1);

    // Then: the Pet's first Frame shows the same way, before any time passes
    assert_eq!(pet::<ShownEmote>(&mut app), shown(TIRED, 0));
    assert_eq!(pet::<EmoteClock>(&mut app), playing(TIRED, 0));
}

#[test]
fn given_a_one_shot_emote_of_the_shortest_frame_a_pack_allows_when_it_is_shown_then_it_shows() {
    // Given: 34 ms, a little over a 33.3 ms tick
    let mut app = emoting();

    // When
    show_emote(&mut app, BLINK);
    ticks(&mut app, 3);

    // Then: at 0 ms and 33.3 ms
    assert_eq!(emote_frames(&app, BLINK), [Some(0), Some(0), None]);
}

#[test]
fn given_a_one_shot_emote_with_a_34_ms_frame_in_the_middle_when_it_is_shown_then_that_frame_shows_for_a_tick()
 {
    // Given: 50, 34 and 100 ms, so the middle Frame runs from 50 to 84 ms, and only the tick at 66.7 ms falls inside it
    let mut app = emoting();

    // When
    show_emote(&mut app, GLANCE);
    ticks(&mut app, 8);

    // Then
    assert_eq!(
        emote_runs(&app, GLANCE),
        [(Some(0), 2), (Some(1), 1), (Some(2), 3), (None, 2)]
    );
}

#[test]
fn given_a_one_shot_emote_of_whole_ticks_when_it_is_shown_then_each_frame_shows_for_exactly_its_ticks_then_it_goes()
 {
    // Given: 100 ms then 200 ms, exactly 3 and 6 ticks
    let durations = vec![ms(100), ms(200)];

    // When
    let runs = emote_played(durations, EmoteMode::OneShot, 11);

    // Then
    assert_eq!(runs, [(Some(0), 3), (Some(1), 6), (None, 2)]);
}

#[test]
fn given_a_looping_emote_with_brims_tired_timings_when_four_loops_pass_then_each_frame_shows_for_exactly_ten_ticks()
 {
    // Given: 333, 333, 334 and 333 ms, as in the Pet's test
    let durations = vec![ms(333), ms(333), ms(334), ms(333)];

    // When
    let runs = emote_played(durations, EmoteMode::Looping, 4 * 40);

    // Then
    let one_loop = [(0, 10), (1, 10), (2, 10), (3, 10)];
    assert_eq!(runs, all_shown(repeated(&one_loop, 4)));
}

#[test]
fn given_a_looping_emote_with_brims_annoyed_timings_when_three_loops_pass_then_each_frame_shows_for_its_exact_ticks()
 {
    // Given: 200, 133 and 200 ms, as in the Pet's test
    let durations = vec![ms(200), ms(133), ms(200)];

    // When
    let runs = emote_played(durations, EmoteMode::Looping, 3 * 16);

    // Then
    let one_loop = [(0, 6), (1, 4), (2, 6)];
    assert_eq!(runs, all_shown(repeated(&one_loop, 3)));
}

#[test]
fn given_a_looping_emote_when_it_is_shown_then_it_loops_until_it_is_hidden() {
    // Given
    let mut app = emoting();
    show_emote(&mut app, TIRED);

    // When: eleven ticks is one loop and a little more, then it plays on for a minute
    ticks(&mut app, 11);
    let first_loop = emote_frames(&app, TIRED);
    ticks(&mut app, 30 * 60);
    let after_a_minute = pet::<ShownEmote>(&mut app);
    hide_emote(&mut app, TIRED);
    ticks(&mut app, 1);

    // Then: 3 ticks is exactly 100 ms. A minute and 10 ticks played is 200 loops and 33 ms.
    let (one, two) = (Some(0), Some(1));
    assert_eq!(
        first_loop,
        [one, one, one, two, two, two, two, two, two, one, one]
    );
    assert_eq!(after_a_minute, shown(TIRED, 0));
    assert_eq!(pet::<ShownEmote>(&mut app), ShownEmote(None));
    assert_eq!(pet::<EmoteClock>(&mut app), EmoteClock(None));
}

#[test]
fn given_a_looping_emote_on_its_second_frame_when_another_emote_is_shown_then_it_replaces_it_from_its_first_frame()
 {
    // Given
    let mut app = emoting();
    show_emote(&mut app, TIRED);
    ticks(&mut app, 5);
    assert_eq!(pet::<ShownEmote>(&mut app), shown(TIRED, 1));

    // When
    show_emote(&mut app, LOVE);
    ticks(&mut app, 1);

    // Then
    assert_eq!(pet::<ShownEmote>(&mut app), shown(LOVE, 0));
    assert_eq!(pet::<EmoteClock>(&mut app), playing(LOVE, 0));
}

#[test]
fn given_a_looping_emote_showing_when_it_is_shown_again_then_it_carries_on_without_starting_over() {
    // Given: 4 ticks played, on its second Frame
    let mut app = emoting();
    show_emote(&mut app, TIRED);
    ticks(&mut app, 5);

    // When
    show_emote(&mut app, TIRED);
    ticks(&mut app, 1);

    // Then
    assert_eq!(pet::<ShownEmote>(&mut app), shown(TIRED, 1));
    assert_eq!(pet::<EmoteClock>(&mut app), playing(TIRED, 5));
}

#[test]
fn given_a_one_shot_emote_on_its_third_frame_when_it_is_shown_again_then_it_starts_over() {
    // Given
    let mut app = emoting();
    show_emote(&mut app, LOVE);
    ticks(&mut app, 5);
    assert_eq!(pet::<ShownEmote>(&mut app), shown(LOVE, 2));

    // When
    show_emote(&mut app, LOVE);
    ticks(&mut app, 1);

    // Then
    assert_eq!(pet::<ShownEmote>(&mut app), shown(LOVE, 0));
    assert_eq!(pet::<EmoteClock>(&mut app), playing(LOVE, 0));
}

#[test]
fn given_a_hidden_pet_with_a_one_shot_emote_when_long_past_its_end_then_its_clock_holds_and_it_carries_on_after_waking()
 {
    // Given: 4 ticks played, on its third Frame, with most of its 966 ms left
    let mut app = emoting();
    show_emote(&mut app, LOVE);
    ticks(&mut app, 5);
    hide(&mut app);
    let before = pet::<EmoteClock>(&mut app);

    // When: an hour hidden, then shown again for one tick
    for _ in 0..360 {
        step(&mut app, Duration::from_secs(10));
    }
    let while_hidden = (pet::<EmoteClock>(&mut app), pet::<ShownEmote>(&mut app));
    app.world_mut().resource_mut::<Hidden>().0 = false;
    step(&mut app, Duration::from_secs(10));
    ticks(&mut app, 1);

    // Then
    assert_eq!(while_hidden, (before, shown(LOVE, 2)));
    assert_eq!(pet::<EmoteClock>(&mut app), playing(LOVE, 5));
}

#[test]
fn given_a_hidden_pet_when_an_emote_is_requested_then_it_plays_exactly_once_after_waking() {
    // Given
    let mut app = emoting();
    hide(&mut app);

    // When: requested while hidden, an hour passes, then it's shown again for 32 ticks
    show_emote(&mut app, LOVE);
    for _ in 0..360 {
        step(&mut app, Duration::from_secs(10));
    }
    let while_hidden = pet::<ShownEmote>(&mut app);
    app.world_mut().resource_mut::<Hidden>().0 = false;
    step(&mut app, Duration::from_secs(10));
    ticks(&mut app, 32);

    // Then: no tick ran while hidden, so the request waited for the first one after
    assert_eq!(while_hidden, ShownEmote(None));
    let mut want = LOVE_PLAYED_ONCE.to_vec();
    want.push((None, 3));
    assert_eq!(emote_runs(&app, LOVE), want);
}

#[test]
fn given_a_looping_emote_showing_when_a_different_emote_is_hidden_then_nothing_changes() {
    // Given: 2 ticks played
    let mut app = emoting();
    show_emote(&mut app, TIRED);
    ticks(&mut app, 3);

    // When
    hide_emote(&mut app, LOVE);
    ticks(&mut app, 1);

    // Then: 3 ticks is exactly 100 ms, when its second Frame starts
    assert_eq!(pet::<EmoteClock>(&mut app), playing(TIRED, 3));
    assert_eq!(pet::<ShownEmote>(&mut app), shown(TIRED, 1));
}

#[test]
fn given_an_emote_shown_then_hidden_before_the_same_tick_when_it_runs_then_the_emote_is_hidden() {
    // Given
    let mut app = emoting();
    show_emote(&mut app, TIRED);
    hide_emote(&mut app, TIRED);

    // When
    ticks(&mut app, 3);

    // Then
    assert_eq!(emote_frames(&app, TIRED), [None, None, None]);
    assert_eq!(pet::<EmoteClock>(&mut app), EmoteClock(None));
}

#[test]
fn given_a_looping_emote_showing_when_it_is_hidden_then_shown_before_the_same_tick_then_it_shows_from_the_start()
 {
    // Given: on its second Frame
    let mut app = emoting();
    show_emote(&mut app, TIRED);
    ticks(&mut app, 5);

    // When
    hide_emote(&mut app, TIRED);
    show_emote(&mut app, TIRED);
    ticks(&mut app, 1);

    // Then
    assert_eq!(pet::<ShownEmote>(&mut app), shown(TIRED, 0));
    assert_eq!(pet::<EmoteClock>(&mut app), playing(TIRED, 0));
}

#[test]
fn given_several_emotes_shown_before_the_same_tick_when_it_runs_then_the_last_one_shows() {
    // Given: the last one has the lower id, so neither the first request nor the first Emote wins by chance
    let mut app = emoting();
    show_emote(&mut app, GLANCE);
    show_emote(&mut app, LOVE);
    show_emote(&mut app, TIRED);

    // When
    ticks(&mut app, 1);

    // Then
    assert_eq!(pet::<ShownEmote>(&mut app), shown(TIRED, 0));
    assert_eq!(pet::<EmoteClock>(&mut app), playing(TIRED, 0));
}

#[test]
fn given_a_pack_without_emotes_when_an_emote_is_shown_then_none_shows() {
    // Given
    let mut app = sim();

    // When
    show_emote(&mut app, EmoteId(0));
    ticks(&mut app, 3);

    // Then
    assert_eq!(pet::<ShownEmote>(&mut app), ShownEmote(None));
    assert_eq!(pet::<EmoteClock>(&mut app), EmoteClock(None));
}

/// The test Pack after an edit: "hatchling" is now the first Form, with a new first Animation, so its ids have all
/// moved. Its default Action is "idle", its second, which plays 200 ms three times. It still starts as a "hatchling".
fn edited_pack() -> PackDefinition {
    let hatchling = form(
        "hatchling",
        vec![action("wave", 0), action("idle", 1)],
        vec![
            animation("wave", vec![ms(100)]),
            animation("idle", vec![ms(200), ms(200), ms(200)]),
        ],
    );
    let egg = form(
        "egg",
        vec![action("idle", 0)],
        vec![animation("idle", vec![ms(500)])],
    );
    PackDefinition {
        name: "Edited".to_owned(),
        starting_form: FormId(0),
        forms: vec![hatchling, egg],
        emotes: Vec::new(),
        hit_mask_threshold: 128,
    }
}

/// A Pack with no "hatchling": an "egg", then an "adult" it starts as, whose default Action plays 300 ms.
fn pack_without_hatchling() -> PackDefinition {
    let egg = form(
        "egg",
        vec![action("idle", 0)],
        vec![animation("idle", vec![ms(500)])],
    );
    let adult = form(
        "adult",
        vec![action("idle", 0)],
        vec![animation("idle", vec![ms(300)])],
    );
    PackDefinition {
        name: "Grown up".to_owned(),
        starting_form: FormId(1),
        forms: vec![egg, adult],
        emotes: Vec::new(),
        hit_mask_threshold: 128,
    }
}

/// Swaps the Pack between updates, as the app does when an edited Pack has loaded.
fn replace(app: &mut App, pack: PackDefinition) {
    replace_pack(app.world_mut(), Arc::new(pack));
}

#[derive(Resource, Default)]
struct Reloads(usize);

fn count_reloads(mut reloads: MessageReader<PackReloaded>, mut count: ResMut<Reloads>) {
    count.0 += reloads.read().count();
}

/// The simulation, counting the `PackReloaded` messages a system in each tick reads.
fn counting_reloads() -> App {
    sim_with(Some(pack()), |app| {
        app.init_resource::<Reloads>()
            .add_systems(FixedUpdate, count_reloads.in_set(Phase::Sense));
    })
}

fn reloads(app: &App) -> usize {
    app.world().resource::<Reloads>().0
}

#[test]
fn given_a_pet_moved_and_turned_when_the_pack_is_replaced_then_it_keeps_its_position_and_facing() {
    // Given
    let mut app = sim();
    let away = Position(Point { x: 321, y: 456 });
    let world = app.world_mut();
    let entity = world
        .query_filtered::<Entity, With<Pet>>()
        .single(world)
        .unwrap();
    world
        .entity_mut(entity)
        .insert((away, Faces(Facing::Mirrored)));

    // When
    replace(&mut app, edited_pack());
    ticks(&mut app, 2);

    // Then
    assert_eq!(pet::<Position>(&mut app), away);
    assert_eq!(pet::<Faces>(&mut app), Faces(Facing::Mirrored));
}

#[test]
fn given_a_pet_in_its_starting_form_when_an_edit_renumbers_that_form_then_it_stays_in_it_by_name() {
    // Given: a "hatchling", the old Pack's second Form and its starting Form
    let mut app = sim();
    ticks(&mut app, 4);

    // When: the edited Pack's "hatchling" is its first Form, and still its starting Form
    replace(&mut app, edited_pack());

    // Then: on the first Frame of that Form's default Action, "idle", now its second Animation
    assert_eq!(
        pet::<ShownFrame>(&mut app),
        ShownFrame {
            form: FormId(0),
            animation: AnimationId(1),
            frame: 0
        }
    );
    assert_eq!(app.world().resource::<CurrentPack>().0.name, "Edited");
}

/// Puts the Pet in `form`, on the first Frame of `animation`, as growing will in roadmap item 4.
fn put_pet_in(app: &mut App, form: FormId, animation: AnimationId) {
    let world = app.world_mut();
    let entity = world
        .query_filtered::<Entity, With<Pet>>()
        .single(world)
        .unwrap();
    world.entity_mut(entity).insert((
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
    ));
}

#[test]
fn given_a_pet_in_a_form_that_is_not_the_starting_one_when_an_edit_removes_that_form_then_it_takes_the_starting_form()
 {
    // Given: an "egg", which isn't the starting Form
    let mut app = sim();
    put_pet_in(&mut app, FormId(0), AnimationId(0));

    // When: the edit has no "egg", and an "adult" comes first, so the starting Form, "hatchling", is the second
    let mut edited = pack();
    edited.forms[0] = form(
        "adult",
        vec![action("idle", 0)],
        vec![animation("idle", vec![ms(300)])],
    );
    replace(&mut app, edited);

    // Then: a "hatchling" on the first Frame of its default Action, "sleep"
    assert_eq!(
        pet::<ShownFrame>(&mut app),
        ShownFrame {
            form: FormId(1),
            animation: AnimationId(1),
            frame: 0
        }
    );
}

/// The test Pack with only the hatchling's default Action changed, to "idle", which plays 250 ms twice.
fn pack_idling() -> PackDefinition {
    let mut pack = pack();
    pack.forms[1].default_action = ActionId(0);
    pack
}

/// The test Pack with only its starting Form changed, to "egg", the first Form, whose default Action plays 500 ms.
fn pack_starting_as_egg() -> PackDefinition {
    PackDefinition {
        starting_form: FormId(0),
        ..pack()
    }
}

/// The simulation, keeping the Frame the Pet shows at the end of each tick.
fn recording_frames() -> App {
    sim_with(Some(pack()), |app| {
        app.init_resource::<Log<ShownFrame>>()
            .add_systems(FixedUpdate, record_frame.in_set(Phase::Present));
    })
}

fn frames_since_cleared(app: &App) -> Vec<(AnimationId, usize)> {
    app.world()
        .resource::<Log<ShownFrame>>()
        .0
        .iter()
        .map(|shown| (shown.animation, shown.frame))
        .collect()
}

#[test]
fn given_a_pet_playing_its_default_action_when_an_edit_changes_only_that_forms_default_action_then_it_plays_the_new_one_with_its_own_timing()
 {
    // Given: a "hatchling" 4 ticks into "sleep", on its second Frame
    let mut app = recording_frames();
    ticks(&mut app, 4);
    assert_eq!(pet::<ShownFrame>(&mut app).animation, AnimationId(1));

    // When: "idle" is two Frames of 7.5 ticks
    replace(&mut app, pack_idling());
    let on_swapping = pet::<AnimationClock>(&mut app);
    app.world_mut().resource_mut::<Log<ShownFrame>>().0.clear();
    ticks(&mut app, 16);

    // Then: from Frame 0 before any tick, so ticks 1 to 7 are before 250 ms, 8 to 14 before 500 ms, and 15 starts
    // the second loop
    assert_eq!(
        on_swapping,
        AnimationClock {
            form: FormId(1),
            animation: AnimationId(0),
            playback: Playback::Looping,
            played_ticks: 0
        }
    );
    let idle = AnimationId(0);
    assert_eq!(
        runs(frames_since_cleared(&app)),
        [((idle, 0), 7), ((idle, 1), 7), ((idle, 0), 2)]
    );
}

#[test]
fn given_a_pet_still_in_its_starting_form_when_an_edit_changes_the_starting_form_then_it_takes_the_new_one_from_its_first_frame()
 {
    // Given: a "hatchling", the starting Form, 4 ticks into "sleep"
    let mut app = sim();
    ticks(&mut app, 4);

    // When: the edit starts as an "egg" instead, which it also had before
    replace(&mut app, pack_starting_as_egg());

    // Then: an "egg" on the first Frame of its default Action, before any time has passed
    assert_eq!(
        pet::<AnimationClock>(&mut app),
        AnimationClock {
            form: FormId(0),
            animation: AnimationId(0),
            playback: Playback::Looping,
            played_ticks: 0
        }
    );
    assert_eq!(
        pet::<ShownFrame>(&mut app),
        ShownFrame {
            form: FormId(0),
            animation: AnimationId(0),
            frame: 0
        }
    );
}

#[test]
fn given_a_pet_that_has_left_its_starting_form_when_an_edit_changes_the_starting_form_then_it_keeps_its_form_by_name()
 {
    // Given: an "egg", which isn't the starting Form, as a Pet that has grown will be in roadmap item 4
    let mut app = sim();
    put_pet_in(&mut app, FormId(0), AnimationId(0));

    // When: the edit adds an "adult" before the others and starts as one, so the "egg" is now the second Form, and
    // its old number is the new starting Form's
    let mut edited = pack();
    edited.forms.insert(
        0,
        form(
            "adult",
            vec![action("idle", 0)],
            vec![animation("idle", vec![ms(300)])],
        ),
    );
    edited.starting_form = FormId(0);
    replace(&mut app, edited);

    // Then: still an "egg", on its default Action
    assert_eq!(
        pet::<ShownFrame>(&mut app),
        ShownFrame {
            form: FormId(1),
            animation: AnimationId(0),
            frame: 0
        }
    );
}

#[test]
fn given_an_actor_that_is_not_the_pet_in_the_starting_form_when_an_edit_changes_the_starting_form_then_only_the_pet_follows_it()
 {
    // Given: another Actor beside the Pet, both "hatchlings", the starting Form. The starting Form is the Pet's alone.
    let mut app = sim();
    let hatchling = (FormId(1), AnimationId(1));
    let actor = app
        .world_mut()
        .spawn((
            AnimationClock {
                form: hatchling.0,
                animation: hatchling.1,
                playback: Playback::Looping,
                played_ticks: 0,
            },
            ShownFrame {
                form: hatchling.0,
                animation: hatchling.1,
                frame: 0,
            },
        ))
        .id();

    // When: the edit starts as an "egg" instead
    replace(&mut app, pack_starting_as_egg());

    // Then: the Pet is an "egg", and the other Actor keeps its Form by name
    assert_eq!(pet::<ShownFrame>(&mut app).form, FormId(0));
    assert_eq!(
        app.world().get::<ShownFrame>(actor).unwrap().form,
        FormId(1)
    );
}

#[test]
fn given_a_pet_part_way_through_its_animation_when_the_pack_is_replaced_then_its_clock_starts_again_on_the_default_action()
 {
    // Given: 4 ticks in, on the second Frame of "sleep"
    let mut app = sim_with(Some(pack()), |app| {
        app.init_resource::<Log<ShownFrame>>()
            .add_systems(FixedUpdate, record_frame.in_set(Phase::Present));
    });
    ticks(&mut app, 4);
    assert_eq!(pet::<ShownFrame>(&mut app).frame, 1);

    // When: the edited "idle" is three Frames of exactly 6 ticks
    replace(&mut app, edited_pack());
    let on_swapping = pet::<AnimationClock>(&mut app);
    app.world_mut().resource_mut::<Log<ShownFrame>>().0.clear();
    ticks(&mut app, 13);

    // Then: the swap shows Frame 0 before any tick, as a Pet appearing does, so it has 5 more ticks of it
    assert_eq!(
        on_swapping,
        AnimationClock {
            form: FormId(0),
            animation: AnimationId(1),
            playback: Playback::Looping,
            played_ticks: 0
        }
    );
    let log = &app.world().resource::<Log<ShownFrame>>().0;
    assert_eq!(
        runs(log.iter().map(|shown| shown.frame)),
        [(0, 5), (1, 6), (2, 2)]
    );
}

#[test]
fn given_a_pet_showing_an_emote_when_the_pack_is_replaced_by_one_with_the_same_emotes_then_the_emote_ends()
 {
    // Given: "tired" on its second Frame
    let mut app = emoting();
    show_emote(&mut app, TIRED);
    ticks(&mut app, 5);
    assert_eq!(pet::<ShownEmote>(&mut app), shown(TIRED, 1));

    // When
    replace(&mut app, pack_with_emotes());
    let on_swapping = pet::<ShownEmote>(&mut app);
    ticks(&mut app, 3);

    // Then
    assert_eq!(on_swapping, ShownEmote(None));
    assert_eq!(emote_frames(&app, TIRED)[5..], [None, None, None]);
    assert_eq!(pet::<EmoteClock>(&mut app), EmoteClock(None));
}

#[test]
fn given_an_emote_requested_against_the_old_pack_when_the_pack_is_replaced_before_a_tick_reads_it_then_it_is_dropped()
 {
    // Given: the request names the old Pack's Emote by its number, which may be another Emote in the new Pack
    let mut app = emoting();
    show_emote(&mut app, LOVE);

    // When
    replace(&mut app, pack_with_emotes());
    ticks(&mut app, 3);

    // Then
    assert_eq!(emote_frames(&app, LOVE), [None, None, None]);
}

#[test]
fn given_a_shown_pet_when_the_pack_is_replaced_then_a_tick_reads_pack_reloaded_exactly_once() {
    // Given
    let mut app = counting_reloads();

    // When
    replace(&mut app, edited_pack());
    ticks(&mut app, 10);

    // Then
    assert_eq!(reloads(&app), 1);
}

#[test]
fn given_a_hidden_pet_when_the_pack_is_replaced_then_it_is_swapped_at_once_and_ticks_read_pack_reloaded_once_after_waking()
 {
    // Given
    let mut app = counting_reloads();
    ticks(&mut app, 4);
    hide(&mut app);

    // When: replaced while Hidden, an hour passes, then it's shown again for 10 ticks
    replace(&mut app, edited_pack());
    let swapped = pet::<ShownFrame>(&mut app);
    for _ in 0..360 {
        step(&mut app, Duration::from_secs(10));
    }
    let while_hidden = (reloads(&app), pet::<AnimationClock>(&mut app).played_ticks);
    app.world_mut().resource_mut::<Hidden>().0 = false;
    step(&mut app, Duration::from_secs(10));
    ticks(&mut app, 10);

    // Then: no tick ran while Hidden, so the message waited for the first one after
    assert_eq!(swapped.form, FormId(0));
    assert_eq!(while_hidden, (0, 0));
    assert_eq!(reloads(&app), 1);
    assert_eq!(pet::<AnimationClock>(&mut app).played_ticks, 10);
}

#[test]
fn given_a_pet_drawn_from_the_old_pack_when_an_identical_pack_replaces_it_then_its_shown_frame_counts_as_changed()
 {
    // Given: every number in the new Pack is the same, so only the swap itself can tell readers to draw again
    #[derive(Resource, Default)]
    struct Changes(usize);
    let mut app = sim_with(Some(pack()), |app| {
        app.init_resource::<Changes>().add_systems(
            Update,
            |pets: Query<(), (With<Pet>, Changed<ShownFrame>)>, mut changes: ResMut<Changes>| {
                changes.0 += pets.iter().count();
            },
        );
    });
    step(&mut app, Duration::ZERO);
    let before = app.world().resource::<Changes>().0;

    // When
    replace(&mut app, pack());
    step(&mut app, Duration::ZERO);

    // Then
    assert_eq!(app.world().resource::<Changes>().0, before + 1);
}

#[test]
fn given_a_pet_deep_in_a_big_pack_when_a_pack_with_one_frame_and_no_emotes_replaces_it_then_ticks_carry_on_without_panicking()
 {
    // Given: a "hatchling" on the second Frame of its second Animation, playing the fifth of five Emotes
    let mut app = emoting();
    show_emote(&mut app, GLANCE);
    ticks(&mut app, 4);
    assert_eq!(pet::<ShownFrame>(&mut app).frame, 1);
    assert!(pet::<ShownEmote>(&mut app).0.is_some());
    let tiny = PackDefinition {
        name: "Tiny".to_owned(),
        starting_form: FormId(0),
        forms: vec![form(
            "egg",
            vec![action("idle", 0)],
            vec![animation("idle", vec![ms(34)])],
        )],
        emotes: Vec::new(),
        hit_mask_threshold: 128,
    };

    // When: the old Pack's Emote is asked for again, which this one doesn't have
    replace(&mut app, tiny);
    show_emote(&mut app, GLANCE);
    ticks(&mut app, 30);

    // Then
    assert_eq!(
        pet::<ShownFrame>(&mut app),
        ShownFrame {
            form: FormId(0),
            animation: AnimationId(0),
            frame: 0
        }
    );
    assert_eq!(pet::<ShownEmote>(&mut app), ShownEmote(None));
}

#[test]
fn given_no_pack_when_a_pack_arrives_then_the_pet_appears_at_home_in_its_starting_form() {
    // Given: the Pack was broken when the app started
    let mut app = sim_with(None, |_| {});

    // When
    replace(&mut app, pack_without_hatchling());
    step(&mut app, Duration::ZERO);

    // Then: an "adult"
    assert_eq!(
        pet::<Position>(&mut app),
        Position(Point { x: 600, y: 800 })
    );
    assert_eq!(pet::<ShownFrame>(&mut app).form, FormId(1));
}
