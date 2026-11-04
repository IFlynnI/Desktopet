//! Draws the Pet the simulation shows: the Frame its clock picked, at an integer scale, with its Anchor on the
//! simulation's position, mirrored when it faces the other way. It also tells the hit-test thread which Frame is
//! showing and where. It only reads the simulation, and nothing flows back. With no Pack there's no Pet to draw.

use bevy::prelude::*;
use pet_core::placement;
use pet_platform::HitTarget;
use pet_sim::{CurrentPack, Faces, Pet, Position, ShownFrame};

use super::overlay::{OverlayRect, PlatformHandle, sprite_translation};
use super::pack_assets::PackAssets;

/// The default integer scale (REN-01).
const SCALE: u32 = 2;

pub struct RendererPlugin;

impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (draw, publish_hit_target::<PlatformHandle>).run_if(resource_exists::<PackAssets>),
        );
    }
}

/// Whatever hit-tests the Pet: in the app, the platform's hit-test thread.
pub trait HitTest: Resource {
    fn publish_hit_target(&self, target: HitTarget);
}

impl HitTest for PlatformHandle {
    fn publish_hit_target(&self, target: HitTarget) {
        self.0.publish_hit_target(target);
    }
}

/// A Pet whose Frame, position or facing has changed since the system last ran.
type ChangedPet = (
    With<Pet>,
    Or<(Changed<ShownFrame>, Changed<Position>, Changed<Faces>)>,
);

/// What the simulation says is drawn, and where.
type Shown = (
    Ref<'static, ShownFrame>,
    Ref<'static, Position>,
    Ref<'static, Faces>,
);

/// Its sprite, once it has one.
type Drawn = (Option<&'static mut Sprite>, Option<&'static mut Transform>);

/// Draws a Pet whose Frame, position or facing has changed, giving it a sprite the first time.
fn draw(
    mut commands: Commands,
    pack: Res<CurrentPack>,
    assets: Res<PackAssets>,
    overlay: Res<OverlayRect>,
    mut pets: Query<(Entity, Shown, Drawn), ChangedPet>,
) {
    for (pet, (shown, position, faces), (sprite, transform)) in &mut pets {
        let form = pack.0.form(shown.form);
        let placement = placement::place(position.0, form, SCALE, faces.0);
        let (width, height) = (
            (form.cell.width * SCALE) as i32,
            (form.cell.height * SCALE) as i32,
        );
        let size = Vec2::new(width as f32, height as f32);
        let image = assets.frames(shown.form, shown.animation)[shown.frame]
            .image
            .clone();
        let translation = sprite_translation(overlay.0, placement.top_left, width, height);
        match (sprite, transform) {
            (Some(mut sprite), Some(mut transform)) => {
                sprite.image = image;
                sprite.custom_size = Some(size);
                sprite.flip_x = placement.flip_x;
                transform.translation = translation;
            }
            _ => {
                commands.entity(pet).insert((
                    Sprite {
                        image,
                        custom_size: Some(size),
                        flip_x: placement.flip_x,
                        ..default()
                    },
                    Transform::from_translation(translation),
                ));
            }
        }
    }
}

/// What the hit-test thread needs for the Frame the simulation shows, where it's drawn.
fn hit_target(
    pack: &CurrentPack,
    assets: &PackAssets,
    shown: &ShownFrame,
    position: &Position,
    faces: &Faces,
) -> HitTarget {
    let form = pack.0.form(shown.form);
    HitTarget {
        mask: assets.frames(shown.form, shown.animation)[shown.frame]
            .hit_mask
            .clone(),
        placement: placement::place(position.0, form, SCALE, faces.0),
    }
}

