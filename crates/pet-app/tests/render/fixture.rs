//! A small Pack for the render tests, written from ASCII art into a folder of its own and loaded the way the app loads
//! one. Its Anchor, (1, 4) in a 6 x 4 cell, is off-centre, so lining the Anchor up shows in the pixels. Its art isn't
//! symmetric, so mirroring shows too. It may have a shadow, 11 Pack pixels wide, in a half-transparent colour. It has
//! two Emotes 5 x 3, an odd width: "note" loops and isn't symmetric, and "pop" plays once, 50 ms then 50 ms.

use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::{env, process};

use pet_content::LoadedPack;
use pet_core::pack::EmoteId;

use crate::harness::Picture;

/// Fully opaque or fully clear, so the drawn pixels come out exactly as they are here, except `h`.
fn colour(pixel: char) -> [u8; 4] {
    match pixel {
        '.' => [0, 0, 0, 0],
        'a' => [240, 140, 40, 255],
        'b' => [40, 30, 60, 255],
        'c' => [60, 200, 120, 255],
        'h' => EMOTE_HALF,
        other => panic!("no colour for {other:?}"),
    }
}

/// A pixel as the renderer leaves it in the image: `h` premultiplied, as `EMOTE_HALF_DRAWN` says.
fn drawn_colour(pixel: char) -> [u8; 4] {
    match pixel {
        'h' => EMOTE_HALF_DRAWN,
        other => colour(other),
    }
}

/// A half-transparent Emote pixel, `#aa5a2860`: alpha 96, as the shadow's.
const EMOTE_HALF: [u8; 4] = [170, 90, 40, 96];

/// `EMOTE_HALF` as drawn over the clear background, premultiplied in sRGB as `SHADOW_DRAWN` explains: 170 -> 64.00,
/// 90 -> 33.88 and 40 -> 15.06, each at least 0.3 from a rounding boundary.
pub const EMOTE_HALF_DRAWN: [u8; 4] = [64, 34, 15, 96];

/// "note", one Frame that loops. Mirrored, its half-transparent corner would be on the left.
const NOTE: [[&str; 3]; 1] = [["cc..h", ".c.h.", "cc..."]];

/// "pop", two Frames that play once.
const POP: [[&str; 3]; 2] = [["..c..", ".ccc.", "..c.."], ["c...c", "..c..", "c...c"]];

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

/// The Pack's manifest. Its `# shadow` line makes way for the shadow, if it has one, and its `# idle body` line for
/// a Body of the idle Action's own. The Form's Body's top is the top of the cell, 4 Pack pixels above the Anchor.
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
# idle body

[forms.hatchling.animations.idle]
sheet = "hatchling/idle.png"
durations_ms = [100, 200]

[emotes.note]
sheet = "emotes/note.png"
cell = { width = 5, height = 3 }
durations_ms = [300]
mode = "looping"

[emotes.pop]
sheet = "emotes/pop.png"
cell = { width = 5, height = 3 }
durations_ms = [50, 50]
mode = "one-shot"
"#;

/// ASCII art as a picture, each character a pixel in the colour `colour` gives it.
fn picture(rows: &[&str], colour: fn(char) -> [u8; 4]) -> Picture {
    let mut picture = Picture::clear(rows[0].len() as u32, rows.len() as u32);
    for (y, row) in rows.iter().enumerate() {
        for (x, pixel) in row.chars().enumerate() {
            picture.set(x as u32, y as u32, colour(pixel));
        }
    }
    picture
}

/// One Frame of the idle Animation, as art pixels.
pub fn frame(index: usize) -> Picture {
    picture(&IDLE[index], colour)
}

/// Frame `index` of the Emote `name` as the renderer draws it, in Pack pixels.
pub fn emote(name: &str, index: usize) -> Picture {
    picture(&emote_art(name)[index], drawn_colour)
}

fn emote_art(name: &str) -> &'static [[&'static str; 3]] {
    match name {
        "note" => &NOTE,
        "pop" => &POP,
        other => panic!("no Emote {other:?}"),
    }
}

/// The id the Pack's Definition gives the Emote `name`.
pub fn emote_id(pack: &LoadedPack, name: &str) -> EmoteId {
    let emotes = &pack.definition.emotes;
    EmoteId(
        emotes
            .iter()
            .position(|emote| emote.name == name)
            .expect("the fixture has the Emote"),
    )
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
    write_and_load(test, "", "")
}

/// The same Pack, its idle Action crouched into a Body of its own: 1 Pack pixel tall, from 1 left of the Anchor to 2
/// right of it. The Pet's art reaches 3 Pack pixels above it.
pub fn load_crouched(test: &str) -> (PathBuf, LoadedPack) {
    write_and_load(test, "", "body = { x = -1, y = -1, width = 3, height = 1 }")
}

/// The same Pack with its shadow.
pub fn load_with_shadow(test: &str) -> (PathBuf, LoadedPack) {
    write_and_load(test, SHADOW, "")
}

fn write_and_load(test: &str, shadow: &str, idle_body: &str) -> (PathBuf, LoadedPack) {
    let folder = env::temp_dir().join(format!("desktop-pet-render-{}-{test}", process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(folder.join("hatchling")).expect("make the Pack folder");
    fs::create_dir_all(folder.join("emotes")).expect("make the Emotes folder");
    fs::write(
        folder.join("manifest.toml"),
        MANIFEST
            .replace("# shadow", shadow)
            .replace("# idle body", idle_body),
    )
    .expect("write the manifest");
    write_strip(&folder.join("hatchling/idle.png"), &[frame(0), frame(1)]);
    for name in ["note", "pop"] {
        let frames: Vec<Picture> = emote_art(name)
            .iter()
            .map(|rows| picture(rows, colour))
            .collect();
        write_strip(&folder.join(format!("emotes/{name}.png")), &frames);
    }
    let pack = pet_content::load(&folder).unwrap_or_else(|errors| panic!("{errors:?}"));
    let _ = fs::remove_dir_all(&folder);
    (folder, pack)
}

/// One row of Frames, left to right, as a Pack's sheet.
fn write_strip(path: &Path, frames: &[Picture]) {
    let (width, height) = (frames[0].width, frames[0].height);
    let mut strip = Picture::clear(width * frames.len() as u32, height);
    for (index, frame) in frames.iter().enumerate() {
        strip.draw(frame, width * index as u32, 0, 1, false);
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
