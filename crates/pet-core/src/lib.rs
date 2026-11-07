//! Pure domain rules for Desktop Pet. No operating system, no Bevy, no IO. Its one dependency is `pet-physics`, which
//! has none of its own.
#![forbid(unsafe_code)]

/// The physics, which `pet-core` depends on, for whoever reaches it through `pet-core`.
pub use pet_physics as physics;

pub mod animation_clock;
pub mod emote;
pub mod fullscreen;
pub mod geometry;
pub mod gesture;
pub mod hit_mask;
pub mod pack;
pub mod placement;
pub mod presence;
pub mod relaunch;
pub mod settle;
pub mod shadow;
