//! Pure per-pixel hit testing: is a screen-space cursor over an opaque sprite pixel?
//! No Win32, no Bevy — so it can be tested in isolation.

/// Which pixels of one sprite frame are opaque.
#[derive(Clone, Debug)]
pub struct AlphaMask {
    width: u32,
    height: u32,
    opaque: Vec<bool>,
}

impl AlphaMask {
    /// Build from tightly packed RGBA8 pixels; a pixel is opaque when alpha >= `threshold`.
    pub fn from_rgba(width: u32, height: u32, rgba: &[u8], threshold: u8) -> Self {
        let opaque = rgba.chunks_exact(4).map(|px| px[3] >= threshold).collect();
        Self { width, height, opaque }
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
    /// Screen x of the sprite's top-left corner.
    pub left: i32,
    /// Screen y of the sprite's top-left corner.
    pub top: i32,
    /// Integer nearest-neighbour scale (1 = one sprite pixel per screen pixel).
    pub scale: u32,
    /// Mirrored horizontally (pet facing the other way).
    pub flip_x: bool,
}

/// True when the screen-space `cursor` lies on an opaque pixel of `mask` drawn at `placement`.
pub fn hits(mask: &AlphaMask, placement: &Placement, cursor: (i32, i32)) -> bool {
    let scale = placement.scale.max(1) as i64;
    let dx = cursor.0 as i64 - placement.left as i64;
    let dy = cursor.1 as i64 - placement.top as i64;
    if dx < 0 || dy < 0 {
        return false;
    }
    let (x, y) = (dx / scale, dy / scale);
    if x >= mask.width() as i64 || y >= mask.height() as i64 {
        return false;
    }
    let x = if placement.flip_x { mask.width() as i64 - 1 - x } else { x };
    mask.is_opaque(x as u32, y as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPAQUE: [u8; 4] = [255, 255, 255, 255];
    const CLEAR: [u8; 4] = [0, 0, 0, 0];

    /// 3x2 mask:
    /// row 0: X . .
    /// row 1: X X .
    fn l_shape() -> AlphaMask {
        let pixels = [OPAQUE, CLEAR, CLEAR, OPAQUE, OPAQUE, CLEAR];
        AlphaMask::from_rgba(3, 2, &pixels.concat(), 128)
    }

    fn at(left: i32, top: i32, scale: u32, flip_x: bool) -> Placement {
        Placement { left, top, scale, flip_x }
    }

    #[test]
    fn given_rgba_with_partial_alpha_when_building_mask_then_threshold_decides_opacity() {
        // Given
        let pixels = [[0, 0, 0, 127], [0, 0, 0, 128]].concat();

        // When
        let mask = AlphaMask::from_rgba(2, 1, &pixels, 128);

        // Then
        assert!(!mask.is_opaque(0, 0));
        assert!(mask.is_opaque(1, 0));
    }

    #[test]
    fn given_scaled_sprite_when_cursor_on_any_screen_pixel_of_an_opaque_sprite_pixel_then_hits() {
        // Given
        let mask = l_shape();
        let placement = at(100, 50, 4, false);

        // When / Then: sprite pixel (0,0) covers screen x 100..=103, y 50..=53
        assert!(hits(&mask, &placement, (100, 50)));
        assert!(hits(&mask, &placement, (103, 53)));
    }

    #[test]
    fn given_scaled_sprite_when_cursor_on_transparent_sprite_pixel_then_misses() {
        // Given
        let mask = l_shape();
        let placement = at(100, 50, 4, false);

        // When / Then: sprite pixel (1,0) is clear → screen x 104..=107, y 50..=53
        assert!(!hits(&mask, &placement, (104, 50)));
        assert!(!hits(&mask, &placement, (107, 53)));
    }

    #[test]
    fn given_sprite_when_cursor_just_outside_its_bounds_then_misses() {
        // Given
        let mask = l_shape();
        let placement = at(100, 50, 4, false);

        // When / Then
        assert!(!hits(&mask, &placement, (99, 50)));
        assert!(!hits(&mask, &placement, (100, 49)));
        assert!(!hits(&mask, &placement, (112, 50))); // right edge: 100 + 3*4
        assert!(!hits(&mask, &placement, (100, 58))); // bottom edge: 50 + 2*4
    }

    #[test]
    fn given_sprite_at_negative_coordinates_when_cursor_left_of_it_then_misses() {
        // Given: sprite partly off the left of the virtual screen (multi-monitor)
        let mask = l_shape();
        let placement = at(-6, 0, 2, false);

        // When / Then: (-7, 0) is outside; (-6, 0) is sprite pixel (0,0)
        assert!(!hits(&mask, &placement, (-7, 0)));
        assert!(hits(&mask, &placement, (-6, 0)));
    }

    #[test]
    fn given_flipped_sprite_when_cursor_on_mirrored_opaque_pixel_then_hits() {
        // Given: flipped, row 0 reads ". . X"
        let mask = l_shape();
        let placement = at(0, 0, 1, true);

        // When / Then
        assert!(!hits(&mask, &placement, (0, 0)));
        assert!(hits(&mask, &placement, (2, 0)));
        assert!(hits(&mask, &placement, (1, 1)));
        assert!(!hits(&mask, &placement, (0, 1)));
    }
}
