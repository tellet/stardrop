//! What note to play, and when. No audio here at all: this is the musical
//! part of Section 6, written as plain functions so it can be tested by ear
//! *and* by unit test.
//!
//! Every level sits in one key, and every note comes from that key's major
//! pentatonic scale. A pentatonic has no semitone clashes, so any two notes
//! played together still sound like music — which matters when the player,
//! not a composer, decides what gets played.

use crate::level::Theme;
use crate::objects::ObjectKind;

/// The major pentatonic, in semitones above the root.
pub const PENTATONIC: [i32; 5] = [0, 2, 4, 7, 9];

/// The key a level is played in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Key {
    /// What to call it, for the journal and the credits.
    pub name: &'static str,
    /// The root note, in hertz.
    pub root_hz: f32,
}

/// How an object sounds: what the player learns without being told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mood {
    /// Helpful: lands on a resolved, bright step of the scale.
    Bright,
    /// Tricky or harmful: an unresolved step, still in the key, never harsh.
    Unsettled,
}

/// The keys the game descends through, one per altitude. They walk downward
/// in pitch as the player falls, so the whole game is one long descent.
const KEYS: [Key; 10] = [
    Key {
        name: "A major pentatonic",
        root_hz: 440.00,
    },
    Key {
        name: "G major pentatonic",
        root_hz: 392.00,
    },
    Key {
        name: "F♯ major pentatonic",
        root_hz: 369.99,
    },
    Key {
        name: "E major pentatonic",
        root_hz: 329.63,
    },
    Key {
        name: "D major pentatonic",
        root_hz: 293.66,
    },
    Key {
        name: "C major pentatonic",
        root_hz: 261.63,
    },
    Key {
        name: "B major pentatonic",
        root_hz: 246.94,
    },
    Key {
        name: "A major pentatonic, low",
        root_hz: 220.00,
    },
    Key {
        name: "G major pentatonic, low",
        root_hz: 196.00,
    },
    Key {
        name: "D major pentatonic, low",
        root_hz: 146.83,
    },
];

/// The key an altitude is played in.
#[must_use]
pub fn key_for(theme: Theme) -> Key {
    let index = Theme::ALL
        .iter()
        .position(|listed| *listed == theme)
        .unwrap_or(0);
    KEYS[index.min(KEYS.len() - 1)]
}

/// The key the menus and the journal sit in.
#[must_use]
pub fn menu_key() -> Key {
    KEYS[0]
}

/// The frequency of a scale step, counting up from the root. Steps past the
/// top of the scale carry on into the next octave, so a long run of notes
/// keeps rising instead of jumping back down.
#[must_use]
pub fn step_hz(key: Key, step: i32) -> f32 {
    let degrees = i32::try_from(PENTATONIC.len()).unwrap_or(5);
    let octave = step.div_euclid(degrees);
    let within = step.rem_euclid(degrees);
    let semitones = PENTATONIC[usize::try_from(within).unwrap_or(0)] + octave * 12;
    // `exp2` of a small number: the equal-tempered ratio.
    key.root_hz * f32::exp2(f32::from(i16::try_from(semitones).unwrap_or(0)) / 12.0)
}

/// Whether an object is a friend. Helpful ones resolve; the ones that cost
/// the player something stay unresolved, so the ear learns the difference
/// before the player works it out.
#[must_use]
pub fn mood(kind: ObjectKind) -> Mood {
    match kind {
        ObjectKind::CloudPuff
        | ObjectKind::WindRibbon
        | ObjectKind::Bubble
        | ObjectKind::TwinPortals => Mood::Bright,
        ObjectKind::Crystal
        | ObjectKind::GravityOrb
        | ObjectKind::SporeFlower
        | ObjectKind::CometStone
        | ObjectKind::StaticCloud => Mood::Unsettled,
    }
}

/// Which step of the scale a touch plays: `streak` is how many objects have
/// been touched in a row this attempt, starting at 0.
///
/// Every touch steps one higher, so a good run rises. Helpful objects land on
/// the resolved steps of the scale (root, third, fifth); tricky ones land
/// between them.
#[must_use]
pub fn touch_step(kind: ObjectKind, streak: u32) -> i32 {
    // Runs longer than this keep rising, but from the bottom again: the
    // melody stays in a comfortable range instead of disappearing upwards.
    const RUN: u32 = 6;
    let rise = i32::try_from(streak % RUN).unwrap_or(0);
    let footing = match mood(kind) {
        // Steps 0, 2 and 4 of the pentatonic are its resolved notes.
        Mood::Bright => 0,
        Mood::Unsettled => 1,
    };
    footing + rise
}

