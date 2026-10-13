//! The little music notes that rise from whatever the astronaut just touched.
//!
//! Every object touched and every mote collected already plays a note
//! (`audio.rs`). Nothing on screen said so, so the melody read as sound
//! effects. A sign is that note made visible: the pitch it stands for
//! decides how high it starts and how small it is, so a climbing melody
//! climbs the screen too.
//!
//! Like the sparks in `particles.rs`, the signs come from a pool spawned
//! once and reused, so no amount of touching can grow the world.

use bevy::prelude::*;

use crate::app_state::AppState;
use crate::astronaut::Astronaut;
use crate::attempt::AttemptRestarted;
use crate::collectibles::StardustCollected;
use crate::music::{stardust_step, touch_step};
use crate::objects::{ObjectTouched, spark_color};
use crate::physics::Motion;
use crate::settings::Settings;

/// How many signs can be in the air at once. A fast run touches a handful.
const POOL: usize = 12;
/// How long a sign takes to rise and fade.
const RISE_SECONDS: f32 = 0.9;
/// How far it travels in that time, in world units.
const RISE_DISTANCE: f32 = 130.0;
/// How far it sways sideways on the way up.
const SWAY: f32 = 16.0;
/// How fast it sways, in radians per second.
const SWAY_SPEED: f32 = 4.5;
/// A sign's size in world units, before pitch shrinks it.
const SIGN_SIZE: f32 = 36.0;
/// In front of the scenery and the sparks, with the astronaut.
const SIGN_DEPTH: f32 = 6.0;
/// The note a mote rings: the stardust's own warm gold.
const MOTE_COLOR: Color = Color::linear_rgba(2.6, 2.0, 0.8, 1.0);

/// The highest scale step a sign is drawn for. Touches climb two octaves of
/// a three-note ladder, so nothing reaches past this.
const TOP_STEP: f32 = 10.0;
/// How much further up a sign starts for the top note than the bottom one.
const PITCH_LIFT: f32 = 70.0;
/// How much smaller the top note is drawn than the bottom one.
const PITCH_SHRINK: f32 = 0.3;

/// How big the drawn note is, in pixels. Made once, shown at any size.
const NOTE_PIXELS: u32 = 64;

/// The note picture, made once and used by the signs and by the phrase row.
#[derive(Resource)]
pub(crate) struct NoteArt(pub Handle<Image>);

/// A sign in flight: how long it has been up, where it started, and the
/// pitch it stands for.
#[derive(Component, Debug, Default, Clone, Copy)]
struct NoteSign {
    /// Seconds since it appeared. At or past [`RISE_SECONDS`] it is resting.
    age: f32,
    from: Vec2,
    /// 0 for the lowest note drawn, 1 for the highest.
    pitch: f32,
    /// How much of the motion the player asked for (reduce motion).
    calm: f32,
}

/// The pool: which slot to use next.
#[derive(Resource)]
struct SignPool {
    slots: Vec<Entity>,
    next: usize,
}

/// Where this attempt's melody has reached.
///
/// `audio.rs` counts the same two things for the same reason, and both reset
/// on [`AttemptRestarted`]. They are kept apart because the sound is played
/// as the message arrives and the picture is drawn from the same message, so
/// neither can read the other's count without an ordering rule between two
/// plugins that are otherwise independent. `steps_match_what_audio_plays`
/// below holds them to the same note.
#[derive(Resource, Default)]
struct MelodySoFar {
    touches: u32,
    motes: u32,
}

/// Notes you can see.
pub(crate) struct NoteSignsPlugin;

impl Plugin for NoteSignsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MelodySoFar>()
            .add_systems(Startup, prepare_signs)
            .add_systems(
                Update,
                (
                    sign_a_touch.run_if(on_message::<ObjectTouched>),
                    sign_a_mote.run_if(on_message::<StardustCollected>),
                    rest_every_sign.run_if(on_message::<AttemptRestarted>),
                    advance_signs,
                ),
            )
            // Leaving the fall takes the air with it. The melody's place is
            // left alone: a pause is not a new attempt.
            .add_systems(OnExit(AppState::Playing), clear_the_air);
    }
}

// --- The shapes of the motion ----------------------------------------------

/// How far a sign has risen, `age` seconds in: quick at first and slowing to
/// a stop, like something let go of where there is nothing to fall towards.
#[must_use]
fn sign_rise(age: f32, pitch: f32) -> f32 {
    let t = (age / RISE_SECONDS).clamp(0.0, 1.0);
    PITCH_LIFT * pitch + RISE_DISTANCE * (1.0 - (1.0 - t) * (1.0 - t))
}

