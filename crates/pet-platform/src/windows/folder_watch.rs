//! Tells whoever asks when the files in a folder change, including in its subfolders. Windows signals a change handle,
//! and a thread of its own sleeps in `WaitForMultipleObjects` until it does, using no CPU, so nothing polls.
//!
//! An editor saves in bursts, so each change only restarts a short wait, and the caller hears once the burst has
//! settled (`pet_core::settle`). That wait is the sleep's timeout: a one-shot timer that each change restarts, not a
//! poll, and with nothing pending the thread sleeps until the next change. The caller is called on this thread, so it
//! can take its time, such as reading the whole folder again, without holding anyone else up. A change made meanwhile
//! is kept, and heard after.
//!
//! Only names, sizes and writes count. Last-access times don't, or reading the folder after a change would be a change.
//!
//! Windows follows the folder itself, not its name. Renamed, it goes on reporting under its new name. Deleted, it stops
//! reporting, and a new folder of the same name would go unheard. So the folder it's in is watched too, for folders
//! coming and going. When the folder's creation time differs, because it has gone, come back or been replaced, the
//! caller hears, and the watch moves to whatever folder has that name now. Another folder next to it changes nothing.

use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::MetadataExt;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use std::{fs, io};

use pet_core::settle::Settle;
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows::Win32::Storage::FileSystem::{
    FILE_NOTIFY_CHANGE, FILE_NOTIFY_CHANGE_DIR_NAME, FILE_NOTIFY_CHANGE_FILE_NAME,
    FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_CHANGE_SIZE, FindCloseChangeNotification,
    FindFirstChangeNotificationW, FindNextChangeNotification,
};
use windows::Win32::System::Threading::{CreateEventW, INFINITE, SetEvent, WaitForMultipleObjects};
use windows::core::PCWSTR;

/// What counts as a change inside the folder.
const CHANGES: FILE_NOTIFY_CHANGE = FILE_NOTIFY_CHANGE(
    FILE_NOTIFY_CHANGE_FILE_NAME.0
        | FILE_NOTIFY_CHANGE_DIR_NAME.0
        | FILE_NOTIFY_CHANGE_SIZE.0
        | FILE_NOTIFY_CHANGE_LAST_WRITE.0,
);

/// Windows' word that something changed in a folder. Closed on drop.
struct ChangeHandle(HANDLE);

// SAFETY: a change handle is a kernel handle, which any thread may wait on, ask for the next change on, and close.
unsafe impl Send for ChangeHandle {}

impl ChangeHandle {
    fn open(folder: &Path, subfolders: bool, changes: FILE_NOTIFY_CHANGE) -> io::Result<Self> {
        let path: Vec<u16> = folder.as_os_str().encode_wide().chain([0]).collect();
        // SAFETY: `path` is a live, nul-terminated local that Windows reads during the call.
        let handle =
            unsafe { FindFirstChangeNotificationW(PCWSTR(path.as_ptr()), subfolders, changes) }?;
        Ok(Self(handle))
    }

    /// Windows keeps the handle signalled until it's asked for the next change, so this must follow each one.
    fn next(&self) -> windows::core::Result<()> {
        // SAFETY: this value owns the handle, which is open until it drops.
        unsafe { FindNextChangeNotification(self.0) }
    }
}

impl Drop for ChangeHandle {
    fn drop(&mut self) {
        // SAFETY: this value owns the handle, and nothing uses it after this.
        let _ = unsafe { FindCloseChangeNotification(self.0) };
    }
}

/// The event that tells the thread to stop. Closed once both its owners have dropped it.
struct StopEvent(HANDLE);

// SAFETY: an event is a kernel handle, which any thread may set, wait on and close.
unsafe impl Send for StopEvent {}

// SAFETY: setting an event on one thread while another waits on it is what events are for.
unsafe impl Sync for StopEvent {}

