//! Turns the starting Pack's pixels into Bevy images, hands its Definition to the simulation, and logs what happened
//! to the Pack. The log file only exists once the engine is up, so this is where a broken or missing Pack gets
//! reported.

use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use pet_content::{FramePixels, LoadedPack};
use pet_core::hit_mask::HitMask;
use pet_core::pack::{AnimationId, EmoteId, FormId, PackDefinition};
use pet_sim::CurrentPack;

use super::{log_file, pixel_art};
use crate::pack::StartingPack;

/// The Pack chosen before the engine started, until `add_starting_pack` has used it.
#[derive(Resource)]
pub struct PackAtStartup(pub StartingPack);

/// One Frame, ready to draw and to hit-test.
pub struct FrameAsset {
    /// The Frame's image as it's drawn, made once so showing a Frame never makes a material.
    pub material: Handle<ColorMaterial>,
    pub hit_mask: Arc<HitMask>,
}

/// Every Frame of the Pack in use as a Bevy image and a material to draw it with. It exists only when a Pack loaded,
/// and its Definition is then the simulation's `CurrentPack`.
#[derive(Resource)]
pub struct PackAssets {
    /// By Form, then Animation, then Frame, numbered as in the Definition.
    frames: Vec<Vec<Vec<FrameAsset>>>,
    /// By Emote, then Frame. Made once with the rest, so showing an Emote never makes a material. An edited Pack
    /// replaces all of these (`hot_reload`).
    emotes: Vec<Vec<Handle<ColorMaterial>>>,
}

impl PackAssets {
    pub fn new(
        pack: &LoadedPack,
        images: &mut Assets<Image>,
        materials: &mut Assets<ColorMaterial>,
    ) -> Self {
        let definition = &pack.definition;
        let mut material = |frame: &FramePixels| {
            let image = images.add(pixel_art(frame.width, frame.height, frame.rgba.clone()));
            pixel_art::material(image, materials)
        };
        let frames = definition
            .forms
            .iter()
            .enumerate()
            .map(|(form, form_def)| {
                (0..form_def.animations.len())
                    .map(|animation| {
                        let pixels = pack.pixels.frames(FormId(form), AnimationId(animation));
                        pixels
                            .iter()
                            .map(|frame| FrameAsset {
                                material: material(frame),
                                hit_mask: Arc::new(frame.hit_mask.clone()),
                            })
                            .collect()
                    })
                    .collect()
            })
            .collect();
        let emotes = (0..definition.emotes.len())
            .map(|emote| {
                let pixels = pack.pixels.emote_frames(EmoteId(emote));
                pixels.iter().map(&mut material).collect()
            })
            .collect();
        Self { frames, emotes }
    }

    pub fn frames(&self, form: FormId, animation: AnimationId) -> &[FrameAsset] {
        &self.frames[form.0][animation.0]
    }

    /// The material of each Frame of `emote`. An Emote never catches clicks, so it has no Hit masks.
    pub fn emote_frames(&self, emote: EmoteId) -> &[Handle<ColorMaterial>] {
        &self.emotes[emote.0]
    }
}

/// Pixel art from sRGB RGBA8 pixels, scaled with nearest-neighbour, so it stays crisp.
pub(super) fn pixel_art(width: u32, height: u32, rgba: Vec<u8>) -> Image {
    let mut image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    image
}

/// Logs what happened to the starting Pack and, if it loaded, makes its images.
pub fn add_starting_pack(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    starting: Res<PackAtStartup>,
) {
    for problem in starting.0.problems() {
        error!("{problem}");
    }
    if let StartingPack::Loaded { folder, pack } = &starting.0 {
        let definition = &pack.definition;
        info!(
            target: log_file::RECORD,
            "Using the Pack \"{}\" in {}; the Pet starts in {}",
            definition.name,
            folder.display(),
            form_and_action(definition, definition.starting_form)
        );
        commands.insert_resource(PackAssets::new(pack, &mut images, &mut materials));
        commands.insert_resource(CurrentPack(Arc::new(pack.definition.clone())));
    }
    // Its pixels are in the images now, so they needn't stay in memory twice.
    commands.remove_resource::<PackAtStartup>();
}

