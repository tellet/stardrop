//! Every sound in the game, made from scratch.
//!
//! No recordings ship with Stardrop: each sound is built here out of sine
//! waves, filtered noise and envelopes, the same way the art is drawn in code.
//! That keeps the game one file, keeps every note exactly in the level's key,
//! and means a sound can be reshaped by changing a number.
//!
//! Plain Rust, no Bevy: the output is a buffer of samples between -1 and 1.
//! `audio.rs` hands those to the mixer.
//!
//! Two rules from Section 6.2 hold everywhere, and the tests enforce them:
//! every sound fades in over a few milliseconds (nothing clicks or stabs),
//! and nothing reaches full scale (nothing clips).
//!
//! A third rule shapes the whole palette: nothing stops dead. Every note
//! played in a level is followed by a long tail of reflections, because the
//! sound of somewhere enormous is the sound of something taking a long time
//! to fade.

#![expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "turning sample counts into seconds and back is what a synthesiser does; \
              every number here is a few hundred thousand at most"
)]

use core::f32::consts::TAU;

use crate::rng::SplitMix64;

/// Samples per second. Nothing here has anything to say above 11 kHz, and
/// half the usual rate halves the memory every loop costs.
pub const SAMPLE_RATE: u32 = 22_050;
/// The peak every sound is normalised to: headroom so that several at once
/// still can't clip.
const PEAK: f32 = 0.72;
/// The shortest fade-in allowed, in seconds. Anything faster clicks.
const SOFTEST_ATTACK: f32 = 0.004;
/// How long a struck note takes to reach full strength. Short enough to
/// still read as a strike, long enough that it never stabs.
const STRIKE_ATTACK: f32 = 0.018;
/// Everything in a level is rolled off above this, on top of each voice's
/// own filtering: two gentle slopes sound softer than one steep one.
const HOUSE_CUTOFF: f32 = 2600.0;
/// Menu sounds are quieter and duller still: they play over everything.
const MENU_CUTOFF: f32 = 1800.0;
/// Every sound fades out over at least this long, for the same reason.
const TAIL: f32 = 0.01;
/// Noise seed: a sound is the same every time it plays.
const NOISE_SEED: u64 = 0x534F_554E_4400_0001;
/// The reflection delays of the space every level note is heard in, in
/// milliseconds. Deliberately not related to each other by whole numbers, so
/// their echoes never line up into a rhythm.
const REFLECTIONS: [f32; 4] = [29.7, 37.1, 41.1, 43.7];
/// Two short diffusers after them, which smear the reflections into a wash.
const DIFFUSERS: [f32; 2] = [5.0, 1.7];
const DIFFUSION: f32 = 0.7;
/// How long a tail takes to fade away, and how much of it is mixed in.
const TAIL_SECONDS: f32 = 2.5;
const TAIL_MIX: f32 = 0.55;

/// One of the game's sounds. Each is rendered once at [`reference_hz`] and
/// pitched from there when it plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sound {
    /// Cloud puff: a soft marimba.
    Marimba,
    /// Wind ribbon: a breathy flute swell, held while inside.
    Flute,
    /// Bubble closing around you: a glass chime.
    Chime,
    /// Bubble letting go: a soft plink.
    Plink,
    /// Crystal: a bright but soft bell.
    Bell,
    /// Gravity orb: a warm low hum that rises while it holds you.
    Hum,
    /// Comet stone: a deep whoosh with a bass note under it.
    Whoosh,
    /// Spore flower: an airy, slightly detuned music box.
    MusicBox,
    /// Twin portals: a shimmer swelling into the other side.
    Shimmer,
    /// Static cloud: a muffled low tone. Sympathetic, never a buzzer.
    Muffled,
    /// A mote of stardust.
    Twinkle,
    /// A failed attempt: a gentle swell backwards, never a buzz.
    Sigh,
    /// Moving between menu items.
    UiMove,
    /// Choosing a menu item.
    UiPick,
    /// A journal page turning.
    PageTurn,
}

