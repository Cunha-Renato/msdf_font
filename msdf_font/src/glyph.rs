use crate::{BitmapData, FieldType, GenerationConfig, GlyphBounds, GlyphData, shape::Shape};
use glam::DVec2;
use ttf_parser::{Face, GlyphId};

#[derive(Debug)]
pub struct GlyphBuilder<'a> {
    pub(crate) face: &'a Face<'a>,
    pub(crate) scale: f64,
    pub(crate) px_range: u32,
    pub(crate) max_angle: f64,
    pub(crate) field_type: FieldType,
    pub(crate) overlapping: bool,
    pub(crate) fix_geometry: bool,
}
impl<'a> GlyphBuilder<'a> {
    pub fn new(face: &'a Face) -> Self {
        let scale = scale_value(f64::from(16), face);

        Self {
            face,
            scale,
            px_range: 2,
            max_angle: 3.0,
            field_type: FieldType::default(),
            overlapping: false,
            fix_geometry: false,
        }
    }

    #[inline]
    pub fn px_size(mut self, px_size: u32) -> Self {
        self.scale = scale_value(f64::from(px_size), self.face);
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

    #[must_use]
    pub fn build(self, c: char) -> Option<Glyph> {
        let px_range = f64::from(self.px_range);
        let glyph_id = self.face.glyph_index(c)?;

        Some(Glyph::new(
            self.face,
            glyph_id,
            self.scale,
            px_range,
            self.field_type,
            self.overlapping,
            self.fix_geometry,
        ))
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
        let mut shape = Shape::new(scale);
        face.outline_glyph(glyph_id, &mut shape);

        // Glyph Bounds in bitmap scale.
        let mut bitmap_bounds = shape.bounds();

        // Glyph Bounds in em scale, (same as in the font file).
        let mut bounds_em = bitmap_bounds;
        bounds_em.min /= scale;
        bounds_em.max /= scale;

        // Padding for px_range.
        bitmap_bounds.min -= DVec2::splat(px_range);
        bitmap_bounds.max += DVec2::splat(px_range);

        // Glyph Bounds in em scale, (same as in the font file), with the padding.
        // We need this for rendering.
        let mut plane_bounds = bitmap_bounds;
        plane_bounds.min /= scale;
        plane_bounds.max /= scale;

        let bitmap_size = bitmap_bounds.size();
        let bitmap_width = bitmap_size.x.ceil() as usize;
        let bitmap_height = bitmap_size.y.ceil() as usize;

        let config = GenerationConfig {
            px_range,
            offset: bitmap_bounds.min,
            bitmap_size: (bitmap_width, bitmap_height),
            field_type,
            overlapping,
            fix_geometry,
        };

        let hor_advance = face.glyph_hor_advance(glyph_id).unwrap_or(0) as i32;
        let ver_advance = face.glyph_ver_advance(glyph_id).unwrap_or(0) as i32;

        let advance = (hor_advance, ver_advance);

        let hor_bearing = face.glyph_hor_side_bearing(glyph_id).unwrap_or(0) as i32;
        let ver_bearing = bounds_em.max.y as i32;

        let bearing = (hor_bearing, ver_bearing);

        let bounds_min = (
            bounds_em.min.x.round() as i32,
            bounds_em.min.y.round() as i32,
        );
        let bounds_max = (
            bounds_em.max.x.round() as i32,
            bounds_em.max.y.round() as i32,
        );

        let plane_bounds_min = (plane_bounds.min.x as f32, plane_bounds.min.y as f32);
        let plane_bounds_max = (plane_bounds.max.x as f32, plane_bounds.max.y as f32);

        Glyph {
            bitmap_data: shape.generate_bitmap(config),
            glyph_data: GlyphData {
                plane_bounds: GlyphBounds {
                    min: plane_bounds_min,
                    max: plane_bounds_max,
                },
                em_bounds: GlyphBounds {
                    min: bounds_min,
                    max: bounds_max,
                },
                advance,
                bearing,
            },
        }
    }
}

#[inline]
fn scale_value(val: f64, face: &Face) -> f64 {
    val / f64::from(face.units_per_em())
}
