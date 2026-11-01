use std::ffi::c_void;

use pet_core::geometry::Rect;
use windows::Win32::Foundation::RECT;
use windows::Win32::UI::WindowsAndMessaging::{
    SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
};

/// The Home monitor's work area, which leaves out the taskbar. The Home monitor is the primary monitor
/// until settings arrive.
pub fn home_work_area() -> Rect {
    let mut rect = RECT::default();
    // SAFETY: `rect` is a live local of the type SPI_GETWORKAREA writes during the call.
    unsafe {
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut rect as *mut RECT as *mut c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    }
    .expect("Windows always reports a work area");
    Rect {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    }
}
