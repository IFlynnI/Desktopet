use std::ffi::c_void;

use pet_core::geometry::Rect;
use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromPoint,
};
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

/// The whole Home monitor, taskbar included.
pub fn home_monitor() -> Rect {
    // SAFETY: takes plain values, and asking for the primary monitor always finds one.
    let monitor = unsafe { MonitorFromPoint(POINT::default(), MONITOR_DEFAULTTOPRIMARY) };
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: `info` is a live local with its size set, as this call requires.
    let found = unsafe { GetMonitorInfoW(monitor, &mut info) };
    assert!(
        found.as_bool(),
        "Windows always reports the primary monitor"
    );
    let rect = info.rcMonitor;
    Rect {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_this_desktop_when_reading_the_home_monitor_then_it_covers_the_work_area() {
        // Given
        let work_area = home_work_area();

        // When
        let monitor = home_monitor();

        // Then
        assert!(monitor.covers(work_area));
    }
}
