//! Guards what the game looks like.
//!
//! Draws every object type and the astronaut in a fixed grid, renders it, and
//! compares the result against a reference committed to the repository. The
//! suite can't see the screen, so a shape can break silently; this is the one
//! check that looks.
//!
//! ```text
//! cargo run --release --example golden            # compare, fail on drift
//! cargo run --release --example golden -- --write # accept the current look
//! ```
//!
//! It needs a GPU, so it stays out of CI and runs from `check.sh --visual`.
//!
//! The reference is not a picture. It is the image reduced to a grid of
//! brightness readings (`tests/golden/shapes.txt`), which is small enough to
//! read in a diff, survives harmless antialiasing noise, and still catches a
//! silhouette that has changed, a part that has gone missing, or a layer that
//! has slipped behind another.
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a command-line tool: printing is the point"
)]

use std::fs;
use std::path::Path;

use bevy::camera::{Hdr, ScalingMode};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use bevy::window::{WindowResolution, WindowTheme};
use stardrop::config::GameConfig;
use stardrop::objects::{ObjectAssets, ObjectKind, preview_placement, spawn_preview};

/// The reference, and the window the shapes are drawn in.
const REFERENCE: &str = "tests/golden/shapes.txt";
const WINDOW: (u32, u32) = (900, 600);
/// What the camera shows, in world units: wide enough for a 5x2 grid.
const VIEW: Vec2 = Vec2::new(1350.0, 900.0);
/// How many readings across and down the fingerprint holds.
const GRID: (usize, usize) = (45, 30);
/// A cell counts as changed past this much brightness (0-255), and the check
/// fails past this many changed cells.
const CELL_TOLERANCE: i32 = 10;
const CHANGED_ALLOWED: usize = 8;
/// Frames to let the renderer settle before the picture is taken.
const SETTLE: u32 = 8;

fn main() {
    let write = std::env::args().any(|argument| argument == "--write");
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.02, 0.015, 0.06)))
        .insert_resource(Mode(write))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Stardrop shapes".into(),
                resolution:
                    WindowResolution::new(WINDOW.0, WINDOW.1).with_scale_factor_override(1.0),
                window_theme: Some(WindowTheme::Dark),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(stardrop::config::ConfigPlugin)
        .init_resource::<ObjectAssets>()
        .add_systems(Startup, lay_out_the_shapes)
        .add_systems(Update, take_the_picture)
        .run();
}

/// Whether this run writes the reference or checks against it.
#[derive(Resource)]
struct Mode(bool);

fn lay_out_the_shapes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
    assets: Res<ObjectAssets>,
    config: Res<GameConfig>,
) {
    commands.spawn((
        Camera2d,
        Hdr,
        Tonemapping::TonyMcMapface,
        stardrop::camera::bloom(0.3),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: VIEW.x,
                min_height: VIEW.y,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));

    // Nine objects and the astronaut, five across and two down.
    let step = Vec2::new(VIEW.x / 5.0, VIEW.y / 2.0);
    let corner = Vec2::new(-VIEW.x / 2.0 + step.x / 2.0, step.y / 2.0);
    for (index, kind) in ObjectKind::ALL.into_iter().enumerate() {
        // Rows of five; `index` is at most 9.
        #[expect(clippy::cast_precision_loss, reason = "ten shapes")]
        let at = corner + Vec2::new((index % 5) as f32 * step.x, -((index / 5) as f32) * step.y);
        spawn_preview(
            &mut commands,
            &assets,
            &mut meshes,
            &config.objects,
            &preview_placement(kind, at),
            (),
        );
    }
    let last = corner + Vec2::new(4.0 * step.x, -step.y);
    stardrop::astronaut::spawn_figure(
        &mut commands,
        meshes,
        materials,
        config.astronaut_radius * 2.2,
        last,
        (),
    );
}

/// Waits for the renderer to settle, takes one picture, and compares it.
fn take_the_picture(mut commands: Commands, mut frame: Local<u32>, mode: Res<Mode>) {
    *frame += 1;
    if *frame != SETTLE {
        return;
    }
    let write = mode.0;
    commands.spawn(Screenshot::primary_window()).observe(
        move |captured: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
            exit.write(judge(&captured.image, write));
        },
    );
}

