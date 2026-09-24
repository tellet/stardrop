//! The look of each altitude: sky colours, background layers and glow.
//!
//! One `ThemeArt` per [`Theme`], so the game moves through a deliberate colour
//! journey from deep space down to a soft dawn at home (Section 5). Everything
//! here is data: `background.rs` builds meshes from it, `camera.rs` reads the
//! bloom strength.

use bevy::prelude::*;

use crate::level::Theme;

/// What a background layer is made of. Shapes differ per mark, so layers read
/// as different kinds of scenery rather than the same dots in new colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mark {
    /// A small dot with a soft halo: stars, distant lights.
    Star,
    /// A big round cloud that fades away at its rim: nebulae, clouds, blossoms.
    Puff,
    /// A tall soft column: aurora curtains, light shafts.
    Curtain,
    /// A thin tilted line: meteors, falling light.
    Streak,
}

/// One drifting background layer.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LayerSpec {
    pub mark: Mark,
    /// How many marks in one repeating tile.
    pub count: usize,
    /// Star and puff: radius. Curtain and streak: width and height.
    pub size: Vec2,
    /// Dim on purpose: the background never blooms and never competes with
    /// what the player has to read.
    pub color: Color,
    /// Drift speed relative to the nearest layer. Nearer layers move faster,
    /// which reads as depth.
    pub parallax: f32,
    /// How far the layer sways sideways, in world units.
    pub sway: f32,
    /// Distance behind the playfield: -800 (farthest) to -600 (nearest).
    pub depth: f32,
}

/// Everything that makes one altitude look like itself.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ThemeArt {
    /// Sky gradient from the top of the view to the bottom.
    pub sky: [Color; 4],
    /// Far to near; 3 to 5 of them.
    pub layers: &'static [LayerSpec],
    /// Bloom strength for this altitude: hazier skies glow more.
    pub bloom: f32,
    /// Dims everything beside the playfield.
    pub veil: Color,
    /// The faint lines marking the playfield's side edges.
    pub edge: Color,
}

/// The depths layers sit at, far to near.
const FAR: f32 = -800.0;
const MID: f32 = -740.0;
const NEAR: f32 = -680.0;
const CLOSE: f32 = -620.0;

/// What the menus and the journal sit in front of.
pub(crate) const MENU_THEME: Theme = Theme::DeepSpace;

/// The art for one altitude.
pub(crate) fn art(theme: Theme) -> &'static ThemeArt {
    match theme {
        Theme::DeepSpace => &DEEP_SPACE,
        Theme::SpaceClouds => &SPACE_CLOUDS,
        Theme::UpperAtmosphere => &UPPER_ATMOSPHERE,
        Theme::CrystalAurora => &CRYSTAL_AURORA,
        Theme::HighClouds => &HIGH_CLOUDS,
        Theme::CloudSea => &CLOUD_SEA,
        Theme::Sunset => &SUNSET,
        Theme::WarmEvening => &WARM_EVENING,
        Theme::Twilight => &TWILIGHT,
        Theme::Homecoming => &HOMECOMING,
    }
}

// --- 1. Deep space: indigo and violet, a quiet starfield -------------------

static DEEP_SPACE: ThemeArt = ThemeArt {
    sky: [
        Color::srgb(0.02, 0.02, 0.08),
        Color::srgb(0.05, 0.03, 0.14),
        Color::srgb(0.10, 0.05, 0.22),
        Color::srgb(0.16, 0.08, 0.30),
    ],
    layers: &[
        LayerSpec {
            mark: Mark::Star,
            count: 150,
            size: Vec2::splat(2.0),
            color: Color::srgba(0.80, 0.82, 1.00, 0.55),
            parallax: 0.10,
            sway: 0.0,
            depth: FAR,
        },
        LayerSpec {
            mark: Mark::Puff,
            count: 7,
            size: Vec2::splat(520.0),
            color: Color::srgba(0.35, 0.16, 0.60, 0.22),
            parallax: 0.22,
            sway: 22.0,
            depth: MID,
        },
        LayerSpec {
            mark: Mark::Star,
            count: 55,
            size: Vec2::splat(3.4),
            color: Color::srgba(0.90, 0.85, 1.00, 0.45),
            parallax: 0.45,
            sway: 0.0,
            depth: NEAR,
        },
        LayerSpec {
            mark: Mark::Puff,
            count: 26,
            size: Vec2::splat(9.0),
            color: Color::srgba(0.75, 0.65, 1.00, 0.16),
            parallax: 0.85,
            sway: 40.0,
            depth: CLOSE,
        },
    ],
    bloom: 0.30,
    veil: Color::srgba(0.01, 0.005, 0.03, 0.50),
    edge: Color::srgba(0.75, 0.70, 1.00, 0.25),
};

