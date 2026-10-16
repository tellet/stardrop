//! The planet the astronaut lands on, generated from one seed.
//!
//! No Bevy beyond `Vec2`: colours leave here as hue, saturation and lightness
//! numbers (`Shade`), and `art.rs` turns those into materials. So a planet can
//! be generated and checked without a running `App`.

use bevy::math::Vec2;

use crate::config::ASTRONAUT_RADIUS;
use crate::level::LEVEL_WIDTH;
use crate::rng::SplitMix64;

/// Height samples across the generated ground, 11.25 world units apart
/// (Section 11). The ground is wider than the playfield, so it takes more
/// than the 96 the playfield alone would need.
pub(super) const SAMPLES: usize = 154;
/// The most props one planet can carry: nothing here is unbounded (Section 7.2).
pub(super) const PROPS_MAX: usize = 24;
/// The fewest, so a planet is never bare.
const PROPS_MIN: usize = 10;
/// The ground is generated wider than the playfield, so a window of any shape
/// sees ground to both edges rather than sky under the horizon.
pub(super) const GROUND_WIDTH: f32 = LEVEL_WIDTH * 1.6;
/// Where the surface sits in world y. Heights are measured from here.
pub(super) const SURFACE_Y: f32 = -1450.0;
/// How far the ground may rise or fall from the surface.
pub(super) const MAX_RELIEF: f32 = 320.0;
/// The slope Section 11 asks the astronaut to land on. A planet of jagged
/// rock may offer nothing this gentle, and then the flattest ground within
/// reach is used instead.
const MAX_SLOPE: f32 = 0.15;
/// Props keep this far from the landing spot: three astronaut radii.
const CLEARANCE: f32 = 84.0;
/// Heights are blended across this many samples where two grounds meet, so a
/// desert runs into a forest instead of ending at a wall.
const SEAM: f32 = 10.0;
/// Water lies this far below the ground it cuts into.
const WATER_DEPTH: f32 = 40.0;
/// Props rolled onto water are kept to its bank, within this of the waterline.
const BANK: f32 = 60.0;
/// A water span never covers more of the planet than this.
const MAX_WATER: f32 = 0.4;
/// The landing spot is looked for this far either side of the middle.
const LANDING_REACH: f32 = LEVEL_WIDTH * 0.3;
/// And this far, if nothing gentle enough is any nearer. The pack stands
/// 2.6 radii to the right of the astronaut and is 4.5 wide, so this is as far
/// out as the whole scene still fits on a portrait screen.
const WIDER_REACH: f32 = 400.0;
/// Prop sizes are written in astronaut radii, so they read as "twice as tall
/// as the person" rather than as numbers.
const R: f32 = ASTRONAUT_RADIUS;

/// What the ground is made of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GroundKind {
    Rock,
    Desert,
    Water,
    Overgrowth,
}

impl GroundKind {
    const ALL: [Self; 4] = [Self::Rock, Self::Desert, Self::Water, Self::Overgrowth];
    /// The three a whole planet can be made of. Nobody lands on open water.
    const DRY: [Self; 3] = [Self::Rock, Self::Desert, Self::Overgrowth];
}

/// Something growing or standing on the ground.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PropKind {
    Mushroom,
    GrapeCluster,
    Flower,
    Boulder,
    SteamVent,
    Reed,
}

impl PropKind {
    const ALL: [Self; 6] = [
        Self::Mushroom,
        Self::GrapeCluster,
        Self::Flower,
        Self::Boulder,
        Self::SteamVent,
        Self::Reed,
    ];
}

/// A colour as three numbers, so this module needs no colour type.
#[derive(Debug, Clone, Copy)]
pub(super) struct Shade {
    /// Degrees around the wheel, 0 to 360.
    pub hue: f32,
    pub saturation: f32,
    pub lightness: f32,
}

/// Every colour one planet is painted in.
#[derive(Debug, Clone, Copy)]
pub(super) struct Palette {
    /// Four bands, top of the sky first.
    pub sky: [Shade; 4],
    pub ground: Shade,
    /// The ground's mass, further down and darker.
    pub ground_far: Shade,
    pub water: Shade,
    pub prop: Shade,
    /// The one bright colour: flower middles, the pack's lamps, the flag.
    pub accent: Shade,
}

