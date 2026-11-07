//! The Pet's body in the world: standing, being grabbed, swung and thrown, falling, landing, and the Situations it
//! plays on the way. These prove rules, such as "it never passes through an Obstacle", never feel numbers.

use std::time::Instant;

use pet_core::gesture::Button;
use pet_core::pack::Situation;
use pet_core::physics::{Material, Obstacle};
use pet_sim::{ButtonChanged, Held, InSituation, Landed, MouseLost, PointerMoved, WorkArea};

use super::*;

/// Home: the bottom centre of the work area, on the Ground plane.
const HOME: Point = Point { x: 600, y: 800 };

/// A point on the Pet's Body at home. At twice its size it is 20 x 12 physical pixels, from 6 left of the Anchor to
/// 14 right of it, and up to 12 above it.
const ON_THE_PET: Point = Point { x: 600, y: 794 };

/// How far the Body reaches from the Anchor, in physical pixels: left, up and right.
const BODY_LEFT: i32 = 6;
const BODY_UP: i32 = 12;
const BODY_RIGHT: i32 = 14;

/// The test Pack, with Animations for the starting Form's Situations.
fn pack_with_situations() -> PackDefinition {
    let mut pack = pack();
    let hatchling = &mut pack.forms[1];
    for (situation, name) in [
        (Situation::Held, "held"),
        (Situation::Falling, "falling"),
        (Situation::Landing, "landing"),
    ] {
        hatchling
            .situations
            .push((situation, AnimationId(hatchling.animations.len())));
        hatchling
            .animations
            .push(animation(name, vec![ms(100), ms(100)]));
    }
    pack
}

const HELD: AnimationId = AnimationId(2);
const FALLING: AnimationId = AnimationId(3);
const LANDING: AnimationId = AnimationId(4);
const DEFAULT: AnimationId = AnimationId(1);

/// The owner's hand: where the cursor is, and the time, which follows the ticks.
struct Hand {
    now: Instant,
    cursor: Point,
}

impl Hand {
    fn at(cursor: Point) -> Self {
        Self {
            now: Instant::now(),
            cursor,
        }
    }

    fn button(&self, app: &mut App, pressed: bool) {
        app.world_mut().write_message(ButtonChanged {
            at: self.now,
            button: Button::Left,
            pressed,
            position: self.cursor,
        });
    }

    fn press(&mut self, app: &mut App) {
        self.button(app, true);
    }

    fn release(&mut self, app: &mut App) {
        self.button(app, false);
    }

    /// Windows takes the mouse from the Overlay, as for Alt-Tab during a drag, so the release goes to another window.
    fn lose_the_mouse(&mut self, app: &mut App) {
        app.world_mut().write_message(MouseLost {
            at: self.now,
            position: self.cursor,
        });
    }

    /// Moves the cursor to `to` in a straight line over one tick, seen every 8 ms or so as the hit test sees it, and
    /// runs that tick.
    fn move_to(&mut self, app: &mut App, to: Point) {
        let from = self.cursor;
        let tick = tick_length(app);
        for sample in 1..=4 {
            let along = |a: i32, b: i32| a + (b - a) * sample / 4;
            self.cursor = Point {
                x: along(from.x, to.x),
                y: along(from.y, to.y),
            };
            app.world_mut().write_message(PointerMoved {
                at: self.now + tick * sample as u32 / 4,
                position: self.cursor,
                over_pet: false,
            });
        }
        self.now += tick;
        ticks(app, 1);
    }

    /// Keeps the cursor where it is for `count` ticks.
    fn wait(&mut self, app: &mut App, count: u32) {
        self.now += tick_length(app) * count;
        ticks(app, count);
    }
}

fn record_landing(mut landings: MessageReader<Landed>, mut log: ResMut<Log<Landed>>) {
    log.0.extend(landings.read().copied());
}

fn record_position(positions: Query<&Position, With<Pet>>, mut log: ResMut<Log<Position>>) {
    log.0.extend(positions.iter().copied());
}

/// The simulation with `pack`, keeping a log of the Pet's Landings, Positions and Frames, tick by tick.
fn watched_with(plugin: SimPlugin, pack: PackDefinition) -> App {
    let mut app = unstarted_with(plugin, Some(pack), |app| {
        app.init_resource::<Log<Landed>>()
            .init_resource::<Log<Position>>()
            .init_resource::<Log<ShownFrame>>()
            .add_systems(
                FixedUpdate,
                (record_landing, record_position, record_frame).in_set(Phase::Present),
            );
    });
    app.update();
    app
}

fn watched() -> App {
    watched_with(plugin(), pack_with_situations())
}

fn landings(app: &App) -> usize {
    app.world().resource::<Log<Landed>>().0.len()
}

