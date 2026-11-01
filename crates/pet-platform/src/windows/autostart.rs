//! Launch on startup: one value in the owner's part of the registry, holding the quoted path of the running exe.
//!
//! It never touches the undocumented `StartupApproved` key. If the owner turns the app off in Windows Settings that
//! wins, and the checkbox can still show ticked. The value is never rewritten on launch: if the exe moves, the checkbox
//! shows unticked, and one click repairs it.

use std::path::Path;

use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW,
};
use windows::core::{HSTRING, Result, w};

/// The list of programs Windows starts when the owner signs in.
pub const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

const VALUE_NAME: windows::core::PCWSTR = w!("Desktop Pet");

/// Long enough for any Windows path, in UTF-16 units.
const LONGEST_VALUE: usize = 32_768;

fn run_value(exe: &Path) -> String {
    format!("\"{}\"", exe.display())
}

/// Windows paths ignore case, and the shell can hand back either form.
fn points_at(value: &str, exe: &Path) -> bool {
    value.eq_ignore_ascii_case(&run_value(exe))
}

/// One startup value. The key is a parameter, so tests can use a location of their own and never the real startup list.
pub struct Autostart {
    key: HSTRING,
}

impl Autostart {
    pub fn at(key: &str) -> Self {
        Self { key: key.into() }
    }

    fn value(&self) -> Option<String> {
        let mut buffer = vec![0u16; LONGEST_VALUE];
        let mut bytes = size_of_val(buffer.as_slice()) as u32;
        // SAFETY: `buffer` and `bytes` are live locals, and `bytes` says how much Windows may write.
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                &self.key,
                VALUE_NAME,
                RRF_RT_REG_SZ,
                None,
                Some(buffer.as_mut_ptr().cast()),
                Some(&mut bytes),
            )
        };
        if status != ERROR_SUCCESS {
            return None;
        }
        // Windows counts the terminating zero, which isn't part of the text.
        let length = (bytes as usize / size_of::<u16>()).saturating_sub(1);
        Some(String::from_utf16_lossy(&buffer[..length]))
    }

    /// True only when the value points at `exe`, so a moved exe reads as off.
    pub fn is_on(&self, exe: &Path) -> bool {
        self.value().is_some_and(|value| points_at(&value, exe))
    }

    pub fn set(&self, on: bool, exe: &Path) -> Result<()> {
        if on { self.write(exe) } else { self.remove() }
    }

    fn write(&self, exe: &Path) -> Result<()> {
        let value = HSTRING::from(run_value(exe));
        let text: &[u16] = &value;
        // SAFETY: `value` outlives the call, and the size covers its text and terminating zero, which Windows requires.
        let status = unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                &self.key,
                VALUE_NAME,
                REG_SZ.0,
                Some(text.as_ptr().cast()),
                ((text.len() + 1) * size_of::<u16>()) as u32,
            )
        };
        status.ok()
    }

    fn remove(&self) -> Result<()> {
        // SAFETY: both names outlive the call.
        let status = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, &self.key, VALUE_NAME) };
        if status == ERROR_FILE_NOT_FOUND {
            return Ok(());
        }
        status.ok()
    }
}

/// What the checkbox should show after a click: what was wanted, unless writing it failed.
pub fn settled_state(wanted: bool, write: impl FnOnce(bool) -> Result<()>) -> bool {
    match write(wanted) {
        Ok(()) => wanted,
        Err(error) => {
            log::error!("couldn't change the startup setting: {error}");
            !wanted
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use windows::Win32::Foundation::E_ACCESSDENIED;
    use windows::Win32::System::Registry::RegDeleteTreeW;

    use super::*;

    /// A key of its own for each test, removed afterwards. Nothing here goes near the real startup list.
    struct TestKey {
        path: String,
    }

    impl TestKey {
        fn new(test: &str) -> Self {
            let path = format!(r"Software\DesktopPetTest-{}-{test}", std::process::id());
            Self { path }
        }

        fn autostart(&self) -> Autostart {
            Autostart::at(&self.path)
        }
    }

    impl Drop for TestKey {
        fn drop(&mut self) {
            let path = HSTRING::from(self.path.as_str());
            // SAFETY: `path` outlives the call, and it's under this test's own key.
            let _ = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, &path) };
        }
    }

    fn exe() -> PathBuf {
        PathBuf::from(r"C:\Program Files\Desktop Pet\desktop-pet.exe")
    }

    fn moved_exe() -> PathBuf {
        PathBuf::from(r"D:\Games\desktop-pet.exe")
    }

    #[test]
    fn given_no_startup_value_when_reading_then_it_is_off() {
        // Given
        let key = TestKey::new("no-value");

        // When
        let on = key.autostart().is_on(&exe());

        // Then
        assert!(!on);
    }

    #[test]
    fn given_it_is_off_when_turning_it_on_then_the_value_is_the_quoted_path_of_the_exe() {
        // Given
        let key = TestKey::new("turn-on");
        let autostart = key.autostart();

        // When
        autostart.set(true, &exe()).unwrap();

        // Then: the quotes are what keep a path with spaces in one piece
        assert!(autostart.is_on(&exe()));
        assert_eq!(
            autostart.value().as_deref(),
            Some(r#""C:\Program Files\Desktop Pet\desktop-pet.exe""#)
        );
    }

    #[test]
    fn given_it_is_on_when_turning_it_off_then_the_value_is_gone() {
        // Given
        let key = TestKey::new("turn-off");
        let autostart = key.autostart();
        autostart.set(true, &exe()).unwrap();

        // When
        autostart.set(false, &exe()).unwrap();

        // Then
        assert_eq!(autostart.value(), None);
    }

    #[test]
    fn given_no_startup_value_when_turning_it_off_then_nothing_goes_wrong() {
        // Given
        let key = TestKey::new("already-off");

        // When
        let result = key.autostart().set(false, &exe());

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn given_the_value_points_at_an_old_location_when_reading_then_it_is_off() {
        // Given
        let key = TestKey::new("moved");
        let autostart = key.autostart();
        autostart.set(true, &exe()).unwrap();

        // When
        let on = autostart.is_on(&moved_exe());

        // Then
        assert!(!on);
    }

    #[test]
    fn given_the_value_points_at_an_old_location_when_turning_it_on_then_it_is_repaired() {
        // Given
        let key = TestKey::new("repair");
        let autostart = key.autostart();
        autostart.set(true, &exe()).unwrap();

        // When
        autostart.set(true, &moved_exe()).unwrap();

        // Then
        assert!(autostart.is_on(&moved_exe()));
    }

    #[test]
    fn given_the_same_path_in_other_letter_case_when_reading_then_it_is_on() {
        // Given
        let key = TestKey::new("case");
        let autostart = key.autostart();
        autostart.set(true, &exe()).unwrap();

        // When
        let on = autostart.is_on(Path::new(r"c:\program files\desktop pet\DESKTOP-PET.EXE"));

        // Then
        assert!(on);
    }

    #[test]
    fn given_writing_works_when_settling_the_checkbox_then_it_shows_what_was_wanted() {
        // Given
        let write = |_| Ok(());

        // When
        let shown = settled_state(true, write);

        // Then
        assert!(shown);
    }

    #[test]
    fn given_writing_fails_when_settling_the_checkbox_then_it_goes_back() {
        // Given
        let write = |_| Err(E_ACCESSDENIED.into());

        // When
        let shown = settled_state(true, write);

        // Then
        assert!(!shown);
    }
}
