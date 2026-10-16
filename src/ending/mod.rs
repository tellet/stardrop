//! The cartoon that plays when every level is done.
//!
//! An 8.5-second scene with nothing written on it: the astronaut comes down
//! on a planet nobody has seen before, the dust settles, and they plant a
//! flag. The beats below are the whole script, and every motion in it is a
//! pure function of how far through we are, so the systems only apply what
//! these decide.

// The planet and the timeline are built and tested before anything draws
// them. This goes away with the plugin, which uses all of it.
#![allow(dead_code)]

use bevy::math::Vec2;

mod art;
mod planet;

/// When the astronaut touches the ground.
const TOUCHDOWN: f32 = 2.2;
/// When the dust has settled and the pack comes off.
const SETTLED: f32 = 3.6;
/// When the flag starts rising out of the pack.
const FLAG: f32 = 5.0;
/// When they lean in and push it into the ground.
const PLANT: f32 = 6.4;
/// When they stand back and look at it.
const HOLD: f32 = 7.4;
/// The whole cartoon, in seconds.
const TOTAL: f32 = 8.5;
// The brief asks for five to ten seconds (Section 11), checked when compiling.
const _: () = assert!(TOTAL >= 5.0 && TOTAL <= 10.0, "the cartoon runs 5-10 s");
// The last beat needs time on screen to be a beat at all.
const _: () = assert!(HOLD < TOTAL);

/// Where the astronaut enters, in world y: above the top of the frame.
const ENTRY_Y: f32 = 120.0;
/// How far they drift sideways on the way down, in world units.
const SWAY: f32 = 90.0;
/// How long the landing squash lasts, in seconds.
const SQUASH_SECONDS: f32 = 0.35;
/// How much they give on impact, as a fraction of their height.
const SQUASH_DEPTH: f32 = 0.22;
/// How far the pack overshoots its size before settling into it.
const PACK_OVERSHOOT: f32 = 0.06;
/// How fast the planted flag waves, in radians per second.
const WAVE_SPEED: f32 = 3.4;
/// How far it swings, in radians, and how much of that survives when the
/// player has asked for less motion.
const WAVE_SWING: f32 = 0.22;
const WAVE_CALM: f32 = 0.35;

/// One beat of the script. Each one is announced once, when it begins, which
/// is what `audio.rs` listens for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Beat {
    /// Coming down.
    Descent,
    /// The moment of landing, and the dust dying away after it.
    Settle,
    /// The pack comes off, absurdly large.
    Pack,
    /// A very small flag rises out of it.
    Flag,
    /// It goes into the ground and unfurls.
    Plant,
    /// Standing beside it.
    Hold,
}

// --- The shapes of the motion ----------------------------------------------

/// Which beat is playing `t` seconds in.
#[must_use]
fn beat_at(t: f32) -> Beat {
    if t < TOUCHDOWN {
        Beat::Descent
    } else if t < SETTLED {
        Beat::Settle
    } else if t < FLAG {
        Beat::Pack
    } else if t < PLANT {
        Beat::Flag
    } else if t < HOLD {
        Beat::Plant
    } else {
        Beat::Hold
    }
}

/// How far through a stretch of the script we are, 0 to 1.
#[must_use]
fn through(t: f32, from: f32, to: f32) -> f32 {
    ((t - from) / (to - from)).clamp(0.0, 1.0)
}

/// The astronaut's height: down from above the frame to their feet on the
/// ground at `feet`, slowing into the landing rather than speeding up. A fall
/// this is not; they are bringing themselves down.
#[must_use]
fn descent_y(t: f32, feet: f32) -> f32 {
    let u = through(t, 0.0, TOUCHDOWN);
    let eased = 1.0 - (1.0 - u) * (1.0 - u);
    ENTRY_Y + (feet - ENTRY_Y) * eased
}

/// Their sideways drift: a slow sway on the way down that is spent by the
/// time they arrive, so they land where they are going to stand.
#[must_use]
fn descent_x(t: f32, landing_x: f32) -> f32 {
    let u = through(t, 0.0, TOUCHDOWN);
    landing_x + SWAY * (1.0 - u) * (u * 6.0).sin()
}

/// The landing squash: they give on impact and spread sideways, keeping their
/// area, and it is over well before the dust is.
#[must_use]
fn squash(t: f32) -> Vec2 {
    let u = through(t, TOUCHDOWN, TOUCHDOWN + SQUASH_SECONDS);
    if u <= 0.0 || u >= 1.0 {
        return Vec2::ONE;
    }
    // Deepest at the moment of contact, easing back out.
    let give = SQUASH_DEPTH * (1.0 - u) * (1.0 - u);
    let y = 1.0 - give;
    Vec2::new(1.0 / y, y)
}