fn positions(app: &App) -> Vec<Point> {
    app.world()
        .resource::<Log<Position>>()
        .0
        .iter()
        .map(|p| p.0)
        .collect()
}

/// The Animations the Pet played, each once for each time in a row it played.
fn animations_played(app: &App) -> Vec<AnimationId> {
    runs(
        app.world()
            .resource::<Log<ShownFrame>>()
            .0
            .iter()
            .map(|frame| frame.animation),
    )
    .into_iter()
    .map(|(animation, _)| animation)
    .collect()
}

/// Grabs the Pet at home and lifts it by the string, up to `to`, then holds it still there for a second.
fn lifted_to(app: &mut App, to: Point) -> Hand {
    let mut hand = Hand::at(ON_THE_PET);
    hand.press(app);
    hand.wait(app, 1);
    let steps = 10;
    for step in 1..=steps {
        let along = |a: i32, b: i32| a + (b - a) * step / steps;
        hand.move_to(
            app,
            Point {
                x: along(ON_THE_PET.x, to.x),
                y: along(ON_THE_PET.y, to.y),
            },
        );
    }
    hand.wait(app, 30);
    hand
}

fn stands_on(app: &mut App, ground: i32) -> bool {
    pet::<Position>(app).0.y == ground && pet::<InSituation>(app) == InSituation(None)
}

#[test]
fn given_the_simulation_starts_when_time_passes_then_the_pet_stands_still_on_the_ground_plane_in_no_situation()
 {
    // Given
    let mut app = watched();

    // When
    ticks(&mut app, 60);

    // Then
    assert!(positions(&app).iter().all(|position| *position == HOME));
    assert_eq!(pet::<InSituation>(&mut app), InSituation(None));
    assert_eq!(landings(&app), 0);
    assert_eq!(animations_played(&app), [DEFAULT]);
}

#[test]
fn given_the_pet_grabbed_when_the_cursor_stays_close_then_the_string_is_slack_and_the_pet_stays_on_the_ground()
 {
    // Given
    let mut app = watched();
    let mut hand = Hand::at(ON_THE_PET);

    // When: a press, as a poke begins, and the cursor barely moves
    hand.press(&mut app);
    hand.wait(&mut app, 3);
    hand.move_to(&mut app, Point { x: 604, y: 790 });
    hand.wait(&mut app, 10);

    // Then
    assert!(positions(&app).iter().all(|position| *position == HOME));
    assert_eq!(
        pet::<InSituation>(&mut app),
        InSituation(Some(Situation::Held))
    );
}

#[test]
fn given_the_pet_held_up_in_the_air_when_time_passes_then_it_hangs_below_the_cursor_playing_its_held_animation()
 {
    // Given
    let mut app = watched();

    // When
    let hand = lifted_to(&mut app, Point { x: 600, y: 400 });

    // Then: it hangs below the cursor, its Anchor somewhere under the grab point, and not on the ground
    let anchor = pet::<Position>(&mut app).0;
    assert!(anchor.y > hand.cursor.y && anchor.y < 600, "{anchor:?}");
    assert!((anchor.x - hand.cursor.x).abs() <= 2, "{anchor:?}");
    assert_eq!(
        pet::<InSituation>(&mut app),
        InSituation(Some(Situation::Held))
    );
    assert_eq!(pet::<AnimationClock>(&mut app).animation, HELD);
    assert_eq!(landings(&app), 0);
}

#[test]
fn given_the_pet_held_up_in_the_air_when_the_owner_lets_go_then_it_falls_lands_once_and_stands_on_the_ground_plane()
 {
    // Given
    let mut app = watched();
    let mut hand = lifted_to(&mut app, Point { x: 600, y: 400 });

    // When
    hand.release(&mut app);
    hand.wait(&mut app, 90);

    // Then
    assert!(stands_on(&mut app, WORK_AREA.bottom));
    assert_eq!(landings(&app), 1);
    assert!(app.world().resource::<Log<Landed>>().0[0].speed > 0.0);
    assert!(
        app.world_mut()
            .query::<&Held>()
            .iter(app.world())
            .next()
            .is_none()
    );
}

#[test]
fn given_a_falling_pet_with_a_landing_animation_when_it_lands_then_that_plays_once_and_the_default_action_plays_again()
 {
    // Given
    let mut app = watched();
    let mut hand = lifted_to(&mut app, Point { x: 600, y: 400 });

    // When
    hand.release(&mut app);
    hand.wait(&mut app, 90);

    // Then
    assert_eq!(animations_played(&app), [HELD, FALLING, LANDING, DEFAULT]);
}

