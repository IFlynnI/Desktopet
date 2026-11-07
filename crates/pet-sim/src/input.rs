//! Sense: what the pointer did since the last tick. The app writes it as messages as it hears of it, each stamped with
//! when it happened, and the tick reads them all at once. The cursor's position comes about every 8 ms, so a tick sees
//! the path it took, not just where it ended, and a quick flick throws as hard as it looked.

use std::time::Instant;

use bevy_ecs::prelude::*;
use pet_core::geometry::Point;
use pet_core::gesture::Button;

/// Where the cursor was at `at`, in physical pixels, and whether it was over the Pet's pixels then.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PointerMoved {
    pub at: Instant,
    pub position: Point,
    pub over_pet: bool,
}

/// The Overlay lost the mouse at `at`, with the cursor at `position`. It happens on every release, and also when
/// Windows takes the mouse away mid-drag, for example for Alt-Tab, when no release will come.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MouseLost {
    pub at: Instant,
    pub position: Point,
}

/// A pointer button went down or came up at `at`, with the cursor at `position`. The Overlay only hears a press while
/// the cursor is on the Pet, and keeps the mouse until it's released.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonChanged {
    pub at: Instant,
    pub button: Button,
    pub pressed: bool,
    pub position: Point,
}

/// What the pointer did in this tick, oldest first. It's read in later phases of the same tick.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct PointerThisTick {
    /// The cursor's path, starting from where it was last seen before this tick, if it has been seen.
    pub path: Vec<(Instant, Point)>,
    pub buttons: Vec<ButtonChanged>,
    pub mouse_lost: Vec<MouseLost>,
}

/// Where the cursor was last seen, so the next tick's path starts there.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct LastCursor(Option<(Instant, Point)>);

pub(crate) fn sense(
    mut moves: MessageReader<PointerMoved>,
    mut buttons: MessageReader<ButtonChanged>,
    mut lost: MessageReader<MouseLost>,
    mut last: ResMut<LastCursor>,
    mut this_tick: ResMut<PointerThisTick>,
) {
    this_tick.path.clear();
    this_tick.path.extend(last.0);
    this_tick
        .path
        .extend(moves.read().map(|moved| (moved.at, moved.position)));
    this_tick.path.sort_by_key(|(at, _)| *at);
    last.0 = this_tick.path.last().copied();

    this_tick.buttons.clear();
    this_tick.buttons.extend(buttons.read().copied());
    this_tick.buttons.sort_by_key(|button| button.at);

    this_tick.mouse_lost.clear();
    this_tick.mouse_lost.extend(lost.read().copied());
    this_tick.mouse_lost.sort_by_key(|lost| lost.at);
}
