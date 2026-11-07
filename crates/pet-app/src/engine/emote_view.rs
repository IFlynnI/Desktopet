//! Draws the Emote the simulation shows above the Pet: centred on the right edge of its Body, a small gap above the top
//! of it, in front of it, and never mirrored, because an Emote reads like text. It follows the Pet's place and Form.
//! An Emote is never part of the Hit target. It only reads the simulation, and nothing flows back.

use bevy::prelude::*;
use pet_core::emote;
use pet_core::geometry::Point;
use pet_sim::{CurrentPack, Pet, Position, ShownEmote, ShownFrame};

use super::overlay::{OverlayRect, sprite_translation};
use super::pack_assets::PackAssets;
use super::pixel_art::{self, UnitQuad};
use super::renderer::SCALE;

/// The Pet is drawn at 0, so this is in front of it, where its art reaches above its Body.
const IN_FRONT_OF_THE_PET: f32 = 1.0;

pub struct EmoteViewPlugin;

impl Plugin for EmoteViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw.run_if(resource_exists::<PackAssets>));
    }
}

/// The Emote's own entity, which belongs to the Pet it names. It isn't on the Pet's entity, because the Pet's
/// transform is scaled to its Frame and mirrored with it.
#[derive(Component, Debug)]
#[relationship(relationship_target = Emoting)]
pub struct EmoteOf(pub Entity);

/// The Pet's Emote. Despawning the Pet despawns it.
#[derive(Component, Debug)]
#[relationship_target(relationship = EmoteOf, linked_spawn)]
pub struct Emoting(Entity);

/// What the simulation says about a Pet that matters to its Emote, and the Emote it has, if any. Which way it faces
/// doesn't matter.
type EmotingPet = (
    Entity,
    Ref<'static, ShownEmote>,
    Ref<'static, ShownFrame>,
    Ref<'static, Position>,
    Option<&'static Emoting>,
);

/// How an Emote is drawn.
type EmoteDrawable = (
    &'static mut MeshMaterial2d<ColorMaterial>,
    &'static mut Transform,
    &'static mut Visibility,
);

