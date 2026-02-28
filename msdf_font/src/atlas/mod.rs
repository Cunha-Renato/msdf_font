mod packer;

use crate::{BitmapData, BitmapDataBuilder, Glyph, GlyphBuilder, GlyphData};
use std::collections::HashMap;
use ttf_parser::Face;

#[derive(Debug)]
pub struct AtlasGlyphData {
    pub offset: (usize, usize),
    pub size: (usize, usize),
    pub data: GlyphData,
}

pub trait GlyphExt {
    fn build_atlas(self, face: &Face, c: &[char]) -> Atlas;
}

impl GlyphExt for GlyphBuilder {
    fn build_atlas(self, face: &Face, c: &[char]) -> Atlas {
        let (chars, glyph_ids) = c
            .iter()
            .filter_map(|c| face.glyph_index(*c).map(|gid| (*c, gid)))
            .collect::<(Vec<_>, Vec<_>)>();

        if glyph_ids.is_empty() {
            todo!();
        }

        let scale = self.get_scale(face);
        let px_range = f64::from(self.px_range);

        let (sizes, glyphs) = glyph_ids
            .iter()
            .map(|gid| {
                let glyph = Glyph::new(
                    face,
                    *gid,
                    scale,
                    px_range,
                    self.field_type,
                    self.overlapping,
                    self.fix_geometry,
                );

                ((glyph.bitmap_data.width, glyph.bitmap_data.height), glyph)
            })
            .collect::<(Vec<_>, Vec<_>)>();

        let packer = packer::Packer::pack(sizes);

        let (packed, p_width, p_height) = (packer.rects, packer.width, packer.height);
        let mut bitmap = BitmapDataBuilder {
            width: p_width,
            height: p_height,
            image_type: glyphs[0].bitmap_data.image_type,
        }
        .build();

        let glyph_table = packed
            .iter()
            .map(|p| {
                (
                    chars[p.index],
                    AtlasGlyphData {
                        offset: (p.x, p.y),
                        size: (
                            glyphs[p.index].bitmap_data.width,
                            glyphs[p.index].bitmap_data.height,
                        ),
                        data: glyphs[p.index].glyph_data,
                    },
                )
            })
            .collect();

        write_atlas_bitmap(&mut bitmap, packed, glyphs);

        Atlas {
            bitmap_data: bitmap,
            glyph_table,
        }
    }
}

pub struct Atlas {
    pub bitmap_data: BitmapData,
    pub glyph_table: HashMap<char, AtlasGlyphData>,
}

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
