use crate::{
    BitmapData, BitmapImageType, FieldType, GenerationConfig, GlyphBitmapData, shape::Shape,
};
use glam::DVec2;
use ttf_parser::Face;

/// Data representing the Glyph.
#[derive(Debug, Clone, Copy)]
pub struct GlyphData {
    /// Bounds for constructing the rendering quad.
    pub plane_bounds: GlyphBounds<f32>,
    /// Bounds of the original glyph.
    pub em_bounds: GlyphBounds<i32>,
    /// Glyph advance (in em).
    pub advance: [i32; 2],
    /// Glyph bearing (in em).
    pub bearing: [i32; 2],
}

#[derive(Debug, Clone, Copy)]
pub struct GlyphBounds<T: Copy> {
    /// (Left, Top).
    pub min: [T; 2],
    /// (Right, Bottom).
    pub max: [T; 2],
}
impl<T: Copy + std::ops::Sub<Output = T>> GlyphBounds<T> {
    #[inline]
    pub fn size(&self) -> (T, T) {
        (self.max[0] - self.min[0], self.max[1] - self.min[1])
    }
}

#[derive(Debug)]
pub struct GlyphBuilder<'a> {
    pub(crate) face: &'a Face<'a>,
    pub(crate) scale: f64,
    pub(crate) px_range: u32,
    pub(crate) field_type: FieldType,
    #[cfg(feature = "fix_geometry")]
    pub(crate) fix_geometry: bool,
}
impl<'a> GlyphBuilder<'a> {
    pub fn new(face: &'a Face) -> Self {
        let scale = scale_value(16.0, face);

        Self {
            face,
            scale,
            px_range: 2,
            field_type: FieldType::default(),
            #[cfg(feature = "fix_geometry")]
            fix_geometry: false,
        }
    }

    /// Default is 16.
    #[inline]
    pub fn px_size(mut self, px_size: u32) -> Self {
        self.scale = scale_value(f64::from(px_size), self.face);
        self
    }

    /// Default is 2.
    #[inline]
    pub const fn px_range(mut self, px_range: u32) -> Self {
        self.px_range = px_range;
        self
    }

    /// Default is [`crate::FieldType::Sdf`].
    #[inline]
    pub const fn field_type(mut self, field_type: FieldType) -> Self {
        self.field_type = field_type;
        self
    }

    /// Default is [`false`].
    ///
    /// This is super expensive to compute.
    #[cfg(feature = "fix_geometry")]
    #[inline]
    pub const fn fix_geometry(mut self, fix_geometry: bool) -> Self {
        self.fix_geometry = fix_geometry;
        self
    }

    /// Returns [`None`] if glyph is not present in the font, or glyph
    /// has width or height == 0 (Space for example).
    #[must_use]
    pub fn build(self, c: char) -> Option<Glyph<GlyphBitmapData>> {
        let mut shape = Shape::new(self.scale);

        let image_type = match &self.field_type {
            FieldType::Msdf { .. } => BitmapImageType::Rgb8,
            FieldType::Sdf => BitmapImageType::L8,
        };

        let build_config = self.prepare_for_build(&mut shape, c)?;

        let mut bitmap = GlyphBitmapData::new(
            build_config.bitmap_size.0,
            build_config.bitmap_size.1,
            image_type,
        );

        shape.generate_bitmap(build_config.generation_config, &mut bitmap);

        Some(Glyph {
            bitmap,
            glyph_data: build_config.glyph_data,
        })
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

        let advance = [hor_advance, ver_advance];

        let hor_bearing = self.face.glyph_hor_side_bearing(glyph_id).unwrap_or(0) as i32;
        let ver_bearing = bounds_em.max.y as i32;

        let bearing = [hor_bearing, ver_bearing];

        let bounds_min = [
            bounds_em.min.x.round() as i32,
            bounds_em.min.y.round() as i32,
        ];
        let bounds_max = [
            bounds_em.max.x.round() as i32,
            bounds_em.max.y.round() as i32,
        ];

        let plane_bounds_min = [plane_bounds.min.x as f32, plane_bounds.min.y as f32];
        let plane_bounds_max = [plane_bounds.max.x as f32, plane_bounds.max.y as f32];

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
                #[cfg(feature = "fix_geometry")]
                fix_geometry: self.fix_geometry,
            },
            bitmap_size: (bitmap_size.x.ceil() as usize, bitmap_size.y.ceil() as usize),
        })
    }
}

pub struct Glyph<T: BitmapData> {
    pub bitmap: T,
    pub glyph_data: GlyphData,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BuildConfig {
    pub(crate) glyph_data: GlyphData,
    pub(crate) generation_config: GenerationConfig,
    pub(crate) bitmap_size: (usize, usize),
}

#[inline]
fn scale_value(val: f64, face: &Face) -> f64 {
    val / f64::from(face.units_per_em())
}
