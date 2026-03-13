use std::error::Error;

use msdf_font::{FieldType, GlyphBuilder, ttf_parser};

#[cfg(feature = "atlas")]
use msdf_font::AtlasBuilder;

fn main() -> Result<(), Box<dyn Error>> {
    let face = ttf_parser::Face::parse(include_bytes!("assets/OpenSans-Medium.ttf"), 0)?;

    #[cfg(not(feature = "atlas"))]
    let char_data = 'ç';
    #[cfg(feature = "atlas")]
    let char_data = (0..=0xff).filter_map(char::from_u32);

    let msdf_builder = GlyphBuilder::new(&face)
        .field_type(FieldType::Msdf { max_angle: 3.0 })
        .px_range(2)
        .px_size(100);

    #[cfg(feature = "fix_geometry")]
    let msdf_builder = msdf_builder.fix_geometry(true);

    let sdf_builder = msdf_builder.field_type(FieldType::Sdf);

    let (msdf, sdf) = {
        #[cfg(not(feature = "atlas"))]
        let (msdf, sdf) = (
            msdf_builder
                .build(char_data)
                .ok_or("Failed to create msdf")?,
            sdf_builder.build(char_data).ok_or("Failed to create sdf")?,
        );

        #[cfg(feature = "atlas")]
        let (msdf, sdf) = (
            msdf_builder
                .build_atlas(char_data.clone())
                .ok_or("Failed to create msdf atlas")?,
            sdf_builder
                .build_atlas(char_data)
                .ok_or("Failed to create sdf atlas")?,
        );

        (msdf, sdf)
    };

    image::save_buffer(
        "simple_msdf.png",
        &msdf.bitmap.bytes,
        msdf.bitmap.width as u32,
        msdf.bitmap.height as u32,
        image::ColorType::Rgb8,
    )?;

    image::save_buffer(
        "simple_sdf.png",
        &sdf.bitmap.bytes,
        sdf.bitmap.width as u32,
        sdf.bitmap.height as u32,
        image::ColorType::L8,
    )?;

    Ok(())
}
