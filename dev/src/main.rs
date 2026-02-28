use msdf_font::{
    BitmapImageType, FieldType, GlyphBuilder, GlyphExt,
    ttf_parser::{self, GlyphId},
};

fn main() {
    let face = if let Ok(face) = ttf_parser::Face::parse(include_bytes!("../../OpenSans.ttf"), 0) {
        face
    } else {
        return;
    };
    let glyph_ids = ['A', 'B', 'C', '&'].map(|c| face.glyph_index(c).unwrap_or(GlyphId(0)));

    let atlas = GlyphBuilder::default()
        // .field_type(FieldType::Sdf)
        .field_type(FieldType::Msdf(3.0))
        .overlapping(false)
        .px_range(4)
        .px_size(100)
        // .build(&face, glyph_id);
        .build_atlas(&face, &glyph_ids);

    if image::save_buffer(
        "image.png",
        &atlas.bitmap_data.bytes,
        atlas.bitmap_data.width as u32,
        atlas.bitmap_data.height as u32,
        match atlas.bitmap_data.image_type {
            BitmapImageType::L8 => image::ColorType::L8,
            BitmapImageType::Rgb8 => image::ColorType::Rgb8,
        },
    )
    .is_ok()
    {}
}
