mod app;

use msdf_font::{BitmapImageType, FieldType, GlyphBuilder, GlyphExt, ttf_parser};

fn main() {
    let face = if let Ok(face) = ttf_parser::Face::parse(include_bytes!("../../OpenSans.ttf"), 0) {
        face
    } else {
        return;
    };
    let glyph_ids = (0..=255u8)
        .filter_map(|ac| face.glyph_index(ac as char))
        .collect::<Vec<_>>();

    let atlas = GlyphBuilder::default()
        // .field_type(FieldType::Sdf)
        .field_type(FieldType::Msdf(3.0))
        .overlapping(true)
        .fix_geometry(false)
        .px_range(4)
        .px_size(32)
        // .build(&face, glyph_ids[4]);
        .build_atlas(&face, &glyph_ids);

    println!(
        "({}, {})",
        atlas.bitmap_data.width, atlas.bitmap_data.height
    );

    let _ = image::save_buffer(
        "image.png",
        &atlas.bitmap_data.bytes,
        atlas.bitmap_data.width as u32,
        atlas.bitmap_data.height as u32,
        match atlas.bitmap_data.image_type {
            BitmapImageType::L8 => image::ColorType::L8,
            BitmapImageType::Rgb8 => image::ColorType::Rgb8,
        },
    )
    .is_ok();

    let el = winit::event_loop::EventLoop::new().unwrap();
    el.run_app(&mut crate::app::App::default()).unwrap();
}
