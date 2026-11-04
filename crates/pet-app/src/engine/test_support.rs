//! What several of the engine's unit tests share: a captured log, a stand-in for the platform's hit-test thread, and
//! Packs written into folders of their own.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::{env, fs, io, process};

use bevy::log::tracing;
use bevy::log::tracing_subscriber::fmt;
use bevy::prelude::*;
use pet_content::LoadedPack;
use pet_core::hit_mask::{HitMask, Placement};
use pet_platform::HitTarget;

use super::renderer::HitTest;

/// Everything logged while it's in use, as text.
#[derive(Clone, Default)]
pub struct CapturedLog(Arc<Mutex<Vec<u8>>>);

impl io::Write for CapturedLog {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl CapturedLog {
    /// Runs `run` with everything it logs on this thread captured.
    pub fn during(&self, run: impl FnOnce()) {
        let writer = self.clone();
        let subscriber = fmt()
            .with_ansi(false)
            .with_writer(move || writer.clone())
            .finish();
        tracing::subscriber::with_default(subscriber, run);
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
    }
}

/// Stands in for the platform, keeping every Hit target it's given.
#[derive(Resource, Default)]
pub struct RecordingPlatform(Mutex<Vec<HitTarget>>);

impl HitTest for RecordingPlatform {
    fn publish_hit_target(&self, target: HitTarget) {
        self.0.lock().unwrap().push(target);
    }
}

impl RecordingPlatform {
    pub fn published(app: &App) -> Vec<(HitMask, Placement)> {
        app.world()
            .resource::<RecordingPlatform>()
            .0
            .lock()
            .unwrap()
            .iter()
            .map(|target| ((*target.mask).clone(), target.placement))
            .collect()
    }
}

pub fn placeholder_folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/placeholder")
}

pub fn placeholder_manifest() -> String {
    fs::read_to_string(placeholder_folder().join("manifest.toml")).unwrap()
}

/// A folder of its own for a test's Pack, removed afterwards.
pub struct TestPack(pub PathBuf);

impl Drop for TestPack {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The Pack `manifest` describes, in a folder named after `test`, loaded. The placeholder's idle strip, two Frames of
/// 24 x 18, is there for it as `hatchling/idle.png`.
pub fn pack_from(test: &str, manifest: &str) -> (TestPack, LoadedPack) {
    let folder = env::temp_dir().join(format!("desktop-pet-engine-{}-{test}", process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(folder.join("hatchling")).unwrap();
    fs::copy(
        placeholder_folder().join("hatchling/idle.png"),
        folder.join("hatchling/idle.png"),
    )
    .unwrap();
    fs::write(folder.join("manifest.toml"), manifest).unwrap();
    let pack = pet_content::load(&folder).unwrap_or_else(|errors| panic!("{errors:?}"));
    (TestPack(folder), pack)
}
