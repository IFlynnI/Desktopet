//! The owner's config files the app starts with: the feel numbers in `balance.toml` and the walls in `obstacles.toml`.
//! They're read before the engine starts, and the engine logs what went wrong, because the log file only exists once
//! the engine is up. A missing file is normal and gives the defaults. A broken one gives the defaults too, and every
//! error is logged. Slice 1 reads them once, so an edit takes effect at the next start; hot reload comes later.

use std::path::Path;

use pet_content::{Balance, ConfigError, ObstacleDef};
use pet_core::geometry::{Point, Rect};
use pet_core::physics::{Obstacle, Tuning};

/// What the app starts with from the config folder, and what went wrong reading it.
pub struct StartingConfig {
    pub physics: Tuning,
    /// On screen, in physical pixels.
    pub obstacles: Vec<Obstacle>,
    /// One line each, for the log.
    pub problems: Vec<String>,
}

impl StartingConfig {
    /// Reads the config folder, if there is one. `home_monitor` is the Home monitor's rect on screen: Obstacles are
    /// given relative to its top-left corner.
    pub fn load(folder: Option<&Path>, home_monitor: Rect) -> Self {
        let Some(folder) = folder else {
            return Self {
                physics: Balance::default().physics,
                obstacles: Vec::new(),
                problems: vec![
                    "There's no config folder, because LOCALAPPDATA isn't set, so the Pet uses the default \
                     balance and no Obstacles."
                        .to_owned(),
                ],
            };
        };
        let mut problems = Vec::new();
        let balance = pet_content::load_balance(folder).unwrap_or_else(|errors| {
            problems.extend(report(
                folder,
                pet_content::BALANCE_FILE,
                "the default balance",
                &errors,
            ));
            Balance::default()
        });
        let obstacles = pet_content::load_obstacles(folder).unwrap_or_else(|errors| {
            problems.extend(report(
                folder,
                pet_content::OBSTACLES_FILE,
                "no Obstacles",
                &errors,
            ));
            Vec::new()
        });
        Self {
            physics: balance.physics,
            obstacles: on_screen(&obstacles, home_monitor),
            problems,
        }
    }
}

/// Obstacles moved from the Home monitor's corner to their place on screen.
pub fn on_screen(obstacles: &[ObstacleDef], home_monitor: Rect) -> Vec<Obstacle> {
    let corner = Point {
        x: home_monitor.left,
        y: home_monitor.top,
    };
    obstacles
        .iter()
        .map(|obstacle| Obstacle {
            rect: Rect {
                left: obstacle.rect.left + corner.x,
                top: obstacle.rect.top + corner.y,
                right: obstacle.rect.right + corner.x,
                bottom: obstacle.rect.bottom + corner.y,
            },
            material: obstacle.material,
        })
        .collect()
}

fn report(folder: &Path, file: &str, instead: &str, errors: &[ConfigError]) -> Vec<String> {
    let mut lines = vec![format!(
        "{} has {} errors, so the Pet uses {instead} until it's fixed and the Pet restarts:",
        folder.join(file).display(),
        errors.len()
    )];
    lines.extend(errors.iter().map(|error| format!("  {error}")));
    lines
}

#[cfg(test)]
mod tests {
    use std::{env, fs, process};

    use pet_core::physics::Material;

    use super::*;

    const MONITOR: Rect = Rect {
        left: -1920,
        top: 100,
        right: 0,
        bottom: 1180,
    };

    /// A config folder of its own for one test, removed afterwards.
    struct Folder(std::path::PathBuf);

    impl Folder {
        fn new(test: &str, files: &[(&str, &str)]) -> Self {
            let path = env::temp_dir().join(format!("pet-app-config-{test}-{}", process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            for (name, text) in files {
                fs::write(path.join(name), text).unwrap();
            }
            Self(path)
        }
    }

    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn given_an_obstacle_relative_to_the_home_monitor_when_placing_it_on_screen_then_it_moves_by_the_monitors_corner()
     {
        // Given: a monitor left of the primary one, so its corner is negative
        let obstacle = ObstacleDef {
            rect: Rect {
                left: 10,
                top: 20,
                right: 50,
                bottom: 520,
            },
            material: Material {
                friction: 0.1,
                bounciness: 0.0,
            },
        };

        // When
        let placed = on_screen(&[obstacle], MONITOR);

        // Then
        assert_eq!(
            placed,
            [Obstacle {
                rect: Rect {
                    left: -1910,
                    top: 120,
                    right: -1870,
                    bottom: 620,
                },
                material: obstacle.material,
            }]
        );
    }

    #[test]
    fn given_an_empty_config_folder_when_starting_then_it_is_the_default_balance_with_no_obstacles_and_no_problems()
     {
        // Given
        let folder = Folder::new("empty", &[]);

        // When
        let config = StartingConfig::load(Some(&folder.0), MONITOR);

        // Then
        assert_eq!(config.physics, Balance::default().physics);
        assert!(config.obstacles.is_empty());
        assert!(config.problems.is_empty());
    }

    #[test]
    fn given_broken_files_when_starting_then_it_is_the_defaults_and_every_error_of_each_file_is_a_problem()
     {
        // Given
        let folder = Folder::new(
            "broken",
            &[
                (pet_content::BALANCE_FILE, "[physics]\ngravity = -1\n"),
                (pet_content::OBSTACLES_FILE, "[[obstacle]]\nrect = 3\n"),
            ],
        );

        // When
        let config = StartingConfig::load(Some(&folder.0), MONITOR);

        // Then
        assert_eq!(config.physics, Balance::default().physics);
        assert!(config.obstacles.is_empty());
        let headers: Vec<&String> = config
            .problems
            .iter()
            .filter(|line| !line.starts_with("  "))
            .collect();
        assert_eq!(headers.len(), 2, "{:?}", config.problems);
        assert!(headers[0].contains(pet_content::BALANCE_FILE));
        assert!(headers[1].contains(pet_content::OBSTACLES_FILE));
        assert!(config.problems.len() >= 4, "{:?}", config.problems);
    }

    #[test]
    fn given_no_config_folder_when_starting_then_it_is_the_defaults_and_says_why() {
        // Given
        let folder = None;

        // When
        let config = StartingConfig::load(folder, MONITOR);

        // Then
        assert_eq!(config.physics, Balance::default().physics);
        assert!(config.obstacles.is_empty());
        assert_eq!(config.problems.len(), 1);
    }
}
