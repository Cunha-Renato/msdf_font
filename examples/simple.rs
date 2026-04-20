use std::error::Error;

use msdf_font::{GlyphBitmapData, GlyphBuilder, ttf_parser};

#[cfg(not(feature = "atlas"))]
fn main() -> Result<(), Box<dyn Error>> {
    let face = ttf_parser::Face::parse(include_bytes!("assets/OpenSans-Medium.ttf"), 0)?;
    let mut glyph = GlyphBuilder::new(&face)
        .px_range(2)
        .px_size(100)
        .build('ç')
        .unwrap();

    let (msdf, sdf) = (glyph.msdf(3.0, true), glyph.sdf());
    save(msdf, sdf)?;

    Ok(())
}

#[cfg(feature = "atlas")]
fn main() -> Result<(), Box<dyn Error>> {
    let face = ttf_parser::Face::parse(include_bytes!("assets/OpenSans-Medium.ttf"), 0)?;
    let char_data = (0..=0xff).filter_map(char::from_u32);

    let atlas_result = GlyphBuilder::new(&face)
        .px_range(2)
        .px_size(100)
        .build_atlas(char_data);

    let mut atlas = atlas_result.atlas.unwrap();

    if let Some(rejected) = atlas_result.rejected {
        println!("{} glyphs where rejected.", rejected.len());
    }

    let (msdf, sdf) = (atlas.msdf(3.0, true), atlas.sdf());

    save(msdf, sdf)?;

    Ok(())
}

fn save(msdf: GlyphBitmapData<u8, 3>, sdf: GlyphBitmapData<u8, 1>) -> Result<(), Box<dyn Error>> {
    image::save_buffer(
        "simple_msdf.png",
        msdf.bytes(),
        msdf.width as u32,
        msdf.height as u32,
        image::ColorType::Rgb8,
    )?;

    image::save_buffer(
        "simple_sdf.png",
        sdf.bytes(),
        sdf.width as u32,
        sdf.height as u32,
        image::ColorType::L8,
    )?;

    Ok(())
}
