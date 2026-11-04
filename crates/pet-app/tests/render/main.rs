//! Windowless render tests: the real renderer draws the Pet into an image in memory, and every pixel is compared. They
//! show nothing, take no screenshots and send no input, so they run with the fast tests, at most 8 at once
//! (`.config/nextest.toml`). The window itself, its transparency and click-through stay with the real-exe tests.

mod fixture;
mod harness;

use std::path::Path;

use pet_core::placement::Facing;
use pet_sim::Faces;

use harness::{Offscreen, Picture};

/// The scale the Pet is drawn at (REN-01).
const SCALE: u32 = 2;

fn assert_drawn(got: &Picture, want: &Picture) {
    let differences = got.differences(want);
    assert!(
        differences.is_empty(),
        "{} pixels differ, the first: {:?}",
        differences.len(),
        &differences[..differences.len().min(10)]
    );
}

#[test]
fn given_the_placeholder_pack_when_drawn_then_every_pixel_is_its_first_frame_at_twice_its_size_at_the_bottom_centre()
 {
    // Given: a 64 x 40 work area, so the 48 x 36 Pet stands with its top-left at (8, 4)
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/placeholder");
    let pack = pet_content::load(&folder).unwrap_or_else(|errors| panic!("{errors:?}"));
    let definition = &pack.definition;
    let form = definition.starting_form;
    let first = &pack
        .pixels
        .frames(form, definition.form(form).default_animation())[0];
    let art = Picture {
        width: first.width,
        height: first.height,
        rgba: first.rgba.clone(),
    };
    let mut want = Picture::clear(64, 40);
    want.draw(&art, 8, 4, SCALE, false);
    let mut offscreen = Offscreen::start(folder, pack, 64, 40);

    // When
    let got = offscreen.capture();

    // Then
    assert_drawn(&got, &want);
}

#[test]
fn given_a_pack_with_an_off_centre_anchor_when_drawn_then_the_anchor_stands_on_the_bottom_centre() {
    // Given: in a 40 x 24 work area the Anchor goes to (20, 24). It's 1 x 2 from the art's left edge and 4 x 2 below
    // its top, so the top-left is (18, 16).
    let (folder, pack) = fixture::load("anchor");
    let mut want = Picture::clear(40, 24);
    want.draw(&fixture::frame(0), 18, 16, SCALE, false);
    let mut offscreen = Offscreen::start(folder, pack, 40, 24);

    // When
    let got = offscreen.capture();

    // Then
    assert_drawn(&got, &want);
}

#[test]
fn given_a_pet_facing_the_other_way_when_drawn_then_it_is_mirrored_around_its_anchor() {
    // Given: mirrored, the Anchor is 6 - 1 = 5 art pixels from the left edge, so the top-left is (20 - 10, 16)
    let (folder, pack) = fixture::load("mirrored");
    let mut want = Picture::clear(40, 24);
    want.draw(&fixture::frame(0), 10, 16, SCALE, true);
    let mut offscreen = Offscreen::start(folder, pack, 40, 24);

    // When
    offscreen.with_pet(|mut pet| {
        pet.insert(Faces(Facing::Mirrored));
    });
    let got = offscreen.capture();

    // Then
    assert_drawn(&got, &want);
}

#[test]
fn given_a_running_pet_when_the_simulations_clock_passes_each_frames_duration_then_that_frame_is_drawn_and_it_loops()
 {
    // Given: Frames of 100 ms and 200 ms, and ticks of 33.3 ms
    let (folder, pack) = fixture::load("clock");
    let mut offscreen = Offscreen::start(folder, pack, 40, 24);
    let frame = |index| {
        let mut picture = Picture::clear(40, 24);
        picture.draw(&fixture::frame(index), 18, 16, SCALE, false);
        picture
    };
    let (first, second) = (frame(0), frame(1));

    // When: 4 ticks is 133 ms, into the second Frame, and 6 more is 333 ms, 33 ms into the second loop
    offscreen.tick(4);
    let after_the_first = offscreen.capture();
    offscreen.tick(6);
    let after_a_loop = offscreen.capture();

    // Then
    assert_drawn(&after_the_first, &second);
    assert_drawn(&after_a_loop, &first);
}
