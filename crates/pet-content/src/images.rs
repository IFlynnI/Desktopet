//! The PNG strips: reading them, cutting each into Frames left to right, and making each Frame's Hit mask.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Cursor, ErrorKind};
use std::path::{Component, Path};

use pet_core::hit_mask::HitMask;
use pet_core::pack::{AnimationId, EmoteId, FormId};
use png::{ColorType, Decoder, Transformations};

use crate::manifest::{Manifest, Size, complete};

/// A whole PNG strip, as RGBA8, top row first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sheet {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Sheet {
    /// The pixels of Frame `index`, for a strip of `cell`-sized Frames.
    pub fn frame(&self, cell: Size, index: u32) -> Vec<u8> {
        let row_bytes = (self.width * 4) as usize;
        let frame_bytes = (cell.width * 4) as usize;
        let left = index as usize * frame_bytes;
        self.rgba
            .chunks_exact(row_bytes)
            .take(cell.height as usize)
            .flat_map(|row| &row[left..left + frame_bytes])
            .copied()
            .collect()
    }
}

/// Every sheet the manifest names, by its path in the manifest. A sheet two Animations share is read once.
pub type Sheets = BTreeMap<String, Result<Sheet, String>>;

/// One Frame's pixels, and the Hit mask made from them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FramePixels {
    pub width: u32,
    pub height: u32,
    /// RGBA8, top row first.
    pub rgba: Vec<u8>,
    pub hit_mask: HitMask,
}

/// The Frames of every Animation and Emote in a Pack, found by the same numbers as the Definition.
#[derive(Clone, Debug)]
pub struct PackPixels {
    /// By Form, then Animation, then Frame.
    forms: Vec<Vec<Vec<FramePixels>>>,
    /// By Emote, then Frame.
    emotes: Vec<Vec<FramePixels>>,
}

impl PackPixels {
    pub fn frames(&self, form: FormId, animation: AnimationId) -> &[FramePixels] {
        &self.forms[form.0][animation.0]
    }

    pub fn emote_frames(&self, emote: EmoteId) -> &[FramePixels] {
        &self.emotes[emote.0]
    }
}

/// Reads a PNG. It must have transparency, because the Pet is drawn over other apps.
pub fn decode(bytes: &[u8]) -> Result<Sheet, String> {
    let mut decoder = Decoder::new(Cursor::new(bytes));
    // Palettes, low bit depths and 16-bit channels all become 8 bits per channel, and a transparent colour becomes alpha.
    decoder.set_transformations(Transformations::normalize_to_color8());
    let unreadable = |error: png::DecodingError| format!("isn't a PNG that can be read: {error}");
    let mut reader = decoder.read_info().map_err(unreadable)?;
    let size = reader.output_buffer_size().ok_or("is too big to read")?;
    let mut pixels = vec![0; size];
    let info = reader.next_frame(&mut pixels).map_err(unreadable)?;
    pixels.truncate(info.buffer_size());
    let rgba = match info.color_type {
        ColorType::Rgba => pixels,
        ColorType::GrayscaleAlpha => pixels
            .chunks_exact(2)
            .flat_map(|pixel| [pixel[0], pixel[0], pixel[0], pixel[1]])
            .collect(),
        _ => return Err("has no transparency: save it with an alpha channel".to_owned()),
    };
    Ok(Sheet {
        width: info.width,
        height: info.height,
        rgba,
    })
}

const INSIDE_THE_PACK: &str = "must be a path inside the Pack folder, such as `hatchling/idle.png`";

/// Reads one sheet. It must be inside the Pack folder, so a Pack is always the whole of its own folder. The path is
/// checked as written, and again once links and junctions are followed, because either could lead out of it.
fn read_sheet(pack_folder: &Path, sheet: &str) -> Result<Sheet, String> {
    if sheet.is_empty() {
        return Err("is empty: name a PNG in the Pack, such as `hatchling/idle.png`".to_owned());
    }
    let path = Path::new(sheet);
    let written_inside = path
        .components()
        .all(|part| matches!(part, Component::Normal(_)));
    if !written_inside {
        return Err(INSIDE_THE_PACK.to_owned());
    }
    let unreadable = |error: io::Error| match error.kind() {
        ErrorKind::NotFound => "is missing".to_owned(),
        _ => format!("can't be read: {error}"),
    };
    let real_path = pack_folder.join(path).canonicalize().map_err(unreadable)?;
    let real_pack_folder = pack_folder.canonicalize().map_err(unreadable)?;
    if !real_path.starts_with(&real_pack_folder) {
        return Err(INSIDE_THE_PACK.to_owned());
    }
    decode(&fs::read(real_path).map_err(unreadable)?)
}

/// Every sheet named by an entry whose shape is right.
pub fn read_sheets(pack_folder: &Path, manifest: &Manifest) -> Sheets {
    let forms = manifest
        .forms
        .iter()
        .flat_map(|forms| forms.values().flatten());
    let animations = forms
        .flat_map(|form| {
            form.animations
                .iter()
                .flat_map(|all| all.values().flatten())
        })
        .filter_map(|animation| animation.sheet.as_ref());
    let emotes = manifest
        .emotes
        .iter()
        .flat_map(|emotes| emotes.values().flatten());
    let emotes = emotes.filter_map(|emote| emote.sheet.as_ref());
    animations
        .chain(emotes)
        .map(|sheet| (sheet.clone(), read_sheet(pack_folder, sheet)))
        .collect()
}

