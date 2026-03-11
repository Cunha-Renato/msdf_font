/// Represents the channes of the bitmap.
#[derive(Debug, Clone, Copy)]
pub enum BitmapImageType {
    L8,
    Rgb8,
}

pub trait BitmapData {
    /// Returns the width in pixels.
    fn width(&self) -> usize;
    /// Returns the height in pixels.
    fn height(&self) -> usize;
    /// Sets the pixel at (x, y) with the [px] value.
    fn set_px(&mut self, px: &[u8], x: usize, y: usize);
    /// Gets the pixel at (x, y).
    fn get_px(&self, x: usize, y: usize, f: impl FnOnce(&[u8]));
}

/// Struct representing the bitmap data.
#[derive(Debug)]
pub struct GlyphBitmapData {
    pub bytes: Vec<u8>,
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
    pub image_type: BitmapImageType,
}
impl GlyphBitmapData {
    pub(crate) fn new(width: usize, height: usize, image_type: BitmapImageType) -> Self {
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
}
impl BitmapData for GlyphBitmapData {
    #[inline]
    fn width(&self) -> usize {
        self.width
    }

    #[inline]
    fn height(&self) -> usize {
        self.height
    }

    fn set_px(&mut self, px: &[u8], mut x: usize, mut y: usize) {
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

    fn get_px(&self, mut x: usize, mut y: usize, f: impl FnOnce(&[u8])) {
        match self.image_type {
            BitmapImageType::L8 => f(&[self.bytes[y * self.width + x]]),
            BitmapImageType::Rgb8 => {
                x *= 3;
                y *= self.width * 3;

                f(&self.bytes[(y + x)..(y + x + 3)]);
            }
        }
    }
}
