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
pub struct GlyphBitmapData<const N: usize> {
    bytes: Vec<[u8; N]>,
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
}
impl<const N: usize> GlyphBitmapData<N> {
    #[inline]
    pub fn bytes(&self) -> &[u8] {
        self.bytes.as_flattened()
    }

    pub(crate) fn new(width: usize, height: usize) -> Self {
        Self {
            bytes: vec![[0u8; N]; width * height],
            width,
            height,
        }
    }
}
impl<const N: usize> BitmapData for GlyphBitmapData<N> {
    type Pixel = [u8; N];

    #[inline]
    fn width(&self) -> usize {
        self.width
    }

    #[inline]
    fn height(&self) -> usize {
        self.height
    }

    fn set_px(&mut self, px: [u8; N], x: usize, y: usize) {
        let j = y * self.width + x;

        (0..N).for_each(|i| self.bytes[j][i] = px[i]);
    }

    #[inline]
    fn get_px(&self, x: usize, y: usize) -> [u8; N] {
        self.bytes[y * self.width + x]
    }
}
