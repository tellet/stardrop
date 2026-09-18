//! Benchmarks for hot, engine-free logic. Run with `cargo bench`.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use stardrop::debug::stats::summarize;

fn frame_summary(c: &mut Criterion) {
    // A full overlay window: 240 frames of ~60 FPS with some jitter.
    let frames: Vec<f64> = (0..240).map(|i| 16.0 + f64::from(i % 7) * 0.3).collect();
    c.bench_function("summarize 240 frame times", |b| {
        b.iter(|| summarize(black_box(&frames).iter().copied()));
    });
}

criterion_group!(benches, frame_summary);
criterion_main!(benches);
