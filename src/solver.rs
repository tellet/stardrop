//! Searches a level for a route to the beacon.
//!
//! A beam search: hold one steering choice for a short slice, keep the most
//! promising states, repeat. It proves levels are beatable and measures the
//! route, which sets each level's target time.

use crate::config::GameConfig;
use crate::level::Level;
use crate::physics::movement::AirInput;
use crate::sim::{Attempt, Progress};

/// Steps a single steering choice is held for (0.15 s).
const SLICE: usize = 18;
/// Give up after 30 s of game time.
const MAX_STEPS: usize = 30 * 120;
/// The beam keeps the best candidate in each cell of this grid, so it stays
/// spread across the screen instead of collapsing onto one line. Speed is
/// part of the cell: two astronauts in the same spot moving opposite ways
/// are heading for very different places.
const CELL_DEPTH: f32 = 120.0;
const CELL_X: f32 = 60.0;
const CELL_SPEED: f32 = 100.0;
/// Hard cap on the beam.
const BEAM: usize = 240;
/// The choices the search tries.
const CHOICES: [AirInput; 3] = [AirInput::LEFT, AirInput::NONE, AirInput::RIGHT];

/// What the search is looking for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Goal {
    /// Land as soon as possible.
    Fastest,
    /// Land with as much stardust as possible.
    MostStardust,
}

/// A route that lands.
#[derive(Debug, Clone, PartialEq)]
pub struct Route {
    /// One input per physics step.
    pub inputs: Vec<AirInput>,
    /// How long the route takes, in seconds.
    pub seconds: f32,
    /// How much stardust it collects.
    pub stardust: usize,
}

/// One state the search is carrying.
#[derive(Clone)]
struct Candidate {
    attempt: Attempt,
    inputs: Vec<AirInput>,
}

impl Candidate {
    /// Higher is better. Depth drives progress; stardust outranks it when
    /// that's the goal.
    fn score(&self, goal: Goal) -> f32 {
        let depth = self.attempt.depth();
        match goal {
            Goal::Fastest => depth,
            // Each mote is worth more than the whole level's depth.
            #[expect(clippy::cast_precision_loss, reason = "at most 64 motes")]
            Goal::MostStardust => self.attempt.stardust() as f32 * 10_000.0 + depth,
        }
    }

    /// Which grid cell this candidate sits in: where it is on screen, plus
    /// its stardust when that's what we're after.
    fn cell(&self, goal: Goal) -> (i32, i32, i32, i32, usize) {
        let at = self.attempt.body().position;
        // Values stay well inside i32 range for a one-screen level.
        #[expect(clippy::cast_possible_truncation, reason = "bucketing")]
        let bucket = |value: f32, size: f32| (value / size) as i32;
        let speed = self.attempt.body().velocity;
        (
            bucket(at.x, CELL_X),
            bucket(at.y, CELL_DEPTH),
            bucket(speed.x, CELL_SPEED),
            bucket(speed.y, CELL_SPEED),
            match goal {
                Goal::Fastest => 0,
                Goal::MostStardust => self.attempt.stardust(),
            },
        )
    }
}

/// Searches `level` for a route, or `None` if the search found none.
#[must_use]
pub fn solve(level: &Level, config: &GameConfig, goal: Goal) -> Option<Route> {
    let mut beam = vec![Candidate {
        attempt: Attempt::new(level),
        inputs: Vec::new(),
    }];
    let mut best: Option<Route> = None;
    let rounds = MAX_STEPS / SLICE;

    for _ in 0..rounds {
        let mut next: Vec<Candidate> = Vec::with_capacity(beam.len() * CHOICES.len());
        for candidate in &beam {
            for input in CHOICES {
                let mut branch = candidate.clone();
                let mut progress = Progress::Falling;
                for _ in 0..SLICE {
                    branch.inputs.push(input);
                    progress = branch.attempt.step(level, config, input);
                    if progress != Progress::Falling {
                        break;
                    }
                }
                match progress {
                    Progress::Landed => keep_best(&mut best, &branch, goal),
                    Progress::Failed => {}
                    Progress::Falling => next.push(branch),
                }
            }
        }
        if next.is_empty() {
            break;
        }
        beam = prune(next, goal);
    }
    best
}

