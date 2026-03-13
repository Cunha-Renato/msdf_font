use core::f64;
use glam::DVec2;

/// Represents the type of the distance field.
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
pub enum FieldType {
    Msdf {
        max_angle: f64,
        // error_correction: bool,
    },
    #[default]
    Sdf,
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct GenerationConfig {
    pub(crate) px_range: f64,
    pub(crate) offset: DVec2,
    pub(crate) field_type: FieldType,
    #[cfg(feature = "fix_geometry")]
    pub(crate) fix_geometry: bool,
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