impl Drop for StopEvent {
    fn drop(&mut self) {
        // SAFETY: this value owns the event, and nothing uses it after this.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// A folder being watched. Dropping it stops the watching, and waits for a call already under way to finish.
pub struct FolderWatch {
    stop: Arc<StopEvent>,
    thread: Option<JoinHandle<()>>,
}

impl FolderWatch {
    /// Starts watching `folder` and everything in it. Once a burst of changes has settled, as `settle` decides,
    /// `on_settled` is called, on the watcher's own thread. Changes made after this returns are all heard.
    pub fn start(
        folder: &Path,
        settle: Settle,
        on_settled: impl FnMut() + Send + 'static,
    ) -> io::Result<Self> {
        let change = ChangeHandle::open(folder, true, CHANGES)?;
        // A folder at the root of a drive has no folder to be in, so then it's only watched itself.
        let around = folder
            .parent()
            .and_then(|parent| ChangeHandle::open(parent, false, FILE_NOTIFY_CHANGE_DIR_NAME).ok());
        // SAFETY: takes plain values. Manual reset, so it stays set once the thread is told to stop.
        let stop = Arc::new(StopEvent(unsafe { CreateEventW(None, true, false, None) }?));
        let watched = Watched {
            folder: folder.to_owned(),
            created: created(folder),
            change: Some(change),
            around,
        };
        let thread_stop = stop.clone();
        let thread = thread::Builder::new()
            .name("folder-watch".into())
            .spawn(move || watched.run(&thread_stop, settle, on_settled))?;
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }
}

impl Drop for FolderWatch {
    fn drop(&mut self) {
        // SAFETY: the event stays open until the last of its owners drops, and this is one of them.
        let _ = unsafe { SetEvent(self.stop.0) };
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// When the folder was made, or `None` when there's no folder of that name. A folder made again has a new one.
fn created(folder: &Path) -> Option<u64> {
    fs::metadata(folder)
        .ok()
        .filter(fs::Metadata::is_dir)
        .map(|metadata| metadata.creation_time())
}

/// Whole milliseconds, rounded up, so a wait never ends just before it's due and has to go round again for nothing.
fn timeout_ms(wait: Option<Duration>) -> u32 {
    wait.map_or(INFINITE, |wait| {
        let ms = wait.as_nanos().div_ceil(1_000_000);
        u32::try_from(ms).unwrap_or(INFINITE).min(INFINITE - 1)
    })
}

/// What the thread watches.
struct Watched {
    folder: PathBuf,
    /// When the folder of that name was made, as last seen.
    created: Option<u64>,
    /// Changes inside the folder. `None` once Windows has stopped reporting them, until a folder of that name is back.
    change: Option<ChangeHandle>,
    /// Folders coming and going next to it. `None` if Windows won't report them.
    around: Option<ChangeHandle>,
}

impl Watched {
    /// Sleeps until a change, the stop event, or the end of the settle wait, until told to stop.
    fn run(mut self, stop: &StopEvent, mut settle: Settle, mut on_settled: impl FnMut()) {
        loop {
            let mut handles = vec![stop.0];
            handles.extend(self.change.as_ref().map(|change| change.0));
            handles.extend(self.around.as_ref().map(|around| around.0));
            let timeout = timeout_ms(settle.wait(Instant::now()));
            // SAFETY: every handle stays open until this call has returned.
            let woke = unsafe { WaitForMultipleObjects(&handles, false, timeout) };
            let signalled = woke.0.wrapping_sub(WAIT_OBJECT_0.0) as usize;
            let inside = |handle: HANDLE| {
                self.change
                    .as_ref()
                    .is_some_and(|change| change.0 == handle)
            };
            match handles.get(signalled).copied() {
                Some(handle) if handle == stop.0 => return,
                Some(handle) if inside(handle) => {
                    settle.changed(Instant::now());
                    self.next_change();
                }
                Some(_) => {
                    if self.folder_replaced() {
                        settle.changed(Instant::now());
                    }
                }
                None if woke == WAIT_TIMEOUT => {}
                None => {
                    log::warn!(
                        "Stopped watching {} for changes, because waiting for one failed: {}",
                        self.folder.display(),
                        io::Error::last_os_error()
                    );
                    return;
                }
            }
            if settle.take_settled(Instant::now()) {
                self.call(&mut on_settled);
            }
        }
    }

    /// Calls the caller. A panic in it would end this thread, and with it every later change, with nobody told, so
    /// it's caught, logged, and the watching goes on.
    fn call(&self, on_settled: &mut impl FnMut()) {
        if let Err(panic) = panic::catch_unwind(AssertUnwindSafe(on_settled)) {
            let reason = panic
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| panic.downcast_ref::<String>().map(String::as_str))
                .unwrap_or("no reason given");
            log::error!(
                "Acting on a change in {} failed, so it's skipped, and later changes still count: {reason}",
                self.folder.display()
            );
        }
    }

    /// Asks for the next change inside the folder. Windows refuses once the folder has gone.
    fn next_change(&mut self) {
        let Some(change) = &self.change else { return };
        if let Err(error) = change.next() {
            log::warn!(
                "Windows stopped reporting changes in {}, so it's watched again only if a folder of that name \
                 comes back: {error}",
                self.folder.display()
            );
            self.change = None;
        }
    }

    /// Whether the folder has gone, come back or been replaced since it was last seen. If a folder of that name is
    /// there now, the watch moves to it.
    fn folder_replaced(&mut self) -> bool {
        if let Some(around) = &self.around
            && around.next().is_err()
        {
            self.around = None;
        }
        let created = created(&self.folder);
        if created == self.created {
            return false;
        }
        self.created = created;
        // The old folder's handle goes with that folder, wherever it went.
        self.change = None;
        if created.is_some() {
            match ChangeHandle::open(&self.folder, true, CHANGES) {
                Ok(change) => self.change = Some(change),
                Err(error) => log::warn!(
                    "Can't watch {} for changes again: {error}",
                    self.folder.display()
                ),
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
    use std::time::Instant;
    use std::{env, fs, process, thread};

    use super::*;

    /// Short, so the tests are quick. The app's is longer.
    const SETTLE: Duration = Duration::from_millis(50);

    fn settle() -> Settle {
        Settle::new(SETTLE, Duration::from_secs(2))
    }

    /// Far longer than a change takes to arrive, so only a missing report runs this long.
    const BOUND: Duration = Duration::from_secs(5);

    /// Long enough after a report for a second one to have arrived, if there were going to be one.
    const NO_MORE: Duration = Duration::from_millis(400);

    /// A folder of its own for each test, removed afterwards.
    struct TestFolder(PathBuf);

    impl TestFolder {
        fn new(test: &str) -> Self {
            let folder =
                env::temp_dir().join(format!("desktop-pet-watch-{}-{test}", process::id()));
            let _ = fs::remove_dir_all(&folder);
            fs::create_dir_all(folder.join("hatchling")).expect("make a test folder");
            Self(folder)
        }

        fn write(&self, file: &str, contents: &str) {
            fs::write(self.0.join(file), contents).expect("write a test file");
        }
    }

    impl Drop for TestFolder {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// A watch on `folder` that sends on the channel each time a burst has settled.
    fn watching(folder: &TestFolder) -> (FolderWatch, Receiver<()>) {
        let (sender, reports) = mpsc::channel();
        let watch = FolderWatch::start(&folder.0, settle(), move || {
            let _ = sender.send(());
        })
        .expect("watch the test folder");
        (watch, reports)
    }

    #[test]
    fn given_a_watched_folder_when_a_file_in_it_is_written_then_the_change_is_reported_once() {
        // Given
        let folder = TestFolder::new("file");
        let (_watch, reports) = watching(&folder);

        // When
        folder.write("manifest.toml", "name = \"Edited\"");

        // Then
        assert_eq!(reports.recv_timeout(BOUND), Ok(()));
        assert_eq!(
            reports.recv_timeout(NO_MORE),
            Err(RecvTimeoutError::Timeout)
        );
    }

    #[test]
    fn given_a_watched_folder_when_a_file_in_a_subfolder_is_written_then_the_change_is_reported() {
        // Given
        let folder = TestFolder::new("subfolder");
        let (_watch, reports) = watching(&folder);

        // When
        folder.write("hatchling/idle.png", "not really a PNG");

        // Then
        assert_eq!(reports.recv_timeout(BOUND), Ok(()));
    }

    #[test]
    fn given_a_watched_folder_when_nothing_changes_then_nothing_is_reported() {
        // Given
        let folder = TestFolder::new("quiet");

        // When
        let (_watch, reports) = watching(&folder);

        // Then
        assert_eq!(
            reports.recv_timeout(NO_MORE),
            Err(RecvTimeoutError::Timeout)
        );
    }

    #[test]
    fn given_a_watched_folder_when_an_editor_saves_in_a_burst_then_it_is_reported_once_after_the_burst()
     {
        // Given
        let folder = TestFolder::new("burst");
        let (_watch, reports) = watching(&folder);

        // When: a temporary file, a rename over the real one, then two more writes, closer together than the settle
        let saved = Instant::now();
        folder.write("manifest.toml.tmp", "name = \"Edited\"");
        fs::rename(
            folder.0.join("manifest.toml.tmp"),
            folder.0.join("manifest.toml"),
        )
        .expect("rename the temporary file");
        for contents in ["name = \"Edited again\"", "name = \"Edited at last\""] {
            thread::sleep(SETTLE / 5);
            folder.write("manifest.toml", contents);
        }

        // Then
        assert_eq!(reports.recv_timeout(BOUND), Ok(()));
        assert!(
            saved.elapsed() >= SETTLE,
            "it waited for the burst to settle"
        );
        assert_eq!(
            reports.recv_timeout(NO_MORE),
            Err(RecvTimeoutError::Timeout)
        );
    }

    #[test]
    fn given_a_folder_that_does_not_exist_when_starting_to_watch_it_then_it_says_why_it_cannot() {
        // Given
        let folder = env::temp_dir().join(format!("desktop-pet-watch-{}-missing", process::id()));

        // When
        let watch = FolderWatch::start(&folder, settle(), || {});

        // Then
        assert!(watch.is_err());
    }

    #[test]
    fn given_a_running_watch_when_it_is_dropped_then_it_stops_straight_away() {
        // Given: the thread is asleep, waiting for a change
        let folder = TestFolder::new("drop");
        let (watch, _reports) = watching(&folder);

        // When
        let dropping = Instant::now();
        drop(watch);

        // Then
        assert!(dropping.elapsed() < Duration::from_secs(1));
    }

    /// The test folder's own folder, so a test can rename and replace it.
    fn parent(folder: &TestFolder) -> PathBuf {
        folder
            .0
            .parent()
            .expect("the test folder is in a folder")
            .to_owned()
    }

    #[test]
    fn given_a_watched_folder_when_it_is_deleted_then_that_is_reported() {
        // Given
        let folder = TestFolder::new("deleted");
        let (_watch, reports) = watching(&folder);

        // When
        fs::remove_dir_all(&folder.0).expect("delete the folder");

        // Then
        assert_eq!(reports.recv_timeout(BOUND), Ok(()));
    }

    #[test]
    fn given_a_watched_folder_was_deleted_when_one_of_the_same_name_is_made_and_written_to_then_both_are_reported()
     {
        // Given: Windows stops reporting on a folder that has gone, so this has to be noticed some other way
        let folder = TestFolder::new("replaced");
        let (_watch, reports) = watching(&folder);
        fs::remove_dir_all(&folder.0).expect("delete the folder");
        assert_eq!(reports.recv_timeout(BOUND), Ok(()));

        // When
        fs::create_dir_all(folder.0.join("hatchling")).expect("make the folder again");
        assert_eq!(reports.recv_timeout(BOUND), Ok(()), "it came back");
        folder.write("hatchling/idle.png", "not really a PNG");

        // Then
        assert_eq!(reports.recv_timeout(BOUND), Ok(()), "a write in it");
    }

    #[test]
    fn given_a_watched_folder_when_it_is_renamed_away_and_back_then_each_is_reported() {
        // Given
        let folder = TestFolder::new("renamed");
        let away =
            parent(&folder).join(format!("desktop-pet-watch-{}-renamed-away", process::id()));
        let _ = fs::remove_dir_all(&away);
        let (_watch, reports) = watching(&folder);

        // When
        fs::rename(&folder.0, &away).expect("rename the folder away");
        let gone = reports.recv_timeout(BOUND);
        fs::rename(&away, &folder.0).expect("rename it back");
        let back = reports.recv_timeout(BOUND);
        folder.write("manifest.toml", "name = \"Edited\"");

        // Then
        assert_eq!((gone, back), (Ok(()), Ok(())));
        assert_eq!(reports.recv_timeout(BOUND), Ok(()), "a write in it");
    }

    #[test]
    fn given_a_watched_folder_when_a_folder_next_to_it_is_made_then_nothing_is_reported() {
        // Given: another Pack's folder, in the same Packs folder
        let folder = TestFolder::new("sibling");
        let sibling =
            parent(&folder).join(format!("desktop-pet-watch-{}-sibling-other", process::id()));
        let (_watch, reports) = watching(&folder);

        // When
        fs::create_dir_all(&sibling).expect("make a folder next to it");

        // Then
        let reported = reports.recv_timeout(NO_MORE);
        let _ = fs::remove_dir_all(&sibling);
        assert_eq!(reported, Err(RecvTimeoutError::Timeout));
    }

    #[test]
    fn given_the_caller_panics_on_a_change_when_the_folder_changes_again_then_that_change_is_still_reported()
     {
        // Given: the first call panics, and later ones report
        let folder = TestFolder::new("panic");
        let (sender, reports) = mpsc::channel();
        let mut calls = 0;
        let _watch = FolderWatch::start(&folder.0, settle(), move || {
            calls += 1;
            assert!(
                calls > 1,
                "the first call fails, as a bug in the caller would"
            );
            let _ = sender.send(());
        })
        .expect("watch the test folder");
        folder.write("manifest.toml", "name = \"Edited\"");
        thread::sleep(NO_MORE);

        // When
        folder.write("manifest.toml", "name = \"Edited again\"");

        // Then
        assert_eq!(reports.recv_timeout(BOUND), Ok(()));
    }
}
