//! The Bevy adapter: draws the Pet on the Overlay.

mod overlay;
mod placeholder;
mod render_config;
#[cfg(windows)]
mod render_config_windows;

use bevy::prelude::*;
use bevy::render::pipelined_rendering::PipelinedRenderingPlugin;
use pet_core::geometry::Rect;

use overlay::{OverlayRect, PlatformHandle};
use placeholder::PlaceholderPlugin;

pub fn app(work_area: Rect) -> App {
    let rect = overlay::overlay_rect(work_area);
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::NONE))
        .insert_resource(render_config::winit_settings())
        .insert_resource(OverlayRect(rect))
        .add_plugins(
            DefaultPlugins
                .build()
                .disable::<PipelinedRenderingPlugin>()
                .set(overlay::window_plugin(rect, render_config::ALPHA_MODE))
                .set(render_config::render_plugin()),
        )
        .add_plugins(PlaceholderPlugin { work_area })
        .add_systems(Startup, overlay::spawn_camera)
        .add_systems(
            Update,
            overlay::attach_platform.run_if(not(resource_exists::<PlatformHandle>)),
        );
    app
}
