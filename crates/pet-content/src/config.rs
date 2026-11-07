//! The files the owner may edit in their config folder, `%LOCALAPPDATA%\DesktopPet\config\`: `balance.toml`, the feel
//! numbers, and `obstacles.toml`, the Obstacles they've drawn. The caller finds the folder.
//!
//! Neither file has to exist: one that's missing means the owner hasn't made it, so its defaults are used. A file that
//! is there loads completely or not at all, and the caller gets every error at once, each with its file and field, as
//! a Pack's are reported. Each is read once at startup for now.

use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use pet_core::animation_clock::TICKS_PER_SECOND;
use pet_core::geometry::Rect;
use pet_core::physics::{GrabString, Material, Tuning};

use toml::Value;

use crate::fields::{Fields, Mistake};
use crate::material;

/// The feel numbers, in the config folder.
pub const BALANCE_FILE: &str = "balance.toml";

/// The owner's Obstacles, in the config folder.
pub const OBSTACLES_FILE: &str = "obstacles.toml";

/// One thing wrong with a config file, and where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigError {
    /// The file it's about, such as `balance.toml`.
    pub file: String,
    /// Where in the file, such as `physics.gravity`. Empty when it's about the whole file.
    pub field: String,
    pub message: String,
}

impl ConfigError {
    pub fn new(
        file: impl Into<String>,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            file: file.into(),
            field: field.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crate::write_error(f, &self.file, &self.field, &self.message)
    }
}

/// Everything in `balance.toml`. Slice 1 has only the physics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Balance {
    /// The physics' feel numbers, shared by every Pet. Any the file leaves out keep their defaults.
    pub physics: Tuning,
}

/// The physics' starting values, tuned by hand: what a `balance.toml` with nothing in it gives.
const DEFAULT_PHYSICS: Tuning = Tuning {
    gravity: 3000.0,
    max_release_speed: 2500.0,
    ground_friction: 0.8,
    grab: GrabString {
        length: 24.0,
        stiffness: 400.0,
        damping: 30.0,
        swing_drag: 4.0,
    },
};

impl Default for Balance {
    fn default() -> Self {
        Self {
            physics: DEFAULT_PHYSICS,
        }
    }
}

/// An Obstacle the owner drew in `obstacles.toml`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ObstacleDef {
    /// In physical pixels from the Home monitor's top-left corner, so the file still fits when the monitors around it
    /// move. The app moves it to screen coordinates.
    pub rect: Rect,
    pub material: Material,
}

/// The Material of an Obstacle that doesn't give one: as grippy as a Pet, and it never bounces.
pub const DEFAULT_OBSTACLE_MATERIAL: Material = Material {
    friction: 0.6,
    bounciness: 0.0,
};

/// Reads and checks `balance.toml` in `config_folder`. With no file, every value is its default.
pub fn load_balance(config_folder: &Path) -> Result<Balance, Vec<ConfigError>> {
    match read(config_folder, BALANCE_FILE)? {
        Some(text) => parse_balance(&text),
        None => Ok(Balance::default()),
    }
}

/// Reads and checks `obstacles.toml` in `config_folder`. With no file, there are no Obstacles.
pub fn load_obstacles(config_folder: &Path) -> Result<Vec<ObstacleDef>, Vec<ConfigError>> {
    match read(config_folder, OBSTACLES_FILE)? {
        Some(text) => parse_obstacles(&text),
        None => Ok(Vec::new()),
    }
}

/// The file's text, or `None` when it isn't there.
fn read(config_folder: &Path, file: &str) -> Result<Option<String>, Vec<ConfigError>> {
    match fs::read_to_string(config_folder.join(file)) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(vec![ConfigError::new(
            file,
            "",
            format!("can't be read: {error}"),
        )]),
    }
}

fn parse_balance(text: &str) -> Result<Balance, Vec<ConfigError>> {
    let root = parse_toml(BALANCE_FILE, text)?;
    let mut mistakes = Vec::new();
    let mut fields = Fields::read(&root, String::new(), &["physics"], &mut mistakes)
        .expect("a TOML file is always a table");
    let physics = fields
        .optional_with("physics", read_physics)
        .unwrap_or(DEFAULT_PHYSICS);
    check_physics(&physics, &mut mistakes);
    finish(BALANCE_FILE, Balance { physics }, mistakes)
}

