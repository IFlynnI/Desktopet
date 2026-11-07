//! One tick of physics for one Body.
//!
//! A tick is split into a few substeps. Each one adds gravity and the string's pull to the velocity, then moves the
//! Body along it, stopping at the first surface it would touch, bouncing and sliding by the Materials, and going on
//! for the rest of the substep. Sweeping the whole path like this is what stops a fast Body passing through a thin
//! Obstacle between two ticks. The substeps keep the string stiff without blowing up, and let it follow a quick
//! flick through the cursor's samples instead of three points a tick.

use crate::body::Body;
use crate::collide::{first_hit, respond, unstick};
use crate::grab::Held;
use crate::vector::Vec2;
use crate::world::World;

/// Substeps per tick. At 30 ticks a second that is 120 Hz, a little faster than the cursor is sampled.
pub const SUBSTEPS: u32 = 4;

/// Touches a Body may make in one substep before it stops for the rest of it, such as the floor and then a wall in a
/// corner.
const MAX_TOUCHES: u32 = 4;

/// A bounce that would lift a Body less than this, in pixels, is dropped, so it settles instead of buzzing.
const REST_HEIGHT: f32 = 1.0;

/// A Body came to stand on an upward-facing surface after being airborne.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Landing {
    /// How hard it came down since it was last let go or last stood, in physical pixels per second. Being set down
    /// while held gives 0: nothing fell.
    pub speed: f32,
}

/// What happened to a Body during one tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step {
    /// Reported once, in the tick an airborne Body comes to stand.
    pub landing: Option<Landing>,
    /// Whether it ends the tick standing on the Ground plane or an Obstacle's top.
    pub standing: bool,
}

/// Moves `body` through one tick of `dt` seconds.
pub fn step(world: &World, body: &mut Body, held: Option<Held<'_>>, dt: f32) -> Step {
    unstick(world, body);
    // A bump in the owner's hand is no fall, so it never counts towards a Landing, and nor does anything from before
    // they grabbed it. The Landing after they let go reports only the fall after it.
    if held.is_some() {
        body.touchdown = 0.0;
    }
    let h = dt / SUBSTEPS as f32;
    let rest_speed = rest_speed(world.gravity, h);
    let mut landing = None;
    for substep in 0..SUBSTEPS {
        let mut acceleration = Vec2::new(0.0, world.gravity);
        if let Some(held) = &held {
            let (start, end) = (h * substep as f32, h * (substep + 1) as f32);
            acceleration += held.pull(body.anchor, body.velocity, start, end);
        }
        body.velocity += acceleration * h;
        if let Some(landed) = travel(world, body, h, rest_speed, held.is_some()) {
            landing = Some(landed);
        }
    }
    Step {
        landing,
        standing: body.standing,
    }
}

/// The slowest bounce kept: one that lifts the Body `REST_HEIGHT`, and never less than two substeps of gravity, which
/// is how fast a resting Body meets the surface it rests on each substep.
fn rest_speed(gravity: f32, h: f32) -> f32 {
    (2.0 * gravity * REST_HEIGHT).sqrt().max(2.0 * gravity * h)
}

