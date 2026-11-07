//! Finding the first thing a moving Body touches, and what the touch does to its velocity.
//!
//! A Body's box against a rectangle is the same question as its Anchor, a single point, against that rectangle grown
//! by the box. So every test here moves a point. On a hit the Anchor is put exactly on the grown edge, the same value
//! the next test compares against, so a Body resting on a surface stays on it rather than creeping in by rounding.

use crate::body::{Body, Extent};
use crate::geometry::Rect;
use crate::material::Material;
use crate::vector::Vec2;
use crate::world::World;

/// Where an Anchor is, or isn't, allowed, in screen pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Region {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

/// Where the Anchor may go for the Body's box to stay inside `bounds`. Its `bottom` is the Ground plane.
pub(crate) fn room(bounds: Rect, extent: Extent) -> Region {
    Region {
        left: bounds.left as f32 - extent.left,
        top: bounds.top as f32 - extent.top,
        right: bounds.right as f32 - (extent.left + extent.width),
        bottom: bounds.bottom as f32 - (extent.top + extent.height),
    }
}

/// The first surface the Anchor touches moving from `from` by `by`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Hit {
    /// The share of `by` travelled before the touch, from 0 to 1.
    pub time: f32,
    /// Points out of the surface, along one axis.
    pub normal: Vec2,
    /// The Anchor's coordinate on the normal's axis at the touch.
    pub edge: f32,
    pub surface: Material,
}

/// The first hit on the screen's edges or the Ground plane. Already being past an edge and moving further out is a
/// hit at once, so a Body can never leave through one.
fn edge_hit(room: Region, surface: Material, from: Vec2, by: Vec2) -> Option<Hit> {
    let to = from + by;
    let crossings = [
        (by.y > 0.0 && to.y > room.bottom)
            .then(|| (room.bottom, from.y, by.y, Vec2::new(0.0, -1.0))),
        (by.y < 0.0 && to.y < room.top).then(|| (room.top, from.y, by.y, Vec2::new(0.0, 1.0))),
        (by.x < 0.0 && to.x < room.left).then(|| (room.left, from.x, by.x, Vec2::new(1.0, 0.0))),
        (by.x > 0.0 && to.x > room.right).then(|| (room.right, from.x, by.x, Vec2::new(-1.0, 0.0))),
    ];
    crossings
        .into_iter()
        .flatten()
        .map(|(edge, from, by, normal)| Hit {
            time: ((edge - from) / by).max(0.0),
            normal,
            edge,
            surface,
        })
        .fold(None, earlier)
}

fn earlier(first: Option<Hit>, hit: Hit) -> Option<Hit> {
    match first {
        Some(first) if first.time <= hit.time => Some(first),
        _ => Some(hit),
    }
}

/// Where the Anchor may not go for the Body's box to stay out of `rect`. Its edges themselves are allowed, so a Body
/// can rest on top of an Obstacle or slide down its side.
pub(crate) fn blocked(rect: Rect, extent: Extent) -> Region {
    Region {
        left: rect.left as f32 - (extent.left + extent.width),
        top: rect.top as f32 - (extent.top + extent.height),
        right: rect.right as f32 - extent.left,
        bottom: rect.bottom as f32 - extent.top,
    }
}

/// Rounding can leave an Anchor this far inside a surface, in pixels, and it still counts as touching it.
const SKIN: f32 = 0.01;

/// When a point moving along one axis is strictly between `low` and `high`: the shares of its move at which it goes
/// in and comes out. `None` if it never is.
fn inside_between(from: f32, by: f32, low: f32, high: f32) -> Option<(f32, f32)> {
    if by == 0.0 {
        (from > low && from < high).then_some((f32::NEG_INFINITY, f32::INFINITY))
    } else {
        let (a, b) = ((low - from) / by, (high - from) / by);
        Some((a.min(b), a.max(b)))
    }
}

/// The hit on one Obstacle, by the slab test: the point is inside once it is inside on both axes, and the axis it
/// went in on last is the side it hit. A corner hit counts as the top or bottom, so a Body clipping a corner lands.
/// A point that starts deeper inside than `SKIN` is left alone; overlaps are sorted out before the tick moves anything.
fn obstacle_hit(blocked: Region, surface: Material, from: Vec2, by: Vec2) -> Option<Hit> {
    let (x_in, x_out) = inside_between(from.x, by.x, blocked.left, blocked.right)?;
    let (y_in, y_out) = inside_between(from.y, by.y, blocked.top, blocked.bottom)?;
    let (time, out) = (x_in.max(y_in), x_out.min(y_out));
    if time >= out || time > 1.0 || out <= 0.0 {
        return None;
    }
    let (normal, edge, depth) = if y_in >= x_in {
        let (normal, edge) = if by.y > 0.0 {
            (Vec2::new(0.0, -1.0), blocked.top)
        } else {
            (Vec2::new(0.0, 1.0), blocked.bottom)
        };
        (normal, edge, -time * by.y.abs())
    } else {
        let (normal, edge) = if by.x > 0.0 {
            (Vec2::new(-1.0, 0.0), blocked.left)
        } else {
            (Vec2::new(1.0, 0.0), blocked.right)
        };
        (normal, edge, -time * by.x.abs())
    };
    (depth <= SKIN).then_some(Hit {
        time: time.max(0.0),
        normal,
        edge,
        surface,
    })
}

/// The first surface the Anchor touches on the way from `from` by `by`.
pub(crate) fn first_hit(world: &World, extent: Extent, from: Vec2, by: Vec2) -> Option<Hit> {
    world
        .obstacles
        .iter()
        .filter_map(|obstacle| {
            obstacle_hit(blocked(obstacle.rect, extent), obstacle.material, from, by)
        })
        .fold(
            edge_hit(room(world.bounds, extent), world.surface, from, by),
            earlier,
        )
}

