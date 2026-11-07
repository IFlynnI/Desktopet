//! Screen geometry in physical pixels, owned here and re-exported by `pet-core`.

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

    /// Whether every part of `other` is inside this rect.
    pub fn covers(&self, other: Rect) -> bool {
        self.left <= other.left
            && self.top <= other.top
            && self.right >= other.right
            && self.bottom >= other.bottom
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MONITOR: Rect = Rect {
        left: 0,
        top: 0,
        right: 100,
        bottom: 100,
    };

    #[test]
    fn given_a_rect_the_same_as_another_when_comparing_then_it_covers_it() {
        // Given
        let rect = MONITOR;

        // When
        let covers = rect.covers(MONITOR);

        // Then
        assert!(covers);
    }

    #[test]
    fn given_a_rect_short_of_any_edge_when_comparing_then_it_does_not_cover_it() {
        // Given
        let short_of_each_edge = [
            Rect { left: 1, ..MONITOR },
            Rect { top: 1, ..MONITOR },
            Rect {
                right: 99,
                ..MONITOR
            },
            Rect {
                bottom: 99,
                ..MONITOR
            },
        ];

        // When
        let covered = short_of_each_edge.map(|rect| rect.covers(MONITOR));

        // Then
        assert_eq!(covered, [false; 4]);
    }

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
}
