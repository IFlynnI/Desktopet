//! The validator's rules. Every rule runs, so the caller gets every error at once, each with its file and field.
//!
//! A field with a mistake in its shape is already reported and is `None` here, so only the rules that need it are
//! skipped. A rule never fires because of another field's mistake: an Action whose Animation couldn't be read might
//! play anything, so no Animation is called unused.
//!
//! A few rules need nothing here: a name used twice in one list is already a TOML error, and a per-Action Body sits
//! inside its Action, so it can't belong to one that doesn't exist.

use std::collections::BTreeMap;

use pet_core::pack::{EmoteMode, Situation};

use crate::PackError;
use crate::images::Sheets;
use crate::manifest::{BodyBox, Emote, Form, MANIFEST, Manifest, Size, parse_colour};

fn error(field: impl Into<String>, message: impl Into<String>) -> PackError {
    PackError::new(MANIFEST, field, message)
}

fn names<T>(entries: &BTreeMap<String, T>) -> String {
    let quoted: Vec<String> = entries.keys().map(|name| format!("\"{name}\"")).collect();
    if quoted.is_empty() {
        "there are none".to_owned()
    } else {
        format!("the choices are {}", quoted.join(", "))
    }
}

/// Checks a manifest and the sheets read for it.
pub fn check(manifest: &Manifest, sheets: &Sheets) -> Vec<PackError> {
    let mut errors = Vec::new();
    let threshold = match manifest.hit_mask_threshold {
        Some(threshold) => {
            let usable = u8::try_from(threshold)
                .ok()
                .filter(|&threshold| threshold > 0);
            if usable.is_none() {
                errors.push(error(
                    "hit_mask_threshold",
                    format!("is {threshold}, but must be between 1 and 255"),
                ));
            }
            usable
        }
        None => None,
    };
    if let Some(forms) = &manifest.forms {
        if forms.is_empty() {
            errors.push(error("forms", "a Pack needs at least one Form"));
        } else if let Some(starting_form) = &manifest.starting_form
            && !forms.contains_key(starting_form)
        {
            errors.push(error(
                "starting_form",
                format!(
                    "\"{starting_form}\" isn't one of the Forms: {}",
                    names(forms)
                ),
            ));
        }
        for (name, form) in forms {
            if let Some(form) = form {
                check_form(
                    &format!("forms.{name}"),
                    form,
                    sheets,
                    threshold,
                    &mut errors,
                );
            }
        }
    }
    for (name, emote) in manifest.emotes.iter().flatten() {
        if let Some(emote) = emote {
            check_emote(&format!("emotes.{name}"), emote, sheets, &mut errors);
        }
    }
    errors
}