impl Sound {
    /// Every sound, so the mixer can build them all and the tests can check
    /// them all.
    pub const ALL: [Self; 15] = [
        Self::Marimba,
        Self::Flute,
        Self::Chime,
        Self::Plink,
        Self::Bell,
        Self::Hum,
        Self::Whoosh,
        Self::MusicBox,
        Self::Shimmer,
        Self::Muffled,
        Self::Twinkle,
        Self::Sigh,
        Self::UiMove,
        Self::UiPick,
        Self::PageTurn,
    ];

    /// The pitch this sound is built at. Playing it at another pitch means
    /// playing it faster or slower, so the note the game wants divided by
    /// this is the playback rate.
    #[must_use]
    pub fn reference_hz(self) -> f32 {
        match self {
            // The middle of the range these instruments are asked for.
            // Stardust is in this list too: its partials sit an octave above
            // the note, and at 880 they were piercing.
            Self::Marimba
            | Self::Flute
            | Self::Chime
            | Self::Bell
            | Self::MusicBox
            | Self::Twinkle => 440.0,
            Self::Plink | Self::UiMove | Self::UiPick => 880.0,
            Self::Shimmer => 330.0,
            Self::Hum | Self::Muffled | Self::Sigh => 110.0,
            Self::Whoosh => 55.0,
            // Not a note: it plays at whatever rate it's given.
            Self::PageTurn => 1.0,
        }
    }

    /// How long it lasts, in seconds.
    #[must_use]
    fn seconds(self) -> f32 {
        match self {
            Self::Marimba | Self::Plink | Self::Twinkle | Self::UiMove | Self::UiPick => 0.5,
            Self::MusicBox | Self::PageTurn => 0.7,
            Self::Chime | Self::Muffled => 1.2,
            Self::Bell => 1.6,
            Self::Flute | Self::Whoosh => 1.4,
            Self::Hum => 2.2,
            Self::Shimmer | Self::Sigh => 1.8,
        }
    }
}

/// A buffer being built up, with the tools to shape it.
pub struct Buffer {
    samples: Vec<f32>,
    noise: SplitMix64,
}

impl Buffer {
    /// An empty buffer of `seconds`, ready to add sound to.
    #[must_use]
    pub fn new(seconds: f32) -> Self {
        // Durations are a few seconds at most.
        let length = (seconds * SAMPLE_RATE as f32) as usize;
        Self {
            samples: vec![0.0; length],
            noise: SplitMix64::new(NOISE_SEED),
        }
    }

    fn len(&self) -> usize {
        self.samples.len()
    }

    /// Seconds from the start of the buffer to sample `index`.
    fn time_at(&self, index: usize) -> f32 {
        index as f32 / SAMPLE_RATE as f32
    }

    /// Adds a sine wave of `hz`, fading away over `decay` seconds. `bend`
    /// lets the pitch glide: 1.0 holds it, 2.0 ends an octave up.
    pub fn partial(&mut self, hz: f32, level: f32, decay: f32, bend: f32) -> &mut Self {
        let mut phase = 0.0_f32;
        for index in 0..self.len() {
            let t = self.time_at(index);
            let progress = t / self.time_at(self.len().max(1) - 1).max(1e-6);
            let pitch = hz * (1.0 + (bend - 1.0) * progress);
            phase += TAU * pitch / SAMPLE_RATE as f32;
            // Swell to full over the attack, then fade: an instant start is
            // what makes a note sound sharp.
            let struck = 1.0 - (-t / STRIKE_ATTACK).exp();
            self.samples[index] += phase.sin() * level * struck * (-t / decay.max(1e-4)).exp();
        }
        self
    }

    /// Adds a sine that swells in and out instead of striking: the shape of
    /// a breath, or of a bowed note.
    pub fn swell(&mut self, hz: f32, level: f32, centre: f32, width: f32) -> &mut Self {
        let mut phase = 0.0_f32;
        for index in 0..self.len() {
            let t = self.time_at(index);
            phase += TAU * hz / SAMPLE_RATE as f32;
            let from_centre = (t - centre) / width.max(1e-4);
            self.samples[index] += phase.sin() * level * (-from_centre * from_centre).exp();
        }
        self
    }

