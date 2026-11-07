//! Loading real config folders: the example in `tests/configs/`, and folders made for each test.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::TestFolder;
use pet_content::{
    BALANCE_FILE, Balance, ConfigError, DEFAULT_OBSTACLE_MATERIAL, OBSTACLES_FILE, load_balance,
    load_obstacles,
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("configs")
        .join(name)
}

fn where_(errors: &[ConfigError]) -> Vec<(&str, &str)> {
    errors
        .iter()
        .map(|error| (error.file.as_str(), error.field.as_str()))
        .collect()
}

#[test]
fn given_the_example_config_folder_when_loading_then_both_files_load() {
    // Given
    let folder = fixture("example");

    // When
    let (balance, obstacles) = (load_balance(&folder), load_obstacles(&folder));

    // Then
    assert!(balance.is_ok(), "{balance:?}");
    let obstacles = obstacles.expect("good Obstacles");
    assert_eq!(obstacles.len(), 2);
    assert_ne!(obstacles[0].material, DEFAULT_OBSTACLE_MATERIAL);
    assert_eq!(obstacles[1].material, DEFAULT_OBSTACLE_MATERIAL);
}

#[test]
fn given_a_config_folder_with_neither_file_when_loading_then_the_defaults_are_used_and_there_are_no_obstacles()
 {
    // Given: the owner hasn't made either file, or even the folder
    let empty = TestFolder::new("config-empty");
    let missing = empty.path().join("not-here");

    // When
    let loaded = [empty.path(), missing.as_path()]
        .map(|folder| (load_balance(folder), load_obstacles(folder)));

    // Then
    for (balance, obstacles) in loaded {
        assert_eq!(balance, Ok(Balance::default()));
        assert_eq!(obstacles, Ok(Vec::new()));
    }
}

#[test]
fn given_folders_where_the_files_should_be_when_loading_then_each_says_it_cant_be_read() {
    // Given: a folder by each file's name, which can't be read as one
    let config = TestFolder::new("config-unreadable");
    fs::create_dir(config.path().join(BALANCE_FILE)).unwrap();
    fs::create_dir(config.path().join(OBSTACLES_FILE)).unwrap();

    // When
    let balance = load_balance(config.path()).expect_err("unreadable");
    let obstacles = load_obstacles(config.path()).expect_err("unreadable");

    // Then
    assert_eq!(where_(&balance), [(BALANCE_FILE, "")]);
    assert_eq!(where_(&obstacles), [(OBSTACLES_FILE, "")]);
    assert!(balance[0].message.contains("can't be read"), "{balance:?}");
}

#[test]
fn given_a_balance_file_with_three_mistakes_when_loading_then_all_three_are_reported_with_the_file()
{
    // Given
    let config = TestFolder::new("config-three-mistakes");
    let balance = "[physics]\ngravity = -3000\nground_friction = 2\nbounce = 1\n";
    config.write(BALANCE_FILE, balance.as_bytes());

    // When
    let errors = load_balance(config.path()).expect_err("a broken file");

    // Then
    assert_eq!(
        where_(&errors),
        [
            (BALANCE_FILE, "physics.bounce"),
            (BALANCE_FILE, "physics.gravity"),
            (BALANCE_FILE, "physics.ground_friction"),
        ]
    );
}

#[test]
fn given_a_broken_obstacle_beside_a_good_one_when_loading_then_neither_loads() {
    // Given
    let config = TestFolder::new("config-broken-obstacle");
    let obstacles = "[[obstacle]]\nrect = { x = 0, y = 0, width = 10, height = 10 }\n\n\
                     [[obstacle]]\nrect = { x = 0, y = 0, width = -10, height = 10 }\n";
    config.write(OBSTACLES_FILE, obstacles.as_bytes());

    // When
    let errors = load_obstacles(config.path()).expect_err("a broken file");

    // Then
    assert_eq!(
        where_(&errors),
        [(OBSTACLES_FILE, "obstacle[1].rect.width")]
    );
}
