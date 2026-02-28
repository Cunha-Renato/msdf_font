mod packer;

use crate::{BitmapData, BitmapDataBuilder, Glyph, GlyphBuilder, shape::Shape};
use ttf_parser::{Face, GlyphId};

pub trait GlyphExt {
    fn build_atlas(self, face: &Face, glyph_ids: &[GlyphId]) -> Atlas;
}

impl GlyphExt for GlyphBuilder {
    fn build_atlas(self, face: &Face, glyph_ids: &[GlyphId]) -> Atlas {
        if glyph_ids.is_empty() {
            todo!();
        }

        let scale = self.get_scale(face);
        let px_range = f64::from(self.px_range);

        let (sizes, glyphs) = glyph_ids
            .iter()
            .map(|gid| {
                let mut shape = Shape::new(scale);
                face.outline_glyph(*gid, &mut shape);

                let glyph = Glyph::new(shape, px_range, self.overlapping, self.field_type);

                ((glyph.bitmap_data.width, glyph.bitmap_data.height), glyph)
            })
            .collect::<(Vec<_>, Vec<_>)>();

        let (packed, p_width, p_height) = packer::pack_rects(&sizes);
        let mut bitmap = BitmapDataBuilder {
            width: p_width,
            height: p_height,
            image_type: glyphs[0].bitmap_data.image_type,
        }
        .build();

        write_atlas_bitmap(&mut bitmap, packed, glyphs);

        Atlas {
            bitmap_data: bitmap,
        }
    }
}

pub struct Atlas {
    pub bitmap_data: BitmapData,
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
