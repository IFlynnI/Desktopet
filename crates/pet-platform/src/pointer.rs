//! Where the cursor was, sampled by the hit-test thread, for the engine to read once a tick.

use std::collections::VecDeque;
use std::time::Instant;

use pet_core::geometry::Point;

/// One reading of the cursor, in physical screen pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PointerSample {
    /// When it was read. Readings are about 8 ms apart, but not exactly, so speeds should come from these times.
    pub at: Instant,
    pub position: Point,
    /// Whether the cursor was over the Pet's drawn pixels, by the Hit target. Holding the Pet doesn't change this.
    pub over_pet: bool,
}

/// The samples the engine hasn't read yet. It keeps only the newest few, so a slow engine never makes it grow.
#[derive(Debug, Default)]
pub(crate) struct PointerBuffer {
    samples: VecDeque<PointerSample>,
}

impl PointerBuffer {
    /// About half a second of samples, more than a tick ever needs.
    pub const CAPACITY: usize = 64;

    /// Adds the newest sample, dropping the oldest when the buffer is full.
    pub fn push(&mut self, sample: PointerSample) {
        if self.samples.len() == Self::CAPACITY {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }

    /// Hands over every sample, oldest first, and leaves the buffer empty.
    pub fn drain(&mut self) -> Vec<PointerSample> {
        self.samples.drain(..).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn sample(start: Instant, index: usize) -> PointerSample {
        PointerSample {
            at: start + Duration::from_millis(8 * index as u64),
            position: Point {
                x: index as i32,
                y: 0,
            },
            over_pet: false,
        }
    }

    #[test]
    fn given_samples_pushed_when_draining_then_they_come_out_oldest_first() {
        // Given
        let start = Instant::now();
        let mut buffer = PointerBuffer::default();
        let pushed: Vec<_> = (0..3).map(|index| sample(start, index)).collect();
        for &one in &pushed {
            buffer.push(one);
        }

        // When
        let drained = buffer.drain();

        // Then
        assert_eq!(drained, pushed);
    }

    #[test]
    fn given_a_full_buffer_when_pushing_another_then_the_oldest_is_dropped() {
        // Given
        let start = Instant::now();
        let mut buffer = PointerBuffer::default();
        for index in 0..PointerBuffer::CAPACITY {
            buffer.push(sample(start, index));
        }

        // When
        buffer.push(sample(start, PointerBuffer::CAPACITY));

        // Then
        let drained = buffer.drain();
        assert_eq!(drained.len(), PointerBuffer::CAPACITY);
        assert_eq!(drained.first(), Some(&sample(start, 1)));
        assert_eq!(
            drained.last(),
            Some(&sample(start, PointerBuffer::CAPACITY))
        );
    }

    #[test]
    fn given_a_drained_buffer_when_draining_again_then_nothing_is_left() {
        // Given
        let start = Instant::now();
        let mut buffer = PointerBuffer::default();
        buffer.push(sample(start, 0));
        let first = buffer.drain();

        // When
        let second = buffer.drain();

        // Then
        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
    }
}
