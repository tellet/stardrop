//! The game's ears: which sound plays when, and how loud.
//!
//! The notes come from `music.rs` and the sounds themselves from `synth.rs`;
//! this module only decides what to play and keeps the mix in order. Five
//! channels match the five volume settings: music, instruments, effects,
//! ambience and menus.
//!
//! The music bed is three loops that all start together and never stop: the
//! pad, a deeper layer that fades in as the astronaut falls, and a muffled
//! copy of the pad that takes over while the game is paused. Because they run
//! in step, moving between them is a volume change, and never a jump.

use core::time::Duration;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};
// Everything from the mixer, including the bits of kira it re-exports.
use bevy_kira_audio::prelude::*;

use crate::app_state::AppState;
use crate::astronaut::Astronaut;
use crate::attempt::{AttemptFailed, AttemptRestarted};
use crate::background::CurrentTheme;
use crate::collectibles::StardustCollected;
use crate::ending::{Beat, EndingBeat};
use crate::level::{ActiveLevel, LevelCompleted, LevelPhase, Theme};
use crate::music::{Key, key_for, landing_chord, menu_key, stardust_step, step_hz, touch_step};
use crate::objects::{ObjectKind, ObjectTouched};
use crate::physics::Motion;
use crate::settings::Settings;
// `Sound` is the game's word for one of our sounds; kira has a trait of the
// same name, so ours is spelled out where both are in scope.
use crate::synth::{self, Bed, SAMPLE_RATE};

/// How long the music takes to move between two states. Section 6.2 asks for
/// one to two seconds, and never a cut.
const CROSSFADE: Duration = Duration::from_millis(1400);
/// Pausing is quicker: the player is waiting for it.
const PAUSE_FADE: Duration = Duration::from_millis(350);
/// How far the music dips under an object's note, and how quickly it comes
/// back. The dip is what keeps a note audible without turning anything up.
const DUCK_DEPTH: f32 = 0.35;
const DUCK_RECOVERY: f32 = 1.8;
/// How much the music, ambience and effects are lowered before the player's
/// own volume settings apply, so that everything together has headroom.
const MUSIC_TRIM: f32 = 0.55;
const AMBIENCE_TRIM: f32 = 0.35;
const INSTRUMENT_TRIM: f32 = 0.7;
/// Repeated sounds vary by this much in pitch, so nothing sounds mechanical.
const PITCH_SPREAD: f64 = 0.05;
/// Where in the fall the deeper layer has fully joined, and how fast it
/// catches up. A fall is only a few seconds: slower than this and it never
/// arrives at all.
const DEEPER_FULL_DEPTH: f32 = 0.55;
const DEEPER_RATE: f32 = 2.5;
/// How loud the pad is in a menu, against a level.
const MENU_PAD: f32 = 0.6;

/// The music bed.
#[derive(Resource)]
struct MusicChannel;
/// Notes played by the objects.
#[derive(Resource)]
struct InstrumentChannel;
/// Everything else that happens in a level.
#[derive(Resource)]
struct SfxChannel;
/// The air of the place.
#[derive(Resource)]
struct AmbienceChannel;
/// Menus and the journal.
#[derive(Resource)]
struct UiChannel;

/// The one-shot sounds, built once at startup.
#[derive(Resource, Default)]
struct Voices(HashMap<synth::Sound, Handle<AudioSource>>);

/// Whether the music runs all of its layers. A browser mixes every loop on
/// the one thread it has, even a loop sitting at silence, and four at once
/// crackle on a phone: a 46 ms buffer has to be filled between frames, and
/// the audio cannot move to a thread of its own on a page served without the
/// cross-origin isolation headers GitHub Pages will not send.
///
/// So a browser gets the pad and the air alone. The pad carries the menus as
/// well as the fall, the deeper layer never thickens the music as the fall
/// goes on, and pausing no longer takes the top off it.
const EVERY_LAYER: bool = !cfg!(target_arch = "wasm32");

