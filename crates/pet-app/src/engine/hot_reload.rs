//! Hot reload: an edited Pack replaces the one in use while the app runs, so a Pack's author sees each save at once.
//!
//! The platform watches the Pack's folder, and once a burst of changes has settled, the Pack is read again on the
//! watcher's thread, so the engine never waits on files or PNG decoding. What was read reaches the engine as a
//! `PackEdited` through a channel, with a wake-up in case the engine is stopped, the way the shell's events do. A
//! valid Pack replaces the one in use, in one exclusive system before the ticks and the renderer run, so nothing reads
//! the new Pack with the old one's numbers. An invalid one changes nothing, and every error goes to the log.
//!
//! The watching starts before the Pack is first read, so no edit can fall between reading it and watching it. The
//! engine's event loop doesn't exist yet then, so the wake-up is bound to it in Startup. Whatever the watcher read
//! before that waits in the channel, and the first update takes it.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bevy::prelude::*;
use bevy::winit::{EventLoopProxyWrapper, WinitUserEvent};
use pet_content::{LoadedPack, PackError};
use pet_core::settle::Settle;
use pet_platform::{FolderWatch, Wake};
use pet_sim::CurrentPack;

use super::log_file;
use super::pack_assets::PackAssets;

/// How long a Pack's files must be left alone before it's read again. An editor's save is a burst of changes a few
/// milliseconds apart, and a quarter of a second is well past that but still feels immediate.
const SETTLE: Duration = Duration::from_millis(250);

/// How long a burst of changes may hold the reading back, from its first change, for a tool that never stops writing.
const SETTLE_AT_MOST: Duration = Duration::from_secs(2);

/// A Pack whose files changed, read again: the Pack, or every error in it.
#[derive(Message)]
pub struct PackEdited {
    pub folder: PathBuf,
    pub loaded: Result<LoadedPack, Vec<PackError>>,
}

/// What the watcher has read and the engine hasn't yet. It's in a `Mutex` only because Bevy needs resources that
/// threads can share, and a `Receiver` on its own can't be.
#[derive(Resource)]
pub(super) struct PackEdits(pub(super) Mutex<Receiver<PackEdited>>);

/// Keeps the watching going for as long as the app runs.
#[derive(Resource)]
struct PackWatch {
    _watch: FolderWatch,
}

/// A wake-up for the engine that can be bound after the watching has started. Until then, waking does nothing, which
/// is fine: the engine's first update hasn't happened yet, and it takes whatever is waiting.
#[derive(Resource, Clone, Default)]
pub(crate) struct LateWake(Arc<Mutex<Option<Wake>>>);

impl LateWake {
    fn wake(&self) {
        if let Ok(wake) = self.0.lock()
            && let Some(wake) = wake.as_ref()
        {
            wake();
        }
    }

    fn bind(&self, wake: Wake) {
        if let Ok(mut slot) = self.0.lock() {
            *slot = Some(wake);
        }
    }
}

/// Watching the chosen Pack, started before it's first read, until the engine takes it over.
pub(crate) enum Watching {
    /// There's no Pack to watch.
    Nothing,
    Started {
        watch: FolderWatch,
        edits: Receiver<PackEdited>,
        wake: LateWake,
    },
    /// The log file only exists once the engine is up, so the reason waits until then.
    Failed { folder: PathBuf, error: io::Error },
}

impl Watching {
    /// Starts watching the chosen Pack's folder, if there is one. It's watched whether or not it loads, so fixing a
    /// broken Pack brings the Pet.
    pub(crate) fn start(folder: Option<&Path>) -> Self {
        let Some(folder) = folder else {
            return Self::Nothing;
        };
        let late = LateWake::default();
        let wake = late.clone();
        match watch(folder, Box::new(move || wake.wake())) {
            Ok((watch, edits)) => Self::Started {
                watch,
                edits,
                wake: late,
            },
            Err(error) => Self::Failed {
                folder: folder.to_owned(),
                error,
            },
        }
    }