// --- 2. Space clouds: glowing violet and magenta ---------------------------

static SPACE_CLOUDS: ThemeArt = ThemeArt {
    sky: [
        Color::srgb(0.05, 0.02, 0.12),
        Color::srgb(0.13, 0.05, 0.24),
        Color::srgb(0.24, 0.08, 0.34),
        Color::srgb(0.32, 0.12, 0.38),
    ],
    layers: &[
        LayerSpec {
            mark: Mark::Star,
            count: 110,
            size: Vec2::splat(2.0),
            color: Color::srgba(0.90, 0.85, 1.00, 0.40),
            parallax: 0.10,
            sway: 0.0,
            depth: FAR,
        },
        LayerSpec {
            mark: Mark::Puff,
            count: 12,
            size: Vec2::splat(430.0),
            color: Color::srgba(0.55, 0.18, 0.62, 0.26),
            parallax: 0.26,
            sway: 30.0,
            depth: MID,
        },
        LayerSpec {
            mark: Mark::Puff,
            count: 9,
            size: Vec2::splat(260.0),
            color: Color::srgba(0.85, 0.35, 0.70, 0.18),
            parallax: 0.5,
            sway: 45.0,
            depth: NEAR,
        },
        LayerSpec {
            mark: Mark::Star,
            count: 30,
            size: Vec2::splat(4.0),
            color: Color::srgba(1.00, 0.90, 0.95, 0.40),
            parallax: 0.9,
            sway: 0.0,
            depth: CLOSE,
        },
    ],
    bloom: 0.34,
    veil: Color::srgba(0.02, 0.01, 0.04, 0.50),
    edge: Color::srgba(0.85, 0.65, 1.00, 0.25),
};

// --- 3. Upper atmosphere: green and teal auroras ---------------------------

static UPPER_ATMOSPHERE: ThemeArt = ThemeArt {
    sky: [
        Color::srgb(0.02, 0.05, 0.11),
        Color::srgb(0.03, 0.10, 0.18),
        Color::srgb(0.05, 0.17, 0.24),
        Color::srgb(0.07, 0.23, 0.28),
    ],
    layers: &[
        LayerSpec {
            mark: Mark::Star,
            count: 130,
            size: Vec2::splat(2.0),
            color: Color::srgba(0.85, 1.00, 0.95, 0.45),
            parallax: 0.10,
            sway: 0.0,
            depth: FAR,
        },
        LayerSpec {
            mark: Mark::Curtain,
            count: 7,
            size: Vec2::new(300.0, 1500.0),
            color: Color::srgba(0.20, 0.80, 0.55, 0.20),
            parallax: 0.28,
            sway: 60.0,
            depth: MID,
        },
        LayerSpec {
            mark: Mark::Curtain,
            count: 4,
            size: Vec2::new(180.0, 1100.0),
            color: Color::srgba(0.35, 0.95, 0.80, 0.16),
            parallax: 0.55,
            sway: 90.0,
            depth: NEAR,
        },
        LayerSpec {
            mark: Mark::Puff,
            count: 22,
            size: Vec2::splat(8.0),
            color: Color::srgba(0.70, 1.00, 0.90, 0.18),
            parallax: 0.9,
            sway: 35.0,
            depth: CLOSE,
        },
    ],
    bloom: 0.32,
    veil: Color::srgba(0.005, 0.02, 0.03, 0.50),
    edge: Color::srgba(0.60, 1.00, 0.90, 0.25),
};

// --- 4. Crystal aurora: cyan curtains and glints ---------------------------

