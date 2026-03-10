mod atlas_bitmap_data;
mod packer;

use crate::{
    BitmapImageType, BuildConfig, FieldType, GlyphBitmapData, GlyphBounds, GlyphBuilder, GlyphData,
    atlas::atlas_bitmap_data::BitmapDataRegion, shape::Shape,
};
use std::collections::HashMap;

#[derive(Debug)]
pub struct AtlasGlyphData {
    pub atlas_bounds: GlyphBounds<f32>,
    pub data: GlyphData,
}

pub trait AtlasBuilder {
    fn build_atlas(self, c: &[char]) -> Option<Atlas>;
}
impl<'a> AtlasBuilder for GlyphBuilder<'a> {
    fn build_atlas(self, c: &[char]) -> Option<Atlas> {
        let image_type = match &self.field_type {
            FieldType::Msdf { .. } => BitmapImageType::Rgb8,
            FieldType::Sdf => BitmapImageType::L8,
        };

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

        let mut bitmap_data = GlyphBitmapData::new(packer.width, packer.height, image_type);

        let glyph_table = shape_configs
            .into_iter()
            .zip(packer.rects)
            .map(|(sc, packer)| {
                let shape = sc.shape;
                let config = sc.config;

                let mut bitmap_region = BitmapDataRegion {
                    data: &mut bitmap_data,
                    x: packer.x,
                    y: packer.y,
                    width: config.bitmap_size.0,
                    height: config.bitmap_size.1,
                };

                shape.generate_bitmap(config.generation_config, &mut bitmap_region);

                let min = (packer.x as f32, packer.y as f32);
                let max = (
                    min.0 + bitmap_region.width as f32,
                    min.1 + bitmap_region.height as f32,
                );

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
            bitmap_data,
            glyph_table,
        })
    }
}

pub struct Atlas {
    pub bitmap_data: GlyphBitmapData,
    pub glyph_table: HashMap<char, AtlasGlyphData>,
}
