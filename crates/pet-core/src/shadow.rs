//! The drop shadow on the Ground plane under the Pet (REN-03). It's pixel art like the Pet: an ellipse with one texel
//! per Pack pixel, drawn at the Pet's integer scale. It's centred on the Anchor's x but sits on the Ground plane, not
//! at the Anchor's y, so it stays on the ground while the Pet is in the air. It never catches clicks.

use crate::geometry::Rect;
use crate::pack::Shadow;

/// The shadow's art, row by row from the top, one texel per Pack pixel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShadowArt {
    pub width: u32,
    pub height: u32,
    /// RGBA8: the Pack's colour inside the ellipse, clear outside it.
    pub rgba: Vec<u8>,
}

/// How tall a shadow `width` Pack pixels wide is: a quarter of its width, rounded half up, and never under 2 so the
/// ellipse has a top and a bottom row.
pub fn height(width: u32) -> u32 {
    ((width + 2) / 4).max(2)
}

/// The shadow as a filled ellipse `shadow.width` x `height(shadow.width)` in the Pack's colour, alpha included.
pub fn art(shadow: Shadow) -> ShadowArt {
    let (width, height) = (shadow.width, height(shadow.width));
    let rgba = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .flat_map(|(x, y)| {
            if inside_ellipse(width, height, x, y) {
                shadow.colour
            } else {
                [0; 4]
            }
        })
        .collect();
    ShadowArt {
        width,
        height,
        rgba,
    }
}

/// Whether texel (`x`, `y`)'s centre is inside the ellipse that fills a `width` x `height` box. In whole numbers,
/// doubled so the centres fall on integers: `(dx / width)^2 + (dy / height)^2 <= 1`, which is symmetric both ways.
fn inside_ellipse(width: u32, height: u32, x: u32, y: u32) -> bool {
    let (width, height) = (u64::from(width), u64::from(height));
    let dx = (2 * u64::from(x) + 1).abs_diff(width);
    let dy = (2 * u64::from(y) + 1).abs_diff(height);
    dx * dx * height * height + dy * dy * width * width <= width * width * height * height
}

/// Where the shadow goes on screen at `scale`, in physical pixels, when the Anchor is at `anchor_x` and the Ground
/// plane is at `ground`. Its bottom row ends on the Ground plane, because nothing is drawn below it. Its middle is on
/// the Anchor. An odd width leaves the extra texel right of the Anchor, so the shadow stays on the Pet's texel grid.
pub fn place(anchor_x: i32, ground: i32, shadow: Shadow, scale: u32) -> Rect {
    let left = anchor_x - (shadow.width / 2 * scale) as i32;
    Rect {
        left,
        top: ground - (height(shadow.width) * scale) as i32,
        right: left + (shadow.width * scale) as i32,
        bottom: ground,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLOUR: [u8; 4] = [26, 19, 38, 96];

    fn shadow(width: u32) -> Shadow {
        Shadow {
            width,
            colour: COLOUR,
        }
    }

    /// The art as text, `#` for a texel in the Pack's colour and `.` for a clear one. Any other texel panics.
    fn ascii(art: &ShadowArt) -> Vec<String> {
        art.rgba
            .chunks(4 * art.width as usize)
            .map(|row| {
                row.chunks(4)
                    .map(|texel| match texel {
                        [0, 0, 0, 0] => '.',
                        texel if texel == COLOUR => '#',
                        other => panic!("a texel neither clear nor the Pack's colour: {other:?}"),
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn given_shadow_widths_when_finding_their_heights_then_each_is_a_quarter_rounded_half_up_and_never_under_2()
     {
        // Given
        let widths = [1, 2, 8, 9, 10, 11, 13, 14, 30];

        // When
        let heights = widths.map(height);

        // Then: 30 / 4 = 7.5 rounds up to 8, and 10 / 4 = 2.5 up to 3
        assert_eq!(heights, [2, 2, 2, 2, 3, 3, 3, 4, 8]);
    }

    #[test]
    fn given_a_shadow_11_wide_when_drawing_its_art_then_it_is_an_ellipse_3_high_in_the_packs_colour()
     {
        // Given
        let shadow = shadow(11);

        // When
        let art = art(shadow);

        // Then
        assert_eq!((art.width, art.height), (11, 3));
        assert_eq!(ascii(&art), [".#########.", "###########", ".#########."]);
    }

    #[test]
    fn given_firstborns_shadow_30_wide_when_drawing_its_art_then_each_row_is_centred_and_it_is_symmetric_top_to_bottom()
     {
        // Given: content/firstborn's shadow width
        let shadow = shadow(30);

        // When
        let art = art(shadow);

        // Then: each row is one run of colour in the middle
        let rows = ascii(&art);
        let runs: Vec<usize> = rows.iter().map(|row| row.matches('#').count()).collect();
        assert_eq!(runs, [14, 24, 28, 30, 30, 28, 24, 14]);
        for (row, run) in rows.iter().zip(&runs) {
            let margin = ".".repeat((30 - run) / 2);
            assert_eq!(*row, format!("{margin}{}{margin}", "#".repeat(*run)));
        }
    }

    #[test]
    fn given_the_narrowest_shadows_when_drawing_their_art_then_every_texel_is_coloured() {
        // Given
        let narrowest = [shadow(1), shadow(2)];

        // When
        let arts = narrowest.map(art);

        // Then
        assert_eq!(ascii(&arts[0]), ["#", "#"]);
        assert_eq!(ascii(&arts[1]), ["##", "##"]);
    }

    #[test]
    fn given_an_anchor_in_the_air_when_placing_an_even_width_shadow_then_it_is_centred_on_the_anchor_and_ends_on_the_ground()
     {
        // Given: the Ground plane is at y = 800, whatever the Anchor's height
        let (anchor_x, ground) = (100, 800);

        // When
        let rect = place(anchor_x, ground, shadow(30), 2);

        // Then: 60 x 16 on screen, 30 either side of the Anchor, the bottom row at y = 799
        assert_eq!(
            rect,
            Rect {
                left: 70,
                top: 784,
                right: 130,
                bottom: 800
            }
        );
    }

    #[test]
    fn given_an_odd_width_when_placing_the_shadow_then_its_edges_stay_on_the_pets_texel_grid() {
        // Given: at 2x the Pet's texels start at even offsets from its Anchor
        let (anchor_x, ground) = (100, 800);

        // When
        let rect = place(anchor_x, ground, shadow(11), 2);

        // Then: 5 texels left of the Anchor and 6 right of it, not 11 screen pixels either side
        assert_eq!(
            rect,
            Rect {
                left: 90,
                top: 794,
                right: 112,
                bottom: 800
            }
        );
    }
}