    /// Adds noise under a swell: breath, wind, the hiss of a whoosh.
    pub fn breath(&mut self, level: f32, centre: f32, width: f32) -> &mut Self {
        for index in 0..self.len() {
            let t = self.time_at(index);
            let from_centre = (t - centre) / width.max(1e-4);
            let shape = (-from_centre * from_centre).exp();
            self.samples[index] += self.noise.range(-1.0, 1.0) * level * shape;
        }
        self
    }

    /// Adds a sine that never stops, breathing in and out at `lfo_hz`. Both
    /// rates are snapped to the loop, so the result repeats exactly: this is
    /// what makes a loop joinable at all.
    pub fn drone(&mut self, hz: f32, level: f32, lfo_hz: f32, depth: f32) -> &mut Self {
        let hz = snap_to_loop(hz);
        let lfo_hz = snap_to_loop(lfo_hz);
        let phase_offset = self.noise.range(0.0, TAU);
        for index in 0..self.len() {
            let t = self.time_at(index);
            let breath = 1.0 - depth + depth * (0.5 + 0.5 * (TAU * lfo_hz * t).sin());
            self.samples[index] += (TAU * hz * t + phase_offset).sin() * level * breath;
        }
        self
    }

    /// A bed of air: many quiet sines at scattered pitches, which sounds like
    /// noise but repeats exactly. Real noise can't loop.
    pub fn air(&mut self, level: f32, voices: usize, low_hz: f32, high_hz: f32) -> &mut Self {
        let each = level / (voices as f32).sqrt();
        for _ in 0..voices {
            let hz = self.noise.range(low_hz, high_hz);
            let lfo = self.noise.range(0.05, 0.4);
            self.drone(hz, each, lfo, 0.7);
        }
        self
    }

    /// Rolls off everything above `cutoff` hertz: the single most useful tool
    /// here, because it turns anything sharp into something soft.
    pub fn soften(&mut self, cutoff: f32) -> &mut Self {
        let dt = 1.0 / SAMPLE_RATE as f32;
        let rc = 1.0 / (TAU * cutoff.max(1.0));
        let alpha = dt / (rc + dt);
        // Warm the filter up on a pass it throws away, so it starts where it
        // would have been had the sound been playing forever. Without this a
        // loop's first moment is filtered differently from its last, which is
        // a seam you can hear.
        let mut held = 0.0;
        for sample in &self.samples {
            held += alpha * (*sample - held);
        }
        for sample in &mut self.samples {
            held += alpha * (*sample - held);
            *sample = held;
        }
        self
    }

    /// Puts the sound in a large, soft space: four delayed copies feeding
    /// back on themselves, then two diffusers to smear them into a wash.
    ///
    /// This is a Schroeder reverb, the oldest trick there is, and still the
    /// right one when the space is meant to be felt rather than identified.
    /// The buffer is lengthened to hold the tail.
    pub fn space(&mut self, tail: f32, mix: f32) -> &mut Self {
        let rate = SAMPLE_RATE as f32;
        self.samples
            .resize(self.len() + (tail * rate) as usize, 0.0);
        let dry = self.samples.clone();
        let mut wet = vec![0.0; self.len()];

        for delay_ms in REFLECTIONS {
            let delay = ((delay_ms / 1000.0) * rate) as usize;
            if delay == 0 || delay >= self.len() {
                continue;
            }
            // How much comes back each time round, for this tail length.
            let feedback = 10.0_f32.powf(-3.0 * (delay_ms / 1000.0) / tail.max(0.1));
            let mut line = vec![0.0_f32; delay];
            let mut cursor = 0;
            for (index, sample) in dry.iter().enumerate() {
                let echo = line[cursor];
                line[cursor] = sample + echo * feedback;
                cursor = (cursor + 1) % delay;
                wet[index] += echo;
            }
        }

        for delay_ms in DIFFUSERS {
            let delay = ((delay_ms / 1000.0) * rate) as usize;
            if delay == 0 || delay >= wet.len() {
                continue;
            }
            let mut line = vec![0.0_f32; delay];
            let mut cursor = 0;
            for sample in &mut wet {
                let held = line[cursor];
                let out = held - DIFFUSION * *sample;
                line[cursor] = *sample + DIFFUSION * out;
                cursor = (cursor + 1) % delay;
                *sample = out;
            }
        }

        // The reflections are darker than the sound that made them: distance
        // takes the top off everything.
        let loudest = wet.iter().fold(0.0_f32, |worst, s| worst.max(s.abs()));
        let evened = if loudest > 1e-6 { 1.0 / loudest } else { 0.0 };
        for (sample, reflected) in self.samples.iter_mut().zip(&wet) {
            *sample += reflected * evened * mix;
        }
        self.soften(2400.0);
        self
    }

