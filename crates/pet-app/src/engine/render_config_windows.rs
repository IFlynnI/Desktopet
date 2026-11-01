//! The Windows half of the render settings. These are Bevy types, so they can't live in `pet-platform`
//! (ADR-0001).

use bevy::render::settings::Backends;
use bevy::window::CompositeAlphaMode;

/// Vulkan cost 231.6 MB against DX12's 91.9 MB in the spike.
pub const BACKENDS: Backends = Backends::DX12;

pub const ALPHA_MODE: CompositeAlphaMode = CompositeAlphaMode::PreMultiplied;