static CRYSTAL_AURORA: ThemeArt = ThemeArt {
    sky: [
        Color::srgb(0.02, 0.06, 0.14),
        Color::srgb(0.04, 0.12, 0.24),
        Color::srgb(0.06, 0.20, 0.32),
        Color::srgb(0.10, 0.26, 0.36),
    ],
    layers: &[
        LayerSpec {
            mark: Mark::Star,
            count: 120,
            size: Vec2::splat(2.2),
            color: Color::srgba(0.85, 0.95, 1.00, 0.45),
            parallax: 0.10,
            sway: 0.0,
            depth: FAR,
        },
        LayerSpec {
            mark: Mark::Curtain,
            count: 5,
            size: Vec2::new(340.0, 1600.0),
            color: Color::srgba(0.18, 0.62, 0.85, 0.20),
            parallax: 0.3,
            sway: 70.0,
            depth: MID,
        },
        LayerSpec {
            mark: Mark::Streak,
            count: 26,
            size: Vec2::new(6.0, 90.0),
            color: Color::srgba(0.70, 0.95, 1.00, 0.30),
            parallax: 0.6,
            sway: 25.0,
            depth: NEAR,
        },
        LayerSpec {
            mark: Mark::Star,
            count: 34,
            size: Vec2::splat(4.5),
            color: Color::srgba(0.90, 1.00, 1.00, 0.40),
            parallax: 0.95,
            sway: 0.0,
            depth: CLOSE,
        },
    ],
    bloom: 0.33,
    veil: Color::srgba(0.005, 0.02, 0.04, 0.50),
    edge: Color::srgba(0.60, 0.95, 1.00, 0.25),
};

// --- 5. High clouds: deep blue, distant lightning glow ---------------------

static HIGH_CLOUDS: ThemeArt = ThemeArt {
    sky: [
        Color::srgb(0.04, 0.06, 0.16),
        Color::srgb(0.07, 0.11, 0.26),
        Color::srgb(0.12, 0.17, 0.34),
        Color::srgb(0.18, 0.22, 0.40),
    ],
    layers: &[
        LayerSpec {
            mark: Mark::Star,
            count: 90,
            size: Vec2::splat(2.0),
            color: Color::srgba(0.85, 0.90, 1.00, 0.35),
            parallax: 0.10,
            sway: 0.0,
            depth: FAR,
        },
        LayerSpec {
            mark: Mark::Puff,
            count: 14,
            size: Vec2::splat(460.0),
            color: Color::srgba(0.30, 0.36, 0.60, 0.30),
            parallax: 0.24,
            sway: 34.0,
            depth: MID,
        },
        LayerSpec {
            mark: Mark::Puff,
            count: 10,
            size: Vec2::splat(300.0),
            color: Color::srgba(0.55, 0.62, 0.95, 0.20),
            parallax: 0.52,
            sway: 55.0,
            depth: NEAR,
        },
        LayerSpec {
            mark: Mark::Streak,
            count: 18,
            size: Vec2::new(4.0, 70.0),
            color: Color::srgba(0.80, 0.85, 1.00, 0.22),
            parallax: 0.95,
            sway: 20.0,
            depth: CLOSE,
        },
    ],
    bloom: 0.30,
    veil: Color::srgba(0.01, 0.015, 0.04, 0.50),
    edge: Color::srgba(0.70, 0.80, 1.00, 0.25),
};

// --- 6. Cloud sea: pastel dawn above the clouds ----------------------------

static CLOUD_SEA: ThemeArt = ThemeArt {
    sky: [
        Color::srgb(0.16, 0.14, 0.30),
        Color::srgb(0.32, 0.22, 0.40),
        Color::srgb(0.55, 0.36, 0.46),
        Color::srgb(0.72, 0.52, 0.50),
    ],
    layers: &[
        LayerSpec {
            mark: Mark::Star,
            count: 60,
            size: Vec2::splat(2.0),
            color: Color::srgba(1.00, 0.95, 0.95, 0.30),
            parallax: 0.10,
            sway: 0.0,
            depth: FAR,
        },
        LayerSpec {
            mark: Mark::Puff,
            count: 16,
            size: Vec2::splat(500.0),
            color: Color::srgba(0.85, 0.60, 0.65, 0.24),
            parallax: 0.22,
            sway: 28.0,
            depth: MID,
        },
        LayerSpec {
            mark: Mark::Puff,
            count: 12,
            size: Vec2::splat(300.0),
            color: Color::srgba(1.00, 0.82, 0.78, 0.22),
            parallax: 0.5,
            sway: 48.0,
            depth: NEAR,
        },
        LayerSpec {
            mark: Mark::Puff,
            count: 20,
            size: Vec2::splat(10.0),
            color: Color::srgba(1.00, 0.92, 0.85, 0.20),
            parallax: 0.9,
            sway: 40.0,
            depth: CLOSE,
        },
    ],
    bloom: 0.26,
    veil: Color::srgba(0.05, 0.03, 0.06, 0.50),
    edge: Color::srgba(1.00, 0.85, 0.80, 0.25),
};

