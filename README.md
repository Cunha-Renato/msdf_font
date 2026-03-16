# msdf_font

[msdfgen](https://github.com/Chlumsky/msdfgen) with atlas support in rust.

Most of it is translated from the original C++ [msdfgen](https://github.com/Chlumsky/msdfgen), and some was taken from [fdsm](https://crates.io/crates/fdsm).

## Similar Crates

* [msdf](https://crates.io/crates/msdf) provides safe bindings for [msdfgen](https://github.com/Chlumsky/msdfgen)
* [fdsm](https://crates.io/crates/fdsm) rust implementation [msdfgen](https://crates.io/crates/fdsm) with a lot more similarities to the original

## Crate Features

* `atlas`: Atlas generation, based on glyph height.
* `fix_geometry`: For fonts that have overlapping contours, only behind a feature because it uses third party crates.

## Supports

* ✅ MSDF
* ✅ SDF
* ✅ Atlas generation (feature `atlas`)
* ✅ Shape correction (feature `fix_geometry`)
* ✅ Error correction (simple)
* ❌ Other types of distance fields

Here we can have a look at a glyph rendered without the `fix_geometry` feature.

![sdf_without_geometry_fix](assets/sdf_no_fix.png)
![msdf_without_geometry_fix](assets/msdf_no_fix.png)

And now with the fix in place.

![sdf_with_geometry_fix](assets/sdf_fix.png)
![msdf_with_geometry_fix](assets/msdf_fix.png)

If using `atlas` feature the resulting atlas will look something similar to this.

![msdf_atlas_with_geometry_fix](assets/msdf_atlas_fix.png)

Images generated from [OpenSans](https://fonts.google.com/specimen/Open+Sans) font.

## Usage

```rust
use image;
use msdf_font::{GlyphBuilder, FieldType, BitmapImageType, ttf_parser};
// use msdf_font::{AtlasBuilder} If using atlas feature.

fn main() {
    let face = ttf_parser::Face::parse(include_bytes("OpenSans.ttf"), 0).unwrap();

    let mut glyph = GlyphBuilder::new(&face)
        .field_type(FieldType::Msdf { max_angle: 3.0 })
        // .fix_geometry(true) If using fix_geometry feature.
        .px_range(2)
        .px_size(40)
        .build('A')
        // .build_atlas(['A', 'B', 'C', 'D']) If using atlas feature.
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

You can also see the examples([OpenSans](https://fonts.google.com/specimen/Open+Sans)) to check the other features.
