//! Turning what the owner does with the pointer into Gestures. The Overlay only catches a press while the cursor is on
//! the Pet, so every press this hears is a press on the Pet.
//!
//! Slice 1 knows one Gesture, the drag: a left press grabs the Pet and letting go of that button ends it. Whether it
//! was a throw or a gentle drop is up to the physics, from how the Pet was moving, so there is no threshold here.
//! Pokes, strokes and the Marker come in later slices, and add to this rather than replacing it.

use crate::geometry::Point;

/// A pointer button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Left,
    Middle,
    Right,
}

/// Something the owner did to the Pet with the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gesture {
    /// The owner grabbed the Pet with the cursor at `at`, in physical pixels.
    DragStart { at: Point },
    /// The owner let go of the Pet with the cursor at `at`.
    DragEnd { at: Point },
}

/// Recognises Gestures from button presses and releases, in the order they happened.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Recogniser {
    dragging: bool,
}

impl Recogniser {
    /// A button went down with the cursor at `at`, on the Pet.
    pub fn pressed(&mut self, button: Button, at: Point) -> Option<Gesture> {
        if button != Button::Left || self.dragging {
            return None;
        }
        self.dragging = true;
        Some(Gesture::DragStart { at })
    }

    /// A button came up with the cursor at `at`, wherever that is: the Overlay keeps the mouse while it's held.
    pub fn released(&mut self, button: Button, at: Point) -> Option<Gesture> {
        if button != Button::Left || !self.dragging {
            return None;
        }
        self.dragging = false;
        Some(Gesture::DragEnd { at })
    }

    /// The Overlay lost the mouse with the cursor at `at`. Windows can take it away mid-drag, for example for Alt-Tab,
    /// and then no release ever comes, so this is the only way the drag ends. It also happens on every release, and
    /// whichever is heard first ends the drag: the other is no second end.
    pub fn mouse_lost(&mut self, at: Point) -> Option<Gesture> {
        self.released(Button::Left, at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ON_THE_PET: Point = Point { x: 500, y: 790 };

    const ELSEWHERE: Point = Point { x: 900, y: 200 };

    /// Whether a drag is going on, by whether letting go of the left button would end one, asked of a copy.
    fn dragging(recogniser: Recogniser) -> bool {
        let mut copy = recogniser;
        copy.released(Button::Left, ELSEWHERE).is_some()
    }

    #[test]
    fn given_nothing_held_when_the_left_button_goes_down_on_the_pet_then_a_drag_starts_there() {
        // Given
        let mut recogniser = Recogniser::default();

        // When
        let gesture = recogniser.pressed(Button::Left, ON_THE_PET);

        // Then
        assert_eq!(gesture, Some(Gesture::DragStart { at: ON_THE_PET }));
        assert!(dragging(recogniser));
    }

    #[test]
    fn given_a_drag_when_the_left_button_comes_up_anywhere_then_the_drag_ends_there() {
        // Given
        let mut recogniser = Recogniser::default();
        recogniser.pressed(Button::Left, ON_THE_PET);

        // When
        let gesture = recogniser.released(Button::Left, ELSEWHERE);

        // Then
        assert_eq!(gesture, Some(Gesture::DragEnd { at: ELSEWHERE }));
        assert!(!dragging(recogniser));
    }

    #[test]
    fn given_nothing_held_when_a_button_comes_up_then_it_is_no_gesture() {
        // Given: the press happened before the Pet could hear it, for example while it was Hidden
        let mut recogniser = Recogniser::default();

        // When
        let gesture = recogniser.released(Button::Left, ON_THE_PET);

        // Then
        assert_eq!(gesture, None);
        assert!(!dragging(recogniser));
    }

    #[test]
    fn given_nothing_held_when_the_middle_or_right_button_goes_down_then_it_is_no_drag() {
        // Given: they mean the Marker and the menu, which later slices recognise
        let mut recogniser = Recogniser::default();

        // When
        let gestures = [Button::Middle, Button::Right].map(|button| {
            let gesture = recogniser.pressed(button, ON_THE_PET);
            recogniser.released(button, ON_THE_PET);
            gesture
        });

        // Then
        assert_eq!(gestures, [None, None]);
        assert!(!dragging(recogniser));
    }

    #[test]
    fn given_a_drag_when_another_button_goes_down_and_up_then_the_drag_goes_on() {
        // Given
        let mut recogniser = Recogniser::default();
        recogniser.pressed(Button::Left, ON_THE_PET);

        // When
        let pressed = recogniser.pressed(Button::Right, ELSEWHERE);
        let released = recogniser.released(Button::Right, ELSEWHERE);

        // Then
        assert_eq!((pressed, released), (None, None));
        assert!(dragging(recogniser));
    }

    #[test]
    fn given_a_drag_when_the_overlay_loses_the_mouse_then_the_drag_ends_there() {
        // Given: Windows took the mouse away, for example for Alt-Tab during a drag, so no release will come
        let mut recogniser = Recogniser::default();
        recogniser.pressed(Button::Left, ON_THE_PET);

        // When
        let gesture = recogniser.mouse_lost(ELSEWHERE);
        let then_released = recogniser.released(Button::Left, ELSEWHERE);

        // Then: one end, not two
        assert_eq!(gesture, Some(Gesture::DragEnd { at: ELSEWHERE }));
        assert_eq!(then_released, None);
        assert!(!dragging(recogniser));
    }

    #[test]
    fn given_nothing_held_when_the_overlay_loses_the_mouse_then_it_is_no_gesture() {
        // Given
        let mut recogniser = Recogniser::default();

        // When
        let gesture = recogniser.mouse_lost(ELSEWHERE);

        // Then
        assert_eq!(gesture, None);
    }

    #[test]
    fn given_a_drag_when_the_left_button_goes_down_again_then_it_is_still_one_drag() {
        // Given: a release can go missing, for example while the Pet was Hidden
        let mut recogniser = Recogniser::default();
        recogniser.pressed(Button::Left, ON_THE_PET);

        // When
        let again = recogniser.pressed(Button::Left, ELSEWHERE);

        // Then
        assert_eq!(again, None);
        assert!(dragging(recogniser));
    }
}
