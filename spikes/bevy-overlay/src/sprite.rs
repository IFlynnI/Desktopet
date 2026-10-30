//! Procedural placeholder pixel-art creature (two frames) so the spike needs no asset files.
//! Deliberately has holes (between the legs, between tail and body) to exercise per-pixel hit testing.

const ART: &[&str] = &[
    "      oo        oo      ",
    "     oyyo      oyyo     ",
    "     oyyoooooooyyo      ",
    "    oggggggggggggo      ",
    "   oggggggggggggggo     ",
    "   oggwwggggggwwggo     ",
    "   oggwkggggggwkggo  oo ",
    "   oggggggggggggggo ogo ",
    "   ogggggorrogggggo ogo ",
    "    oggggoooogggggoogo  ",
    "   oolllllllllllgggggo  ",
    "  ogolllllllllllgooo    ",
    "  ogolllllllllllgo      ",
    "   oolllllllllllgo      ",
    "    ogggggggggggo       ",
    "    oggo    oggo        ",
    "    oggo    oggo        ",
    "    oooo    oooo        ",
];

/// Rows at the bottom that stay planted while the body bobs.
const FEET_ROWS: usize = 3;

pub struct Frames {
    pub width: u32,
    pub height: u32,
    /// RGBA8, one Vec per frame.
    pub rgba: Vec<Vec<u8>>,
}

fn color(c: char) -> [u8; 4] {
    match c {
        'o' => [34, 32, 52, 255],
        'g' => [106, 190, 48, 255],
        'l' => [238, 195, 154, 255],
        'w' => [255, 255, 255, 255],
        'k' => [20, 20, 28, 255],
        'y' => [251, 242, 54, 255],
        'r' => [217, 87, 99, 255],
        _ => [0, 0, 0, 0],
    }
}

pub fn creature() -> Frames {
    let width = ART.iter().map(|r| r.chars().count()).max().unwrap_or(0);
    let height = ART.len();
    let rows: Vec<Vec<char>> = ART
        .iter()
        .map(|r| {
            let mut v: Vec<char> = r.chars().collect();
            v.resize(width, ' ');
            v
        })
        .collect();

    // Frame 1: body squashes down one pixel, feet stay put.
    let body_end = height - FEET_ROWS;
    let mut bob = vec![vec![' '; width]];
    bob.extend(rows[..body_end - 1].iter().cloned());
    bob.extend(rows[body_end..].iter().cloned());

    let to_rgba = |grid: &[Vec<char>]| -> Vec<u8> {
        grid.iter().flat_map(|row| row.iter().flat_map(|&c| color(c))).collect()
    };

    Frames {
        width: width as u32,
        height: height as u32,
        rgba: vec![to_rgba(&rows), to_rgba(&bob)],
    }
}
