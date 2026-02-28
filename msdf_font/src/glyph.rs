use crate::{BitmapData, FieldType, GenerationConfig, shape::Shape};
use glam::DVec2;
use ttf_parser::Face;

#[derive(Debug)]
pub struct GlyphBuilder {
    pub(crate) px_size: u32,
    pub(crate) px_range: u32,
    pub(crate) max_angle: f64,
    pub(crate) field_type: FieldType,
    pub(crate) overlapping: bool,
}
impl Default for GlyphBuilder {
    fn default() -> Self {
        Self {
            px_size: 16,
            px_range: 2,
            max_angle: 3.0,
            field_type: FieldType::default(),
            overlapping: true,
        }
    }
}
impl GlyphBuilder {
    #[inline]
    pub const fn px_size(mut self, px_size: u32) -> Self {
        self.px_size = px_size;
        self
    }

    #[inline]
    pub const fn px_range(mut self, px_range: u32) -> Self {
        self.px_range = px_range;
        self
    }

    #[inline]
    pub const fn max_angle(mut self, max_angle: f64) -> Self {
        self.max_angle = max_angle;
        self
    }

    #[inline]
    pub const fn field_type(mut self, field_type: FieldType) -> Self {
        self.field_type = field_type;
        self
    }

    #[inline]
    pub const fn overlapping(mut self, overlapping: bool) -> Self {
        self.overlapping = overlapping;
        self
    }

    #[inline]
    pub fn get_scale(&self, face: &Face) -> f64 {
        f64::from(self.px_size) / f64::from(face.units_per_em())
    }

    #[must_use]
    pub fn build(self, face: &Face, glyph_id: ttf_parser::GlyphId) -> Glyph {
        let scale = self.get_scale(face);
        let px_range = f64::from(self.px_range);
        let mut shape = Shape::new(scale);
        face.outline_glyph(glyph_id, &mut shape);

        Glyph::new(shape, px_range, self.overlapping, self.field_type)
    }
}

pub struct Glyph {
    pub bitmap_data: BitmapData,
}
impl Glyph {
    pub(crate) fn new(
        shape: Shape,
        px_range: f64,
        overlapping: bool,
        field_type: FieldType,
    ) -> Self {
        let mut bounds = shape.bounds();
        bounds.min -= DVec2::splat(px_range);
        bounds.max += DVec2::splat(px_range);
        let size = bounds.size();
        let width = size.x.ceil() as usize;
        let height = size.y.ceil() as usize;

        let config = GenerationConfig {
            px_range,
            offset: bounds.min,
            bitmap_size: (width, height),
            overlapping,
            field_type,
        };

        Glyph {
            bitmap_data: shape.generate_bitmap(config),
        }
    }
}
