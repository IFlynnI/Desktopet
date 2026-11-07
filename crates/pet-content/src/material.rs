//! A Material as a Pack's Forms and the owner's Obstacles write it: `{ friction = 0.6, bounciness = 0.1 }`. Both values
//! are given, so a Material in a file always says what it is, and each is a share between 0 and 1.

use pet_core::physics::Material;
use toml::Value;

use crate::fields::{Fields, Mistake};

/// Reads the shape. The values are judged by `check`.
pub fn read(value: &Value, path: String, errors: &mut Vec<Mistake>) -> Option<Material> {
    let mut fields = Fields::read(value, path, &["friction", "bounciness"], errors)?;
    let friction = fields.required("friction");
    let bounciness = fields.required("bounciness");
    Some(Material {
        friction: friction?,
        bounciness: bounciness?,
    })
}

/// Checks each value is between 0 and 1.
pub fn check(path: &str, material: Material, errors: &mut Vec<Mistake>) {
    check_share(&format!("{path}.friction"), material.friction, errors);
    check_share(&format!("{path}.bounciness"), material.bounciness, errors);
}

/// Checks `value` is between 0 and 1, both included. That also rules out `nan` and `inf`, which TOML allows.
pub fn check_share(path: &str, value: f32, errors: &mut Vec<Mistake>) {
    if !(0.0..=1.0).contains(&value) {
        errors.push(Mistake::new(
            path,
            format!("is {value}, but must be between 0 and 1"),
        ));
    }
}