/// One stretch of ground across the planet.
#[derive(Debug, Clone, Copy)]
pub(super) struct GroundSpan {
    pub kind: GroundKind,
    pub from: f32,
    pub to: f32,
}

/// One thing standing on the ground, placed but not yet drawn.
#[derive(Debug, Clone, Copy)]
pub(super) struct Prop {
    pub kind: PropKind,
    /// Where it stands, in world coordinates.
    pub at: Vec2,
    /// How big, in world units: what that measures depends on the kind.
    pub size: f32,
    /// Its own seed, so `art.rs` can vary it without affecting anything else.
    pub seed: u64,
}

/// A whole planet: what it is made of, what it looks like, and where the
/// astronaut can put their feet.
#[derive(Debug, Clone)]
pub(super) struct Planet {
    pub palette: Palette,
    /// Height above (or below) `SURFACE_Y`, left edge first.
    pub ground: [f32; SAMPLES],
    pub spans: Vec<GroundSpan>,
    pub props: Vec<Prop>,
    pub landing_x: f32,
}

/// How much a ground kind rises and falls, and over how few lattice points.
/// Fewer points make longer, smoother features; the second octave is what
/// turns a hill into a ridge.
struct Relief {
    amplitude: f32,
    points: usize,
    detail: f32,
    detail_points: usize,
}

fn relief(kind: GroundKind) -> Relief {
    match kind {
        // Jagged ridges and canyons.
        GroundKind::Rock => Relief {
            amplitude: 260.0,
            points: 9,
            detail: 0.5,
            detail_points: 17,
        },
        // Long smooth dunes.
        GroundKind::Desert => Relief {
            amplitude: 120.0,
            points: 5,
            detail: 0.0,
            detail_points: 2,
        },
        // Soft mounds.
        GroundKind::Overgrowth => Relief {
            amplitude: 90.0,
            points: 7,
            detail: 0.25,
            detail_points: 13,
        },
        // Flat: `heightfield` sets water from the ground it cuts into.
        GroundKind::Water => Relief {
            amplitude: 0.0,
            points: 2,
            detail: 0.0,
            detail_points: 2,
        },
    }
}

/// How often each prop grows on each ground, out of that row's total. A zero
/// means never: boulders do not grow in lakes, and mushrooms dislike bare rock.
fn weight(ground: GroundKind, prop: PropKind) -> u8 {
    // One row per ground, in the order of `PropKind::ALL`: mushroom, grapes,
    // flower, boulder, steam vent, reed. A zero means never, so boulders do
    // not grow in lakes and mushrooms dislike bare rock. Water's row is for
    // its bank: reeds in the shallows, and a little of whatever grows on an
    // edge.
    let row: [u8; PropKind::ALL.len()] = match ground {
        GroundKind::Rock => [0, 0, 1, 6, 3, 0],
        GroundKind::Desert => [0, 0, 1, 3, 1, 2],
        GroundKind::Overgrowth => [5, 4, 5, 1, 1, 3],
        GroundKind::Water => [1, 1, 2, 1, 0, 6],
    };
    row[prop as usize]
}

/// Where a height sample sits, in world x.
pub(super) fn sample_x(index: usize) -> f32 {
    #[expect(clippy::cast_precision_loss, reason = "SAMPLES is 154")]
    let along = index as f32 / (SAMPLES - 1) as f32;
    -GROUND_WIDTH / 2.0 + along * GROUND_WIDTH
}

/// The space between two height samples, in world units.
fn step() -> f32 {
    #[expect(clippy::cast_precision_loss, reason = "SAMPLES is 96")]
    let last = (SAMPLES - 1) as f32;
    GROUND_WIDTH / last
}

/// One in `count` of these, uniformly in -1..1: the lattice a height curve is
/// read off.
fn lattice(rng: &mut SplitMix64, count: usize) -> Vec<f32> {
    (0..count.max(2)).map(|_| rng.range(-1.0, 1.0)).collect()
}

