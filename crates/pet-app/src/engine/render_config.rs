//! Render settings that keep the app inside the performance budget (PRD §6.2). Each one comes from the
//! spike's measurements in spikes/bevy-overlay/RESULTS.md.

use std::time::Duration;

use bevy::render::RenderPlugin;
use bevy::render::settings::{MemoryHints, WgpuSettings};
use bevy::winit::{UpdateMode, WinitSettings};

#[cfg(windows)]
use super::render_config_windows as os;

pub use os::ALPHA_MODE;

const FRAME_WAIT: Duration = Duration::from_nanos(1_000_000_000 / 30);

pub fn winit_settings() -> WinitSettings {
    WinitSettings {
        focused_mode: UpdateMode::reactive_low_power(FRAME_WAIT),
        unfocused_mode: UpdateMode::reactive_low_power(FRAME_WAIT),
    }
}

pub fn render_plugin() -> RenderPlugin {
    let settings = WgpuSettings {
        backends: Some(os::BACKENDS),
        memory_hints: MemoryHints::MemoryUsage,
        ..Default::default()
    };
    RenderPlugin {
        render_creation: settings.into(),
        ..Default::default()
    }
}
