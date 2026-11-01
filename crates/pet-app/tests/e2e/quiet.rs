//! Tests that never touch the mouse, so they can run while the owner is working.

use std::thread;
use std::time::Duration;

use windows::Win32::Foundation::{HANDLE, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetPropW, GetWindowRect, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST,
};
use windows::core::w;

use crate::harness::{HotkeyTaken, PetProcess, make_dpi_aware, press_hotkey, work_area};
use crate::metrics::{GpuUse, cpu_percent_of_one_core};

/// A judgement, not a measurement: the most that still counts as stopped.
const MAX_HIDDEN_CPU_PERCENT: f64 = 0.5;

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

#[test]
fn given_a_shown_pet_when_pressing_the_hotkey_then_it_hides_and_a_second_press_shows_it() {
    // Given
    let pet = PetProcess::launch();

    // When
    press_hotkey();

    // Then
    assert!(pet.wait_until_hidden(), "the first press hides the Overlay");

    // When
    press_hotkey();

    // Then
    assert!(pet.wait_until_shown(), "the second press shows it again");
}

#[test]
fn given_a_hidden_pet_when_asked_to_close_then_it_exits_on_its_own() {
    // Given
    let pet = PetProcess::launch();
    press_hotkey();
    assert!(pet.wait_until_hidden());

    // When
    let status = pet.close().expect("wait for the app");

    // Then
    assert!(status.success(), "{status}");
}

#[test]
fn given_another_app_owns_the_hotkey_when_launching_then_the_pet_still_runs() {
    // Given
    let _taken = HotkeyTaken::by_this_test();

    // When
    let pet = PetProcess::launch();

    // Then: it stays up and closes normally, with no pop-up in the way
    thread::sleep(Duration::from_secs(1));
    assert!(pet.is_shown());
    let status = pet.close().expect("wait for the app");
    assert!(status.success(), "{status}");
}

#[test]
fn given_a_hidden_pet_when_measured_then_it_uses_next_to_no_cpu_and_no_gpu() {
    // Given: the counter reads above zero while shown, so a zero below means something
    let pet = PetProcess::launch();
    let gpu = GpuUse::of(pet.child().id());
    thread::sleep(Duration::from_secs(2));
    let gpu_shown = gpu.percent();
    press_hotkey();
    assert!(pet.wait_until_hidden());
    thread::sleep(Duration::from_secs(5));
    gpu.percent();

    // When
    let cpu_hidden = cpu_percent_of_one_core(pet.child(), Duration::from_secs(10));
    let gpu_hidden = gpu.percent();

    // Then
    println!("GPU shown {gpu_shown:.2}%, GPU hidden {gpu_hidden:.2}%, CPU hidden {cpu_hidden:.3}%");
    assert!(gpu_shown > 0.0, "the GPU counter isn't reading this app");
    assert!(
        cpu_hidden < MAX_HIDDEN_CPU_PERCENT,
        "{cpu_hidden:.3}% of one core"
    );
    assert_eq!(gpu_hidden, 0.0);
}
