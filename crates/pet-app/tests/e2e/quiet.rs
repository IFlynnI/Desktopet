//! Tests that never touch the mouse, so they can run while the owner is working.

use std::panic::{self, AssertUnwindSafe};
use std::thread;
use std::time::Duration;

use pet_app::RENDER_FAILURE_EXIT;
use pet_core::geometry::Point;
use windows::Win32::Foundation::{HANDLE, RECT};
use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RegDeleteTreeW};
use windows::Win32::UI::WindowsAndMessaging::{
    GetPropW, GetWindowRect, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST,
};
use windows::core::w;

use crate::harness::{
    FailWhen, HotkeyTaken, PetProcess, Relaunched, StartupListGuard, assert_cursor_is_at,
    cursor_position, make_dpi_aware, press_hotkey, read_startup_entry, wait_until, work_area,
    write_startup_entry,
};
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

    // Then: it stays up and closes normally, with no pop-up in the way, and the log says why the hotkey is off
    thread::sleep(Duration::from_secs(1));
    assert!(pet.is_shown());
    let data = pet.data();
    let status = pet.close().expect("wait for the app");
    assert!(status.success(), "{status}");
    assert!(data.log().contains("Ctrl+Alt+P"), "log: {}", data.log());
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

#[test]
fn given_the_app_started_when_asking_the_taskbar_then_it_has_the_tray_icon() {
    // Given
    let pet = PetProcess::launch();

    // When
    let tray = pet.tray();

    // Then
    assert!(tray.icon_exists());
}

#[test]
fn given_a_shown_pet_when_left_clicking_the_tray_icon_then_it_hides_and_a_second_click_shows_it() {
    // Given
    let pet = PetProcess::launch();
    let tray = pet.tray();

    // When
    tray.left_click();

    // Then
    assert!(pet.wait_until_hidden(), "the first click hides the Overlay");

    // When
    tray.left_click();

    // Then
    assert!(pet.wait_until_shown(), "the second click shows it again");
}

#[test]
fn given_a_shown_pet_when_choosing_show_hide_then_it_hides() {
    // Given
    let pet = PetProcess::launch();
    let tray = pet.tray();

    // When
    tray.choose_show_hide();

    // Then
    assert!(pet.wait_until_hidden());
}

#[test]
fn given_a_running_pet_when_choosing_quit_then_it_exits_cleanly() {
    // Given
    let pet = PetProcess::launch();
    let tray = pet.tray();

    // When
    tray.choose_quit();

    // Then
    let status = pet.wait_for_exit().expect("wait for the app");
    assert!(status.success(), "{status}");
}

#[test]
fn given_a_hidden_pet_when_choosing_quit_then_it_still_exits() {
    // Given: the engine is stopped, so the wake-up has to reach it
    let pet = PetProcess::launch();
    let tray = pet.tray();
    tray.left_click();
    assert!(pet.wait_until_hidden());

    // When
    tray.choose_quit();

    // Then
    let status = pet.wait_for_exit().expect("wait for the app");
    assert!(status.success(), "{status}");
}

#[test]
fn given_a_hidden_pet_when_launching_again_then_the_first_shows_and_the_second_exits() {
    // Given
    let pet = PetProcess::launch();
    press_hotkey();
    assert!(pet.wait_until_hidden());

    // When
    let status = pet.launch_again().expect("run the second launch");

    // Then
    assert!(status.success(), "{status}");
    assert!(pet.wait_until_shown());
}

#[test]
fn given_a_shown_pet_when_launching_again_then_it_stays_shown_and_the_second_exits() {
    // Given
    let pet = PetProcess::launch();

    // When
    let status = pet.launch_again().expect("run the second launch");

    // Then
    assert!(status.success(), "{status}");
    assert!(pet.is_shown());
}

#[test]
#[should_panic(expected = "quit the running Pet first")]
fn given_a_pet_is_running_when_the_harness_launches_one_then_it_stops_with_a_clear_reason() {
    // Given
    let _pet = PetProcess::launch();

    // When / Then
    PetProcess::launch();
}

#[test]
fn given_a_test_changed_the_startup_list_when_it_ends_then_the_entry_goes_back_and_the_test_fails()
{
    // Given: a key of its own, so this never touches the real list
    let key = w!(r"Software\DesktopPetTest-guard");
    let changed_entry = [b'x', 0, 0, 0];

    // When
    let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
        let _guard = StartupListGuard::of(key);
        write_startup_entry(key, Some(&changed_entry));
    }));

    // Then
    let after = read_startup_entry(key);
    // SAFETY: the name is a static string, and it's the key this test made.
    let _ = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, key) };
    assert!(outcome.is_err(), "the test should have failed");
    assert_eq!(after, None, "the entry should have gone back");
}

#[test]
fn given_an_app_that_keeps_failing_when_it_relaunches_then_it_stops_after_three_and_says_why() {
    // Given / When: the first run plus 3 Relaunches, each failing soon after it appears
    let pet = PetProcess::launch_failing(FailWhen::Shown);
    let data = pet.data();

    // Then
    assert!(
        wait_until(Duration::from_secs(30), || data
            .log()
            .contains("stays closed")),
        "log: {}",
        data.log()
    );
    drop(pet);
    let record = data.record();
    let count = |wanted: &str| record.iter().filter(|line| *line == wanted).count();
    assert_eq!(count("started"), 4, "{record:?}");
    assert_eq!(
        count("stopped"),
        4,
        "the shell thread had stopped each time: {record:?}"
    );
}

#[test]
fn given_a_manual_hide_when_the_app_relaunches_then_it_stays_hidden_until_the_hotkey_shows_it() {
    // Given
    let first = PetProcess::launch_failing(FailWhen::Hidden);
    let _data = first.data();

    // When: this copy fails as soon as it's Hidden
    press_hotkey();
    let status = first.wait_for_exit().expect("wait for the app");
    assert_eq!(status.code(), Some(RENDER_FAILURE_EXIT.into()));
    let relaunched = Relaunched::wait_for_it();

    // Then
    thread::sleep(Duration::from_secs(1));
    assert!(relaunched.overlay().is_none(), "it starts Hidden");
    press_hotkey();
    assert!(
        wait_until(Duration::from_secs(5), || relaunched.overlay().is_some()),
        "the hotkey shows it"
    );
}

#[test]
fn given_the_cursor_is_where_it_was_left_when_checking_it_then_the_run_carries_on() {
    // Given
    let left_at = cursor_position();

    // When / Then
    assert_cursor_is_at(left_at);
}

#[test]
#[should_panic(expected = "the mouse moved during the run")]
fn given_the_cursor_is_somewhere_else_when_checking_it_then_the_run_stops_with_a_clear_reason() {
    // Given: this only compares positions, so the mouse stays where it is
    let cursor = cursor_position();
    let expected = Point {
        x: cursor.x + 1,
        y: cursor.y,
    };

    // When / Then
    assert_cursor_is_at(expected);
}