fn check_form(
    path: &str,
    form: &Form,
    sheets: &Sheets,
    threshold: Option<u8>,
    errors: &mut Vec<PackError>,
) {
    let cell = form
        .cell
        .and_then(|cell| check_cell(&format!("{path}.cell"), cell, errors));
    if let (Some(cell), Some(anchor)) = (cell, form.anchor)
        && !(anchor.x <= cell.width && anchor.y <= cell.height)
    {
        errors.push(error(
            format!("{path}.anchor"),
            format!(
                "({}, {}) isn't inside the {} x {} cell. It's measured between pixels from the top-left, \
                 so x can go up to {} and y up to {}",
                anchor.x, anchor.y, cell.width, cell.height, cell.width, cell.height
            ),
        ));
    }
    if let Some(body) = form.body {
        check_body(&format!("{path}.body"), body, errors);
    }
    if let (Some(default_action), Some(actions)) = (&form.default_action, &form.actions)
        && !actions.contains_key(default_action)
    {
        errors.push(error(
            format!("{path}.default_action"),
            format!(
                "\"{default_action}\" isn't one of this Form's Actions: {}",
                names(actions)
            ),
        ));
    }
    if let Some(shadow) = &form.shadow {
        if shadow.width == 0 {
            errors.push(error(format!("{path}.shadow.width"), "must be above 0"));
        }
        if parse_colour(&shadow.colour).is_none() {
            errors.push(error(
                format!("{path}.shadow.colour"),
                format!(
                    "\"{}\" isn't a colour: write it as \"#rrggbb\" or \"#rrggbbaa\"",
                    shadow.colour
                ),
            ));
        }
    }

    let animations = form.animations.as_ref();
    let is_an_animation = |name: &String| animations.is_none_or(|all| all.contains_key(name));
    let all_names = || animations.map(names).unwrap_or_default();
    for (name, action) in form.actions.iter().flatten() {
        let Some(action) = action else { continue };
        let action_path = format!("{path}.actions.{name}");
        if let Some(animation) = &action.animation
            && !is_an_animation(animation)
        {
            errors.push(error(
                format!("{action_path}.animation"),
                format!(
                    "\"{animation}\" isn't one of this Form's Animations: {}",
                    all_names()
                ),
            ));
        }
        if let Some(body) = action.body {
            check_body(&format!("{action_path}.body"), body, errors);
        }
    }
    for (name, animation) in form.situations.iter().flatten() {
        let situation_path = format!("{path}.situations.{name}");
        if Situation::from_name(name).is_none() {
            let allowed: Vec<&str> = Situation::ALL.iter().map(|s| s.name()).collect();
            errors.push(error(
                &situation_path,
                format!(
                    "isn't a Situation: the Situations are {}",
                    allowed.join(", ")
                ),
            ));
        }
        if let Some(animation) = animation
            && !is_an_animation(animation)
        {
            errors.push(error(
                &situation_path,
                format!(
                    "\"{animation}\" isn't one of this Form's Animations: {}",
                    all_names()
                ),
            ));
        }
    }

    // Each Animation an Action or Situation plays, or `None` when one of those couldn't be read and might play any.
    let played: Option<Vec<&String>> = match (&form.actions, &form.situations) {
        (Some(actions), Some(situations)) => {
            let by_actions = actions
                .values()
                .map(|action| action.as_ref().and_then(|action| action.animation.as_ref()));
            by_actions
                .chain(situations.values().map(Option::as_ref))
                .collect()
        }
        _ => None,
    };
    for (name, animation) in form.animations.iter().flatten() {
        let animation_path = format!("{path}.animations.{name}");
        if played
            .as_ref()
            .is_some_and(|played| !played.contains(&name))
        {
            errors.push(error(
                &animation_path,
                "isn't played by any Action or Situation of this Form. Is a name misspelt?",
            ));
        }
        let Some(animation) = animation else { continue };
        let frames = animation.durations_ms.as_ref().map(|durations_ms| {
            check_durations(
                &format!("{animation_path}.durations_ms"),
                durations_ms,
                errors,
            )
        });
        if let Some(frames) = frames {
            for marker in &animation.markers {
                if marker.frame as usize >= frames {
                    errors.push(error(
                        format!("{animation_path}.markers"),
                        format!(
                            "\"{}\" is on Frame {}, but this Animation has {frames} Frames, counting from 0",
                            marker.name, marker.frame
                        ),
                    ));
                }
            }
        }
        if let Some(sheet) = &animation.sheet {
            let strip = Strip {
                cell,
                frames,
                threshold,
            };
            check_sheet(
                &format!("{animation_path}.sheet"),
                sheet,
                sheets,
                strip,
                errors,
            );
        }
    }
}

fn check_emote(path: &str, emote: &Emote, sheets: &Sheets, errors: &mut Vec<PackError>) {
    let cell = emote
        .cell
        .and_then(|cell| check_cell(&format!("{path}.cell"), cell, errors));
    let frames = emote
        .durations_ms
        .as_ref()
        .map(|durations_ms| check_durations(&format!("{path}.durations_ms"), durations_ms, errors));
    if let Some(mode) = &emote.mode
        && EmoteMode::from_name(mode).is_none()
    {
        errors.push(error(
            format!("{path}.mode"),
            format!("is \"{mode}\", but must be \"one-shot\" or \"looping\""),
        ));
    }
    if let Some(sheet) = &emote.sheet {
        // Emotes never catch clicks, so their Frames may be fully clear.
        let strip = Strip {
            cell,
            frames,
            threshold: None,
        };
        check_sheet(&format!("{path}.sheet"), sheet, sheets, strip, errors);
    }
}

