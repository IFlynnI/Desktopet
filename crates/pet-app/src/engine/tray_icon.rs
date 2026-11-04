//! The app's own tray icon, a paw print. It's built in rather than taken from a Pack, so the tray works with no Pack
//! and you can always quit.

use pet_platform::TrayIconImage;

const ART: &[&str] = &[
    "                ",
    "    oo    oo    ",
    "   oppo  oppo   ",
    "   oppo  oppo   ",
    "    oo    oo    ",
    " oo          oo ",
    "oppo        oppo",
    "oppo  oooo  oppo",
    " oo  oppppo  oo ",
    "    oppppppo    ",
    "   oppppppppo   ",
    "   oppppppppo   ",
    "   oppppppppo   ",
    "    oppppppo    ",
    "     oooooo     ",
    "                ",
];

/// Screen pixels per art pixel. Windows shrinks the icon to 16 pixels at 100% display scale, which this divides evenly.
const SCALE: usize = 2;

fn colour(pixel: char) -> [u8; 4] {
    match pixel {
        'o' => [34, 32, 52, 255],
        'p' => [240, 150, 60, 255],
        _ => [0, 0, 0, 0],
    }
}

pub fn tray_icon_image() -> TrayIconImage {
    let side = ART.len() * SCALE;
    let rgba = ART
        .iter()
        .flat_map(|row| {
            let scaled_row: Vec<u8> = row
                .chars()
                .flat_map(|pixel| colour(pixel).repeat(SCALE))
                .collect();
            scaled_row.repeat(SCALE)
        })
        .collect();
    TrayIconImage {
        rgba,
        width: side as u32,
        height: side as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_the_built_in_art_when_making_the_tray_icon_then_it_is_square_with_every_pixel_there() {
        // Given: Windows stretches an icon that isn't square
        let side = ART.len() * SCALE;

        // When
        let image = tray_icon_image();

        // Then
        assert_eq!((image.width, image.height), (side as u32, side as u32));
        assert_eq!(image.rgba.len(), side * side * 4);
    }

    #[test]
    fn given_the_built_in_art_when_making_the_tray_icon_then_each_art_pixel_is_a_block_of_the_same_colour()
     {
        // Given: the top-left of the paw's first toe is outline, at art pixel (4, 1)
        let side = ART.len() * SCALE;
        let pixel_at = |image: &TrayIconImage, x: usize, y: usize| {
            let start = (y * side + x) * 4;
            image.rgba[start..start + 4].to_vec()
        };

        // When
        let image = tray_icon_image();

        // Then
        for (x, y) in [(8, 2), (9, 2), (8, 3), (9, 3)] {
            assert_eq!(pixel_at(&image, x, y), colour('o'), "({x}, {y})");
        }
        assert_eq!(pixel_at(&image, 0, 0), colour(' '));
    }
}
