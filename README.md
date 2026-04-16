# msdf_font

[msdfgen](https://github.com/Chlumsky/msdfgen) with atlas support in rust.

Most of it is translated from the original C++ [msdfgen](https://github.com/Chlumsky/msdfgen), and some was taken from [fdsm](https://crates.io/crates/fdsm).

## Similar Crates

* [msdf](https://crates.io/crates/msdf) provides safe bindings for [msdfgen](https://github.com/Chlumsky/msdfgen)
* [fdsm](https://crates.io/crates/fdsm) rust implementation [msdfgen](https://crates.io/crates/fdsm) with a lot more similarities to the original

## Crate Features

* `atlas`: Atlas generation, based on glyph height. Currently this feature uses the [rayon](https://crates.io/crates/rayon) crate.

## Supports

* ✅ MSDF
* ✅ SDF
* ✅ Shape correction
* ✅ Error correction
* ✅ Atlas generation (feature `atlas`)
* ❌ Other types of distance fields

If using `atlas` feature the resulting atlas will look something similar to this.

![msdf_atlas_with_geometry_fix](assets/msdf_atlas_fix.png)

Images generated from [OpenSans](https://fonts.google.com/specimen/Open+Sans) font.

## Usage

Default
```rust
use image;
use msdf_font::{GlyphBuilder, FieldType, BitmapImageType, ttf_parser};

fn main() {
    let face = ttf_parser::Face::parse(include_bytes("OpenSans.ttf"), 0).unwrap();

    let mut glyph = GlyphBuilder::new(&face)
        .px_range(2)
        .px_size(40)
        .build('A')
        .unwrap();

    let msdf = glyph.msdf(3.0, true);

    image::save_buffer(
        "image.png",
        &msdf.bytes(),
        msdf.width as u32,
        msdf.height as u32,
        image::ColorType::Rgb8,
    ).unwrap();
}
```
With `atlas` feature.
```rust
use image;
use msdf_font::{GlyphBuilder, FieldType, BitmapImageType, ttf_parser};

fn main() {
    let face = ttf_parser::Face::parse(include_bytes("OpenSans.ttf"), 0).unwrap();

    let mut atlas_result = GlyphBuilder::new(&face)
        .px_range(2)
        .px_size(40)
        .build(['A', 'B', 'C']);
    
    if let Some(rejected_chars) = atlas_result.rejected {
        println!("Rejected {} chars.", rejected_chars.len());
    }

    let mut atlas = atlas_result.atlas.unwrap();

    let msdf = glyph.msdf(3.0, true);

    image::save_buffer(
        "image.png",
        &msdf.bytes(),
        msdf.width as u32,
        msdf.height as u32,
        image::ColorType::Rgb8,
    ).unwrap();
}
```

You can also see the examples([OpenSans](https://fonts.google.com/specimen/Open+Sans)) to check usage.
