use core::f64;
use glam::DVec2;
use std::cmp::Ordering;

#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
pub enum FieldType {
    Msdf(f32),
    #[default]
    Sdf,
}

#[derive(Debug, Clone, Copy)]
pub enum BitmapImageType {
    L8,
    Rgb8,
}

#[derive(Debug)]
pub(crate) struct BitmapDataBuilder {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) image_type: BitmapImageType,
}
impl BitmapDataBuilder {
    #[inline]
    pub(crate) fn build(self) -> BitmapData {
        BitmapData::new(self.width, self.height, self.image_type)
    }
}

#[derive(Debug)]
pub struct BitmapData {
    pub bytes: Vec<u8>,
    pub width: usize,
    pub height: usize,
    pub image_type: BitmapImageType,
}
impl BitmapData {
    fn new(width: usize, height: usize, image_type: BitmapImageType) -> Self {
        let channels = match image_type {
            BitmapImageType::L8 => 1,
            BitmapImageType::Rgb8 => 3,
        };

        Self {
            bytes: vec![0u8; width * height * channels],
            width,
            height,
            image_type,
        }
    }

    pub fn set_px(&mut self, px: &[u8], mut x: usize, mut y: usize) {
        match self.image_type {
            BitmapImageType::L8 => self.bytes[y * self.width + x] = px[0],
            BitmapImageType::Rgb8 => {
                x *= 3;
                y *= self.width * 3;

                self.bytes[y + x] = px[0];
                self.bytes[y + x + 1] = px[1];
                self.bytes[y + x + 2] = px[2];
            }
        }
    }

    pub fn get_px(&self, mut x: usize, mut y: usize, f: impl FnOnce(&[u8])) {
        match self.image_type {
            BitmapImageType::L8 => f(&[self.bytes[y * self.width + x]]),
            BitmapImageType::Rgb8 => {
                x *= 3;
                y *= self.width * 3;

                f(&self.bytes[(y + x)..=(y + x + 2)]);
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct GenerationConfig {
    pub(crate) px_range: f64,
    pub(crate) offset: DVec2,
    pub(crate) bitmap_size: (usize, usize),
    pub(crate) overlapping: bool,
    pub(crate) field_type: FieldType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SignedDistance {
    pub(crate) distance: f64,
    pub(crate) dot: f64,
}
impl Default for SignedDistance {
    fn default() -> Self {
        Self {
            distance: f64::INFINITY,
            dot: f64::NEG_INFINITY,
        }
    }
}
impl SignedDistance {
    #[inline]
    pub(crate) const fn new(distance: f64, dot: f64) -> Self {
        Self { distance, dot }
    }
}
impl PartialOrd for SignedDistance {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let a = self.distance.abs();
        let b = other.distance.abs();

        match a.partial_cmp(&b)? {
            Ordering::Equal => self.dot.partial_cmp(&other.dot),
            ord => Some(ord),
        }
    }
}

pub(crate) struct MultiDistance {
    pub(crate) r: f64,
    pub(crate) g: f64,
    pub(crate) b: f64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Bounds {
    pub(crate) min: DVec2,
    pub(crate) max: DVec2,
}
impl Bounds {
    #[inline]
    pub(crate) const fn new() -> Self {
        Self {
            min: DVec2::new(f64::INFINITY, f64::INFINITY),
            max: DVec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        }
    }

    #[inline]
    pub(crate) const fn size(self) -> DVec2 {
        DVec2::new(self.max.x - self.min.x, self.max.y - self.min.y)
    }
}