    /// Turns the sound back to front: a strike becomes a swell towards
    /// something, which is how a reverse cymbal and our failed attempt work.
    pub fn reverse(&mut self) -> &mut Self {
        self.samples.reverse();
        self
    }

    /// Scales a loop to `peak` and leaves its edges alone: a loop must join
    /// its own start, so fading its ends would be the very thing that breaks
    /// it. Everything in it repeats exactly, so there's nothing to fade.
    pub fn finish_loop(&mut self, peak: f32) -> Vec<f32> {
        self.scale_to(peak);
        core::mem::take(&mut self.samples)
    }

    fn scale_to(&mut self, peak: f32) {
        let loudest = self
            .samples
            .iter()
            .fold(0.0_f32, |worst, sample| worst.max(sample.abs()));
        let gain = if loudest > 1e-6 { peak / loudest } else { 0.0 };
        for sample in &mut self.samples {
            *sample *= gain;
        }
    }

    /// Scales the whole buffer to the house peak, then eases its very start
    /// and end to silence so it can never click.
    pub fn finish(&mut self) -> Vec<f32> {
        self.scale_to(PEAK);
        self.ease_edges();
        core::mem::take(&mut self.samples)
    }

    /// The fade in and out that Section 6.2 asks for on every sound.
    fn ease_edges(&mut self) {
        let attack = ((SOFTEST_ATTACK * SAMPLE_RATE as f32) as usize).min(self.len() / 2);
        let tail = ((TAIL * SAMPLE_RATE as f32) as usize).min(self.len() / 2);
        let length = self.len();
        for index in 0..attack {
            // Smoothstep, so even the fade has no corner in it.
            let t = index as f32 / attack as f32;
            self.samples[index] *= t * t * (3.0 - 2.0 * t);
        }
        for index in 0..tail {
            let t = index as f32 / tail as f32;
            self.samples[length - 1 - index] *= t * t * (3.0 - 2.0 * t);
        }
    }
}

