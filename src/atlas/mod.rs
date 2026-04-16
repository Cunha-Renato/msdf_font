mod bitmap;
mod packer;

use crate::{
    Glyph, GlyphBitmapData, GlyphBounds, GlyphBuilder, GlyphData,
    atlas::{bitmap::BitmapDataRegion, packer::Packer},
};
use rayon::iter::{Either, ParallelBridge, ParallelIterator};
use std::collections::HashMap;

/// The result of [`GlyphBuilder::build_atlas`].
pub struct AtlasBuildResult {
    /// [`Some`] if at least one [`Glyph`] was built, [`None`] otherwise.
    pub atlas: Option<Atlas>,
    /// [`Some`] if at least one [`Glyph`] failed do build, containing the [`char`], [`None`] otherwise.
    pub rejected: Option<Vec<char>>,
}

struct GlyphChar {
    glyph: Glyph,
    c: char,
}

/// Similar to [`GlyphData`] but for the [`Atlas`] mode.
#[derive(Debug)]
pub struct AtlasGlyphData {
    /// Location and size of the glyph (in px) inside the atlas.
    pub atlas_bounds: GlyphBounds<f32>,
    pub data: GlyphData,
}
impl<'a> GlyphBuilder<'a> {
    /// See [`GlyphBuilder::build`] and [`AtlasBuildResult`].
    ///
    /// For the packing it uses a height based packer.
    pub fn build_atlas(
        self,
        c: impl IntoIterator<Item = char, IntoIter: Send>,
    ) -> AtlasBuildResult {
        let (mut glyphs_char, rejected): (Vec<_>, Vec<_>) = c
            .into_iter()
            .par_bridge()
            .partition_map(|c| match self.build(c) {
                Some(glyph) => Either::Left(GlyphChar { glyph, c }),
                None => Either::Right(c),
            });

        let rejected = if !rejected.is_empty() {
            Some(rejected)
        } else {
            None
        };

        // Either no glyph was present, or all the glyphs where rejected.
        if glyphs_char.is_empty() {
            return AtlasBuildResult {
                atlas: None,
                rejected,
            };
        }

        let packer = Packer::pack(&mut glyphs_char);
        let mut glyphs = Vec::with_capacity(glyphs_char.len());

        let glyph_table = glyphs_char
            .into_iter()
            .zip(&packer.rects)
            .map(|(gc, packer)| {
                let min = [packer.x as f32, packer.y as f32];
                let max = [
                    min[0] + gc.glyph.build_config.bitmap_size[0] as f32,
                    min[1] + gc.glyph.build_config.bitmap_size[1] as f32,
                ];

                let atlas_bounds = GlyphBounds { min, max };
                let data = gc.glyph.data;

                glyphs.push(gc.glyph);

                (gc.c, AtlasGlyphData { atlas_bounds, data })
            })
            .collect();

        let atlas = Some(Atlas {
            glyph_table,
            glyphs,
            packer,
        });

        AtlasBuildResult { atlas, rejected }
    }
}

/// Represents the glyph atlas.
pub struct Atlas {
    /// Table of data for glyphs.
    pub glyph_table: HashMap<char, AtlasGlyphData>,
    glyphs: Vec<Glyph>,
    packer: Packer,
}
impl Atlas {
    #[inline]
    pub fn builder<'a>(face: &'a ttf_parser::Face) -> GlyphBuilder<'a> {
        GlyphBuilder::new(face)
    }

    /// Generates sdf atlas bitmap with the [`GlyphBuilder`] configuration.
    ///
    /// The bitmap has only one channel (L8).
    pub fn sdf(&mut self) -> GlyphBitmapData<u8, 1> {
        self.gen_field(|g, region| {
            g.shape
                .generate_sdf(g.build_config.px_range, g.build_config.offset, region)
        })
    }

    /// Generates msdf atlas bitmap with the [`GlyphBuilder`] configuration.
    ///
    /// The bitmap has 3 channels (Rgb8).
    pub fn msdf(&mut self, max_angle: f64, error_correction: bool) -> GlyphBitmapData<u8, 3> {
        self.gen_field(|g, region| {
            g.shape.generate_msdf(
                g.build_config.px_range,
                g.build_config.offset,
                max_angle,
                error_correction,
                region,
            )
        })
    }

    fn gen_field<const N: usize>(
        &mut self,
        f: impl Fn(&mut Glyph, &mut BitmapDataRegion<N>) + Sync,
    ) -> GlyphBitmapData<u8, N> {
        let bitmap_size = [self.packer.width, self.packer.height];
        let mut bitmap = GlyphBitmapData::new(bitmap_size[0], bitmap_size[1]);

        let bitmap_ptr = &mut bitmap as *mut GlyphBitmapData<u8, N> as usize;

        self.glyphs
            // .par_iter_mut()
            .iter_mut()
            .zip(&self.packer.rects)
            .par_bridge()
            .for_each(|(g, rect)| {
                let bitmap_ref = unsafe { &mut *(bitmap_ptr as *mut GlyphBitmapData<u8, N>) };

                let mut bitmap_region = BitmapDataRegion {
                    data: bitmap_ref,
                    x: rect.x,
                    y: rect.y,
                    width: g.build_config.bitmap_size[0],
                    height: g.build_config.bitmap_size[1],
                };

                f(g, &mut bitmap_region);
            });

        bitmap
    }
}
