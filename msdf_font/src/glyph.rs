use crate::{
    BitmapData, BitmapImageType, BuildConfig, FieldType, GenerationConfig, GlyphBitmapData,
    GlyphBounds, GlyphData, shape::Shape,
};
use glam::DVec2;
use ttf_parser::Face;

#[derive(Debug)]
pub struct GlyphBuilder<'a> {
    pub(crate) face: &'a Face<'a>,
    pub(crate) scale: f64,
    pub(crate) px_range: u32,
    pub(crate) max_angle: f64,
    pub(crate) field_type: FieldType,
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
    pub const fn fix_geometry(mut self, fix_geometry: bool) -> Self {
        self.fix_geometry = fix_geometry;
        self
    }

    pub(crate) fn prepare_for_build(&self, shape: &mut Shape, c: char) -> Option<BuildConfig> {
        let px_range = f64::from(self.px_range);
        let glyph_id = self.face.glyph_index(c)?;

        self.face.outline_glyph(glyph_id, shape);

        let mut bitmap_bounds = shape.bounds();

        // Glyph Bounds in em scale, (same as in the font file).
        let mut bounds_em = bitmap_bounds;
        bounds_em.min /= self.scale;
        bounds_em.max /= self.scale;

        // Padding for px_range.
        bitmap_bounds.min -= DVec2::splat(px_range);
        bitmap_bounds.max += DVec2::splat(px_range);
        let bitmap_size = bitmap_bounds.size();

        if bitmap_size.x.ceil() as usize == 0 || bitmap_size.y.ceil() as usize == 0 {
            return None;
        }

        // Glyph Bounds in em scale, (same as in the font file), with the padding.
        // We need this for rendering.
        let mut plane_bounds = bitmap_bounds;
        plane_bounds.min /= self.scale;
        plane_bounds.max /= self.scale;

        let hor_advance = self.face.glyph_hor_advance(glyph_id).unwrap_or(0) as i32;
        let ver_advance = self.face.glyph_ver_advance(glyph_id).unwrap_or(0) as i32;

        let advance = (hor_advance, ver_advance);

        let hor_bearing = self.face.glyph_hor_side_bearing(glyph_id).unwrap_or(0) as i32;
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

        Some(BuildConfig {
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
            generation_config: GenerationConfig {
                px_range,
                offset: bitmap_bounds.min,
                field_type: self.field_type,
                fix_geometry: self.fix_geometry,
            },
            bitmap_size: (bitmap_size.x.ceil() as usize, bitmap_size.y.ceil() as usize),
        })
    }

    #[must_use]
    pub fn build(self, c: char) -> Option<Glyph<GlyphBitmapData>> {
        let mut shape = Shape::new(self.scale);

        let image_type = match &self.field_type {
            FieldType::Msdf { .. } => BitmapImageType::Rgb8,
            FieldType::Sdf => BitmapImageType::L8,
        };

        let build_config = self.prepare_for_build(&mut shape, c)?;

        let mut bitmap_data = GlyphBitmapData::new(
            build_config.bitmap_size.0,
            build_config.bitmap_size.1,
            image_type,
        );

        shape.generate_bitmap(build_config.generation_config, &mut bitmap_data);

        Some(Glyph {
            bitmap_data,
            glyph_data: build_config.glyph_data,
        })
    }
}

pub struct Glyph<T: BitmapData> {
    pub bitmap_data: T,
    pub glyph_data: GlyphData,
}

#[inline]
fn scale_value(val: f64, face: &Face) -> f64 {
    val / f64::from(face.units_per_em())
}