/// What a Pet in `form` is doing, for the log: `its "egg" Form, playing "idle"`. A Pet that has just appeared, or just
/// had its Pack reloaded, always plays its Form's default Action, so that's the one named.
pub(super) fn form_and_action(definition: &PackDefinition, form: FormId) -> String {
    let form = definition.form(form);
    format!(
        "its \"{}\" Form, playing \"{}\"",
        form.name,
        form.action(form.default_action).name
    )
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::{env, fs, process};

    use std::time::Duration;

    use bevy::ecs::schedule::SingleThreadedExecutor;
    use bevy::time::TimeUpdateStrategy;
    use pet_core::geometry::Rect;
    use pet_sim::SimPlugin;

    use super::*;
    use crate::engine::test_support::{
        CapturedLog, assert_pet_plays, egg_and_hatchling, pack_from,
    };

    fn placeholder_folder() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/placeholder")
    }

    fn placeholder() -> LoadedPack {
        pet_content::load(&placeholder_folder()).unwrap_or_else(|errors| panic!("{errors:?}"))
    }

    /// The placeholder Pack with two Emotes that borrow its idle strip as their sheet, loaded from a folder of its
    /// own, which is removed again.
    fn placeholder_with_emotes() -> LoadedPack {
        let folder = env::temp_dir().join(format!("desktop-pet-pack-assets-{}", process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(folder.join("hatchling")).unwrap();
        fs::copy(
            placeholder_folder().join("hatchling/idle.png"),
            folder.join("hatchling/idle.png"),
        )
        .unwrap();
        let manifest = fs::read_to_string(placeholder_folder().join("manifest.toml")).unwrap();
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
        let _ = fs::remove_dir_all(&folder);
        pack
    }

    fn startup_with(starting: StartingPack) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<ColorMaterial>>()
            .insert_resource(PackAtStartup(starting))
            .add_systems(Startup, add_starting_pack);
        app
    }

    #[test]
    fn given_the_placeholder_pack_when_making_its_images_then_each_frame_is_a_crisp_image_with_its_material_and_hit_mask()
     {
        // Given
        let pack = placeholder();
        let mut images = Assets::<Image>::default();
        let mut materials = Assets::<ColorMaterial>::default();

        // When
        let assets = PackAssets::new(&pack, &mut images, &mut materials);

        // Then
        let form = pack.definition.starting_form;
        let idle = pack.definition.form(form).default_animation();
        let frames = assets.frames(form, idle);
        let pixels = pack.pixels.frames(form, idle);
        assert_eq!(frames.len(), 2);
        for (frame, pixels) in frames.iter().zip(pixels) {
            let material = materials
                .get(&frame.material)
                .expect("the material was added");
            assert_eq!(material.color, Color::WHITE);
            let texture = material
                .texture
                .as_ref()
                .expect("the material has the image");
            let image = images.get(texture).expect("the image was added");
            assert_eq!(image.size(), UVec2::new(24, 18));
            assert_eq!(image.data.as_deref(), Some(pixels.rgba.as_slice()));
            assert_eq!(image.sampler, ImageSampler::nearest());
            assert_eq!(*frame.hit_mask, pixels.hit_mask);
        }
    }

    #[test]
    fn given_a_loaded_pack_at_startup_when_the_engine_starts_then_its_assets_are_ready() {
        // Given
        let mut app = startup_with(StartingPack::Loaded {
            folder: "placeholder".into(),
            pack: placeholder(),
        });

        // When
        app.update();

        // Then
        assert!(app.world().contains_resource::<PackAssets>());
        let current = app.world().resource::<CurrentPack>();
        assert_eq!(current.0.name, "Placeholder dragon");
        assert!(!app.world().contains_resource::<PackAtStartup>());
    }

    #[test]
    fn given_a_pack_starting_as_an_egg_when_the_engine_starts_then_the_using_line_says_the_pet_starts_as_an_egg_playing_its_default_action()
     {
        // Given: the line is logged on this thread, where the log is captured. The simulation brings the Pet, with
        // time standing still.
        let (folder, pack) = pack_from("say-starting-form", &egg_and_hatchling("egg"));
        let mut app = startup_with(StartingPack::Loaded {
            folder: folder.0.clone(),
            pack,
        });
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
            .add_plugins(SimPlugin {
                work_area: Rect {
                    left: 0,
                    top: 0,
                    right: 1000,
                    bottom: 800,
                },
            });
        app.edit_schedule(Startup, |schedule| {
            schedule.set_executor(SingleThreadedExecutor::new());
        });
        let log = CapturedLog::default();

        // When
        log.during(|| app.update());

        // Then
        let logged = log.text();
        let line = format!(
            "Using the Pack \"Two Forms\" in {}; the Pet starts in its \"egg\" Form, playing \"wobble\"",
            folder.0.display()
        );
        assert!(logged.contains(&line), "{logged}");
        assert_pet_plays(&mut app, "egg", "wobble");
    }

    #[test]
    fn given_no_pack_at_startup_when_the_engine_starts_then_there_are_no_pack_assets() {
        // Given
        let mut app = startup_with(StartingPack::NoPack { packs_folder: None });

        // When
        app.update();

        // Then
        assert!(!app.world().contains_resource::<PackAssets>());
        assert!(!app.world().contains_resource::<CurrentPack>());
        assert!(!app.world().contains_resource::<PackAtStartup>());
    }

    #[test]
    fn given_a_pack_with_emotes_when_making_its_images_then_each_emote_frame_is_a_crisp_image_with_its_material()
     {
        // Given: "wave" is the second Emote, so picking the first shows up
        let pack = placeholder_with_emotes();
        let mut images = Assets::<Image>::default();
        let mut materials = Assets::<ColorMaterial>::default();

        // When
        let assets = PackAssets::new(&pack, &mut images, &mut materials);

        // Then
        let wave = EmoteId(1);
        assert_eq!(pack.definition.emote(wave).name, "wave");
        let frames = assets.emote_frames(wave);
        let pixels = pack.pixels.emote_frames(wave);
        assert_eq!(frames.len(), 2);
        for (material, pixels) in frames.iter().zip(pixels) {
            let material = materials.get(material).expect("the material was added");
            assert_eq!(material.color, Color::WHITE);
            let texture = material
                .texture
                .as_ref()
                .expect("the material has the image");
            let image = images.get(texture).expect("the image was added");
            assert_eq!(image.size(), UVec2::new(24, 18));
            assert_eq!(image.data.as_deref(), Some(pixels.rgba.as_slice()));
            assert_eq!(image.sampler, ImageSampler::nearest());
        }
    }
}
