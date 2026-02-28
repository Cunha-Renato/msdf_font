use crate::{BitmapData, FieldType, GenerationConfig, shape::Shape};
use glam::DVec2;
use ttf_parser::{Face, GlyphId};

#[derive(Debug, Clone, Copy)]
pub struct GlyphBounds {
    pub min: (f32, f32),
    pub max: (f32, f32),
}

#[derive(Debug, Clone, Copy)]
pub struct GlyphData {
    pub advance: (f32, f32),
    pub bearing: (f32, f32),
    pub bounds: GlyphBounds,
}

#[derive(Debug)]
pub struct GlyphBuilder {
    pub(crate) px_size: u32,
    pub(crate) px_range: u32,
    pub(crate) max_angle: f64,
    pub(crate) field_type: FieldType,
    pub(crate) overlapping: bool,
    pub(crate) fix_geometry: bool,
}
impl Default for GlyphBuilder {
    fn default() -> Self {
        Self {
            px_size: 16,
            px_range: 2,
            max_angle: 3.0,
            field_type: FieldType::default(),
            overlapping: false,
            fix_geometry: false,
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
    pub const fn fix_geometry(mut self, fix_geometry: bool) -> Self {
        self.fix_geometry = fix_geometry;
        self
    }

    #[inline]
    pub fn get_scale(&self, face: &Face) -> f64 {
        f64::from(self.px_size) / f64::from(face.units_per_em())
    }

    #[must_use]
    pub fn build(self, face: &Face, glyph_id: GlyphId) -> Glyph {
        let scale = self.get_scale(face);
        let px_range = f64::from(self.px_range);

        Glyph::new(
            face,
            glyph_id,
            scale,
            px_range,
            self.field_type,
            self.overlapping,
            self.fix_geometry,
        )
    }
}

pub struct Glyph {
    pub bitmap_data: BitmapData,
    pub glyph_data: GlyphData,
}
impl Glyph {
    pub(crate) fn new(
        face: &Face,
        glyph_id: GlyphId,
        scale: f64,
        px_range: f64,
        field_type: FieldType,
        overlapping: bool,
        fix_geometry: bool,
    ) -> Self {
        let unit_scale = 1.0 / f32::from(face.units_per_em());
        let mut shape = Shape::new(scale);
        face.outline_glyph(glyph_id, &mut shape);

        let mut bounds = shape.bounds();
        let mut bounds_unit_scale = bounds;
        bounds_unit_scale.min = (bounds_unit_scale.min / scale) * unit_scale as f64;
        bounds_unit_scale.max = (bounds_unit_scale.max / scale) * unit_scale as f64;

        bounds.min -= DVec2::splat(px_range);
        bounds.max += DVec2::splat(px_range);
        let size = bounds.size();
        let width = size.x.ceil() as usize;
        let height = size.y.ceil() as usize;

        let config = GenerationConfig {
            px_range,
            offset: bounds.min,
            bitmap_size: (width, height),
            field_type,
            overlapping,
            fix_geometry,
        };

        let hor_advance = face.glyph_hor_advance(glyph_id).unwrap_or(0) as f32 * unit_scale;
        let ver_advance = face.glyph_ver_advance(glyph_id).unwrap_or(0) as f32 * unit_scale;

        let advance = (hor_advance, ver_advance);

        let hor_bearing = face.glyph_hor_side_bearing(glyph_id).unwrap_or(0) as f32 * unit_scale;
        let ver_bearing = face.glyph_ver_side_bearing(glyph_id).unwrap_or(0) as f32 * unit_scale;

        let bearing = (hor_bearing, ver_bearing);

        let bounds_min = (
            bounds_unit_scale.min.x as f32,
            bounds_unit_scale.min.y as f32,
        );
        let bounds_max = (
            bounds_unit_scale.max.x as f32,
            bounds_unit_scale.max.y as f32,
        );

        Glyph {
            bitmap_data: shape.generate_bitmap(config),
            glyph_data: GlyphData {
                advance,
                bearing,
                bounds: GlyphBounds {
                    min: bounds_min,
                    max: bounds_max,
                },
            },
        }
    }
}
