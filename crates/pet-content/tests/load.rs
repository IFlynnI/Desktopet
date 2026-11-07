//! Loading real Pack folders: the fixtures in `tests/packs/`, and the placeholder Pack the app ships with.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use common::TestFolder;
use pet_content::{PackError, discover, load};
use pet_core::pack::{AnimationId, EmoteId, EmoteMode, Situation};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("packs")
        .join(name)
}

fn placeholder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/placeholder")
}

fn where_(errors: &[PackError]) -> Vec<(&str, &str)> {
    errors
        .iter()
        .map(|error| (error.file.as_str(), error.field.as_str()))
        .collect()
}

/// A copy of the placeholder Pack in a folder of its own, with `edit` made to its manifest.
fn placeholder_with(test: &str, edit: impl Fn(String) -> String) -> TestFolder {
    let folder = TestFolder::new(test);
    let manifest = fs::read_to_string(placeholder().join("manifest.toml")).unwrap();
    let edited = edit(manifest.clone());
    assert_ne!(edited, manifest, "the edit should change the manifest");
    folder.write("manifest.toml", edited.as_bytes());
    folder.write(
        "hatchling/idle.png",
        &fs::read(placeholder().join("hatchling/idle.png")).unwrap(),
    );
    folder
}

#[test]
fn given_a_typo_in_an_action_a_wrong_starting_form_and_a_missing_sheet_when_loading_then_all_three_are_reported()
 {
    // Given
    let pack = placeholder_with("three-mistakes", |manifest| {
        manifest
            .replace("animation = \"idle\"", "animaton = \"idle\"")
            .replace("starting_form = \"hatchling\"", "starting_form = \"egg\"")
            .replace("\"hatchling/idle.png\"", "\"hatchling/gone.png\"")
    });

    // When
    let errors = load(pack.path()).err().expect("a broken Pack");

    // Then: the typo also leaves the Action with no Animation
    assert_eq!(
        where_(&errors),
        [
            ("manifest.toml", "forms.hatchling.actions.idle.animaton"),
            ("manifest.toml", "forms.hatchling.actions.idle.animation"),
            ("manifest.toml", "starting_form"),
            (
                "hatchling/gone.png",
                "forms.hatchling.animations.idle.sheet"
            ),
        ]
    );
}

#[test]
fn given_an_unknown_top_level_field_no_starting_form_and_a_0_ms_frame_when_loading_then_all_three_are_reported()
 {
    // Given
    let pack = placeholder_with("top-level-mistakes", |manifest| {
        manifest
            .replace("name = ", "author = \"me\"\nname = ")
            .replace("starting_form = \"hatchling\"\n", "")
            .replace("[250, 250]", "[250, 0]")
    });

    // When
    let errors = load(pack.path()).err().expect("a broken Pack");

    // Then
    assert_eq!(
        where_(&errors),
        [
            ("manifest.toml", "author"),
            ("manifest.toml", "starting_form"),
            (
                "manifest.toml",
                "forms.hatchling.animations.idle.durations_ms"
            ),
        ]
    );
}

#[test]
fn given_six_mistakes_in_sibling_fields_when_loading_then_all_six_are_reported() {
    // Given: a bad shadow beside a bad Anchor, Body and default Action, and a bad duration beside a missing sheet
    let pack = placeholder_with("six-mistakes", |manifest| {
        manifest
            .replace(
                "default_action = \"idle\"",
                "default_action = \"walk\"\nshadow = 7",
            )
            .replace("anchor = { x = 12, y = 18 }", "anchor = { x = 30, y = 18 }")
            .replace("width = 18, height = 15", "width = 0, height = 15")
            .replace("[250, 250]", "[250, -1]")
            .replace("\"hatchling/idle.png\"", "\"hatchling/gone.png\"")
    });

    // When
    let errors = load(pack.path()).err().expect("a broken Pack");

    // Then
    assert_eq!(
        where_(&errors),
        [
            ("manifest.toml", "forms.hatchling.shadow"),
            (
                "manifest.toml",
                "forms.hatchling.animations.idle.durations_ms"
            ),
            ("manifest.toml", "forms.hatchling.anchor"),
            ("manifest.toml", "forms.hatchling.body"),
            ("manifest.toml", "forms.hatchling.default_action"),
            (
                "hatchling/gone.png",
                "forms.hatchling.animations.idle.sheet"
            ),
        ]
    );
}