/// Publishes when the Frame, position or facing changes, and once when the platform arrives. The platform comes in
/// the first update, after the Pet has appeared, so without that a Pet whose Frame never changes would never publish.
fn publish_hit_target<Platform: HitTest>(
    pack: Res<CurrentPack>,
    assets: Res<PackAssets>,
    pets: Query<Shown, With<Pet>>,
    platform: Option<Res<Platform>>,
) {
    let Some(platform) = platform else { return };
    for (shown, position, faces) in &pets {
        let changed = shown.is_changed() || position.is_changed() || faces.is_changed();
        if changed || platform.is_added() {
            platform.publish_hit_target(hit_target(&pack, &assets, &shown, &position, &faces));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use std::{env, fs, io, process};

    use bevy::ecs::component::Mutable;
    use bevy::ecs::schedule::SingleThreadedExecutor;
    use bevy::log::tracing;
    use bevy::log::tracing_subscriber::fmt;
    use bevy::time::TimeUpdateStrategy;
    use pet_content::{LoadedPack, PackError};
    use pet_core::geometry::{Point, Rect};
    use pet_core::hit_mask::{HitMask, Placement};
    use pet_core::pack::{AnimationId, FormId};
    use pet_core::placement::Facing;
    use pet_sim::SimPlugin;

    use super::*;
    use crate::engine::pack_assets::PackAtStartup;
    use crate::pack::StartingPack;

    /// Everything logged while it's in use, as text.
    #[derive(Clone, Default)]
    struct CapturedLog(Arc<Mutex<Vec<u8>>>);

    impl io::Write for CapturedLog {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl CapturedLog {
        /// Runs `run` with everything it logs on this thread captured.
        fn during(&self, run: impl FnOnce()) {
            let writer = self.clone();
            let subscriber = fmt()
                .with_ansi(false)
                .with_writer(move || writer.clone())
                .finish();
            tracing::subscriber::with_default(subscriber, run);
        }

        fn text(&self) -> String {
            String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
        }
    }

    const WORK_AREA: Rect = Rect {
        left: 0,
        top: 0,
        right: 1000,
        bottom: 800,
    };

    fn startup_with(starting: StartingPack) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Image>>()
            .insert_resource(OverlayRect(WORK_AREA))
            .insert_resource(PackAtStartup(starting))
            .add_systems(Startup, crate::engine::pack_assets::add_starting_pack)
            .add_plugins((
                SimPlugin {
                    work_area: WORK_AREA,
                },
                RendererPlugin,
            ));
        app
    }

    fn pet_sprites(app: &mut App) -> Vec<(Sprite, Transform)> {
        let world = app.world_mut();
        let mut query = world.query_filtered::<(&Sprite, &Transform), With<Pet>>();
        query
            .iter(world)
            .map(|(sprite, transform)| (sprite.clone(), *transform))
            .collect()
    }

    fn placeholder() -> (std::path::PathBuf, LoadedPack) {
        let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/placeholder");
        let pack = pet_content::load(&folder).unwrap_or_else(|errors| panic!("{errors:?}"));
        (folder, pack)
    }

    #[test]
    fn given_the_simulation_shows_a_frame_when_building_the_hit_target_then_it_is_that_frames_mask_placed_by_the_anchor()
     {
        // Given: the placeholder's second Frame, mirrored, with its Anchor at (500, 800)
        let (_, pack) = placeholder();
        let mut images = Assets::<Image>::default();
        let assets = PackAssets::new(&pack, &mut images);
        let current = CurrentPack(Arc::new(pack.definition.clone()));
        let form = pack.definition.starting_form;
        let animation = pack.definition.form(form).default_animation();
        let shown = ShownFrame {
            form,
            animation,
            frame: 1,
        };

        // When
        let target = hit_target(
            &current,
            &assets,
            &shown,
            &Position(Point { x: 500, y: 800 }),
            &Faces(Facing::Mirrored),
        );

        // Then: the Anchor is (12, 18) in a 24 x 18 cell, so 12 x 2 from either side and 18 x 2 up
        assert_eq!(
            *target.mask,
            pack.pixels.frames(form, animation)[1].hit_mask
        );
        assert_eq!(
            target.placement,
            Placement {
                top_left: Point { x: 476, y: 764 },
                scale: 2,
                flip_x: true
            }
        );
    }

    #[test]
    fn given_the_placeholder_pack_when_the_engine_starts_then_one_pet_stands_at_the_bottom_centre_at_twice_its_size()
     {
        // Given
        let (folder, pack) = placeholder();
        let mut app = startup_with(StartingPack::Loaded { folder, pack });

        // When
        app.update();

        // Then: 48 x 36 on screen, centred across the work area, with its centre 18 up from the bottom
        let sprites = pet_sprites(&mut app);
        assert_eq!(sprites.len(), 1);
        let (sprite, transform) = &sprites[0];
        assert_eq!(sprite.custom_size, Some(Vec2::new(48.0, 36.0)));
        assert_eq!(transform.translation, Vec3::new(0.0, -382.0, 0.0));
    }

    #[test]
    fn given_a_broken_pack_when_the_engine_starts_then_there_is_no_pet_and_every_error_is_logged() {
        // Given: Startup runs on this thread, where the log is captured
        let errors = vec![
            PackError::new(
                "manifest.toml",
                "starting_form",
                "\"egg\" isn't one of the Forms",
            ),
            PackError::new(
                "hatchling/idle.png",
                "forms.hatchling.animations.idle.sheet",
                "is missing",
            ),
        ];
        let mut app = startup_with(StartingPack::Broken {
            folder: "packs/broken".into(),
            errors,
        });
        app.edit_schedule(Startup, |startup| {
            startup.set_executor(SingleThreadedExecutor::new());
        });
        let log = CapturedLog::default();

        // When
        log.during(|| app.update());

        // Then
        assert!(pet_sprites(&mut app).is_empty());
        let logged = log.text();
        let errors: Vec<&str> = logged
            .lines()
            .filter(|line| line.contains("ERROR"))
            .collect();
        assert_eq!(errors.len(), 3, "{logged}");
        assert!(errors[0].contains("packs/broken"), "{logged}");
        assert!(
            errors[1].contains("manifest.toml: starting_form"),
            "{logged}"
        );
        assert!(
            errors[2].contains("hatchling/idle.png: forms.hatchling"),
            "{logged}"
        );
    }

    #[test]
    fn given_no_pack_when_the_engine_starts_then_there_is_no_pet() {
        // Given
        let mut app = startup_with(StartingPack::NoPack { packs_folder: None });

        // When
        app.update();
        app.update();

        // Then
        assert!(pet_sprites(&mut app).is_empty());
    }

    /// Stands in for the platform, keeping every Hit target it's given.
    #[derive(Resource, Default)]
    struct RecordingPlatform(Mutex<Vec<HitTarget>>);

    impl HitTest for RecordingPlatform {
        fn publish_hit_target(&self, target: HitTarget) {
            self.0.lock().unwrap().push(target);
        }
    }

    impl RecordingPlatform {
        fn published(app: &App) -> Vec<(HitMask, Placement)> {
            app.world()
                .resource::<RecordingPlatform>()
                .0
                .lock()
                .unwrap()
                .iter()
                .map(|target| ((*target.mask).clone(), target.placement))
                .collect()
        }
    }

    /// The placeholder Pet with the recording platform's publisher, and time standing still.
    fn publishing() -> (App, LoadedPack) {
        let (folder, pack) = placeholder();
        let (_, kept) = placeholder();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
            .init_resource::<Assets<Image>>()
            .insert_resource(PackAtStartup(StartingPack::Loaded { folder, pack }))
            .add_systems(Startup, crate::engine::pack_assets::add_starting_pack)
            .add_plugins(SimPlugin {
                work_area: WORK_AREA,
            })
            .add_systems(
                Update,
                publish_hit_target::<RecordingPlatform>.run_if(resource_exists::<PackAssets>),
            );
        (app, kept)
    }

    fn change_pet<T: Component<Mutability = Mutable>>(app: &mut App, change: impl FnOnce(&mut T)) {
        let world = app.world_mut();
        let mut pet = world
            .query_filtered::<&mut T, With<Pet>>()
            .single_mut(world)
            .expect("one Pet");
        change(&mut pet);
    }

    fn default_frames(pack: &LoadedPack) -> &[pet_content::FramePixels] {
        let form = pack.definition.starting_form;
        pack.pixels
            .frames(form, pack.definition.form(form).default_animation())
    }

    /// Where the placeholder goes with its Anchor at (`x`, 800): its Anchor is (12, 18) in a 24 x 18 cell, at 2x.
    fn placed(x: i32, flip_x: bool) -> Placement {
        Placement {
            top_left: Point { x: x - 24, y: 764 },
            scale: 2,
            flip_x,
        }
    }

    #[test]
    fn given_a_published_pet_when_its_frame_then_position_then_facing_change_then_each_change_is_published_once()
     {
        // Given: the platform is there from the start, so the first Frame is published at once
        let (mut app, pack) = publishing();
        app.insert_resource(RecordingPlatform::default());
        app.update();
        app.update();

        // When: an update with no change after each
        change_pet::<ShownFrame>(&mut app, |shown| shown.frame = 1);
        app.update();
        app.update();
        change_pet::<Position>(&mut app, |position| position.0 = Point { x: 100, y: 800 });
        app.update();
        app.update();
        change_pet::<Faces>(&mut app, |faces| faces.0 = Facing::Mirrored);
        app.update();
        app.update();

        // Then
        let [first, second] = default_frames(&pack) else {
            panic!("the placeholder has two Frames");
        };
        assert_eq!(
            RecordingPlatform::published(&app),
            [
                (first.hit_mask.clone(), placed(500, false)),
                (second.hit_mask.clone(), placed(500, false)),
                (second.hit_mask.clone(), placed(100, false)),
                (second.hit_mask.clone(), placed(100, true)),
            ]
        );
    }

    #[test]
    fn given_a_pet_drawn_before_the_platform_arrives_when_it_arrives_then_the_hit_target_is_published_once()
     {
        // Given: by now the Pet's own changes are old news
        let (mut app, pack) = publishing();
        for _ in 0..3 {
            app.update();
        }

        // When
        app.insert_resource(RecordingPlatform::default());
        app.update();
        app.update();

        // Then
        let first = &default_frames(&pack)[0];
        assert_eq!(
            RecordingPlatform::published(&app),
            [(first.hit_mask.clone(), placed(500, false))]
        );
    }

    /// A folder of its own for a test's Pack, removed afterwards.
    struct TestPack(PathBuf);

    impl Drop for TestPack {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// The placeholder's Form, and a second Form twice as wide that shows both its Frames side by side as one.
    fn two_sizes(test: &str) -> (TestPack, LoadedPack) {
        let folder = env::temp_dir().join(format!("desktop-pet-renderer-{}-{test}", process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(folder.join("hatchling")).unwrap();
        let (placeholder_folder, _) = placeholder();
        fs::copy(
            placeholder_folder.join("hatchling/idle.png"),
            folder.join("hatchling/idle.png"),
        )
        .unwrap();
        let form = |name: &str, width: u32, durations: &str| {
            let anchor = width / 2;
            format!(
                r#"
[forms.{name}]
cell = {{ width = {width}, height = 18 }}
anchor = {{ x = {anchor}, y = 18 }}
body = {{ x = -9, y = -15, width = 18, height = 15 }}
default_action = "idle"

[forms.{name}.actions.idle]
animation = "idle"

[forms.{name}.animations.idle]
sheet = "hatchling/idle.png"
durations_ms = {durations}
"#
            )
        };
        let manifest = format!(
            "format_version = 1\nname = \"Two sizes\"\nstarting_form = \"hatchling\"\n{}{}",
            form("hatchling", 24, "[250, 250]"),
            form("wide", 48, "[250]")
        );
        fs::write(folder.join("manifest.toml"), manifest).unwrap();
        let pack = pet_content::load(&folder).unwrap_or_else(|errors| panic!("{errors:?}"));
        (TestPack(folder), pack)
    }

    #[test]
    fn given_a_drawn_pet_when_it_changes_to_a_form_with_a_bigger_cell_then_its_sprite_takes_that_size()
     {
        // Given
        let (folder, pack) = two_sizes("sizes");
        let wide = pack
            .definition
            .forms
            .iter()
            .position(|form| form.name == "wide")
            .map(FormId)
            .unwrap();
        let mut app = startup_with(StartingPack::Loaded {
            folder: folder.0.clone(),
            pack,
        });
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
        app.update();

        // When
        change_pet::<ShownFrame>(&mut app, |shown| {
            *shown = ShownFrame {
                form: wide,
                animation: AnimationId(0),
                frame: 0,
            }
        });
        app.update();

        // Then: 48 x 18 art at 2x
        let sprites = pet_sprites(&mut app);
        assert_eq!(sprites[0].0.custom_size, Some(Vec2::new(96.0, 36.0)));
    }
}
