//! Windowless render tests: the real renderer draws the Pet into an image in memory, and every pixel is compared. They
//! show nothing, take no screenshots and send no input, so they run with the fast tests, at most 8 at once
//! (`.config/nextest.toml`). The window itself, its transparency and click-through stay with the real-exe tests.

mod fixture;
mod harness;

use std::path::{Path, PathBuf};

use bevy::prelude::With;
use pet_app::PackEdited;
use pet_content::LoadedPack;
use pet_core::geometry::Point;
use pet_core::pack::EmoteId;
use pet_core::placement::Facing;
use pet_sim::{EmoteRequest, Faces, Pet, Position, ShownFrame};

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

#[test]
fn given_a_pet_in_the_air_off_centre_and_mirrored_when_drawn_then_its_shadow_is_centred_under_its_anchor_on_the_ground()
 {
    // Given: the Anchor at (14, 10) in a 40 x 24 work area. Mirrored, the Pet's top-left is (14 - 5 x 2, 10 - 4 x 2).
    // The shadow is 11 x 3 at 2x: 5 texels left of the Anchor, so its left is 14 - 10, and it ends on the ground at 24.
    let (folder, pack) = fixture::load_with_shadow("shadow-off-centre");
    let mut want = Picture::clear(40, 24);
    want.draw(&fixture::shadow(), 4, 18, SCALE, false);
    want.draw(&fixture::frame(0), 4, 2, SCALE, true);
    let mut offscreen = Offscreen::start(folder, pack, 40, 24);

    // When
    offscreen.with_pet(|mut pet| {
        pet.insert((Position(Point { x: 14, y: 10 }), Faces(Facing::Mirrored)));
    });
    let got = offscreen.capture();

    // Then: a shadow pixel holds the Pack's colour premultiplied by its alpha
    assert_eq!(got.pixel(6, 18), fixture::SHADOW_DRAWN);
    assert_drawn(&got, &want);
}

#[test]
fn given_a_form_without_a_shadow_when_drawn_in_the_air_then_only_the_pet_is_drawn() {
    // Given: the Anchor at (14, 10), so the Pet's top-left is (14 - 1 x 2, 10 - 4 x 2), and the ground below is clear
    let (folder, pack) = fixture::load("no-shadow");
    let mut want = Picture::clear(40, 24);
    want.draw(&fixture::frame(0), 12, 2, SCALE, false);
    let mut offscreen = Offscreen::start(folder, pack, 40, 24);

    // When
    offscreen.with_pet(|mut pet| {
        pet.insert(Position(Point { x: 14, y: 10 }));
    });
    let got = offscreen.capture();

    // Then
    assert_drawn(&got, &want);
}

#[test]
fn given_a_pet_standing_on_its_shadow_when_drawn_then_the_pet_covers_the_shadow_where_they_overlap()
{
    // Given: at home the Anchor is (20, 24), so the Pet's top-left is (18, 16) and the shadow's is (20 - 10, 24 - 6)
    let (folder, pack) = fixture::load_with_shadow("shadow-behind");
    let mut want = Picture::clear(40, 24);
    want.draw(&fixture::shadow(), 10, 18, SCALE, false);
    want.draw(&fixture::frame(0), 18, 16, SCALE, false);
    let mut offscreen = Offscreen::start(folder, pack, 40, 24);

    // When
    let got = offscreen.capture();

    // Then: on the Pet's bottom row, "a....a", its foot hides the shadow and the gap between its feet shows it
    assert_eq!(got.pixel(18, 22), [240, 140, 40, 255]);
    assert_eq!(got.pixel(20, 22), fixture::SHADOW_DRAWN);
    assert_drawn(&got, &want);
}

#[test]
fn given_two_pets_each_with_a_shadow_when_drawn_then_every_one_is_drawn_in_its_own_place() {
    // Given: in an 80 x 24 work area the first Pet stands at home, (40, 24), on Frame 0, so its top-left is (38, 16) and
    // its shadow's is (30, 18). A second Pet on Frame 1 is in the air at (66, 8), so its top-left is (64, 0) and its
    // shadow's (56, 18). That's three different images in four places, which takes more than one draw call. Bevy's
    // `Sprite` loses every draw call after the first on DX12, so this guards against going back to it.
    let (folder, pack) = fixture::load_with_shadow("several");
    let mut want = Picture::clear(80, 24);
    want.draw(&fixture::shadow(), 30, 18, SCALE, false);
    want.draw(&fixture::shadow(), 56, 18, SCALE, false);
    want.draw(&fixture::frame(0), 38, 16, SCALE, false);
    want.draw(&fixture::frame(1), 64, 0, SCALE, false);
    let mut offscreen = Offscreen::start(folder, pack, 80, 24);
    let world = offscreen.app.world_mut();
    let shown = *world
        .query_filtered::<&ShownFrame, With<Pet>>()
        .single(world)
        .expect("one Pet");

    // When
    world.spawn((
        Pet,
        Position(Point { x: 66, y: 8 }),
        Faces::default(),
        ShownFrame { frame: 1, ..shown },
    ));
    let got = offscreen.capture();

    // Then
    assert_drawn(&got, &want);
}

