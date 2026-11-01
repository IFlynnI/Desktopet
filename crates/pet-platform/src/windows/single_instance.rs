//! One Pet per Windows session, using a named mutex. A mutex here is only a name Windows keeps while a handle to it is
//! open: nobody locks it, and it goes away by itself when the app exits, however that happens.
//!
//! Microsoft warns another program could take the name first. That program would have to run in the owner's session,
//! where it could simply kill the Pet, so a random name adds nothing.

use windows::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, LPARAM, SetLastError, WIN32_ERROR,
    WPARAM,
};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{FindWindowExW, HWND_MESSAGE, PostMessageW};
use windows::core::w;

use super::shell_thread::{CLASS, PLEASE_SHOW};

/// Held by the first Pet for as long as it runs. Dropping it lets another Pet start.
pub struct SingleInstance(HANDLE);

impl Drop for SingleInstance {
    fn drop(&mut self) {
        // SAFETY: this value owns the handle, and `drop` runs exactly once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// Asks the first Pet to show itself. If its message window isn't there yet, it's still starting and will show anyway.
fn ask_first_pet_to_show() {
    // SAFETY: the class name is a static string and the rest are plain values.
    let Ok(window) = (unsafe { FindWindowExW(Some(HWND_MESSAGE), None, CLASS, None) }) else {
        return;
    };
    // SAFETY: takes plain values. A window that's gone makes this fail, and then there's no Pet to show.
    let _ = unsafe { PostMessageW(Some(window), PLEASE_SHOW, WPARAM(0), LPARAM(0)) };
}

/// A guard for the first Pet. A second launch asks the first to show and gets nothing, and should then exit.
pub fn acquire_single_instance() -> Option<SingleInstance> {
    // SAFETY: takes a plain value. It clears any old error, so the check below reads what this call reported.
    unsafe { SetLastError(WIN32_ERROR(0)) };
    // SAFETY: the name is a static string, and passing no security attributes asks for the defaults.
    let created = unsafe { CreateMutexW(None, false, w!("Local\\DesktopPet")) };
    // SAFETY: takes no arguments. Windows reports "it already existed" here, even though the call succeeded.
    let already_exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    match created {
        Ok(mutex) if !already_exists => Some(SingleInstance(mutex)),
        Ok(mutex) => {
            // SAFETY: this handle came from the call above and nothing else uses it.
            let _ = unsafe { CloseHandle(mutex) };
            ask_first_pet_to_show();
            None
        }
        Err(error) => {
            // Only a Pet running with more rights than this one can refuse us the name.
            log::warn!("couldn't open the single-instance mutex: {error}");
            ask_first_pet_to_show();
            None
        }
    }
}