/// Reads the physics' feel numbers. Any that are missing, or have a mistake, take their default, so the others are
/// still checked. A mistake means nothing loads, so a default never stands in for it.
fn read_physics(value: &Value, path: String, errors: &mut Vec<Mistake>) -> Option<Tuning> {
    let known = ["gravity", "max_release_speed", "ground_friction", "grab"];
    let mut fields = Fields::read(value, path, &known, errors)?;
    let default = DEFAULT_PHYSICS;
    Some(Tuning {
        gravity: optional_number(&mut fields, "gravity", default.gravity),
        max_release_speed: optional_number(
            &mut fields,
            "max_release_speed",
            default.max_release_speed,
        ),
        ground_friction: optional_number(&mut fields, "ground_friction", default.ground_friction),
        grab: fields
            .optional_with("grab", read_grab)
            .unwrap_or(default.grab),
    })
}

fn read_grab(value: &Value, path: String, errors: &mut Vec<Mistake>) -> Option<GrabString> {
    let mut fields = Fields::read(
        value,
        path,
        &["length", "stiffness", "damping", "swing_drag"],
        errors,
    )?;
    let default = DEFAULT_PHYSICS.grab;
    Some(GrabString {
        length: optional_number(&mut fields, "length", default.length),
        stiffness: optional_number(&mut fields, "stiffness", default.stiffness),
        damping: optional_number(&mut fields, "damping", default.damping),
        swing_drag: optional_number(&mut fields, "swing_drag", default.swing_drag),
    })
}

/// The number at `key`, or `default` when it's missing or has a mistake.
fn optional_number(fields: &mut Fields, key: &str, default: f32) -> f32 {
    fields.optional(key).flatten().unwrap_or(default)
}

fn check_physics(physics: &Tuning, errors: &mut Vec<Mistake>) {
    use Lowest::{AboveZero, Zero};
    check_lowest("physics.gravity", physics.gravity, AboveZero, errors);
    check_lowest(
        "physics.max_release_speed",
        physics.max_release_speed,
        AboveZero,
        errors,
    );
    material::check_share("physics.ground_friction", physics.ground_friction, errors);
    let grab = physics.grab;
    check_lowest("physics.grab.length", grab.length, Zero, errors);
    check_lowest("physics.grab.stiffness", grab.stiffness, AboveZero, errors);
    check_lowest("physics.grab.damping", grab.damping, Zero, errors);
    check_lowest("physics.grab.swing_drag", grab.swing_drag, Zero, errors);
    let fine = errors.is_empty();
    if fine {
        check_stable(grab, errors);
    }
}

/// The string's numbers must also let the physics step it without the Pet shaking or whirling for ever. That's a
/// limit of the physics, not a matter of taste, so values past it are mistakes. Its stiffness is checked first,
/// because how much damping it may have depends on it.
fn check_stable(grab: GrabString, errors: &mut Vec<Mistake>) {
    let tick = 1.0 / TICKS_PER_SECOND as f32;
    if grab.is_stable(tick) {
        return;
    }
    // How stiff any string may be, however it's damped: past that, more or less damping can't help.
    let undamped = GrabString {
        damping: 0.0,
        swing_drag: 0.0,
        ..grab
    };
    let stiffest = undamped.max_stable_stiffness(tick);
    let damping = grab.damping + grab.swing_drag;
    let (least, most) = (grab.min_stable_damping(), grab.max_stable_damping(tick));
    let mistake = if grab.stiffness > stiffest {
        Mistake::new(
            "physics.grab.stiffness",
            format!(
                "is {}, but must be at most {stiffest:.0}, or the Pet shakes on the string for ever",
                grab.stiffness
            ),
        )
    } else if damping < least {
        Mistake::new(
            "physics.grab.damping",
            format!(
                "plus swing_drag is {damping}, but must be at least {least:.1} for this stiffness, or the Pet whirls on \
                 the string for ever"
            ),
        )
    } else {
        Mistake::new(
            "physics.grab.damping",
            format!(
                "plus swing_drag is {damping}, but must be at most {most:.0} for this stiffness, or the Pet shakes on \
                 the string for ever"
            ),
        )
    };
    errors.push(mistake);
}

/// The smallest a feel number may be.
enum Lowest {
    AboveZero,
    Zero,
}

