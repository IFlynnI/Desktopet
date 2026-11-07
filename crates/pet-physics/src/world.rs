//! What a Body collides with: the screen's edges, the Ground plane and the Obstacles. All of it holds still, which
//! is what lets a sweep against it be exact.

use crate::geometry::Rect;
use crate::material::Material;

/// A solid rectangle, in absolute screen coordinates, solid on all four sides, so a Body can stand on top of one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Obstacle {
    pub rect: Rect,
    pub material: Material,
}

/// The space Bodies move in.
#[derive(Clone, Debug, PartialEq)]
pub struct World {
    /// `left`, `top` and `right` are the screen's edges and `bottom` is the Ground plane. A Body's box stays inside.
    pub bounds: Rect,
    /// The Material of the Ground plane and the edges.
    pub surface: Material,
    /// In physical pixels per second squared, pointing down.
    pub gravity: f32,
    pub obstacles: Vec<Obstacle>,
}