/// The three loops that make up the music (one of them in a browser), the
/// ambience loop, and the key they're all in.
#[derive(Resource, Default)]
struct MusicBed {
    key: Option<Key>,
    /// Kept so the sounds stay loaded while they play.
    sources: Vec<Handle<AudioSource>>,
    pad: Option<Handle<AudioInstance>>,
    muffled: Option<Handle<AudioInstance>>,
    deeper: Option<Handle<AudioInstance>>,
    air: Option<Handle<AudioInstance>>,
    /// How much of the deeper layer is wanted, 0 to 1.
    depth: f32,
    /// Whether a level is loaded, which is what the menus aren't.
    in_level: bool,
    /// How much the music is currently ducked, 0 to 1.
    duck: f32,
}

/// A bed being built on a background thread.
#[derive(Resource)]
struct BuildingBed {
    theme: Theme,
    task: Task<BuiltBed>,
}

/// The samples a background build produced. The two optional layers are only
/// built where they are mixed (see [`EVERY_LAYER`]).
struct BuiltBed {
    key: Key,
    pad: Vec<f32>,
    muffled: Option<Vec<f32>>,
    deeper: Option<Vec<f32>>,
    air: Vec<f32>,
}

/// Something the interface did that deserves a sound. Menus and the journal
/// write these rather than reaching for the mixer themselves.
#[derive(Message, Debug, Clone, Copy)]
pub(crate) enum Interface {
    /// The focus moved to the item at this position.
    Moved(usize),
    /// An item was chosen.
    Chose,
    /// A journal page turned.
    TurnedPage,
}

/// How many objects have been touched in a row this attempt: the melody's
/// place in the scale.
#[derive(Resource, Default)]
struct Streak {
    touches: u32,
    motes: u32,
}

/// Everything the game sounds like.
pub(crate) struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AudioPlugin)
            .add_audio_channel::<MusicChannel>()
            .add_audio_channel::<InstrumentChannel>()
            .add_audio_channel::<SfxChannel>()
            .add_audio_channel::<AmbienceChannel>()
            .add_audio_channel::<UiChannel>()
            .init_resource::<Voices>()
            .init_resource::<MusicBed>()
            .init_resource::<Streak>()
            .add_message::<Interface>()
            .add_systems(Startup, build_voices)
            .add_systems(
                Update,
                (
                    start_building_the_bed.run_if(resource_changed::<CurrentTheme>),
                    finish_building_the_bed.run_if(resource_exists::<BuildingBed>),
                    follow_the_fall,
                    mix.after(follow_the_fall),
                    muffle_while_paused.run_if(state_changed::<AppState>),
                    reset_the_melody.run_if(on_message::<AttemptRestarted>),
                    play_object_notes.run_if(on_message::<ObjectTouched>),
                    ring_stardust.run_if(on_message::<StardustCollected>),
                    play_the_landing.run_if(on_message::<LevelCompleted>),
                    play_the_ending.run_if(on_message::<EndingBeat>),
                    sigh_on_a_failed_attempt.run_if(on_message::<AttemptFailed>),
                    play_interface_sounds.run_if(on_message::<Interface>),
                    pop_when_a_bubble_lets_go,
                ),
            );
    }
}

/// Turns a volume setting (0-100) into the decibels kira wants. A setting of
/// zero is silence, not "very quiet".
fn decibels(percent: u8, trim: f32) -> Decibels {
    let amplitude = f32::from(percent) / 100.0 * trim;
    if amplitude <= 0.0001 {
        return Decibels::SILENCE;
    }
    Decibels(20.0 * amplitude.log10())
}

/// How fast to play a sound to get `note` out of something built at
/// `reference`, folded into the octave around it.
///
/// Playing a sample faster raises its pitch *and* shortens it, so a note
/// several octaves up arrives as a short, sharp chirp. Folding by octaves
/// keeps the note in the key — an octave is the one interval the ear hears
/// as the same note — while every sound keeps the shape it was given.
#[must_use]
pub(crate) fn playable_rate(note: f32, reference: f32) -> f64 {
    if note <= 0.0 || reference <= 0.0 {
        return 1.0;
    }
    let mut ratio = note / reference;
    while ratio > 1.5 {
        ratio *= 0.5;
    }
    while ratio < 0.75 {
        ratio *= 2.0;
    }
    f64::from(ratio)
}

