//! Passes the pointer to the simulation. The hit-test thread sees the cursor about every 8 ms, and the Overlay hears
//! the buttons, because it only catches clicks while the cursor is on the Pet, and hears when it loses the mouse.
//! Each update hands all of it to the simulation as messages, stamped with when they happened, before the ticks run.
//!
//! While the owner holds the Pet, the platform is told to keep the Overlay catching the mouse, since the Pet trails
//! behind the cursor and its pixels can slip out from under it.

use std::time::Instant;

use bevy::input::ButtonState;
use bevy::input::mouse::MouseButtonInput;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use pet_core::geometry::Point;
use pet_core::gesture::Button;
use pet_platform::PointerSample;
use pet_sim::{ButtonChanged, Held, MouseLost, Pet, PointerMoved};

use super::overlay::{OverlayRect, PlatformHandle};

/// What the platform does for the pointer, so the engine can be tested without one.
pub trait PointerSource: Resource {
    fn take_pointer_samples(&self) -> Vec<PointerSample>;
    fn take_mouse_lost(&self) -> Vec<(Instant, Point)>;
    fn set_holding(&self, holding: bool);
}

impl PointerSource for PlatformHandle {
    fn take_pointer_samples(&self) -> Vec<PointerSample> {
        self.0.take_pointer_samples()
    }

    fn take_mouse_lost(&self) -> Vec<(Instant, Point)> {
        self.0.take_mouse_lost()
    }

    fn set_holding(&self, holding: bool) {
        self.0.set_holding(holding);
    }
}

/// Where the cursor was last seen, for a button heard when the Overlay can't say where the cursor is.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct CursorLastSeen(pub Option<Point>);

pub struct PointerPlugin;

impl Plugin for PointerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CursorLastSeen>()
            .add_systems(
                PreUpdate,
                (forward_readings::<PlatformHandle>, forward_buttons)
                    .chain()
                    .after(bevy::input::InputSystems),
            )
            .add_systems(PostUpdate, keep_the_mouse_while_held::<PlatformHandle>);
    }
}

/// The pointer button a Bevy button stands for, if it's one a Gesture uses.
pub fn button(button: MouseButton) -> Option<Button> {
    match button {
        MouseButton::Left => Some(Button::Left),
        MouseButton::Middle => Some(Button::Middle),
        MouseButton::Right => Some(Button::Right),
        _ => None,
    }
}

/// Where a point `in_window` physical pixels from the Overlay's top-left corner is on screen.
pub fn on_screen(overlay: &OverlayRect, in_window: Vec2) -> Point {
    Point {
        x: overlay.0.left + in_window.x.floor() as i32,
        y: overlay.0.top + in_window.y.floor() as i32,
    }
}

/// Passes on what the platform read: the cursor's samples, and each time the Overlay lost the mouse.
pub(super) fn forward_readings<Source: PointerSource>(
    source: Option<Res<Source>>,
    mut last: ResMut<CursorLastSeen>,
    mut moved: MessageWriter<PointerMoved>,
    mut lost: MessageWriter<MouseLost>,
) {
    let Some(source) = source else { return };
    for sample in source.take_pointer_samples() {
        last.0 = Some(sample.position);
        moved.write(PointerMoved {
            at: sample.at,
            position: sample.position,
            over_pet: sample.over_pet,
        });
    }
    for (at, position) in source.take_mouse_lost() {
        lost.write(MouseLost { at, position });
    }
}

/// Passes on the buttons the Overlay heard, with where the cursor was. It runs after `forward_readings`, so a button
/// heard while the Overlay can't say where the cursor is goes with the newest sample.
pub(super) fn forward_buttons(
    overlay: Res<OverlayRect>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut presses: MessageReader<MouseButtonInput>,
    last: Res<CursorLastSeen>,
    mut changed: MessageWriter<ButtonChanged>,
) {
    let in_window = windows
        .single()
        .ok()
        .and_then(Window::physical_cursor_position)
        .map(|position| on_screen(&overlay, position));
    let Some(position) = in_window.or(last.0) else {
        presses.clear();
        return;
    };
    for press in presses.read() {
        let Some(button) = button(press.button) else {
            continue;
        };
        changed.write(ButtonChanged {
            at: Instant::now(),
            button,
            pressed: press.state == ButtonState::Pressed,
            position,
        });
    }
}

