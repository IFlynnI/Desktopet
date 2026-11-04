//! Where an Emote goes on screen. It floats above the Pet and to its right: centred on the right edge of the Body,
//! with its bottom a small gap above the top of the Body. It's pixel art like the Pet, at the Pet's integer scale and
//! on its texel grid. It's never mirrored with the Pet, because an Emote reads like text, and it stays on the right
//! whichever way the Pet faces. A Pack gives an Emote no Anchor of its own, so its bottom-centre is the point that
//! lines up. It never catches clicks.

use crate::geometry::{Point, Rect};
use crate::pack::{Body, CellSize};

/// How far the Emote's bottom is above the top of the Body, in Pack pixels: clear of the Pet's head, but close enough
/// to read as the Pet's.
pub const GAP: u32 = 2;

/// How far the Emote's centre is right of the Body's right edge, in Pack pixels. 0 centres it on that edge, so it
/// reads as the Pet's while leaving its face clear. More moves it further right, and less moves it in over the Body.
/// It's whole Pack pixels, so the Emote stays on the Pet's texel grid.
pub const RIGHT_OF_BODY: i32 = 0;

/// Where an Emote `cell` in size goes at `scale`, in physical pixels, when the Anchor is at `anchor_at` and the Pet's
/// Body is `body`. An odd width leaves the extra texel right of its centre, so the Emote stays on the Pet's texel
/// grid. The Body is as the Pack gives it, unmirrored, which keeps the Emote on the right when the Pet is mirrored.
pub fn place(anchor_at: Point, body: Body, cell: CellSize, scale: u32) -> Rect {
    let scale_i = scale as i32;
    let centre = anchor_at.x + (body.x + body.width as i32 + RIGHT_OF_BODY) * scale_i;
    let left = centre - (cell.width / 2 * scale) as i32;
    let body_top = anchor_at.y + body.y * scale_i;
    let bottom = body_top - (GAP * scale) as i32;
    Rect {
        left,
        top: bottom - (cell.height * scale) as i32,
        right: left + (cell.width * scale) as i32,
        bottom,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Body 15 Pack pixels tall standing on the Anchor, off-centre: 3 left of the Anchor and 7 right of it.
    const BODY: Body = Body {
        x: -3,
        y: -15,
        width: 10,
        height: 15,
    };

    const ANCHOR_AT: Point = Point { x: 100, y: 200 };

    #[test]
    fn given_an_even_width_emote_when_placing_it_then_it_is_centred_on_the_bodys_right_edge_a_gap_above_the_body()
     {
        // Given
        let cell = CellSize {
            width: 10,
            height: 8,
        };

        // When
        let rect = place(ANCHOR_AT, BODY, cell, 2);

        // Then: 20 x 16 on screen, 10 either side of the Body's right edge, which is 7 x 2 right of the Anchor, at
        // x = 114. The Body's top is 15 x 2 above the Anchor, at y = 170, so the Emote's bottom is 2 x 2 above that.
        assert_eq!(
            rect,
            Rect {
                left: 104,
                top: 150,
                right: 124,
                bottom: 166
            }
        );
    }

    #[test]
    fn given_an_odd_width_emote_when_placing_it_then_its_edges_stay_on_the_pets_texel_grid() {
        // Given: at 2x the Pet's texels start at even offsets from its Anchor
        let cell = CellSize {
            width: 5,
            height: 3,
        };

        // When
        let rect = place(ANCHOR_AT, BODY, cell, 2);

        // Then: 2 texels left of the Body's right edge, x = 114, and 3 right of it, not 5 screen pixels either side
        assert_eq!(
            rect,
            Rect {
                left: 110,
                top: 160,
                right: 120,
                bottom: 166
            }
        );
    }

    #[test]
    fn given_a_shorter_body_when_placing_an_emote_then_it_sits_the_same_gap_above_that_body() {
        // Given: curled up, the Body is 9 Pack pixels tall
        let curled = Body {
            y: -9,
            height: 9,
            ..BODY
        };
        let cell = CellSize {
            width: 10,
            height: 8,
        };

        // When
        let rect = place(ANCHOR_AT, curled, cell, 3);

        // Then: at 3x the Body's right edge is at 100 + 7 x 3 and its top at 200 - 27, and the Emote's bottom is 2 x 3
        // above that
        assert_eq!(
            rect,
            Rect {
                left: 106,
                top: 143,
                right: 136,
                bottom: 167
            }
        );
    }

    #[test]
    fn given_a_wider_body_when_placing_an_emote_then_it_moves_right_with_the_bodys_right_edge() {
        // Given: the same Body, 4 Pack pixels wider on the right
        let wider = Body { width: 14, ..BODY };
        let cell = CellSize {
            width: 10,
            height: 8,
        };

        // When
        let rect = place(ANCHOR_AT, wider, cell, 2);

        // Then: its right edge is 11 x 2 right of the Anchor, at x = 122, so the Emote is 4 x 2 further right
        assert_eq!(
            rect,
            Rect {
                left: 112,
                top: 150,
                right: 132,
                bottom: 166
            }
        );
    }
}
