//! Windows implementation of the platform API. All Win32 and unsafe code lives under this folder.

mod dpi;
mod hit_thread;
mod monitor;
mod window_styles;

use std::ffi::c_void;
use std::sync::{Arc, Mutex};

use raw_window_handle::RawWindowHandle;
use windows::Win32::Foundation::HWND;

use crate::HitTarget;

pub use monitor::home_work_area;

/// OS setup that must run before Bevy starts. Call it first in `main`.
pub fn prepare_process() {
    dpi::make_aware();
    // The Visual swapchain is what makes the window truly transparent on NVIDIA (spike finding 1).
    // SAFETY: the standard library documents `set_var` as always safe on Windows.
    unsafe { std::env::set_var("WGPU_DX12_PRESENTATION_SYSTEM", "Visual") };
}

pub struct Platform {
    hit_target: Arc<Mutex<Option<HitTarget>>>,
}

impl Platform {
    /// Attaches to the Overlay and starts the background work. Call it before the Overlay is first shown.
    pub fn start(window: RawWindowHandle) -> Self {
        let RawWindowHandle::Win32(handle) = window else {
            panic!("the Overlay on Windows has a Win32 window handle");
        };
        let overlay = handle.hwnd.get();
        let hwnd = HWND(overlay as *mut c_void);
        window_styles::declare_not_fullscreen(hwnd);
        window_styles::apply(hwnd, false);

        let hit_target = Arc::new(Mutex::new(None));
        hit_thread::spawn(overlay, hit_target.clone());
        Self { hit_target }
    }

    pub fn publish_hit_target(&self, target: HitTarget) {
        *self.hit_target.lock().unwrap() = Some(target);
    }
}
