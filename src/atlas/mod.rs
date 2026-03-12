mod bitmap;
mod packer;

use crate::{
    BitmapImageType, BuildConfig, FieldType, GlyphBitmapData, GlyphBounds, GlyphBuilder, GlyphData,
    atlas::bitmap::BitmapDataRegion, shape::Shape,
};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use std::collections::HashMap;

/// Similar to [`crate::GlyphData`] but for the atlas mode.
#[derive(Debug)]
pub struct AtlasGlyphData {
    /// Location and size of the glyph (in px) inside the atlas.
    pub atlas_bounds: GlyphBounds<f32>,
    pub data: GlyphData,
}

pub trait AtlasBuilder {
    fn build_atlas(self, c: &[char]) -> Option<Atlas>;
}
impl<'a> AtlasBuilder for GlyphBuilder<'a> {
    /// Returns [`None`] if no glyph could be build.
    ///
    /// See [`crate::GlyphBuilder::build`].
    ///
    /// For the packing it uses a simple height based packer.
    fn build_atlas(self, c: &[char]) -> Option<Atlas> {
        struct ShapeConfig {
            config: BuildConfig,
            shape: Shape,
            c: char,
        }

        let mut shape_configs = c
            .iter()
            .filter_map(|c| {
                let mut shape = Shape::new(self.scale);
                let config = self.prepare_for_build(&mut shape, *c)?;

                Some(ShapeConfig {
                    config,
                    shape,
                    c: *c,
                })
            })
            .collect::<Vec<_>>();

        if shape_configs.is_empty() {
            return None;
        }

        let packer = packer::Packer::pack(&mut shape_configs, |sc| sc.config.bitmap_size);

        let image_type = match &self.field_type {
            FieldType::Msdf { .. } => BitmapImageType::Rgb8,
            FieldType::Sdf => BitmapImageType::L8,
        };
        let mut bitmap = GlyphBitmapData::new(packer.width, packer.height, image_type);
        let bitmap_ptr = &mut bitmap as *mut GlyphBitmapData as usize;

        let glyph_table = shape_configs
            .into_par_iter()
            .zip(packer.rects)
            .map(|(sc, packer)| {
                // This is fine, we don't have overlapping pixels.
                let bitmap_ref = unsafe { &mut *(bitmap_ptr as *mut GlyphBitmapData) };
                let shape = sc.shape;
                let config = sc.config;

                let mut bitmap_region = BitmapDataRegion {
                    data: bitmap_ref,
                    x: packer.x,
                    y: packer.y,
                    width: config.bitmap_size.0,
                    height: config.bitmap_size.1,
                };

                shape.generate_bitmap(config.generation_config, &mut bitmap_region);

                let min = [packer.x as f32, packer.y as f32];
                let max = [
                    min[0] + bitmap_region.width as f32,
                    min[1] + bitmap_region.height as f32,
                ];

                let atlas_bounds = GlyphBounds { min, max };

                (
                    sc.c,
                    AtlasGlyphData {
                        atlas_bounds,
                        data: config.glyph_data,
                    },
                )
            })
            .collect();

        Some(Atlas {
            bitmap,
            glyph_table,
        })
    }
}

/// Represents the glyph atlas.
pub struct Atlas {
    /// Bitmap of the entire atlas.
    pub bitmap: GlyphBitmapData,
    /// Table of data for glyphs.
    pub glyph_table: HashMap<char, AtlasGlyphData>,
}