/// Asks the simulation for `emote`, as the Mind will in roadmap item 4.
fn show_emote(offscreen: &mut Offscreen, emote: EmoteId) {
    offscreen
        .app
        .world_mut()
        .write_message(EmoteRequest::Show(emote));
}

#[test]
fn given_a_pet_showing_an_emote_when_it_moves_and_turns_round_then_the_emote_follows_above_its_body_unmirrored()
 {
    // Given: the tick that reads the request shows the Emote's first Frame
    let (folder, pack) = fixture::load("emote-follows");
    let note = fixture::emote_id(&pack, "note");
    let mut offscreen = Offscreen::start(folder, pack, 40, 24);
    show_emote(&mut offscreen, note);
    offscreen.tick(1);

    // When: drawn at home, then moved to (22, 24) and mirrored
    let at_home = offscreen.capture();
    offscreen.with_pet(|mut pet| {
        pet.insert((Position(Point { x: 22, y: 24 }), Faces(Facing::Mirrored)));
    });
    let moved = offscreen.capture();

    // Then: at home the Anchor is (20, 24), so the Pet's top-left is (18, 16), which is also its Body's top. The
    // Body's right edge is 5 x 2 right of the Anchor, at x = 30. The 5 x 3 Emote at 2x is 2 texels left of that edge
    // and ends 2 x 2 above the Body, so its top-left is (30 - 2 x 2, 16 - 4 - 6).
    let mut want_at_home = Picture::clear(40, 24);
    want_at_home.draw(&fixture::frame(0), 18, 16, SCALE, false);
    want_at_home.draw(&fixture::emote("note", 0), 26, 6, SCALE, false);
    assert_drawn(&at_home, &want_at_home);
    // Its half-transparent corner, top right, holds its colour premultiplied by its alpha
    assert_eq!(at_home.pixel(34, 6), fixture::EMOTE_HALF_DRAWN);
    // Moved and mirrored, the Pet's top-left is (22 - 5 x 2, 16). The Body isn't mirrored, so its right edge is still
    // 5 x 2 right of the Anchor, and the Emote's top-left is (22 + 5 x 2 - 2 x 2, 6).
    let mut want_moved = Picture::clear(40, 24);
    want_moved.draw(&fixture::frame(0), 12, 16, SCALE, true);
    want_moved.draw(&fixture::emote("note", 0), 28, 6, SCALE, false);
    assert_drawn(&moved, &want_moved);
}

#[test]
fn given_a_one_shot_emote_when_its_last_frame_has_had_its_time_then_it_is_no_longer_drawn() {
    // Given: "pop" is 50 ms then 50 ms. At home its top-left is (26, 6), as in the test above.
    let (folder, pack) = fixture::load("emote-one-shot");
    let pop = fixture::emote_id(&pack, "pop");
    let mut offscreen = Offscreen::start(folder, pack, 40, 24);
    show_emote(&mut offscreen, pop);

    // When: the tick that reads the request shows its first Frame with no time played, so 3 ticks is 67 ms played, into
    // its second Frame. One more is exactly 100 ms, its end.
    offscreen.tick(3);
    let playing = offscreen.capture();
    offscreen.tick(1);
    let ended = offscreen.capture();

    // Then: the Pet has played 3 ticks, then 4, exactly 100 ms and more, so it's on its second Frame each time
    let mut on_its_second_frame = Picture::clear(40, 24);
    on_its_second_frame.draw(&fixture::frame(1), 18, 16, SCALE, false);
    on_its_second_frame.draw(&fixture::emote("pop", 1), 26, 6, SCALE, false);
    let mut gone = Picture::clear(40, 24);
    gone.draw(&fixture::frame(1), 18, 16, SCALE, false);
    assert_drawn(&playing, &on_its_second_frame);
    assert_drawn(&ended, &gone);
}

#[test]
fn given_a_crouched_pet_whose_art_reaches_above_its_body_when_it_shows_an_emote_then_the_emote_covers_its_art()
 {
    // Given: crouched, the Body's top is 1 x 2 above the Anchor at (20, 24) and its right edge 2 x 2 right of it. So
    // the Emote's top-left is (24 - 2 x 2, 22 - 4 - 6), and its bottom row lands on the Pet's top row, y = 16 and 17.
    let (folder, pack) = fixture::load_crouched("emote-in-front");
    let note = fixture::emote_id(&pack, "note");
    let mut want = Picture::clear(40, 24);
    want.draw(&fixture::frame(0), 18, 16, SCALE, false);
    want.draw(&fixture::emote("note", 0), 20, 12, SCALE, false);
    let mut offscreen = Offscreen::start(folder, pack, 40, 24);
    show_emote(&mut offscreen, note);

    // When
    offscreen.tick(1);
    let got = offscreen.capture();

    // Then: where the Emote's "cc..." meets the Pet's ".aa...", the Emote's colour shows
    assert_eq!(got.pixel(20, 16), [60, 200, 120, 255]);
    assert_drawn(&got, &want);
}

