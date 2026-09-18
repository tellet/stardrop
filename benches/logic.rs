//! Benchmarks for hot, engine-free logic. Run with `cargo bench`.

use std::hint::black_box;

use bevy::math::Vec2;
use criterion::{Criterion, criterion_group, criterion_main};
use stardrop::debug::stats::summarize;
use stardrop::physics::movement::{AirInput, Body, MovementConfig, Walls, step};

fn frame_summary(c: &mut Criterion) {
    // A full overlay window: 240 frames of ~60 FPS with some jitter.
    let frames: Vec<f64> = (0..240).map(|i| 16.0 + f64::from(i % 7) * 0.3).collect();
    c.bench_function("summarize 240 frame times", |b| {
        b.iter(|| summarize(black_box(&frames).iter().copied()));
    });
}

fn movement_step(c: &mut Criterion) {
    let config = MovementConfig {
        gravity: 450.0,
        terminal_velocity: 450.0,
        max_air_speed: 500.0,
        air_acceleration: 2000.0,
        air_braking: 1250.0,
        wall_restitution: 0.6,
        wall_recovery_time: 0.5,
        coast_drag: 100.0,
    };
    let walls = Walls {
        left: -932.0,
        right: 932.0,
    };
    let body = Body {
        position: Vec2::new(100.0, -2000.0),
        velocity: Vec2::new(250.0, -300.0),
        ..Body::default()
    };
    c.bench_function("movement step", |b| {
        b.iter(|| {
            step(
                black_box(body),
                black_box(AirInput::new(0.6)),
                &config,
                walls,
                1.0 / 120.0,
            )
        });
    });
}

criterion_group!(benches, frame_summary, movement_step);
criterion_main!(benches);