    /// Hands the watching over to the engine. If it couldn't start, the app carries on without hot reload, and the log
    /// says why.
    pub(super) fn install(self, app: &mut App) {
        match self {
            Self::Nothing => {}
            Self::Started { watch, edits, wake } => {
                app.insert_resource(PackWatch { _watch: watch })
                    .insert_resource(PackEdits(Mutex::new(edits)))
                    .insert_resource(wake)
                    .add_systems(
                        Startup,
                        bind_wake.run_if(resource_exists::<EventLoopProxyWrapper>),
                    )
                    .add_systems(PreUpdate, receive_edits.before(apply_edits));
            }
            Self::Failed { folder, error } => {
                let reason = format!(
                    "Can't watch the Pack in {} for edits, so the app needs restarting to see them: {error}",
                    folder.display()
                );
                app.add_systems(Startup, move || warn!("{reason}"));
            }
        }
    }
}

/// Points the wake-up at the engine's event loop, now that it exists.
fn bind_wake(wake: Res<LateWake>, proxy: Res<EventLoopProxyWrapper>) {
    let proxy = (*proxy).clone();
    wake.bind(Box::new(move || {
        // This fails only once the engine has gone, and then nobody needs telling.
        let _ = proxy.send_event(WinitUserEvent::WakeUp);
    }));
}

/// Watches the Pack in `folder`. Once a burst of edits has settled, it's read again on the watcher's thread and sent,
/// then `wake` is called.
pub(super) fn watch(folder: &Path, wake: Wake) -> io::Result<(FolderWatch, Receiver<PackEdited>)> {
    let (sender, edits) = mpsc::channel();
    let pack_folder = folder.to_owned();
    let read_again = move || {
        let edited = PackEdited {
            folder: pack_folder.clone(),
            loaded: pet_content::load(&pack_folder),
        };
        // This fails only once the engine has gone, and then nobody needs telling.
        if sender.send(edited).is_ok() {
            wake();
        }
    };
    let watch = FolderWatch::start(folder, Settle::new(SETTLE, SETTLE_AT_MOST), read_again)?;
    Ok((watch, edits))
}

/// Passes on what the watcher has read.
pub(super) fn receive_edits(edits: Res<PackEdits>, mut edited: MessageWriter<PackEdited>) {
    let Ok(edits) = edits.0.lock() else { return };
    edited.write_batch(edits.try_iter());
}

/// Applies each edit in the order it arrived, so each line in the log is true when it's written: an invalid edit
/// keeps whichever Pack is in use at that moment, which may be one an earlier edit in the same update brought. Edits
/// rarely come more than one at a time, because the watcher waits for a save to settle, so applying one that a later
/// one replaces costs little.
pub(super) fn apply_edits(world: &mut World) {
    let edits: Vec<PackEdited> = world
        .resource_mut::<Messages<PackEdited>>()
        .drain()
        .collect();
    for edit in edits {
        match edit.loaded {
            Ok(pack) => apply(world, &edit.folder, pack),
            Err(errors) => log_errors(world, &edit.folder, &errors),
        }
    }
}

/// Swaps in a valid Pack. Its images are made here, its Definition replaces the simulation's, and the simulation moves
/// each Actor onto it, all in this one system. The Pet's `ShownFrame` then counts as changed, so the renderer, the
/// shadow and the Emote draw it from the new Pack, and the Hit target is published again.
fn apply(world: &mut World, folder: &Path, pack: LoadedPack) {
    info!(
        target: log_file::RECORD,
        "Reloaded the Pack \"{}\" from {}",
        pack.definition.name,
        folder.display()
    );
    let assets = world.resource_scope(|world, mut images: Mut<Assets<Image>>| {
        let mut materials = world.resource_mut::<Assets<ColorMaterial>>();
        PackAssets::new(&pack, &mut images, &mut materials)
    });
    world.insert_resource(assets);
    pet_sim::replace_pack(world, Arc::new(pack.definition));
}

