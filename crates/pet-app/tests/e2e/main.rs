//! Tests against the real exe on the real desktop. They take over the mouse, so keep your hands off it.
//! Run with `cargo e2e`.
#![cfg(windows)]

mod harness;
mod screen;
mod test_window;

use std::thread;
use std::time::Duration;

use harness::{
    CursorGuard, PetProcess, click, foreground_window, make_dpi_aware, wait_until,
    window_under_cursor_at, work_area,
};
use pet_core::geometry::{Point, Rect};
use screen::{Expected, PetView};
use test_window::TestWindow;
use windows::Win32::Foundation::{HANDLE, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetPropW, GetWindowRect, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST,
};
use windows::core::w;

/// The largest the placeholder can be: 24 x 18 sprite pixels at the default 2x scale.
const PET_WIDTH: i32 = 48;
const PET_HEIGHT: i32 = 36;

/// A few 8 ms polls, enough for the hit test to react to each step of a sweep.
const SWEEP_SETTLE: Duration = Duration::from_millis(30);

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

fn away_from_the_pet(area: Rect) -> Point {
    Point {
        x: area.left + 10,
        y: area.top + 10,
    }
}

#[test]
fn given_the_app_started_when_inspecting_the_overlay_then_it_is_a_topmost_tool_window_that_never_activates()
 {
    // Given
    let scene = scene();

    // When: winit reset styles once at startup in the spike, so check after it has settled
    thread::sleep(Duration::from_secs(2));
    let style = scene.pet.ex_style();

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
    let scene = scene();
    let work = work_area();

    // When
    let mut rect = RECT::default();
    // SAFETY: `rect` is a live local that Windows writes during the call.
    unsafe { GetWindowRect(scene.pet.overlay(), &mut rect) }.expect("read the Overlay's rect");

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
    let scene = scene();

    // When
    // SAFETY: the name is a static string and the Overlay is alive for the whole test.
    let property: HANDLE = unsafe { GetPropW(scene.pet.overlay(), w!("NonRudeHWND")) };

    // Then
    assert!(!property.is_invalid() && !property.0.is_null());
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
