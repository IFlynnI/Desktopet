//! Helpers the test files share.

use std::path::{Path, PathBuf};
use std::{env, fs, process};

/// A folder of its own for each test, removed afterwards.
pub struct TestFolder(PathBuf);

impl TestFolder {
    pub fn new(test: &str) -> Self {
        let folder = env::temp_dir().join(format!("pet-content-{}-{test}", process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("make a test folder");
        Self(folder)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn write(&self, file: &str, contents: &[u8]) {
        let path = self.0.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
}

impl Drop for TestFolder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
