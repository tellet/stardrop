//! Everything that moves for the look of it: objects breathing, wind flowing,
//! the astronaut's trail and visor, sparks on contact, the landing.
//!
//! Nothing here changes the game: it reacts to what the rules already
//! decided. The shapes of the motions are plain functions with tests; the
//! systems only apply them.

use bevy::prelude::*;

use crate::astronaut::{Astronaut, CaptureBubble, Shimmer, VisorFlash};
use crate::level::{BeaconHalo, LevelCompleted, StardustMote, Twinkle, still_falling};
use crate::objects::{ObjectKind, ObjectTouched, spark_color};
use crate::particles::Burst;
use crate::physics::Motion;
use crate::settings::Settings;

/// How long a contact squish lasts, in seconds.
const PULSE_SECONDS: f32 = 0.35;
/// At its widest, the object is this much wider and this much shorter.
const PULSE_WIDEN: f32 = 0.12;
const PULSE_FLATTEN: f32 = 0.08;

/// How long the visor stays lit after touching something.
const FLASH_SECONDS: f32 = 0.22;

/// A spark is left behind this often while falling, in seconds.
const TRAIL_INTERVAL: f32 = 0.045;
/// The trail's colour: the astronaut's own warm light.
const TRAIL_COLOR: Color = Color::linear_rgba(2.0, 1.5, 0.9, 0.9);
/// The landing celebration's colour.
const LANDING_COLOR: Color = Color::linear_rgba(2.6, 2.0, 1.0, 1.0);

/// How far the beacon's light swells as it breathes, and how slowly.
const BEACON_BREATH: f32 = 0.10;
const BEACON_PERIOD: f32 = 3.4;
/// The same for a mote of stardust, which twinkles faster.
const TWINKLE_AMOUNT: f32 = 0.16;
const TWINKLE_PERIOD: f32 = 1.9;

/// How fast the wind's chevrons travel along their ribbon, and how far apart
/// in the wave two neighbouring chevrons are.
const FLOW_SPEED: f32 = 1.6;
const FLOW_WAVELENGTH: f32 = 260.0;

/// A running squish, if any: seconds since it started.
#[derive(Component, Debug, Default, Clone, Copy)]
pub(crate) struct ContactPulse(Option<f32>);

/// An object's idle motion: what it is, and where in its own cycle it sits so
/// that two of the same kind never breathe in step.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct Idle {
    pub kind: ObjectKind,
    pub phase: f32,
}

/// One chevron of a wind ribbon, and how far along the ribbon it sits.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct FlowMark {
    pub distance: f32,
}

/// The visor flash's remaining time.
#[derive(Resource, Debug, Default)]
struct FlashLeft(f32);

/// Visual reactions to gameplay events.
pub(crate) struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FlashLeft>().add_systems(
            Update,
            (
                (
                    react_to_contact.run_if(on_message::<ObjectTouched>),
                    animate_objects,
                )
                    .chain(),
                flow_chevrons,
                (show_shimmer, show_capture_bubble),
                (light_visor, fade_visor_flash).chain(),
                leave_a_trail.run_if(still_falling),
                (breathe_beacon, twinkle_stardust),
                celebrate_landing.run_if(on_message::<LevelCompleted>),
            ),
        );
    }
}

// --- The shapes of the motions ---------------------------------------------

/// Scale of a squish `elapsed` seconds in: a smooth bump (half a sine wave)
/// from 1 out to the widest point and back, never a snap.
#[must_use]
pub(crate) fn pulse_scale(elapsed: f32) -> Vec2 {
    let t = (elapsed / PULSE_SECONDS).clamp(0.0, 1.0);
    let bump = (t * core::f32::consts::PI).sin();
    Vec2::new(1.0 + PULSE_WIDEN * bump, 1.0 - PULSE_FLATTEN * bump)
}