/// Trims the candidates: best first, keeping one per grid cell.
///
/// The grid spans where the astronaut is (depth *and* across), so the beam
/// covers the whole screen. Without the across part it collapses onto one
/// line and misses objects sitting elsewhere.
fn prune(mut candidates: Vec<Candidate>, goal: Goal) -> Vec<Candidate> {
    candidates.sort_by(|a, b| b.score(goal).total_cmp(&a.score(goal)));
    let mut seen: Vec<(i32, i32, i32, i32, usize)> = Vec::with_capacity(BEAM);
    let mut kept = Vec::with_capacity(BEAM);
    for candidate in candidates {
        if kept.len() >= BEAM {
            break;
        }
        let cell = candidate.cell(goal);
        if seen.contains(&cell) {
            continue;
        }
        seen.push(cell);
        kept.push(candidate);
    }
    kept
}

/// Keeps `branch` if it beats the route found so far.
fn keep_best(best: &mut Option<Route>, branch: &Candidate, goal: Goal) {
    let route = Route {
        inputs: branch.inputs.clone(),
        seconds: branch.attempt.elapsed(),
        stardust: branch.attempt.stardust(),
    };
    let better = match (best.as_ref(), goal) {
        (None, _) => true,
        (Some(current), Goal::Fastest) => route.seconds < current.seconds,
        (Some(current), Goal::MostStardust) => {
            (route.stardust, -route.seconds) > (current.stardust, -current.seconds)
        }
    };
    if better {
        *best = Some(route);
    }
}

/// The target time for a level: the fastest route, with room to breathe.
#[must_use]
pub fn suggested_target_time(fastest_seconds: f32) -> f32 {
    // A clean run earns the star; a hesitant one doesn't (the user's choice).
    (fastest_seconds * 1.4 * 2.0).round() / 2.0
}

#[cfg(test)]
#[expect(clippy::print_stdout, reason = "these tests report level measurements")]
mod tests {
    use super::*;
    use crate::sim::Attempt;

    #[test]
    fn a_plain_level_is_solvable_and_the_route_replays() {
        let level = Level::for_tests();
        let config = GameConfig::default();
        let route = solve(&level, &config, Goal::Fastest).expect("a straight drop lands");
        assert!(route.seconds > 1.0 && route.seconds < 10.0, "{route:?}");
        // Replaying the route must land again: the search and the game agree.
        let (_, progress) = crate::sim::play(&level, &config, &route.inputs);
        assert_eq!(progress, Progress::Landed);
    }

    #[test]
    fn target_times_round_to_half_seconds() {
        assert!((suggested_target_time(10.0) - 14.0).abs() < 1e-6);
        assert!((suggested_target_time(5.2) - 7.5).abs() < 1e-6);
    }

    /// Every shipped level must be landable, and all its stardust reachable
    /// in one attempt, or its stars can't be earned.
    #[test]
    fn every_shipped_level_can_be_finished() {
        let config = GameConfig::default();
        for (name, file, level) in crate::level::tests::shipped_levels() {
            let fastest = solve(&level, &config, Goal::Fastest)
                .unwrap_or_else(|| panic!("{file}: no route to the beacon"));
            let thorough = solve(&level, &config, Goal::MostStardust)
                .unwrap_or_else(|| panic!("{file}: no route to the beacon"));
            assert_eq!(
                thorough.stardust,
                level.stardust.len(),
                "{file}: only {} of {} stardust is reachable in one attempt",
                thorough.stardust,
                level.stardust.len()
            );
            let suggested = suggested_target_time(fastest.seconds);
            assert!(
                level.target_time >= suggested - 0.01,
                "{file}: target_time {} is tighter than the suggested {suggested} \
                 (fastest route {:.1} s)",
                level.target_time,
                fastest.seconds
            );
            println!(
                "{name:<14} fastest {:>5.1} s  suggested target {suggested:>5.1} s  \
                 set {:>5.1} s  stardust {}/{}",
                fastest.seconds,
                level.target_time,
                thorough.stardust,
                level.stardust.len()
            );
        }
    }