/// Wraps generated samples as something the mixer can play.
fn as_source(samples: &[f32]) -> AudioSource {
    // Mono in, the same on both sides out: nothing in this game is to one
    // side of the player.
    let frames: Vec<Frame> = samples
        .iter()
        .map(|sample| Frame::new(*sample, *sample))
        .collect();
    AudioSource {
        sound: StaticSoundData {
            sample_rate: SAMPLE_RATE,
            frames: frames.into(),
            settings: StaticSoundSettings::default(),
            slice: None,
        },
    }
}

fn build_voices(mut voices: ResMut<Voices>, mut sources: ResMut<Assets<AudioSource>>) {
    for sound in synth::Sound::ALL {
        voices
            .0
            .insert(sound, sources.add(as_source(&synth::render(sound))));
    }
}

/// The instrument each object plays (Section 6.1).
fn instrument(kind: ObjectKind) -> synth::Sound {
    match kind {
        ObjectKind::CloudPuff => synth::Sound::Marimba,
        ObjectKind::WindRibbon => synth::Sound::Flute,
        ObjectKind::Bubble => synth::Sound::Chime,
        ObjectKind::Crystal => synth::Sound::Bell,
        ObjectKind::GravityOrb => synth::Sound::Hum,
        ObjectKind::CometStone => synth::Sound::Whoosh,
        ObjectKind::SporeFlower => synth::Sound::MusicBox,
        ObjectKind::TwinPortals => synth::Sound::Shimmer,
        ObjectKind::StaticCloud => synth::Sound::Muffled,
    }
}

// --- The bed ---------------------------------------------------------------

/// Starts building the music for a new altitude. It runs off the main thread:
/// the loops take tens of milliseconds to work out, which would be a visible
/// stutter at the start of a level.
fn start_building_the_bed(
    mut commands: Commands,
    theme: Res<CurrentTheme>,
    building: Option<Res<BuildingBed>>,
) {
    if building.is_some_and(|building| building.theme == **theme) {
        return;
    }
    let theme = **theme;
    let key = key_for(theme);
    // How low this altitude is, which decides what the air sounds like.
    #[expect(clippy::cast_precision_loss, reason = "ten themes")]
    let air = Theme::ALL
        .iter()
        .position(|listed| *listed == theme)
        .unwrap_or(0) as f32
        / (Theme::ALL.len() - 1) as f32;
    let task = AsyncComputeTaskPool::get().spawn(async move {
        BuiltBed {
            key,
            pad: synth::bed(Bed::Pad, key.root_hz, air, false),
            muffled: EVERY_LAYER.then(|| synth::bed(Bed::Pad, key.root_hz, air, true)),
            deeper: EVERY_LAYER.then(|| synth::bed(Bed::Deeper, key.root_hz, air, false)),
            air: synth::bed(Bed::Ambience, key.root_hz, air, false),
        }
    });
    commands.insert_resource(BuildingBed { theme, task });
}

