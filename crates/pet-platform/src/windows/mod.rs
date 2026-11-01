//! Windows implementation of the platform API. All Win32 and unsafe code lives under this folder.

mod autostart;
mod dpi;
mod hit_thread;
mod hotkey;
mod monitor;
mod shell_thread;
mod single_instance;
mod tray;
mod window_styles;

use std::env;
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use raw_window_handle::RawWindowHandle;
use windows::Win32::Foundation::HWND;

use crate::{HitTarget, ShellEvent, TrayIconImage, Wake};
use hit_thread::HitTestPause;
use shell_thread::ShellThread;

pub use monitor::home_work_area;
pub use single_instance::{SingleInstance, acquire_single_instance};

/// OS setup that must run before Bevy starts. Call it first in `main`.
pub fn prepare_process() {
    dpi::make_aware();
    // The Visual swapchain is what makes the window truly transparent on NVIDIA (spike finding 1).
    // SAFETY: the standard library documents `set_var` as always safe on Windows.
    unsafe { std::env::set_var("WGPU_DX12_PRESENTATION_SYSTEM", "Visual") };
}

/// Where warnings and errors go. Roadmap item 6 can use the same folder for the save file.
pub fn log_file_path() -> Option<PathBuf> {
    let local_data = env::var_os("LOCALAPPDATA")?;
    Some(
        PathBuf::from(local_data)
            .join("DesktopPet")
            .join("desktop-pet.log"),
    )
}

pub struct Platform {
    hit_target: Arc<Mutex<Option<HitTarget>>>,
    hit_test_pause: Arc<HitTestPause>,
    _shell: ShellThread,
}

impl Platform {
    /// Attaches to the Overlay and starts the background work. Call it before the Overlay is first shown.
    pub fn start(
        window: RawWindowHandle,
        events: Sender<ShellEvent>,
        wake: Wake,
        tray_icon: TrayIconImage,
    ) -> Self {
        let RawWindowHandle::Win32(handle) = window else {
            panic!("the Overlay on Windows has a Win32 window handle");
        };
        let overlay = handle.hwnd.get();
        let hwnd = HWND(overlay as *mut c_void);
        window_styles::declare_not_fullscreen(hwnd);
        window_styles::apply(hwnd, false);

        let hit_target = Arc::new(Mutex::new(None));
        let hit_test_pause = Arc::new(HitTestPause::default());
        hit_thread::spawn(overlay, hit_target.clone(), hit_test_pause.clone());
        Self {
            hit_target,
            hit_test_pause,
            _shell: ShellThread::spawn(events, wake, tray_icon),
        }
    }

    pub fn publish_hit_target(&self, target: HitTarget) {
        *self.hit_target.lock().unwrap() = Some(target);
    }

    /// Pauses or resumes the hit-test thread. The engine shows and hides the Overlay itself.
    pub fn set_hidden(&self, hidden: bool) {
        self.hit_test_pause.set_hidden(hidden);
    }
}
