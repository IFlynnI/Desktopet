//! A procedural stand-in for the Pet, so nothing here needs asset files until content packs arrive in
//! item 3. It stands still and has holes (between the legs, between tail and body) so click-through has
//! something real to prove.

use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use pet_core::geometry::Rect;
use pet_core::hit_mask::{HitMask, Placement};
use pet_platform::HitTarget;

use super::overlay::{OverlayRect, PlatformHandle, sprite_translation};

const ART: &[&str] = &[
    "      oo        oo      ",
    "     oyyo      oyyo     ",
    "     oyyoooooooyyo      ",
    "    oggggggggggggo      ",
    "   oggggggggggggggo     ",
    "   oggwwggggggwwggo     ",
    "   oggwkggggggwkggo  oo ",
    "   oggggggggggggggo ogo ",
    "   ogggggorrogggggo ogo ",
    "    oggggoooogggggoogo  ",
    "   oolllllllllllgggggo  ",
    "  ogolllllllllllgooo    ",
    "  ogolllllllllllgo      ",
    "   oolllllllllllgo      ",
    "    ogggggggggggo       ",
    "    oggo    oggo        ",
    "    oggo    oggo        ",
    "    oooo    oooo        ",
];

/// Rows at the bottom that stay planted while the body bobs.
const FEET_ROWS: usize = 3;

/// The default integer scale (REN-01).
const SCALE: u32 = 2;

const FRAME_SECONDS: f32 = 0.25;

/// A pixel counts as opaque from this alpha up.
const ALPHA_THRESHOLD: u8 = 128;

struct Art {
    width: u32,
    height: u32,
    /// RGBA8, one entry per frame.
    frames: Vec<Vec<u8>>,
}

impl Art {
    fn hit_masks(&self) -> Vec<HitMask> {
        let mask =
            |rgba: &Vec<u8>| HitMask::from_rgba(self.width, self.height, rgba, ALPHA_THRESHOLD);
        self.frames.iter().map(mask).collect()
    }
}

fn colour(pixel: char) -> [u8; 4] {
    match pixel {
        'o' => [34, 32, 52, 255],
        'g' => [106, 190, 48, 255],
        'l' => [238, 195, 154, 255],
        'w' => [255, 255, 255, 255],
        'k' => [20, 20, 28, 255],
        'y' => [251, 242, 54, 255],
        'r' => [217, 87, 99, 255],
        _ => [0, 0, 0, 0],
    }
}

fn art() -> Art {
    let width = ART.iter().map(|row| row.chars().count()).max().unwrap_or(0);
    let height = ART.len();
    let rows: Vec<Vec<char>> = ART
        .iter()
        .map(|row| {
            let mut pixels: Vec<char> = row.chars().collect();
            pixels.resize(width, ' ');
            pixels
        })
        .collect();

    // Second frame: the body squashes down one pixel while the feet stay put.
    let body_end = height - FEET_ROWS;
    let mut bob = vec![vec![' '; width]];
    bob.extend(rows[..body_end - 1].iter().cloned());
    bob.extend(rows[body_end..].iter().cloned());

    let to_rgba = |grid: &[Vec<char>]| -> Vec<u8> {
        grid.iter()
            .flat_map(|row| row.iter().flat_map(|&pixel| colour(pixel)))
            .collect()
    };
    Art {
        width: width as u32,
        height: height as u32,
        frames: vec![to_rgba(&rows), to_rgba(&bob)],
    }
}

#[derive(Resource)]
struct Animation {
    frame: usize,
    frame_count: usize,
    timer: Timer,
}

impl Animation {
    fn new(frame_count: usize) -> Self {
        let timer = Timer::from_seconds(FRAME_SECONDS, TimerMode::Repeating);
        Self {
            frame: 0,
            frame_count,
            timer,
        }
    }
}

#[derive(Resource)]
struct FrameAssets {
    images: Vec<Handle<Image>>,
    masks: Vec<Arc<HitMask>>,
    placement: Placement,
}

#[derive(Component)]
struct PlaceholderSprite;

/// Shows the placeholder standing at the bottom centre of `work_area`.
pub struct PlaceholderPlugin {
    pub work_area: Rect,
}