// --- 7. Sunset: gold and coral, falling meteors ----------------------------

static SUNSET: ThemeArt = ThemeArt {
    sky: [
        Color::srgb(0.22, 0.12, 0.26),
        Color::srgb(0.45, 0.20, 0.28),
        Color::srgb(0.70, 0.34, 0.24),
        Color::srgb(0.85, 0.52, 0.28),
    ],
    layers: &[
        LayerSpec {
            mark: Mark::Puff,
            count: 12,
            size: Vec2::splat(560.0),
            color: Color::srgba(0.80, 0.35, 0.30, 0.22),
            parallax: 0.16,
            sway: 24.0,
            depth: FAR,
        },
        LayerSpec {
            mark: Mark::Puff,
            count: 14,
            size: Vec2::splat(280.0),
            color: Color::srgba(1.00, 0.62, 0.35, 0.20),
            parallax: 0.38,
            sway: 42.0,
            depth: MID,
        },
        LayerSpec {
            mark: Mark::Streak,
            count: 22,
            size: Vec2::new(5.0, 120.0),
            color: Color::srgba(1.00, 0.85, 0.60, 0.28),
            parallax: 0.7,
            sway: 30.0,
            depth: NEAR,
        },
        LayerSpec {
            mark: Mark::Star,
            count: 26,
            size: Vec2::splat(3.5),
            color: Color::srgba(1.00, 0.90, 0.75, 0.35),
            parallax: 0.95,
            sway: 0.0,
            depth: CLOSE,
        },
    ],
    bloom: 0.28,
    veil: Color::srgba(0.06, 0.02, 0.04, 0.50),
    edge: Color::srgba(1.00, 0.75, 0.55, 0.25),
};

// --- 8. Warm evening: amber sky, drifting blossoms -------------------------

static WARM_EVENING: ThemeArt = ThemeArt {
    sky: [
        Color::srgb(0.18, 0.10, 0.24),
        Color::srgb(0.36, 0.18, 0.28),
        Color::srgb(0.58, 0.30, 0.30),
        Color::srgb(0.74, 0.46, 0.34),
    ],
    layers: &[
        LayerSpec {
            mark: Mark::Star,
            count: 70,
            size: Vec2::splat(2.0),
            color: Color::srgba(1.00, 0.92, 0.85, 0.30),
            parallax: 0.10,
            sway: 0.0,
            depth: FAR,
        },
        LayerSpec {
            mark: Mark::Puff,
            count: 11,
            size: Vec2::splat(480.0),
            color: Color::srgba(0.72, 0.34, 0.38, 0.22),
            parallax: 0.24,
            sway: 26.0,
            depth: MID,
        },
        LayerSpec {
            mark: Mark::Puff,
            count: 34,
            size: Vec2::splat(14.0),
            color: Color::srgba(1.00, 0.78, 0.80, 0.30),
            parallax: 0.6,
            sway: 70.0,
            depth: NEAR,
        },
        LayerSpec {
            mark: Mark::Puff,
            count: 18,
            size: Vec2::splat(22.0),
            color: Color::srgba(1.00, 0.85, 0.85, 0.22),
            parallax: 1.0,
            sway: 95.0,
            depth: CLOSE,
        },
    ],
    bloom: 0.27,
    veil: Color::srgba(0.05, 0.02, 0.05, 0.50),
    edge: Color::srgba(1.00, 0.80, 0.75, 0.25),
};

// --- 9. Twilight: first stars, lights below --------------------------------

