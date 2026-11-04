//! The `pet-pack` command-line tool, run as a real program.

use std::path::Path;
use std::process::{Command, Output};

fn pet_pack(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pet-pack"))
        .args(args)
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .expect("run pet-pack")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn given_the_placeholder_pack_when_validating_it_then_it_passes() {
    // Given
    let folder = "content/placeholder";

    // When
    let output = pet_pack(&["validate", folder]);

    // Then
    assert!(output.status.success(), "{output:?}");
    assert!(stdout(&output).contains("Placeholder dragon"), "{output:?}");
}

#[test]
fn given_a_broken_pack_when_validating_it_then_every_error_is_printed_and_it_fails() {
    // Given
    let folder = "crates/pet-content/tests/packs/broken";

    // When
    let output = pet_pack(&["validate", folder]);

    // Then
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let printed = stdout(&output);
    let lines: Vec<&str> = printed.lines().collect();
    assert_eq!(lines.len(), 4, "{printed}");
    assert!(
        lines[0].starts_with("manifest.toml: starting_form: "),
        "{printed}"
    );
    assert!(lines[3].starts_with("hatchling/sleep.png: "), "{printed}");
}

#[test]
fn given_no_folder_when_running_then_it_says_how_to_use_it() {
    // Given
    let args = ["validate"];

    // When
    let output = pet_pack(&args);

    // Then
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(said.contains("pet-pack validate <pack folder>"), "{said}");
}