/// How much an object of this kind swells and shrinks as it idles: soft
/// things breathe, glassy ones barely move, the hazard crackles.
#[must_use]
pub(crate) fn idle_scale(kind: ObjectKind, t: f32) -> Vec2 {
    let (amount, speed) = match kind {
        ObjectKind::CloudPuff => (0.035, 0.7),
        ObjectKind::Bubble => (0.05, 1.1),
        ObjectKind::GravityOrb => (0.045, 0.9),
        ObjectKind::TwinPortals => (0.04, 1.4),
        ObjectKind::SporeFlower => (0.03, 1.2),
        ObjectKind::StaticCloud => (0.022, 5.5),
        ObjectKind::Crystal | ObjectKind::CometStone | ObjectKind::WindRibbon => (0.0, 0.0),
    };
    let breath = (t * speed).sin() * amount;
    // Wider as it flattens, like something soft holding its volume.
    Vec2::new(1.0 + breath, 1.0 - breath * 0.6)
}

/// How far an object of this kind has turned while idling. The flower turns
/// all the way round, slowly; everything else only sways.
#[must_use]
pub(crate) fn idle_spin(kind: ObjectKind, t: f32) -> f32 {
    match kind {
        ObjectKind::SporeFlower => t * 0.22,
        ObjectKind::Crystal => (t * 0.6).sin() * 0.10,
        ObjectKind::CometStone => (t * 0.8).sin() * 0.05,
        ObjectKind::StaticCloud => (t * 0.35).sin() * 0.16,
        ObjectKind::CloudPuff => (t * 0.5).sin() * 0.03,
        // Turning a ribbon would drag its whole path around; the round ones
        // have no "up" to turn.
        ObjectKind::Bubble
        | ObjectKind::GravityOrb
        | ObjectKind::TwinPortals
        | ObjectKind::WindRibbon => 0.0,
    }
}

/// How big a chevron is at time `t`, `distance` units along its ribbon: a
/// bright pulse travelling along the flow, so the wind reads as moving.
#[must_use]
pub(crate) fn flow_scale(t: f32, distance: f32) -> f32 {
    let wave = (t * FLOW_SPEED - distance / FLOW_WAVELENGTH * core::f32::consts::TAU).sin();
    0.75 + 0.35 * wave
}

// --- Objects ---------------------------------------------------------------

/// One contact: the object squishes, sparks fly in its colour, and the
/// astronaut's visor catches the light.
fn react_to_contact(
    mut touched: MessageReader<ObjectTouched>,
    mut pulses: Query<(&mut ContactPulse, &GlobalTransform)>,
    astronaut: Option<Single<&Motion, With<Astronaut>>>,
    mut bursts: MessageWriter<Burst>,
    mut flash: ResMut<FlashLeft>,
) {
    for event in touched.read() {
        let Ok((mut pulse, transform)) = pulses.get_mut(event.entity) else {
            continue;
        };
        // A ribbon is a long path: squishing it would drag the whole thing
        // around its middle. Its reaction is the sparks and its own flow.
        if event.kind != ObjectKind::WindRibbon {
            pulse.0 = Some(0.0);
        }
        flash.0 = FLASH_SECONDS;
        // Sparks come from where the astronaut is, not the object's middle:
        // that's where the player is looking.
        let at = astronaut.as_ref().map_or_else(
            || transform.translation().truncate(),
            |motion| motion.body().position,
        );
        bursts.write(Burst {
            at,
            color: spark_color(event.kind),
            count: 12,
            speed: 170.0,
            life: 0.55,
            size: 7.0,
            drift: Vec2::ZERO,
        });
    }
}

/// Breathes, sways and squishes every object. One pass over the level's
/// objects, which is a couple of dozen entities at most.
fn animate_objects(
    time: Res<Time>,
    settings: Res<Settings>,
    mut objects: Query<(&Idle, &mut ContactPulse, &mut Transform)>,
) {
    let now = time.elapsed_secs();
    let calm = settings.motion_scale();
    for (idle, mut pulse, mut transform) in &mut objects {
        let t = now + idle.phase;
        let idle_scale = Vec2::ONE.lerp(idle_scale(idle.kind, t), calm);
        let squish = match pulse.0 {
            Some(elapsed) => {
                let elapsed = elapsed + time.delta_secs();
                pulse.0 = (elapsed < PULSE_SECONDS).then_some(elapsed);
                pulse_scale(elapsed)
            }
            None => Vec2::ONE,
        };
        transform.scale = (idle_scale * squish).extend(1.0);
        transform.rotation = Quat::from_rotation_z(idle_spin(idle.kind, t) * calm);
    }
}

