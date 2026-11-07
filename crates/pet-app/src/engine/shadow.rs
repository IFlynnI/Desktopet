//! Draws the drop shadow under the Pet (REN-03), behind it, on the Ground plane below its Anchor. Its width and colour
//! come from the Form the simulation shows, and a Form with no shadow has none. The shadow is never part of the Hit
//! target. It only reads the simulation, and nothing flows back.

use bevy::prelude::*;
use pet_core::geometry::Point;
use pet_core::pack::Shadow;
use pet_core::{placement, shadow};
use pet_sim::{CurrentPack, Pet, Position, ShownFrame, WorkArea};

use super::overlay::{OverlayRect, sprite_translation};
use super::pack_assets::pixel_art;
use super::pixel_art::{self as drawing, UnitQuad};
use super::renderer::SCALE;

/// The Pet is drawn at 0, so this is behind it.
const BEHIND_THE_PET: f32 = -1.0;

pub struct ShadowPlugin;

impl Plugin for ShadowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                // Again whenever an edited Pack replaces it, which may have other Forms.
                make_images.run_if(resource_exists_and_changed::<CurrentPack>),
                draw.run_if(resource_exists::<ShadowImages>),
            )
                .chain(),
        );
    }
}

/// The shadow's own entity, which belongs to the Pet it names. It isn't on the Pet's entity, because it doesn't move
/// with the Pet's art: it stays on the ground.
#[derive(Component, Debug)]
#[relationship(relationship_target = Shadowed)]
pub struct ShadowOf(pub Entity);

/// The Pet's shadow. Despawning the Pet despawns it.
#[derive(Component, Debug)]
#[relationship_target(relationship = ShadowOf, linked_spawn)]
pub struct Shadowed(Entity);

/// Each Form's shadow and the material it's drawn with, numbered as in the Definition. A Form with no shadow has
/// none.
#[derive(Resource)]
struct ShadowImages(Vec<Option<(Shadow, Handle<ColorMaterial>)>>);

/// Makes the image and material of each Form's shadow, once per Pack, from the Definition alone.
fn make_images(
    mut commands: Commands,
    pack: Res<CurrentPack>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let shadows = pack
        .0
        .forms
        .iter()
        .map(|form| {
            form.shadow.map(|shadow| {
                let art = shadow::art(shadow);
                let image = images.add(pixel_art(art.width, art.height, art.rgba));
                (shadow, drawing::material(image, &mut materials))
            })
        })
        .collect();
    commands.insert_resource(ShadowImages(shadows));
}

/// What the simulation says about a Pet that matters to its shadow, and the shadow it has, if any.
type ShadowedPet = (
    Entity,
    Ref<'static, ShownFrame>,
    Ref<'static, Position>,
    Option<&'static Shadowed>,
);

/// How a shadow is drawn.
type ShadowDrawable = (
    &'static mut MeshMaterial2d<ColorMaterial>,
    &'static mut Transform,
    &'static mut Visibility,
);

