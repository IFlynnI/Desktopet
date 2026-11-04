//! Turns the starting Pack's pixels into Bevy images, hands its Definition to the simulation, and logs what happened
//! to the Pack. The log file only exists once the engine is up, so this is where a broken or missing Pack gets
//! reported.

use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use pet_content::LoadedPack;
use pet_core::hit_mask::HitMask;
use pet_core::pack::{AnimationId, FormId};
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
}

impl PackAssets {
    pub fn new(
        pack: &LoadedPack,
        images: &mut Assets<Image>,
        materials: &mut Assets<ColorMaterial>,
    ) -> Self {
        let definition = &pack.definition;
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
                            .map(|frame| {
                                let image = images.add(pixel_art(
                                    frame.width,
                                    frame.height,
                                    frame.rgba.clone(),
                                ));
                                FrameAsset {
                                    material: pixel_art::material(image, materials),
                                    hit_mask: Arc::new(frame.hit_mask.clone()),
                                }
                            })
                            .collect()
                    })
                    .collect()
            })
            .collect();
        Self { frames }
    }

    pub fn frames(&self, form: FormId, animation: AnimationId) -> &[FrameAsset] {
        &self.frames[form.0][animation.0]
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
        info!(
            target: log_file::RECORD,
            "Using the Pack \"{}\" in {}",
            pack.definition.name,
            folder.display()
        );
        commands.insert_resource(PackAssets::new(pack, &mut images, &mut materials));
        commands.insert_resource(CurrentPack(Arc::new(pack.definition.clone())));
    }
    // Its pixels are in the images now, so they needn't stay in memory twice.
    commands.remove_resource::<PackAtStartup>();
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn placeholder() -> LoadedPack {
        let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/placeholder");
        pet_content::load(&folder).unwrap_or_else(|errors| panic!("{errors:?}"))
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
}
