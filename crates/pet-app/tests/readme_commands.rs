//! Keeps the README's commands working. The real-exe tests launch the exe directly, so they can't notice these break.

use std::process::Command;

#[test]
fn given_the_package_has_two_programs_when_asking_cargo_which_one_runs_then_it_is_the_pet() {
    // Given: `failing-app` is the second program, so `cargo run -p pet-app` needs telling which to start
    let manifest = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");

    // When
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .args(["--manifest-path", manifest])
        .output()
        .expect("run cargo");

    // Then
    let metadata = String::from_utf8_lossy(&output.stdout);
    assert!(metadata.contains(r#""default_run":"desktop-pet""#));
}