/// Moves a Body that starts overlapping something out of it, such as when the work area shrank under it or an
/// Obstacle now covers it. It goes back inside the bounds first, the Ground plane winning, so a Body below the plane
/// comes up onto it. Then out of the Obstacles, to the nearest spot that is inside the bounds and inside none of
/// them, so it never comes out through the Ground plane or into a neighbouring Obstacle. If there is no such spot, it
/// stays inside the bounds. The part of its velocity heading back in is dropped.
pub(crate) fn unstick(world: &World, body: &mut Body) {
    let room = room(world.bounds, body.extent);
    let at = body.anchor;
    // `max` then `min`, never `clamp`, which panics on a Body bigger than the bounds; the later one wins.
    body.anchor.x = at.x.min(room.right).max(room.left);
    body.anchor.y = at.y.max(room.top).min(room.bottom);
    if let Some(out) = way_out(world, body.extent, room, body.anchor) {
        body.anchor = out;
    }
    stop_heading_back(body, at);
}

fn inside(region: Region, point: Vec2) -> bool {
    point.x > region.left
        && point.x < region.right
        && point.y > region.top
        && point.y < region.bottom
}

/// Where an Anchor inside one or more Obstacles goes to be inside none of them and inside `room`, or `None` if it is
/// already free or there is nowhere to go.
///
/// Straight up, left, right or down comes first, the nearest of them: each walks on through every Obstacle it meets,
/// so two touching Obstacles are left together rather than one into the other. Only if all four leave the room does
/// it look anywhere else. Then the nearest free spot is a corner of the free space, and every such corner is where an
/// edge of the room or of an Obstacle crosses another, so it tries every crossing. That costs the cube of the number
/// of Obstacles, which is fine for the few an owner sets and only paid by a Body boxed in on all four sides.
fn way_out(world: &World, extent: Extent, room: Region, at: Vec2) -> Option<Vec2> {
    let blocks = || {
        world
            .obstacles
            .iter()
            .map(move |obstacle| blocked(obstacle.rect, extent))
    };
    let free = |point: Vec2| !blocks().any(|block| inside(block, point));
    if free(at) {
        return None;
    }
    let in_room = |point: &Vec2| {
        point.x >= room.left
            && point.x <= room.right
            && point.y >= room.top
            && point.y <= room.bottom
    };
    let walk = |exit: fn(Region, Vec2) -> Vec2| {
        let mut point = at;
        // Each Obstacle can stop the walk only once, because the walk never turns back.
        for _ in 0..=world.obstacles.len() {
            match blocks().find(|&block| inside(block, point)) {
                Some(block) => point = exit(block, point),
                None => return Some(point),
            }
        }
        None
    };
    let straight = [
        walk(|block, point| Vec2::new(point.x, block.top)),
        walk(|block, point| Vec2::new(block.left, point.y)),
        walk(|block, point| Vec2::new(block.right, point.y)),
        walk(|block, point| Vec2::new(point.x, block.bottom)),
    ];
    if let Some(out) = nearest(at, straight.into_iter().flatten().filter(in_room)) {
        return Some(out);
    }
    let xs = || {
        [room.left, room.right]
            .into_iter()
            .chain(blocks().flat_map(|block| [block.left, block.right]))
    };
    let ys = || {
        [room.top, room.bottom]
            .into_iter()
            .chain(blocks().flat_map(|block| [block.top, block.bottom]))
    };
    let crossings = xs().flat_map(|x| ys().map(move |y| Vec2::new(x, y)));
    nearest(
        at,
        crossings.filter(in_room).filter(|&crossing| free(crossing)),
    )
}

/// The exit closest to `at`, counting the distance along each axis. The higher one wins a tie, as getting out on top
/// of something beats getting out under it; between two at the same height, the first does.
fn nearest(at: Vec2, exits: impl Iterator<Item = Vec2>) -> Option<Vec2> {
    let distance = |exit: Vec2| (exit.x - at.x).abs() + (exit.y - at.y).abs();
    exits.fold(None, |nearest, exit| match nearest {
        Some(nearest) if (distance(nearest), nearest.y) <= (distance(exit), exit.y) => {
            Some(nearest)
        }
        _ => Some(exit),
    })
}

/// After the Body was moved from `was`, drops the part of its velocity that heads back the way it came.
fn stop_heading_back(body: &mut Body, was: Vec2) {
    let moved = body.anchor - was;
    if moved.x * body.velocity.x < 0.0 {
        body.velocity.x = 0.0;
    }
    if moved.y * body.velocity.y < 0.0 {
        body.velocity.y = 0.0;
    }
}

/// What a touch does to the Body's velocity: the part into the surface bounces back by the bounciness, and the
/// sliding part loses as much speed as friction allows for the push the surface gave. A bounce slower than
/// `rest_speed` is dropped, so a Body settles instead of buzzing on the spot. Gives the speed it came in at, and
/// whether it stopped against the surface.
pub(crate) fn respond(body: &mut Body, hit: Hit, rest_speed: f32) -> (f32, bool) {
    let contact = crate::material::combine(body.material, hit.surface, body.overrides);
    let into = -body.velocity.dot(hit.normal);
    let mut out = contact.bounciness * into;
    if out < rest_speed {
        out = 0.0;
    }
    let sliding = body.velocity + hit.normal * into;
    let speed = sliding.length();
    let slowed = (speed - contact.friction * (into + out)).max(0.0);
    let sliding = if speed > 0.0 {
        sliding * (slowed / speed)
    } else {
        sliding
    };
    body.velocity = sliding + hit.normal * out;
    (into, out == 0.0)
}
