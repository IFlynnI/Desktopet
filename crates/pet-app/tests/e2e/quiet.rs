//! Tests that show things on screen but send no input: no keys, no mouse, no screenshots. The owner starts them with
//! `cargo e2e-quiet` and can keep working meanwhile. The tray is driven with the messages Windows sends it.

use std::fs;
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
    BROKEN_FIELDS, FailWhen, HotkeyTaken, Packs, PetProcess, Relaunched, StartupListGuard,
    assert_cursor_is_at, break_manifest, cursor_position, make_dpi_aware, read_startup_entry,
    wait_until, work_area, write_startup_entry,
};
use crate::metrics::{GpuUse, MAX_HIDDEN_CPU_PERCENT, cpu_percent_of_one_core};

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
fn given_the_app_listens_for_the_shell_when_it_runs_then_the_work_area_stays_the_same() {
    // Given: the shell's fullscreen message needs an appbar, and a docked one takes screen space
    make_dpi_aware();
    let before = work_area();

    // When
    let pet = PetProcess::launch();
    thread::sleep(Duration::from_secs(1));

    // Then
    assert_eq!(work_area(), before);
    let status = pet.close().expect("wait for the app");
    assert!(status.success(), "{status}");
    assert_eq!(work_area(), before);
}

#[test]
fn given_a_hidden_pet_when_asked_to_close_then_it_exits_on_its_own() {
    // Given
    let pet = PetProcess::launch();
    pet.tray().left_click();
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
    pet.tray().left_click();
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
    pet.tray().left_click();
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
fn given_a_manual_hide_when_the_app_relaunches_then_it_stays_hidden_until_the_tray_shows_it() {
    // Given
    let first = PetProcess::launch_failing(FailWhen::Hidden);
    let _data = first.data();

    // When: this copy fails as soon as it's Hidden
    first.tray().left_click();
    let status = first.wait_for_exit().expect("wait for the app");
    assert_eq!(status.code(), Some(RENDER_FAILURE_EXIT.into()));
    let relaunched = Relaunched::wait_for_it();

    // Then
    thread::sleep(Duration::from_secs(1));
    assert!(relaunched.overlay().is_none(), "it starts Hidden");
    relaunched.tray().left_click();
    assert!(
        wait_until(Duration::from_secs(5), || relaunched.overlay().is_some()),
        "the tray shows it"
    );
}

/// Quits through the tray and checks the app exited cleanly, which is how the owner gets out of an app with no Pet.
fn quit_through_the_tray(pet: PetProcess) {
    pet.tray().choose_quit();
    let status = pet.wait_for_exit().expect("wait for the app");
    assert!(status.success(), "{status}");
}

#[test]
fn given_a_good_pack_when_launching_then_the_log_file_says_which_pack_is_in_use() {
    // Given
    let pet = PetProcess::launch();
    let data = pet.data();

    // When
    let logged = wait_until(Duration::from_secs(5), || {
        data.log().contains("Using the Pack \"Placeholder dragon\"")
    });

    // Then
    assert!(logged, "log: {}", data.log());
}

#[test]
fn given_no_pack_when_launching_then_the_log_says_so_and_the_tray_still_quits() {
    // Given
    let pet = PetProcess::launch_with(Packs::Empty);
    let data = pet.data();

    // When
    let logged = wait_until(Duration::from_secs(5), || data.log().contains("no Pack"));

    // Then
    assert!(logged, "log: {}", data.log());
    quit_through_the_tray(pet);
}

#[test]
fn given_a_broken_pack_when_launching_then_every_error_is_in_the_log_and_the_tray_still_quits() {
    // Given
    let pet = PetProcess::launch_with(Packs::Broken);
    let data = pet.data();

    // When
    let logged = wait_until(Duration::from_secs(5), || {
        BROKEN_FIELDS.iter().all(|field| data.log().contains(field))
    });

    // Then
    assert!(logged, "log: {}", data.log());
    quit_through_the_tray(pet);
}

#[test]
fn given_the_override_names_another_packs_folder_when_launching_then_the_app_looks_there() {
    // Given: a good Pack in the usual folder, and a broken one in the folder the override names
    let pet = PetProcess::launch_with(Packs::OverriddenWithBroken);
    let data = pet.data();

    // When: only the broken Pack writes anything to the log
    let logged = wait_until(Duration::from_secs(5), || {
        BROKEN_FIELDS.iter().all(|field| data.log().contains(field))
    });

    // Then
    assert!(logged, "log: {}", data.log());
    assert!(data.log().contains("other-packs"), "log: {}", data.log());
}

/// How long a saved edit takes to show: the app waits 250 ms for the save to settle, then reads the Pack again.
const RELOAD: Duration = Duration::from_secs(5);

#[test]
fn given_a_running_pet_when_its_pack_is_edited_then_the_log_says_the_edited_pack_is_in_use() {
    // Given
    let pet = PetProcess::launch();
    let data = pet.data();
    let started = wait_until(RELOAD, || {
        data.log().contains("Using the Pack \"Placeholder dragon\"")
    });
    assert!(started, "log: {}", data.log());

    // When: the name changes, so the reload can't be mistaken for the start
    let manifest = data.pack_folder().join("manifest.toml");
    let text = fs::read_to_string(&manifest).expect("read the manifest");
    let edited = text.replace("name = \"Placeholder dragon\"", "name = \"Edited dragon\"");
    assert_ne!(
        edited, text,
        "the placeholder manifest changed, so update this"
    );
    fs::write(&manifest, edited).expect("write the edited manifest");

    // Then
    let reloaded = wait_until(RELOAD, || {
        data.log().contains("Reloaded the Pack \"Edited dragon\"")
    });
    assert!(reloaded, "log: {}", data.log());
    assert!(pet.is_shown());
}

#[test]
fn given_a_running_pet_when_its_manifest_is_broken_then_every_error_is_logged_and_the_pet_keeps_its_pack_until_it_is_fixed()
 {
    // Given
    let pet = PetProcess::launch();
    let data = pet.data();
    let started = wait_until(RELOAD, || {
        data.log().contains("Using the Pack \"Placeholder dragon\"")
    });
    assert!(started, "log: {}", data.log());
    let manifest = data.pack_folder().join("manifest.toml");
    let good = fs::read_to_string(&manifest).expect("read the manifest");

    // When
    break_manifest(&data.pack_folder());
    let logged = wait_until(RELOAD, || {
        BROKEN_FIELDS.iter().all(|field| data.log().contains(field))
    });

    // Then: still the old Pack, and still running
    assert!(logged, "log: {}", data.log());
    assert!(
        !data.log().contains("Reloaded the Pack"),
        "log: {}",
        data.log()
    );
    assert!(pet.is_shown());

    // When: fixed again, so the watching went on after the broken edit
    fs::write(&manifest, good).expect("write the good manifest");

    // Then
    let reloaded = wait_until(RELOAD, || {
        data.log()
            .contains("Reloaded the Pack \"Placeholder dragon\"")
    });
    assert!(reloaded, "log: {}", data.log());
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

#[test]
fn given_a_broken_balance_file_when_the_app_starts_then_the_pet_still_appears_and_the_log_names_each_error()
 {
    // Given
    let balance = "[physics]
gravity = -5
wobble = 1
";

    // When
    let pet = PetProcess::launch_with_config(Packs::Placeholder, &[("balance.toml", balance)]);

    // Then: the Overlay showed, so the app started with the default balance
    let data = pet.data();
    assert!(
        wait_until(Duration::from_secs(5), || data
            .log()
            .contains("balance.toml")),
        "the log never mentioned balance.toml: {}",
        data.log()
    );
    let log = data.log();
    assert!(log.contains("gravity"), "{log}");
    assert!(log.contains("wobble"), "{log}");
}
