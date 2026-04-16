use criterion::{Criterion, criterion_group, criterion_main};
use msdf_font::{GlyphBuilder, ttf_parser};

static FONT: &[u8] = include_bytes!("../assets/OpenSans-Medium.ttf");
const PX_RANGE: u32 = 2;
const PX_SIZE: u32 = 100;

fn bench_build(c: &mut Criterion) {
    let face = ttf_parser::Face::parse(FONT, 0).unwrap();

    c.bench_function("glyph_build", |b| {
        b.iter(|| {
            GlyphBuilder::new(&face)
                .px_range(PX_RANGE)
                .px_size(PX_SIZE)
                .build(std::hint::black_box('A'))
                .unwrap()
        })
    });
}

fn bench_sdf(c: &mut Criterion) {
    let face = ttf_parser::Face::parse(FONT, 0).unwrap();
    let glyph = GlyphBuilder::new(&face)
        .px_range(PX_RANGE)
        .px_size(PX_SIZE)
        .build('A')
        .unwrap();

    c.bench_function("sdf_generate", |b| {
        b.iter(|| std::hint::black_box(glyph.sdf()))
    });
}

fn bench_msdf(c: &mut Criterion) {
    let face = ttf_parser::Face::parse(FONT, 0).unwrap();
    let mut glyph = GlyphBuilder::new(&face)
        .px_range(PX_RANGE)
        .px_size(PX_SIZE)
        .build('A')
        .unwrap();

    c.bench_function("msdf_generate", |b| {
        b.iter(|| std::hint::black_box(glyph.msdf(3.0, true)))
    });
}

// Benchmark across different glyph complexities
fn bench_glyph_complexity(c: &mut Criterion) {
    let face = ttf_parser::Face::parse(FONT, 0).unwrap();
    let mut group = c.benchmark_group("glyph_complexity");

    for ch in ['I', 'A', 'B', 'S', 'W', '@'] {
        let mut glyph = GlyphBuilder::new(&face)
            .px_range(PX_RANGE)
            .px_size(PX_SIZE)
            .build(ch)
            .unwrap();

        group.bench_function(format!("msdf_{ch}"), |b| {
            b.iter(|| std::hint::black_box(glyph.msdf(3.0, true)))
        });
    }

    group.finish();
}

// Benchmark different px_size values
fn bench_px_size(c: &mut Criterion) {
    let face = ttf_parser::Face::parse(FONT, 0).unwrap();
    let mut group = c.benchmark_group("px_size");

    for size in [16, 32, 64, 128] {
        let mut glyph = GlyphBuilder::new(&face)
            .px_range(PX_RANGE)
            .px_size(size)
            .build('A')
            .unwrap();

        group.bench_function(format!("msdf_{size}px"), |b| {
            b.iter(|| std::hint::black_box(glyph.msdf(3.0, true)))
        });
    }

    group.finish();
}

use std::time::Duration;
criterion_group!(
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(60));
    targets = bench_build,
    bench_sdf,
    bench_msdf,
    bench_glyph_complexity,
    bench_px_size,
);
criterion_main!(benches);
