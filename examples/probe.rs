//! Design tool: search one level for a route and print what it found.
//!
//! `cargo run --release --example probe -- assets/levels/04_glass_garden.level.ron`
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a command-line tool: printing is the point"
)]

use bevy::math::Vec2;
use stardrop::config::GameConfig;
use stardrop::level::Level;
use stardrop::physics::movement::AirInput;
use stardrop::sim::{Attempt, Progress};
use stardrop::solver::{Goal, solve, suggested_target_time};

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: probe <level file>");
        return;
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("{path}: {error}");
            return;
        }
    };
    let level = match Level::parse(&text) {
        Ok(level) => level,
        Err(error) => {
            eprintln!("{path}: {error}");
            return;
        }
    };
    let config = GameConfig::default();

    // Simple strategies first: they show what kills a level outright.
    for (label, input) in [
        ("hold left ", AirInput::LEFT),
        ("no input  ", AirInput::NONE),
        ("hold right", AirInput::RIGHT),
    ] {
        let mut attempt = Attempt::new(&level);
        let mut progress = Progress::Falling;
        for _ in 0..3600 {
            progress = attempt.step(&level, &config, input);
            if progress != Progress::Falling {
                break;
            }
        }
        let at = attempt.body().position;
        println!(
            "{label}: {progress:?} after {:.2} s at ({:.0},{:.0})",
            attempt.elapsed(),
            at.x + 540.0,
            -at.y
        );
    }

    for goal in [Goal::Fastest, Goal::MostStardust] {
        match solve(&level, &config, goal) {
            Some(route) => {
                if goal == Goal::MostStardust {
                    let motes: Vec<String> = level
                        .stardust
                        .iter()
                        .map(|mote| format!("({:.0},{:.0})", mote.x + 540.0, -mote.y))
                        .collect();
                    println!("  motes: {}", motes.join(" "));
                }
                println!(
                    "{goal:?}: {:.2} s, stardust {}/{}, suggested target {:.1} s",
                    route.seconds,
                    route.stardust,
                    level.stardust.len(),
                    suggested_target_time(route.seconds)
                );
                print_path(&level, &config, &route.inputs);
                if goal == Goal::MostStardust {
                    report_missed(&level, &config, &route.inputs);
                }
            }
            None => println!("{goal:?}: no route found"),
        }
    }
}

/// Prints the route in level-file coordinates, every quarter second.
fn print_path(level: &Level, config: &GameConfig, inputs: &[AirInput]) {
    let mut attempt = Attempt::new(level);
    let mut path = Vec::new();
    for (step, input) in inputs.iter().enumerate() {
        if step % 30 == 0 {
            let at = attempt.body().position;
            path.push(format!("({:.0},{:.0})", at.x + 540.0, -at.y));
        }
        if attempt.step(level, config, *input) != Progress::Falling {
            break;
        }
    }
    println!("  {}", path.join(" "));
}

/// For every mote this route missed, how close it came and where.
fn report_missed(level: &Level, config: &GameConfig, inputs: &[AirInput]) {
    let mut attempt = Attempt::new(level);
    let mut closest = vec![(f32::MAX, Vec2::ZERO); level.stardust.len()];
    for input in inputs {
        let at = attempt.body().position;
        for (index, mote) in level.stardust.iter().enumerate() {
            let distance = at.distance(*mote);
            if distance < closest[index].0 {
                closest[index] = (distance, at);
            }
        }
        if attempt.step(level, config, *input) != Progress::Falling {
            break;
        }
    }
    for (index, (distance, at)) in closest.iter().enumerate() {
        if *distance > 38.0 {
            let mote = level.stardust[index];
            println!(
                "  missed mote {} at ({:.0},{:.0}): came within {distance:.0}, passing ({:.0},{:.0})",
                index + 1,
                mote.x + 540.0,
                -mote.y,
                at.x + 540.0,
                -at.y
            );
        }
    }
}