/// Swaps in a bed once it's ready: the old one fades out while the new one
/// fades in, so the music never cuts.
fn finish_building_the_bed(
    mut commands: Commands,
    mut building: ResMut<BuildingBed>,
    mut bed: ResMut<MusicBed>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut instances: ResMut<Assets<AudioInstance>>,
    music: Res<AudioChannel<MusicChannel>>,
    ambience: Res<AudioChannel<AmbienceChannel>>,
) {
    let Some(built) = block_on(poll_once(&mut building.task)) else {
        return;
    };
    let fade = AudioTween::new(CROSSFADE, AudioEasing::Linear);
    for old in [&bed.pad, &bed.muffled, &bed.deeper, &bed.air] {
        if let Some(mut instance) = old.as_ref().and_then(|handle| instances.get_mut(handle)) {
            instance.stop(fade.clone());
        }
    }

    let pad = sources.add(as_source(&built.pad));
    let air = sources.add(as_source(&built.air));
    // Every layer starts together and runs forever, so moving between them is
    // only ever a change of volume.
    let start = |channel: &AudioChannel<MusicChannel>, source: &Handle<AudioSource>| {
        channel
            .play(source.clone())
            .looped()
            .fade_in(fade.clone())
            .with_volume(Decibels::SILENCE)
            .handle()
    };
    bed.pad = Some(start(&music, &pad));
    bed.air = Some(
        ambience
            .play(air.clone())
            .looped()
            .fade_in(fade.clone())
            .with_volume(Decibels::SILENCE)
            .handle(),
    );
    bed.sources = vec![pad, air];
    // The layers a browser does without: never built, so never mixed.
    let muffled = built
        .muffled
        .as_deref()
        .map(|samples| sources.add(as_source(samples)));
    let deeper = built
        .deeper
        .as_deref()
        .map(|samples| sources.add(as_source(samples)));
    bed.muffled = muffled.as_ref().map(|source| start(&music, source));
    bed.deeper = deeper.as_ref().map(|source| start(&music, source));
    bed.sources.extend(muffled);
    bed.sources.extend(deeper);
    bed.key = Some(built.key);
    commands.remove_resource::<BuildingBed>();
}

/// The deeper layer joins as the astronaut falls, and the music dips for a
/// moment under every note an object plays.
fn follow_the_fall(
    time: Res<Time>,
    mut bed: ResMut<MusicBed>,
    level: Option<Res<ActiveLevel>>,
    astronaut: Option<Single<&Motion, With<Astronaut>>>,
    phase: Option<Res<State<LevelPhase>>>,
) {
    bed.duck = (bed.duck - time.delta_secs() * DUCK_RECOVERY).max(0.0);
    bed.in_level = level.is_some();
    let falling = phase.is_some_and(|phase| *phase.get() == LevelPhase::Falling);
    let wanted = match (falling, level, astronaut) {
        (true, Some(level), Some(motion)) => {
            let fallen = -motion.body().position.y / level.bottom().abs().max(1.0);
            (fallen / DEEPER_FULL_DEPTH).clamp(0.0, 1.0)
        }
        _ => 0.0,
    };
    // Eased, so the layer swells in rather than tracking every wobble.
    bed.depth += (wanted - bed.depth) * (time.delta_secs() * DEEPER_RATE).min(1.0);
}

/// Sets every channel's volume from the settings, the fall and the duck.
/// One place decides how loud everything is.
fn mix(
    settings: Res<Settings>,
    state: Option<Res<State<AppState>>>,
    bed: Res<MusicBed>,
    mut instances: ResMut<Assets<AudioInstance>>,
    music: Res<AudioChannel<MusicChannel>>,
    instrument_channel: Res<AudioChannel<InstrumentChannel>>,
    sfx: Res<AudioChannel<SfxChannel>>,
    ambience: Res<AudioChannel<AmbienceChannel>>,
    ui: Res<AudioChannel<UiChannel>>,
) {
    let master = f32::from(settings.master_volume) / 100.0;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a percentage of a percentage"
    )]
    let scaled = |percent: u8| (f32::from(percent) * master).round() as u8;
    music.set_volume(decibels(scaled(settings.music_volume), MUSIC_TRIM));
    instrument_channel.set_volume(decibels(scaled(settings.sfx_volume), INSTRUMENT_TRIM));
    sfx.set_volume(decibels(scaled(settings.sfx_volume), 1.0));
    ambience.set_volume(decibels(scaled(settings.ambience_volume), AMBIENCE_TRIM));
    ui.set_volume(decibels(scaled(settings.sfx_volume), 1.0));

    let paused = state.is_some_and(|state| *state.get() != AppState::Playing);
    let duck = 1.0 - DUCK_DEPTH * bed.duck;
    let quick = AudioTween::new(Duration::from_millis(120), AudioEasing::Linear);
    // A menu is nowhere: the pad alone, quieter, with no air moving in it.
    // A level is somewhere, and falling through it brings the layer in.
    let (pad, air) = if bed.in_level {
        (duck, if bed.depth > 0.0 { 1.0 } else { 0.55 })
    } else {
        (MENU_PAD * duck, 0.0)
    };
    let levels = [
        // Without the muffled layer the pad plays in the menus itself.
        (&bed.pad, if paused && EVERY_LAYER { 0.0 } else { pad }),
        (&bed.muffled, if paused { duck } else { 0.0 }),
        (&bed.deeper, if paused { 0.0 } else { bed.depth * duck }),
        (&bed.air, if paused { air * 0.5 } else { air }),
    ];
    for (handle, level) in levels {
        let Some(mut instance) = handle.as_ref().and_then(|handle| instances.get_mut(handle))
        else {
            continue;
        };
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a level between 0 and 1"
        )]
        let percent = (level.clamp(0.0, 1.0) * 100.0) as u8;
        instance.set_decibels(decibels(percent, 1.0), quick.clone());
    }
}

