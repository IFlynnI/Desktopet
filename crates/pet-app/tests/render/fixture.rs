//! A small Pack for the render tests, written from ASCII art into a folder of its own and loaded the way the app loads
//! one. Its Anchor, (1, 4) in a 6 x 4 cell, is off-centre, so lining the Anchor up shows in the pixels. Its art isn't
//! symmetric, so mirroring shows too.

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

const MANIFEST: &str = r#"
format_version = 1
name = "Render test"
starting_form = "hatchling"

[forms.hatchling]
cell = { width = 6, height = 4 }
anchor = { x = 1, y = 4 }
body = { x = -1, y = -4, width = 6, height = 4 }
default_action = "idle"

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

/// Writes the Pack into a folder for `test`, loads it, and removes the folder again.
pub fn load(test: &str) -> (PathBuf, LoadedPack) {
    let folder = env::temp_dir().join(format!("desktop-pet-render-{}-{test}", process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(folder.join("hatchling")).expect("make the Pack folder");
    fs::write(folder.join("manifest.toml"), MANIFEST).expect("write the manifest");
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