/// Hands the engine an edited Pack, as the watcher does once a save has settled.
fn reload(offscreen: &mut Offscreen, folder: PathBuf, edited: LoadedPack) {
    offscreen.app.world_mut().write_message(PackEdited {
        folder,
        loaded: Ok(edited),
    });
}

/// `art` drawn at 2x with its top-left at (`left`, `top`) in a clear 40 x 24 work area.
fn drawn_at(art: &Picture, left: u32, top: u32) -> Picture {
    let mut picture = Picture::clear(40, 24);
    picture.draw(art, left, top, SCALE, false);
    picture
}

#[test]
fn given_a_drawn_pet_when_its_pack_is_edited_with_new_art_then_every_pixel_is_the_new_art() {
    // Given: on its first Frame at home, with its top-left at (18, 16). The edit swaps the art's two colours and
    // changes nothing else, so the Pet is on the same Frame of the same Animation before and after.
    let (folder, pack) = fixture::load("reload");
    let (_, edited) = fixture::load_recoloured("reload-edited");
    let mut offscreen = Offscreen::start(folder.clone(), pack, 40, 24);
    let before = offscreen.capture();

    // When
    reload(&mut offscreen, folder, edited);
    let after = offscreen.capture();

    // Then
    let mut want_before = Picture::clear(40, 24);
    want_before.draw(&fixture::frame(0), 18, 16, SCALE, false);
    let mut want_after = Picture::clear(40, 24);
    want_after.draw(&fixture::recoloured_frame(0), 18, 16, SCALE, false);
    assert_drawn(&before, &want_before);
    assert_drawn(&after, &want_after);
}

#[test]
fn given_a_drawn_pet_when_an_edit_changes_only_its_forms_default_action_then_that_actions_animation_is_drawn_with_its_own_timing()
 {
    // Given: "idle", 100 ms then 200 ms, is the default Action, and "nap", 50 ms then 150 ms, has art of its own. At
    // home the top-left is (18, 16).
    let (folder, pack) = fixture::load_with_nap("default-action");
    let (_, edited) = fixture::load_napping("default-action-edited");
    let mut offscreen = Offscreen::start(folder.clone(), pack, 40, 24);
    let before = offscreen.capture();

    // When: the clock starts again at the reload. 1 tick later is 33 ms, still nap's first Frame. 2 are 67 ms, into its
    // second, where idle would still be on its first. 6 are exactly 200 ms, the start of its second loop, where idle
    // would be on its second Frame.
    reload(&mut offscreen, folder, edited);
    let on_reloading = offscreen.capture();
    offscreen.tick(1);
    let at_33_ms = offscreen.capture();
    offscreen.tick(1);
    let at_67_ms = offscreen.capture();
    offscreen.tick(4);
    let at_200_ms = offscreen.capture();

    // Then
    assert_drawn(&before, &drawn_at(&fixture::frame(0), 18, 16));
    assert_drawn(&on_reloading, &drawn_at(&fixture::nap_frame(0), 18, 16));
    assert_drawn(&at_33_ms, &drawn_at(&fixture::nap_frame(0), 18, 16));
    assert_drawn(&at_67_ms, &drawn_at(&fixture::nap_frame(1), 18, 16));
    assert_drawn(&at_200_ms, &drawn_at(&fixture::nap_frame(0), 18, 16));
}

#[test]
fn given_a_pet_still_in_its_starting_form_when_an_edit_changes_the_starting_form_then_the_new_starting_forms_art_is_drawn()
 {
    // Given: an "egg", 4 x 3, with its Anchor in the middle of its bottom edge. At home, (20, 24), its top-left is
    // (20 - 2 x 2, 24 - 3 x 2).
    let (folder, pack) = fixture::load_as_egg("starting-form");
    let (_, edited) = fixture::load_with_egg("starting-form-edited");
    let mut offscreen = Offscreen::start(folder.clone(), pack, 40, 24);
    let before = offscreen.capture();

    // When: the edit changes only `starting_form`, to "hatchling"
    reload(&mut offscreen, folder, edited);
    let after = offscreen.capture();

    // Then: a "hatchling" on the first Frame of its default Action, with its top-left at (18, 16)
    assert_drawn(&before, &drawn_at(&fixture::egg(), 16, 18));
    assert_drawn(&after, &drawn_at(&fixture::frame(0), 18, 16));
}
