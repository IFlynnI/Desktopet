//! Keeps the Overlay on the work area. When the taskbar is resized, moved or set to hide itself, the shell thread
//! says so, the simulation's `WorkArea` changes, and the Pet lands on, or is pushed up onto, the new Ground plane. This
//! moves and resizes the Overlay to match, so it still covers exactly where the Pet can be.

use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResolution};
use pet_sim::WorkArea;

use super::overlay::{OverlayRect, overlay_rect};

/// Runs when the work area has changed. The first run, at startup, finds the Overlay already in place and changes
/// nothing.
pub fn follow(
    work_area: Res<WorkArea>,
    mut overlay: ResMut<OverlayRect>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let rect = overlay_rect(work_area.0);
    if overlay.0 == rect {
        return;
    }
    overlay.0 = rect;
    if let Ok(mut window) = windows.single_mut() {
        window.position = WindowPosition::At(IVec2::new(rect.left, rect.top));
        window.resolution = WindowResolution::new(rect.width() as u32, rect.height() as u32)
            .with_scale_factor_override(1.0);
    }
}

#[cfg(test)]
mod tests {
    use pet_core::geometry::Rect;

    use super::*;

    const WORK_AREA: Rect = Rect {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1032,
    };

    fn engine() -> App {
        let mut app = App::new();
        app.insert_resource(WorkArea(WORK_AREA))
            .insert_resource(OverlayRect(overlay_rect(WORK_AREA)))
            .add_systems(Update, follow.run_if(resource_changed::<WorkArea>));
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.update();
        app
    }

    fn window(app: &mut App) -> Window {
        let world = app.world_mut();
        world.query::<&Window>().single(world).unwrap().clone()
    }

    #[test]
    fn given_the_overlay_on_the_work_area_when_the_taskbar_grows_then_the_overlay_moves_and_resizes_to_the_new_one()
     {
        // Given
        let mut app = engine();
        let taller_taskbar = Rect {
            bottom: 980,
            ..WORK_AREA
        };

        // When
        app.world_mut().resource_mut::<WorkArea>().0 = taller_taskbar;
        app.update();

        // Then
        let rect = overlay_rect(taller_taskbar);
        assert_eq!(app.world().resource::<OverlayRect>().0, rect);
        let window = window(&mut app);
        assert_eq!(
            window.position,
            WindowPosition::At(IVec2::new(rect.left, rect.top))
        );
        assert_eq!(
            (
                window.resolution.physical_width(),
                window.resolution.physical_height()
            ),
            (rect.width() as u32, rect.height() as u32)
        );
    }

    #[test]
    fn given_the_overlay_on_the_work_area_when_the_work_area_is_set_to_the_same_then_the_window_is_left_alone()
     {
        // Given
        let mut app = engine();
        let before = window(&mut app);

        // When
        app.world_mut().resource_mut::<WorkArea>().0 = WORK_AREA;
        app.update();

        // Then
        let after = window(&mut app);
        assert_eq!(after.position, before.position);
        assert_eq!(after.resolution, before.resolution);
        assert_eq!(
            app.world().resource::<OverlayRect>().0,
            overlay_rect(WORK_AREA)
        );
    }
}
