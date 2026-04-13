//! Code taken from https://github.com/Chlumsky/msdfgen.

use crate::bounds::Bounds;
use glam::DVec2;

pub(crate) trait Vec2Ext {
    fn orthonormal(self, polarity: bool, allow_zero: bool) -> Self;

    fn bound_point(self, bounds: &mut Bounds);
}
impl Vec2Ext for DVec2 {
    fn orthonormal(self, polarity: bool, allow_zero: bool) -> Self {
        let len = self.length();

        if len > f64::EPSILON {
            let inv = 1.0 / len;

            if polarity {
                Self::new(-self.y * inv, self.x * inv)
            } else {
                Self::new(self.y * inv, -self.x * inv)
            }
        } else if polarity {
            Self::new(0.0, if allow_zero { 0.0 } else { 1.0 })
        } else {
            Self::new(0.0, if allow_zero { 0.0 } else { -1.0 })
        }
    }

    #[inline]
    fn bound_point(self, bounds: &mut Bounds) {
        bounds.min = bounds.min.min(self);
        bounds.max = bounds.max.max(self);
    }
}
