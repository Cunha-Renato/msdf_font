use core::f64;
use glam::DVec2;

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
