//! Where a Frame is drawn on screen. Where the Pet is means where its Anchor is, so a Frame is placed by lining its
//! Anchor up with that point, at an integer scale, mirrored when the Pet faces the other way.

use crate::geometry::{Point, Rect};
use crate::hit_mask::Placement;
use crate::pack::FormDef;

/// Which way the Pet faces, compared with how its art is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Facing {
    #[default]
    AsDrawn,
    /// Flipped left to right, turning around the Anchor.
    Mirrored,
}

/// Where a Frame of `form` goes when its Anchor is at `anchor_at`, in physical pixels.
pub fn place(anchor_at: Point, form: &FormDef, scale: u32, facing: Facing) -> Placement {
    let flip_x = facing == Facing::Mirrored;
    let from_left = if flip_x {
        form.cell.width - form.anchor.x
    } else {
        form.anchor.x
    };
    Placement {
        top_left: Point {
            x: anchor_at.x - (from_left * scale) as i32,
            y: anchor_at.y - (form.anchor.y * scale) as i32,
        },
        scale,
        flip_x,
    }
}

/// The Ground plane's height on screen: the bottom of the work area, the top of the taskbar. Nothing is drawn below it.
pub fn ground(work_area: Rect) -> i32 {
    work_area.bottom
}

/// Where the Anchor stands until physics arrives in roadmap item 4: the bottom centre of the work area, on the Ground
/// plane.
pub fn home(work_area: Rect) -> Point {
    Point {
        x: work_area.left + work_area.width() / 2,
        y: ground(work_area),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hit_mask::{HitMask, hits};
    use crate::pack::{ActionDef, ActionId, Anchor, AnimationId, Body, CellSize};

    /// A 10 x 6 cell whose Anchor is off-centre, 3 from the left and on the bottom edge.
    fn form() -> FormDef {
        FormDef {
            name: "hatchling".to_owned(),
            cell: CellSize {
                width: 10,
                height: 6,
            },
            anchor: Anchor { x: 3, y: 6 },
            body: Body {
                x: -3,
                y: -6,
                width: 10,
                height: 6,
            },
            default_action: ActionId(0),
            shadow: None,
            actions: vec![ActionDef {
                name: "idle".to_owned(),
                animation: AnimationId(0),
                body: None,
            }],
            animations: Vec::new(),
            situations: Vec::new(),
        }
    }

    #[test]
    fn given_a_pet_facing_as_drawn_when_placing_a_frame_then_its_anchor_lands_on_the_point_at_that_scale()
     {
        // Given
        let anchor_at = Point { x: 100, y: 50 };

        // When
        let placement = place(anchor_at, &form(), 2, Facing::AsDrawn);

        // Then: the Anchor is 3 x 2 right of the top-left and 6 x 2 below it
        assert_eq!(
            placement,
            Placement {
                top_left: Point { x: 94, y: 38 },
                scale: 2,
                flip_x: false
            }
        );
    }

    #[test]
    fn given_a_mirrored_pet_when_placing_a_frame_then_it_turns_around_its_anchor() {
        // Given
        let anchor_at = Point { x: 100, y: 50 };

        // When
        let placement = place(anchor_at, &form(), 2, Facing::Mirrored);

        // Then: mirrored, the Anchor is 10 - 3 = 7 art pixels from the left edge
        assert_eq!(
            placement,
            Placement {
                top_left: Point { x: 86, y: 38 },
                scale: 2,
                flip_x: true
            }
        );
    }

    #[test]
    fn given_a_mirrored_pet_when_hit_testing_the_art_pixel_left_of_the_anchor_then_the_click_lands_right_of_it()
     {
        // Given: only art pixel (2, 5) is opaque, just left of the Anchor at x = 3 on the bottom row
        let mut rgba = vec![0u8; 10 * 6 * 4];
        let pixel = (5 * 10 + 2) * 4;
        rgba[pixel..pixel + 4].copy_from_slice(&[255; 4]);
        let mask = HitMask::from_rgba(10, 6, &rgba, 128);
        let anchor_at = Point { x: 100, y: 50 };

        // When
        let placement = place(anchor_at, &form(), 2, Facing::Mirrored);

        // Then: it covers screen x 100..=101 on the row just above the Anchor, not x 98..=99
        assert!(hits(&mask, &placement, Point { x: 100, y: 49 }));
        assert!(hits(&mask, &placement, Point { x: 101, y: 48 }));
        assert!(!hits(&mask, &placement, Point { x: 99, y: 49 }));
    }

    #[test]
    fn given_a_work_area_away_from_the_origin_when_finding_home_then_it_is_the_bottom_centre() {
        // Given: a taskbar on the left or top moves the work area's origin
        let work_area = Rect {
            left: 100,
            top: 50,
            right: 1101,
            bottom: 1050,
        };

        // When
        let home = home(work_area);

        // Then: an odd width rounds down
        assert_eq!(home, Point { x: 600, y: 1050 });
    }

    #[test]
    fn given_a_work_area_away_from_the_origin_when_finding_the_ground_then_it_is_the_bottom_edge_home_stands_on()
     {
        // Given
        let work_area = Rect {
            left: 100,
            top: 50,
            right: 1101,
            bottom: 1050,
        };

        // When
        let ground = ground(work_area);

        // Then
        assert_eq!(ground, 1050);
        assert_eq!(ground, home(work_area).y);
    }
}
