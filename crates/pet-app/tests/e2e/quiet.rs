//! Tests that never touch the mouse, so they can run while the owner is working.

use std::thread;
use std::time::Duration;

use windows::Win32::Foundation::{HANDLE, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetPropW, GetWindowRect, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST,
};
use windows::core::w;

use crate::harness::{PetProcess, make_dpi_aware, work_area};

#[test]
fn given_a_running_pet_when_asked_to_close_then_it_exits_on_its_own() {
    // Given
    let pet = PetProcess::launch();

    // When
    let status = pet.close().expect("wait for the app");

    // Then: a killed process reports a failure
    assert!(status.success(), "{status}");
}

#[test]
fn given_the_app_started_when_inspecting_the_overlay_then_it_is_a_topmost_tool_window_that_never_activates()
 {
    // Given
    let pet = PetProcess::launch();

    // When: winit reset styles once at startup in the spike, so check after it has settled
    thread::sleep(Duration::from_secs(2));
    let style = pet.ex_style();

    // Then
    assert_ne!(
        style & WS_EX_TOOLWINDOW.0,
        0,
        "hidden from the taskbar and Alt-Tab"
    );
    assert_eq!(style & WS_EX_APPWINDOW.0, 0, "hidden from the taskbar");
    assert_ne!(style & WS_EX_NOACTIVATE.0, 0, "never takes focus");
    assert_ne!(style & WS_EX_TOPMOST.0, 0, "above normal windows");
    assert_ne!(style & WS_EX_LAYERED.0, 0, "per-pixel click-through");
}

#[test]
fn given_the_app_started_when_inspecting_the_overlay_then_it_covers_the_work_area() {
    // Given
    make_dpi_aware();
    let pet = PetProcess::launch();
    let work = work_area();

    // When
    let mut rect = RECT::default();
    // SAFETY: `rect` is a live local that Windows writes during the call.
    unsafe { GetWindowRect(pet.overlay(), &mut rect) }.expect("read the Overlay's rect");

    // Then: the Overlay's size is rounded down to even numbers
    assert_eq!(rect.left, work.left);
    assert_eq!(rect.bottom, work.bottom);
    assert!(work.width() - (rect.right - rect.left) <= 1);
    assert!(work.height() - (rect.bottom - rect.top) <= 1);
}

#[test]
fn given_the_app_started_when_inspecting_the_overlay_then_the_shell_is_told_it_is_not_a_fullscreen_app()
 {
    // Given
    let pet = PetProcess::launch();

    // When
    // SAFETY: the name is a static string and the Overlay is alive for the whole test.
    let property: HANDLE = unsafe { GetPropW(pet.overlay(), w!("NonRudeHWND")) };

    // Then
    assert!(!property.is_invalid() && !property.0.is_null());
}
