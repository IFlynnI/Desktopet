//! Reads real pixels off the screen and finds the Pet in them.

use std::ffi::c_void;
use std::thread;
use std::time::Duration;

use pet_core::geometry::{Point, Rect};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, CreateCompatibleBitmap,
    CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC,
    SRCCOPY, SelectObject,
};

use crate::test_window::BACKGROUND;

/// Screen pixels per sprite pixel at the default scale.
const SPRITE_PIXEL: usize = 2;
const SWEEP_MARGIN: i32 = 2;

struct Shot {
    /// BGRA8, top row first.
    bgra: Vec<u8>,
}

fn capture(area: Rect) -> Shot {
    let (width, height) = (area.width(), area.height());
    let mut bgra = vec![0u8; (width * height * 4) as usize];
    // SAFETY: every handle is created and released here on one thread, the old object is reselected
    // before the bitmap is deleted, and `bgra` holds exactly the rows GetDIBits is told to write.
    unsafe {
        let screen = GetDC(None);
        let memory = CreateCompatibleDC(Some(screen));
        let bitmap = CreateCompatibleBitmap(screen, width, height);
        let previous = SelectObject(memory, bitmap.into());
        let _ = BitBlt(
            memory,
            0,
            0,
            width,
            height,
            Some(screen),
            area.left,
            area.top,
            SRCCOPY | CAPTUREBLT,
        );
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // negative means top row first
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        GetDIBits(
            memory,
            bitmap,
            0,
            height as u32,
            Some(bgra.as_mut_ptr() as *mut c_void),
            &mut info,
            DIB_RGB_COLORS,
        );
        SelectObject(memory, previous);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(memory);
        ReleaseDC(None, screen);
    }
    Shot { bgra }
}

impl Shot {
    fn is_background(&self, index: usize) -> bool {
        let px = &self.bgra[index * 4..index * 4 + 3];
        [px[2], px[1], px[0]] == BACKGROUND
    }
}

/// Which window should be under the cursor at a point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expected {
    Overlay,
    WindowBehind,
}

/// The Pet as seen on screen over a test window, across its animation frames.
pub struct PetView {
    area: Rect,
    /// Not background in every capture.
    always_pet: Vec<bool>,
    /// Not background in at least one capture.
    ever_pet: Vec<bool>,
}

impl PetView {
    /// Captures `area` across more than one animation cycle.
    pub fn observe(area: Rect) -> Self {
        let count = (area.width() * area.height()) as usize;
        let (mut always_pet, mut ever_pet) = (vec![true; count], vec![false; count]);
        for _ in 0..8 {
            let shot = capture(area);
            for i in 0..count {
                let pet = !shot.is_background(i);
                always_pet[i] &= pet;
                ever_pet[i] |= pet;
            }
            thread::sleep(Duration::from_millis(100));
        }
        Self {
            area,
            always_pet,
            ever_pet,
        }
    }

    pub fn pet_pixels(&self) -> usize {
        self.ever_pet.iter().filter(|&&pet| pet).count()
    }

    pub fn background_share(&self) -> f64 {
        1.0 - self.pet_pixels() as f64 / self.ever_pet.len() as f64
    }

    fn index(&self, x: i32, y: i32) -> usize {
        (y * self.area.width() + x) as usize
    }

    /// Bounding box of everything that's ever Pet, in screen coordinates.
    pub fn bounds(&self) -> Option<Rect> {
        let (width, height) = (self.area.width(), self.area.height());
        let cells = (0..height).flat_map(|y| (0..width).map(move |x| (x, y)));
        let pet: Vec<(i32, i32)> = cells
            .filter(|&(x, y)| self.ever_pet[self.index(x, y)])
            .collect();
        let left = pet.iter().map(|p| p.0).min()?;
        let right = pet.iter().map(|p| p.0).max()?;
        let top = pet.iter().map(|p| p.1).min()?;
        let bottom = pet.iter().map(|p| p.1).max()?;
        Some(Rect {
            left: self.area.left + left,
            top: self.area.top + top,
            right: self.area.left + right + 1,
            bottom: self.area.top + bottom + 1,
        })
    }

    /// A screen point on the Pet in every frame, away from its edges.
    pub fn solid_point(&self) -> Option<Point> {
        self.find(|view, x, y| view.square(x, y, 2, |i| view.always_pet[i]))
    }

    /// A screen point inside the Pet's bounds where it's transparent in every frame, such as between
    /// its legs.
    pub fn hole_point(&self) -> Option<Point> {
        let bounds = self.bounds()?;
        self.find(|view, x, y| {
            let (sx, sy) = (view.area.left + x, view.area.top + y);
            let inside = sx > bounds.left + 2
                && sx < bounds.right - 3
                && sy > bounds.top + 2
                && sy < bounds.bottom - 3;
            inside && view.square(x, y, 2, |i| !view.ever_pet[i])
        })
    }

    /// One screen pixel per sprite pixel across the Pet and just past its edges, each with the window
    /// that should catch it. Pixels that change between animation frames have no single right answer,
    /// so they're left out.
    pub fn sweep(&self) -> Vec<(Point, Expected)> {
        let Some(bounds) = self.bounds() else {
            return Vec::new();
        };
        let (left, right) = (bounds.left - SWEEP_MARGIN, bounds.right + SWEEP_MARGIN);
        let (top, bottom) = (
            bounds.top - SWEEP_MARGIN,
            (bounds.bottom + SWEEP_MARGIN).min(self.area.bottom),
        );
        let mut points = Vec::new();
        for y in (top..bottom).step_by(SPRITE_PIXEL) {
            for x in (left..right).step_by(SPRITE_PIXEL) {
                let i = self.index(x - self.area.left, y - self.area.top);
                if self.always_pet[i] {
                    points.push((Point { x, y }, Expected::Overlay));
                } else if !self.ever_pet[i] {
                    points.push((Point { x, y }, Expected::WindowBehind));
                }
            }
        }
        points
    }

    fn find(&self, matches: impl Fn(&Self, i32, i32) -> bool) -> Option<Point> {
        let (width, height) = (self.area.width(), self.area.height());
        (2..height - 2)
            .flat_map(|y| (2..width - 2).map(move |x| (x, y)))
            .find(|&(x, y)| matches(self, x, y))
            .map(|(x, y)| Point {
                x: self.area.left + x,
                y: self.area.top + y,
            })
    }

    fn square(&self, x: i32, y: i32, radius: i32, check: impl Fn(usize) -> bool) -> bool {
        (-radius..=radius).all(|dy| (-radius..=radius).all(|dx| check(self.index(x + dx, y + dy))))
    }
}