impl Plugin for PlaceholderPlugin {
    fn build(&self, app: &mut App) {
        let work_area = self.work_area;
        app.add_systems(
            Startup,
            move |commands: Commands, images: ResMut<Assets<Image>>, overlay: Res<OverlayRect>| {
                spawn(commands, images, *overlay, work_area)
            },
        )
        .add_systems(Update, (animate, show_frame, publish_hit_target).chain());
    }
}

fn spawn(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    overlay: OverlayRect,
    work_area: Rect,
) {
    let art = art();
    let (width, height) = ((art.width * SCALE) as i32, (art.height * SCALE) as i32);
    let top_left = work_area.bottom_centre(width, height);
    let frame_images: Vec<Handle<Image>> = art
        .frames
        .iter()
        .map(|rgba| {
            let mut image = Image::new(
                Extent3d {
                    width: art.width,
                    height: art.height,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                rgba.clone(),
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::RENDER_WORLD,
            );
            image.sampler = ImageSampler::nearest();
            images.add(image)
        })
        .collect();

    commands.spawn((
        PlaceholderSprite,
        Sprite {
            image: frame_images[0].clone(),
            custom_size: Some(Vec2::new(width as f32, height as f32)),
            ..default()
        },
        Transform::from_translation(sprite_translation(overlay.0, top_left, width, height)),
    ));
    commands.insert_resource(Animation::new(frame_images.len()));
    commands.insert_resource(FrameAssets {
        images: frame_images,
        masks: art.hit_masks().into_iter().map(Arc::new).collect(),
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
        animation.frame = (animation.frame + 1) % animation.frame_count;
    }
}

fn show_frame(
    animation: Res<Animation>,
    assets: Res<FrameAssets>,
    mut sprites: Query<&mut Sprite, With<PlaceholderSprite>>,
) {
    let Ok(mut sprite) = sprites.single_mut() else {
        return;
    };
    let image = &assets.images[animation.frame];
    if sprite.image != *image {
        sprite.image = image.clone();
    }
}

fn publish_hit_target(
    animation: Res<Animation>,
    assets: Res<FrameAssets>,
    platform: Option<Res<PlatformHandle>>,
    mut published: Local<Option<usize>>,
) {
    let Some(platform) = platform else { return };
    if *published != Some(animation.frame) {
        platform.0.publish_hit_target(HitTarget {
            mask: assets.masks[animation.frame].clone(),
            placement: assets.placement,
        });
        *published = Some(animation.frame);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::time::TimeUpdateStrategy;

    use super::*;

    #[test]
    fn given_the_placeholder_art_when_building_frames_then_every_frame_has_the_full_pixel_count() {
        // Given
        let art = art();
        let bytes = (art.width * art.height * 4) as usize;

        // When
        let sizes: Vec<usize> = art.frames.iter().map(Vec::len).collect();

        // Then
        assert!(sizes.len() >= 2);
        assert!(sizes.iter().all(|&size| size == bytes));
    }

    #[test]
    fn given_the_placeholder_art_when_building_hit_masks_then_a_hole_between_the_legs_lets_clicks_through()
     {
        // Given: legs start on row 15, with four clear sprite pixels between them from column 8
        let art = art();

        // When
        let masks = art.hit_masks();

        // Then
        assert!(masks.iter().all(|mask| !mask.is_opaque(9, 16)));
        assert!(masks.iter().all(|mask| mask.is_opaque(5, 16)));
    }

    #[test]
    fn given_a_running_animation_when_a_frame_duration_passes_then_the_next_frame_shows_and_it_wraps_around()
     {
        // Given
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
                130,
            )))
            .insert_resource(Animation::new(2))
            .add_systems(Update, animate);
        app.update();
        let start = app.world().resource::<Animation>().frame;

        // When: 130 ms per update crosses the 250 ms frame duration every second update
        app.update();
        app.update();
        let next = app.world().resource::<Animation>().frame;
        app.update();
        app.update();
        let wrapped = app.world().resource::<Animation>().frame;

        // Then
        assert_eq!(next, (start + 1) % 2);
        assert_eq!(wrapped, start);
    }
}
