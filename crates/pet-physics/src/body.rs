//! A moving Body: an upright box that never rotates, so the art, the Hit mask, the shadow and the Emotes stay as they
//! are however it flies.

use crate::material::{Material, MaterialOverride};
use crate::vector::Vec2;

/// A Body's box relative to its Anchor, in physical pixels. The Anchor is where the feet are, so `left` and `top` are
/// usually negative.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Extent {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

impl Extent {
    /// The Pack's Body, given in art pixels relative to the Anchor, at the Pet's integer scale. Physics works in
    /// physical pixels so a throw feels the same at every scale.
    pub fn scaled(x: i32, y: i32, width: u32, height: u32, scale: u32) -> Self {
        let scale = scale as f32;
        Self {
            left: x as f32 * scale,
            top: y as f32 * scale,
            width: width as f32 * scale,
            height: height as f32 * scale,
        }
    }
}

/// One moving Body. It is a plain value: `step` moves it, and nothing else about it lives anywhere else.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    /// Where the feet are, in physical screen pixels.
    pub anchor: Vec2,
    /// In physical pixels per second.
    pub velocity: Vec2,
    pub extent: Extent,
    pub material: Material,
    /// What the running Action or Situation sets, which wins over the combine rule.
    pub overrides: MaterialOverride,
    /// Whether it ended its last step standing on an upward-facing surface. A Body placed on the Ground plane can
    /// start with this set, so its first step doesn't report a Landing.
    pub standing: bool,
    /// The hardest it has come down onto an upward-facing surface since it last stood, which its Landing reports.
    /// Only the time nobody held it counts, so a bump in the owner's hand never shows up in a later Landing.
    pub(crate) touchdown: f32,
}

impl Body {
    /// A Body at rest and airborne. Set `standing` if it is placed on a surface.
    pub fn new(anchor: Vec2, extent: Extent, material: Material) -> Self {
        Self {
            anchor,
            velocity: Vec2::ZERO,
            extent,
            material,
            overrides: MaterialOverride::default(),
            standing: false,
            touchdown: 0.0,
        }
    }
}

/// The owner lets go. The Body keeps whatever speed the string gave it, so there is no throw or drop threshold, but
/// never more than `max_speed`, because a swing that resonates with the string can reach several times the cursor's
/// own speed.
pub fn release(body: &mut Body, max_speed: f32) {
    let speed = body.velocity.length();
    if speed > max_speed {
        body.velocity = body.velocity * (max_speed / speed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MATERIAL: Material = Material {
        friction: 0.5,
        bounciness: 0.1,
    };

    #[test]
    fn given_a_pack_body_in_art_pixels_when_scaled_then_every_value_is_multiplied_by_the_scale() {
        // Given: 16 art pixels left of the Anchor, 30 above it, 32 wide and 30 tall
        let (x, y, width, height) = (-16, -30, 32, 30);

        // When
        let extent = Extent::scaled(x, y, width, height, 3);

        // Then
        assert_eq!(
            extent,
            Extent {
                left: -48.0,
                top: -90.0,
                width: 96.0,
                height: 90.0
            }
        );
    }

    #[test]
    fn given_a_body_faster_than_the_cap_when_released_then_its_speed_is_the_cap_in_the_same_direction()
     {
        // Given
        let mut body = Body::new(Vec2::ZERO, Extent::scaled(0, -1, 1, 1, 1), MATERIAL);
        body.velocity = Vec2::new(3000.0, -4000.0);

        // When
        release(&mut body, 2500.0);

        // Then
        assert!((body.velocity.length() - 2500.0).abs() < 1e-2);
        assert!((body.velocity.x - 1500.0).abs() < 1e-2);
        assert!((body.velocity.y + 2000.0).abs() < 1e-2);
    }

    #[test]
    fn given_a_body_slower_than_the_cap_when_released_then_it_keeps_its_velocity() {
        // Given
        let mut body = Body::new(Vec2::ZERO, Extent::scaled(0, -1, 1, 1, 1), MATERIAL);
        body.velocity = Vec2::new(300.0, -400.0);

        // When
        release(&mut body, 2500.0);

        // Then
        assert_eq!(body.velocity, Vec2::new(300.0, -400.0));
    }
}
