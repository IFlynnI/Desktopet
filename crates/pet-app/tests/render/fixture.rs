//! A small Pack for the render tests, written from ASCII art into a folder of its own and loaded the way the app loads
//! one. Its Anchor, (1, 4) in a 6 x 4 cell, is off-centre, so lining the Anchor up shows in the pixels. Its art isn't
//! symmetric, so mirroring shows too. It may have a shadow, 11 Pack pixels wide, in a half-transparent colour.

use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::{env, process};

use pet_content::LoadedPack;

use crate::harness::Picture;

pub const CELL_WIDTH: u32 = 6;
pub const CELL_HEIGHT: u32 = 4;

/// Fully opaque or fully clear, so the drawn pixels come out exactly as they are here.
fn colour(pixel: char) -> [u8; 4] {
    match pixel {
        '.' => [0, 0, 0, 0],
        'a' => [240, 140, 40, 255],
        'b' => [40, 30, 60, 255],
        other => panic!("no colour for {other:?}"),
    }
}

/// The idle Animation: 100 ms, then 200 ms.
const IDLE: [[&str; 4]; 2] = [
    [".aa...", ".abba.", "aaaaab", "a....a"],
    ["......", "..aa..", ".abbaa", "aa..aa"],
];

/// The shadow's width and colour, `#1b224560`: alpha 96, a little over a third.
const SHADOW: &str = r##"shadow = { width = 11, colour = "#1b224560" }"##;

/// A shadow pixel as the renderer leaves it in the image: premultiplied by its alpha, in sRGB, the way an image editor
/// blends. Over the clear background each colour channel becomes `c x 96 / 255`: 27 -> 10.16, 34 -> 12.80 and
/// 69 -> 25.98. Each is at least 0.3 from a rounding boundary, so any GPU rounds them the same. The window's compositor
/// then adds `behind x (1 - 96 / 255)`, which gives the Pack's colour over whatever is behind. Blending in linear light
/// instead would give `srgb(linear(c) x 96 / 255)`, which is about (13, 18, 41).
pub const SHADOW_DRAWN: [u8; 4] = [10, 13, 26, 96];

/// The shadow's art: an ellipse 11 x 3, a quarter of its width rounded half up.
const SHADOW_ART: [&str; 3] = [".#########.", "###########", ".#########."];

/// The Pack's manifest. Its `# shadow` line makes way for the shadow, if it has one.
const MANIFEST: &str = r#"
format_version = 1
name = "Render test"
starting_form = "hatchling"

[forms.hatchling]
cell = { width = 6, height = 4 }
anchor = { x = 1, y = 4 }
body = { x = -1, y = -4, width = 6, height = 4 }
default_action = "idle"
# shadow

[forms.hatchling.actions.idle]
animation = "idle"

[forms.hatchling.animations.idle]
sheet = "hatchling/idle.png"
durations_ms = [100, 200]
"#;

/// One Frame of the idle Animation, as art pixels.
pub fn frame(index: usize) -> Picture {
    let mut picture = Picture::clear(CELL_WIDTH, CELL_HEIGHT);
    for (y, row) in IDLE[index].iter().enumerate() {
        for (x, pixel) in row.chars().enumerate() {
            picture.set(x as u32, y as u32, colour(pixel));
        }
    }
    picture
}

/// The shadow as the renderer draws it, in Pack pixels.
pub fn shadow() -> Picture {
    let mut picture = Picture::clear(11, 3);
    for (y, row) in SHADOW_ART.iter().enumerate() {
        for (x, pixel) in row.chars().enumerate() {
            if pixel == '#' {
                picture.set(x as u32, y as u32, SHADOW_DRAWN);
            }
        }
    }
    picture
}

/// Writes the Pack, with no shadow, into a folder for `test`, loads it, and removes the folder again.
pub fn load(test: &str) -> (PathBuf, LoadedPack) {
    write_and_load(test, "")
}

/// The same Pack with its shadow.
pub fn load_with_shadow(test: &str) -> (PathBuf, LoadedPack) {
    write_and_load(test, SHADOW)
}

fn write_and_load(test: &str, shadow: &str) -> (PathBuf, LoadedPack) {
    let folder = env::temp_dir().join(format!("desktop-pet-render-{}-{test}", process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(folder.join("hatchling")).expect("make the Pack folder");
    fs::write(
        folder.join("manifest.toml"),
        MANIFEST.replace("# shadow", shadow),
    )
    .expect("write the manifest");
    write_strip(&folder.join("hatchling/idle.png"), &[frame(0), frame(1)]);
    let pack = pet_content::load(&folder).unwrap_or_else(|errors| panic!("{errors:?}"));
    let _ = fs::remove_dir_all(&folder);
    (folder, pack)
}

/// One row of Frames, left to right, as a Pack's sheet.
fn write_strip(path: &Path, frames: &[Picture]) {
    let mut strip = Picture::clear(CELL_WIDTH * frames.len() as u32, CELL_HEIGHT);
    for (index, frame) in frames.iter().enumerate() {
        strip.draw(frame, CELL_WIDTH * index as u32, 0, 1, false);
    }
    let file = BufWriter::new(File::create(path).expect("create the sheet"));
    let mut encoder = png::Encoder::new(file, strip.width, strip.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("write the PNG header");
    writer
        .write_image_data(&strip.rgba)
        .expect("write the PNG pixels");
}
