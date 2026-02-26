use msdf_font::{
    BitmapImageType, FieldType, GlyphBuilder,
    ttf_parser::{self, GlyphId},
};

fn main() {
    let face =
        if let Ok(face) = ttf_parser::Face::parse(include_bytes!("../../Roboto-Black.ttf"), 0) {
            face
        } else {
            return;
        };
    let glyph_id = face.glyph_index('B').unwrap_or(GlyphId(0));

    let glyph = GlyphBuilder::default()
        .field_type(FieldType::Sdf)
        .overlapping(true)
        .px_range(4)
        .px_size(100)
        .build(&face, glyph_id);

    if image::save_buffer(
        "image.png",
        &glyph.bitmap_data.bytes,
        glyph.bitmap_data.width as u32,
        glyph.bitmap_data.height as u32,
        match glyph.bitmap_data.image_type {
            BitmapImageType::L8 => image::ColorType::L8,
            BitmapImageType::Rgb8 => image::ColorType::Rgb8,
        },
    )
    .is_ok()
    {}
}
