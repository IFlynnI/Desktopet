//! Applies Presence to the engine. Only this changes `Window.visible`, because Bevy keeps its own record of it.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowCloseRequested};
use bevy::winit::{EventLoopProxyWrapper, WinitSettings, WinitUserEvent};
use pet_core::presence::Presence;
use pet_platform::ShellEvent;

use super::overlay::PlatformHandle;
use super::render_config;

#[derive(Resource, Deref, DerefMut)]
pub struct CurrentPresence(pub Presence);

/// Shared with `pet_app::run`, which needs the Manual hide for a Relaunch after the world has gone.
#[derive(Resource)]
pub struct ManualHideAtExit(pub Arc<AtomicBool>);

/// What the shell thread has sent. It's in a `Mutex` only because Bevy needs resources that threads can share, and a
/// `Receiver` on its own can't be.
#[derive(Resource)]
pub struct ShellEvents(pub Mutex<Receiver<ShellEvent>>);

pub fn drain_shell_events(
    events: Res<ShellEvents>,
    mut presence: ResMut<CurrentPresence>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ok(events) = events.0.lock() else { return };
    for event in events.try_iter() {
        match event {
            ShellEvent::Toggle => presence.toggle(),
            ShellEvent::Show => presence.show(),
            ShellEvent::Quit => {
                exit.write(AppExit::Success);
            }
            ShellEvent::FullscreenChanged(fullscreen) => presence.set_auto_hide(fullscreen),
        }
    }
}

/// Bevy closes a window over two updates, and a Hidden engine never runs the second, so this exits straight away.
pub fn exit_when_close_requested(
    mut requests: MessageReader<WindowCloseRequested>,
    mut exit: MessageWriter<AppExit>,
) {
    if requests.read().next().is_some() {
        exit.write(AppExit::Success);
    }
}

/// winit notices an exit only when its next message arrives, and a stopped engine has none coming, so this sends one.
pub fn wake_for_exit(mut exits: MessageReader<AppExit>, proxy: Res<EventLoopProxyWrapper>) {
    if exits.read().next().is_some() {
        // This fails only if the event loop has already gone, which is the outcome this is after.
        let _ = proxy.send_event(WinitUserEvent::WakeUp);
    }
}