/// Pausing takes the top off the music instead of stopping it.
fn muffle_while_paused(
    state: Res<State<AppState>>,
    bed: Res<MusicBed>,
    mut instances: ResMut<Assets<AudioInstance>>,
) {
    if !EVERY_LAYER {
        // Nothing to swap to: the pad plays through the pause as it is.
        return;
    }
    let paused = *state.get() != AppState::Playing;
    let fade = AudioTween::new(PAUSE_FADE, AudioEasing::Linear);
    for (handle, wanted) in [(&bed.pad, !paused), (&bed.muffled, paused)] {
        if let Some(mut instance) = handle.as_ref().and_then(|handle| instances.get_mut(handle)) {
            instance.set_decibels(
                if wanted {
                    Decibels::IDENTITY
                } else {
                    Decibels::SILENCE
                },
                fade.clone(),
            );
        }
    }
}

// --- What the fall plays ---------------------------------------------------

fn reset_the_melody(mut streak: ResMut<Streak>) {
    *streak = Streak::default();
}

/// The level's key, or the menus' when no level is loaded.
fn current_key(bed: &MusicBed) -> Key {
    bed.key.unwrap_or_else(menu_key)
}

/// Every object is an instrument, and every touch is the next note of the
/// melody this attempt is writing.
fn play_object_notes(
    mut touched: MessageReader<ObjectTouched>,
    voices: Res<Voices>,
    mut bed: ResMut<MusicBed>,
    mut streak: ResMut<Streak>,
    channel: Res<AudioChannel<InstrumentChannel>>,
) {
    let key = current_key(&bed);
    for touch in touched.read() {
        let sound = instrument(touch.kind);
        let Some(source) = voices.0.get(&sound) else {
            continue;
        };
        let note = step_hz(key, touch_step(touch.kind, streak.touches));
        streak.touches += 1;
        // The music steps aside for the note.
        bed.duck = 1.0;
        channel
            .play(source.clone())
            .with_playback_rate(playable_rate(note, sound.reference_hz()))
            .linear_fade_in(Duration::from_millis(8));
    }
}

/// Each mote rings one step higher than the last.
fn ring_stardust(
    mut collected: MessageReader<StardustCollected>,
    voices: Res<Voices>,
    bed: Res<MusicBed>,
    mut streak: ResMut<Streak>,
    channel: Res<AudioChannel<SfxChannel>>,
) {
    let key = current_key(&bed);
    for _ in collected.read() {
        let Some(source) = voices.0.get(&synth::Sound::Twinkle) else {
            continue;
        };
        let note = step_hz(key, stardust_step(streak.motes));
        streak.motes += 1;
        channel
            .play(source.clone())
            .with_playback_rate(playable_rate(note, synth::Sound::Twinkle.reference_hz()));
    }
}

