//! How grippy and how bouncy a surface or a Body is.

/// Friction slows sliding along a surface: 0 is ice, 1 grips fully. Bounciness is the share of approach speed kept
/// after a hit: 0 sticks, 1 bounces fully. Both are between 0 and 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    pub friction: f32,
    pub bounciness: f32,
}

/// Values an Action or a Situation sets while it runs. Each one that is set wins over both Materials and the combine
/// rule, so a landing that never bounces works even on a bouncy floor.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MaterialOverride {
    pub friction: Option<f32>,
    pub bounciness: Option<f32>,
}

/// The Material a contact between a Body and a surface uses. Friction is averaged, so a grippy Body still slides a
/// little on ice. The bouncier of the two wins, so a bouncy prop bounces any Body. Each value the Body's override
/// sets wins outright, so an Action's choice never depends on what it lands on.
pub fn combine(body: Material, surface: Material, overrides: MaterialOverride) -> Material {
    Material {
        friction: overrides
            .friction
            .unwrap_or((body.friction + surface.friction) * 0.5),
        bounciness: overrides
            .bounciness
            .unwrap_or(body.bounciness.max(surface.bounciness)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRIPPY_DEAD: Material = Material {
        friction: 0.8,
        bounciness: 0.0,
    };

    const SLIPPERY_BOUNCY: Material = Material {
        friction: 0.2,
        bounciness: 0.9,
    };

    #[test]
    fn given_two_materials_when_combined_then_friction_is_their_average() {
        // Given
        let (body, surface) = (GRIPPY_DEAD, SLIPPERY_BOUNCY);

        // When
        let contact = combine(body, surface, MaterialOverride::default());

        // Then
        assert!((contact.friction - 0.5).abs() < 1e-6);
    }

    #[test]
    fn given_two_materials_when_combined_then_the_bouncier_one_wins() {
        // Given
        let (body, surface) = (GRIPPY_DEAD, SLIPPERY_BOUNCY);

        // When
        let contact = combine(body, surface, MaterialOverride::default());
        let swapped = combine(surface, body, MaterialOverride::default());

        // Then
        assert_eq!(contact.bounciness, 0.9);
        assert_eq!(swapped.bounciness, 0.9);
    }

    #[test]
    fn given_an_override_when_combined_then_each_value_it_sets_wins_outright() {
        // Given
        let overrides = MaterialOverride {
            friction: Some(0.05),
            bounciness: Some(0.0),
        };

        // When
        let contact = combine(SLIPPERY_BOUNCY, SLIPPERY_BOUNCY, overrides);

        // Then
        assert_eq!(contact.friction, 0.05);
        assert_eq!(contact.bounciness, 0.0);
    }

    #[test]
    fn given_an_override_of_one_value_when_combined_then_the_other_follows_the_rule() {
        // Given
        let overrides = MaterialOverride {
            friction: None,
            bounciness: Some(0.0),
        };

        // When
        let contact = combine(GRIPPY_DEAD, SLIPPERY_BOUNCY, overrides);

        // Then
        assert!((contact.friction - 0.5).abs() < 1e-6);
        assert_eq!(contact.bounciness, 0.0);
    }
}