/// Draws the Emote of a Pet whose Emote, Form or position has changed, giving it an Emote entity the first time one
/// shows, and hiding that while none does. Nothing is written when nothing differs, so a new Frame of the Pet leaves
/// its Emote untouched. Drawn the `pixel_art` way, not as a `Sprite`.
fn draw(
    mut commands: Commands,
    pack: Res<CurrentPack>,
    assets: Res<PackAssets>,
    overlay: Res<OverlayRect>,
    quad: Res<UnitQuad>,
    pets: Query<EmotingPet, With<Pet>>,
    mut emotes: Query<EmoteDrawable, (With<EmoteOf>, Without<Pet>)>,
) {
    for (pet, shown_emote, shown, position, emoting) in &pets {
        let changed = shown_emote.is_changed()
            || shown.is_changed()
            || position.is_changed()
            || overlay.is_changed();
        if !changed {
            continue;
        }
        let drawn = emoting.and_then(|emoting| emotes.get_mut(emoting.0).ok());
        // The simulation shows only Emotes the Pack has.
        let Some(showing) = shown_emote.0 else {
            if let Some((_, _, mut visibility)) = drawn {
                visibility.set_if_neq(Visibility::Hidden);
            }
            continue;
        };
        // In roadmap item 3 the Pet always plays its Form's default Action, so that Action's Body is the one it has.
        let form = pack.0.form(shown.form);
        let body = form.body_of(form.default_action);
        let rect = emote::place(position.0, body, pack.0.emote(showing.emote).cell, SCALE);
        let (width, height) = (rect.width(), rect.height());
        let top_left = Point {
            x: rect.left,
            y: rect.top,
        };
        let translation =
            sprite_translation(overlay.0, top_left, width, height).with_z(IN_FRONT_OF_THE_PET);
        let transform = pixel_art::transform(translation, width, height, false);
        let material = assets.emote_frames(showing.emote)[showing.frame].clone();
        match drawn {
            Some((mut drawn_material, mut drawn_transform, mut visibility)) => {
                if drawn_material.0 != material {
                    drawn_material.0 = material;
                }
                drawn_transform.set_if_neq(transform);
                visibility.set_if_neq(Visibility::Inherited);
            }
            None => {
                commands.spawn((
                    EmoteOf(pet),
                    pixel_art::drawable(&quad, material, transform),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::Duration;
    use std::{env, fs, process};

    use bevy::ecs::component::Mutable;
    use bevy::time::TimeUpdateStrategy;
    use pet_core::geometry::{Point, Rect};
    use pet_core::pack::EmoteId;
    use pet_core::placement::Facing;
    use pet_sim::{EmoteFrame, Faces, Pet, Position, ShownEmote, ShownFrame};

    use super::*;
    use crate::engine::overlay::OverlayRect;
    use crate::engine::pack_assets::{PackAssets, PackAtStartup, add_starting_pack};
    use crate::engine::pixel_art::PixelArtPlugin;
    use crate::pack::StartingPack;

    const WORK_AREA: Rect = Rect {
        left: 0,
        top: 0,
        right: 1000,
        bottom: 800,
    };

    /// "wave", the Pack's second Emote, so picking the first shows up.
    const WAVE: EmoteId = EmoteId(1);

    /// A folder of its own for a test's Pack, removed afterwards.
    struct TestPack(PathBuf);

    impl Drop for TestPack {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn placeholder() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/placeholder")
    }

    /// The placeholder Pet, whose Body's top is 15 Pack pixels above its Anchor, with two Emotes.
    fn emoting(test: &str) -> (App, TestPack) {
        let manifest = fs::read_to_string(placeholder().join("manifest.toml")).unwrap();
        emoting_with(test, manifest)
    }

    /// The Pet `manifest` describes, with two Emotes added that borrow the placeholder's idle strip as their sheet.
    /// "wave" is 24 x 18. The manifest may use the strip too, as `hatchling/idle.png`.
    fn emoting_with(test: &str, manifest: String) -> (App, TestPack) {
        let folder = env::temp_dir().join(format!("desktop-pet-emote-{}-{test}", process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(folder.join("hatchling")).unwrap();
        fs::copy(
            placeholder().join("hatchling/idle.png"),
            folder.join("hatchling/idle.png"),
        )
        .unwrap();
        let emotes = r#"
[emotes.spare]
sheet = "hatchling/idle.png"
cell = { width = 48, height = 18 }
durations_ms = [500]
mode = "looping"

[emotes.wave]
sheet = "hatchling/idle.png"
cell = { width = 24, height = 18 }
durations_ms = [250, 250]
mode = "one-shot"
"#;
        fs::write(folder.join("manifest.toml"), manifest + emotes).unwrap();
        let pack = pet_content::load(&folder).unwrap_or_else(|errors| panic!("{errors:?}"));
        let folder = TestPack(folder);
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<ColorMaterial>>()
            .init_resource::<Assets<Mesh>>()
            .insert_resource(OverlayRect(WORK_AREA))
            .insert_resource(PackAtStartup(StartingPack::Loaded {
                folder: folder.0.clone(),
                pack,
            }))
            .add_systems(Startup, add_starting_pack)
            .add_plugins((
                crate::engine::test_support::sim_plugin(WORK_AREA),
                PixelArtPlugin,
                EmoteViewPlugin,
            ));
        (app, folder)
    }

    /// Changes something about the Pet, as the simulation would. Time stands still, so no tick runs to undo it.
    fn change_pet<T: Component<Mutability = Mutable>>(app: &mut App, change: impl FnOnce(&mut T)) {
        let world = app.world_mut();
        let mut pet = world
            .query_filtered::<&mut T, With<Pet>>()
            .single_mut(world)
            .expect("one Pet");
        change(&mut pet);
    }

    fn show(app: &mut App, emote: EmoteId, frame: usize) {
        change_pet::<ShownEmote>(app, |shown| {
            *shown = ShownEmote(Some(EmoteFrame { emote, frame }));
        });
    }

    /// The Emote's material, its place with its size as its scale, and its visibility.
    fn emote(app: &mut App) -> (Handle<ColorMaterial>, Transform, Visibility) {
        let world = app.world_mut();
        let (material, transform, visibility) = world
            .query_filtered::<(&MeshMaterial2d<ColorMaterial>, &Transform, &Visibility), With<EmoteOf>>()
            .single(world)
            .expect("one Emote");
        (material.0.clone(), *transform, *visibility)
    }

    fn emote_material(app: &App, emote: EmoteId, frame: usize) -> Handle<ColorMaterial> {
        app.world().resource::<PackAssets>().emote_frames(emote)[frame].clone()
    }

    #[test]
    fn given_a_mirrored_pet_away_from_home_when_it_shows_an_emote_then_it_is_drawn_above_its_body_in_front_and_unmirrored()
     {
        // Given: the Anchor at (100, 700), so the Body's right edge is at 100 + 9 x 2 and its top at 700 - 15 x 2
        let (mut app, _folder) = emoting("place");
        app.update();
        change_pet::<Position>(&mut app, |position| position.0 = Point { x: 100, y: 700 });
        change_pet::<Faces>(&mut app, |faces| faces.0 = Facing::Mirrored);

        // When
        show(&mut app, WAVE, 1);
        app.update();

        // Then: 48 x 36 on screen, from x = 94 to 142 and y = 630 to 666, 2 x 2 above the Body. Its centre is
        // (118, 648), and the Overlay's is (500, 400).
        assert_eq!(
            emote(&mut app),
            (
                emote_material(&app, WAVE, 1),
                Transform::from_xyz(-382.0, -248.0, 1.0).with_scale(Vec3::new(48.0, 36.0, 1.0)),
                Visibility::Inherited
            )
        );
    }

    #[test]
    fn given_a_drawn_emote_when_it_goes_then_it_hides_and_returns_when_shown_again() {
        // Given: at home, (500, 800)
        let (mut app, _folder) = emoting("hide");
        app.update();
        show(&mut app, WAVE, 0);
        app.update();
        let drawn = emote(&mut app);

        // When
        change_pet::<ShownEmote>(&mut app, |shown| shown.0 = None);
        app.update();
        let gone = emote(&mut app);
        show(&mut app, WAVE, 0);
        app.update();

        // Then
        assert_eq!(gone.2, Visibility::Hidden);
        assert_eq!(emote(&mut app), drawn);
        assert_eq!(drawn.1.translation, Vec3::new(18.0, -348.0, 1.0));
    }

    /// How many updates changed the Emote's material, place or visibility.
    #[derive(Resource, Default)]
    struct EmoteWrites(usize);

    /// An Emote whose material, place or visibility changed.
    type Written = (
        With<EmoteOf>,
        Or<(
            Changed<MeshMaterial2d<ColorMaterial>>,
            Changed<Transform>,
            Changed<Visibility>,
        )>,
    );

    fn count_writes(mut writes: ResMut<EmoteWrites>, emotes: Query<(), Written>) {
        if !emotes.is_empty() {
            writes.0 += 1;
        }
    }

    #[test]
    fn given_a_drawn_emote_when_the_pet_shows_its_next_frame_then_the_emote_is_left_untouched() {
        // Given: the second update draws the Emote
        let (mut app, _folder) = emoting("next-frame");
        app.init_resource::<EmoteWrites>()
            .add_systems(Last, count_writes);
        app.update();
        show(&mut app, WAVE, 0);
        app.update();

        // When
        change_pet::<ShownFrame>(&mut app, |shown| shown.frame = 1);
        app.update();
        app.update();

        // Then: only drawing it changed it, so nothing goes to the GPU again for the Pet's new Frame
        assert_eq!(app.world().resource::<EmoteWrites>().0, 1);
    }

    /// The placeholder's art in two Forms: `standing`, whose Body's top is 15 Pack pixels above the Anchor, then
    /// `curled`, whose Body's top is 9 above it.
    fn standing_and_curled() -> String {
        let form = |name: &str, body: &str| {
            format!(
                r#"
[forms.{name}]
cell = {{ width = 24, height = 18 }}
anchor = {{ x = 12, y = 18 }}
body = {body}
default_action = "idle"

[forms.{name}.actions.idle]
animation = "idle"

[forms.{name}.animations.idle]
sheet = "hatchling/idle.png"
durations_ms = [250, 250]
"#
            )
        };
        format!(
            "format_version = 1
name = \"Standing and curled\"
starting_form = \"standing\"
{}{}",
            form("standing", "{ x = -9, y = -15, width = 18, height = 15 }"),
            form("curled", "{ x = -10, y = -9, width = 20, height = 9 }"),
        )
    }

    #[test]
    fn given_a_drawn_emote_when_the_pet_changes_to_a_form_with_a_lower_body_then_the_emote_moves_down_with_it()
     {
        // Given: at home, (500, 800), standing, the Emote's centre is 18 right of the Overlay's and 348 below it
        let (mut app, _folder) = emoting_with("form-change", standing_and_curled());
        app.update();
        show(&mut app, WAVE, 0);
        app.update();
        let curled = {
            let pack = app.world().resource::<CurrentPack>();
            let position = pack.0.forms.iter().position(|form| form.name == "curled");
            pet_core::pack::FormId(position.expect("the Pack has the Form"))
        };

        // When
        change_pet::<ShownFrame>(&mut app, |shown| shown.form = curled);
        app.update();

        // Then: the Body's top is 15 - 9 = 6 Pack pixels lower, so the Emote is 6 x 2 lower. Its right edge is 1 Pack
        // pixel further right, 10 rather than 9 right of the Anchor, so the Emote is 10 x 2 right of the Overlay's
        // centre rather than 9 x 2.
        assert_eq!(emote(&mut app).1.translation, Vec3::new(20.0, -360.0, 1.0));
    }

    /// The placeholder's art in one Form whose Body's top is 15 Pack pixels above the Anchor. Its default Action is
    /// `sleep`, which curls up into a Body of its own, whose top is 9 above it and whose right edge is 10 right of it.
    fn asleep_by_default() -> String {
        r#"
format_version = 1
name = "Asleep by default"
starting_form = "hatchling"

[forms.hatchling]
cell = { width = 24, height = 18 }
anchor = { x = 12, y = 18 }
body = { x = -9, y = -15, width = 18, height = 15 }
default_action = "sleep"

[forms.hatchling.actions.idle]
animation = "idle"

[forms.hatchling.actions.sleep]
animation = "idle"
body = { x = -10, y = -9, width = 20, height = 9 }

[forms.hatchling.animations.idle]
sheet = "hatchling/idle.png"
durations_ms = [250, 250]
"#
        .to_owned()
    }

    #[test]
    fn given_a_pet_whose_default_action_has_a_body_of_its_own_when_it_shows_an_emote_then_the_emote_sits_on_that_body()
     {
        // Given: at home, (500, 800)
        let (mut app, _folder) = emoting_with("action-body", asleep_by_default());
        app.update();

        // When
        show(&mut app, WAVE, 0);
        app.update();

        // Then: centred on x = 500 + 10 x 2, so 24 x 2 wide its left is 496, and its bottom 2 x 2 above the Body's top
        // at 800 - 9 x 2. Its centre is (520, 760), and the Overlay's is (500, 400).
        assert_eq!(emote(&mut app).1.translation, Vec3::new(20.0, -360.0, 1.0));
    }
}
