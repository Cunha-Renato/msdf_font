use criterion::{Criterion, black_box, criterion_group, criterion_main};
use msdf_font::{AtlasBuilder, ttf_parser};

static FONT: &[u8] = include_bytes!("../assets/OpenSans.ttf");
static ASCII: &[char] = &[
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
];

fn bench_atlas_small(c: &mut Criterion) {
    let face = ttf_parser::Face::parse(FONT, 0).unwrap();

    c.bench_function("atlas_10_glyphs", |b| {
        b.iter(|| {
            GlyphBuilder::new(&face)
                .px_range(2)
                .px_size(40)
                .build_atlas(black_box(&ASCII[..10]))
                .unwrap()
        })
    });
}

fn bench_atlas_size(c: &mut Criterion) {
    let face = ttf_parser::Face::parse(FONT, 0).unwrap();
    let mut group = c.benchmark_group("atlas_glyph_count");

    for count in [10, 26, 52, 96] {
        let glyphs: Vec<char> = ASCII.iter().cycle().take(count).copied().collect();
        group.bench_with_input(format!("{count}_glyphs"), &glyphs, |b, g| {
            b.iter(|| {
                GlyphBuilder::new(&face)
                    .px_range(2)
                    .px_size(40)
                    .build_atlas(black_box(g))
                    .unwrap()
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_atlas_small, bench_atlas_size);
criterion_main!(benches);