/// The lattice read at `t` (0 to 1), smoothed so the curve has no corners.
fn read(lattice: &[f32], t: f32) -> f32 {
    #[expect(clippy::cast_precision_loss, reason = "a handful of points")]
    let span = (lattice.len() - 1) as f32;
    let at = (t.clamp(0.0, 1.0) * span).max(0.0);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0..span"
    )]
    let index = (at.floor() as usize).min(lattice.len() - 1);
    let next = (index + 1).min(lattice.len() - 1);
    #[expect(clippy::cast_precision_loss, reason = "a handful of points")]
    let fraction = at - index as f32;
    // Smoothstep: the slope is zero at both ends, so lattice points don't show.
    let smooth = fraction * fraction * (3.0 - 2.0 * fraction);
    lattice[index] * (1.0 - smooth) + lattice[next] * smooth
}

/// One ground kind's height curve, sampled across the whole planet.
fn curve(rng: &mut SplitMix64, kind: GroundKind) -> [f32; SAMPLES] {
    let shape = relief(kind);
    let coarse = lattice(rng, shape.points);
    let fine = lattice(rng, shape.detail_points);
    let mut heights = [0.0; SAMPLES];
    for (index, height) in heights.iter_mut().enumerate() {
        #[expect(clippy::cast_precision_loss, reason = "SAMPLES is 96")]
        let t = index as f32 / (SAMPLES - 1) as f32;
        *height = shape.amplitude * (read(&coarse, t) + shape.detail * read(&fine, t));
    }
    heights
}

/// One or two stretches of ground, left to right across the planet.
fn spans_of(rng: &mut SplitMix64) -> Vec<GroundSpan> {
    let half = GROUND_WIDTH / 2.0;
    let pick = |rng: &mut SplitMix64, from: &[GroundKind]| {
        #[expect(clippy::cast_possible_truncation, reason = "four kinds at most")]
        let index = (rng.next_u64() % from.len() as u64) as usize;
        from[index]
    };
    // A third of planets are one ground all the way across, and never water:
    // the astronaut has to be able to stand somewhere.
    if rng.next_f32() < 0.35 {
        return vec![GroundSpan {
            kind: pick(rng, &GroundKind::DRY),
            from: -half,
            to: half,
        }];
    }
    let left = pick(rng, &GroundKind::ALL);
    // Always a different kind, so a mixed planet reads as a mix. Stepping on
    // by a random amount keeps that bounded instead of re-rolling.
    let step_on = 1 + (rng.next_u64() % 3) as usize;
    let left_index = GroundKind::ALL
        .iter()
        .position(|kind| *kind == left)
        .unwrap_or(0);
    let right = GroundKind::ALL[(left_index + step_on) % GroundKind::ALL.len()];
    // Water is held to `MAX_WATER` of the width, which is what guarantees dry
    // land in the middle for the astronaut.
    let seam = if left == GroundKind::Water {
        rng.range(-half, -half + GROUND_WIDTH * MAX_WATER)
    } else if right == GroundKind::Water {
        rng.range(half - GROUND_WIDTH * MAX_WATER, half)
    } else {
        rng.range(-GROUND_WIDTH * 0.3, GROUND_WIDTH * 0.3)
    };
    vec![
        GroundSpan {
            kind: left,
            from: -half,
            to: seam,
        },
        GroundSpan {
            kind: right,
            from: seam,
            to: half,
        },
    ]
}