/// Tells the platform when the owner starts or stops holding the Pet.
pub(super) fn keep_the_mouse_while_held<Source: PointerSource>(
    source: Option<Res<Source>>,
    pets: Query<Has<Held>, With<Pet>>,
    mut holding: Local<bool>,
) {
    let Some(source) = source else { return };
    let now = pets.iter().any(|held| held);
    if now != *holding {
        *holding = now;
        source.set_holding(now);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::time::Instant;

    use bevy::ecs::message::Messages;
    use pet_core::geometry::Rect;

    use super::*;

    const OVERLAY: Rect = Rect {
        left: 100,
        top: 50,
        right: 1100,
        bottom: 850,
    };

    #[derive(Resource, Default)]
    struct FakePlatform {
        samples: Mutex<Vec<PointerSample>>,
        lost: Mutex<Vec<(Instant, Point)>>,
        holding: Mutex<Vec<bool>>,
    }

    impl PointerSource for FakePlatform {
        fn take_pointer_samples(&self) -> Vec<PointerSample> {
            std::mem::take(&mut self.samples.lock().unwrap())
        }

        fn take_mouse_lost(&self) -> Vec<(Instant, Point)> {
            std::mem::take(&mut self.lost.lock().unwrap())
        }

        fn set_holding(&self, holding: bool) {
            self.holding.lock().unwrap().push(holding);
        }
    }

    fn engine() -> App {
        let mut app = App::new();
        app.insert_resource(OverlayRect(OVERLAY))
            .init_resource::<FakePlatform>()
            .init_resource::<CursorLastSeen>()
            .add_message::<MouseButtonInput>()
            .add_message::<PointerMoved>()
            .add_message::<ButtonChanged>()
            .add_message::<MouseLost>()
            .add_systems(
                Update,
                (
                    (forward_readings::<FakePlatform>, forward_buttons).chain(),
                    keep_the_mouse_while_held::<FakePlatform>,
                ),
            );
        app
    }

    fn sent<T: Message + Clone>(app: &App) -> Vec<T> {
        let messages = app.world().resource::<Messages<T>>();
        messages.iter_current_update_messages().cloned().collect()
    }

    fn press(app: &mut App, state: ButtonState) {
        let window = app.world_mut().spawn_empty().id();
        app.world_mut().write_message(MouseButtonInput {
            button: MouseButton::Left,
            state,
            window,
        });
    }

    #[test]
    fn given_a_point_in_the_overlay_when_placing_it_on_screen_then_it_is_offset_by_the_overlays_top_left()
     {
        // Given
        let overlay = OverlayRect(OVERLAY);

        // When
        let point = on_screen(&overlay, Vec2::new(20.6, 30.0));

        // Then
        assert_eq!(point, Point { x: 120, y: 80 });
    }

    #[test]
    fn given_samples_from_the_hit_test_when_an_update_runs_then_each_goes_to_the_simulation_in_order()
     {
        // Given
        let mut app = engine();
        let start = Instant::now();
        let samples: Vec<PointerSample> = (0..3)
            .map(|i| PointerSample {
                at: start + std::time::Duration::from_millis(8 * i),
                position: Point {
                    x: 500 + i as i32,
                    y: 700,
                },
                over_pet: i == 1,
            })
            .collect();
        *app.world()
            .resource::<FakePlatform>()
            .samples
            .lock()
            .unwrap() = samples.clone();

        // When
        app.update();

        // Then
        let moved: Vec<PointerMoved> = sent(&app);
        let want: Vec<PointerMoved> = samples
            .iter()
            .map(|sample| PointerMoved {
                at: sample.at,
                position: sample.position,
                over_pet: sample.over_pet,
            })
            .collect();
        assert_eq!(moved, want);
    }

    #[test]
    fn given_a_button_heard_with_no_window_position_when_an_update_runs_then_it_goes_with_the_last_cursor_seen()
     {
        // Given
        let mut app = engine();
        let seen = Point { x: 640, y: 790 };
        app.world()
            .resource::<FakePlatform>()
            .samples
            .lock()
            .unwrap()
            .push(PointerSample {
                at: Instant::now(),
                position: seen,
                over_pet: true,
            });
        press(&mut app, ButtonState::Pressed);

        // When
        app.update();

        // Then
        let changed: Vec<ButtonChanged> = sent(&app);
        assert_eq!(changed.len(), 1);
        assert_eq!(
            (changed[0].button, changed[0].pressed, changed[0].position),
            (Button::Left, true, seen)
        );
    }

    #[test]
    fn given_the_overlay_lost_the_mouse_when_an_update_runs_then_the_simulation_hears_when_and_where()
     {
        // Given
        let mut app = engine();
        let at = Instant::now();
        let position = Point { x: 900, y: 300 };
        app.world()
            .resource::<FakePlatform>()
            .lost
            .lock()
            .unwrap()
            .push((at, position));

        // When
        app.update();

        // Then
        let lost: Vec<MouseLost> = sent(&app);
        assert_eq!(lost, [MouseLost { at, position }]);
    }

    #[test]
    fn given_the_pet_when_it_is_grabbed_and_let_go_then_the_platform_is_told_once_each_time() {
        // Given
        let mut app = engine();
        let pet = app.world_mut().spawn(Pet).id();
        app.update();

        // When
        app.world_mut().entity_mut(pet).insert(Held {
            grab: pet_core::physics::Vec2::ZERO,
            let_go_at: None,
        });
        app.update();
        app.update();
        app.world_mut().entity_mut(pet).remove::<Held>();
        app.update();

        // Then
        assert_eq!(
            *app.world()
                .resource::<FakePlatform>()
                .holding
                .lock()
                .unwrap(),
            [true, false]
        );
    }
}
