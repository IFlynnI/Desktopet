//! The Bevy adapter: draws the Pet on the Overlay.

mod emote_view;
mod hot_reload;
mod log_file;
mod overlay;
mod pack_assets;
mod pixel_art;
mod presence;
mod render_config;
#[cfg(windows)]
mod render_config_windows;
mod render_failure;
mod renderer;
mod shadow;
#[cfg(test)]
mod test_support;
mod tray_icon;

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, mpsc};

use bevy::app::PluginGroupBuilder;
use bevy::camera::CompositingSpace;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::render::pipelined_rendering::PipelinedRenderingPlugin;
use bevy::render::view::Msaa;
use bevy::window::ExitCondition;
use pet_core::geometry::Rect;
use pet_core::presence::Presence;
use pet_sim::SimPlugin;

use emote_view::EmoteViewPlugin;
use overlay::{OverlayRect, PlatformHandle, ShellEventSender};
use pixel_art::PixelArtPlugin;
use presence::{CurrentPresence, ManualHideAtExit, ShellEvents};
use renderer::RendererPlugin;
use shadow::ShadowPlugin;

pub use hot_reload::PackEdited;
pub(crate) use hot_reload::Watching;
pub use pack_assets::PackAtStartup;
pub use render_config::render_plugin;
pub use render_failure::{RENDER_FAILURE_EXIT, is_render_failure};

use crate::pack::StartingPack;

/// Draws the Pet the simulation shows, its shadow and its Emote, into whatever camera exists, on a clear background.
/// `overlay` is the rect the camera covers, in physical pixels. The Pack comes from `PackAtStartup`, and an edited one
/// replaces it when a `PackEdited` arrives.
pub struct PetRenderPlugin {
    pub overlay: Rect,
}

impl Plugin for PetRenderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::NONE))
            .insert_resource(OverlayRect(self.overlay))
            .add_message::<PackEdited>()
            .add_systems(
                Startup,
                pack_assets::add_starting_pack.run_if(resource_exists::<PackAtStartup>),
            )
            // Before the ticks and the renderer, so both only ever see one Pack.
            .add_systems(
                PreUpdate,
                hot_reload::apply_edits.run_if(on_message::<PackEdited>),
            )
            .add_plugins((
                PixelArtPlugin,
                ShadowPlugin,
                RendererPlugin,
                EmoteViewPlugin,
            ));
    }
}

/// The camera the Pet is drawn with. Pixel art needs no smoothing of its edges. Half-transparent pixels blend in sRGB,
/// as an image editor shows them, not in linear light. Over the clear background that leaves each pixel premultiplied
/// in sRGB, which is what the see-through window's compositor expects, so it shows the Pack's colour over whatever is
/// behind. Bevy does it with no extra pass: the copy to the window it always makes turns the sRGB values back.
pub fn pet_camera() -> impl Bundle {
    (Camera2d, Msaa::Off, CompositingSpace::Srgb)
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

/// The engine, ready to run. With no usable Pack it still runs, with no Pet, so the tray can still quit. `watching`
/// is the chosen Pack's folder being watched for edits, started before the Pack was read.
pub(crate) fn app(
    work_area: Rect,
    start: Presence,
    manual_hide_at_exit: Arc<AtomicBool>,
    pack: StartingPack,
    watching: Watching,
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
    watching.install(&mut app);
    app
}