/// How far a sign has drifted sideways: a slow sway that widens as it rises.
#[must_use]
fn sign_sway(age: f32) -> f32 {
    let t = (age / RISE_SECONDS).clamp(0.0, 1.0);
    (age * SWAY_SPEED).sin() * SWAY * t
}

/// How solid a sign is: there at once, thinning away over its whole life.
#[must_use]
fn sign_alpha(age: f32) -> f32 {
    let t = (age / RISE_SECONDS).clamp(0.0, 1.0);
    let appear = (t / 0.08).clamp(0.0, 1.0);
    appear * (1.0 - t) * (1.0 - t)
}

/// How big a sign is drawn: higher notes are smaller, the way a high note
/// sounds lighter.
#[must_use]
fn sign_size(pitch: f32) -> f32 {
    SIGN_SIZE * (1.0 - PITCH_SHRINK * pitch.clamp(0.0, 1.0))
}

/// Where a scale step sits between the lowest note drawn and the highest.
#[must_use]
fn pitch_of(step: i32) -> f32 {
    f32::from(i16::try_from(step).unwrap_or(0)) / TOP_STEP
}

// --- The pool ---------------------------------------------------------------

/// Draws the note once and fills the pool with it.
fn prepare_signs(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let art = images.add(note_image());
    let slots = (0..POOL)
        .map(|_| {
            commands
                .spawn((
                    NoteSign::default(),
                    Sprite {
                        image: art.clone(),
                        custom_size: Some(Vec2::splat(SIGN_SIZE)),
                        ..default()
                    },
                    Transform::default(),
                    Visibility::Hidden,
                ))
                .id()
        })
        .collect();
    commands.insert_resource(SignPool { slots, next: 0 });
    commands.insert_resource(NoteArt(art));
}

/// Puts one sign in the air. The oldest slot makes way when they are all busy.
fn release(
    pool: &mut SignPool,
    signs: &mut Query<(&mut NoteSign, &mut Transform, &mut Sprite, &mut Visibility)>,
    at: Vec2,
    step: i32,
    color: Color,
    calm: f32,
) {
    let Some(&slot) = pool.slots.get(pool.next % pool.slots.len()) else {
        return;
    };
    pool.next = pool.next.wrapping_add(1);
    let Ok((mut sign, mut transform, mut sprite, mut visibility)) = signs.get_mut(slot) else {
        return;
    };
    let pitch = pitch_of(step).clamp(0.0, 1.0);
    *sign = NoteSign {
        age: 0.0,
        from: at,
        pitch,
        calm,
    };
    transform.translation = at.extend(SIGN_DEPTH);
    sprite.color = color.with_alpha(0.0);
    sprite.custom_size = Some(Vec2::splat(sign_size(pitch)));
    visibility.set_if_neq(Visibility::Inherited);
}