/// Logs every error of an invalid edit, which changes nothing.
fn log_errors(world: &World, folder: &Path, errors: &[PackError]) {
    let kept = if world.contains_resource::<CurrentPack>() {
        "so the Pet keeps the Pack it had"
    } else {
        "so there's still no Pet"
    };
    warn!(
        "The edited Pack in {} has {} errors, {kept}:",
        folder.display(),
        errors.len()
    );
    for error in errors {
        warn!("  {error}");
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::io::BufWriter;
    use std::thread;
    use std::time::{Duration, Instant};

    use bevy::ecs::schedule::SingleThreadedExecutor;
    use bevy::time::{Fixed, TimeUpdateStrategy};
    use pet_core::geometry::{Point, Rect};
    use pet_core::hit_mask::Placement;
    use pet_core::pack::{AnimationId, EmoteId, FormId};
    use pet_sim::{
        AnimationClock, EmoteRequest, PackReloaded, Pet, ShownEmote, ShownFrame, SimPlugin,
    };

    use super::*;
    use crate::engine::PetRenderPlugin;
    use crate::engine::emote_view::EmoteOf;
    use crate::engine::pack_assets::PackAtStartup;
    use crate::engine::renderer::publish_hit_target;
    use crate::engine::shadow::ShadowOf;
    use crate::engine::test_support::{
        CapturedLog, RecordingPlatform, TestPack, pack_from, placeholder_folder,
        placeholder_manifest,
    };
    use crate::pack::StartingPack;

    const WORK_AREA: Rect = Rect {
        left: 0,
        top: 0,
        right: 1000,
        bottom: 800,
    };

    #[derive(Resource, Default)]
    struct Reloads(usize);

    fn count_reloads(mut reloads: MessageReader<PackReloaded>, mut count: ResMut<Reloads>) {
        count.0 += reloads.read().count();
    }

    /// The engine as the app has it, without a window, publishing to the recording platform, with time standing still.
    fn engine(starting: StartingPack) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<ColorMaterial>>()
            .init_resource::<Assets<Mesh>>()
            .insert_resource(PackAtStartup(starting))
            .insert_resource(RecordingPlatform::default())
            .init_resource::<Reloads>()
            .add_plugins((
                SimPlugin {
                    work_area: WORK_AREA,
                },
                PetRenderPlugin { overlay: WORK_AREA },
            ))
            .add_systems(
                Update,
                (
                    publish_hit_target::<RecordingPlatform>.run_if(resource_exists::<PackAssets>),
                    count_reloads,
                ),
            );
        app
    }

    /// The engine running `pack`, after its first update.
    fn running(pack: (TestPack, LoadedPack)) -> (App, TestPack) {
        let (folder, pack) = pack;
        let mut app = engine(StartingPack::Loaded {
            folder: folder.0.clone(),
            pack,
        });
        app.update();
        (app, folder)
    }

    fn edit(app: &mut App, folder: &TestPack, loaded: Result<LoadedPack, Vec<PackError>>) {
        app.world_mut().write_message(PackEdited {
            folder: folder.0.clone(),
            loaded,
        });
    }

    fn placeholder(test: &str) -> (TestPack, LoadedPack) {
        pack_from(test, &placeholder_manifest())
    }

    /// The placeholder with every opaque colour inverted, and nothing else changed: the same names, Frames and sizes.
    fn recoloured_placeholder(test: &str) -> (TestPack, LoadedPack) {
        let (folder, _) = placeholder(test);
        let original = pet_content::load(&placeholder_folder()).unwrap();
        let frames = original.pixels.frames(FormId(0), AnimationId(0));
        let (width, height) = (frames[0].width as usize, frames[0].height as usize);
        let strip_width = width * frames.len();
        let mut strip = vec![0; strip_width * height * 4];
        for (index, frame) in frames.iter().enumerate() {
            for y in 0..height {
                for x in 0..width {
                    let from = (y * width + x) * 4;
                    let to = (y * strip_width + index * width + x) * 4;
                    let [r, g, b, a] = frame.rgba[from..from + 4] else {
                        unreachable!()
                    };
                    strip[to..to + 4].copy_from_slice(&[255 - r, 255 - g, 255 - b, a]);
                }
            }
        }
        let file = BufWriter::new(File::create(folder.0.join("hatchling/idle.png")).unwrap());
        let mut encoder = png::Encoder::new(file, strip_width as u32, height as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&strip).unwrap();
        writer.finish().unwrap();
        let pack = pet_content::load(&folder.0).unwrap_or_else(|errors| panic!("{errors:?}"));
        (folder, pack)
    }

    fn the_pet(app: &mut App) -> Entity {
        let world = app.world_mut();
        world
            .query_filtered::<Entity, With<Pet>>()
            .single(world)
            .expect("one Pet")
    }

    /// The material the Pet is drawn with.
    fn drawn_material(app: &mut App) -> Handle<ColorMaterial> {
        let pet = the_pet(app);
        app.world()
            .get::<MeshMaterial2d<ColorMaterial>>(pet)
            .expect("the Pet is drawn")
            .0
            .clone()
    }

    fn frame_material(app: &App, frame: usize) -> Handle<ColorMaterial> {
        app.world()
            .resource::<PackAssets>()
            .frames(FormId(0), AnimationId(0))[frame]
            .material
            .clone()
    }

    /// The pixels of the image a material draws.
    fn pixels_of(app: &App, material: &Handle<ColorMaterial>) -> Vec<u8> {
        let material = app
            .world()
            .resource::<Assets<ColorMaterial>>()
            .get(material)
            .unwrap();
        let image = material.texture.as_ref().unwrap();
        let image = app.world().resource::<Assets<Image>>().get(image).unwrap();
        image.data.clone().unwrap()
    }

    #[test]
    fn given_a_running_pet_when_a_valid_edit_with_new_art_arrives_then_its_frames_are_made_again_and_the_new_art_is_drawn()
     {
        // Given: on its first Frame
        let (mut app, folder) = running(placeholder("valid"));
        let before = frame_material(&app, 0);
        let (_, recoloured) = recoloured_placeholder("valid-recoloured");
        let new_art = recoloured.pixels.frames(FormId(0), AnimationId(0))[0]
            .rgba
            .clone();

        // When: still on its first Frame, so only the reload can make it draw again
        edit(&mut app, &folder, Ok(recoloured));
        app.update();

        // Then
        let after = frame_material(&app, 0);
        assert_ne!(after, before);
        assert_eq!(pixels_of(&app, &after), new_art);
        assert_eq!(drawn_material(&mut app), after);
        assert_eq!(app.world().resource::<Reloads>().0, 1);
    }

    #[test]
    fn given_a_published_pet_when_a_valid_edit_moves_its_anchor_then_the_hit_target_is_published_again()
     {
        // Given: the Anchor at x = 12 of 24, then x = 4, so the art sits 8 x 2 further right of it
        let (mut app, folder) = running(placeholder("hit-target"));
        let (_, moved) = pack_from(
            "hit-target-moved",
            &placeholder_manifest()
                .replace("anchor = { x = 12, y = 18 }", "anchor = { x = 4, y = 18 }"),
        );
        let mask = moved.pixels.frames(FormId(0), AnimationId(0))[0]
            .hit_mask
            .clone();

        // When
        edit(&mut app, &folder, Ok(moved));
        app.update();

        // Then: the Anchor stays at (500, 800)
        let published = RecordingPlatform::published(&app);
        assert_eq!(published.len(), 2, "{published:?}");
        assert_eq!(
            published[1],
            (
                mask,
                Placement {
                    top_left: Point { x: 492, y: 764 },
                    scale: 2,
                    flip_x: false
                }
            )
        );
    }

    #[test]
    fn given_a_running_pet_when_an_invalid_edit_arrives_then_nothing_changes_and_every_error_is_logged()
     {
        // Given: the edit runs on this thread, where the log is captured
        let (mut app, folder) = running(placeholder("invalid"));
        app.edit_schedule(PreUpdate, |schedule| {
            schedule.set_executor(SingleThreadedExecutor::new());
        });
        let (material, pack) = (
            frame_material(&app, 0),
            app.world().resource::<CurrentPack>().0.clone(),
        );
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
        let log = CapturedLog::default();

        // When
        edit(&mut app, &folder, Err(errors));
        log.during(|| app.update());
        app.update();

        // Then
        assert_eq!(frame_material(&app, 0), material);
        assert!(Arc::ptr_eq(&app.world().resource::<CurrentPack>().0, &pack));
        assert_eq!(drawn_material(&mut app), material);
        assert_eq!(app.world().resource::<Reloads>().0, 0);
        assert_eq!(RecordingPlatform::published(&app).len(), 1);
        let logged = log.text();
        let warnings: Vec<&str> = logged
            .lines()
            .filter(|line| line.contains("WARN"))
            .collect();
        assert_eq!(warnings.len(), 3, "{logged}");
        assert!(
            warnings[0].contains(&folder.0.display().to_string()),
            "{logged}"
        );
        assert!(
            warnings[1].contains("manifest.toml: starting_form"),
            "{logged}"
        );
        assert!(
            warnings[2].contains("hatchling/idle.png: forms.hatchling"),
            "{logged}"
        );
    }

    /// Two Forms with shadows, the second with two Animations, and two Emotes.
    fn big_pack(test: &str) -> (TestPack, LoadedPack) {
        let form = |name: &str| {
            format!(
                r##"
[forms.{name}]
cell = {{ width = 24, height = 18 }}
anchor = {{ x = 12, y = 18 }}
body = {{ x = -9, y = -15, width = 18, height = 15 }}
default_action = "idle"
shadow = {{ width = 16, colour = "#22203480" }}

[forms.{name}.actions.idle]
animation = "idle"

[forms.{name}.actions.nap]
animation = "nap"

[forms.{name}.animations.idle]
sheet = "hatchling/idle.png"
durations_ms = [100, 100]

[forms.{name}.animations.nap]
sheet = "hatchling/idle.png"
durations_ms = [250, 250]
"##
            )
        };
        let manifest = format!(
            r#"format_version = 1
name = "Big"
starting_form = "hatchling"
{}{}
[emotes.spare]
sheet = "hatchling/idle.png"
cell = {{ width = 48, height = 18 }}
durations_ms = [500]
mode = "looping"

[emotes.wave]
sheet = "hatchling/idle.png"
cell = {{ width = 24, height = 18 }}
durations_ms = [200, 200]
mode = "looping"
"#,
            form("hatchling"),
            form("adult")
        );
        pack_from(test, &manifest)
    }

    /// One Form, one Animation of one Frame, no shadow and no Emotes.
    fn tiny_pack(test: &str) -> (TestPack, LoadedPack) {
        pack_from(
            test,
            r#"format_version = 1
name = "Tiny"
starting_form = "egg"

[forms.egg]
cell = { width = 48, height = 18 }
anchor = { x = 24, y = 18 }
body = { x = -9, y = -15, width = 18, height = 15 }
default_action = "idle"

[forms.egg.actions.idle]
animation = "idle"

[forms.egg.animations.idle]
sheet = "hatchling/idle.png"
durations_ms = [250]
"#,
        )
    }

    /// `count` updates of one tick each.
    fn ticks(app: &mut App, count: u32) {
        let tick = app.world().resource::<Time<Fixed>>().timestep();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(tick));
        for _ in 0..count {
            app.update();
        }
    }

    fn visibility_of<T: Component>(app: &mut App) -> Visibility {
        let world = app.world_mut();
        *world
            .query_filtered::<&Visibility, With<T>>()
            .single(world)
            .expect("one of them")
    }

    #[test]
    fn given_a_pet_deep_in_a_big_pack_when_a_tiny_pack_replaces_it_then_it_is_drawn_from_the_tiny_pack_without_panicking()
     {
        // Given: an "adult", its second Form, on the second Frame of its second Animation, with its shadow and the
        // second Frame of the second Emote drawn
        let (mut app, folder) = running(big_pack("shrink"));
        let pet = the_pet(&mut app);
        let adult = FormId(1);
        let mut clock = app.world_mut().get_mut::<AnimationClock>(pet).unwrap();
        clock.form = adult;
        clock.animation = AnimationId(1);
        app.world_mut()
            .write_message(EmoteRequest::Show(EmoteId(1)));
        ticks(&mut app, 9);
        let shown = *app.world().get::<ShownFrame>(pet).unwrap();
        assert_eq!(
            (shown.form, shown.animation, shown.frame),
            (adult, AnimationId(1), 1)
        );
        assert_eq!(
            app.world().get::<ShownEmote>(pet).unwrap().0.unwrap().frame,
            1
        );
        assert_eq!(visibility_of::<ShadowOf>(&mut app), Visibility::Inherited);
        let (_, tiny) = tiny_pack("shrink-tiny");

        // When: ticks and drawing go on, and the old Emote is asked for again
        edit(&mut app, &folder, Ok(tiny));
        app.world_mut()
            .write_message(EmoteRequest::Show(EmoteId(1)));
        ticks(&mut app, 10);

        // Then
        assert_eq!(drawn_material(&mut app), frame_material(&app, 0));
        assert_eq!(visibility_of::<ShadowOf>(&mut app), Visibility::Hidden);
        assert_eq!(visibility_of::<EmoteOf>(&mut app), Visibility::Hidden);
    }

    #[test]
    fn given_no_pet_because_the_pack_was_broken_at_startup_when_a_valid_edit_arrives_then_the_pet_appears()
     {
        // Given
        let (folder, pack) = placeholder("fixed");
        let mut app = engine(StartingPack::Broken {
            folder: folder.0.clone(),
            errors: vec![PackError::new("manifest.toml", "", "is missing")],
        });
        app.update();
        let world = app.world_mut();
        assert_eq!(world.query::<&Pet>().iter(world).count(), 0);

        // When
        edit(&mut app, &folder, Ok(pack));
        app.update();
        app.update();

        // Then
        assert_eq!(drawn_material(&mut app), frame_material(&app, 0));
        assert_eq!(RecordingPlatform::published(&app).len(), 1);
    }

    /// The placeholder with another name, so it shows which Pack is in use.
    fn renamed_placeholder(test: &str, name: &str) -> (TestPack, LoadedPack) {
        pack_from(
            test,
            &placeholder_manifest().replace(
                "name = \"Placeholder dragon\"",
                &format!("name = \"{name}\""),
            ),
        )
    }

    fn current_name(app: &App) -> String {
        app.world().resource::<CurrentPack>().0.name.clone()
    }

    /// Writes the placeholder manifest, renamed, into `folder`, as an editor saving it would.
    fn save_renamed(folder: &TestPack, name: &str) {
        let manifest = placeholder_manifest().replace(
            "name = \"Placeholder dragon\"",
            &format!("name = \"{name}\""),
        );
        fs::write(folder.0.join("manifest.toml"), manifest).unwrap();
    }

    /// Far longer than the settle wait and a read of the placeholder, so only a lost edit takes this long.
    const BOUND: Duration = Duration::from_secs(3);

    #[test]
    fn given_the_watcher_has_sent_an_edit_from_its_thread_when_the_engine_updates_then_it_is_applied()
     {
        // Given
        let (mut app, folder) = running(placeholder("receive"));
        let (sender, receiver) = mpsc::channel();
        app.insert_resource(PackEdits(Mutex::new(receiver)))
            .add_systems(PreUpdate, receive_edits.before(apply_edits));
        let (_, renamed) = renamed_placeholder("receive-renamed", "Edited dragon");
        let pack_folder = folder.0.clone();

        // When
        thread::spawn(move || {
            sender
                .send(PackEdited {
                    folder: pack_folder,
                    loaded: Ok(renamed),
                })
                .unwrap();
        })
        .join()
        .unwrap();
        app.update();

        // Then
        assert_eq!(current_name(&app), "Edited dragon");
        assert_eq!(app.world().resource::<Reloads>().0, 1);
    }

    #[test]
    fn given_a_watched_pack_when_its_manifest_is_saved_then_the_pack_read_again_is_sent_and_the_engine_is_woken()
     {
        // Given
        let (folder, _) = placeholder("watch");
        let (woke, wakes) = mpsc::channel();
        let (_watch, edits) = watch(
            &folder.0,
            Box::new(move || {
                let _ = woke.send(());
            }),
        )
        .expect("watch the Pack");

        // When
        save_renamed(&folder, "Edited dragon");

        // Then
        let edited = edits.recv_timeout(BOUND).expect("the edit is read");
        assert_eq!(edited.folder, folder.0);
        let pack = edited.loaded.unwrap_or_else(|errors| panic!("{errors:?}"));
        assert_eq!(pack.definition.name, "Edited dragon");
        assert_eq!(wakes.recv_timeout(BOUND), Ok(()));
    }

    #[test]
    fn given_the_pack_is_watched_and_read_when_it_is_edited_before_the_engine_first_updates_then_the_edit_is_applied()
     {
        // Given: as the app starts, watching first, then reading the Pack
        let (folder, _) = placeholder("before-first-update");
        let watching = Watching::start(Some(&folder.0));
        let starting = StartingPack::load_chosen(None, Some(folder.0.clone()));

        // When: saved before the engine exists
        save_renamed(&folder, "Edited dragon");
        let mut app = engine(starting);
        watching.install(&mut app);
        let started = Instant::now();
        app.update();
        while current_name(&app) != "Edited dragon" && started.elapsed() < BOUND {
            thread::sleep(Duration::from_millis(20));
            app.update();
        }

        // Then
        assert_eq!(current_name(&app), "Edited dragon");
    }

    #[test]
    fn given_a_valid_edit_then_an_invalid_one_in_the_same_update_when_applied_then_each_is_logged_in_turn_and_the_valid_one_stays()
     {
        // Given: the log is captured on this thread
        let (mut app, folder) = running(placeholder("order"));
        app.edit_schedule(PreUpdate, |schedule| {
            schedule.set_executor(SingleThreadedExecutor::new());
        });
        let (_, renamed) = renamed_placeholder("order-renamed", "Edited dragon");
        let errors = vec![PackError::new("manifest.toml", "name", "is missing")];
        let log = CapturedLog::default();

        // When
        edit(&mut app, &folder, Ok(renamed));
        edit(&mut app, &folder, Err(errors));
        log.during(|| app.update());

        // Then: the invalid edit keeps the Pack the valid one brought, so that's applied before it's said
        let logged = log.text();
        let reloaded = logged.find("Reloaded the Pack \"Edited dragon\"");
        let kept = logged.find("so the Pet keeps the Pack it had");
        assert!(reloaded.is_some() && kept.is_some(), "{logged}");
        assert!(reloaded < kept, "{logged}");
        assert_eq!(current_name(&app), "Edited dragon");
    }

    #[test]
    fn given_a_pet_with_a_shadow_in_a_one_form_pack_when_an_edit_adds_forms_before_its_own_then_it_and_its_shadow_are_drawn_from_the_new_pack()
     {
        // Given: one Form with a shadow
        let shadowed = |name: &str| {
            format!(
                r##"
[forms.{name}]
cell = {{ width = 24, height = 18 }}
anchor = {{ x = 12, y = 18 }}
body = {{ x = -9, y = -15, width = 18, height = 15 }}
default_action = "idle"
shadow = {{ width = 16, colour = "#22203480" }}

[forms.{name}.actions.idle]
animation = "idle"

[forms.{name}.animations.idle]
sheet = "hatchling/idle.png"
durations_ms = [250, 250]
"##
            )
        };
        let manifest = |forms: &[&str]| {
            let forms: String = forms.iter().map(|name| shadowed(name)).collect();
            format!(
                "format_version = 1\nname = \"Growing\"\nstarting_form = \"hatchling\"\n{forms}"
            )
        };
        let (mut app, folder) = running(pack_from("grow", &manifest(&["hatchling"])));

        // When: two Forms come before "hatchling", by name and in the file, so it's numbered past the old Pack's end
        let (_, grown) = pack_from("grow-grown", &manifest(&["adult", "egg", "hatchling"]));
        let hatchling = grown
            .definition
            .forms
            .iter()
            .position(|form| form.name == "hatchling")
            .map(FormId)
            .unwrap();
        assert_eq!(hatchling, FormId(2));
        edit(&mut app, &folder, Ok(grown));
        app.update();

        // Then
        let pet = the_pet(&mut app);
        assert_eq!(app.world().get::<ShownFrame>(pet).unwrap().form, hatchling);
        let drawn = app
            .world()
            .resource::<PackAssets>()
            .frames(hatchling, AnimationId(0))[0]
            .material
            .clone();
        assert_eq!(drawn_material(&mut app), drawn);
        assert_eq!(visibility_of::<ShadowOf>(&mut app), Visibility::Inherited);
    }
}
