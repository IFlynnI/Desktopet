//! How everything on the Overlay is drawn: one unit quad, scaled and placed by its `Transform`, with a `ColorMaterial`
//! holding the image. The Pet, its shadow and, from slice 4, Emotes all go this way.
//!
//! Not Bevy's `Sprite`. On DX12 its vertex shader reads the instance index twice offset, so every draw call after the
//! first in a frame picks up another drawable's place, or none. The default 2D mesh shader that `ColorMaterial` uses
//! reads it correctly. See "Sprites on DX12" in docs/research/simulation-architecture.md.

use bevy::prelude::*;

/// Makes the `UnitQuad` everything is drawn with.
pub struct PixelArtPlugin;

impl Plugin for PixelArtPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UnitQuad>();
    }
}

/// The one quad every drawable shares: 1 x 1, its image the right way up.
#[derive(Resource)]
pub struct UnitQuad(Handle<Mesh>);

impl FromWorld for UnitQuad {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        Self(meshes.add(Rectangle::new(1.0, 1.0)))
    }
}

/// An image ready to draw as it is: untinted, alpha blended.
pub fn material(
    image: Handle<Image>,
    materials: &mut Assets<ColorMaterial>,
) -> Handle<ColorMaterial> {
    materials.add(ColorMaterial {
        texture: Some(image),
        ..default()
    })
}

/// Where a drawable `width` x `height` screen pixels goes: centred on `translation`, mirrored left to right when
/// `flip_x`.
pub fn transform(translation: Vec3, width: i32, height: i32, flip_x: bool) -> Transform {
    let x_scale = if flip_x { -width } else { width };
    Transform::from_translation(translation).with_scale(Vec3::new(
        x_scale as f32,
        height as f32,
        1.0,
    ))
}

/// What a drawable entity needs.
pub fn drawable(
    quad: &UnitQuad,
    material: Handle<ColorMaterial>,
    transform: Transform,
) -> impl Bundle {
    (
        Mesh2d(quad.0.clone()),
        MeshMaterial2d(material),
        transform,
        Visibility::default(),
    )
}