/// Sends a pulse of brightness travelling along each wind ribbon.
fn flow_chevrons(
    time: Res<Time>,
    settings: Res<Settings>,
    mut chevrons: Query<(&FlowMark, &mut Transform)>,
) {
    if chevrons.is_empty() {
        return;
    }
    let t = time.elapsed_secs() * settings.motion_scale();
    for (mark, mut transform) in &mut chevrons {
        let scale = flow_scale(t, mark.distance);
        transform.scale = Vec3::splat(scale * crate::objects::CHEVRON_SIZE);
    }
}

/// How big the beacon's light is at time `t`: a slow, even breath, so the
/// goal is always the calmest moving thing on screen.
#[must_use]
pub(crate) fn beacon_breath(t: f32) -> f32 {
    1.0 + BEACON_BREATH * (t * core::f32::consts::TAU / BEACON_PERIOD).sin()
}

/// How big a mote of stardust is at time `t`: a quicker sparkle.
#[must_use]
pub(crate) fn twinkle_scale(t: f32) -> f32 {
    1.0 + TWINKLE_AMOUNT * (t * core::f32::consts::TAU / TWINKLE_PERIOD).sin()
}

/// Breathes the light above the beacon. The pad itself never moves: the
/// player is aiming at it.
fn breathe_beacon(
    time: Res<Time>,
    settings: Res<Settings>,
    mut halos: Query<(&BeaconHalo, &mut Transform)>,
) {
    if halos.is_empty() {
        return;
    }
    let breath = 1.0 + (beacon_breath(time.elapsed_secs()) - 1.0) * settings.motion_scale();
    for (halo, mut transform) in &mut halos {
        transform.scale = (halo.0 * breath).extend(1.0);
    }
}

/// Twinkles each mote of stardust, each at its own point in the cycle.
fn twinkle_stardust(
    time: Res<Time>,
    settings: Res<Settings>,
    mut motes: Query<(&Twinkle, &mut Transform), With<StardustMote>>,
) {
    let now = time.elapsed_secs();
    let calm = settings.motion_scale();
    for (twinkle, mut transform) in &mut motes {
        let scale = 1.0 + (twinkle_scale(now + twinkle.0) - 1.0) * calm;
        transform.scale = Vec3::splat(scale);
    }
}

// --- The astronaut ---------------------------------------------------------

/// Leaves a faint spark behind every so often while falling, so the recent
/// path stays visible for a moment.
fn leave_a_trail(
    time: Res<Time>,
    mut since: Local<f32>,
    astronaut: Option<Single<&Motion, With<Astronaut>>>,
    mut bursts: MessageWriter<Burst>,
) {
    let Some(motion) = astronaut else {
        return;
    };
    *since += time.delta_secs();
    if *since < TRAIL_INTERVAL {
        return;
    }
    *since = 0.0;
    let body = motion.body();
    bursts.write(Burst {
        at: body.position,
        color: TRAIL_COLOR,
        count: 1,
        speed: 18.0,
        life: 0.5,
        size: 5.0,
        // A little of the astronaut's own motion, so the trail streams away.
        drift: -body.velocity * 0.06,
    });
}