/// Builds one of the game's sounds.
#[must_use]
pub fn render(sound: Sound) -> Vec<f32> {
    let hz = sound.reference_hz();
    let mut buffer = Buffer::new(sound.seconds());
    match sound {
        // Struck metal rather than wood: two tines a hair apart, so the note
        // beats slowly against itself instead of sitting still.
        Sound::Marimba => {
            buffer
                .partial(hz, 0.9, 0.9, 1.0)
                .partial(hz * 1.004, 0.7, 0.85, 1.0)
                .partial(hz * 2.76, 0.18, 0.35, 1.0)
                .breath(0.03, 0.01, 0.01)
                .soften(2400.0)
                .space(TAIL_SECONDS, TAIL_MIX);
        }
        // Air across an edge: mostly breath, with the note growing through it.
        Sound::Flute => {
            buffer
                .swell(hz, 0.7, 0.7, 0.5)
                .swell(hz * 1.5, 0.14, 0.8, 0.4)
                // Two voices a hair apart: the waver of air in a big space.
                .swell(hz * 1.006, 0.3, 0.8, 0.55)
                .breath(0.26, 0.6, 0.55)
                .soften(1800.0)
                .space(TAIL_SECONDS, TAIL_MIX);
        }
        // Glass: high, clean, a long ring.
        Sound::Chime => {
            buffer
                .partial(hz * 2.0, 0.8, 0.8, 1.0)
                .partial(hz * 3.0, 0.22, 0.5, 1.0)
                .partial(hz * 4.02, 0.10, 0.35, 1.0)
                .soften(3600.0)
                .space(TAIL_SECONDS, TAIL_MIX);
        }
        // A bubble letting go: a little drop of pitch.
        Sound::Plink => {
            buffer
                .partial(hz, 1.0, 0.12, 0.55)
                .soften(2800.0)
                .space(1.4, 0.4);
        }
        // A bell's partials aren't whole multiples; that's what makes it a bell.
        // Glass heard from a long way off: the strike is nearly gone, only
        // the ringing is left.
        Sound::Bell => {
            buffer
                .partial(hz, 0.6, 1.4, 1.0)
                .partial(hz * 1.5, 0.3, 1.0, 1.0)
                .partial(hz * 2.76, 0.16, 0.6, 1.0)
                .partial(hz * 0.5, 0.3, 1.6, 1.0)
                .soften(2800.0)
                .space(TAIL_SECONDS, 0.65);
        }
        // Held, and rising while it holds you.
        Sound::Hum => {
            buffer
                .partial(hz, 0.9, 3.0, 1.35)
                .partial(hz * 1.497, 0.3, 2.2, 1.35)
                .partial(hz * 2.0, 0.18, 1.6, 1.35)
                .soften(900.0)
                .space(TAIL_SECONDS, 0.4);
        }
        // Weight, falling: a wide rush of air over a low note.
        Sound::Whoosh => {
            buffer
                .breath(0.9, 0.5, 0.4)
                .partial(hz, 0.8, 1.0, 0.6)
                .soften(700.0)
                .space(TAIL_SECONDS, 0.5);
        }
        // Two tines, very slightly apart, which is the music-box shimmer.
        // Further out of tune than a music box would ever be, and without
        // its sparkle: the sound of something not quite right.
        Sound::MusicBox => {
            buffer
                .partial(hz * 2.0, 0.55, 0.5, 1.0)
                .partial(hz * 2.03, 0.55, 0.45, 1.0)
                .partial(hz * 2.98, 0.12, 0.2, 1.0)
                .soften(3000.0)
                .space(TAIL_SECONDS, TAIL_MIX);
        }
        // Swells up to the moment you arrive on the other side.
        Sound::Shimmer => {
            buffer
                .breath(0.7, 0.9, 0.6)
                .partial(hz * 3.0, 0.22, 1.4, 1.0)
                .soften(3400.0)
                .reverse();
            // The open fifth you arrive in, after the swell.
            buffer
                .partial(hz, 0.35, 1.4, 1.0)
                .partial(hz * 1.5, 0.28, 1.3, 1.0)
                .space(TAIL_SECONDS, 0.6);
        }
        // The hazard: low, dull, and sorry about it.
        Sound::Muffled => {
            buffer
                .partial(hz, 0.9, 0.9, 0.92)
                .partial(hz * 1.5, 0.3, 0.5, 0.92)
                .breath(0.15, 0.1, 0.12)
                .soften(260.0)
                .space(TAIL_SECONDS, 0.45);
        }
        Sound::Twinkle => {
            buffer
                .partial(hz * 2.0, 0.8, 0.16, 1.0)
                .partial(hz * 2.997, 0.25, 0.12, 1.0)
                .soften(3600.0)
                // A mote is tiny, but it's tiny in a very large place.
                .space(2.4, 0.5);
        }
        // A breath drawn in, not a buzzer: the attempt gathering itself again.
        Sound::Sigh => {
            buffer
                .swell(hz, 0.6, 1.2, 0.65)
                .swell(hz * 1.5, 0.3, 1.3, 0.55)
                .breath(0.2, 1.1, 0.75)
                .soften(800.0)
                .reverse()
                .space(TAIL_SECONDS, 0.5);
        }
        Sound::UiMove => {
            buffer.partial(hz, 0.7, 0.09, 1.0).soften(2600.0);
        }
        Sound::UiPick => {
            buffer
                .partial(hz, 0.8, 0.22, 1.0)
                .partial(hz * 1.5, 0.4, 0.18, 1.0)
                .soften(3000.0)
                .space(0.8, 0.35);
        }
        // Paper: breath with no pitch at all.
        Sound::PageTurn => {
            buffer
                .breath(0.8, 0.12, 0.05)
                .breath(0.5, 0.26, 0.07)
                .soften(2200.0);
        }
    }
    let cutoff = match sound {
        Sound::UiMove | Sound::UiPick | Sound::PageTurn => MENU_CUTOFF,
        _ => HOUSE_CUTOFF,
    };
    buffer.soften(cutoff).finish()
}