/// The heightfield: each span's own curve, blended across the seam, with
/// water sitting flat below the ground it cuts into.
fn heightfield(rng: &mut SplitMix64, spans: &[GroundSpan]) -> [f32; SAMPLES] {
    let curves: Vec<[f32; SAMPLES]> = spans.iter().map(|span| curve(rng, span.kind)).collect();
    let mut heights = [0.0; SAMPLES];
    let half = GROUND_WIDTH / 2.0;
    let blend = SEAM * step();
    for (index, height) in heights.iter_mut().enumerate() {
        #[expect(clippy::cast_precision_loss, reason = "SAMPLES is 96")]
        let x = -half + index as f32 * step();
        let mut value = curves[0][index];
        if let (Some(second), Some(first_span)) = (curves.get(1), spans.first()) {
            // 0 on the left of the seam, 1 on its right, smooth between.
            let across = ((x - first_span.to) / blend).clamp(-1.0, 1.0) * 0.5 + 0.5;
            let smooth = across * across * (3.0 - 2.0 * across);
            value = value * (1.0 - smooth) + second[index] * smooth;
        }
        *height = value;
    }
    // Water is a level surface, set from the mean of the ground beside it.
    for (span_index, span) in spans.iter().enumerate() {
        if span.kind != GroundKind::Water {
            continue;
        }
        let other = spans
            .iter()
            .enumerate()
            .find(|(index, _)| *index != span_index)
            .map_or(0.0, |(index, _)| mean(&curves[index]));
        let level = other - WATER_DEPTH;
        for (index, height) in heights.iter_mut().enumerate() {
            #[expect(clippy::cast_precision_loss, reason = "SAMPLES is 96")]
            let x = -half + index as f32 * step();
            if x >= span.from && x <= span.to {
                // Flat, apart from the blend across its own shoreline.
                let to_shore = ((x - span.from).abs()).min((x - span.to).abs());
                let shore = (to_shore / blend).clamp(0.0, 1.0);
                *height = *height * (1.0 - shore) + level * shore;
            }
        }
    }
    for height in &mut heights {
        *height = height.clamp(-MAX_RELIEF, MAX_RELIEF);
    }
    heights
}

fn mean(heights: &[f32; SAMPLES]) -> f32 {
    #[expect(clippy::cast_precision_loss, reason = "SAMPLES is 96")]
    let count = SAMPLES as f32;
    heights.iter().sum::<f32>() / count
}

/// Where the astronaut puts their feet: the flattest dry sample near the
/// middle of the screen. Always answers, because a planet always has land.
fn landing_spot(heights: &[f32; SAMPLES], spans: &[GroundSpan]) -> f32 {
    let dry = |reach: f32| -> Vec<(f32, f32)> {
        (1..SAMPLES - 1)
            .filter_map(|index| {
                let x = sample_x(index);
                // On screen, and never in the water.
                if x.abs() > reach || kind_of(spans, x) == GroundKind::Water {
                    return None;
                }
                let slope = (heights[index + 1] - heights[index - 1]) / (2.0 * step());
                Some((x, slope.abs()))
            })
            .collect()
    };
    // Gentle enough to stand on, as near the middle of the screen as can be.
    let gentlest = |candidates: &[(f32, f32)]| {
        candidates
            .iter()
            .filter(|(_, slope)| *slope < MAX_SLOPE)
            .min_by(|(left, _), (right, _)| left.abs().total_cmp(&right.abs()))
            .map(|(x, _)| *x)
    };
    let flattest = |candidates: &[(f32, f32)]| {
        candidates
            .iter()
            .min_by(|(left_x, left_slope), (right_x, right_slope)| {
                left_slope
                    .total_cmp(right_slope)
                    .then_with(|| left_x.abs().total_cmp(&right_x.abs()))
            })
            .map(|(x, _)| *x)
    };
    let near = dry(LANDING_REACH);
    if let Some(x) = gentlest(&near) {
        return x;
    }
    // Nothing gentle in the middle: look further out before settling for a
    // slope, because a planet of jagged rock usually has a shelf somewhere.
    let wide = dry(WIDER_REACH);
    gentlest(&wide)
        .or_else(|| flattest(&wide))
        // A planet always has dry land in the middle (`spans_of`), so this
        // answers; zero is the centre of the screen if it somehow could not.
        .unwrap_or(0.0)
}

/// Which ground covers this x.
fn kind_of(spans: &[GroundSpan], x: f32) -> GroundKind {
    spans
        .iter()
        .find(|span| x >= span.from && x <= span.to)
        .or_else(|| spans.last())
        .map_or(GroundKind::Rock, |span| span.kind)
}

/// What one prop of this kind measures across or up, in world units.
fn prop_size(rng: &mut SplitMix64, kind: PropKind) -> f32 {
    match kind {
        // The cap, across.
        PropKind::Mushroom => rng.range(1.4, 4.0) * R,
        // The whole bunch, across.
        PropKind::GrapeCluster => rng.range(2.0, 4.0) * R,
        PropKind::Flower => rng.range(0.6, 1.2) * R,
        PropKind::Boulder => rng.range(0.8, 2.5) * R,
        // Up.
        PropKind::Reed => rng.range(2.0, 5.0) * R,
        // The crack in the ground, across.
        PropKind::SteamVent => rng.range(1.0, 2.0) * R,
    }
}