/// Draws the shadow of a Pet whose Form or position has changed, or when the Ground plane has. It gives the Pet a
/// shadow the first time its Form has one, and hides it while its Form has none. Nothing is written when nothing
/// differs, so a new Frame of the same Form doesn't touch the shadow.
fn draw(
    mut commands: Commands,
    images: Res<ShadowImages>,
    work_area: Res<WorkArea>,
    overlay: Res<OverlayRect>,
    quad: Res<UnitQuad>,
    pets: Query<ShadowedPet, With<Pet>>,
    mut shadows: Query<ShadowDrawable, (With<ShadowOf>, Without<Pet>)>,
) {
    for (pet, shown, position, shadowed) in &pets {
        let changed = shown.is_changed()
            || position.is_changed()
            || work_area.is_changed()
            || overlay.is_changed();
        if !changed {
            continue;
        }
        let drawn = shadowed.and_then(|shadowed| shadows.get_mut(shadowed.0).ok());
        let Some((shadow, material)) = images.0[shown.form.0].clone() else {
            if let Some((_, _, mut visibility)) = drawn {
                visibility.set_if_neq(Visibility::Hidden);
            }
            continue;
        };
        let rect = shadow::place(position.0.x, placement::ground(work_area.0), shadow, SCALE);
        let (width, height) = (rect.width(), rect.height());
        let top_left = Point {
            x: rect.left,
            y: rect.top,
        };
        let translation =
            sprite_translation(overlay.0, top_left, width, height).with_z(BEHIND_THE_PET);
        let transform = drawing::transform(translation, width, height, false);
        match drawn {
            Some((mut drawn_material, mut drawn_transform, mut visibility)) => {
                if drawn_material.0 != material {
                    drawn_material.0 = material;
                }
                drawn_transform.set_if_neq(transform);
                visibility.set_if_neq(Visibility::Inherited);
            }
            None => {
                commands.spawn((ShadowOf(pet), drawing::drawable(&quad, material, transform)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;
    use std::{env, fs, process};

    use bevy::time::TimeUpdateStrategy;
    use pet_core::geometry::Rect;
    use pet_core::pack::{AnimationId, FormId};

    use super::*;
    use crate::engine::pack_assets::{PackAtStartup, add_starting_pack};
    use crate::engine::pixel_art::PixelArtPlugin;
    use crate::pack::StartingPack;

    const WORK_AREA: Rect = Rect {
        left: 0,
        top: 0,
        right: 1000,
        bottom: 800,
    };

    /// A folder of its own for a test's Pack, removed afterwards.
    struct TestPack(std::path::PathBuf);

    impl Drop for TestPack {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// The placeholder's art in two Forms: `hatchling` with a shadow 16 wide, and `bare` with none.
    fn shadowed_and_bare(test: &str) -> App {
        let folder = env::temp_dir().join(format!("desktop-pet-shadow-{}-{test}", process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(folder.join("hatchling")).unwrap();
        let placeholder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/placeholder");
        fs::copy(
            placeholder.join("hatchling/idle.png"),
            folder.join("hatchling/idle.png"),
        )
        .unwrap();
        let form = |name: &str, shadow: &str| {
            format!(
                r#"
[forms.{name}]
cell = {{ width = 24, height = 18 }}
anchor = {{ x = 12, y = 18 }}
body = {{ x = -9, y = -15, width = 18, height = 15 }}
default_action = "idle"
{shadow}

[forms.{name}.actions.idle]
animation = "idle"

[forms.{name}.animations.idle]
sheet = "hatchling/idle.png"
durations_ms = [250, 250]
"#
            )
        };
        let manifest = format!(
            "format_version = 1
name = \"Shadowed and bare\"
starting_form = \"hatchling\"
{}{}",
            form(
                "hatchling",
                r##"shadow = { width = 16, colour = "#22203480" }"##
            ),
            form("bare", "")
        );
        fs::write(folder.join("manifest.toml"), manifest).unwrap();
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
                ShadowPlugin,
            ));
        app
    }

    fn form_named(app: &App, name: &str) -> FormId {
        let pack = app.world().resource::<CurrentPack>();
        FormId(
            pack.0
                .forms
                .iter()
                .position(|form| form.name == name)
                .unwrap(),
        )
    }

    fn show(app: &mut App, form: FormId, frame: usize) {
        let world = app.world_mut();
        let mut shown = world
            .query_filtered::<&mut ShownFrame, With<Pet>>()
            .single_mut(world)
            .expect("one Pet");
        *shown = ShownFrame {
            form,
            animation: AnimationId(0),
            frame,
        };
    }

    /// The shadow's place, its size as its scale, and its visibility.
    fn shadow(app: &mut App) -> (Transform, Visibility) {
        let world = app.world_mut();
        let (transform, visibility) = world
            .query_filtered::<(&Transform, &Visibility), With<ShadowOf>>()
            .single(world)
            .expect("one shadow");
        (*transform, *visibility)
    }

    #[test]
    fn given_a_drawn_shadow_when_the_pet_changes_to_a_form_without_one_then_it_hides_and_returns_with_the_form()
     {
        // Given: at home, (500, 800), the 16 x 4 shadow is 32 x 8 on screen, behind the Pet
        let mut app = shadowed_and_bare("form-change");
        app.update();
        let (hatchling, bare) = (form_named(&app, "hatchling"), form_named(&app, "bare"));
        let drawn = (
            Transform::from_xyz(0.0, -396.0, -1.0).with_scale(Vec3::new(32.0, 8.0, 1.0)),
            Visibility::Inherited,
        );
        assert_eq!(shadow(&mut app), drawn);

        // When
        show(&mut app, bare, 0);
        app.update();
        let without = shadow(&mut app);
        show(&mut app, hatchling, 0);
        app.update();
        let back = shadow(&mut app);

        // Then
        assert_eq!(without.1, Visibility::Hidden);
        assert_eq!(back, drawn);
    }

    /// How many updates changed the shadow's material, place or visibility.
    #[derive(Resource, Default)]
    struct ShadowWrites(usize);

    /// A shadow whose material, place or visibility changed.
    type Written = (
        With<ShadowOf>,
        Or<(
            Changed<MeshMaterial2d<ColorMaterial>>,
            Changed<Transform>,
            Changed<Visibility>,
        )>,
    );

    fn count_writes(mut writes: ResMut<ShadowWrites>, shadows: Query<(), Written>) {
        if !shadows.is_empty() {
            writes.0 += 1;
        }
    }

    #[test]
    fn given_a_drawn_shadow_when_the_pet_shows_its_next_frame_then_the_shadow_is_left_untouched() {
        // Given: the first update draws the shadow
        let mut app = shadowed_and_bare("next-frame");
        app.init_resource::<ShadowWrites>()
            .add_systems(Last, count_writes);
        app.update();
        let hatchling = form_named(&app, "hatchling");

        // When
        show(&mut app, hatchling, 1);
        app.update();

        // Then: only drawing it changed it, so nothing goes to the GPU again for a new Frame
        assert_eq!(app.world().resource::<ShadowWrites>().0, 1);
    }
}