/// Every object touched sends up the note it just played, in the object's
/// own colour.
fn sign_a_touch(
    mut touched: MessageReader<ObjectTouched>,
    astronaut: Option<Single<&Motion, With<Astronaut>>>,
    settings: Res<Settings>,
    mut melody: ResMut<MelodySoFar>,
    pool: Option<ResMut<SignPool>>,
    mut signs: Query<(&mut NoteSign, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let (Some(astronaut), Some(mut pool)) = (astronaut, pool) else {
        return;
    };
    let at = astronaut.body().position;
    let calm = settings.motion_scale();
    for touch in touched.read() {
        let step = touch_step(touch.kind, melody.touches);
        melody.touches += 1;
        release(
            &mut pool,
            &mut signs,
            at,
            step,
            spark_color(touch.kind),
            calm,
        );
    }
}

/// So does every mote, in the stardust's own gold.
fn sign_a_mote(
    mut collected: MessageReader<StardustCollected>,
    astronaut: Option<Single<&Motion, With<Astronaut>>>,
    settings: Res<Settings>,
    mut melody: ResMut<MelodySoFar>,
    pool: Option<ResMut<SignPool>>,
    mut signs: Query<(&mut NoteSign, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let (Some(astronaut), Some(mut pool)) = (astronaut, pool) else {
        return;
    };
    let at = astronaut.body().position;
    let calm = settings.motion_scale();
    for _ in collected.read() {
        let step = stardust_step(melody.motes);
        melody.motes += 1;
        release(&mut pool, &mut signs, at, step, MOTE_COLOR, calm);
    }
}

/// A fresh attempt starts a fresh melody, and clears the air.
fn rest_every_sign(
    mut melody: ResMut<MelodySoFar>,
    signs: Query<(&mut NoteSign, &mut Visibility)>,
) {
    *melody = MelodySoFar::default();
    clear_the_air(signs);
}

/// Takes down every sign still in the air.
fn clear_the_air(mut signs: Query<(&mut NoteSign, &mut Visibility)>) {
    for (mut sign, mut visibility) in &mut signs {
        sign.age = RISE_SECONDS;
        visibility.set_if_neq(Visibility::Hidden);
    }
}

/// Lifts and fades every sign in the air. Resting slots cost one comparison.
fn advance_signs(
    time: Res<Time>,
    mut signs: Query<(&mut NoteSign, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let delta = time.delta_secs();
    for (mut sign, mut transform, mut sprite, mut visibility) in &mut signs {
        if sign.age >= RISE_SECONDS {
            continue;
        }
        sign.age += delta;
        let alpha = sign_alpha(sign.age);
        if sign.age >= RISE_SECONDS {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        }
        let offset = Vec2::new(sign_sway(sign.age), sign_rise(sign.age, sign.pitch)) * sign.calm;
        transform.translation = (sign.from + offset).extend(SIGN_DEPTH);
        sprite.color.set_alpha(alpha);
    }
}

// --- The picture ------------------------------------------------------------

/// Distance from a point to a rectangle of half-size `half` around the
/// origin. Negative inside it.
fn box_distance(p: Vec2, half: Vec2) -> f32 {
    let q = p.abs() - half;
    q.max(Vec2::ZERO).length() + q.max_element().min(0.0)
}

/// Distance from `p` to the segment `a`-`b`, and how far along that segment
/// the nearest point lies: 0 at `a`, 1 at `b`.
fn segment_distance(p: Vec2, a: Vec2, b: Vec2) -> (f32, f32) {
    let along = b - a;
    let t = ((p - a).dot(along) / along.length_squared().max(1e-6)).clamp(0.0, 1.0);
    ((p - (a + along * t)).length(), t)
}

/// Draws a quaver: a tilted oval head, a stem up its right side, and a flag
/// off the top. White, with the shape in the alpha, so whatever shows it
/// picks the colour — the same trick as the star in `ui.rs`.
fn note_image() -> Image {
    let size = NOTE_PIXELS as usize;
    // Pixel coordinates in a 64-wide picture: exact in f32.
    let at = |value: usize| u16::try_from(value).map_or(0.0, f32::from);
    let last = at(size) - 1.0;
    // The note is laid out in a 0..1 square, y downwards, then moved to sit
    // in the middle of it: a note is not a symmetrical shape, and a sign
    // that rises off-centre looks like it came from somewhere else.
    const SHIFT: Vec2 = Vec2::new(0.07, 0.08);
    let head_middle = Vec2::new(0.34, 0.74);
    let head_radii = Vec2::new(0.21, 0.155);
    /// How far the head leans, in radians. A written note is never upright.
    const TILT: f32 = -0.36;
    let stem_x = head_middle.x + head_radii.x - 0.04;
    let stem_top = 0.09;
    let stem_half = Vec2::new(0.036, (head_middle.y - stem_top) / 2.0);
    let stem_middle = Vec2::new(stem_x, (head_middle.y + stem_top) / 2.0);
    // The flag leaves the top of the stem and falls away to the right.
    let flag_from = Vec2::new(stem_x, stem_top + 0.02);
    let flag_to = Vec2::new(stem_x + 0.21, stem_top + 0.33);
    // Soft over about a pixel and a half, so the edge never stairsteps.
    let edge = 1.5 / last;
    let (sin, cos) = TILT.sin_cos();
    let mut pixels = Vec::with_capacity(size * size * 4);
    for y in 0..size {
        for x in 0..size {
            let p = Vec2::new(at(x) / last, at(y) / last) - SHIFT;
            // The head: an ellipse, turned. Scaling the distance back by the
            // smaller radius turns the stretched measure into real distance.
            let from_middle = p - head_middle;
            let turned = Vec2::new(
                from_middle.x * cos - from_middle.y * sin,
                from_middle.x * sin + from_middle.y * cos,
            );
            let head = ((turned / head_radii).length() - 1.0) * head_radii.min_element();
            let stem = box_distance(p - stem_middle, stem_half);
            // The flag thins as it falls away from the stem.
            let (reach, along) = segment_distance(p, flag_from, flag_to);
            let flag = reach - (0.055 * (1.0 - along) + 0.013);
            let shape = head.min(stem).min(flag);
            let alpha = (0.5 - shape / edge).clamp(0.0, 1.0);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a clamped fraction"
            )]
            let byte = (alpha * 255.0) as u8;
            pixels.extend_from_slice(&[255, 255, 255, byte]);
        }
    }
    Image::new(
        bevy::render::render_resource::Extent3d {
            width: NOTE_PIXELS,
            height: NOTE_PIXELS,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        pixels,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::{PENTATONIC, key_for, step_hz, touch_step};
    use crate::objects::ObjectKind;

    /// The alpha of one pixel of the note picture, by its place in the 0..1
    /// square the note is drawn in.
    fn ink(image: &Image, x: f32, y: f32) -> u8 {
        let size = NOTE_PIXELS as usize;
        let last = f32::from(u16::try_from(size - 1).unwrap_or(0));
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a fraction of 64"
        )]
        let pixel = |v: f32| ((v * last).round() as usize).min(size - 1);
        let at = (pixel(y) * size + pixel(x)) * 4 + 3;
        image.data.as_ref().map_or(0, |data| data[at])
    }

    /// The note has to read as a note: a head down on the left, a stem up its
    /// right side, a flag off the top, and nothing in the corners.
    #[test]
    fn the_picture_is_a_note() {
        let image = note_image();
        assert!(ink(&image, 0.41, 0.82) > 250, "the head is solid");
        assert!(ink(&image, 0.58, 0.48) > 250, "the stem runs up from it");
        assert!(ink(&image, 0.67, 0.30) > 128, "the flag hangs off the top");
        assert_eq!(ink(&image, 0.02, 0.02), 0, "the corners are empty");
        assert_eq!(ink(&image, 0.98, 0.98), 0);
        // The head is left of the stem, not centred under it.
        assert_eq!(ink(&image, 0.27, 0.38), 0, "nothing above the head");
    }

    #[test]
    fn a_sign_rises_and_fades_away() {
        assert!(sign_alpha(0.0) < 0.01, "it appears rather than blinks on");
        assert!(sign_alpha(0.1) > 0.7, "and is there straight away");
        assert!(sign_alpha(RISE_SECONDS) < 1e-6, "gone by the end");
        assert!(sign_rise(0.0, 0.0) < 1e-6);
        // Slowing, not speeding up: the first half covers more than the second.
        let half = sign_rise(RISE_SECONDS / 2.0, 0.0);
        let all = sign_rise(RISE_SECONDS, 0.0);
        assert!(half > all / 2.0, "{half} of {all}");
        assert!((all - RISE_DISTANCE).abs() < 1e-3);
    }

    #[test]
    fn a_higher_note_starts_higher_and_smaller() {
        assert!(sign_rise(0.0, 1.0) > sign_rise(0.0, 0.0));
        assert!(sign_size(1.0) < sign_size(0.0));
        // Every step a touch can play stays inside the range that was drawn for.
        for streak in 0..64 {
            for kind in ObjectKind::ALL {
                let pitch = pitch_of(touch_step(kind, streak));
                assert!((0.0..=1.0).contains(&pitch), "{kind:?} at {streak}");
            }
        }
        for collected in 0..64 {
            let pitch = pitch_of(stardust_step(collected));
            assert!((0.0..=1.0).contains(&pitch), "mote {collected}");
        }
    }

    /// The sign drawn must stand for the note played. Both count touches
    /// from the start of the attempt and both ask `music.rs` for the step,
    /// so a melody that climbs in the ear climbs on the screen.
    #[test]
    fn steps_match_what_audio_plays() {
        let key = key_for(crate::level::Theme::DeepSpace);
        let mut melody = MelodySoFar::default();
        let mut previous = 0.0;
        for _ in 0..PENTATONIC.len() {
            let step = touch_step(ObjectKind::CloudPuff, melody.touches);
            melody.touches += 1;
            let hz = step_hz(key, step);
            assert!(hz > previous, "the melody climbs");
            assert!(pitch_of(step) >= pitch_of(0), "and so does the sign");
            previous = hz;
        }
    }
}