impl Planet {
    /// Generates the planet for `seed`. The same seed is always the same
    /// planet, down to the last mushroom.
    pub(super) fn generate(seed: u64) -> Self {
        let mut rng = SplitMix64::new(seed);
        let palette = palette_of(&mut rng);
        let spans = spans_of(&mut rng);
        let ground = heightfield(&mut rng, &spans);
        let landing_x = landing_spot(&ground, &spans);
        let props = props_of(&mut rng, &ground, &spans, landing_x);
        Self {
            palette,
            ground,
            spans,
            props,
            landing_x,
        }
    }

    /// The ground's height in world y at this x, between samples.
    pub(super) fn height_at(&self, x: f32) -> f32 {
        let t = (x + GROUND_WIDTH / 2.0) / GROUND_WIDTH;
        #[expect(clippy::cast_precision_loss, reason = "SAMPLES is 96")]
        let span = (SAMPLES - 1) as f32;
        let at = (t.clamp(0.0, 1.0) * span).max(0.0);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to 0..span"
        )]
        let index = (at.floor() as usize).min(SAMPLES - 1);
        let next = (index + 1).min(SAMPLES - 1);
        #[expect(clippy::cast_precision_loss, reason = "SAMPLES is 96")]
        let fraction = at - index as f32;
        SURFACE_Y + self.ground[index] * (1.0 - fraction) + self.ground[next] * fraction
    }
}

/// Everything standing on one planet: rolled against the ground each one
/// lands on, so a planet's life belongs to its ground.
fn props_of(
    rng: &mut SplitMix64,
    heights: &[f32; SAMPLES],
    spans: &[GroundSpan],
    landing_x: f32,
) -> Vec<Prop> {
    let half = GROUND_WIDTH / 2.0;
    #[expect(clippy::cast_possible_truncation, reason = "PROPS_MAX is 24")]
    let count = PROPS_MIN + (rng.next_u64() % (PROPS_MAX - PROPS_MIN + 1) as u64) as usize;
    let mut props = Vec::with_capacity(count);
    for _ in 0..count {
        let x = rng.range(-half, half);
        let ground = kind_of(spans, x);
        let total: u32 = PropKind::ALL
            .iter()
            .map(|prop| u32::from(weight(ground, *prop)))
            .sum();
        if total == 0 {
            continue;
        }
        #[expect(clippy::cast_possible_truncation, reason = "total is under 20")]
        let mut pick = (rng.next_u64() % u64::from(total)) as u32;
        let mut kind = PropKind::Boulder;
        for candidate in PropKind::ALL {
            let share = u32::from(weight(ground, candidate));
            if pick < share {
                kind = candidate;
                break;
            }
            pick -= share;
        }
        let size = prop_size(rng, kind);
        let seed = rng.next_u64();
        // The landing spot stays clear: dropping a prop rather than moving it
        // keeps the roll honest and the loop finite.
        if (x - landing_x).abs() < CLEARANCE {
            continue;
        }
        // On water, only the bank is planted.
        if ground == GroundKind::Water {
            // The outer edges of the planet are not shores, so a lake that
            // runs off the side of the world is open water all the way.
            let to_shore = spans
                .iter()
                .skip(1)
                .map(|span| (x - span.from).abs())
                .fold(f32::MAX, f32::min);
            if to_shore > BANK {
                continue;
            }
        }
        let t = (x + half) / GROUND_WIDTH;
        #[expect(clippy::cast_precision_loss, reason = "SAMPLES is 96")]
        let at = t.clamp(0.0, 1.0) * (SAMPLES - 1) as f32;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to 0..SAMPLES"
        )]
        let index = (at.round() as usize).min(SAMPLES - 1);
        props.push(Prop {
            kind,
            at: Vec2::new(x, SURFACE_Y + heights[index]),
            size,
            seed,
        });
    }
    props
}

