use crate::{GlyphBitmapData, shape::Shape};
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
    pub fn size(&self) -> [T; 2] {
        [self.max[0] - self.min[0], self.max[1] - self.min[1]]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GlyphBuilder<'a> {
    face: &'a Face<'a>,
    build_config: BuildConfig,
    #[cfg(feature = "fix_geometry")]
    fix_geometry: bool,
}
impl<'a> GlyphBuilder<'a> {
    pub fn new(face: &'a Face) -> Self {
        Self {
            face,
            build_config: BuildConfig {
                px_range: 2.0,
                scale: scale_value(40.0, face),
                ..Default::default()
            },
            #[cfg(feature = "fix_geometry")]
            fix_geometry: false,
        }
    }

    /// Default is 40.
    #[inline]
    pub fn px_size(mut self, px_size: u32) -> Self {
        self.build_config.scale = scale_value(f64::from(px_size), self.face);
        self
    }

    /// Default is 2.
    #[inline]
    pub fn px_range(mut self, px_range: u32) -> Self {
        self.build_config.px_range = f64::from(px_range);
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

    pub fn build(mut self, c: char) -> Option<Glyph> {
        let mut shape = Shape::new(self.build_config.scale);
        let glyph_id = self.face.glyph_index(c)?;

        self.face.outline_glyph(glyph_id, &mut shape);

        #[cfg(feature = "fix_geometry")]
        if self.fix_geometry {
            shape.resolve_shape_geometry();
        }

        let mut bitmap_bounds = shape.bounds();
        let bitmap_size = bitmap_bounds.size();
        if bitmap_size.x.ceil() as usize == 0 || bitmap_size.y.ceil() as usize == 0 {
            return None;
        }

        // Glyph Bounds in em scale, (same as in the font file).
        let mut bounds_em = bitmap_bounds;
        bounds_em.min /= self.build_config.scale;
        bounds_em.max /= self.build_config.scale;

        // Padding for px_range.
        bitmap_bounds.min -= DVec2::splat(self.build_config.px_range);
        bitmap_bounds.max += DVec2::splat(self.build_config.px_range);
        let bitmap_size = bitmap_bounds.size();

        // Glyph Bounds in em scale, (same as in the font file), with the padding.
        // We need this for rendering.
        let mut plane_bounds = bitmap_bounds;
        plane_bounds.min /= self.build_config.scale;
        plane_bounds.max /= self.build_config.scale;

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

        self.build_config.bitmap_size =
            [bitmap_size.x.ceil() as usize, bitmap_size.y.ceil() as usize];
        self.build_config.offset = bitmap_bounds.min;

        Some(Glyph {
            shape,
            build_config: self.build_config,
            data: GlyphData {
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
        })
    }
}

pub struct Glyph {
    pub data: GlyphData,
    pub(crate) build_config: BuildConfig,
    pub(crate) shape: Shape,
}
impl Glyph {
    pub fn sdf(&self) -> GlyphBitmapData<u8, 1> {
        let bitmap_size = self.build_config.bitmap_size;
        let mut bitmap = GlyphBitmapData::new(bitmap_size[0], bitmap_size[1]);

        self.shape.generate_sdf(
            self.build_config.px_range,
            self.build_config.offset,
            &mut bitmap,
        );

        bitmap
    }

    pub fn msdf(&mut self, max_angle: f64) -> GlyphBitmapData<u8, 3> {
        let bitmap_size = self.build_config.bitmap_size;
        let mut bitmap = GlyphBitmapData::new(bitmap_size[0], bitmap_size[1]);

        self.shape.generate_msdf(
            self.build_config.px_range,
            self.build_config.offset,
            max_angle,
            &mut bitmap,
        );

        bitmap
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct BuildConfig {
    pub(crate) offset: DVec2,
    pub(crate) px_range: f64,
    pub(crate) scale: f64,
    pub(crate) bitmap_size: [usize; 2],
}

#[inline]
fn scale_value(val: f64, face: &Face) -> f64 {
    val / f64::from(face.units_per_em())
}
