//! What note to play, and when. No audio here at all: this is the musical
//! part of Section 6, written as plain functions so it can be tested by ear
//! *and* by unit test.
//!
//! Every level sits in one key, and every note comes from that key's minor
//! pentatonic scale. A pentatonic has no semitone clashes, so any two notes
//! played together still sound like music — which matters when the player,
//! not a composer, decides what gets played.
//!
//! Minor, and built on fourths and fifths rather than thirds: a third is a
//! warm, domestic interval, and this game is set in empty space.

use crate::level::Theme;
use crate::objects::ObjectKind;

/// The minor pentatonic, in semitones above the root: root, flat third,
/// fourth, fifth, flat seventh.
pub const PENTATONIC: [i32; 5] = [0, 3, 5, 7, 10];
/// The open steps of that scale — root, fourth, fifth. Nothing here leans
/// warm or sad; they just sound wide.
const OPEN: [i32; 3] = [0, 2, 3];
/// The two that colour it: the flat third and the flat seventh.
const SHADED: [i32; 2] = [1, 4];
/// Steps in one octave of the scale.
const RUNGS: u32 = 5;
/// How far a run of touches climbs before starting again from the bottom.
const OCTAVES: u32 = 2;

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
        name: "A minor pentatonic",
        root_hz: 440.00,
    },
    Key {
        name: "G minor pentatonic",
        root_hz: 392.00,
    },
    Key {
        name: "F♯ minor pentatonic",
        root_hz: 369.99,
    },
    Key {
        name: "E minor pentatonic",
        root_hz: 329.63,
    },
    Key {
        name: "D minor pentatonic",
        root_hz: 293.66,
    },
    Key {
        name: "C minor pentatonic",
        root_hz: 261.63,
    },
    Key {
        name: "B minor pentatonic",
        root_hz: 246.94,
    },
    Key {
        name: "A minor pentatonic, low",
        root_hz: 220.00,
    },
    Key {
        name: "G minor pentatonic, low",
        root_hz: 196.00,
    },
    Key {
        name: "D minor pentatonic, low",
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
/// Every touch climbs, so a good run rises. Helpful objects climb the open
/// notes, tricky ones the shaded ones, and after two octaves the run starts
/// again from the bottom rather than disappearing upwards.
#[must_use]
pub fn touch_step(kind: ObjectKind, streak: u32) -> i32 {
    let ladder: &[i32] = match mood(kind) {
        Mood::Bright => &OPEN,
        Mood::Unsettled => &SHADED,
    };
    let rungs = u32::try_from(ladder.len()).unwrap_or(1);
    let position = streak % (rungs * OCTAVES);
    let octave = i32::try_from(position / rungs).unwrap_or(0);
    let rung = usize::try_from(position % rungs).unwrap_or(0);
    ladder[rung] + i32::try_from(RUNGS).unwrap_or(5) * octave
}

/// The chord that finishes the melody when the astronaut lands: root, fifth,
/// octave. No third at all — an open fifth is the sound of somewhere big,
/// and the third is what would make it sound like a living room.
#[must_use]
pub fn landing_chord(key: Key) -> [f32; 3] {
    [step_hz(key, 0), step_hz(key, 3), step_hz(key, 5)]
}

/// Each mote of stardust rings one step higher than the last, within a
/// single octave above the root. Climbing further would leave the twinkles
/// shrill and short — a sample played much faster is both.
#[must_use]
pub fn stardust_step(collected: u32) -> i32 {
    i32::try_from(RUNGS).unwrap_or(5) + i32::try_from(collected % RUNGS).unwrap_or(0)
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
        let d_minor = Key {
            name: "D minor pentatonic",
            root_hz: 293.66,
        };
        // D F G A C, then the octave.
        assert!(close(step_hz(d_minor, 0), 293.66), "D4");
        assert!(close(step_hz(d_minor, 1), 349.23), "F4");
        assert!(close(step_hz(d_minor, 2), 392.00), "G4");
        assert!(close(step_hz(d_minor, 3), 440.00), "A4");
        assert!(close(step_hz(d_minor, 4), 523.25), "C5");
        assert!(close(step_hz(d_minor, 5), 587.33), "D5, the octave");
        // The fifth consecutive touch of a helpful object: the second time up
        // the open notes, so the fourth an octave above the root.
        assert!(close(
            step_hz(d_minor, touch_step(ObjectKind::CloudPuff, 4)),
            784.00
        ));
    }

    #[test]
    fn steps_below_the_root_drop_an_octave() {
        let key = Key {
            name: "A",
            root_hz: 440.0,
        };
        assert!(close(step_hz(key, -5), 220.0));
        // A minor pentatonic is A C D E G, so the step below the root is G.
        assert!(close(step_hz(key, -1), 392.00));
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
