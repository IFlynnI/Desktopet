//! The Pet's physics, in physical screen pixels: upright Bodies that never rotate, Materials, Obstacles, the Ground
//! plane and screen edges, and the springy grab string (ADR-0003). Pure and deterministic, with no dependencies, so
//! `pet-core` and the Mind can ask "if I do this, where do I land" and `pet-sim` runs it each tick.
#![forbid(unsafe_code)]

pub mod body;
mod collide;
pub mod geometry;
pub mod grab;
pub mod material;
pub mod step;
pub mod tuning;
pub mod vector;
pub mod world;

pub use body::{Body, Extent, release};
pub use grab::{CursorSample, GrabString, Held};
pub use material::{Material, MaterialOverride, combine};
pub use step::{Landing, Step, step};
pub use tuning::Tuning;
pub use vector::Vec2;
pub use world::{Obstacle, World};
