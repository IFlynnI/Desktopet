//! Screen geometry in physical pixels.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }

    /// Top-left corner of a `width` x `height` box centred along this rect's bottom edge.
    pub fn bottom_centre(&self, width: i32, height: i32) -> Point {
        Point {
            x: self.left + (self.width() - width) / 2,
            y: self.bottom - height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_a_rect_when_asked_for_its_size_then_width_and_height_are_right_minus_left_and_bottom_minus_top()
     {
        // Given
        let rect = Rect {
            left: 10,
            top: 20,
            right: 110,
            bottom: 220,
        };

        // When
        let (width, height) = (rect.width(), rect.height());

        // Then
        assert_eq!(width, 100);
        assert_eq!(height, 200);
    }

    #[test]
    fn given_a_work_area_when_centring_a_box_at_the_bottom_then_it_is_centred_and_rests_on_the_bottom_edge()
     {
        // Given
        let work_area = Rect {
            left: 0,
            top: 0,
            right: 3840,
            bottom: 2100,
        };

        // When
        let top_left = work_area.bottom_centre(48, 80);

        // Then
        assert_eq!(top_left, Point { x: 1896, y: 2020 });
    }

    #[test]
    fn given_a_work_area_away_from_the_origin_when_centring_at_the_bottom_then_the_offset_is_kept()
    {
        // Given: a taskbar on the left or top moves the work area's origin
        let work_area = Rect {
            left: 100,
            top: 50,
            right: 1100,
            bottom: 1050,
        };

        // When
        let top_left = work_area.bottom_centre(100, 200);

        // Then
        assert_eq!(top_left, Point { x: 550, y: 850 });
    }
}
