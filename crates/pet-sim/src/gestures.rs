//! Interpret: the pointer's buttons become Gestures, by `pet_core::gesture`'s rules. The Overlay hears the buttons,
//! and also hears when it loses the mouse, so a drag whose release went to another window, as after Alt-Tab, still
//! ends. Both are taken in the order they happened.

use std::time::Instant;

use bevy_ecs::prelude::*;
use pet_core::geometry::Point;
use pet_core::gesture::{Button, Gesture, Recogniser};

use crate::input::PointerThisTick;

/// What the recogniser hears, in the order it happened.
enum Heard {
    Button {
        button: Button,
        pressed: bool,
        at: Point,
    },
    MouseLost {
        at: Point,
    },
}

/// A Gesture the owner made, and when it happened. Read in React and later phases of the tick it was made in.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct GestureMade {
    pub gesture: Gesture,
    pub at: Instant,
}

/// The recogniser, which remembers whether a drag is going on from one tick to the next.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub(crate) struct Recognising(pub(crate) Recogniser);

pub(crate) fn interpret(
    pointer: Res<PointerThisTick>,
    mut recognising: ResMut<Recognising>,
    mut gestures: MessageWriter<GestureMade>,
) {
    let buttons = pointer.buttons.iter().map(|change| {
        let heard = Heard::Button {
            button: change.button,
            pressed: change.pressed,
            at: change.position,
        };
        (change.at, heard)
    });
    let lost = pointer
        .mouse_lost
        .iter()
        .map(|lost| (lost.at, Heard::MouseLost { at: lost.position }));
    let mut heard: Vec<_> = buttons.chain(lost).collect();
    // Stable, so of two heard at the same moment, the button comes first.
    heard.sort_by_key(|(when, _)| *when);
    for (when, heard) in heard {
        let gesture = match heard {
            Heard::Button {
                button,
                pressed: true,
                at,
            } => recognising.0.pressed(button, at),
            Heard::Button {
                button,
                pressed: false,
                at,
            } => recognising.0.released(button, at),
            Heard::MouseLost { at } => recognising.0.mouse_lost(at),
        };
        if let Some(gesture) = gesture {
            gestures.write(GestureMade { gesture, at: when });
        }
    }
}
