pub trait BitmapData {
    type Pixel;

    /// Returns the width in pixels.
    fn width(&self) -> usize;
    /// Returns the height in pixels.
    fn height(&self) -> usize;
    /// Sets the pixel at (x, y) with the [px] value.
    fn set_px(&mut self, px: Self::Pixel, x: usize, y: usize);
    /// Gets the pixel at (x, y).
    fn get_px(&self, x: usize, y: usize) -> Self::Pixel;
}

/// Struct representing the bitmap data.
#[derive(Debug)]
pub struct GlyphBitmapData<T: Copy, const N: usize> {
    bytes: Vec<[T; N]>,
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
}
impl<const N: usize> GlyphBitmapData<u8, N> {
    #[inline]
    pub fn bytes(&self) -> &[u8] {
        self.bytes.as_flattened()
    }

    #[inline]
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        self.bytes.as_flattened_mut()
    }
}
impl<T: Default + Copy, const N: usize> GlyphBitmapData<T, N> {
    pub(crate) fn new(width: usize, height: usize) -> Self {
        Self {
            bytes: vec![[T::default(); N]; width * height],
            width,
            height,
        }
    }
}
impl<T: Copy, const N: usize> BitmapData for GlyphBitmapData<T, N> {
    type Pixel = [T; N];

    #[inline]
    fn width(&self) -> usize {
        self.width
    }

    #[inline]
    fn height(&self) -> usize {
        self.height
    }

    #[inline]
    fn set_px(&mut self, px: Self::Pixel, x: usize, y: usize) {
        self.bytes[y * self.width + x] = px;
    }

    #[inline]
    fn get_px(&self, x: usize, y: usize) -> Self::Pixel {
        self.bytes[y * self.width + x]
    }
}