/// The cell, if it's usable: one with no width or height can't hold a Frame.
fn check_cell(path: &str, cell: Size, errors: &mut Vec<PackError>) -> Option<Size> {
    if cell.width == 0 || cell.height == 0 {
        errors.push(error(path, "width and height must both be above 0"));
        return None;
    }
    Some(cell)
}

fn check_body(path: &str, body: BodyBox, errors: &mut Vec<PackError>) {
    if body.width == 0 || body.height == 0 {
        errors.push(error(path, "width and height must both be above 0"));
    }
}

/// Checks there's at least one Frame and none of them lasts 0 ms. Returns how many Frames there are.
fn check_durations(path: &str, durations_ms: &[u32], errors: &mut Vec<PackError>) -> usize {
    if durations_ms.is_empty() {
        errors.push(error(path, "needs at least one duration, one per Frame"));
    }
    for (frame, &ms) in durations_ms.iter().enumerate() {
        if ms == 0 {
            errors.push(error(
                path,
                format!(
                    "Frame {frame}, counting from 0, lasts 0 ms, but every Frame must last longer"
                ),
            ));
        }
    }
    durations_ms.len()
}

/// What a sheet must hold: `frames` Frames of `cell` size in one row. With a `threshold`, each needs a pixel at or above
/// it, so the Pet can be clicked. The size can only be judged when the cell and the number of Frames are known.
struct Strip {
    cell: Option<Size>,
    frames: Option<usize>,
    threshold: Option<u8>,
}