fn light_visor(flash: Res<FlashLeft>, mut visor: Query<&mut Visibility, With<VisorFlash>>) {
    for mut visibility in &mut visor {
        visibility.set_if_neq(if flash.0 > 0.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

/// Shrinks the flash away over its short life.
fn fade_visor_flash(
    time: Res<Time>,
    mut flash: ResMut<FlashLeft>,
    mut visor: Query<&mut Transform, With<VisorFlash>>,
) {
    if flash.0 <= 0.0 {
        return;
    }
    flash.0 = (flash.0 - time.delta_secs()).max(0.0);
    let left = flash.0 / FLASH_SECONDS;
    for mut transform in &mut visor {
        transform.scale = crate::astronaut::visor_flash_scale(left).extend(1.0);
    }
}

/// A soft burst of light when the astronaut reaches the beacon.
fn celebrate_landing(
    astronaut: Option<Single<&Motion, With<Astronaut>>>,
    mut bursts: MessageWriter<Burst>,
) {
    let Some(motion) = astronaut else {
        return;
    };
    let at = motion.body().position;
    // A bright close burst inside a slower, wider one.
    bursts.write(Burst {
        at,
        color: LANDING_COLOR,
        count: 26,
        speed: 300.0,
        life: 1.1,
        size: 9.0,
        drift: Vec2::Y * 40.0,
    });
    bursts.write(Burst {
        at,
        color: TRAIL_COLOR,
        count: 14,
        speed: 90.0,
        life: 1.6,
        size: 5.0,
        drift: Vec2::Y * 15.0,
    });
}

/// Shows a bubble around the astronaut exactly while one is carrying it.
fn show_capture_bubble(
    astronaut: Query<&Motion, With<Astronaut>>,
    mut bubble: Query<&mut Visibility, With<CaptureBubble>>,
) {
    let held = astronaut
        .iter()
        .any(|motion| motion.body().effects.held_by_bubble());
    for mut visibility in &mut bubble {
        visibility.set_if_neq(if held {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

/// Shows the shimmer around the astronaut exactly while spores have swapped
/// the controls.
fn show_shimmer(
    astronaut: Query<&Motion, With<Astronaut>>,
    mut shimmer: Query<&mut Visibility, With<Shimmer>>,
) {
    let swapped = astronaut
        .iter()
        .any(|motion| motion.body().effects.controls_swapped());
    for mut visibility in &mut shimmer {
        visibility.set_if_neq(if swapped {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulse_starts_and_ends_at_normal_size() {
        assert!((pulse_scale(0.0) - Vec2::ONE).length() < 1e-6);
        assert!((pulse_scale(PULSE_SECONDS) - Vec2::ONE).length() < 1e-5);
        let middle = pulse_scale(PULSE_SECONDS / 2.0);
        assert!(middle.x > 1.1 && middle.y < 0.93);
    }

    /// Idle motion must stay a whisper: it plays under everything else, all
    /// the time, and must never be mistaken for the game reacting.
    #[test]
    fn idle_motion_stays_gentle() {
        for kind in ObjectKind::ALL {
            for step in 0..600_u16 {
                let t = f32::from(step) / 20.0;
                let scale = idle_scale(kind, t);
                assert!(
                    (scale - Vec2::ONE).length() < 0.09,
                    "{kind:?} swells too much"
                );
                if kind != ObjectKind::SporeFlower {
                    assert!(idle_spin(kind, t).abs() < 0.2, "{kind:?} turns too far");
                }
            }
        }
        // A ribbon's path must never be scaled or turned.
        assert!((idle_scale(ObjectKind::WindRibbon, 3.0) - Vec2::ONE).length() < 1e-6);
        assert!(idle_spin(ObjectKind::WindRibbon, 3.0).abs() < 1e-6);
    }

    /// The goal and its stardust always move, and always gently.
    #[test]
    fn the_beacon_breathes_and_the_stardust_twinkles() {
        let mut lowest = f32::MAX;
        let mut highest = f32::MIN;
        for step in 0..400_u16 {
            let t = f32::from(step) / 20.0;
            let breath = beacon_breath(t);
            assert!((0.85..=1.15).contains(&breath), "the beacon breathes hard");
            let twinkle = twinkle_scale(t);
            lowest = lowest.min(twinkle);
            highest = highest.max(twinkle);
        }
        assert!(highest - lowest > 0.2, "the stardust barely twinkles");
        assert!(highest < 1.2 && lowest > 0.8, "the stardust flickers");
    }

    /// The flow must always be visible (never scale to nothing) and must
    /// travel: two chevrons a wavelength apart are at the same point in it.
    #[test]
    fn the_wind_flows_along_the_ribbon() {
        for step in 0..200_u16 {
            let t = f32::from(step) / 10.0;
            assert!((0.3..=1.2).contains(&flow_scale(t, 0.0)));
        }
        let (a, b) = (flow_scale(2.0, 0.0), flow_scale(2.0, FLOW_WAVELENGTH));
        assert!((a - b).abs() < 1e-3, "the wave doesn't repeat");
        assert!(
            (flow_scale(0.0, 0.0) - flow_scale(0.4, 0.0)).abs() > 0.05,
            "the wave doesn't travel"
        );
    }
}