/// How big the pack is: nothing until its beat, then eased up to full size
/// with a little overshoot, because something that heavy should land.
#[must_use]
fn pack_scale(t: f32) -> f32 {
    let u = through(t, SETTLED, FLAG);
    if u <= 0.0 {
        return 0.0;
    }
    let eased = 1.0 - (1.0 - u) * (1.0 - u) * (1.0 - u);
    // The overshoot peaks four fifths of the way through and is gone by the end.
    let bump = PACK_OVERSHOOT * (u * core::f32::consts::PI).sin() * (1.0 - u);
    eased + bump
}

/// How far the flag has risen out of the pack, 0 to 1.
#[must_use]
fn flag_rise(t: f32) -> f32 {
    let u = through(t, FLAG, PLANT);
    1.0 - (1.0 - u) * (1.0 - u)
}

/// How far the planted cloth is swung, in radians. `motion` is the player's
/// motion setting: a calmer wave, never a still one.
#[must_use]
fn flag_wave(t: f32, motion: f32) -> f32 {
    if t < PLANT {
        return 0.0;
    }
    let swing = WAVE_SWING * (WAVE_CALM + (1.0 - WAVE_CALM) * motion.clamp(0.0, 1.0));
    (t * WAVE_SPEED).sin() * swing
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_beats_tile_the_whole_cartoon() {
        let mut seen = Vec::new();
        let mut t = 0.0;
        while t <= TOTAL {
            let beat = beat_at(t);
            if seen.last() != Some(&beat) {
                seen.push(beat);
            }
            t += 1.0 / 240.0;
        }
        assert_eq!(
            seen,
            vec![
                Beat::Descent,
                Beat::Settle,
                Beat::Pack,
                Beat::Flag,
                Beat::Plant,
                Beat::Hold
            ],
            "the beats must come once each, in order"
        );
    }

    #[test]
    fn the_astronaut_arrives_as_the_dust_does() {
        let feet = -1400.0;
        assert!(
            descent_y(0.0, feet) >= ENTRY_Y,
            "they start above the frame"
        );
        assert!(
            (descent_y(TOUCHDOWN, feet) - feet).abs() < 0.5,
            "and land on the ground"
        );
        assert!(
            (descent_y(TOTAL, feet) - feet).abs() < 0.5,
            "and stay there"
        );
        // Slowing into the landing, not speeding up: the first half covers more.
        let half = descent_y(0.0, feet) - descent_y(TOUCHDOWN / 2.0, feet);
        let all = descent_y(0.0, feet) - feet;
        assert!(half > all / 2.0, "{half} of {all}");
        // They come down where they are going to stand.
        assert!((descent_x(TOUCHDOWN, 120.0) - 120.0).abs() < 0.5);
    }

    #[test]
    fn the_squash_is_one_bounce_and_then_nothing() {
        assert!(
            (squash(0.0) - Vec2::ONE).length() < 1e-6,
            "nothing before the landing"
        );
        assert!(squash(TOUCHDOWN + 0.05).y < 0.9, "it gives on impact");
        assert!(squash(TOUCHDOWN + 0.05).x > 1.05, "and spreads sideways");
        assert!(
            (squash(SETTLED).y - 1.0).abs() < 0.01,
            "and is over by the settle"
        );
    }

    #[test]
    fn the_pack_and_the_flag_wait_their_turn() {
        assert!(pack_scale(SETTLED - 0.01) < 1e-6, "no pack before its beat");
        assert!(
            pack_scale(FLAG) > 0.99,
            "full size by the time the flag comes"
        );
        assert!(flag_rise(FLAG - 0.01) < 1e-6);
        assert!(flag_rise(PLANT) > 0.99, "up before it is planted");
        assert!(
            flag_wave(PLANT - 0.01, 1.0).abs() < 1e-6,
            "still in the pack"
        );
        // Reduce motion calms the wave instead of stopping it.
        let calm = flag_wave(HOLD + 0.2, 0.0);
        let full = flag_wave(HOLD + 0.2, 1.0);
        assert!(calm.abs() > 1e-3, "it still moves when motion is reduced");
        assert!(calm.abs() < full.abs(), "{calm} against {full}");
    }
}