fn check_sheet(
    path: &str,
    sheet: &str,
    sheets: &Sheets,
    strip: Strip,
    errors: &mut Vec<PackError>,
) {
    // With no name there's no file to blame, so the error is about the manifest.
    let file = if sheet.is_empty() { MANIFEST } else { sheet };
    let sheet_error = |message: String| PackError::new(file, path, message);
    let read = match sheets.get(sheet) {
        Some(Ok(read)) => read,
        Some(Err(message)) => return errors.push(sheet_error(message.clone())),
        None => return errors.push(sheet_error("is missing".to_owned())),
    };
    let (Some(cell), Some(frames)) = (strip.cell, strip.frames) else {
        return;
    };
    if frames == 0 {
        return;
    }
    let width = u64::from(cell.width) * frames as u64;
    if (u64::from(read.width), read.height) != (width, cell.height) {
        return errors.push(sheet_error(format!(
            "is {} x {} pixels, but {frames} Frames of the {} x {} cell in one row need {width} x {}",
            read.width, read.height, cell.width, cell.height, cell.height
        )));
    }
    let Some(threshold) = strip.threshold else {
        return;
    };
    for frame in 0..frames as u32 {
        let clickable = read
            .frame(cell, frame)
            .chunks_exact(4)
            .any(|pixel| pixel[3] >= threshold);
        if !clickable {
            errors.push(sheet_error(format!(
                "Frame {frame}, counting from 0, has no pixel with an alpha of {threshold} or more, so the Pet couldn't be clicked"
            )));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::images::Sheet;
    use crate::manifest::parse;

    /// The example from the item 3 proposal, which is the format's contract.
    const EXAMPLE: &str = r##"
        format_version = 1
        name = "Placeholder dragon"
        starting_form = "hatchling"

        [forms.hatchling]
        cell = { width = 24, height = 18 }
        anchor = { x = 12, y = 18 }
        body = { x = -9, y = -15, width = 18, height = 15 }
        default_action = "idle"
        shadow = { width = 16, colour = "#22203480" }

        [forms.hatchling.actions.idle]
        animation = "idle"

        [forms.hatchling.actions.sleep]
        animation = "sleep"
        body = { x = -10, y = -9, width = 20, height = 9 }

        [forms.hatchling.situations]
        held = "idle"

        [forms.hatchling.animations.idle]
        sheet = "hatchling/idle.png"
        durations_ms = [250, 250]
        markers = [{ frame = 1, name = "footstep" }]

        [forms.hatchling.animations.sleep]
        sheet = "hatchling/sleep.png"
        durations_ms = [600, 300, 600]

        [emotes.tired]
        sheet = "emotes/tired.png"
        cell = { width = 10, height = 10 }
        durations_ms = [200, 200, 400]
        mode = "looping"
    "##;

    /// An Animation that nothing plays, to go just before the sleep Animation.
    const WAVE: &str = r#"[forms.hatchling.animations.wave]
        sheet = "hatchling/idle.png"
        durations_ms = [1, 1]

        [forms.hatchling.animations.sleep]"#;

    fn opaque(width: u32, height: u32) -> Sheet {
        Sheet {
            width,
            height,
            rgba: [255].repeat((width * height * 4) as usize),
        }
    }

    /// Sheets of the right size for `EXAMPLE`, every pixel opaque.
    fn example_sheets() -> Sheets {
        [
            ("hatchling/idle.png", opaque(48, 18)),
            ("hatchling/sleep.png", opaque(72, 18)),
            ("emotes/tired.png", opaque(30, 10)),
        ]
        .into_iter()
        .map(|(path, sheet)| (path.to_owned(), Ok(sheet)))
        .collect()
    }

    /// Reads a manifest whose shape is right.
    fn manifest(text: &str) -> Manifest {
        let (manifest, errors) = parse(text).expect("TOML in a known format");
        assert_eq!(errors, [], "the shape should be right");
        manifest
    }

    /// `EXAMPLE` with `from` replaced by `to`, which must change something.
    fn example_with(from: &str, to: &str) -> Manifest {
        assert!(EXAMPLE.contains(from), "the example has no {from}");
        manifest(&EXAMPLE.replacen(from, to, 1))
    }

    fn fields(errors: &[PackError]) -> Vec<&str> {
        errors.iter().map(|error| error.field.as_str()).collect()
    }

    #[test]
    fn given_the_example_pack_when_checking_then_there_are_no_errors() {
        // Given: it has an Anchor on the bottom edge, and no falling or landing Situation
        let manifest = manifest(EXAMPLE);
        let sheets = example_sheets();

        // When
        let errors = check(&manifest, &sheets);

        // Then
        assert_eq!(errors, []);
    }

    #[test]
    fn given_no_forms_when_checking_then_it_says_a_pack_needs_one() {
        // Given
        let manifest = manifest("format_version = 1\nname = \"Empty\"\nstarting_form = \"egg\"\n");

        // When
        let errors = check(&manifest, &Sheets::new());

        // Then
        assert_eq!(fields(&errors), ["forms"]);
    }

    #[test]
    fn given_a_starting_form_that_is_not_a_form_when_checking_then_it_is_reported() {
        // Given
        let manifest = example_with("starting_form = \"hatchling\"", "starting_form = \"egg\"");

        // When
        let errors = check(&manifest, &example_sheets());

        // Then
        assert_eq!(fields(&errors), ["starting_form"]);
        assert!(errors[0].message.contains("\"hatchling\""), "{errors:?}");
    }

    #[test]
    fn given_a_default_action_that_is_not_one_of_the_forms_actions_when_checking_then_it_is_reported()
     {
        // Given
        let manifest = example_with("default_action = \"idle\"", "default_action = \"walk\"");

        // When
        let errors = check(&manifest, &example_sheets());

        // Then
        assert_eq!(fields(&errors), ["forms.hatchling.default_action"]);
    }

    #[test]
    fn given_an_action_naming_an_animation_the_form_lacks_when_checking_then_it_is_reported() {
        // Given: "sleep" now plays nothing that exists, so its Animation is also unused
        let manifest = example_with("animation = \"sleep\"", "animation = \"slep\"");

        // When
        let errors = check(&manifest, &example_sheets());

        // Then
        assert_eq!(
            fields(&errors),
            [
                "forms.hatchling.actions.sleep.animation",
                "forms.hatchling.animations.sleep"
            ]
        );
    }

    #[test]
    fn given_an_animation_no_action_or_situation_plays_when_checking_then_it_is_reported() {
        // Given: a "wave" Animation that nothing plays, which is usually a misspelt name
        let manifest = example_with("[forms.hatchling.animations.sleep]", WAVE);

        // When
        let errors = check(&manifest, &example_sheets());

        // Then
        assert_eq!(fields(&errors), ["forms.hatchling.animations.wave"]);
    }

    #[test]
    fn given_an_action_with_a_mistake_in_its_shape_when_checking_then_no_animation_is_called_unused()
     {
        // Given: the sleep Action's Animation isn't text, which `parse` reports, so it might have played anything
        let text = EXAMPLE.replacen("animation = \"sleep\"", "animation = 7", 1);
        let (manifest, shape_errors) = parse(&text).expect("TOML in a known format");
        assert_eq!(
            fields(&shape_errors),
            ["forms.hatchling.actions.sleep.animation"]
        );

        // When
        let errors = check(&manifest, &example_sheets());

        // Then
        assert_eq!(errors, []);
    }

    #[test]
    fn given_a_situation_that_is_not_one_when_checking_then_it_is_reported_with_the_allowed_ones() {
        // Given
        let manifest = example_with("held = \"idle\"", "sitting = \"idle\"");

        // When
        let errors = check(&manifest, &example_sheets());

        // Then
        assert_eq!(fields(&errors), ["forms.hatchling.situations.sitting"]);
        assert!(
            errors[0].message.contains("held, falling, landing"),
            "{errors:?}"
        );
    }

    #[test]
    fn given_a_situation_naming_an_animation_the_form_lacks_when_checking_then_it_is_reported() {
        // Given
        let manifest = example_with("held = \"idle\"", "held = \"wriggle\"");

        // When
        let errors = check(&manifest, &example_sheets());

        // Then
        assert_eq!(fields(&errors), ["forms.hatchling.situations.held"]);
    }

    #[test]
    fn given_a_sheet_that_could_not_be_read_when_checking_then_the_error_names_that_file() {
        // Given
        let manifest = manifest(EXAMPLE);
        let mut sheets = example_sheets();
        sheets.insert(
            "hatchling/sleep.png".to_owned(),
            Err("is missing".to_owned()),
        );

        // When
        let errors = check(&manifest, &sheets);

        // Then
        assert_eq!(fields(&errors), ["forms.hatchling.animations.sleep.sheet"]);
        assert_eq!(errors[0].file, "hatchling/sleep.png");
        assert_eq!(errors[0].message, "is missing");
    }

    #[test]
    fn given_a_sheet_one_frame_short_when_checking_then_it_says_what_size_it_needs() {
        // Given: sleep has three durations, but the strip holds two Frames
        let manifest = manifest(EXAMPLE);
        let mut sheets = example_sheets();
        sheets.insert("hatchling/sleep.png".to_owned(), Ok(opaque(48, 18)));

        // When
        let errors = check(&manifest, &sheets);

        // Then
        assert_eq!(fields(&errors), ["forms.hatchling.animations.sleep.sheet"]);
        assert!(errors[0].message.contains("need 72 x 18"), "{errors:?}");
    }

    #[test]
    fn given_a_sheet_taller_than_the_cell_when_checking_then_it_is_reported() {
        // Given
        let manifest = manifest(EXAMPLE);
        let mut sheets = example_sheets();
        sheets.insert("hatchling/idle.png".to_owned(), Ok(opaque(48, 36)));

        // When
        let errors = check(&manifest, &sheets);

        // Then
        assert_eq!(fields(&errors), ["forms.hatchling.animations.idle.sheet"]);
    }

    #[test]
    fn given_a_frame_lasting_0_ms_when_checking_then_it_is_reported() {
        // Given
        let manifest = example_with(
            "durations_ms = [600, 300, 600]",
            "durations_ms = [600, 0, 600]",
        );

        // When
        let errors = check(&manifest, &example_sheets());

        // Then
        assert_eq!(
            fields(&errors),
            ["forms.hatchling.animations.sleep.durations_ms"]
        );
        assert!(errors[0].message.contains("Frame 1"), "{errors:?}");
    }

    #[test]
    fn given_an_animation_with_no_durations_when_checking_then_it_needs_at_least_one() {
        // Given
        let manifest = example_with("durations_ms = [600, 300, 600]", "durations_ms = []");

        // When
        let errors = check(&manifest, &example_sheets());

        // Then
        assert_eq!(
            fields(&errors),
            ["forms.hatchling.animations.sleep.durations_ms"]
        );
    }

    #[test]
    fn given_an_anchor_on_the_right_edge_when_checking_then_there_are_no_errors() {
        // Given: the Anchor is measured between pixels, so x = 24 is the right edge of a 24-wide cell
        let manifest = example_with("anchor = { x = 12, y = 18 }", "anchor = { x = 24, y = 18 }");

        // When
        let errors = check(&manifest, &example_sheets());

        // Then
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn given_an_anchor_past_the_right_edge_when_checking_then_it_is_reported() {
        // Given: x = 24 is the right edge and allowed, so x = 25 is the first that isn't
        let manifest = example_with("anchor = { x = 12, y = 18 }", "anchor = { x = 25, y = 18 }");

        // When
        let errors = check(&manifest, &example_sheets());

        // Then
        assert_eq!(fields(&errors), ["forms.hatchling.anchor"]);
    }

    #[test]
    fn given_an_anchor_below_the_bottom_edge_when_checking_then_it_is_reported() {
        // Given: y = 18 is the bottom edge and allowed, so y = 19 is the first that isn't
        let manifest = example_with("anchor = { x = 12, y = 18 }", "anchor = { x = 12, y = 19 }");

        // When
        let errors = check(&manifest, &example_sheets());

        // Then
        assert_eq!(fields(&errors), ["forms.hatchling.anchor"]);
    }

    #[test]
    fn given_a_form_body_with_no_width_when_checking_then_it_is_reported() {
        // Given
        let manifest = example_with(
            "body = { x = -9, y = -15, width = 18, height = 15 }",
            "body = { x = -9, y = -15, width = 0, height = 15 }",
        );

        // When
        let errors = check(&manifest, &example_sheets());

        // Then
        assert_eq!(fields(&errors), ["forms.hatchling.body"]);
    }

    #[test]
    fn given_an_action_body_with_no_height_when_checking_then_it_is_reported() {
        // Given
        let manifest = example_with(
            "body = { x = -10, y = -9, width = 20, height = 9 }",
            "body = { x = -10, y = -9, width = 20, height = 0 }",
        );

        // When
        let errors = check(&manifest, &example_sheets());

        // Then
        assert_eq!(fields(&errors), ["forms.hatchling.actions.sleep.body"]);
    }

    #[test]
    fn given_a_cell_with_no_height_when_checking_then_only_the_cell_is_reported() {
        // Given: the Anchor and sheets can't be judged against it, so they say nothing
        let manifest = example_with(
            "cell = { width = 24, height = 18 }",
            "cell = { width = 24, height = 0 }",
        );

        // When
        let errors = check(&manifest, &example_sheets());

        // Then
        assert_eq!(fields(&errors), ["forms.hatchling.cell"]);
    }

    #[test]
    fn given_a_frame_with_no_pixel_at_the_threshold_when_checking_then_that_frame_is_reported() {
        // Given: the second idle Frame is fully clear
        let manifest = manifest(EXAMPLE);
        let mut idle = opaque(48, 18);
        for row in idle.rgba.chunks_exact_mut(48 * 4) {
            row[24 * 4..].fill(0);
        }
        let mut sheets = example_sheets();
        sheets.insert("hatchling/idle.png".to_owned(), Ok(idle));

        // When
        let errors = check(&manifest, &sheets);

        // Then
        assert_eq!(fields(&errors), ["forms.hatchling.animations.idle.sheet"]);
        assert!(errors[0].message.contains("Frame 1"), "{errors:?}");
    }

    #[test]
    fn given_a_threshold_of_0_when_checking_then_it_is_reported() {
        // Given
        let manifest = example_with("name = ", "hit_mask_threshold = 0\nname = ");

        // When
        let errors = check(&manifest, &example_sheets());

        // Then
        assert_eq!(fields(&errors), ["hit_mask_threshold"]);
    }

    #[test]
    fn given_a_threshold_above_255_when_checking_then_it_is_reported() {
        // Given
        let manifest = example_with("name = ", "hit_mask_threshold = 256\nname = ");

        // When
        let errors = check(&manifest, &example_sheets());

        // Then
        assert_eq!(fields(&errors), ["hit_mask_threshold"]);
    }

    #[test]
    fn given_a_marker_past_the_last_frame_when_checking_then_it_is_reported() {
        // Given: idle has Frames 0 and 1
        let manifest = example_with("frame = 1", "frame = 2");

        // When
        let errors = check(&manifest, &example_sheets());

        // Then
        assert_eq!(fields(&errors), ["forms.hatchling.animations.idle.markers"]);
    }

    #[test]
    fn given_an_emote_with_an_unknown_mode_when_checking_then_it_is_reported() {
        // Given
        let manifest = example_with("mode = \"looping\"", "mode = \"once\"");

        // When
        let errors = check(&manifest, &example_sheets());

        // Then
        assert_eq!(fields(&errors), ["emotes.tired.mode"]);
    }

    #[test]
    fn given_an_emote_sheet_of_the_wrong_size_when_checking_then_it_is_reported_against_its_own_cell()
     {
        // Given: right for a 10 x 10 cell with two Frames, but the Emote has three
        let manifest = manifest(EXAMPLE);
        let mut sheets = example_sheets();
        sheets.insert("emotes/tired.png".to_owned(), Ok(opaque(20, 10)));

        // When
        let errors = check(&manifest, &sheets);

        // Then
        assert_eq!(fields(&errors), ["emotes.tired.sheet"]);
        assert!(errors[0].message.contains("need 30 x 10"), "{errors:?}");
    }

    #[test]
    fn given_a_shadow_colour_that_is_not_one_when_checking_then_it_is_reported() {
        // Given
        let manifest = example_with("#22203480", "dark grey");

        // When
        let errors = check(&manifest, &example_sheets());

        // Then
        assert_eq!(fields(&errors), ["forms.hatchling.shadow.colour"]);
    }

    #[test]
    fn given_mistakes_under_several_rules_when_checking_then_every_one_is_reported() {
        // Given: a wrong starting Form, a 0 ms Frame, a bad Emote mode and a missing sheet
        let text = EXAMPLE
            .replacen(
                "starting_form = \"hatchling\"",
                "starting_form = \"egg\"",
                1,
            )
            .replacen("[250, 250]", "[250, 0]", 1)
            .replacen("mode = \"looping\"", "mode = \"sometimes\"", 1);
        let manifest = manifest(&text);
        let mut sheets = example_sheets();
        sheets.remove("hatchling/sleep.png");

        // When
        let errors = check(&manifest, &sheets);

        // Then
        assert_eq!(
            fields(&errors),
            [
                "starting_form",
                "forms.hatchling.animations.idle.durations_ms",
                "forms.hatchling.animations.sleep.sheet",
                "emotes.tired.mode"
            ]
        );
    }
}