/// The chord that finishes the melody when the astronaut lands: the root, its
/// third and its fifth, an octave apart, all from the level's own scale.
#[must_use]
pub fn landing_chord(key: Key) -> [f32; 3] {
    [step_hz(key, 0), step_hz(key, 2), step_hz(key, 5)]
}

/// Each mote of stardust rings one step higher than the last.
#[must_use]
pub fn stardust_step(collected: u32) -> i32 {
    // Starts an octave up: twinkles sit above the melody, out of its way.
    5 + i32::try_from(collected % 8).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Within a hundredth of a semitone.
    fn close(a: f32, b: f32) -> bool {
        (a / b).log2().abs() * 12.0 < 0.01
    }

    #[test]
    fn a_scale_step_is_the_note_it_should_be() {
        let d_major = Key {
            name: "D major pentatonic",
            root_hz: 293.66,
        };
        // D E F# A B, then the octave.
        assert!(close(step_hz(d_major, 0), 293.66), "D4");
        assert!(close(step_hz(d_major, 1), 329.63), "E4");
        assert!(close(step_hz(d_major, 2), 369.99), "F#4");
        assert!(close(step_hz(d_major, 3), 440.00), "A4");
        assert!(close(step_hz(d_major, 4), 493.88), "B4");
        assert!(close(step_hz(d_major, 5), 587.33), "D5, the octave");
        // The fifth consecutive touch of a helpful object, from the root.
        assert!(close(
            step_hz(d_major, touch_step(ObjectKind::CloudPuff, 4)),
            493.88
        ));
    }

    #[test]
    fn steps_below_the_root_drop_an_octave() {
        let key = Key {
            name: "A",
            root_hz: 440.0,
        };
        assert!(close(step_hz(key, -5), 220.0));
        // A major pentatonic is A B C# E F#, so the step below the root is F#.
        assert!(close(step_hz(key, -1), 369.99));
    }

    /// A run of touches must climb, and must stay inside a range a player can
    /// listen to for a whole level.
    #[test]
    fn a_run_of_touches_climbs_and_stays_singable() {
        let key = key_for(Theme::DeepSpace);
        let mut previous = 0.0;
        for streak in 0..6 {
            let hz = step_hz(key, touch_step(ObjectKind::CloudPuff, streak));
            assert!(hz > previous, "touch {streak} didn't rise");
            previous = hz;
        }
        for streak in 0..64 {
            for kind in ObjectKind::ALL {
                let hz = step_hz(key, touch_step(kind, streak));
                assert!(
                    (key.root_hz..=key.root_hz * 4.0).contains(&hz),
                    "{kind:?} at {streak} left the singable range: {hz} Hz"
                );
            }
        }
    }

    #[test]
    fn helpful_objects_resolve_and_tricky_ones_dont() {
        assert_eq!(mood(ObjectKind::CloudPuff), Mood::Bright);
        assert_eq!(mood(ObjectKind::StaticCloud), Mood::Unsettled);
        // The first touch of each: a friend lands on the root, a hazard doesn't.
        let key = key_for(Theme::CloudSea);
        assert!(close(
            step_hz(key, touch_step(ObjectKind::CloudPuff, 0)),
            key.root_hz
        ));
        assert!(step_hz(key, touch_step(ObjectKind::StaticCloud, 0)) > key.root_hz);
    }

    /// The game descends: each altitude's key is lower than the one above it.
    #[test]
    fn the_keys_fall_as_the_game_does() {
        for pair in KEYS.windows(2) {
            assert!(
                pair[1].root_hz < pair[0].root_hz,
                "{} is not below {}",
                pair[1].name,
                pair[0].name
            );
        }
        for theme in Theme::ALL {
            let key = key_for(theme);
            assert!((100.0..=500.0).contains(&key.root_hz), "{theme:?}");
        }
    }

    #[test]
    fn the_landing_chord_is_in_the_key() {
        let key = key_for(Theme::Homecoming);
        let chord = landing_chord(key);
        assert!(close(chord[0], key.root_hz));
        assert!(chord[1] > chord[0] && chord[2] > chord[1]);
        assert!(
            close(chord[2], key.root_hz * 2.0),
            "it finishes on the octave"
        );
    }
}