#[test]
fn given_forms_that_is_a_number_when_loading_then_only_that_is_reported_with_its_kind() {
    // Given
    let pack = TestFolder::new("forms-a-number");
    let manifest = "format_version = 1\nname = \"N\"\nstarting_form = \"egg\"\nforms = 3\n";
    pack.write("manifest.toml", manifest.as_bytes());

    // When
    let errors = load(pack.path()).err().expect("a broken Pack");

    // Then
    assert_eq!(where_(&errors), [("manifest.toml", "forms")]);
    assert_eq!(errors[0].message, "should be a table, but is an integer");
}

#[test]
fn given_two_unknown_fields_in_one_action_when_loading_then_both_are_reported() {
    // Given
    let pack = placeholder_with("two-unknown-fields", |manifest| {
        manifest.replace(
            "animation = \"idle\"",
            "animation = \"idle\"\nloop = true\nspeed = 2",
        )
    });

    // When
    let errors = load(pack.path()).err().expect("a broken Pack");

    // Then
    assert_eq!(
        where_(&errors),
        [
            ("manifest.toml", "forms.hatchling.actions.idle.loop"),
            ("manifest.toml", "forms.hatchling.actions.idle.speed"),
        ]
    );
}

#[test]
fn given_an_empty_sheet_name_when_loading_then_the_manifest_is_named_and_it_says_the_name_is_empty()
{
    // Given
    let pack = placeholder_with("empty-sheet", |manifest| {
        manifest.replace("\"hatchling/idle.png\"", "\"\"")
    });

    // When
    let errors = load(pack.path()).err().expect("a broken Pack");

    // Then
    assert_eq!(
        where_(&errors),
        [("manifest.toml", "forms.hatchling.animations.idle.sheet")]
    );
    assert!(errors[0].message.contains("empty"), "{errors:?}");
}

/// Makes `link` a junction to `target`. Unlike a symbolic link, a junction needs no special rights.
fn make_junction(link: &Path, target: &Path) {
    let output = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .expect("run mklink");
    assert!(output.status.success(), "{output:?}");
}

#[test]
fn given_a_junction_in_the_pack_to_a_folder_outside_it_when_loading_then_the_sheet_is_refused() {
    // Given: `link` in the Pack leads to a real PNG outside it
    let pack = placeholder_with("junction", |manifest| {
        manifest.replace("\"hatchling/idle.png\"", "\"link/idle.png\"")
    });
    let outside = TestFolder::new("junction-outside");
    outside.write(
        "idle.png",
        &fs::read(placeholder().join("hatchling/idle.png")).unwrap(),
    );
    make_junction(&pack.path().join("link"), outside.path());

    // When
    let errors = load(pack.path()).err().expect("the sheet is outside");

    // Then
    assert_eq!(
        where_(&errors),
        [("link/idle.png", "forms.hatchling.animations.idle.sheet")]
    );
    assert!(
        errors[0].message.contains("inside the Pack folder"),
        "{errors:?}"
    );
}

#[test]
fn given_the_example_pack_when_loading_then_its_definition_and_every_frame_are_there() {
    // Given
    let folder = fixture("example");

    // When
    let pack = load(&folder).expect("the example is a good Pack");

    // Then
    let definition = &pack.definition;
    assert_eq!(definition.name, "Placeholder dragon");
    let form = definition.form(definition.starting_form);
    assert_eq!(form.name, "hatchling");
    assert_eq!(
        form.situations,
        [(Situation::Held, form.default_animation())]
    );
    let sleep = form
        .animations
        .iter()
        .position(|animation| animation.name == "sleep")
        .map(AnimationId)
        .expect("a sleep Animation");
    let frames = pack.pixels.frames(definition.starting_form, sleep);
    assert_eq!(frames.len(), 3);
    assert!(frames.iter().all(|frame| frame.rgba.len() == 24 * 18 * 4));
    assert_eq!(
        form.animation(sleep).durations[1],
        Duration::from_millis(300)
    );
    assert_eq!(definition.emote(EmoteId(0)).mode, EmoteMode::Looping);
    assert_eq!(pack.pixels.emote_frames(EmoteId(0)).len(), 3);
}

#[test]
fn given_a_pack_with_four_mistakes_when_loading_then_all_four_are_reported_and_nothing_loads() {
    // Given
    let folder = fixture("broken");

    // When
    let errors = load(&folder).err().expect("a broken Pack loads nothing");

    // Then
    assert_eq!(
        where_(&errors),
        [
            ("manifest.toml", "starting_form"),
            (
                "hatchling/idle.png",
                "forms.hatchling.animations.idle.sheet"
            ),
            (
                "manifest.toml",
                "forms.hatchling.animations.sleep.durations_ms"
            ),
            (
                "hatchling/sleep.png",
                "forms.hatchling.animations.sleep.sheet"
            ),
        ]
    );
    assert!(errors[1].message.contains("no transparency"), "{errors:?}");
    assert_eq!(errors[3].message, "is missing");
}

