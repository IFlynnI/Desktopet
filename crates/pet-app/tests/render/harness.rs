//! A windowless app that draws the Pet into an image in memory, and reads its pixels back. There's no window, no
//! screenshot and no input: the pixels are a GPU copy of the app's own image
//! (docs/research/windowless-render-tests.md).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bevy::app::PluginsState;
use bevy::asset::RenderAssetUsages;
use bevy::camera::RenderTarget;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::render::render_resource::{PollType, TextureFormat, TextureUsages};
use bevy::render::renderer::RenderDevice;
use bevy::time::TimeUpdateStrategy;
use bevy::winit::WinitPlugin;
use pet_app::{PackAtStartup, PetRenderPlugin, StartingPack};
use pet_content::LoadedPack;
use pet_core::geometry::Rect;
use pet_core::physics::Obstacle;
use pet_sim::{Pet, SimPlugin};

/// An RGBA8 picture, row by row from the top.
#[derive(Clone, PartialEq, Eq)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Picture {
    pub fn clear(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            rgba: vec![0; (width * height * 4) as usize],
        }
    }

    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.width + x) * 4) as usize;
        self.rgba[i..i + 4].try_into().unwrap()
    }

    pub fn set(&mut self, x: u32, y: u32, colour: [u8; 4]) {
        let i = ((y * self.width + x) * 4) as usize;
        self.rgba[i..i + 4].copy_from_slice(&colour);
    }

    /// Draws `art` at `scale` with its top-left at (`left`, `top`), each art pixel a `scale` x `scale` square,
    /// mirrored when `mirrored`. A clear art pixel leaves what is underneath, as blending does. Other art pixels
    /// replace it, so they must be opaque or land on clear pixels.
    pub fn draw(&mut self, art: &Picture, left: u32, top: u32, scale: u32, mirrored: bool) {
        for y in 0..art.height * scale {
            for x in 0..art.width * scale {
                let column = x / scale;
                let column = if mirrored {
                    art.width - 1 - column
                } else {
                    column
                };
                let pixel = art.pixel(column, y / scale);
                if pixel != [0; 4] {
                    self.set(left + x, top + y, pixel);
                }
            }
        }
    }

    /// The pixels that differ, as "(x, y) got .. want ..", for a readable failure.
    pub fn differences(&self, want: &Picture) -> Vec<String> {
        let mut differences = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let (got, wanted) = (self.pixel(x, y), want.pixel(x, y));
                if got != wanted {
                    differences.push(format!("({x}, {y}) got {got:?} want {wanted:?}"));
                }
            }
        }
        differences
    }
}

pub struct Offscreen {
    pub app: App,
    image: Handle<Image>,
    width: u32,
    height: u32,
}

impl Offscreen {
    /// The Pet from `pack`, drawn into a `width` x `height` image that is the whole work area and the whole Overlay.
    pub fn start(folder: PathBuf, pack: LoadedPack, width: u32, height: u32) -> Self {
        Self::start_with(folder, pack, width, height, Vec::new())
    }

    /// As `start`, with `obstacles` in the world.
    pub fn start_with(
        folder: PathBuf,
        pack: LoadedPack,
        width: u32,
        height: u32,
        obstacles: Vec<Obstacle>,
    ) -> Self {
        let area = Rect {
            left: 0,
            top: 0,
            right: width as i32,
            bottom: height as i32,
        };
        let plugins = pet_app::base_plugins()
            .disable::<WinitPlugin>()
            // Each test is a process of its own under nextest, but `cargo test` runs them together, and they'd
            // fight over the one global logger.
            .disable::<LogPlugin>()
            // Shaders compile before the first frame, so no frame comes out blank while they do.
            .set(RenderPlugin {
                synchronous_pipeline_compilation: true,
                ..pet_app::render_plugin()
            });
        let mut app = App::new();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
            .insert_resource(PackAtStartup(StartingPack::Loaded { folder, pack }))
            .add_plugins(plugins)
            .add_plugins((
                SimPlugin {
                    work_area: area,
                    scale: 2,
                    physics: pet_content::Balance::default().physics,
                    obstacles,
                },
                PetRenderPlugin { overlay: area },
            ));
        // With no `run`, the test does what it would have done.
        while app.plugins_state() == PluginsState::Adding {
            bevy::tasks::tick_global_task_pools_on_main_thread();
        }
        app.finish();
        app.cleanup();

        let mut target =
            Image::new_target_texture(width, height, TextureFormat::Rgba8UnormSrgb, None);
        target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
        target.asset_usage = RenderAssetUsages::RENDER_WORLD;
        let image = app.world_mut().resource_mut::<Assets<Image>>().add(target);
        app.world_mut()
            .spawn((pet_app::pet_camera(), RenderTarget::from(image.clone())));
        let mut offscreen = Self {
            app,
            image,
            width,
            height,
        };
        // The first update draws nothing, even with the shaders compiled (measured in the research), so every test
        // starts after it.
        offscreen.step();
        offscreen
    }

    /// One update, then a wait until the GPU has finished it.
    fn step(&mut self) {
        self.app.update();
        self.app
            .world()
            .resource::<RenderDevice>()
            .wgpu_device()
            .poll(PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .expect("the GPU finishes the frame");
    }

    /// Lets `count` simulation ticks pass in one update. Bevy takes at most 250 ms from one update, about 7 ticks.
    pub fn tick(&mut self, count: u32) {
        assert!(count <= 7, "Bevy would drop the time past 250 ms");
        self.app
            .insert_resource(TimeUpdateStrategy::FixedTimesteps(count));
        self.step();
        self.app
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    }

    /// Changes something about the Pet, as the simulation would.
    pub fn with_pet(&mut self, change: impl FnOnce(EntityWorldMut)) {
        let world = self.app.world_mut();
        let pet = world
            .query_filtered::<Entity, With<Pet>>()
            .single(world)
            .expect("one Pet");
        change(world.entity_mut(pet));
    }

    /// Draws a frame, with no time passing, and returns its pixels. A readback arrives one update after it's asked
    /// for, so this waits up to a few updates.
    pub fn capture(&mut self) -> Picture {
        let slot: Arc<Mutex<Option<Vec<u8>>>> = Arc::default();
        let write = slot.clone();
        let readback = self
            .app
            .world_mut()
            .spawn(Readback::texture(self.image.clone()))
            .observe(move |event: On<ReadbackComplete>| {
                write
                    .lock()
                    .unwrap()
                    .get_or_insert_with(|| event.data.clone());
            })
            .id();
        for _ in 0..8 {
            self.step();
            if slot.lock().unwrap().is_some() {
                break;
            }
        }
        self.app.world_mut().despawn(readback);
        let padded = slot
            .lock()
            .unwrap()
            .take()
            .expect("a readback within 8 updates");
        Picture {
            width: self.width,
            height: self.height,
            rgba: unpad(&padded, self.width, self.height),
        }
    }
}

/// A readback pads each row to a multiple of 256 bytes.
fn unpad(padded: &[u8], width: u32, height: u32) -> Vec<u8> {
    let row = (width * 4) as usize;
    let stride = row.div_ceil(256) * 256;
    assert_eq!(padded.len(), stride * height as usize, "readback size");
    padded
        .chunks(stride)
        .flat_map(|chunk| chunk[..row].iter().copied())
        .collect()
}
