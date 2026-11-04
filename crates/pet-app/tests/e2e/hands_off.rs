//! Tests that send real input, with the mouse or the keyboard, or that read the screen, fill it and take the foreground.
//! The owner starts them with `cargo e2e` and keeps their hands off while they run.

use std::thread;
use std::time::Duration;

use pet_core::geometry::{Point, Rect};

use crate::harness::{
    CursorGuard, Packs, PetProcess, bottom_centre, click, foreground_window, make_dpi_aware,
    monitor_area, placeholder_opaque_colours, press_hotkey, wait_until, window_under_cursor_at,
    work_area,
};
use crate::metrics::{GpuUse, MAX_HIDDEN_CPU_PERCENT, cpu_percent_of_one_core};
use crate::screen::{Expected, PetView, colour_at, colours_in};
use crate::test_window::{BACKGROUND, TestWindow};

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
    scene_with(Packs::Placeholder)
}

fn scene_with(packs: Packs) -> Scene {
    make_dpi_aware();
    let cursor = CursorGuard::new();
    let work = work_area();
    let corner = bottom_centre(work, 400, 300);
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
    let pet = PetProcess::launch_with(packs);
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
    let misrouted = misrouted(&scene, &points);

    // Then
    assert!(
        misrouted.is_empty(),
        "{} of {} points misrouted, first few: {:?}",
        misrouted.len(),
        points.len(),
        &misrouted[..misrouted.len().min(10)]
    );
}

/// The points of a sweep where the cursor lands on the wrong window.
fn misrouted(scene: &Scene, points: &[(Point, Expected)]) -> Vec<(Point, Expected)> {
    points
        .iter()
        .copied()
        .filter(|&(point, expected)| {
            let wanted = match expected {
                Expected::Overlay => scene.pet.overlay(),
                Expected::WindowBehind => scene.window.hwnd(),
            };
            window_under_cursor_at(point, SWEEP_SETTLE) != wanted
        })
        .collect()
}

#[test]
fn given_a_pack_whose_anchor_is_off_centre_when_sweeping_the_cursor_across_the_pet_then_it_stands_by_its_anchor_and_only_opaque_pixels_catch_it()
 {
    // Given: the Anchor 8 art pixels left of the cell's middle puts the art 16 pixels right of the placeholder's spot.
    // The placeholder's opaque pixels are centred to within a few pixels.
    let scene = scene_with(Packs::OffCentreAnchor);
    let work = work_area();
    let view = PetView::observe(scene.area);
    let bounds = view.bounds().expect("the Pet is on screen");
    let points = view.sweep();
    assert!(points.len() > 100, "too few points: {}", points.len());

    // When
    let misrouted = misrouted(&scene, &points);

    // Then: drawn and hit-tested in the same place, which is the Anchor's
    let right_of_centre = (bounds.left + bounds.right) / 2 - (work.left + work.right) / 2;
    assert!(
        (8..=24).contains(&right_of_centre),
        "{right_of_centre} pixels right of centre: {bounds:?}"
    );
    assert_eq!(bounds.bottom, work.bottom);
    assert!(
        misrouted.is_empty(),
        "{} of {} points misrouted, first few: {:?}",
        misrouted.len(),
        points.len(),
        &misrouted[..misrouted.len().min(10)]
    );
}

/// A point on `Packs::WithShadow`'s shadow but off the Pet: 27 pixels left of the Anchor, past the Pet's 24, and in the
/// shadow's middle rows, which are its full 60 pixels wide.
fn on_the_shadow_beside_the_pet() -> Point {
    let work = work_area();
    Point {
        x: work.left + work.width() / 2 - 27,
        y: work.bottom - 8,
    }
}

#[test]
fn given_a_pack_with_a_shadow_when_clicking_the_shadow_beside_the_pet_then_the_window_behind_gets_the_click()
 {
    // Given
    let scene = scene_with(Packs::WithShadow);
    let point = on_the_shadow_beside_the_pet();
    assert_ne!(colour_at(point), BACKGROUND, "the shadow is drawn there");
    let clicks_before = scene.window.clicks();

    // When
    click(point, &[scene.window.hwnd(), scene.pet.overlay()]);

    // Then
    assert_eq!(scene.window.clicks(), clicks_before + 1);
}

#[test]
fn given_a_pack_with_a_shadow_when_reading_the_screen_then_the_pets_own_pixels_and_its_shadow_are_both_there()
 {
    // Given: the shadow covers only the bottom 16 rows, so a placeholder colour higher up is the Pet's own art
    let scene = scene_with(Packs::WithShadow);
    let work = work_area();
    let pet_colours = placeholder_opaque_colours();
    let above_the_shadow = Rect {
        bottom: work.bottom - 16,
        ..scene.area
    };

    // When
    let above = colours_in(above_the_shadow);
    let beside = colour_at(on_the_shadow_beside_the_pet());

    // Then
    assert!(
        above.iter().any(|colour| pet_colours.contains(colour)),
        "none of the Pet's own opaque pixels is on screen"
    );
    assert_ne!(beside, BACKGROUND, "the shadow isn't on screen");
    assert!(
        !pet_colours.contains(&beside),
        "beside the Pet should be shadow, not the Pet's art: {beside:?}"
    );
}

#[test]
fn given_a_pack_with_a_shadow_when_reading_the_screen_beside_the_pet_then_the_shadow_is_the_packs_colour_over_the_window()
 {
    // Given: #1a132660 over the window's magenta, as an image editor shows it: c x a + behind x (1 - a), with
    // a = 96 / 255. The engine blends in sRGB and leaves the pixel premultiplied, (10, 7, 14, 96), and the window's
    // compositor adds the rest. Blending in linear light would have shown (172, 8, 180).
    let _scene = scene_with(Packs::WithShadow);
    let want = [169, 7, 173];

    // When
    let got = colour_at(on_the_shadow_beside_the_pet());

    // Then: within 2 for rounding
    assert!(
        got.iter()
            .zip(want)
            .all(|(&got, want)| got.abs_diff(want) <= 2),
        "got {got:?}, want {want:?}"
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
fn given_a_broken_pack_when_launching_then_nothing_is_drawn_over_the_window_behind() {
    // Given: the same window behind as `scene`, where a good Pack's Pet does appear
    make_dpi_aware();
    let _cursor = CursorGuard::new();
    let corner = bottom_centre(work_area(), 400, 300);
    let area = Rect {
        left: corner.x,
        top: corner.y,
        right: corner.x + 400,
        bottom: corner.y + 300,
    };
    let window = TestWindow::open(area);
    click(away_from_the_pet(area), &[window.hwnd()]);
    window.drop_to_normal();

    // When
    let pet = PetProcess::launch_with(Packs::Broken);
    thread::sleep(Duration::from_secs(2));

    // Then
    assert!(pet.is_shown(), "the empty Overlay is up");
    assert_eq!(PetView::observe(area).pet_pixels(), 0);
}