#[test]
fn given_a_folder_with_no_manifest_when_loading_then_it_says_the_manifest_is_missing() {
    // Given
    let folder = TestFolder::new("no-manifest");

    // When
    let errors = load(folder.path()).err().expect("no Pack here");

    // Then
    assert_eq!(where_(&errors), [("manifest.toml", "")]);
    assert!(errors[0].message.contains("missing"), "{errors:?}");
}

#[test]
fn given_a_sheet_outside_the_pack_folder_when_loading_then_it_is_refused() {
    // Given: a real PNG one folder up, which a Pack mustn't reach
    let folder = TestFolder::new("outside");
    let manifest = fs::read_to_string(placeholder().join("manifest.toml")).unwrap();
    let pack = folder.path().join("pack");
    folder.write(
        "pack/manifest.toml",
        manifest
            .replace("\"hatchling/idle.png\"", "\"../idle.png\"")
            .as_bytes(),
    );
    folder.write(
        "idle.png",
        &fs::read(placeholder().join("hatchling/idle.png")).unwrap(),
    );

    // When
    let errors = load(&pack).err().expect("the sheet is outside");

    // Then
    assert_eq!(
        where_(&errors),
        [("../idle.png", "forms.hatchling.animations.idle.sheet")]
    );
}

/// The art the app drew in code before Packs existed, kept here to prove the placeholder Pack is the same Pet.
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

/// Rows at the bottom that stayed planted while the body bobbed.
const FEET_ROWS: usize = 3;

fn colour(pixel: char) -> [u8; 4] {
    match pixel {
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

/// The two Frames the code drew: standing, then the body one pixel lower with the feet in place.
fn frames_drawn_in_code() -> Vec<Vec<u8>> {
    let standing: Vec<&str> = ART.to_vec();
    let body_end = ART.len() - FEET_ROWS;
    let blank = " ".repeat(24);
    let mut bob = vec![blank.as_str()];
    bob.extend(&ART[..body_end - 1]);
    bob.extend(&ART[body_end..]);
    [standing, bob]
        .iter()
        .map(|rows| {
            rows.iter()
                .flat_map(|row| row.chars().flat_map(colour))
                .collect()
        })
        .collect()
}

#[test]
fn given_the_placeholder_pack_when_loading_then_its_frames_are_the_art_the_app_drew_in_code() {
    // Given
    let folder = placeholder();

    // When
    let pack = load(&folder).expect("the placeholder is a good Pack");

    // Then
    let definition = &pack.definition;
    let form = definition.form(definition.starting_form);
    let idle = form.default_animation();
    let frames = pack.pixels.frames(definition.starting_form, idle);
    let drawn: Vec<&[u8]> = frames.iter().map(|frame| frame.rgba.as_slice()).collect();
    assert_eq!(drawn, frames_drawn_in_code());
    assert_eq!(
        form.animation(idle).durations,
        [Duration::from_millis(250); 2]
    );
}

#[test]
fn given_the_placeholder_pack_when_loading_then_a_hole_between_the_legs_lets_clicks_through() {
    // Given: legs start on row 15, with four clear pixels between them from column 8
    let folder = placeholder();

    // When
    let pack = load(&folder).expect("the placeholder is a good Pack");

    // Then
    let definition = &pack.definition;
    let idle = definition
        .form(definition.starting_form)
        .default_animation();
    let frames = pack.pixels.frames(definition.starting_form, idle);
    assert!(frames.iter().all(|frame| !frame.hit_mask.is_opaque(9, 16)));
    assert!(frames.iter().all(|frame| frame.hit_mask.is_opaque(5, 16)));
}

#[test]
fn given_two_packs_and_a_file_when_discovering_then_the_first_folder_by_name_is_chosen() {
    // Given: the file sorts first, and folders are made out of order
    let packs = TestFolder::new("discover");
    packs.write("a-readme.txt", b"not a Pack");
    packs.write("zebra/manifest.toml", b"");
    packs.write("dragon/manifest.toml", b"");

    // When
    let chosen = discover(packs.path());

    // Then
    assert_eq!(chosen, Some(packs.path().join("dragon")));
}

#[test]
fn given_a_packs_folder_with_no_folders_or_none_at_all_when_discovering_then_there_is_no_pack() {
    // Given
    let empty = TestFolder::new("discover-empty");
    let missing = empty.path().join("not-here");

    // When
    let found = (discover(empty.path()), discover(&missing));

    // Then
    assert_eq!(found, (None, None));
}
