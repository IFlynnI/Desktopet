//! Warnings and errors go to a small file. The release build has no console, so this is the only record of a failure.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::Path;
use std::sync::Mutex;

use bevy::app::App;
use bevy::log::BoxedLayer;
use bevy::log::tracing_subscriber::Layer;
use bevy::log::tracing_subscriber::filter::LevelFilter;
use bevy::log::tracing_subscriber::fmt;

const START_FRESH_ABOVE_BYTES: u64 = 1024 * 1024;

/// Keeps adding to the file until it passes the limit, then starts fresh the next time the app starts.
fn open(path: &Path) -> io::Result<File> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    let too_big = fs::metadata(path).is_ok_and(|file| file.len() > START_FRESH_ABOVE_BYTES);
    let mut options = OpenOptions::new();
    options.create(true);
    if too_big {
        options.write(true).truncate(true);
    } else {
        options.append(true);
    }
    options.open(path)
}

/// Adds the file to Bevy's logging. If it can't be opened the app runs without one, since there's nowhere to say so.
pub fn layer(_app: &mut App) -> Option<BoxedLayer> {
    let file = open(&pet_platform::log_file_path()?).ok()?;
    let layer = fmt::layer()
        .with_ansi(false)
        .with_writer(Mutex::new(file))
        .with_filter(LevelFilter::WARN);
    Some(Box::new(layer))
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::path::PathBuf;
    use std::{env, process};

    use super::*;

    /// A folder of its own for each test, removed afterwards.
    struct TestFolder(PathBuf);

    impl TestFolder {
        fn new(test: &str) -> Self {
            let name = format!("desktop-pet-log-{}-{test}", process::id());
            Self(env::temp_dir().join(name))
        }

        fn log(&self) -> PathBuf {
            self.0.join("nested").join("desktop-pet.log")
        }
    }

    impl Drop for TestFolder {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn fill(path: &Path, bytes: u64) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, vec![b'x'; bytes as usize]).unwrap();
    }

    #[test]
    fn given_no_folder_yet_when_opening_the_log_then_it_is_created() {
        // Given
        let folder = TestFolder::new("new");

        // When
        let opened = open(&folder.log());

        // Then
        assert!(opened.is_ok());
        assert!(folder.log().exists());
    }

    #[test]
    fn given_a_small_log_when_opening_it_then_new_lines_are_added_to_the_end() {
        // Given
        let folder = TestFolder::new("small");
        fill(&folder.log(), 10);

        // When
        writeln!(open(&folder.log()).unwrap(), "new").unwrap();

        // Then
        assert_eq!(fs::read_to_string(folder.log()).unwrap().len(), 14);
    }

    #[test]
    fn given_a_log_past_the_limit_when_opening_it_then_it_starts_fresh() {
        // Given
        let folder = TestFolder::new("big");
        fill(&folder.log(), START_FRESH_ABOVE_BYTES + 1);

        // When
        writeln!(open(&folder.log()).unwrap(), "new").unwrap();

        // Then
        assert_eq!(fs::read_to_string(folder.log()).unwrap().trim(), "new");
    }

    #[test]
    fn given_a_log_exactly_at_the_limit_when_opening_it_then_it_is_kept() {
        // Given
        let folder = TestFolder::new("limit");
        fill(&folder.log(), START_FRESH_ABOVE_BYTES);

        // When
        open(&folder.log()).unwrap();

        // Then
        assert_eq!(
            fs::metadata(folder.log()).unwrap().len(),
            START_FRESH_ABOVE_BYTES
        );
    }
}
