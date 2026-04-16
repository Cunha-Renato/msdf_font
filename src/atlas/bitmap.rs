use crate::bitmap::{BitmapData, GlyphBitmapData};

pub(super) struct BitmapDataRegion<'a, const N: usize> {
    pub(super) data: &'a mut GlyphBitmapData<u8, N>,
    pub(super) x: usize,
    pub(super) y: usize,
    pub(super) width: usize,
    pub(super) height: usize,
}
impl<const N: usize> BitmapData for BitmapDataRegion<'_, N> {
    type Pixel = [u8; N];

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
        self.data.set_px(px, self.x + x, self.y + y);
    }

    #[inline]
    fn get_px(&self, x: usize, y: usize) -> Self::Pixel {
        self.data.get_px(self.x + x, self.y + y)
    }

    #[inline]
    fn iter_row(&self, y: usize) -> impl Iterator<Item = &Self::Pixel> {
        self.data.ir(self.x, y + self.y, self.width)
    }

    #[inline]
    fn iter_row_mut(&mut self, y: usize) -> impl Iterator<Item = &mut Self::Pixel> {
        self.data.irm(self.x, y + self.y, self.width)
    }
}