/// A planet's colours: one hue carries the sky and the ground, and its
/// opposite carries everything growing.
fn palette_of(rng: &mut SplitMix64) -> Palette {
    let hue = rng.range(0.0, 360.0);
    let saturation = rng.range(0.25, 0.7);
    // The sky climbs in lightness from the top down, so the horizon glows.
    let top = rng.range(0.18, 0.34);
    let horizon = top + rng.range(0.18, 0.34);
    let sky = [
        Shade {
            hue,
            saturation: saturation * 0.9,
            lightness: top,
        },
        Shade {
            hue: wrap_hue(hue + 8.0),
            saturation,
            lightness: top + (horizon - top) * 0.45,
        },
        Shade {
            hue: wrap_hue(hue + 16.0),
            saturation,
            lightness: top + (horizon - top) * 0.78,
        },
        Shade {
            hue: wrap_hue(hue + 24.0),
            saturation: saturation * 0.8,
            lightness: horizon,
        },
    ];
    // The ground is the same hue, and always well darker than the sky it sits
    // against: the silhouette of a horizon is the whole picture.
    // Written as min then max, never `clamp`: `clamp` panics if its bounds
    // cross, and a panic in game logic is not allowed (Section 7.4).
    let ground_light = (horizon - rng.range(0.22, 0.34))
        .min(horizon - 0.2)
        .max(0.03);
    let opposite = wrap_hue(hue + 180.0 + rng.range(-30.0, 30.0));
    Palette {
        sky,
        ground: Shade {
            hue,
            saturation: saturation * 0.85,
            lightness: ground_light,
        },
        ground_far: Shade {
            hue,
            saturation: saturation * 0.7,
            lightness: (ground_light * 0.55).max(0.03),
        },
        water: Shade {
            hue: wrap_hue(hue - 20.0),
            saturation: (saturation * 1.1).min(0.7),
            lightness: (ground_light + 0.1).min(0.6),
        },
        prop: Shade {
            hue: opposite,
            saturation: (saturation * 1.05).min(0.7),
            lightness: (ground_light + 0.14).min(0.62),
        },
        accent: Shade {
            hue: wrap_hue(opposite + 18.0),
            saturation: 0.7,
            lightness: 0.72,
        },
    }
}

