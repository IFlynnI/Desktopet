use std::sync::Arc;

use pet_core::geometry::Point;
use pet_core::hit_mask::{HitMask, Placement, hits};

/// What's drawn and where, published as one value so the hit test never pairs a new frame with an old
/// position.
#[derive(Clone, Debug)]
pub struct HitTarget {
    pub mask: Arc<HitMask>,
    pub placement: Placement,
}

impl HitTarget {
    pub fn contains(&self, cursor: Point) -> bool {
        hits(&self.mask, &self.placement, cursor)
    }
}
