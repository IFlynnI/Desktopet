//! A small Pack for the render tests, written from ASCII art into a folder of its own and loaded the way the app loads
//! one. Its Anchor, (1, 4) in a 6 x 4 cell, is off-centre, so lining the Anchor up shows in the pixels. Its art isn't
//! symmetric, so mirroring shows too. It may have a shadow, 11 Pack pixels wide, in a half-transparent colour. It has
//! two Emotes 5 x 3, an odd width: "note" loops and isn't symmetric, and "pop" plays once, 50 ms then 50 ms. It may
//! have a second Action, "nap", with art and timing of its own, and a second Form, "egg", smaller and one colour.

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

/// The nap Animation: 50 ms, then 150 ms. At every tick from the second to the sixth, it's on another Frame than idle
/// would be.
const NAP: [[&str; 4]; 2] = [
    ["......", "......", ".cccc.", "cc..cc"],
    ["......", "..cc..", "cbbbbc", "c....c"],
];

/// The "nap" Action and its Animation, for the `# nap` line.
const NAP_SECTION: &str = r#"
[forms.hatchling.actions.nap]
animation = "nap"

[forms.hatchling.animations.nap]
sheet = "hatchling/nap.png"
durations_ms = [50, 150]
"#;

/// The egg's one Frame, 4 x 3.
const EGG: [&str; 3] = [".cc.", "cbbc", ".cc."];

/// The "egg" Form, for the `# egg` line. Its Anchor is the middle of its bottom edge, and its Body is its cell.
const EGG_SECTION: &str = r#"
[forms.egg]
cell = { width = 4, height = 3 }
anchor = { x = 2, y = 3 }
body = { x = -2, y = -3, width = 4, height = 3 }
default_action = "idle"

[forms.egg.actions.idle]
animation = "idle"

[forms.egg.animations.idle]
sheet = "egg/idle.png"
durations_ms = [500]
"#;

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

/// The Pack's manifest. Its `# shadow` line makes way for the shadow, if it has one, its `# idle body` line for a
/// Body of the idle Action's own, its `# nap` line for the "nap" Action and its `# egg` line for the "egg" Form. The
/// Form's Body's top is the top of the cell, 4 Pack pixels above the Anchor.
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
# nap
# egg

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

/// One Frame of the nap Animation, as art pixels.
pub fn nap_frame(index: usize) -> Picture {
    picture(&NAP[index], colour)
}

/// The egg's one Frame, as art pixels.
pub fn egg() -> Picture {
    picture(&EGG, colour)
}

/// One Frame of the idle Animation after an edit that swaps its two colours, `a` and `b`. Nothing else changes.
pub fn recoloured_frame(index: usize) -> Picture {
    picture(&IDLE[index], |pixel| match pixel {
        'a' => colour('b'),
        'b' => colour('a'),
        other => colour(other),
    })
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
    write_and_load(test, &[], frame)
}

/// The same Pack with its idle art in other colours, as `recoloured_frame` draws it.
pub fn load_recoloured(test: &str) -> (PathBuf, LoadedPack) {
    write_and_load(test, &[], recoloured_frame)
}

/// The same Pack, its idle Action crouched into a Body of its own: 1 Pack pixel tall, from 1 left of the Anchor to 2
/// right of it. The Pet's art reaches 3 Pack pixels above it.
pub fn load_crouched(test: &str) -> (PathBuf, LoadedPack) {
    write_and_load(
        test,
        &[(
            "# idle body",
            "body = { x = -1, y = -1, width = 3, height = 1 }",
        )],
        frame,
    )
}

/// The same Pack with its shadow.
pub fn load_with_shadow(test: &str) -> (PathBuf, LoadedPack) {
    write_and_load(test, &[("# shadow", SHADOW)], frame)
}

/// The same Pack with the "nap" Action too, and "idle" still its default.
pub fn load_with_nap(test: &str) -> (PathBuf, LoadedPack) {
    write_and_load(test, &[("# nap", NAP_SECTION)], frame)
}

/// The same Pack with the "nap" Action as its default: an edit of `load_with_nap`'s that changes only that.
pub fn load_napping(test: &str) -> (PathBuf, LoadedPack) {
    write_and_load(
        test,
        &[
            ("# nap", NAP_SECTION),
            (r#"default_action = "idle""#, r#"default_action = "nap""#),
        ],
        frame,
    )
}

/// The same Pack with the "egg" Form too, which it starts as.
pub fn load_as_egg(test: &str) -> (PathBuf, LoadedPack) {
    write_and_load(
        test,
        &[
            ("# egg", EGG_SECTION),
            (r#"starting_form = "hatchling""#, r#"starting_form = "egg""#),
        ],
        frame,
    )
}

/// The same Pack with the "egg" Form too, still starting as "hatchling": an edit of `load_as_egg`'s that changes only
/// its `starting_form`.
pub fn load_with_egg(test: &str) -> (PathBuf, LoadedPack) {
    write_and_load(test, &[("# egg", EGG_SECTION)], frame)
}

/// Writes the Pack into a folder for `test`, each `edits` line in `MANIFEST` replaced, with `idle` as its idle art, and
/// loads it. A `#` line left alone is a comment.
fn write_and_load(
    test: &str,
    edits: &[(&str, &str)],
    idle: fn(usize) -> Picture,
) -> (PathBuf, LoadedPack) {
    let folder = env::temp_dir().join(format!("desktop-pet-render-{}-{test}", process::id()));
    let _ = fs::remove_dir_all(&folder);
    for subfolder in ["hatchling", "egg", "emotes"] {
        fs::create_dir_all(folder.join(subfolder)).expect("make the Pack's folders");
    }
    let manifest = edits
        .iter()
        .fold(MANIFEST.to_owned(), |manifest, (line, with)| {
            manifest.replace(line, with)
        });
    fs::write(folder.join("manifest.toml"), manifest).expect("write the manifest");
    write_strip(&folder.join("hatchling/idle.png"), &[idle(0), idle(1)]);
    write_strip(
        &folder.join("hatchling/nap.png"),
        &[nap_frame(0), nap_frame(1)],
    );
    write_strip(&folder.join("egg/idle.png"), &[egg()]);
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
