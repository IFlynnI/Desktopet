//! Draws the Pet: the starting Form's default Action on a loop, each Frame for its own duration, standing at the
//! bottom centre of the work area. It also tells the hit-test thread which Frame is showing. With no Pack it draws
//! nothing. Roadmap item 3's slice 2 moves the choice of Frame into the simulation.

use std::sync::Arc;
use std::time::Duration;

use bevy::prelude::*;
use pet_core::geometry::Rect;
use pet_core::hit_mask::{HitMask, Placement};
use pet_platform::HitTarget;

use super::overlay::{OverlayRect, PlatformHandle, sprite_translation};
use super::pack_assets::{self, PackAssets};

/// The default integer scale (REN-01).
const SCALE: u32 = 2;

/// Which Frame is showing, and for how much longer.
#[derive(Resource)]
struct Animation {
    frame: usize,
    /// One per Frame.
    durations: Vec<Duration>,
    timer: Timer,
}

impl Animation {
    fn new(durations: Vec<Duration>) -> Self {
        let timer = Timer::new(durations[0], TimerMode::Repeating);
        Self {
            frame: 0,
            durations,
            timer,
        }
    }
}

/// The Frames of the Animation being shown.
#[derive(Resource)]
struct ShownFrames {
    images: Vec<Handle<Image>>,
    masks: Vec<Arc<HitMask>>,
    placement: Placement,
}

#[derive(Component)]
struct PetSprite;

/// Shows the Pet standing at the bottom centre of `work_area`.
pub struct RendererPlugin {
    pub work_area: Rect,
}

impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        let work_area = self.work_area;
        let spawn_pet = move |commands: Commands,
                              assets: Option<Res<PackAssets>>,
                              overlay: Res<OverlayRect>| {
            spawn(commands, assets, *overlay, work_area)
        };
        app.add_systems(Startup, spawn_pet.after(pack_assets::add_starting_pack))
            .add_systems(
                Update,
                (animate, show_frame, publish_hit_target)
                    .chain()
                    .run_if(resource_exists::<Animation>),
            );
    }
}

fn spawn(
    mut commands: Commands,
    assets: Option<Res<PackAssets>>,
    overlay: OverlayRect,
    work_area: Rect,
) {
    // No Pack loaded, so there's no Pet. The reasons are in the log.
    let Some(assets) = assets else { return };
    let definition = &assets.definition;
    let form = definition.form(definition.starting_form);
    let frames = assets.frames(definition.starting_form, form.default_animation());
    let animation = form.animation(form.default_animation());

    let (width, height) = (
        (form.cell.width * SCALE) as i32,
        (form.cell.height * SCALE) as i32,
    );
    let top_left = work_area.bottom_centre(width, height);
    commands.spawn((
        PetSprite,
        Sprite {
            image: frames[0].image.clone(),
            custom_size: Some(Vec2::new(width as f32, height as f32)),
            ..default()
        },
        Transform::from_translation(sprite_translation(overlay.0, top_left, width, height)),
    ));
    commands.insert_resource(Animation::new(animation.durations.clone()));
    commands.insert_resource(ShownFrames {
        images: frames.iter().map(|frame| frame.image.clone()).collect(),
        masks: frames.iter().map(|frame| frame.hit_mask.clone()).collect(),
        placement: Placement {
            top_left,
            scale: SCALE,
            flip_x: false,
        },
    });
}

fn animate(time: Res<Time>, mut animation: ResMut<Animation>) {
    animation.timer.tick(time.delta());
    if animation.timer.just_finished() {
        animation.frame = (animation.frame + 1) % animation.durations.len();
        let duration = animation.durations[animation.frame];
        animation.timer.set_duration(duration);
    }
}

fn show_frame(
    animation: Res<Animation>,
    frames: Res<ShownFrames>,
    mut sprites: Query<&mut Sprite, With<PetSprite>>,
) {
    let Ok(mut sprite) = sprites.single_mut() else {
        return;
    };
    let image = &frames.images[animation.frame];
    if sprite.image != *image {
        sprite.image = image.clone();
    }
}

fn publish_hit_target(
    animation: Res<Animation>,
    frames: Res<ShownFrames>,
    platform: Option<Res<PlatformHandle>>,
    mut published: Local<Option<usize>>,
) {
    let Some(platform) = platform else { return };
    if *published != Some(animation.frame) {
        platform.0.publish_hit_target(HitTarget {
            mask: frames.masks[animation.frame].clone(),
            placement: frames.placement,
        });
        *published = Some(animation.frame);
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::Path;
    use std::sync::Mutex;

    use bevy::ecs::schedule::SingleThreadedExecutor;
    use bevy::log::tracing;
    use bevy::log::tracing_subscriber::fmt;
    use bevy::time::TimeUpdateStrategy;
    use pet_content::PackError;

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

    fn animation_app(step: Duration, durations: Vec<Duration>) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(step))
            .insert_resource(Animation::new(durations))
            .add_systems(Update, animate);
        app
    }

    fn frame(app: &App) -> usize {
        app.world().resource::<Animation>().frame
    }

    fn startup_with(starting: StartingPack) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Image>>()
            .insert_resource(OverlayRect(WORK_AREA))
            .insert_resource(PackAtStartup(starting))
            .add_systems(Startup, pack_assets::add_starting_pack)
            .add_plugins(RendererPlugin {
                work_area: WORK_AREA,
            });
        app
    }

    fn pet_sprites(app: &mut App) -> Vec<(Sprite, Transform)> {
        let world = app.world_mut();
        let mut query = world.query_filtered::<(&Sprite, &Transform), With<PetSprite>>();
        query
            .iter(world)
            .map(|(sprite, transform)| (sprite.clone(), *transform))
            .collect()
    }

    #[test]
    fn given_a_running_animation_when_a_frame_duration_passes_then_the_next_frame_shows_and_it_wraps_around()
     {
        // Given
        let quarter_second = Duration::from_millis(250);
        let mut app = animation_app(Duration::from_millis(130), vec![quarter_second; 2]);
        app.update();
        let start = frame(&app);

        // When: 130 ms per update crosses the 250 ms frame duration every second update
        app.update();
        app.update();
        let next = frame(&app);
        app.update();
        app.update();
        let wrapped = frame(&app);

        // Then
        assert_eq!(next, (start + 1) % 2);
        assert_eq!(wrapped, start);
    }

    #[test]
    fn given_frames_of_different_durations_when_time_passes_then_each_shows_for_its_own_duration() {
        // Given: 100 ms then 300 ms, so one loop is four 100 ms updates
        let durations = vec![Duration::from_millis(100), Duration::from_millis(300)];
        let mut app = animation_app(Duration::from_millis(100), durations);
        app.update();

        // When
        let shown: Vec<usize> = (0..8)
            .map(|_| {
                app.update();
                frame(&app)
            })
            .collect();

        // Then
        assert_eq!(
            shown.iter().filter(|&&frame| frame == 0).count(),
            2,
            "{shown:?}"
        );
        assert_eq!(
            shown.iter().filter(|&&frame| frame == 1).count(),
            6,
            "{shown:?}"
        );
    }

    #[test]
    fn given_the_placeholder_pack_when_the_engine_starts_then_one_pet_stands_at_the_bottom_centre_at_twice_its_size()
     {
        // Given
        let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/placeholder");
        let pack = pet_content::load(&folder).unwrap_or_else(|errors| panic!("{errors:?}"));
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
        assert!(!app.world().contains_resource::<Animation>());
    }
}
