use std::{collections::HashMap, io::Read};

use msdf_font::{AtlasGlyphData, BitmapData, FieldType, GlyphBuilder, GlyphExt, ttf_parser};

pub(crate) struct FontData {
    pub(crate) glyph_table: HashMap<char, AtlasGlyphData>,
    pub(crate) ascender: f64,
    pub(crate) descender: f64,
    pub(crate) line_gap: f64,
    pub(crate) line_height: f64,
    pub(crate) units_per_em: f64,
    pub(crate) atlas_size: (f32, f32),
}
impl FontData {
    pub(crate) fn new(path: impl AsRef<std::path::Path>) -> Option<(Self, BitmapData)> {
        let mut file = std::fs::File::open(path).ok()?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).ok()?;

        let face = ttf_parser::Face::parse(&buf, 0).ok()?;

        let chars = (0..=255u8).map(|u| u as char).collect::<Vec<_>>();
        let atlas = GlyphBuilder::new(&face)
            .field_type(FieldType::Msdf(3.0))
            .overlapping(true)
            .fix_geometry(false)
            .px_range(4)
            .px_size(50)
            .build_atlas(&chars)?;

        let ascender = f64::from(face.ascender());
        let descender = f64::from(face.descender());
        let line_gap = f64::from(face.line_gap());
        let line_height = ascender - descender + line_gap;

        Some((
            Self {
                glyph_table: atlas.glyph_table,
                ascender,
                descender,
                line_gap,
                line_height,
                units_per_em: f64::from(face.units_per_em()),
                atlas_size: (
                    atlas.bitmap_data.width as f32,
                    atlas.bitmap_data.height as f32,
                ),
            },
            atlas.bitmap_data,
        ))
    }
}