#[test]
fn given_a_pet_flung_as_hard_as_the_owner_can_in_any_direction_when_it_flies_then_it_never_leaves_the_work_area()
 {
    // Given: flings up, left, right, and up into each top corner, far faster than the release speed cap
    let directions = [(0, -1), (-1, 0), (1, 0), (-1, -1), (1, -1)];

    for (dx, dy) in directions {
        let mut app = watched();
        let mut hand = Hand::at(ON_THE_PET);
        hand.press(&mut app);
        hand.wait(&mut app, 1);

        // When
        for step in 1..=3 {
            hand.move_to(
                &mut app,
                Point {
                    x: ON_THE_PET.x + dx * 300 * step,
                    y: ON_THE_PET.y + dy * 300 * step,
                },
            );
        }
        hand.release(&mut app);
        hand.wait(&mut app, 150);

        // Then
        for anchor in positions(&app) {
            assert!(
                anchor.x - BODY_LEFT >= WORK_AREA.left,
                "{dx},{dy}: {anchor:?}"
            );
            assert!(
                anchor.x + BODY_RIGHT <= WORK_AREA.right,
                "{dx},{dy}: {anchor:?}"
            );
            assert!(anchor.y - BODY_UP >= WORK_AREA.top, "{dx},{dy}: {anchor:?}");
            assert!(anchor.y <= WORK_AREA.bottom, "{dx},{dy}: {anchor:?}");
        }
        assert!(stands_on(&mut app, WORK_AREA.bottom), "{dx},{dy}");
    }
}

#[test]
fn given_a_thin_obstacle_when_the_pet_is_flung_at_it_as_hard_as_the_owner_can_then_it_never_passes_through()
 {
    // Given: a wall 4 pixels thin, 100 pixels right of the Pet, from the top of the work area to the ground
    let wall = Rect {
        left: 714,
        top: 0,
        right: 718,
        bottom: 800,
    };
    let mut app = watched_with(
        SimPlugin {
            obstacles: vec![Obstacle {
                rect: wall,
                material: Material {
                    friction: 0.5,
                    bounciness: 0.5,
                },
            }],
            ..plugin()
        },
        pack_with_situations(),
    );
    let mut hand = Hand::at(ON_THE_PET);
    hand.press(&mut app);
    hand.wait(&mut app, 1);

    // When: flung right, low and slightly up, much faster than the cap
    for step in 1..=3 {
        hand.move_to(
            &mut app,
            Point {
                x: ON_THE_PET.x + 250 * step,
                y: ON_THE_PET.y - 20 * step,
            },
        );
    }
    hand.release(&mut app);
    hand.wait(&mut app, 150);

    // Then: its right edge never got past the wall's left side
    for anchor in positions(&app) {
        assert!(anchor.x + BODY_RIGHT <= wall.left, "{anchor:?}");
    }
    assert!(stands_on(&mut app, WORK_AREA.bottom));
}

#[test]
fn given_an_obstacle_under_the_pet_when_it_is_dropped_then_it_lands_and_stands_on_its_top() {
    // Given: a shelf the Pet hangs above
    let shelf = Rect {
        left: 500,
        top: 600,
        right: 700,
        bottom: 620,
    };
    let mut app = watched_with(
        SimPlugin {
            obstacles: vec![Obstacle {
                rect: shelf,
                material: Material {
                    friction: 0.8,
                    bounciness: 0.0,
                },
            }],
            ..plugin()
        },
        pack_with_situations(),
    );
    // Lifted up the side of the shelf, then over its top
    let mut hand = Hand::at(ON_THE_PET);
    hand.press(&mut app);
    hand.wait(&mut app, 1);
    for to in [
        Point { x: 450, y: 790 },
        Point { x: 450, y: 650 },
        Point { x: 450, y: 500 },
        Point { x: 600, y: 500 },
    ] {
        for _ in 0..5 {
            let from = hand.cursor;
            hand.move_to(
                &mut app,
                Point {
                    x: from.x + (to.x - from.x) / 2,
                    y: from.y + (to.y - from.y) / 2,
                },
            );
        }
    }
    hand.wait(&mut app, 30);

    // When
    hand.release(&mut app);
    hand.wait(&mut app, 90);

    // Then
    assert!(
        stands_on(&mut app, shelf.top),
        "it ended at {:?}",
        pet::<Position>(&mut app)
    );
}

#[test]
fn given_the_pet_on_the_ground_when_the_taskbar_grows_then_it_is_pushed_up_onto_the_new_ground_plane()
 {
    // Given
    let mut app = watched();
    ticks(&mut app, 1);

    // When
    app.world_mut().resource_mut::<WorkArea>().0.bottom = 700;
    ticks(&mut app, 30);

    // Then: it never went through to the other side
    assert!(stands_on(&mut app, 700));
    assert!(positions(&app).iter().all(|position| position.y >= 700));
}

