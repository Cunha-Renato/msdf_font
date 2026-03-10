use crate::{BitmapData, GlyphBitmapData};

pub(super) struct BitmapDataRegion<'a> {
    pub(super) data: &'a mut GlyphBitmapData,
    pub(super) x: usize,
    pub(super) y: usize,
    pub(super) width: usize,
    pub(super) height: usize,
}
impl BitmapData for BitmapDataRegion<'_> {
    #[inline]
    fn width(&self) -> usize {
        self.width
    }

    #[inline]
    fn height(&self) -> usize {
        self.height
    }

    #[inline]
    fn set_px(&mut self, px: &[u8], x: usize, y: usize) {
        self.data.set_px(px, self.x + x, self.y + y);
    }

    #[inline]
    fn get_px(&self, x: usize, y: usize, f: impl FnOnce(&[u8])) {
        self.data.get_px(self.x + x, self.y + y, f);
    }
}
