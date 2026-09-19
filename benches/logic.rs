//! Benchmarks for hot, engine-free logic. Run with `cargo bench`.

use std::hint::black_box;

use bevy::math::Vec2;
use criterion::{Criterion, criterion_group, criterion_main};
use stardrop::debug::stats::summarize;
use stardrop::level::Level;
use stardrop::objects::{
    CloudPuffConfig, CrystalConfig, ObjectKind, ObjectsConfig, Placement, StaticCloudConfig,
    WindRibbonConfig, on_enter, while_inside,
};
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
        control_recovery_time: 0.5,
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

fn level_parse(c: &mut Criterion) {
    let text = include_str!("../assets/levels/test.level.ron");
    c.bench_function("parse + validate test level", |b| {
        b.iter(|| Level::parse(black_box(text)));
    });
}

const OBJECTS: ObjectsConfig = ObjectsConfig {
    cloud_puff: CloudPuffConfig {
        bounce_speed: 650.0,
        max_tilt: core::f32::consts::FRAC_PI_4,
    },
    wind_ribbon: WindRibbonConfig {
        flow_speed: 500.0,
        pull: 1800.0,
    },
    crystal: CrystalConfig {
        min_sideways_speed: 350.0,
    },
    static_cloud: StaticCloudConfig { hitbox_scale: 0.8 },
};

/// A busy one-screen level: `count` objects of mixed types spread over the
/// 1080 × 1920 playfield, including a few long ribbons.
fn busy_level(count: usize) -> Vec<Placement> {
    (0..count)
        .map(|i| {
            #[expect(clippy::cast_precision_loss, reason = "small counts")]
            let (column, row) = ((i % 6) as f32, (i / 6) as f32);
            let center = Vec2::new(-450.0 + column * 180.0, -150.0 - row * 230.0);
            match i % 4 {
                0 => Placement::CloudPuff {
                    center,
                    radius: 70.0,
                },
                1 => Placement::Crystal { center, size: 45.0 },
                2 => Placement::StaticCloud {
                    center,
                    radius: 80.0,
                },
                _ => Placement::WindRibbon {
                    path: (0..8)
                        .map(|p| {
                            #[expect(clippy::cast_precision_loss, reason = "small counts")]
                            let step = p as f32;
                            center + Vec2::new(step * 40.0, -step * 25.0)
                        })
                        .collect(),
                    width: 80.0,
                },
            }
        })
        .collect()
}

fn object_effects(c: &mut Criterion) {
    let body = Body {
        position: Vec2::new(30.0, -480.0),
        velocity: Vec2::new(240.0, -450.0),
        ..Body::default()
    };
    c.bench_function("cloud puff on_enter", |b| {
        b.iter(|| {
            on_enter(
                ObjectKind::CloudPuff,
                black_box(Vec2::new(0.0, -500.0)),
                &OBJECTS,
                black_box(body),
            )
        });
    });
    c.bench_function("crystal on_enter", |b| {
        b.iter(|| {
            on_enter(
                ObjectKind::Crystal,
                black_box(Vec2::new(0.0, -500.0)),
                &OBJECTS,
                black_box(body),
            )
        });
    });
    let ribbon = Placement::WindRibbon {
        path: (0..16)
            .map(|i| {
                #[expect(clippy::cast_precision_loss, reason = "small counts")]
                let step = i as f32;
                Vec2::new(-400.0 + step * 50.0, -300.0 - step * 20.0)
            })
            .collect(),
        width: 90.0,
    };
    c.bench_function("wind ribbon while_inside (16 points)", |b| {
        b.iter(|| while_inside(black_box(&ribbon), &OBJECTS, black_box(body), 1.0 / 120.0));
    });
}

/// Broad phase: which objects does the astronaut touch this step? Compares a
/// plain loop over every object with a uniform grid (240-unit cells) that
/// only checks objects registered in the astronaut's cell.
fn broad_phase(c: &mut Criterion) {
    const CELL: f32 = 240.0;
    let astronaut = Vec2::new(12.0, -900.0);
    for count in [12, 40] {
        let objects = busy_level(count);
        c.bench_function(&format!("broad phase: plain loop, {count} objects"), |b| {
            b.iter(|| {
                black_box(&objects)
                    .iter()
                    .filter(|object| object.touches(&OBJECTS, black_box(astronaut), 28.0))
                    .count()
            });
        });
        // Grid built once per level load (not measured): each object goes in
        // every cell its bounding box covers.
        let columns = 5_usize;
        let mut grid: Vec<Vec<usize>> = vec![Vec::new(); columns * 9];
        let cell_of = |point: Vec2| -> Option<usize> {
            let column = ((point.x + 540.0) / CELL).floor();
            let row = (-point.y / CELL).floor();
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "bench-only grid"
            )]
            let index = (row as usize) * columns + column as usize;
            (column >= 0.0 && row >= 0.0 && index < columns * 9).then_some(index)
        };
        for (i, object) in objects.iter().enumerate() {
            // Reach: object size plus the astronaut, as a square around its center.
            let reach = object.size()
                + 28.0
                + 400.0 * f32::from(u8::from(object.kind() == ObjectKind::WindRibbon));
            let (low, high) = (
                object.center() - Vec2::splat(reach),
                object.center() + Vec2::splat(reach),
            );
            let mut y = low.y;
            while y <= high.y {
                let mut x = low.x;
                while x <= high.x {
                    // A "let chain" (Rust 2024): match the cell *and* check it in one `if`.
                    if let Some(cell) = cell_of(Vec2::new(x, y))
                        && !grid[cell].contains(&i)
                    {
                        grid[cell].push(i);
                    }
                    x += CELL / 2.0;
                }
                y += CELL / 2.0;
            }
        }
        c.bench_function(&format!("broad phase: grid, {count} objects"), |b| {
            b.iter(|| {
                let Some(cell) = cell_of(black_box(astronaut)) else {
                    return 0;
                };
                grid[cell]
                    .iter()
                    .filter(|&&i| objects[i].touches(&OBJECTS, astronaut, 28.0))
                    .count()
            });
        });
    }
}

criterion_group!(
    benches,
    frame_summary,
    movement_step,
    level_parse,
    object_effects,
    broad_phase
);
criterion_main!(benches);
