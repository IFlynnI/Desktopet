//! Tests that need the real mouse, or that fill the screen and take the foreground. Keep your hands off while they run.

use std::thread;
use std::time::Duration;

use pet_core::geometry::{Point, Rect};

use crate::harness::{
    CursorGuard, PetProcess, click, foreground_window, make_dpi_aware, monitor_area, wait_until,
    window_under_cursor_at, work_area,
};
use crate::metrics::{GpuUse, MAX_HIDDEN_CPU_PERCENT, cpu_percent_of_one_core};
use crate::screen::{Expected, PetView};
use crate::test_window::TestWindow;

/// The largest the placeholder can be: 24 x 18 sprite pixels at the default 2x scale.
const PET_WIDTH: i32 = 48;
const PET_HEIGHT: i32 = 36;

/// A few 8 ms polls, enough for the hit test to react to each step of a sweep.
const SWEEP_SETTLE: Duration = Duration::from_millis(30);

/// Faster than the 2 s re-check, so only the shell's message can have done it.
const BEFORE_THE_RE_CHECK: Duration = Duration::from_secs(1);

/// A test window behind where the Pet stands, with the app running in front of it.
struct Scene {
    _cursor: CursorGuard,
    pet: PetProcess,
    window: TestWindow,
    area: Rect,
}

fn scene() -> Scene {
    make_dpi_aware();
    let cursor = CursorGuard::new();
    let work = work_area();
    let corner = work.bottom_centre(400, 300);
    let area = Rect {
        left: corner.x,
        top: corner.y,
        right: corner.x + 400,
        bottom: corner.y + 300,
    };
    let window = TestWindow::open(area);
    // A real click puts it in front of whatever is open, and focus moves to it as it would to a real app.
    click(away_from_the_pet(area), &[window.hwnd()]);
    window.drop_to_normal();
    let pet = PetProcess::launch();
    assert!(
        wait_until(Duration::from_secs(10), || PetView::observe(area)
            .pet_pixels()
            > 0),
        "the Pet never appeared over the test window"
    );
    Scene {
        _cursor: cursor,
        pet,
        window,
        area,
    }
}

/// Opens a window over `area` and gives it the foreground with a real click, as a person would.
fn app_in_front(area: Rect) -> TestWindow {
    let window = TestWindow::open(area);
    click(away_from_the_pet(area), &[window.hwnd()]);
    window
}

fn away_from_the_pet(area: Rect) -> Point {
    Point {
        x: area.left + 10,
        y: area.top + 10,
    }
}

#[test]
fn given_a_window_behind_the_overlay_when_reading_the_screen_then_it_shows_through_everywhere_but_the_pet()
 {
    // Given
    let scene = scene();

    // When
    let view = PetView::observe(scene.area);

    // Then
    let bounds = view.bounds().expect("the Pet is on screen");
    assert!(
        view.background_share() > 0.9,
        "a black Overlay would hide the window behind it"
    );
    assert!(
        bounds.width() <= PET_WIDTH && bounds.height() <= PET_HEIGHT,
        "only the Pet is drawn: {bounds:?}"
    );
}

#[test]
fn given_the_app_started_when_reading_the_screen_then_the_pet_stands_at_the_bottom_centre_of_the_work_area()
 {
    // Given
    let scene = scene();
    let work = work_area();

    // When
    let bounds = PetView::observe(scene.area)
        .bounds()
        .expect("the Pet is on screen");

    // Then
    assert_eq!(bounds.bottom, work.bottom);
    let centre = (bounds.left + bounds.right) / 2;
    assert!(
        (centre - (work.left + work.right) / 2).abs() <= PET_WIDTH / 2,
        "centred: {bounds:?}"
    );
}

#[test]
fn given_a_window_behind_the_pet_when_clicking_a_transparent_pixel_inside_the_pet_then_the_window_gets_the_click()
 {
    // Given
    let scene = scene();
    let hole = PetView::observe(scene.area)
        .hole_point()
        .expect("the placeholder has a hole");

    let clicks_before = scene.window.clicks();

    // When
    click(hole, &[scene.window.hwnd(), scene.pet.overlay()]);

    // Then
    assert_eq!(scene.window.clicks(), clicks_before + 1);
}

#[test]
fn given_a_window_behind_the_pet_when_clicking_an_opaque_pixel_then_the_pet_catches_the_click() {
    // Given
    let scene = scene();
    let solid = PetView::observe(scene.area)
        .solid_point()
        .expect("the Pet has solid pixels");

    let clicks_before = scene.window.clicks();

    // When
    click(solid, &[scene.window.hwnd(), scene.pet.overlay()]);

    // Then
    assert_eq!(scene.window.clicks(), clicks_before);
}