static TWILIGHT: ThemeArt = ThemeArt {
    sky: [
        Color::srgb(0.04, 0.05, 0.18),
        Color::srgb(0.08, 0.10, 0.28),
        Color::srgb(0.14, 0.15, 0.36),
        Color::srgb(0.22, 0.18, 0.38),
    ],
    layers: &[
        LayerSpec {
            mark: Mark::Star,
            count: 150,
            size: Vec2::splat(2.2),
            color: Color::srgba(0.90, 0.92, 1.00, 0.50),
            parallax: 0.10,
            sway: 0.0,
            depth: FAR,
        },
        LayerSpec {
            mark: Mark::Puff,
            count: 9,
            size: Vec2::splat(520.0),
            color: Color::srgba(0.24, 0.24, 0.55, 0.24),
            parallax: 0.22,
            sway: 20.0,
            depth: MID,
        },
        LayerSpec {
            mark: Mark::Star,
            count: 44,
            size: Vec2::splat(4.0),
            color: Color::srgba(1.00, 0.88, 0.70, 0.45),
            parallax: 0.5,
            sway: 0.0,
            depth: NEAR,
        },
        LayerSpec {
            mark: Mark::Puff,
            count: 22,
            size: Vec2::splat(8.0),
            color: Color::srgba(0.85, 0.85, 1.00, 0.18),
            parallax: 0.9,
            sway: 38.0,
            depth: CLOSE,
        },
    ],
    bloom: 0.31,
    veil: Color::srgba(0.01, 0.01, 0.04, 0.50),
    edge: Color::srgba(0.80, 0.80, 1.00, 0.25),
};

// --- 10. Homecoming: night giving way to dawn ------------------------------

static HOMECOMING: ThemeArt = ThemeArt {
    sky: [
        Color::srgb(0.05, 0.07, 0.20),
        Color::srgb(0.14, 0.16, 0.32),
        Color::srgb(0.38, 0.30, 0.40),
        Color::srgb(0.68, 0.52, 0.46),
    ],
    layers: &[
        LayerSpec {
            mark: Mark::Star,
            count: 100,
            size: Vec2::splat(2.0),
            color: Color::srgba(0.90, 0.92, 1.00, 0.40),
            parallax: 0.10,
            sway: 0.0,
            depth: FAR,
        },
        LayerSpec {
            mark: Mark::Curtain,
            count: 4,
            size: Vec2::new(420.0, 1300.0),
            color: Color::srgba(0.55, 0.40, 0.70, 0.16),
            parallax: 0.26,
            sway: 40.0,
            depth: MID,
        },
        LayerSpec {
            mark: Mark::Puff,
            count: 13,
            size: Vec2::splat(360.0),
            color: Color::srgba(0.90, 0.62, 0.52, 0.20),
            parallax: 0.5,
            sway: 36.0,
            depth: NEAR,
        },
        LayerSpec {
            mark: Mark::Puff,
            count: 24,
            size: Vec2::splat(9.0),
            color: Color::srgba(1.00, 0.90, 0.80, 0.22),
            parallax: 0.95,
            sway: 45.0,
            depth: CLOSE,
        },
    ],
    bloom: 0.29,
    veil: Color::srgba(0.02, 0.02, 0.05, 0.50),
    edge: Color::srgba(0.95, 0.85, 0.85, 0.25),
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Every theme a level file can ask for must have art, and that art must
    /// follow the rules the look depends on.
    #[test]
    fn every_theme_is_dressed() {
        for theme in Theme::ALL {
            let art = art(theme);
            assert!(
                (3..=5).contains(&art.layers.len()),
                "{theme:?}: {} layers, Section 5 asks for 3 to 5",
                art.layers.len()
            );
            assert!(
                (0.2..=0.4).contains(&art.bloom),
                "{theme:?}: bloom {} is outside the tuned range",
                art.bloom
            );
            for layer in art.layers {
                let brightest = layer
                    .color
                    .to_linear()
                    .to_f32_array()
                    .into_iter()
                    .take(3)
                    .fold(0.0_f32, f32::max);
                assert!(
                    brightest <= 1.0,
                    "{theme:?}: a background layer is bright enough to bloom"
                );
                assert!(layer.count > 0 && layer.size.min_element() > 0.0);
                assert!((0.0..=1.0).contains(&layer.parallax));
            }
        }
    }

    /// Layers must be listed far to near, or nearer ones would drift behind
    /// farther ones and the depth would read backwards.
    #[test]
    fn layers_run_from_far_to_near() {
        for theme in Theme::ALL {
            let art = art(theme);
            for pair in art.layers.windows(2) {
                assert!(
                    pair[0].depth < pair[1].depth && pair[0].parallax < pair[1].parallax,
                    "{theme:?}: layers are out of order"
                );
            }
        }
    }
}