/// Moves the Body along its velocity for `h` seconds, touching what it meets on the way. Gives a Landing if it comes
/// to stand. While it is `held`, how hard it comes down isn't counted.
fn travel(world: &World, body: &mut Body, h: f32, rest_speed: f32, held: bool) -> Option<Landing> {
    let mut landing = None;
    let mut supported = false;
    let mut left = h;
    for _ in 0..MAX_TOUCHES {
        let by = body.velocity * left;
        let Some(hit) = first_hit(world, body.extent, body.anchor, by) else {
            body.anchor += by;
            break;
        };
        body.anchor += by * hit.time;
        if hit.normal.x == 0.0 {
            body.anchor.y = hit.edge;
        } else {
            body.anchor.x = hit.edge;
        }
        left *= 1.0 - hit.time;
        let (speed, stopped) = respond(body, hit, rest_speed);
        if hit.normal.y < 0.0 {
            if !body.standing && !held {
                body.touchdown = body.touchdown.max(speed);
            }
            if stopped {
                supported = true;
                if !body.standing {
                    landing = Some(Landing {
                        speed: body.touchdown,
                    });
                    body.touchdown = 0.0;
                    body.standing = true;
                }
            }
        }
    }
    body.standing = supported;
    landing
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::{Extent, release};
    use crate::geometry::Rect;
    use crate::grab::{CursorSample, GrabString};
    use crate::material::Material;
    use crate::world::Obstacle;

    const DT: f32 = 1.0 / 30.0;

    /// The release speed cap's starting value in `balance.toml`.
    const MAX_SPEED: f32 = 2500.0;

    const SCREEN: Rect = Rect {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1040,
    };

    const SURFACE: Material = Material {
        friction: 0.8,
        bounciness: 0.1,
    };

    const PET: Material = Material {
        friction: 0.6,
        bounciness: 0.1,
    };

    fn world() -> World {
        World {
            bounds: SCREEN,
            surface: SURFACE,
            gravity: 3000.0,
            obstacles: Vec::new(),
        }
    }

    /// A 64 × 60 px Body with its Anchor at the middle of its bottom edge, as a Pack at 2× gives.
    fn pet_at(x: f32, y: f32) -> Body {
        Body::new(Vec2::new(x, y), Extent::scaled(-16, -30, 32, 30, 2), PET)
    }

    /// The Body's box in screen pixels: left, top, right, bottom.
    fn box_of(body: &Body) -> [f32; 4] {
        let left = body.anchor.x + body.extent.left;
        let top = body.anchor.y + body.extent.top;
        [
            left,
            top,
            left + body.extent.width,
            top + body.extent.height,
        ]
    }

    fn ground(world: &World) -> f32 {
        world.bounds.bottom as f32
    }

    /// Steps `body` for `seconds` with nobody holding it, and gives every tick's result.
    fn fly(world: &World, body: &mut Body, seconds: f32, mut each: impl FnMut(&Body, Step)) {
        for _ in 0..(seconds * 30.0) as usize {
            let result = step(world, body, None, DT);
            each(body, result);
        }
    }

    #[test]
    fn given_a_body_dropped_from_mid_air_when_stepped_then_it_lands_once_and_comes_to_rest_standing_on_the_ground_plane()
     {
        // Given
        let world = world();
        let mut body = pet_at(500.0, 300.0);

        // When
        let mut landings = Vec::new();
        let mut lowest = f32::MIN;
        fly(&world, &mut body, 5.0, |body, result| {
            landings.extend(result.landing);
            lowest = lowest.max(box_of(body)[3]);
        });

        // Then
        assert_eq!(landings.len(), 1, "{landings:?}");
        assert!(landings[0].speed > 0.0);
        assert!(lowest <= ground(&world), "went below the plane to {lowest}");
        assert_eq!(box_of(&body)[3], ground(&world));
        assert_eq!(body.velocity, Vec2::ZERO);
        assert!(body.standing);
    }

    #[test]
    fn given_a_bouncy_body_dropped_from_mid_air_when_stepped_then_it_bounces_but_reports_one_landing_once_it_stands()
     {
        // Given
        let world = world();
        let mut body = pet_at(500.0, 100.0);
        body.material.bounciness = 0.6;

        // When
        let mut landings = Vec::new();
        let mut bounced = false;
        fly(&world, &mut body, 8.0, |body, result| {
            landings.extend(result.landing);
            bounced |= body.velocity.y < 0.0;
        });

        // Then
        assert!(bounced);
        assert_eq!(landings.len(), 1, "{landings:?}");
        assert!(body.standing);
        assert_eq!(body.velocity, Vec2::ZERO);
    }

    #[test]
    fn given_a_body_placed_standing_on_the_ground_plane_when_stepped_then_it_stays_put_and_reports_no_landing()
     {
        // Given
        let world = world();
        let mut body = pet_at(500.0, ground(&world));
        body.standing = true;

        // When
        let mut landings = Vec::new();
        fly(&world, &mut body, 2.0, |_, result| {
            landings.extend(result.landing)
        });

        // Then
        assert_eq!(landings, []);
        assert!(body.standing);
        assert_eq!(body.anchor, Vec2::new(500.0, ground(&world)));
    }

    #[test]
    fn given_bodies_thrown_at_the_max_speed_in_every_direction_when_stepped_then_their_boxes_never_leave_the_bounds()
     {
        // Given
        let world = world();
        let directions = 64;

        // When
        let mut escapes = Vec::new();
        for i in 0..directions {
            let angle = i as f32 / directions as f32 * std::f32::consts::TAU;
            let mut body = pet_at(960.0, 600.0);
            body.material.bounciness = 0.9;
            body.velocity = Vec2::new(angle.cos(), angle.sin()) * MAX_SPEED;
            fly(&world, &mut body, 6.0, |body, _| {
                let [left, top, right, bottom] = box_of(body);
                if left < 0.0 || top < 0.0 || right > 1920.0 || bottom > 1040.0 {
                    escapes.push((i, box_of(body)));
                }
            });
        }

        // Then
        assert_eq!(escapes, []);
    }

    #[test]
    fn given_fast_bodies_thrown_at_a_thin_obstacle_when_stepped_then_none_passes_through() {
        // Given: a 4 px wall from top to bottom, and 200 Bodies on both sides of it, some tiny, all at least as fast
        // as the cap, which crosses the wall many times over in one substep
        let mut world = world();
        world.obstacles.push(Obstacle {
            rect: Rect {
                left: 1000,
                top: 0,
                right: 1004,
                bottom: 1040,
            },
            material: SURFACE,
        });
        let bodies = (0..200).map(|i| {
            let from_left = i % 2 == 0;
            let size = if i % 4 < 2 { 2 } else { 32 };
            let mut body = Body::new(
                Vec2::new(
                    if from_left { 900.0 } else { 1104.0 } + (i % 7) as f32,
                    200.0 + (i * 3) as f32,
                ),
                Extent::scaled(-1, -2, 2, 2, size / 2),
                PET,
            );
            let speed = MAX_SPEED * (1.0 + (i % 5) as f32 * 0.5);
            let towards = if from_left { 1.0 } else { -1.0 };
            body.velocity = Vec2::new(towards * speed, ((i % 9) as f32 - 4.0) * 100.0);
            (from_left, body)
        });

        // When
        let mut through = 0;
        for (from_left, mut body) in bodies {
            let mut passed = false;
            fly(&world, &mut body, 1.0, |body, _| {
                let [left, _, right, _] = box_of(body);
                passed |= if from_left {
                    left >= 1004.0
                } else {
                    right <= 1000.0
                };
            });
            through += usize::from(passed);
        }

        // Then
        assert_eq!(through, 0);
    }

    fn obstacle(left: i32, top: i32, right: i32, bottom: i32) -> Obstacle {
        Obstacle {
            rect: Rect {
                left,
                top,
                right,
                bottom,
            },
            material: SURFACE,
        }
    }

    #[test]
    fn given_a_body_dropped_above_an_obstacle_when_stepped_then_it_lands_and_stands_on_its_top() {
        // Given
        let mut world = world();
        world.obstacles.push(obstacle(400, 600, 800, 620));
        let mut body = pet_at(600.0, 300.0);

        // When
        let mut landings = Vec::new();
        fly(&world, &mut body, 3.0, |_, result| {
            landings.extend(result.landing)
        });

        // Then
        assert_eq!(landings.len(), 1, "{landings:?}");
        assert!(body.standing);
        assert_eq!(box_of(&body)[3], 600.0);
        assert_eq!(body.velocity, Vec2::ZERO);
    }

    #[test]
    fn given_a_body_thrown_up_into_an_obstacles_underside_when_stepped_then_it_comes_back_down() {
        // Given
        let mut world = world();
        world.obstacles.push(obstacle(400, 600, 800, 620));
        let mut body = pet_at(600.0, ground(&world));
        body.velocity = Vec2::new(0.0, -MAX_SPEED);

        // When
        let mut highest = f32::MAX;
        fly(&world, &mut body, 3.0, |body, _| {
            highest = highest.min(box_of(body)[1])
        });

        // Then
        assert!(highest >= 620.0, "went into the Obstacle to {highest}");
        assert!(body.standing);
        assert_eq!(box_of(&body)[3], ground(&world));
    }

    #[test]
    fn given_a_body_thrown_sideways_into_an_obstacle_when_stepped_then_it_does_not_pass() {
        // Given: a block taller than the Body, standing on the Ground plane
        let mut world = world();
        world.obstacles.push(obstacle(1000, 800, 1100, 1040));
        let mut body = pet_at(700.0, ground(&world));
        body.standing = true;
        body.velocity = Vec2::new(MAX_SPEED, 0.0);

        // When
        let mut furthest = f32::MIN;
        fly(&world, &mut body, 2.0, |body, _| {
            furthest = furthest.max(box_of(body)[2])
        });

        // Then
        assert!(furthest <= 1000.0, "went into the Obstacle to {furthest}");
    }

    /// Drops `body` and gives its velocity in the first tick it either comes to stand or moves up again.
    fn after_first_touch(world: &World, mut body: Body) -> (Option<Landing>, Vec2) {
        for _ in 0..300 {
            let result = step(world, &mut body, None, DT);
            if result.landing.is_some() || body.velocity.y < 0.0 {
                return (result.landing, body.velocity);
            }
        }
        panic!("never touched down");
    }

    #[test]
    fn given_a_bounciness_override_of_zero_when_dropped_on_a_bouncy_surface_then_it_lands_dead() {
        // Given
        let mut world = world();
        world.surface.bounciness = 0.9;
        let mut body = pet_at(500.0, 100.0);
        body.overrides.bounciness = Some(0.0);

        // When
        let (landing, velocity) = after_first_touch(&world, body);

        // Then
        assert!(landing.is_some_and(|landing| landing.speed > 0.0));
        assert_eq!(velocity.y, 0.0);
    }

    #[test]
    fn given_no_override_when_dropped_on_a_bouncy_surface_then_it_bounces_back() {
        // Given
        let mut world = world();
        world.surface.bounciness = 0.9;
        let body = pet_at(500.0, 100.0);

        // When
        let (landing, velocity) = after_first_touch(&world, body);

        // Then
        assert_eq!(landing, None);
        assert!(velocity.y < 0.0);
    }

    /// Throws a Body diagonally up into a wall of `friction`, with no gravity, and gives its speed along the wall
    /// just after the hit.
    fn speed_along_wall_after_hitting(friction: f32) -> f32 {
        let mut world = world();
        world.gravity = 0.0;
        let mut wall = obstacle(1000, 0, 1040, 1040);
        wall.material = Material {
            friction,
            bounciness: 0.0,
        };
        world.obstacles.push(wall);
        let mut body = pet_at(900.0, 700.0);
        body.velocity = Vec2::new(1500.0, -1000.0);
        for _ in 0..30 {
            step(&world, &mut body, None, DT);
            if body.velocity.x <= 0.0 {
                return body.velocity.y.abs();
            }
        }
        panic!("never reached the wall");
    }

    #[test]
    fn given_a_body_thrown_diagonally_into_walls_when_it_hits_then_it_keeps_more_sliding_speed_on_the_slippery_one()
     {
        // Given
        let (slippery, grippy) = (0.05, 0.95);

        // When
        let along_slippery = speed_along_wall_after_hitting(slippery);
        let along_grippy = speed_along_wall_after_hitting(grippy);

        // Then
        assert!(
            along_slippery > along_grippy,
            "{along_slippery} on the slippery wall, {along_grippy} on the grippy one"
        );
    }

    /// Slides a standing Body along the Ground plane of `friction` and gives how many ticks it takes to stop.
    fn ticks_to_stop_sliding_on(friction: f32) -> usize {
        let mut world = world();
        world.surface.friction = friction;
        let mut body = pet_at(300.0, ground(&world));
        body.standing = true;
        body.velocity = Vec2::new(800.0, 0.0);
        (1..300)
            .find(|_| {
                step(&world, &mut body, None, DT);
                body.velocity.x == 0.0
            })
            .expect("never stopped")
    }

    #[test]
    fn given_a_body_sliding_on_the_ground_when_the_ground_is_grippier_then_it_stops_sooner() {
        // Given
        let (slippery, grippy) = (0.1, 0.9);

        // When
        let on_slippery = ticks_to_stop_sliding_on(slippery);
        let on_grippy = ticks_to_stop_sliding_on(grippy);

        // Then
        assert!(
            on_grippy < on_slippery,
            "{on_grippy} ticks on grippy, {on_slippery} on slippery"
        );
    }

    #[test]
    fn given_a_body_sunk_below_the_ground_plane_when_stepped_then_it_is_moved_up_onto_it() {
        // Given: the work area shrank, so the Ground plane is now 30 px above the Body's feet
        let world = world();
        let mut body = pet_at(500.0, ground(&world) + 30.0);
        body.standing = true;

        // When
        step(&world, &mut body, None, DT);

        // Then
        assert_eq!(box_of(&body)[3], ground(&world));
        assert!(body.standing);
    }

    #[test]
    fn given_a_body_entirely_below_the_ground_plane_when_stepped_then_it_is_moved_up_onto_it() {
        // Given
        let world = world();
        let mut body = pet_at(500.0, ground(&world) + 500.0);

        // When
        step(&world, &mut body, None, DT);

        // Then
        assert_eq!(box_of(&body)[3], ground(&world));
    }

    #[test]
    fn given_a_body_whose_feet_overlap_an_obstacles_top_when_stepped_then_it_is_moved_up_onto_it() {
        // Given
        let mut world = world();
        world.obstacles.push(obstacle(400, 600, 800, 700));
        let mut body = pet_at(600.0, 610.0);

        // When
        step(&world, &mut body, None, DT);

        // Then
        assert_eq!(box_of(&body)[3], 600.0);
    }

    #[test]
    fn given_a_body_whose_side_overlaps_an_obstacle_when_stepped_then_it_is_moved_out_sideways() {
        // Given: its right edge is 10 px into the Obstacle, and its feet are 60 px below its top
        let mut world = world();
        world.obstacles.push(obstacle(400, 600, 800, 700));
        let mut body = pet_at(400.0 - 32.0 + 10.0, 660.0);

        // When
        step(&world, &mut body, None, DT);

        // Then
        assert_eq!(box_of(&body)[2], 400.0);
    }

    #[test]
    fn given_a_body_inside_an_obstacle_on_the_ground_whose_nearest_side_is_below_the_plane_when_stepped_then_it_is_moved_up_onto_it()
     {
        // Given: going down would be shorter, but that is through the Ground plane
        let mut world = world();
        world
            .obstacles
            .push(obstacle(400, 900, 800, ground(&world) as i32));
        let mut body = pet_at(600.0, 1010.0);

        // When
        step(&world, &mut body, None, DT);

        // Then
        assert_eq!(box_of(&body)[3], 900.0);
    }

    /// Every tick in which the Body's box overlaps an Obstacle or leaves the bounds, with its box.
    fn stuck_ticks(world: &World, body: &mut Body, seconds: f32) -> Vec<(usize, [f32; 4])> {
        let mut stuck = Vec::new();
        let mut tick = 0;
        fly(world, body, seconds, |body, _| {
            let [left, top, right, bottom] = box_of(body);
            let bounds = world.bounds;
            let outside_bounds = left < bounds.left as f32
                || top < bounds.top as f32
                || right > bounds.right as f32
                || bottom > bounds.bottom as f32;
            let in_obstacle = world.obstacles.iter().any(|obstacle| {
                let rect = obstacle.rect;
                left < rect.right as f32
                    && right > rect.left as f32
                    && top < rect.bottom as f32
                    && bottom > rect.top as f32
            });
            let lost = !(left.is_finite() && top.is_finite());
            if outside_bounds || in_obstacle || lost {
                stuck.push((tick, box_of(body)));
            }
            tick += 1;
        });
        stuck
    }

    #[test]
    fn given_a_body_overlapping_two_touching_obstacles_side_by_side_when_stepped_then_it_never_overlaps_either()
     {
        // Given: pushed out of the right one the shortest way, it would land inside the left one
        let mut world = world();
        world.obstacles.push(obstacle(400, 500, 500, 700));
        world.obstacles.push(obstacle(500, 500, 600, 700));
        let mut body = pet_at(520.0, 650.0);

        // When
        let stuck = stuck_ticks(&world, &mut body, 3.0);

        // Then
        assert_eq!(stuck, []);
    }

    #[test]
    fn given_a_body_jammed_where_an_l_shapes_post_meets_its_foot_when_stepped_then_it_never_overlaps_either()
     {
        // Given: a low foot on the Ground plane and a tall post beside it; the post's nearest side is in the foot
        let mut world = world();
        world.obstacles.push(obstacle(200, 980, 400, 1040));
        world.obstacles.push(obstacle(400, 600, 600, 1040));
        let mut body = pet_at(440.0, 1030.0);

        // When
        let stuck = stuck_ticks(&world, &mut body, 3.0);

        // Then
        assert_eq!(stuck, []);
    }

    #[test]
    fn given_a_body_inside_obstacles_with_room_only_in_a_far_corner_when_stepped_then_it_ends_in_that_room()
     {
        // Given: the screen is filled except for a pocket at the top right, which is in no straight line from the Body
        let mut world = world();
        world.obstacles.push(obstacle(0, 0, 1800, 1040));
        world.obstacles.push(obstacle(1800, 200, 1920, 1040));
        let mut body = pet_at(900.0, 600.0);

        // When
        let stuck = stuck_ticks(&world, &mut body, 3.0);

        // Then
        assert_eq!(stuck, []);
        let [left, _, _, bottom] = box_of(&body);
        assert!(left >= 1800.0 && bottom <= 200.0, "{:?}", box_of(&body));
    }

    #[test]
    fn given_a_body_inside_an_obstacle_covering_the_whole_screen_when_stepped_then_it_stays_inside_the_bounds()
     {
        // Given: there is nowhere free to go
        let mut world = world();
        world.obstacles.push(obstacle(0, 0, 1920, 1040));
        let mut body = pet_at(900.0, 600.0);

        // When
        let mut boxes = Vec::new();
        fly(&world, &mut body, 1.0, |body, _| boxes.push(box_of(body)));

        // Then
        let outside: Vec<_> = boxes
            .into_iter()
            .filter(|&[left, top, right, bottom]| {
                !(left >= 0.0 && top >= 0.0 && right <= 1920.0 && bottom <= 1040.0)
            })
            .collect();
        assert_eq!(outside, Vec::<[f32; 4]>::new());
    }

    /// The defaults in `balance.toml`.
    const STRING: GrabString = GrabString {
        length: 24.0,
        stiffness: 400.0,
        damping: 30.0,
        swing_drag: 4.0,
    };

    /// Near the top of the Body, as where the owner would grab its head.
    const GRAB: Vec2 = Vec2::new(0.0, -50.0);

    /// Holds `body` by `GRAB` for `ticks`, with the cursor's path in each tick given by `path(tick)`.
    fn hold(
        world: &World,
        body: &mut Body,
        ticks: usize,
        path: impl Fn(usize) -> Vec<CursorSample>,
        mut each: impl FnMut(&Body, Step),
    ) {
        for tick in 0..ticks {
            let cursor = path(tick);
            let held = Held {
                grab: GRAB,
                string: STRING,
                cursor: &cursor,
            };
            let result = step(world, body, Some(held), DT);
            each(body, result);
        }
    }

    fn still_at(position: Vec2) -> impl Fn(usize) -> Vec<CursorSample> {
        move |_| vec![CursorSample { at: 0.0, position }]
    }

    #[test]
    fn given_a_standing_body_when_grabbed_and_held_still_within_the_strings_length_then_it_does_not_lift()
     {
        // Given: the cursor just above the grab point, closer than the string's length
        let world = world();
        let mut body = pet_at(500.0, ground(&world));
        body.standing = true;
        let cursor = body.anchor + GRAB + Vec2::new(0.0, -(STRING.length - 4.0));

        // When
        let mut landings = Vec::new();
        hold(&world, &mut body, 60, still_at(cursor), |_, result| {
            landings.extend(result.landing)
        });

        // Then
        assert_eq!(body.anchor, Vec2::new(500.0, ground(&world)));
        assert!(body.standing);
        assert_eq!(landings, []);
    }

    #[test]
    fn given_a_body_held_with_the_cursor_still_in_mid_air_when_stepped_then_it_comes_to_rest_hanging_below_the_cursor()
     {
        // Given: the cursor up and to the side of the grab point, so the Body swings as well as bobs
        let world = world();
        let mut body = pet_at(500.0, 500.0);
        let cursor = body.anchor + GRAB + Vec2::new(80.0, -40.0);

        // When
        let mut finite = true;
        hold(&world, &mut body, 30 * 30, still_at(cursor), |body, _| {
            finite &= body.anchor.x.is_finite() && body.anchor.y.is_finite();
        });

        // Then
        let grab = body.anchor + GRAB;
        assert!(finite);
        assert!(
            (grab.x - cursor.x).abs() < 0.5,
            "{grab:?} isn't under {cursor:?}"
        );
        assert!(
            grab.y > cursor.y + STRING.length,
            "{grab:?} isn't hanging below {cursor:?}"
        );
        assert!(
            body.velocity.length() < 1.0,
            "still moving at {:?}",
            body.velocity
        );
    }

    #[test]
    fn given_a_held_body_when_the_cursor_flicks_right_and_lets_go_then_the_body_keeps_moving_right()
    {
        // Given: hanging at rest below the cursor
        let world = world();
        let mut body = pet_at(500.0, 500.0);
        let start = body.anchor + GRAB + Vec2::new(0.0, -STRING.length);
        hold(&world, &mut body, 60, still_at(start), |_, _| {});

        // When: the cursor moves right at 4,800 px/s for three ticks, sampled every 8 ms, then lets go
        let flick = |tick: usize| -> Vec<CursorSample> {
            (0..5)
                .map(|sample| {
                    let at = sample as f32 * 0.008;
                    let x = (tick as f32 * DT + at).min(3.0 * DT) * 4800.0;
                    CursorSample {
                        at,
                        position: start + Vec2::new(x, 0.0),
                    }
                })
                .collect()
        };
        hold(&world, &mut body, 3, flick, |_, _| {});
        release(&mut body, MAX_SPEED);

        // Then
        assert!(body.velocity.x > 0.0, "{:?}", body.velocity);
    }

    #[test]
    fn given_a_held_body_when_the_cursor_drags_it_through_a_wall_and_below_the_ground_then_it_stays_out_of_both()
     {
        // Given: a wall from the top of the screen down to 200 px above the Ground plane
        let mut world = world();
        world.obstacles.push(obstacle(1000, 0, 1020, 840));
        let mut body = pet_at(800.0, 600.0);
        let start = body.anchor + GRAB;

        // When: the cursor goes right through the wall at 3,000 px/s for half a second, then down below the plane
        let drag = |tick: usize| {
            let t = tick as f32 * DT;
            let to = if t < 0.5 {
                start + Vec2::new(t * 3000.0, 0.0)
            } else {
                start + Vec2::new(1500.0, (t - 0.5) * 3000.0)
            };
            vec![CursorSample {
                at: 0.0,
                position: to,
            }]
        };
        let mut inside = Vec::new();
        hold(&world, &mut body, 60, drag, |body, _| {
            let [left, top, right, bottom] = box_of(body);
            if (right > 1000.0 && left < 1020.0 && top < 840.0) || bottom > ground(&world) {
                inside.push(box_of(body));
            }
        });

        // Then
        assert_eq!(inside, Vec::<[f32; 4]>::new());
    }

    /// Where to hold the cursor still for `body` to hang with its feet `height` above the Ground plane.
    fn cursor_to_hang(world: &World, body: &Body, height: f32) -> Vec2 {
        let sag = world.gravity / STRING.stiffness;
        Vec2::new(body.anchor.x, ground(world) - height) + GRAB
            - Vec2::new(0.0, STRING.length + sag)
    }

    /// Lets go of `body` and drops it. Gives its Landing and the fastest a free fall from where it was let go could
    /// meet the Ground plane: the speed it had plus what the height adds, and one substep of gravity, because each
    /// substep adds its gravity before it moves.
    fn let_go_and_land(world: &World, body: &mut Body) -> (Landing, f32) {
        let height = ground(world) - box_of(body)[3];
        release(body, MAX_SPEED);
        let speed = body.velocity.length();
        let free_fall = (speed * speed + 2.0 * world.gravity * height).sqrt()
            + world.gravity * DT / SUBSTEPS as f32;
        let mut landings = Vec::new();
        fly(world, body, 3.0, |_, result| {
            landings.extend(result.landing)
        });
        assert_eq!(landings.len(), 1, "{landings:?}");
        (landings[0], free_fall)
    }

    #[test]
    fn given_a_held_body_slammed_into_the_floor_when_lifted_a_little_and_let_go_then_its_landing_reports_only_the_fall()
     {
        // Given: hanging at rest, then yanked down at the floor so hard it bounces off it, then lifted to hang just
        // above it
        let world = world();
        let mut body = pet_at(500.0, 500.0);
        body.material.bounciness = 0.3;
        let start = body.anchor + GRAB + Vec2::new(0.0, -STRING.length);
        hold(&world, &mut body, 60, still_at(start), |_, _| {});
        let below_the_floor = Vec2::new(start.x, ground(&world) + 2000.0);
        hold(&world, &mut body, 10, still_at(below_the_floor), |_, _| {});
        let lifted = cursor_to_hang(&world, &body, 17.0);
        hold(&world, &mut body, 60, still_at(lifted), |_, _| {});

        // When
        let (landing, free_fall) = let_go_and_land(&world, &mut body);

        // Then
        assert!(
            landing.speed <= free_fall,
            "landed at {} but a fall from there reaches {free_fall}",
            landing.speed
        );
    }

    #[test]
    fn given_a_body_grabbed_while_bouncing_high_when_let_go_just_above_the_floor_then_its_landing_reports_only_the_fall()
     {
        // Given: dropped from high up, it bounces off the floor and is grabbed on the way up, then hangs just above it
        let world = world();
        let mut body = pet_at(500.0, 100.0);
        body.material.bounciness = 0.6;
        while body.velocity.y >= 0.0 {
            step(&world, &mut body, None, DT);
        }
        let hang = cursor_to_hang(&world, &body, 17.0);
        hold(&world, &mut body, 60, still_at(hang), |_, _| {});

        // When
        let (landing, free_fall) = let_go_and_land(&world, &mut body);

        // Then
        assert!(
            landing.speed <= free_fall,
            "landed at {} but a fall from there reaches {free_fall}",
            landing.speed
        );
    }

    /// A busy run: swung in circles near a bouncy wall and a ledge, let go, and left to bounce and land. Gives the
    /// bits of the Anchor and the velocity after every tick.
    fn busy_run() -> Vec<[u32; 4]> {
        let mut world = world();
        world.surface.bounciness = 0.7;
        world.obstacles.push(obstacle(1200, 300, 1240, 900));
        world.obstacles.push(obstacle(300, 700, 700, 716));
        let mut body = pet_at(900.0, 500.0);
        let centre = body.anchor + GRAB;
        let mut bits = Vec::new();
        let mut record = |body: &Body| {
            bits.push([
                body.anchor.x.to_bits(),
                body.anchor.y.to_bits(),
                body.velocity.x.to_bits(),
                body.velocity.y.to_bits(),
            ])
        };
        let circles = |tick: usize| -> Vec<CursorSample> {
            (0..4)
                .map(|sample| {
                    let at = sample as f32 * 0.008;
                    let angle = (tick as f32 * DT + at) * 12.0;
                    CursorSample {
                        at,
                        position: centre + Vec2::new(angle.cos(), angle.sin()) * 120.0,
                    }
                })
                .collect()
        };
        hold(&world, &mut body, 60, circles, |body, _| record(body));
        release(&mut body, MAX_SPEED);
        fly(&world, &mut body, 8.0, |body, _| record(body));
        bits
    }

    #[test]
    fn given_the_same_inputs_when_run_twice_then_every_tick_is_bit_identical() {
        // Given
        let runs = 2;

        // When
        let results: Vec<_> = (0..runs).map(|_| busy_run()).collect();

        // Then
        assert_eq!(results[0], results[1]);
    }

    #[test]
    fn given_a_still_body_below_the_ground_plane_with_no_gravity_when_stepped_then_it_is_moved_up_onto_it()
     {
        // Given: nothing moves it, so only sorting out the overlap can bring it up
        let mut world = world();
        world.gravity = 0.0;
        let mut body = pet_at(500.0, ground(&world) + 30.0);

        // When
        step(&world, &mut body, None, DT);

        // Then
        assert_eq!(box_of(&body)[3], ground(&world));
    }

    #[test]
    fn given_a_body_jerked_sideways_when_the_cursor_then_holds_still_for_two_seconds_then_its_swing_has_died_down()
     {
        // Given: hanging at rest, then the cursor jumps 75 px sideways a tick for four ticks, which sets it swinging
        let world = world();
        let mut body = pet_at(500.0, 500.0);
        let start = body.anchor + GRAB + Vec2::new(0.0, -STRING.length);
        hold(&world, &mut body, 60, still_at(start), |_, _| {});
        let jerks = |tick: usize| -> Vec<CursorSample> {
            vec![CursorSample {
                at: 0.0,
                position: start + Vec2::new(75.0 * (tick + 1) as f32, 0.0),
            }]
        };
        hold(&world, &mut body, 4, jerks, |_, _| {});
        let cursor = start + Vec2::new(300.0, 0.0);

        // When: the cursor holds still for two seconds
        let mut sway = [0.0_f32; 4];
        for quarter in &mut sway {
            hold(&world, &mut body, 15, still_at(cursor), |body, _| {
                *quarter = quarter.max(((body.anchor + GRAB).x - cursor.x).abs());
            });
        }

        // Then: it swung, and in the last half second it sways less than a tenth as far as in the first
        let [first, .., last] = sway;
        assert!(first > 20.0, "never swung: {sway:?}");
        assert!(last < first / 10.0, "still swinging: {sway:?}");
    }

    /// Where the cursor holds still relative to the grab point in each start: up and to the side, so the Body swings
    /// as well as bobs; straight up and far beyond the string; level with it; and far up and across.
    const STARTS: [Vec2; 4] = [
        Vec2::new(80.0, -40.0),
        Vec2::new(0.0, -90.0),
        Vec2::new(150.0, 0.0),
        Vec2::new(-300.0, -200.0),
    ];

    /// A string with `stiffness` and `resisting` damping plus swing drag, the swing drag as in the defaults if there
    /// is that much.
    fn string_of(stiffness: f32, resisting: f32) -> GrabString {
        let swing_drag = resisting.min(STRING.swing_drag);
        GrabString {
            stiffness,
            damping: resisting - swing_drag,
            swing_drag,
            ..STRING
        }
    }

    /// Whether a Body hung from `string` comes to rest within twenty seconds from every one of `STARTS`, the cursor
    /// holding still: slower than 1 px/s for the whole last second.
    fn settles(string: GrabString) -> bool {
        STARTS.iter().all(|&offset| {
            let world = world();
            let mut body = pet_at(900.0, 500.0);
            let cursor = [CursorSample {
                at: 0.0,
                position: body.anchor + GRAB + offset,
            }];
            let mut settled = true;
            for tick in 0..20 * 30 {
                let held = Held {
                    grab: GRAB,
                    string,
                    cursor: &cursor,
                };
                step(&world, &mut body, Some(held), DT);
                if tick >= 19 * 30 {
                    settled &= body.velocity.length() < 1.0;
                }
            }
            settled
        })
    }

    #[test]
    fn given_strings_at_the_edges_of_every_stability_limit_when_hanging_from_a_still_cursor_then_each_one_settles()
     {
        // Given: the default stiffness and just under the most, each with just over the least damping and just under
        // the most
        let strings: Vec<GrabString> = [STRING.stiffness, STRING.max_stable_stiffness(DT) * 0.99]
            .into_iter()
            .flat_map(|stiffness| {
                let string = string_of(stiffness, 0.0);
                [
                    string.min_stable_damping() * 1.01,
                    string.max_stable_damping(DT) * 0.99,
                ]
                .map(|resisting| string_of(stiffness, resisting))
            })
            .collect();

        // When
        let results: Vec<_> = strings
            .iter()
            .map(|string| (string.is_stable(DT), settles(*string)))
            .collect();

        // Then
        assert_eq!(results, [(true, true); 4], "{strings:?}");
    }

    #[test]
    fn given_a_string_far_stiffer_than_its_limit_when_hanging_from_a_still_cursor_then_it_never_settles()
     {
        // Given: twelve times the stiffest, which is still inside the budget a string that never went slack would
        // allow, so it is the slack that shakes it
        let string = GrabString {
            stiffness: STRING.max_stable_stiffness(DT) * 12.0,
            ..STRING
        };

        // When
        let settled = settles(string);

        // Then
        assert!(!settled);
    }

    #[test]
    fn given_a_string_with_twice_its_most_damping_when_hanging_from_a_still_cursor_then_it_never_settles()
     {
        // Given: the extra in swing drag, which damps every direction and never lets up
        let string = GrabString {
            swing_drag: STRING.max_stable_damping(DT) * 2.0 - STRING.damping,
            ..STRING
        };

        // When
        let settled = settles(string);

        // Then
        assert!(!settled);
    }
}