#[test]
fn given_a_window_behind_the_pet_when_sweeping_the_cursor_across_it_then_only_opaque_pixels_catch_it()
 {
    // Given
    let scene = scene();
    let points = PetView::observe(scene.area).sweep();
    assert!(points.len() > 100, "too few points: {}", points.len());

    // When
    let misrouted: Vec<_> = points
        .iter()
        .filter(|&&(point, expected)| {
            let wanted = match expected {
                Expected::Overlay => scene.pet.overlay(),
                Expected::WindowBehind => scene.window.hwnd(),
            };
            window_under_cursor_at(point, SWEEP_SETTLE) != wanted
        })
        .collect();

    // Then
    assert!(
        misrouted.is_empty(),
        "{} of {} points misrouted, first few: {:?}",
        misrouted.len(),
        points.len(),
        &misrouted[..misrouted.len().min(10)]
    );
}

#[test]
fn given_another_window_has_focus_when_clicking_the_pet_then_that_window_keeps_it() {
    // Given
    let scene = scene();
    let allowed = [scene.window.hwnd(), scene.pet.overlay()];
    let solid = PetView::observe(scene.area)
        .solid_point()
        .expect("the Pet has solid pixels");
    click(away_from_the_pet(scene.area), &allowed);
    assert_eq!(
        foreground_window(),
        scene.window.hwnd(),
        "the test window took focus"
    );

    // When
    click(solid, &allowed);

    // Then
    assert_eq!(foreground_window(), scene.window.hwnd());
}

#[test]
fn given_the_tray_menu_is_open_when_the_app_closes_then_it_exits() {
    // Given: this opens a real menu and it takes the foreground, so it isn't a quiet test
    let pet = PetProcess::launch();
    let tray = pet.tray();
    tray.right_click();
    thread::sleep(Duration::from_millis(300));

    // When
    let status = pet.close().expect("wait for the app");

    // Then: a menu left open keeps its own message loop running, and would stop the app from ever exiting
    assert!(status.success(), "{status}");
}

#[test]
fn given_a_shown_pet_when_a_fullscreen_app_comes_to_the_front_then_it_hides_and_returns_when_the_app_closes()
 {
    // Given
    make_dpi_aware();
    let _cursor = CursorGuard::new();
    let pet = PetProcess::launch();

    // When
    let app = app_in_front(monitor_area());

    // Then
    assert!(pet.wait_until_hidden(), "the app filled the screen");

    // When
    drop(app);

    // Then
    assert!(pet.wait_until_shown(), "the app closed");
}

#[test]
fn given_a_window_in_front_when_it_grows_to_fill_the_monitor_then_the_pet_hides_with_no_change_of_foreground()
 {
    // Given
    make_dpi_aware();
    let _cursor = CursorGuard::new();
    let pet = PetProcess::launch();
    let small = Rect {
        left: 100,
        top: 100,
        right: 500,
        bottom: 400,
    };
    let app = app_in_front(small);

    // Several rounds, because one could land just before a re-check and pass with the shell's message broken
    for _ in 0..3 {
        // When
        app.move_to(monitor_area());

        // Then
        assert!(
            wait_until(BEFORE_THE_RE_CHECK, || !pet.is_shown()),
            "the window grew"
        );
        assert_eq!(foreground_window(), app.hwnd());

        // When
        app.move_to(small);

        // Then
        assert!(
            wait_until(BEFORE_THE_RE_CHECK, || pet.is_shown()),
            "the window shrank"
        );
    }
}

#[test]
fn given_a_fullscreen_app_in_front_when_launching_then_the_overlay_never_shows_until_the_app_closes()
 {
    // Given
    make_dpi_aware();
    let _cursor = CursorGuard::new();
    let app = app_in_front(monitor_area());

    // When
    let mut pet = PetProcess::launch_kept_hidden();
    drop(app);

    // Then
    assert!(pet.wait_for_overlay(), "the app closed");
}

#[test]
fn given_a_window_stopping_at_the_taskbar_when_it_is_in_front_then_the_pet_stays_shown() {
    // Given
    make_dpi_aware();
    let _cursor = CursorGuard::new();
    let pet = PetProcess::launch();

    // When: long enough for a re-check as well
    let _app = app_in_front(work_area());
    thread::sleep(Duration::from_secs(3));

    // Then
    assert!(pet.is_shown());
}

#[test]
fn given_an_auto_hide_when_measured_then_the_pet_uses_next_to_no_cpu_and_no_gpu() {
    // Given: the counter reads above zero while shown, so a zero below means something
    make_dpi_aware();
    let _cursor = CursorGuard::new();
    let pet = PetProcess::launch();
    let gpu = GpuUse::of(pet.child().id());
    thread::sleep(Duration::from_secs(2));
    let gpu_shown = gpu.percent();
    let _app = app_in_front(monitor_area());
    assert!(pet.wait_until_hidden());
    thread::sleep(Duration::from_secs(5));
    gpu.percent();

    // When: six 2 s re-checks fall inside this
    let cpu_hidden = cpu_percent_of_one_core(pet.child(), Duration::from_secs(12));
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