/// Landing finishes the melody: the chord the whole level was in.
fn play_the_landing(
    voices: Res<Voices>,
    bed: Res<MusicBed>,
    channel: Res<AudioChannel<SfxChannel>>,
) {
    let Some(source) = voices.0.get(&synth::Sound::Bell) else {
        return;
    };
    for note in landing_chord(current_key(&bed)) {
        channel
            .play(source.clone())
            .with_playback_rate(playable_rate(note, synth::Sound::Bell.reference_hz()));
    }
}

/// What each beat of the landing cartoon sounds like. The descent and the
/// hold are silent on purpose: the first is a long way from anywhere, and the
/// second is someone looking at a flag.
fn voice_for(beat: Beat) -> Option<synth::Sound> {
    match beat {
        // The landing is a chord rather than one voice; `play_the_ending` has it.
        Beat::Descent | Beat::Settle | Beat::Hold => None,
        // Something far too heavy coming off a shoulder.
        Beat::Pack => Some(synth::Sound::Muffled),
        Beat::Flag => Some(synth::Sound::Shimmer),
        Beat::Plant => Some(synth::Sound::Marimba),
    }
}

/// The cartoon's sound: one voice a beat, and the landing chord under the
/// moment they touch down.
fn play_the_ending(
    mut beats: MessageReader<EndingBeat>,
    voices: Res<Voices>,
    bed: Res<MusicBed>,
    channel: Res<AudioChannel<SfxChannel>>,
) {
    for EndingBeat(beat) in beats.read() {
        if let Some(sound) = voice_for(*beat)
            && let Some(source) = voices.0.get(&sound)
        {
            channel.play(source.clone());
        }
        if *beat != Beat::Settle {
            continue;
        }
        // Touchdown: the low sound of arriving, and the chord that closes the
        // fall, the same one a beacon plays.
        if let Some(whoosh) = voices.0.get(&synth::Sound::Whoosh) {
            channel.play(whoosh.clone());
        }
        if let Some(bell) = voices.0.get(&synth::Sound::Bell) {
            for note in landing_chord(current_key(&bed)) {
                channel
                    .play(bell.clone())
                    .with_playback_rate(playable_rate(note, synth::Sound::Bell.reference_hz()));
            }
        }
    }
}

/// A failed attempt sighs; it never buzzes.
fn sigh_on_a_failed_attempt(voices: Res<Voices>, channel: Res<AudioChannel<SfxChannel>>) {
    if let Some(source) = voices.0.get(&synth::Sound::Sigh) {
        channel.play(source.clone());
    }
}

/// The menus and the journal, in the menus' own key: quiet, tuned notes
/// rather than clicks.
fn play_interface_sounds(
    mut interface: MessageReader<Interface>,
    voices: Res<Voices>,
    channel: Res<AudioChannel<UiChannel>>,
) {
    for event in interface.read() {
        let (sound, step) = match event {
            // Moving down the list plays down the scale, so the ear follows
            // the eye.
            Interface::Moved(index) => (
                synth::Sound::UiMove,
                -i32::try_from(*index % 5).unwrap_or(0),
            ),
            Interface::Chose => (synth::Sound::UiPick, 2),
            Interface::TurnedPage => (synth::Sound::PageTurn, 0),
        };
        let Some(source) = voices.0.get(&sound) else {
            continue;
        };
        let rate = if sound == synth::Sound::PageTurn {
            // Not a note: just a breath, a little different each time.
            1.0 + PITCH_SPREAD * f64::from(step)
        } else {
            playable_rate(step_hz(menu_key(), step), sound.reference_hz())
        };
        channel.play(source.clone()).with_playback_rate(rate);
    }
}

