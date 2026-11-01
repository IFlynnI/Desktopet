//! The Bevy adapter: draws the Pet on the Overlay.

mod log_file;
mod overlay;
mod placeholder;
mod presence;
mod render_config;
#[cfg(windows)]
mod render_config_windows;
mod render_failure;

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, mpsc};

use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::render::pipelined_rendering::PipelinedRenderingPlugin;
use pet_core::geometry::Rect;
use pet_core::presence::Presence;

use overlay::{OverlayRect, PlatformHandle, ShellEventSender};
use placeholder::PlaceholderPlugin;
use presence::{CurrentPresence, ManualHideAtExit, ShellEvents};

pub use render_failure::{RENDER_FAILURE_EXIT, is_render_failure};

pub fn app(work_area: Rect, start: Presence, manual_hide_at_exit: Arc<AtomicBool>) -> App {
    let rect = overlay::overlay_rect(work_area);
    let (sender, receiver) = mpsc::channel();
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::NONE))
        .insert_resource(render_config::winit_settings(false))
        .insert_resource(OverlayRect(rect))
        .insert_resource(render_failure::handler())
        .insert_resource(CurrentPresence(start))
        .insert_resource(ManualHideAtExit(manual_hide_at_exit))
        .insert_resource(ShellEventSender(sender))
        .insert_resource(ShellEvents(Mutex::new(receiver)))
        .add_plugins(
            DefaultPlugins
                .build()
                .disable::<PipelinedRenderingPlugin>()
                .set(LogPlugin {
                    custom_layer: log_file::layer,
                    ..default()
                })
                .set(overlay::window_plugin(rect, render_config::ALPHA_MODE))
                .set(render_config::render_plugin()),
        )
        .add_plugins(PlaceholderPlugin { work_area })
        .add_systems(Startup, overlay::spawn_camera)
        .add_systems(
            Update,
            (
                overlay::attach_platform.run_if(not(resource_exists::<PlatformHandle>)),
                presence::exit_when_close_requested,
                presence::drain_shell_events,
                presence::apply_presence.run_if(resource_exists::<PlatformHandle>.and_then(
                    resource_changed::<CurrentPresence>.or_eager(resource_added::<PlatformHandle>),
                )),
            )
                .chain(),
        )
        .add_systems(Last, presence::wake_for_exit);
    app
}
