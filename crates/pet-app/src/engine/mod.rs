//! The Bevy adapter: draws the Pet on the Overlay.

mod log_file;
mod overlay;
mod pack_assets;
mod presence;
mod render_config;
#[cfg(windows)]
mod render_config_windows;
mod render_failure;
mod renderer;
mod tray_icon;

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, mpsc};

use bevy::app::PluginGroupBuilder;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::render::pipelined_rendering::PipelinedRenderingPlugin;
use bevy::render::view::Msaa;
use bevy::window::ExitCondition;
use pet_core::geometry::Rect;
use pet_core::presence::Presence;
use pet_sim::SimPlugin;

use overlay::{OverlayRect, PlatformHandle, ShellEventSender};
use presence::{CurrentPresence, ManualHideAtExit, ShellEvents};
use renderer::RendererPlugin;

pub use pack_assets::PackAtStartup;
pub use render_config::render_plugin;
pub use render_failure::{RENDER_FAILURE_EXIT, is_render_failure};

use crate::pack::StartingPack;

/// Draws the Pet the simulation shows into whatever camera exists, on a clear background. `overlay` is the rect the
/// camera covers, in physical pixels. The Pack comes from `PackAtStartup`.
pub struct PetRenderPlugin {
    pub overlay: Rect,
}

impl Plugin for PetRenderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::NONE))
            .insert_resource(OverlayRect(self.overlay))
            .add_systems(
                Startup,
                pack_assets::add_starting_pack.run_if(resource_exists::<PackAtStartup>),
            )
            .add_plugins(RendererPlugin);
    }
}

/// The camera the Pet is drawn with. Pixel art needs no smoothing of its edges.
pub fn pet_camera() -> impl Bundle {
    (Camera2d, Msaa::Off)
}

/// Bevy with the render settings the Pet needs, and no window yet. Pipelined rendering is off: it would hold a frame
/// back for a speed-up the Pet doesn't need.
pub fn base_plugins() -> PluginGroupBuilder {
    DefaultPlugins
        .build()
        .disable::<PipelinedRenderingPlugin>()
        .set(WindowPlugin {
            primary_window: None,
            primary_cursor_options: None,
            exit_condition: ExitCondition::DontExit,
            close_when_requested: false,
        })
        .set(render_config::render_plugin())
}

/// The engine, ready to run. With no usable Pack it still runs, with no Pet, so the tray can still quit.
pub fn app(
    work_area: Rect,
    start: Presence,
    manual_hide_at_exit: Arc<AtomicBool>,
    pack: StartingPack,
) -> App {
    let rect = overlay::overlay_rect(work_area);
    let (sender, receiver) = mpsc::channel();
    let mut app = App::new();
    app.insert_resource(render_config::winit_settings(false))
        .insert_resource(render_failure::handler())
        .insert_resource(CurrentPresence(start))
        .insert_resource(ManualHideAtExit(manual_hide_at_exit))
        .insert_resource(ShellEventSender(sender))
        .insert_resource(ShellEvents(Mutex::new(receiver)))
        .insert_resource(PackAtStartup(pack))
        .add_plugins(
            base_plugins()
                .set(LogPlugin {
                    custom_layer: log_file::layer,
                    ..default()
                })
                .set(overlay::window_plugin(rect, render_config::ALPHA_MODE)),
        )
        .add_plugins((SimPlugin { work_area }, PetRenderPlugin { overlay: rect }))
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