/// A bubble lets go quietly, which the player needs to hear: it's the moment
/// they get their steering back.
fn pop_when_a_bubble_lets_go(
    astronaut: Option<Single<&Motion, With<Astronaut>>>,
    voices: Res<Voices>,
    bed: Res<MusicBed>,
    channel: Res<AudioChannel<SfxChannel>>,
    mut was_held: Local<bool>,
) {
    let held = astronaut.is_some_and(|motion| motion.body().effects.held_by_bubble());
    if *was_held
        && !held
        && let Some(source) = voices.0.get(&synth::Sound::Plink)
    {
        let note = step_hz(current_key(&bed), 3);
        channel
            .play(source.clone())
            .with_playback_rate(f64::from(note / synth::Sound::Plink.reference_hz()));
    }
    *was_held = held;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every beat that should make a sound has a voice the mixer has built,
    /// and the quiet ones have none.
    #[test]
    fn the_cartoon_has_a_voice_for_each_of_its_beats() {
        for beat in [
            Beat::Descent,
            Beat::Settle,
            Beat::Pack,
            Beat::Flag,
            Beat::Plant,
            Beat::Hold,
        ] {
            if let Some(sound) = voice_for(beat) {
                assert!(
                    synth::Sound::ALL.contains(&sound),
                    "{beat:?} wants {sound:?}, which the mixer never builds"
                );
            }
        }
        // The landing is a chord, not one voice, so it is played apart.
        assert_eq!(voice_for(Beat::Settle), None, "the landing is its own case");
        assert_eq!(voice_for(Beat::Pack), Some(synth::Sound::Muffled));
        assert_eq!(voice_for(Beat::Descent), None, "coming down is silent");
    }

    /// However high the melody climbs, no sound is ever played more than an
    /// octave from the pitch it was built at: beyond that a sample turns
    /// into a chirp.
    #[test]
    fn notes_stay_within_an_octave_of_the_sound_they_came_from() {
        for (note, reference) in [
            (440.0, 440.0),
            (9000.0, 880.0),
            (55.0, 440.0),
            (1320.0, 440.0),
            (147.0, 110.0),
        ] {
            let rate = playable_rate(note, reference);
            assert!(
                (0.75..=1.5).contains(&rate),
                "{note} Hz from a {reference} Hz sound plays at {rate}x"
            );
        }
        // The note that comes out is still the note asked for, give or take
        // whole octaves.
        let rate = playable_rate(1760.0, 440.0);
        let octaves = (rate.log2() - (1760.0_f64 / 440.0).log2()).abs();
        assert!((octaves - octaves.round()).abs() < 1e-6);
        // Nonsense in, something playable out.
        assert!((playable_rate(0.0, 440.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn a_volume_of_zero_is_silence_and_full_is_unchanged() {
        assert_eq!(decibels(0, 1.0), Decibels::SILENCE);
        assert!((decibels(100, 1.0).0 - Decibels::IDENTITY.0).abs() < 1e-6);
        // Half the amplitude is about six decibels down.
        assert!((decibels(50, 1.0).0 + 6.02).abs() < 0.05);
        // The trim lowers a channel before the player's setting applies.
        assert!(decibels(100, 0.5).0 < decibels(100, 1.0).0);
    }

    /// Every object must have a voice, and no two helpful objects should
    /// sound the same.
    #[test]
    fn every_object_has_its_own_instrument() {
        let mut seen = Vec::new();
        for kind in ObjectKind::ALL {
            let sound = instrument(kind);
            assert!(
                !seen.contains(&sound),
                "{kind:?} sounds like something else"
            );
            seen.push(sound);
        }
    }

    /// A generated sound must survive the trip into the mixer's format.
    #[test]
    fn samples_become_a_playable_sound() {
        let samples = synth::render(synth::Sound::Marimba);
        let source = as_source(&samples);
        assert_eq!(source.sound.sample_rate, SAMPLE_RATE);
        assert_eq!(source.sound.frames.len(), samples.len());
        let loudest = source
            .sound
            .frames
            .iter()
            .fold(0.0_f32, |worst, frame| worst.max(frame.left.abs()));
        assert!(loudest > 0.4 && loudest <= 1.0, "{loudest}");
    }
}