/// Cuts a strip into `count` Frames of `cell` size, each with its Hit mask.
pub fn cut(sheet: &Sheet, cell: Size, count: usize, threshold: u8) -> Vec<FramePixels> {
    (0..count as u32)
        .map(|index| {
            let rgba = sheet.frame(cell, index);
            FramePixels {
                width: cell.width,
                height: cell.height,
                hit_mask: HitMask::from_rgba(cell.width, cell.height, &rgba, threshold),
                rgba,
            }
        })
        .collect()
}

/// Cuts every sheet of a manifest with no errors, in the order the Definition numbers things.
pub fn pack_pixels(manifest: &Manifest, sheets: &Sheets, threshold: u8) -> PackPixels {
    let sheet = |path: &Option<String>| {
        sheets[complete(path)]
            .as_ref()
            .expect("every sheet was read")
    };
    let forms = complete(&manifest.forms)
        .values()
        .map(|form| {
            let form = complete(form);
            let cell = *complete(&form.cell);
            complete(&form.animations)
                .values()
                .map(|animation| {
                    let animation = complete(animation);
                    let count = complete(&animation.durations_ms).len();
                    cut(sheet(&animation.sheet), cell, count, threshold)
                })
                .collect()
        })
        .collect();
    let emotes = complete(&manifest.emotes)
        .values()
        .map(|emote| {
            let emote = complete(emote);
            let count = complete(&emote.durations_ms).len();
            cut(
                sheet(&emote.sheet),
                *complete(&emote.cell),
                count,
                threshold,
            )
        })
        .collect();
    PackPixels { forms, emotes }
}

#[cfg(test)]
mod tests {
    use png::{BitDepth, Encoder};

    use super::*;

    const RED: [u8; 4] = [255, 0, 0, 255];
    const CLEAR: [u8; 4] = [0, 0, 0, 0];

    fn encode(width: u32, height: u32, colour: ColorType, pixels: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = Encoder::new(&mut bytes, width, height);
        encoder.set_color(colour);
        encoder.set_depth(BitDepth::Eight);
        let mut writer = encoder.write_header().expect("write the header");
        writer.write_image_data(pixels).expect("write the pixels");
        writer.finish().expect("finish the PNG");
        bytes
    }

    #[test]
    fn given_an_rgba_png_when_decoding_then_the_pixels_are_the_ones_written() {
        // Given
        let pixels = [RED, CLEAR].concat();
        let bytes = encode(2, 1, ColorType::Rgba, &pixels);

        // When
        let sheet = decode(&bytes).expect("a good PNG");

        // Then
        assert_eq!((sheet.width, sheet.height), (2, 1));
        assert_eq!(sheet.rgba, pixels);
    }

    #[test]
    fn given_a_grey_png_with_alpha_when_decoding_then_it_becomes_rgba() {
        // Given
        let bytes = encode(1, 1, ColorType::GrayscaleAlpha, &[200, 128]);

        // When
        let sheet = decode(&bytes).expect("a good PNG");

        // Then
        assert_eq!(sheet.rgba, [200, 200, 200, 128]);
    }

    #[test]
    fn given_a_png_with_no_transparency_when_decoding_then_it_is_refused_and_says_why() {
        // Given
        let bytes = encode(1, 1, ColorType::Rgb, &[255, 0, 0]);

        // When
        let error = decode(&bytes).expect_err("no alpha");

        // Then
        assert!(error.contains("no transparency"), "{error}");
    }

    #[test]
    fn given_bytes_that_are_not_a_png_when_decoding_then_it_is_refused() {
        // Given
        let bytes = b"GIF89a, not a PNG";

        // When
        let error = decode(bytes).expect_err("not a PNG");

        // Then
        assert!(error.contains("isn't a PNG"), "{error}");
    }

    #[test]
    fn given_a_strip_of_two_frames_when_cutting_it_then_each_frame_has_its_own_pixels_and_hit_mask()
    {
        // Given: a 2 x 2 cell, twice. The first Frame is red on the left, the second on the right.
        let rows = [[RED, CLEAR, CLEAR, RED], [RED, CLEAR, CLEAR, RED]];
        let sheet = Sheet {
            width: 4,
            height: 2,
            rgba: rows.concat().concat(),
        };
        let cell = Size {
            width: 2,
            height: 2,
        };

        // When
        let frames = cut(&sheet, cell, 2, 128);

        // Then
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].rgba, [RED, CLEAR, RED, CLEAR].concat());
        assert_eq!(frames[1].rgba, [CLEAR, RED, CLEAR, RED].concat());
        assert!(frames[0].hit_mask.is_opaque(0, 1) && !frames[0].hit_mask.is_opaque(1, 1));
        assert!(frames[1].hit_mask.is_opaque(1, 1) && !frames[1].hit_mask.is_opaque(0, 1));
    }
}
