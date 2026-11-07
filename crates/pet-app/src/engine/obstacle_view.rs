//! Draws each Obstacle as a faint outline, so the owner can see the walls and shelves they set in `obstacles.toml`.
//! Outlines are never part of the Hit target, so clicks pass through them to whatever is behind. It only reads the
//! simulation, and nothing flows back.

use bevy::prelude::*;
use pet_core::geometry::{Point, Rect};
use pet_sim::Obstacles;

use super::overlay::{OverlayRect, sprite_translation};
use super::pixel_art::{self as drawing, UnitQuad};

/// Behind the Pet, drawn at 0, and its shadow, at -1.
const BEHIND_THE_SHADOW: f32 = -2.0;

/// How thick an outline is, in physical pixels.
pub const OUTLINE: i32 = 2;

/// White, mostly see-through: there to find, not to look at.
pub const OUTLINE_COLOUR: Color = Color::srgba(1.0, 1.0, 1.0, 0.35);

pub struct ObstacleViewPlugin;

impl Plugin for ObstacleViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            draw.run_if(
                resource_exists::<Obstacles>.and_then(
                    resource_changed::<Obstacles>.or_eager(resource_changed::<OverlayRect>),
                ),
            ),
        );
    }
}

/// One side of an Obstacle's outline.
#[derive(Component, Debug)]
pub struct OutlineSide;

/// The sides of `rect`'s outline, inside its edges, so the outline shows exactly where the Obstacle is solid. One
/// too thin to have a hole inside is filled instead.
pub fn sides(rect: Rect) -> Vec<Rect> {
    if rect.width() <= 2 * OUTLINE || rect.height() <= 2 * OUTLINE {
        return vec![rect];
    }
    vec![
        Rect {
            bottom: rect.top + OUTLINE,
            ..rect
        },
        Rect {
            top: rect.bottom - OUTLINE,
            ..rect
        },
        Rect {
            top: rect.top + OUTLINE,
            bottom: rect.bottom - OUTLINE,
            right: rect.left + OUTLINE,
            ..rect
        },
        Rect {
            top: rect.top + OUTLINE,
            bottom: rect.bottom - OUTLINE,
            left: rect.right - OUTLINE,
            ..rect
        },
    ]
}

/// Draws every outline again: Obstacles change only at startup, and the Overlay only when the work area does.
fn draw(
    mut commands: Commands,
    obstacles: Res<Obstacles>,
    overlay: Res<OverlayRect>,
    quad: Res<UnitQuad>,
    drawn: Query<Entity, With<OutlineSide>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for side in &drawn {
        commands.entity(side).despawn();
    }
    if obstacles.0.is_empty() {
        return;
    }
    let material = materials.add(ColorMaterial::from_color(OUTLINE_COLOUR));
    for obstacle in &obstacles.0 {
        for side in sides(obstacle.rect) {
            let (width, height) = (side.width(), side.height());
            let top_left = Point {
                x: side.left,
                y: side.top,
            };
            let translation =
                sprite_translation(overlay.0, top_left, width, height).with_z(BEHIND_THE_SHADOW);
            commands.spawn((
                OutlineSide,
                drawing::drawable(
                    &quad,
                    material.clone(),
                    drawing::transform(translation, width, height, false),
                ),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_an_obstacle_when_outlining_it_then_the_sides_cover_its_edge_inside_it_without_overlapping()
     {
        // Given
        let rect = Rect {
            left: 100,
            top: 200,
            right: 140,
            bottom: 700,
        };

        // When
        let sides = sides(rect);

        // Then: every side is inside the Obstacle, and together they cover each edge pixel exactly once
        let area: i32 = sides.iter().map(|side| side.width() * side.height()).sum();
        let ring = rect.width() * rect.height()
            - (rect.width() - 2 * OUTLINE) * (rect.height() - 2 * OUTLINE);
        assert_eq!(area, ring);
        assert!(sides.iter().all(|side| rect.covers(*side)));
    }

    #[test]
    fn given_an_obstacle_thinner_than_two_outlines_when_outlining_it_then_it_is_filled_and_never_drawn_outside_it()
     {
        // Given: a 3 pixel wall
        let rect = Rect {
            left: 100,
            top: 200,
            right: 103,
            bottom: 700,
        };

        // When
        let sides = sides(rect);

        // Then
        assert_eq!(sides, [rect]);
    }
}
