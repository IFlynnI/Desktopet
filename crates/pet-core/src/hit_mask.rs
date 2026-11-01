//! Per-pixel hit testing: is a screen position on an opaque pixel of the Pet?

use crate::geometry::Point;

/// Which pixels of one frame of art catch clicks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HitMask {
    width: u32,
    height: u32,
    opaque: Vec<bool>,
}

impl HitMask {
    /// Builds a mask from tightly packed RGBA8 pixels. A pixel is opaque when alpha >= `threshold`.
    pub fn from_rgba(width: u32, height: u32, rgba: &[u8], threshold: u8) -> Self {
        let opaque = rgba.chunks_exact(4).map(|px| px[3] >= threshold).collect();
        Self {
            width,
            height,
            opaque,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn is_opaque(&self, x: u32, y: u32) -> bool {
        x < self.width && y < self.height && self.opaque[(y * self.width + x) as usize]
    }
}

/// Where a frame is drawn on screen, in physical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Placement {
    pub top_left: Point,
    /// Integer nearest-neighbour scale (1 = one sprite pixel per screen pixel).
    pub scale: u32,
    /// Mirrored horizontally, for a Pet facing the other way.
    pub flip_x: bool,
}

/// True when `cursor` lies on an opaque pixel of `mask` drawn at `placement`.
pub fn hits(mask: &HitMask, placement: &Placement, cursor: Point) -> bool {
    let scale = placement.scale.max(1) as i64;
    let dx = cursor.x as i64 - placement.top_left.x as i64;
    let dy = cursor.y as i64 - placement.top_left.y as i64;
    if dx < 0 || dy < 0 {
        return false;
    }
    let (x, y) = (dx / scale, dy / scale);
    if x >= mask.width() as i64 || y >= mask.height() as i64 {
        return false;
    }
    let x = if placement.flip_x {
        mask.width() as i64 - 1 - x
    } else {
        x
    };
    mask.is_opaque(x as u32, y as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    const OPAQUE: [u8; 4] = [255, 255, 255, 255];
    const CLEAR: [u8; 4] = [0, 0, 0, 0];

    /// 3x2 mask:
    /// row 0: X . .
    /// row 1: X X .
    fn l_shape() -> HitMask {
        let pixels = [OPAQUE, CLEAR, CLEAR, OPAQUE, OPAQUE, CLEAR];
        HitMask::from_rgba(3, 2, &pixels.concat(), 128)
    }

    fn at(x: i32, y: i32, scale: u32, flip_x: bool) -> Placement {
        Placement {
            top_left: Point { x, y },
            scale,
            flip_x,
        }
    }

    fn point(x: i32, y: i32) -> Point {
        Point { x, y }
    }

    #[test]
    fn given_rgba_with_partial_alpha_when_building_a_mask_then_the_threshold_decides_opacity() {
        // Given
        let pixels = [[0, 0, 0, 127], [0, 0, 0, 128]].concat();

        // When
        let mask = HitMask::from_rgba(2, 1, &pixels, 128);

        // Then
        assert!(!mask.is_opaque(0, 0));
        assert!(mask.is_opaque(1, 0));
    }

    #[test]
    fn given_a_scaled_pet_when_the_cursor_is_on_any_screen_pixel_of_an_opaque_sprite_pixel_then_it_hits()
     {
        // Given
        let mask = l_shape();
        let placement = at(100, 50, 4, false);

        // When / Then: sprite pixel (0,0) covers screen x 100..=103, y 50..=53
        assert!(hits(&mask, &placement, point(100, 50)));
        assert!(hits(&mask, &placement, point(103, 53)));
    }

    #[test]
    fn given_a_scaled_pet_when_the_cursor_is_on_a_transparent_sprite_pixel_then_it_misses() {
        // Given
        let mask = l_shape();
        let placement = at(100, 50, 4, false);

        // When / Then: sprite pixel (1,0) is clear, so screen x 104..=107, y 50..=53
        assert!(!hits(&mask, &placement, point(104, 50)));
        assert!(!hits(&mask, &placement, point(107, 53)));
    }

    #[test]
    fn given_a_pet_when_the_cursor_is_just_outside_its_bounds_then_it_misses() {
        // Given
        let mask = l_shape();
        let placement = at(100, 50, 4, false);

        // When / Then
        assert!(!hits(&mask, &placement, point(99, 50)));
        assert!(!hits(&mask, &placement, point(100, 49)));
        assert!(!hits(&mask, &placement, point(112, 50))); // right edge: 100 + 3*4
        assert!(!hits(&mask, &placement, point(100, 58))); // bottom edge: 50 + 2*4
    }

    #[test]
    fn given_a_pet_at_negative_coordinates_when_the_cursor_is_left_of_it_then_it_misses() {
        // Given: a work area can start left of the primary monitor's origin
        let mask = l_shape();
        let placement = at(-6, 0, 2, false);

        // When / Then: (-7, 0) is outside; (-6, 0) is sprite pixel (0,0)
        assert!(!hits(&mask, &placement, point(-7, 0)));
        assert!(hits(&mask, &placement, point(-6, 0)));
    }

    #[test]
    fn given_a_flipped_pet_when_the_cursor_is_on_the_mirrored_opaque_pixel_then_it_hits() {
        // Given: flipped, row 0 reads ". . X"
        let mask = l_shape();
        let placement = at(0, 0, 1, true);

        // When / Then
        assert!(!hits(&mask, &placement, point(0, 0)));
        assert!(hits(&mask, &placement, point(2, 0)));
        assert!(hits(&mask, &placement, point(1, 1)));
        assert!(!hits(&mask, &placement, point(0, 1)));
    }
}
