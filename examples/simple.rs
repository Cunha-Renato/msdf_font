use std::error::Error;

use msdf_font::{GlyphBuilder, ttf_parser};

fn main() -> Result<(), Box<dyn Error>> {
    let face = ttf_parser::Face::parse(include_bytes!("assets/OpenSans-Medium.ttf"), 0)?;

    #[cfg(not(feature = "atlas"))]
    let char_data = 'ç';
    #[cfg(feature = "atlas")]
    let char_data = (0..=0xff).filter_map(char::from_u32);

    let builder = GlyphBuilder::new(&face).px_range(2).px_size(100);

    #[cfg(feature = "fix_geometry")]
    let builder = builder.fix_geometry(true);

    #[cfg(not(feature = "atlas"))]
    let mut glyph = builder.build(char_data).unwrap();

    let (msdf, sdf) = {
        #[cfg(not(feature = "atlas"))]
        let (msdf, sdf) = (glyph.msdf(3.0, true), glyph.sdf());

        #[cfg(feature = "atlas")]
        let mut atlas = builder.build_atlas(char_data).unwrap();

        #[cfg(feature = "atlas")]
        let (msdf, sdf) = (atlas.msdf(3.0, true), atlas.sdf());

        (msdf, sdf)
    };

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