// --- The music bed ---------------------------------------------------------

/// How long one loop of the pad or ambience lasts. Every partial in them is
/// snapped to fit whole cycles into this, so the loop joins seamlessly.
pub const LOOP_SECONDS: f32 = 8.0;

/// Which part of the bed to build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bed {
    /// The pad under every level: the level's chord, breathing slowly.
    Pad,
    /// The layer that joins once the astronaut is past halfway down.
    Deeper,
    /// The air of the place: a hum, a wind, depending on how high it is.
    Ambience,
}

/// Rounds `hz` so a whole number of its cycles fits in one loop. Without
/// this, a loop clicks every time it comes round.
#[must_use]
pub fn snap_to_loop(hz: f32) -> f32 {
    let cycles = (hz * LOOP_SECONDS).round().max(1.0);
    cycles / LOOP_SECONDS
}

/// How loud the pause version of the bed is, against the bright one.
const MUFFLED_PEAK: f32 = 0.45;
/// What's left of the bed while paused: everything above this is gone.
const MUFFLED_CUTOFF: f32 = 180.0;

/// Builds one loop of the bed, in the key of `root_hz`. `air` is how low the
/// level sits (0 in space, 1 near the ground), which decides what the
/// ambience is made of; `muffled` is the version heard while paused.
///
/// Every part of a bed is periodic within the loop, so it can be played end
/// to end forever without a tick at the join.
#[must_use]
pub fn bed(part: Bed, root_hz: f32, air: f32, muffled: bool) -> Vec<f32> {
    let mut buffer = Buffer::new(LOOP_SECONDS);
    match part {
        // The chord of the level: root, fifth and octave, each breathing at
        // its own slow rate so the chord never sits still.
        Bed::Pad => {
            let root = root_hz * 0.5;
            buffer
                .drone(root, 0.50, 1.0 / LOOP_SECONDS, 0.35)
                // The fifth, twice, a beat apart: the chord never settles.
                .drone(root * 1.5, 0.30, 2.0 / LOOP_SECONDS, 0.45)
                .drone(root * 1.503, 0.22, 3.0 / LOOP_SECONDS, 0.5)
                .drone(root * 2.0, 0.18, 3.0 / LOOP_SECONDS, 0.55)
                .drone(root * 0.5, 0.25, 1.0 / LOOP_SECONDS, 0.25)
                .soften(700.0);
        }
        // What joins underneath once the fall is properly under way. It has
        // to be *different* from the pad, not more of it: an octave lower,
        // and breathing visibly rather than holding still, so that the
        // player hears the fall deepen instead of just getting louder.
        Bed::Deeper => {
            let root = root_hz * 0.5;
            buffer
                // A slow swell, four times per loop: the pulse of falling.
                .drone(root * 0.5, 0.45, 4.0 / LOOP_SECONDS, 0.85)
                .drone(root * 0.75, 0.25, 4.0 / LOOP_SECONDS, 0.8)
                // A fourth, not a third: the chord stays open.
                .drone(root * 4.0 / 3.0, 0.22, 2.0 / LOOP_SECONDS, 0.5)
                .soften(850.0);
        }
        // High up it's a cold hum; lower down it's moving air.
        Bed::Ambience => {
            let low = air.clamp(0.0, 1.0);
            buffer
                .air(
                    0.6 + 0.3 * low,
                    48,
                    120.0 + 500.0 * low,
                    900.0 + 2500.0 * low,
                )
                .drone(
                    55.0 + 25.0 * (1.0 - low),
                    0.45 * (1.0 - low) + 0.1,
                    0.125,
                    0.6,
                )
                .soften(600.0 + 900.0 * low);
        }
    }
    if muffled {
        // What the music sounds like through a closed door, which is what
        // the pause screen wants. Quieter as well as darker.
        buffer.soften(MUFFLED_CUTOFF);
        return buffer.finish_loop(MUFFLED_PEAK);
    }
    buffer.finish_loop(PEAK)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peak(samples: &[f32]) -> f32 {
        samples
            .iter()
            .fold(0.0_f32, |worst, sample| worst.max(sample.abs()))
    }

    /// Section 6.2's two hard rules, on every sound the game can make. Beds
    /// are exempt from the fades: they loop, so their edges must meet, not
    /// fall to silence (see the seam test).
    #[test]
    fn nothing_clips_and_nothing_clicks() {
        let beds = [
            bed(Bed::Pad, 293.66, 0.5, false),
            bed(Bed::Deeper, 293.66, 0.5, false),
            bed(Bed::Ambience, 293.66, 0.5, false),
            bed(Bed::Pad, 293.66, 0.5, true),
        ];
        for samples in Sound::ALL.map(render).iter().chain(beds.iter()) {
            assert!(!samples.is_empty());
            let loudest = peak(samples);
            assert!(loudest <= 1.0, "clips at {loudest}");
            assert!(loudest > 0.4, "too quiet to hear: {loudest}");
            assert!(
                samples.iter().all(|sample| sample.is_finite()),
                "not a sound at all"
            );
        }
        for sound in Sound::ALL {
            let samples = render(sound);
            let loudest = peak(&samples);
            // The first and last moments must be all but silent.
            let edge = (SOFTEST_ATTACK * SAMPLE_RATE as f32) as usize / 2;
            assert!(
                peak(&samples[..edge]) < loudest * 0.5,
                "{sound:?} starts with a click"
            );
            assert!(
                peak(&samples[samples.len() - edge..]) < loudest * 0.5,
                "{sound:?} ends with a click"
            );
        }
    }

    /// Nothing played in a level may stop dead: the tail is what makes the
    /// place sound big. Menu notes are exempt, being nowhere in particular.
    #[test]
    fn notes_keep_ringing_after_they_are_played() {
        /// How long the sound stays within 1 % of its loudest moment: how
        /// long you can still hear it, in other words.
        fn rings_for(samples: &[f32]) -> f32 {
            let block = SAMPLE_RATE as usize / 20;
            let blocks: Vec<f32> = samples
                .chunks(block)
                .map(|chunk| {
                    let sum: f32 = chunk.iter().map(|sample| sample * sample).sum();
                    (sum / chunk.len().max(1) as f32).sqrt()
                })
                .collect();
            let loudest = blocks.iter().fold(0.0_f32, |worst, b| worst.max(*b));
            let last = blocks
                .iter()
                .rposition(|level| *level > loudest * 0.01)
                .unwrap_or(0);
            last as f32 * 0.05
        }

        for sound in [
            Sound::Marimba,
            Sound::Bell,
            Sound::Chime,
            Sound::Flute,
            Sound::MusicBox,
            Sound::Twinkle,
        ] {
            let rings = rings_for(&render(sound));
            assert!(rings > 1.0, "{sound:?} stops after {rings:.2}s");
        }
        // A menu note is close by, and stays close.
        assert!(rings_for(&render(Sound::UiMove)) < 0.6);
    }

    /// A loop that doesn't join is a tick, once every eight seconds, forever.
    /// The join must be no bigger a step than any other pair of samples.
    #[test]
    fn the_music_bed_loops_without_a_seam() {
        for part in [Bed::Pad, Bed::Deeper, Bed::Ambience] {
            let samples = bed(part, 293.66, 0.5, false);
            let expected = (LOOP_SECONDS * SAMPLE_RATE as f32) as usize;
            assert!(
                samples.len().abs_diff(expected) <= 1,
                "{part:?}: wrong length"
            );
            let typical: f32 = samples
                .windows(2)
                .map(|pair| (pair[1] - pair[0]).abs())
                .sum::<f32>()
                / samples.len() as f32;
            let join = (samples[0] - samples[samples.len() - 1]).abs();
            assert!(
                join <= typical * 4.0,
                "{part:?}: the join jumps {join}, ordinary steps are {typical}"
            );
        }
    }

    /// Partials must fit the loop exactly, or the seam comes back.
    #[test]
    fn snapping_fits_whole_cycles_into_a_loop() {
        for hz in [55.0_f32, 110.0, 146.83, 293.66, 440.0] {
            let snapped = snap_to_loop(hz);
            let cycles = snapped * LOOP_SECONDS;
            assert!((cycles - cycles.round()).abs() < 1e-3, "{hz} -> {snapped}");
            // Close enough that nobody can hear the difference.
            assert!(
                (snapped / hz).log2().abs() * 1200.0 < 5.0,
                "{hz} moved too far"
            );
        }
    }

    /// The muffled version is what the pause screen hears: the same music,
    /// with the top taken off it.
    #[test]
    fn the_muffled_bed_is_darker_than_the_bright_one() {
        /// How much the samples jump about: more wiggle, more treble.
        fn wiggle(samples: &[f32]) -> f32 {
            let sum: f32 = samples
                .windows(2)
                .map(|pair| (pair[1] - pair[0]).abs())
                .sum();
            sum / samples.len() as f32
        }
        for part in [Bed::Pad, Bed::Deeper, Bed::Ambience] {
            let bright = bed(part, 293.66, 0.5, false);
            let dark = bed(part, 293.66, 0.5, true);
            assert!(
                wiggle(&dark) < wiggle(&bright) * 0.7,
                "{part:?}: the pause version isn't muffled"
            );
            assert!(
                peak(&dark) < peak(&bright),
                "{part:?}: it isn't quieter either"
            );
        }
    }

    /// What the music costs to make and to keep. A measurement, not a check:
    /// `cargo test --release --lib -- --ignored bed_cost --nocapture`.
    #[test]
    #[ignore = "a measurement, not a check"]
    #[expect(clippy::print_stdout, reason = "this test reports a measurement")]
    fn bed_cost() {
        let started = std::time::Instant::now();
        let loops = [
            bed(Bed::Pad, 293.66, 0.5, false),
            bed(Bed::Pad, 293.66, 0.5, true),
            bed(Bed::Deeper, 293.66, 0.5, false),
            bed(Bed::Ambience, 293.66, 0.5, false),
        ];
        let built = started.elapsed();
        let samples: usize = loops.iter().map(Vec::len).sum();
        // The mixer holds each one as stereo pairs of f32.
        let megabytes = (samples * 2 * size_of::<f32>()) as f32 / (1024.0 * 1024.0);
        println!(
            "one altitude's music: {:.0} ms to build, {megabytes:.1} MB to hold",
            built.as_secs_f64() * 1000.0
        );
        let one_shots: usize = Sound::ALL.iter().map(|sound| render(*sound).len()).sum();
        let one_shot_mb = (one_shots * 2 * size_of::<f32>()) as f32 / (1024.0 * 1024.0);
        println!("every one-shot sound: {one_shot_mb:.2} MB");
        // Budgets, so a louder idea can't quietly cost a hundred megabytes.
        assert!(
            megabytes < 8.0,
            "one altitude's music holds {megabytes:.1} MB"
        );
        assert!(one_shot_mb < 12.0, "the sounds hold {one_shot_mb:.1} MB");
    }

    /// The same sound every time: no sound may drift or surprise.
    #[test]
    fn sounds_are_the_same_every_time() {
        for sound in Sound::ALL {
            assert_eq!(render(sound), render(sound), "{sound:?} changed");
        }
    }
}