/// Checks `value` is a finite number no smaller than `lowest`. TOML allows `inf` and `nan`, and a number too big for
/// an `f32` becomes `inf`, so finite is checked first.
fn check_lowest(path: &str, value: f32, lowest: Lowest, errors: &mut Vec<Mistake>) {
    let rule = if !value.is_finite() {
        "must be a finite number"
    } else {
        match lowest {
            Lowest::AboveZero if value <= 0.0 => "must be above 0",
            Lowest::Zero if value < 0.0 => "must be 0 or more",
            _ => return,
        }
    };
    errors.push(Mistake::new(path, format!("is {value}, but {rule}")));
}

fn parse_obstacles(text: &str) -> Result<Vec<ObstacleDef>, Vec<ConfigError>> {
    let root = parse_toml(OBSTACLES_FILE, text)?;
    let mut mistakes = Vec::new();
    let mut fields = Fields::read(&root, String::new(), &["obstacle"], &mut mistakes)
        .expect("a TOML file is always a table");
    // Only those that read cleanly are here, but with a mistake in any of them, none load.
    let obstacles = fields.list_with("obstacle", read_obstacle);
    finish(OBSTACLES_FILE, obstacles, mistakes)
}

fn read_obstacle(value: &Value, path: String, errors: &mut Vec<Mistake>) -> Option<ObstacleDef> {
    let mut fields = Fields::read(value, path, &["rect", "material"], errors)?;
    let material_path = fields.path_of("material");
    let rect = fields.required_with("rect", read_rect);
    let material = fields.optional_with("material", material::read);
    if let Some(material) = material {
        material::check(&material_path, material, errors);
    }
    Some(ObstacleDef {
        rect: rect?,
        material: material.unwrap_or(DEFAULT_OBSTACLE_MATERIAL),
    })
}

/// Reads `{ x, y, width, height }`, which is easier to write by hand than the four edges a `Rect` keeps.
fn read_rect(value: &Value, path: String, errors: &mut Vec<Mistake>) -> Option<Rect> {
    let mut fields = Fields::read(value, path.clone(), &["x", "y", "width", "height"], errors)?;
    let x: Option<i32> = fields.required("x");
    let y: Option<i32> = fields.required("y");
    let width: Option<i32> = fields.required("width");
    let height: Option<i32> = fields.required("height");
    for (key, size) in [("width", width), ("height", height)] {
        if let Some(size) = size
            && size <= 0
        {
            fields.error(key, format!("is {size}, but must be above 0"));
        }
    }
    let (x, y, width, height) = (x?, y?, width?, height?);
    if width <= 0 || height <= 0 {
        return None;
    }
    let (Some(right), Some(bottom)) = (x.checked_add(width), y.checked_add(height)) else {
        let message = format!("reaches past {}, the largest coordinate there is", i32::MAX);
        errors.push(Mistake::new(path, message));
        return None;
    };
    Some(Rect {
        left: x,
        top: y,
        right,
        bottom,
    })
}

/// The file's text as a table, or the one error saying it isn't TOML.
fn parse_toml(file: &str, text: &str) -> Result<Value, Vec<ConfigError>> {
    let table: toml::Table = text.parse().map_err(|error: toml::de::Error| {
        vec![ConfigError::new(
            file,
            "",
            format!("isn't valid TOML: {error}"),
        )]
    })?;
    Ok(Value::Table(table))
}

