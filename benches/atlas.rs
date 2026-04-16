use criterion::{Criterion, criterion_group, criterion_main};
use msdf_font::{GlyphBuilder, ttf_parser};

static FONT: &[u8] = include_bytes!("../assets/OpenSans-Medium.ttf");
const ASCII: [char; 26] = [
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
];
const PX_RANGE: u32 = 2;
const PX_SIZE: u32 = 100;

fn bench_atlas_build(c: &mut Criterion) {
    let face = ttf_parser::Face::parse(FONT, 0).unwrap();

    c.bench_function("bench_atlas_build", |b| {
        b.iter(|| {
            GlyphBuilder::new(&face)
                .px_range(PX_RANGE)
                .px_size(PX_SIZE)
                .build_atlas(ASCII);
        })
    });
}

fn bench_sdf_atlas(c: &mut Criterion) {
    let face = ttf_parser::Face::parse(FONT, 0).unwrap();
    let mut glyph = GlyphBuilder::new(&face)
        .px_range(PX_RANGE)
        .px_size(PX_SIZE)
        .build_atlas(ASCII)
        .atlas
        .unwrap();

    c.bench_function("bench_sdf_atlas", |b| {
        b.iter(|| std::hint::black_box(glyph.sdf()))
    });
}

fn bench_msdf_atlas(c: &mut Criterion) {
    let face = ttf_parser::Face::parse(FONT, 0).unwrap();
    let mut glyph = GlyphBuilder::new(&face)
        .px_range(PX_RANGE)
        .px_size(PX_SIZE)
        .build_atlas(ASCII)
        .atlas
        .unwrap();

    c.bench_function("bench_msdf_atlas", |b| {
        b.iter(|| std::hint::black_box(glyph.msdf(3.0, true)))
    });
}

use std::time::Duration;
criterion_group!(
    name = benches_atlas;
    config = Criterion::default().measurement_time(Duration::from_secs(120));
    targets = bench_atlas_build, bench_sdf_atlas, bench_msdf_atlas
    // targets = bench_msdf_atlas
);
criterion_main!(benches_atlas);
