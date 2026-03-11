use msdf_font::{
    AtlasBuilder, AtlasGlyphData, FieldType, GlyphBitmapData, GlyphBuilder, ttf_parser,
};
use std::{collections::HashMap, io::Read};

pub(crate) struct FontData {
    pub(crate) glyph_table: HashMap<char, AtlasGlyphData>,
    pub(crate) ascender: f64,
    pub(crate) descender: f64,
    pub(crate) line_gap: f64,
    pub(crate) line_height: f64,
    pub(crate) units_per_em: f64,
    pub(crate) atlas_size: [f32; 2],
    pub(crate) px_range: f64,
}
impl FontData {
    pub(crate) fn new(path: impl AsRef<std::path::Path>) -> Option<(Self, GlyphBitmapData)> {
        let mut file = std::fs::File::open(path).ok()?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).ok()?;

        let face = ttf_parser::Face::parse(&buf, 0).ok()?;

        let px_range = 4;
        let chars = (0..=0x0ff).filter_map(char::from_u32).collect::<Vec<_>>();
        // let chars = ['$'];
        let atlas = GlyphBuilder::new(&face)
            .field_type(FieldType::Msdf { max_angle: 3.0 })
            // .field_type(FieldType::Sdf)
            .fix_geometry(true)
            .px_range(px_range)
            .px_size(40)
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
                px_range: px_range as f64,
                atlas_size: [
                    atlas.bitmap_data.width as f32,
                    atlas.bitmap_data.height as f32,
                ],
            },
            atlas.bitmap_data,
        ))
    }
}
