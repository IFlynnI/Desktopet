//! Which Pack the app starts with. It's read before the engine starts, and the engine logs what happened, because the
//! log file only exists once the engine is up.

use std::ffi::OsString;
use std::path::PathBuf;

use pet_content::{LoadedPack, PackError};

/// Points the app at another folder of Packs, for development.
pub const PACKS_OVERRIDE: &str = "DESKTOP_PET_PACKS";

/// The Pack the app starts with, or why there isn't one.
pub enum StartingPack {
    Loaded {
        folder: PathBuf,
        pack: LoadedPack,
    },
    Broken {
        folder: PathBuf,
        errors: Vec<PackError>,
    },
    /// `None` when there's no folder to look in at all.
    NoPack {
        packs_folder: Option<PathBuf>,
    },
}

/// The override when it's set to something, or else the usual folder.
pub fn packs_folder(override_folder: Option<OsString>, usual: Option<PathBuf>) -> Option<PathBuf> {
    match override_folder {
        Some(folder) if !folder.is_empty() => Some(PathBuf::from(folder)),
        _ => usual,
    }
}

impl StartingPack {
    /// Loads the first Pack in `packs_folder` by folder name.
    pub fn load(packs_folder: Option<PathBuf>) -> Self {
        let chosen = packs_folder.as_deref().and_then(pet_content::discover);
        Self::load_chosen(packs_folder, chosen)
    }

    /// Loads the Pack `chosen` from `packs_folder`, or says there was none to choose.
    pub fn load_chosen(packs_folder: Option<PathBuf>, chosen: Option<PathBuf>) -> Self {
        let Some(folder) = chosen else {
            return Self::NoPack { packs_folder };
        };
        match pet_content::load(&folder) {
            Ok(pack) => Self::Loaded { folder, pack },
            Err(errors) => Self::Broken { folder, errors },
        }
    }

    /// What's wrong, one line each, for the log. Empty when the Pack loaded.
    pub fn problems(&self) -> Vec<String> {
        match self {
            Self::Loaded { .. } => Vec::new(),
            Self::Broken { folder, errors } => {
                let mut lines = vec![format!(
                    "The Pack in {} has {} errors, so there's no Pet:",
                    folder.display(),
                    errors.len()
                )];
                lines.extend(errors.iter().map(|error| format!("  {error}")));
                lines
            }
            Self::NoPack {
                packs_folder: Some(folder),
            } => vec![format!(
                "There's no Pack in {}, so there's no Pet. Each Pack is a folder in there.",
                folder.display()
            )],
            Self::NoPack { packs_folder: None } => vec![format!(
                "There's nowhere to look for Packs, because LOCALAPPDATA isn't set, so there's no Pet. \
                 Set {PACKS_OVERRIDE} to a folder of Packs."
            )],
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::{env, fs, process};

    use super::*;

    /// A folder of Packs of its own for each test, removed afterwards.
    struct TestPacks(PathBuf);

    impl TestPacks {
        fn new(test: &str) -> Self {
            let folder =
                env::temp_dir().join(format!("desktop-pet-packs-{}-{test}", process::id()));
            let _ = fs::remove_dir_all(&folder);
            fs::create_dir_all(&folder).expect("make a test folder");
            Self(folder)
        }

        fn write(&self, file: &str, contents: &[u8]) {
            let path = self.0.join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }

        fn add_placeholder(&self) {
            let placeholder =
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/placeholder");
            for file in ["manifest.toml", "hatchling/idle.png"] {
                self.write(
                    &format!("placeholder/{file}"),
                    &fs::read(placeholder.join(file)).unwrap(),
                );
            }
        }
    }

    impl Drop for TestPacks {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn given_the_override_is_set_when_choosing_the_packs_folder_then_it_wins() {
        // Given
        let override_folder = Some(OsString::from(r"D:\art\packs"));

        // When
        let chosen = packs_folder(override_folder, Some(PathBuf::from(r"C:\usual")));

        // Then
        assert_eq!(chosen, Some(PathBuf::from(r"D:\art\packs")));
    }

    #[test]
    fn given_the_override_is_unset_or_empty_when_choosing_the_packs_folder_then_it_is_the_usual_one()
     {
        // Given
        let usual = Some(PathBuf::from(r"C:\usual"));

        // When
        let chosen = [None, Some(OsString::new())].map(|set| packs_folder(set, usual.clone()));

        // Then
        assert_eq!(chosen, [usual.clone(), usual]);
    }

    #[test]
    fn given_a_good_pack_when_loading_then_it_is_used_and_nothing_is_wrong() {
        // Given
        let packs = TestPacks::new("good");
        packs.add_placeholder();

        // When
        let starting = StartingPack::load(Some(packs.0.clone()));

        // Then
        assert!(starting.problems().is_empty());
        let StartingPack::Loaded { folder, pack } = starting else {
            panic!("the placeholder loads");
        };
        assert_eq!(folder, packs.0.join("placeholder"));
        assert_eq!(pack.definition.name, "Placeholder dragon");
    }

    #[test]
    fn given_a_broken_pack_when_loading_then_every_error_is_a_line_for_the_log() {
        // Given: two mistakes
        let packs = TestPacks::new("broken");
        packs.add_placeholder();
        let manifest = fs::read_to_string(packs.0.join("placeholder/manifest.toml")).unwrap();
        let broken = manifest
            .replace("default_action = \"idle\"", "default_action = \"walk\"")
            .replace("[250, 250]", "[250, 0]");
        packs.write("placeholder/manifest.toml", broken.as_bytes());

        // When
        let problems = StartingPack::load(Some(packs.0.clone())).problems();

        // Then: a line saying which Pack, then one per error
        assert_eq!(problems.len(), 3, "{problems:?}");
        assert!(problems[0].contains("placeholder"), "{problems:?}");
        assert!(
            problems[1].contains("forms.hatchling.default_action"),
            "{problems:?}"
        );
        assert!(problems[2].contains("durations_ms"), "{problems:?}");
    }

    #[test]
    fn given_no_pack_in_the_folder_when_loading_then_the_log_says_where_it_looked() {
        // Given
        let packs = TestPacks::new("empty");

        // When
        let problems = StartingPack::load(Some(packs.0.clone())).problems();

        // Then
        assert_eq!(problems.len(), 1);
        assert!(
            problems[0].contains(&packs.0.display().to_string()),
            "{problems:?}"
        );
    }

    #[test]
    fn given_no_folder_to_look_in_when_loading_then_the_log_says_how_to_name_one() {
        // Given
        let packs_folder = None;

        // When
        let problems = StartingPack::load(packs_folder).problems();

        // Then
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains(PACKS_OVERRIDE), "{problems:?}");
        assert!(
            !problems[0].contains("  "),
            "no runs of spaces: {problems:?}"
        );
    }
}
