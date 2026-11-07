//! Screen geometry in physical pixels. The types live in `pet-physics`, which owns them, and are re-exported here so
//! code that names `pet_core::geometry` keeps working.

pub use pet_physics::geometry::{Point, Rect};
