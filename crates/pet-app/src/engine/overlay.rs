//! The transparent window the Pet is drawn on.

use std::sync::mpsc::Sender;

use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::render::view::Msaa;
use bevy::window::{
    CompositeAlphaMode, CursorOptions, PresentMode, PrimaryWindow, RawHandleWrapper, WindowLevel,
    WindowResolution,
};
use bevy::winit::{EventLoopProxyWrapper, WinitUserEvent};
use pet_core::geometry::{Point, Rect};
use pet_platform::{Platform, ShellEvent};

use super::placeholder;

/// The Overlay's rect on screen, in physical pixels.
#[derive(Resource, Clone, Copy)]
pub struct OverlayRect(pub Rect);

#[derive(Resource)]
pub struct PlatformHandle(pub Platform);

/// The sending end of the channel, waiting to go to `Platform::start`.
#[derive(Resource)]
pub struct ShellEventSender(pub Sender<ShellEvent>);

/// The largest even-sized rect in the bottom-left of `work_area`.
pub fn overlay_rect(work_area: Rect) -> Rect {
    let (width, height) = (work_area.width() & !1, work_area.height() & !1);
    Rect {
        left: work_area.left,
        top: work_area.bottom - height,
        right: work_area.left + width,
        bottom: work_area.bottom,
    }
}

/// Bevy's position for a box drawn at `top_left` on screen: relative to the Overlay's centre, with y up.
pub fn sprite_translation(overlay: Rect, top_left: Point, width: i32, height: i32) -> Vec3 {
    let x = top_left.x + width / 2 - (overlay.left + overlay.right) / 2;
    let y = (overlay.top + overlay.bottom) / 2 - (top_left.y + height / 2);
    Vec3::new(x as f32, y as f32, 0.0)
}

pub fn window_plugin(rect: Rect, composite_alpha_mode: CompositeAlphaMode) -> WindowPlugin {
    WindowPlugin {
        primary_window: Some(Window {
            title: "Desktop Pet".into(),
            decorations: false,
            transparent: true,
            resizable: false,
            focused: false,
            // Invisible until `attach_platform` has set what must be in place before the first show. After
            // that, Presence decides.
            visible: false,
            composite_alpha_mode,
            window_level: WindowLevel::AlwaysOnTop,
            skip_taskbar: true,
            position: WindowPosition::At(IVec2::new(rect.left, rect.top)),
            resolution: WindowResolution::new(rect.width() as u32, rect.height() as u32)
                .with_scale_factor_override(1.0),
            present_mode: PresentMode::AutoVsync,
            ..default()
        }),
        primary_cursor_options: Some(CursorOptions {
            hit_test: false,
            ..default()
        }),
        ..default()
    }
}

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera2d, Msaa::Off));
}

/// Has to run on the main thread. `Platform::start` changes the Overlay's styles, and Windows answers that
/// on the thread that owns the window. From a worker thread it would wait on the main thread, which is
/// itself waiting for this system to finish.
pub fn attach_platform(
    mut commands: Commands,
    windows: Query<&RawHandleWrapper, With<PrimaryWindow>>,
    events: Res<ShellEventSender>,
    proxy: Res<EventLoopProxyWrapper>,
    _main_thread: NonSendMarker,
) {
    let Ok(handle) = windows.single() else {
        return;
    };
    let proxy = (*proxy).clone();
    let wake = Box::new(move || {
        // This fails only once the engine has gone, and then there's nothing to wake.
        let _ = proxy.send_event(WinitUserEvent::WakeUp);
    });
    let window = handle.get_window_handle();
    let tray_icon = placeholder::tray_icon_image();
    let platform = Platform::start(window, events.0.clone(), wake, tray_icon);
    commands.insert_resource(PlatformHandle(platform));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_an_odd_sized_work_area_when_fitting_the_overlay_then_its_size_is_even_and_it_keeps_the_bottom_left_corner()
     {
        // Given: an odd size would make the centre fall between pixels and blur the art
        let work_area = Rect {
            left: 100,
            top: 50,
            right: 1101,
            bottom: 851,
        };

        // When
        let rect = overlay_rect(work_area);

        // Then
        assert_eq!(
            rect,
            Rect {
                left: 100,
                top: 51,
                right: 1100,
                bottom: 851
            }
        );
    }

    #[test]
    fn given_a_position_on_screen_when_converting_for_bevy_then_it_is_relative_to_the_overlay_centre_with_y_up()
     {
        // Given
        let overlay = Rect {
            left: 0,
            top: 0,
            right: 1000,
            bottom: 800,
        };
        let top_left = Point { x: 100, y: 700 };

        // When
        let translation = sprite_translation(overlay, top_left, 48, 36);

        // Then: the box's centre is (124, 718), the Overlay's is (500, 400)
        assert_eq!(translation, Vec3::new(-376.0, -318.0, 0.0));
    }
}