/// Keeps a hue on the wheel, 0 up to (but not including) 360.
fn wrap_hue(hue: f32) -> f32 {
    hue.rem_euclid(360.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::LEVEL_HEIGHT;

    #[test]
    fn one_seed_is_always_the_same_planet() {
        for seed in [0, 1, 7, u64::MAX] {
            let a = Planet::generate(seed);
            let b = Planet::generate(seed);
            assert_eq!(
                a.ground.map(f32::to_bits),
                b.ground.map(f32::to_bits),
                "seed {seed}"
            );
            assert!((a.landing_x - b.landing_x).abs() < f32::EPSILON);
            assert_eq!(a.props.len(), b.props.len());
        }
    }

    #[test]
    fn the_ground_stays_inside_the_playfield() {
        for seed in 0..2_000 {
            let planet = Planet::generate(seed);
            for (index, height) in planet.ground.iter().enumerate() {
                assert!(
                    height.abs() <= MAX_RELIEF,
                    "seed {seed} sample {index} is {height}"
                );
                let y = SURFACE_Y + height;
                assert!((-LEVEL_HEIGHT..0.0).contains(&y), "seed {seed}: y {y}");
            }
        }
    }

    #[test]
    fn there_is_always_dry_land_to_stand_on() {
        for seed in 0..2_000 {
            let planet = Planet::generate(seed);
            assert_ne!(
                kind_of(&planet.spans, planet.landing_x),
                GroundKind::Water,
                "seed {seed} lands in the water"
            );
            let water: f32 = planet
                .spans
                .iter()
                .filter(|span| span.kind == GroundKind::Water)
                .map(|span| span.to - span.from)
                .sum();
            assert!(
                water <= GROUND_WIDTH * 0.4 + 1.0,
                "seed {seed}: {water} of water"
            );
        }
    }

    #[test]
    fn the_landing_spot_is_flat_and_clear() {
        let step = step();
        for seed in 0..2_000 {
            let planet = Planet::generate(seed);
            let x = planet.landing_x;
            assert!(
                x.abs() <= WIDER_REACH,
                "seed {seed} lands off screen at {x}"
            );
            let slope = (planet.height_at(x + step) - planet.height_at(x - step)) / (2.0 * step);
            assert!(slope.abs() < 0.6, "seed {seed}: slope {slope}");
            for prop in &planet.props {
                assert!(
                    (prop.at.x - x).abs() >= CLEARANCE,
                    "seed {seed}: a {:?} sits on the landing spot",
                    prop.kind
                );
            }
        }
    }

    /// Section 11 asks for a slope under 0.15. On a planet of jagged rock
    /// there may be nothing that gentle within reach, and then the flattest
    /// ground available is the right answer; it is never worse than that.
    #[test]
    fn the_landing_spot_is_as_gentle_as_the_planet_allows() {
        let step = step();
        for seed in 0..2_000 {
            let planet = Planet::generate(seed);
            let slope_at = |x: f32| {
                ((planet.height_at(x + step) - planet.height_at(x - step)) / (2.0 * step)).abs()
            };
            let candidates: Vec<f32> = (1..SAMPLES - 1)
                .map(sample_x)
                .filter(|x| {
                    x.abs() <= WIDER_REACH && kind_of(&planet.spans, *x) != GroundKind::Water
                })
                .collect();
            let flattest = candidates
                .iter()
                .map(|x| slope_at(*x))
                .fold(f32::MAX, f32::min);
            let chosen = slope_at(planet.landing_x);
            if flattest < MAX_SLOPE {
                assert!(
                    chosen < MAX_SLOPE,
                    "seed {seed}: landed on {chosen} with {flattest} going spare"
                );
            } else {
                assert!(
                    (chosen - flattest).abs() < 1e-3,
                    "seed {seed}: landed on {chosen}, flattest within reach was {flattest}"
                );
            }
        }
    }

    #[test]
    fn only_the_bank_of_a_lake_is_planted() {
        for seed in 0..2_000 {
            let planet = Planet::generate(seed);
            // The shorelines, which are the boundaries between spans. The
            // outer edges of the planet are not shores.
            let shores: Vec<f32> = planet.spans.iter().skip(1).map(|span| span.from).collect();
            for prop in &planet.props {
                if kind_of(&planet.spans, prop.at.x) != GroundKind::Water {
                    continue;
                }
                let to_shore = shores
                    .iter()
                    .map(|shore| (prop.at.x - shore).abs())
                    .fold(f32::MAX, f32::min);
                assert!(
                    to_shore <= BANK,
                    "seed {seed}: a {:?} stands {to_shore} out in open water",
                    prop.kind
                );
            }
        }
    }

    #[test]
    fn every_ground_kind_turns_up_and_props_match_it() {
        let mut seen = [false; 4];
        for seed in 0..2_000 {
            let planet = Planet::generate(seed);
            assert!(planet.props.len() <= PROPS_MAX, "seed {seed}");
            for span in &planet.spans {
                seen[span.kind as usize] = true;
            }
            for prop in &planet.props {
                // A mushroom belongs to overgrowth, a boulder never grows in a lake.
                let ground = kind_of(&planet.spans, prop.at.x);
                assert!(
                    weight(ground, prop.kind) > 0,
                    "seed {seed}: a {:?} on {ground:?}",
                    prop.kind
                );
            }
        }
        assert!(seen.iter().all(|&kind| kind), "{seen:?}");
    }

    #[test]
    fn no_planet_is_ugly() {
        for seed in 0..2_000 {
            let palette = Planet::generate(seed).palette;
            assert!(
                palette.ground.lightness + 0.2 <= palette.sky[3].lightness,
                "seed {seed}: ground {} against sky {}",
                palette.ground.lightness,
                palette.sky[3].lightness
            );
            for shade in [palette.ground, palette.water, palette.prop, palette.accent] {
                assert!(shade.saturation <= 0.7, "seed {seed}: {shade:?}");
                assert!((0.0..360.0).contains(&shade.hue), "seed {seed}: {shade:?}");
            }
        }
    }
}