    /// Letting go must not beat a level. The first two teach the controls,
    /// so a straight drop may land there; from the third on it must miss.
    #[test]
    fn a_plain_fall_misses_from_the_third_level_on() {
        let config = GameConfig::default();
        for (number, (name, _, level)) in crate::level::tests::shipped_levels()
            .into_iter()
            .enumerate()
            .skip(2)
        {
            let inputs = [AirInput::NONE; MAX_STEPS];
            let (_, progress) = crate::sim::play(&level, &config, &inputs);
            assert_ne!(
                progress,
                Progress::Landed,
                "level {}, {name}: a straight drop lands on the beacon",
                number + 1
            );
        }
    }

    /// How often careless play lands: the honest measure of how hard a
    /// level is. Plays a few hundred coarse random runs per level — the
    /// sort of thing a player does on a first look — and counts the ones
    /// that reach the beacon.
    ///
    /// `cargo test --release --lib -- --ignored difficulty --nocapture`
    #[test]
    #[ignore = "a measurement, not a check"]
    fn difficulty_of_every_level() {
        use crate::rng::SplitMix64;

        let config = GameConfig::default();
        for (number, (name, _, level)) in crate::level::tests::shipped_levels()
            .into_iter()
            .enumerate()
        {
            let seed = 0x5041_594C_4F41_4400 + u64::try_from(number).unwrap_or(0);
            let mut rng = SplitMix64::new(seed);
            let tries = 300;
            let mut landed = 0;
            for _ in 0..tries {
                // Careless play: hold a direction for a third of a second,
                // then pick again.
                let mut inputs = Vec::with_capacity(MAX_STEPS);
                while inputs.len() < MAX_STEPS {
                    // A third each way: left, straight, right.
                    let roll = rng.range(0.0, 3.0);
                    let choice = if roll < 1.0 {
                        CHOICES[0]
                    } else if roll < 2.0 {
                        CHOICES[1]
                    } else {
                        CHOICES[2]
                    };
                    for _ in 0..40 {
                        inputs.push(choice);
                    }
                }
                if crate::sim::play(&level, &config, &inputs).1 == Progress::Landed {
                    landed += 1;
                }
            }
            println!(
                "{:>2}. {name:<14} careless play lands {:>5.1}% of the time",
                number + 1,
                f64::from(landed) * 100.0 / f64::from(tries)
            );
        }
    }

    /// Prints where the fastest route goes, to place stardust along it.
    /// Run on demand: `cargo test --release print_routes -- --ignored --nocapture`.
    #[test]
    #[ignore = "a design tool, not a check"]
    fn print_routes() {
        let config = GameConfig::default();
        for (name, _, level) in crate::level::tests::shipped_levels() {
            let Some(route) = solve(&level, &config, Goal::MostStardust) else {
                println!("{name}: no route");
                continue;
            };
            let mut attempt = Attempt::new(&level);
            let mut path = Vec::new();
            for (step, input) in route.inputs.iter().enumerate() {
                if step % 30 == 0 {
                    let at = attempt.body().position;
                    // Back to file coordinates, which is how levels are written.
                    path.push(format!("({:.0},{:.0})", at.x + 540.0, -at.y));
                }
                if attempt.step(&level, &config, *input) != Progress::Falling {
                    break;
                }
            }
            println!(
                "{name}: {} s, stardust {} -> {}",
                route.seconds,
                route.stardust,
                path.join(" ")
            );
        }
    }
}
