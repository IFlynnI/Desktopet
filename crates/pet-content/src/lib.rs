//! Reads and checks Packs, and the files in the owner's config folder. A Pack is a folder with a `manifest.toml` and one
//! PNG strip per Animation. A Pack and each config file load completely or not at all: either every check passes, or
//! the caller gets every error at once.

mod config;
mod fields;
mod images;
mod manifest;
mod material;
mod validate;

use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use pet_core::pack::PackDefinition;

pub use config::{
    BALANCE_FILE, Balance, ConfigError, DEFAULT_OBSTACLE_MATERIAL, OBSTACLES_FILE, ObstacleDef,
    load_balance, load_obstacles,
};
pub use images::{FramePixels, PackPixels};

/// One thing wrong with a Pack, and where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackError {
    /// The file it's about, relative to the Pack folder, such as `manifest.toml` or `hatchling/idle.png`.
    pub file: String,
    /// Where in the manifest, such as `forms.hatchling.default_action`. Empty when it's about the whole file.
    pub field: String,
    pub message: String,
}

impl PackError {
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

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_error(f, &self.file, &self.field, &self.message)
    }
}

/// An error as the log and `pet-pack` show it: `file: field: message`, or `file: message` about the whole file.
fn write_error(f: &mut fmt::Formatter<'_>, file: &str, field: &str, message: &str) -> fmt::Result {
    if field.is_empty() {
        write!(f, "{file}: {message}")
    } else {
        write!(f, "{file}: {field}: {message}")
    }
}

/// A Pack that passed every check: its Definition, and the pixels of every Frame.
pub struct LoadedPack {
    pub definition: PackDefinition,
    pub pixels: PackPixels,
}

/// Reads and checks the Pack in `pack_folder`.
pub fn load(pack_folder: &Path) -> Result<LoadedPack, Vec<PackError>> {
    let text = fs::read_to_string(pack_folder.join(manifest::MANIFEST)).map_err(|error| {
        let message = match error.kind() {
            ErrorKind::NotFound => "is missing: every Pack folder needs one".to_owned(),
            _ => format!("can't be read: {error}"),
        };
        vec![PackError::new(manifest::MANIFEST, "", message)]
    })?;
    let (manifest, mut errors) = manifest::parse(&text)?;
    let sheets = images::read_sheets(pack_folder, &manifest);
    errors.extend(validate::check(&manifest, &sheets));
    if !errors.is_empty() {
        return Err(errors);
    }
    let definition = manifest::to_definition(&manifest);
    let pixels = images::pack_pixels(&manifest, &sheets, definition.hit_mask_threshold);
    Ok(LoadedPack { definition, pixels })
}

/// The first Pack in `packs_folder` by folder name, or `None` when there are no folders in it. The folder is chosen
/// by name alone, so one with a mistake in it is still the one chosen, and `load` says what's wrong.
pub fn discover(packs_folder: &Path) -> Option<PathBuf> {
    let mut folders: Vec<PathBuf> = fs::read_dir(packs_folder)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path())
        .collect();
    // Windows usually lists folders by name, but nothing promises it.
    folders.sort();
    folders.into_iter().next()
}