/// While Hidden, everything stops. The world freezes, so the Pet reappears where it was. A wake-up from the shell
/// thread still runs an update, which is how this gets undone.
pub fn apply_presence(
    presence: Res<CurrentPresence>,
    manual_hide_at_exit: Res<ManualHideAtExit>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut cameras: Query<&mut Camera>,
    mut settings: ResMut<WinitSettings>,
    platform: Option<Res<PlatformHandle>>,
) {
    let hidden = presence.is_hidden();
    manual_hide_at_exit
        .0
        .store(presence.manual_hide(), Ordering::Relaxed);
    if let Ok(mut window) = windows.single_mut() {
        window.visible = !hidden;
    }
    for mut camera in &mut cameras {
        camera.is_active = !hidden;
    }
    *settings = render_config::winit_settings(hidden);
    if let Some(platform) = platform {
        platform.0.set_hidden(hidden);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::{self, Sender};
    use std::time::Duration;

    use bevy::winit::UpdateMode;

    use super::*;

    struct Engine {
        app: App,
        shell: Sender<ShellEvent>,
        manual_hide_at_exit: Arc<AtomicBool>,
    }

    impl Engine {
        fn starting_with(presence: Presence) -> Self {
            let (shell, receiver) = mpsc::channel();
            let manual_hide_at_exit = Arc::new(AtomicBool::new(presence.manual_hide()));
            let mut app = App::new();
            app.insert_resource(CurrentPresence(presence))
                .insert_resource(ManualHideAtExit(manual_hide_at_exit.clone()))
                .insert_resource(ShellEvents(Mutex::new(receiver)))
                .insert_resource(render_config::winit_settings(false))
                .add_message::<AppExit>()
                .add_message::<WindowCloseRequested>()
                .add_systems(
                    Update,
                    (
                        exit_when_close_requested,
                        drain_shell_events,
                        apply_presence,
                    )
                        .chain(),
                );
            app.world_mut().spawn((Window::default(), PrimaryWindow));
            app.world_mut().spawn(Camera::default());
            Self {
                app,
                shell,
                manual_hide_at_exit,
            }
        }

        fn is_visible(&mut self) -> bool {
            let world = self.app.world_mut();
            world.query::<&Window>().single(world).unwrap().visible
        }

        fn camera_is_active(&mut self) -> bool {
            let world = self.app.world_mut();
            world.query::<&Camera>().single(world).unwrap().is_active
        }

        fn wait(&self) -> Duration {
            let settings = self.app.world().resource::<WinitSettings>();
            assert_eq!(settings.focused_mode, settings.unfocused_mode);
            match settings.unfocused_mode {
                UpdateMode::Reactive { wait, .. } => wait,
                UpdateMode::Continuous => panic!("the engine never runs continuously"),
            }
        }
    }

    #[test]
    fn given_a_shown_pet_when_a_toggle_arrives_then_the_engine_stops_completely() {
        // Given
        let mut engine = Engine::starting_with(Presence::default());
        engine.app.update();
        assert!(engine.is_visible());

        // When
        engine.shell.send(ShellEvent::Toggle).unwrap();
        engine.app.update();

        // Then
        assert!(!engine.is_visible());
        assert!(!engine.camera_is_active());
        assert_eq!(engine.wait(), Duration::MAX);
    }

    #[test]
    fn given_a_manual_hide_when_a_toggle_arrives_then_the_engine_runs_again() {
        // Given
        let mut engine = Engine::starting_with(Presence::with_manual_hide(true));
        engine.app.update();
        assert!(!engine.is_visible());

        // When
        engine.shell.send(ShellEvent::Toggle).unwrap();
        engine.app.update();

        // Then
        assert!(engine.is_visible());
        assert!(engine.camera_is_active());
        assert!(engine.wait() < Duration::from_secs(1));
    }

    #[test]
    fn given_a_manual_hide_when_asked_to_show_then_the_pet_is_shown() {
        // Given
        let mut engine = Engine::starting_with(Presence::with_manual_hide(true));

        // When
        engine.shell.send(ShellEvent::Show).unwrap();
        engine.app.update();

        // Then
        assert!(engine.is_visible());
    }

    #[test]
    fn given_a_hidden_pet_when_the_overlay_is_asked_to_close_then_the_app_exits_in_that_one_update()
    {
        // Given
        let mut engine = Engine::starting_with(Presence::with_manual_hide(true));
        let window = engine
            .app
            .world_mut()
            .query_filtered::<Entity, With<Window>>()
            .single(engine.app.world())
            .unwrap();

        // When
        engine
            .app
            .world_mut()
            .write_message(WindowCloseRequested { window });
        engine.app.update();

        // Then
        assert_eq!(engine.app.should_exit(), Some(AppExit::Success));
    }

    #[test]
    fn given_a_hidden_pet_when_quit_arrives_then_the_app_exits_in_that_one_update() {
        // Given
        let mut engine = Engine::starting_with(Presence::with_manual_hide(true));

        // When
        engine.shell.send(ShellEvent::Quit).unwrap();
        engine.app.update();

        // Then
        assert_eq!(engine.app.should_exit(), Some(AppExit::Success));
    }

    #[test]
    fn given_a_manual_hide_is_set_or_cleared_when_updating_then_the_shared_copy_follows() {
        // Given
        let mut engine = Engine::starting_with(Presence::default());
        engine.shell.send(ShellEvent::Toggle).unwrap();

        // When
        engine.app.update();

        // Then
        assert!(engine.manual_hide_at_exit.load(Ordering::Relaxed));

        // When
        engine.shell.send(ShellEvent::Toggle).unwrap();
        engine.app.update();

        // Then
        assert!(!engine.manual_hide_at_exit.load(Ordering::Relaxed));
    }

    #[test]
    fn given_a_shown_pet_when_a_fullscreen_app_arrives_then_the_engine_stops_completely() {
        // Given
        let mut engine = Engine::starting_with(Presence::default());
        engine.app.update();

        // When
        engine
            .shell
            .send(ShellEvent::FullscreenChanged(true))
            .unwrap();
        engine.app.update();

        // Then
        assert!(!engine.is_visible());
        assert!(!engine.camera_is_active());
        assert_eq!(engine.wait(), Duration::MAX);
    }

    #[test]
    fn given_an_auto_hide_when_the_fullscreen_app_leaves_then_the_engine_runs_again() {
        // Given
        let mut engine = Engine::starting_with(Presence::default());
        engine
            .shell
            .send(ShellEvent::FullscreenChanged(true))
            .unwrap();
        engine.app.update();

        // When
        engine
            .shell
            .send(ShellEvent::FullscreenChanged(false))
            .unwrap();
        engine.app.update();

        // Then
        assert!(engine.is_visible());
        assert!(engine.camera_is_active());
        assert!(engine.wait() < Duration::from_secs(1));
    }

    #[test]
    fn given_a_manual_hide_when_a_fullscreen_app_comes_and_goes_then_the_pet_stays_hidden() {
        // Given
        let mut engine = Engine::starting_with(Presence::with_manual_hide(true));

        // When
        engine
            .shell
            .send(ShellEvent::FullscreenChanged(true))
            .unwrap();
        engine
            .shell
            .send(ShellEvent::FullscreenChanged(false))
            .unwrap();
        engine.app.update();

        // Then
        assert!(!engine.is_visible());
        assert!(engine.manual_hide_at_exit.load(Ordering::Relaxed));
    }

    #[test]
    fn given_an_auto_hide_when_the_app_exits_then_it_is_not_kept_as_a_manual_hide() {
        // Given
        let mut engine = Engine::starting_with(Presence::default());

        // When
        engine
            .shell
            .send(ShellEvent::FullscreenChanged(true))
            .unwrap();
        engine.app.update();

        // Then: a Relaunch keeps a Manual hide only, and checks for a fullscreen app itself
        assert!(!engine.is_visible());
        assert!(!engine.manual_hide_at_exit.load(Ordering::Relaxed));
    }

    #[test]
    fn given_two_events_at_once_when_updating_then_both_apply_in_order() {
        // Given
        let mut engine = Engine::starting_with(Presence::default());
        engine.shell.send(ShellEvent::Toggle).unwrap();
        engine.shell.send(ShellEvent::Toggle).unwrap();

        // When
        engine.app.update();

        // Then
        assert!(engine.is_visible());
    }
}