#[test]
fn given_the_pet_on_the_ground_when_the_taskbar_shrinks_then_it_falls_onto_the_new_ground_plane() {
    // Given
    let mut app = watched();
    ticks(&mut app, 1);

    // When
    app.world_mut().resource_mut::<WorkArea>().0.bottom = 900;
    ticks(&mut app, 60);

    // Then
    assert!(stands_on(&mut app, 900));
    assert_eq!(landings(&app), 1);
}

#[test]
fn given_a_held_pet_when_it_is_hidden_then_it_is_let_go_and_falls_once_it_is_shown_again() {
    // Given
    let mut app = watched();
    let mut hand = lifted_to(&mut app, Point { x: 600, y: 400 });
    let held_at = pet::<Position>(&mut app);

    // When: hidden, so the button's release is never heard, then shown
    hide(&mut app);
    hand.wait(&mut app, 30);
    let while_hidden = pet::<Position>(&mut app);
    app.world_mut().resource_mut::<Hidden>().0 = false;
    step(&mut app, Duration::ZERO);
    hand.wait(&mut app, 90);

    // Then
    assert_eq!(while_hidden, held_at);
    assert!(stands_on(&mut app, WORK_AREA.bottom));

    // And a new press grabs it again, rather than counting as the old drag
    hand.cursor = ON_THE_PET;
    hand.press(&mut app);
    hand.wait(&mut app, 1);
    assert_eq!(
        pet::<InSituation>(&mut app),
        InSituation(Some(Situation::Held))
    );
}

#[test]
fn given_a_pet_in_mid_air_when_the_pack_is_replaced_with_a_bigger_body_then_it_keeps_its_place_and_takes_the_new_body()
 {
    // Given
    let mut app = watched();
    let _hand = lifted_to(&mut app, Point { x: 600, y: 400 });
    let before = pet::<Motion>(&mut app).0;
    let mut bigger = pack_with_situations();
    bigger.forms[1].body = Body {
        x: -5,
        y: -10,
        width: 10,
        height: 10,
    };

    // When
    replace_pack(app.world_mut(), Arc::new(bigger));

    // Then: the new Body at twice its size, and the same place and speed
    let after = pet::<Motion>(&mut app).0;
    assert_eq!(
        (after.anchor, after.velocity),
        (before.anchor, before.velocity)
    );
    assert_eq!(
        (
            after.extent.left,
            after.extent.top,
            after.extent.width,
            after.extent.height
        ),
        (-10.0, -20.0, 20.0, 20.0)
    );
    assert_eq!(pet::<InSituation>(&mut app), InSituation(None));
}

#[test]
fn given_a_held_pet_when_the_owner_lowers_it_onto_the_ground_then_it_is_no_landing() {
    // Given
    let mut app = watched();
    let mut hand = lifted_to(&mut app, Point { x: 600, y: 400 });

    // When: lowered gently until it rests on the Ground plane, still held
    for step in 1..=20 {
        hand.move_to(
            &mut app,
            Point {
                x: 600,
                y: 400 + 20 * step,
            },
        );
    }
    hand.wait(&mut app, 30);

    // Then
    assert_eq!(pet::<Position>(&mut app).0.y, WORK_AREA.bottom);
    assert_eq!(landings(&app), 0);
}

#[test]
fn given_a_held_pet_when_the_overlay_loses_the_mouse_then_the_drag_ends() {
    // Given
    let mut app = watched();
    let mut hand = lifted_to(&mut app, Point { x: 600, y: 400 });

    // When: Windows takes the mouse away, so the release goes to another window, and the cursor moves on
    hand.lose_the_mouse(&mut app);
    hand.move_to(&mut app, Point { x: 650, y: 400 });
    hand.wait(&mut app, 90);

    // Then
    assert!(stands_on(&mut app, WORK_AREA.bottom));
    assert!(
        app.world_mut()
            .query::<&Held>()
            .iter(app.world())
            .next()
            .is_none()
    );
    assert_eq!(landings(&app), 1);
}

#[test]
fn given_the_pet_on_the_ground_when_a_press_and_its_release_arrive_in_one_tick_then_it_is_not_held()
{
    // Given: a tap shorter than a tick, such as a touchpad's
    let mut app = watched();
    let mut hand = Hand::at(ON_THE_PET);

    // When
    hand.press(&mut app);
    hand.release(&mut app);
    hand.wait(&mut app, 3);

    // Then
    assert!(
        app.world_mut()
            .query::<&Held>()
            .iter(app.world())
            .next()
            .is_none()
    );
    assert_eq!(pet::<InSituation>(&mut app), InSituation(None));
}