/// The mistakes in `file`, or `read` if there are none, so a file loads completely or not at all.
fn finish<T>(file: &str, read: T, mistakes: Vec<Mistake>) -> Result<T, Vec<ConfigError>> {
    if mistakes.is_empty() {
        return Ok(read);
    }
    Err(mistakes
        .into_iter()
        .map(|mistake| ConfigError::new(file, mistake.field, mistake.message))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `obstacles.toml` as the item 4 design shows it.
    const OBSTACLES_EXAMPLE: &str = r#"
        [[obstacle]]
        rect = { x = 1600, y = 1500, width = 40, height = 500 }
        material = { friction = 0.1, bounciness = 0.0 }   # a wall to slide down
    "#;

    /// An Obstacle with nothing wrong, to go beside broken ones.
    const GOOD_OBSTACLE: &str = "[[obstacle]]\nrect = { x = 0, y = 0, width = 10, height = 10 }\n";

    fn fields(errors: &[ConfigError]) -> Vec<&str> {
        errors.iter().map(|error| error.field.as_str()).collect()
    }

    #[test]
    fn given_an_empty_balance_file_when_parsing_then_every_value_is_its_default() {
        // Given
        let text = "# nothing tuned yet\n";

        // When
        let balance = parse_balance(text);

        // Then
        assert_eq!(balance, Ok(Balance::default()));
    }

    #[test]
    fn given_some_values_including_one_inside_grab_when_parsing_then_the_others_keep_their_defaults()
     {
        // Given
        let text = "[physics]\ngravity = 1234\ngrab = { stiffness = 567 }\n";

        // When
        let balance = parse_balance(text).expect("a good file");

        // Then
        let default = Balance::default().physics;
        let expected = Tuning {
            gravity: 1234.0,
            grab: GrabString {
                stiffness: 567.0,
                ..default.grab
            },
            ..default
        };
        assert_eq!(balance.physics, expected);
    }

    #[test]
    fn given_an_unknown_table_and_an_unknown_field_when_parsing_the_balance_then_both_are_reported()
    {
        // Given: XP comes in a later slice, so a table for it is a mistake for now
        let text = "[physics]\ngravty = 1000\n\n[xp]\nlevel_curve = \"100\"\n";

        // When
        let errors = parse_balance(text).expect_err("unknown fields");

        // Then
        assert_eq!(fields(&errors), ["xp", "physics.gravty"]);
        assert!(errors.iter().all(|error| error.file == BALANCE_FILE));
    }

    #[test]
    fn given_every_value_out_of_its_range_when_parsing_the_balance_then_every_one_is_reported() {
        // Given
        let text = "[physics]\ngravity = 0\nmax_release_speed = -1\nground_friction = 1.5\n\
                    grab = { length = -1, stiffness = 0, damping = -0.5, swing_drag = -1 }\n";

        // When
        let errors = parse_balance(text).expect_err("out of range");

        // Then
        assert_eq!(
            fields(&errors),
            [
                "physics.gravity",
                "physics.max_release_speed",
                "physics.ground_friction",
                "physics.grab.length",
                "physics.grab.stiffness",
                "physics.grab.damping",
                "physics.grab.swing_drag"
            ]
        );
    }

    #[test]
    fn given_a_grab_string_too_stiff_to_settle_when_parsing_the_balance_then_its_stiffness_is_reported_with_the_limit()
     {
        // Given: far stiffer than the physics can step without the Pet shaking for ever
        let text = "[physics]\ngrab = { stiffness = 100000 }\n";

        // When
        let errors = parse_balance(text).expect_err("too stiff");

        // Then
        assert_eq!(fields(&errors), ["physics.grab.stiffness"]);
        assert!(
            errors[0].message.contains("at most"),
            "{}",
            errors[0].message
        );
    }

    #[test]
    fn given_a_grab_string_with_too_little_or_too_much_damping_when_parsing_then_its_damping_is_reported()
     {
        // Given: none at all, so it whirls for ever, and so much the physics can't step it
        let texts = [
            "[physics]\ngrab = { damping = 0, swing_drag = 0 }\n",
            "[physics]\ngrab = { damping = 100000 }\n",
        ];

        // When
        let errors = texts.map(|text| parse_balance(text).expect_err("unstable"));

        // Then
        for errors in errors {
            assert_eq!(fields(&errors), ["physics.grab.damping"]);
        }
    }

    #[test]
    fn given_values_that_are_not_finite_when_parsing_the_balance_then_each_is_reported() {
        // Given: TOML has nan and inf, and 1e300 is too big for an f32, so it becomes inf
        let text = "[physics]\ngravity = inf\nmax_release_speed = nan\nground_friction = nan\n\
                    grab = { length = inf, stiffness = 1e300, damping = inf, swing_drag = nan }\n";

        // When
        let errors = parse_balance(text).expect_err("not finite");

        // Then
        assert_eq!(
            fields(&errors),
            [
                "physics.gravity",
                "physics.max_release_speed",
                "physics.ground_friction",
                "physics.grab.length",
                "physics.grab.stiffness",
                "physics.grab.damping",
                "physics.grab.swing_drag"
            ]
        );
    }

    #[test]
    fn given_values_of_the_wrong_kind_when_parsing_the_balance_then_each_is_reported_and_nothing_loads()
     {
        // Given: a value that can't be read doesn't quietly become its default
        let text = "[physics]\ngravity = \"strong\"\ngrab = 24\n";

        // When
        let errors = parse_balance(text).expect_err("wrong kinds");

        // Then
        assert_eq!(fields(&errors), ["physics.gravity", "physics.grab"]);
        assert_eq!(errors[1].message, "should be a table, but is an integer");
    }

    #[test]
    fn given_a_balance_file_that_is_not_toml_when_parsing_then_one_error_says_so() {
        // Given
        let text = "[physics\ngravity = 1";

        // When
        let errors = parse_balance(text).expect_err("not TOML");

        // Then
        assert_eq!(fields(&errors), [""]);
        assert!(errors[0].message.contains("isn't valid TOML"), "{errors:?}");
    }

    #[test]
    fn given_the_example_obstacles_file_when_parsing_then_it_gives_that_obstacle() {
        // Given
        let text = OBSTACLES_EXAMPLE;

        // When
        let obstacles = parse_obstacles(text);

        // Then
        let wall = ObstacleDef {
            rect: Rect {
                left: 1600,
                top: 1500,
                right: 1640,
                bottom: 2000,
            },
            material: Material {
                friction: 0.1,
                bounciness: 0.0,
            },
        };
        assert_eq!(obstacles, Ok(vec![wall]));
    }

    #[test]
    fn given_an_obstacle_with_no_material_when_parsing_then_it_has_the_default_obstacle_material() {
        // Given
        let text = GOOD_OBSTACLE;

        // When
        let obstacles = parse_obstacles(text).expect("a good file");

        // Then
        assert_eq!(obstacles.len(), 1);
        assert_eq!(obstacles[0].material, DEFAULT_OBSTACLE_MATERIAL);
    }

    #[test]
    fn given_an_obstacles_file_with_none_in_it_when_parsing_then_there_are_no_obstacles() {
        // Given
        let text = "# none yet\n";

        // When
        let obstacles = parse_obstacles(text);

        // Then
        assert_eq!(obstacles, Ok(Vec::new()));
    }

    #[test]
    fn given_mistakes_in_several_obstacles_beside_a_good_one_when_parsing_then_every_one_is_reported_and_none_load()
     {
        // Given: no width, a negative height, a bounciness above 1, an unknown field and no rect at all
        let text = format!(
            "{GOOD_OBSTACLE}\
             [[obstacle]]\nrect = {{ x = 0, y = 0, width = 0, height = -5 }}\n\
             [[obstacle]]\nrect = {{ x = 0, y = 0, width = 5, height = 5 }}\n\
             material = {{ friction = 0.5, bounciness = 2 }}\n\
             [[obstacle]]\ncolour = \"red\"\n"
        );

        // When
        let errors = parse_obstacles(&text).expect_err("broken Obstacles");

        // Then
        assert_eq!(
            fields(&errors),
            [
                "obstacle[1].rect.width",
                "obstacle[1].rect.height",
                "obstacle[2].material.bounciness",
                "obstacle[3].colour",
                "obstacle[3].rect"
            ]
        );
        assert!(errors.iter().all(|error| error.file == OBSTACLES_FILE));
    }

    #[test]
    fn given_a_rect_reaching_past_the_largest_coordinate_when_parsing_then_it_is_reported() {
        // Given: x + width doesn't fit in an i32
        let text =
            "[[obstacle]]\nrect = { x = 2000000000, y = 0, width = 500000000, height = 10 }\n";

        // When
        let errors = parse_obstacles(text).expect_err("too far");

        // Then
        assert_eq!(fields(&errors), ["obstacle[0].rect"]);
    }

    #[test]
    fn given_a_misspelt_list_name_when_parsing_the_obstacles_then_it_is_reported() {
        // Given
        let text = "[[obstacles]]\nrect = { x = 0, y = 0, width = 10, height = 10 }\n";

        // When
        let errors = parse_obstacles(text).expect_err("a misspelt name");

        // Then
        assert_eq!(fields(&errors), ["obstacles"]);
        assert!(errors[0].message.contains("obstacle"), "{errors:?}");
    }

    #[test]
    fn given_an_error_about_a_field_and_one_about_the_whole_file_when_shown_then_they_read_as_a_packs_do()
     {
        // Given
        let about_a_field = ConfigError::new(BALANCE_FILE, "physics.gravity", "must be above 0");
        let about_the_file = ConfigError::new(OBSTACLES_FILE, "", "can't be read");

        // When
        let shown = [about_a_field.to_string(), about_the_file.to_string()];

        // Then
        assert_eq!(
            shown,
            [
                "balance.toml: physics.gravity: must be above 0",
                "obstacles.toml: can't be read"
            ]
        );
    }
}
