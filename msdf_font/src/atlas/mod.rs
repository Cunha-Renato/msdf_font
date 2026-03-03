mod packer;

#[cfg(feature = "rayon")]
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::{BitmapData, BitmapDataBuilder, Glyph, GlyphBounds, GlyphBuilder, GlyphData};
use std::collections::HashMap;

#[derive(Debug)]
pub struct AtlasGlyphData {
    pub atlas_bounds: GlyphBounds<f32>,
    pub data: GlyphData,
}

pub trait GlyphExt {
    fn build_atlas(self, c: &[char]) -> Option<Atlas>;
}

impl<'a> GlyphExt for GlyphBuilder<'a> {
    fn build_atlas(self, c: &[char]) -> Option<Atlas> {
        let px_range = f64::from(self.px_range);

        #[cfg(feature = "rayon")]
        let char_iter = c.into_par_iter();
        #[cfg(not(feature = "rayon"))]
        let char_iter = c.into_iter();

        struct GlyphCharPair {
            glyph: Glyph,
            c: char,
        }

        let (gc_pair, sizes) = char_iter
            .filter_map(|c| {
                let (size, gc_pair) = self.face.glyph_index(*c).map(|gid| {
                    let glyph = Glyph::new(
                        self.face,
                        gid,
                        self.scale,
                        px_range,
                        self.field_type,
                        self.overlapping,
                        self.fix_geometry,
                    );

                    (
                        (glyph.bitmap_data.width, glyph.bitmap_data.height),
                        GlyphCharPair { glyph, c: *c },
                    )
                })?;

                if gc_pair.glyph.bitmap_data.bytes.is_empty() {
                    None
                } else {
                    Some((gc_pair, size))
                }
            })
            .collect::<(Vec<_>, Vec<_>)>();

        if sizes.is_empty() {
            return None;
        }

        let packer = packer::Packer::pack(sizes);

        let (packed, p_width, p_height) = (packer.rects, packer.width, packer.height);
        let mut bitmap_data = BitmapDataBuilder {
            width: p_width,
            height: p_height,
            image_type: gc_pair[0].glyph.bitmap_data.image_type,
        }
        .build();

        let glyph_table = packed
            .iter()
            .map(|p| {
                let gc_pair = &gc_pair[p.index];

                let min = (p.x as f32, p.y as f32);
                let max = (
                    min.0 + gc_pair.glyph.bitmap_data.width as f32,
                    min.1 + gc_pair.glyph.bitmap_data.height as f32,
                );

                let atlas_bounds = GlyphBounds { min, max };

                (
                    gc_pair.c,
                    AtlasGlyphData {
                        atlas_bounds,
                        data: gc_pair.glyph.glyph_data,
                    },
                )
            })
            .collect();

        let glyphs = gc_pair.into_iter().map(|gc| gc.glyph).collect();

        write_atlas_bitmap(&mut bitmap_data, packed, glyphs);

        Some(Atlas {
            bitmap_data,
            glyph_table,
        })
    }
}

pub struct Atlas {
    pub bitmap_data: BitmapData,
    pub glyph_table: HashMap<char, AtlasGlyphData>,
}

#[cfg(not(feature = "rayon"))]
fn write_atlas_bitmap(
    bitmap: &mut BitmapData,
    packed: Vec<packer::PackedRect>,
    glyphs: Vec<Glyph>,
) {
    for packed in packed {
        let glyph_data = &glyphs[packed.index].bitmap_data;

        for y in 0..glyph_data.height {
            for x in 0..glyph_data.width {
                let atlas_x = x + packed.x;
                let atlas_y = y + packed.y;

                glyph_data.get_px(x, y, |px| bitmap.set_px(px, atlas_x, atlas_y));
            }
        }
    }
}

#[cfg(feature = "rayon")]
fn write_atlas_bitmap(
    bitmap: &mut BitmapData,
    packed: Vec<packer::PackedRect>,
    glyphs: Vec<Glyph>,
) {
    let bp = bitmap as *mut BitmapData as usize;

    packed.par_iter().for_each(|packed| {
        let bitmap = unsafe { &mut *(bp as *mut BitmapData) };
        let glyph_data = &glyphs[packed.index].bitmap_data;

        for y in 0..glyph_data.height {
            for x in 0..glyph_data.width {
                let atlas_x = x + packed.x;
                let atlas_y = y + packed.y;

                glyph_data.get_px(x, y, |px| bitmap.set_px(px, atlas_x, atlas_y));
            }
        }
    });
}