/// Turns the picture into a fingerprint and either writes or checks it.
fn judge(image: &Image, write: bool) -> AppExit {
    let Some(fingerprint) = fingerprint(image) else {
        eprintln!("the picture came back in a format this check can't read");
        return AppExit::error();
    };
    let path = Path::new(REFERENCE);
    if write {
        if let Some(folder) = path.parent() {
            let _ = fs::create_dir_all(folder);
        }
        match fs::write(path, to_text(&fingerprint)) {
            Ok(()) => {
                println!("wrote {REFERENCE}");
                AppExit::Success
            }
            Err(error) => {
                eprintln!("{REFERENCE}: {error}");
                AppExit::error()
            }
        }
    } else {
        match fs::read_to_string(path) {
            Ok(text) => compare(&from_text(&text), &fingerprint),
            Err(error) => {
                eprintln!("{REFERENCE}: {error}. Run with --write to accept the current look.");
                AppExit::error()
            }
        }
    }
}

/// Reduces the picture to a grid of brightness readings.
fn fingerprint(image: &Image) -> Option<Vec<u8>> {
    let data = image.data.as_ref()?;
    let (width, height) = (image.width() as usize, image.height() as usize);
    let channels = data.len().checked_div(width * height)?;
    if channels < 3 {
        return None;
    }
    let mut cells = Vec::with_capacity(GRID.0 * GRID.1);
    for row in 0..GRID.1 {
        for column in 0..GRID.0 {
            let from = (column * width / GRID.0, row * height / GRID.1);
            let to = ((column + 1) * width / GRID.0, (row + 1) * height / GRID.1);
            let mut total = 0_u64;
            let mut count = 0_u64;
            for y in from.1..to.1 {
                for x in from.0..to.0 {
                    let at = (y * width + x) * channels;
                    // Rough luminance: green carries most of what the eye sees.
                    let value = u64::from(data[at]) * 2
                        + u64::from(data[at + 1]) * 5
                        + u64::from(data[at + 2]);
                    total += value / 8;
                    count += 1;
                }
            }
            #[expect(clippy::cast_possible_truncation, reason = "an average of bytes")]
            cells.push((total / count.max(1)) as u8);
        }
    }
    Some(cells)
}

fn to_text(cells: &[u8]) -> String {
    let mut text = String::with_capacity(cells.len() * 3);
    text.push_str("# Stardrop's shapes, as brightness per cell. Written by\n");
    text.push_str("# `cargo run --release --example golden -- --write`.\n");
    for row in cells.chunks(GRID.0) {
        for cell in row {
            text.push_str(&format!("{cell:02x}"));
        }
        text.push('\n');
    }
    text
}

fn from_text(text: &str) -> Vec<u8> {
    text.lines()
        .filter(|line| !line.starts_with('#'))
        .flat_map(|line| {
            line.as_bytes()
                .chunks(2)
                .filter_map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
                .collect::<Vec<u8>>()
        })
        .collect()
}

/// Reports what moved, and fails if too much did.
fn compare(reference: &[u8], now: &[u8]) -> AppExit {
    if reference.len() != now.len() {
        eprintln!(
            "the reference holds {} cells and this run made {}: delete it and --write a new one",
            reference.len(),
            now.len()
        );
        return AppExit::error();
    }
    let changed: Vec<usize> = (0..now.len())
        .filter(|&index| {
            (i32::from(reference[index]) - i32::from(now[index])).abs() > CELL_TOLERANCE
        })
        .collect();
    if changed.len() <= CHANGED_ALLOWED {
        println!(
            "the shapes are unchanged ({} of {} cells differ at all)",
            changed.len(),
            now.len()
        );
        return AppExit::Success;
    }
    eprintln!(
        "the shapes have changed: {} of {} cells differ by more than {CELL_TOLERANCE}",
        changed.len(),
        now.len()
    );
    // A map of where, so the difference can be read without opening anything.
    for row in 0..GRID.1 {
        let line: String = (0..GRID.0)
            .map(|column| {
                if changed.contains(&(row * GRID.0 + column)) {
                    '#'
                } else {
                    '.'
                }
            })
            .collect();
        eprintln!("  {line}");
    }
    eprintln!("If this was on purpose, run the example with --write.");
    AppExit::error()
}
