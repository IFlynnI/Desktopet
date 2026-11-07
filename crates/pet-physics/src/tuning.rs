//! The physics feel numbers that hold for the whole app rather than for one Body or one surface.

use crate::grab::GrabString;
use crate::material::Material;

/// The app-wide feel numbers for physics, from `balance.toml`. It has no default here: the defaults live with the
/// config, so there is one place to tune them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tuning {
    /// In physical pixels per second squared, downwards.
    pub gravity: f32,
    /// The fastest a Body leaves the owner's hand, in physical pixels per second.
    pub max_release_speed: f32,
    /// How grippy the Ground plane and the screen edges are.
    pub ground_friction: f32,
    pub grab: GrabString,
}

impl Tuning {
    /// The Material of the Ground plane and the screen edges, for `World::surface`. They never bounce of their own
    /// accord, so how bouncy a landing is comes from the Body.
    pub fn surface(&self) -> Material {
        Material {
            friction: self.ground_friction,
            bounciness: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_a_tuning_when_asked_for_the_surface_then_it_grips_by_the_ground_friction_and_never_bounces()
     {
        // Given
        let tuning = Tuning {
            gravity: 3000.0,
            max_release_speed: 2500.0,
            ground_friction: 0.7,
            grab: GrabString {
                length: 24.0,
                stiffness: 400.0,
                damping: 30.0,
                swing_drag: 4.0,
            },
        };

        // When
        let surface = tuning.surface();

        // Then
        assert_eq!(
            surface,
            Material {
                friction: 0.7,
                bounciness: 0.0
            }
        );
    }
}
