//! Visual reactions. For now: objects squish briefly when touched, so the
//! player always sees *what* they touched. Placeholder for Milestone 7's
//! particles and flashes.

use bevy::prelude::*;

use crate::astronaut::{Astronaut, CaptureBubble, Shimmer};
use crate::objects::ObjectTouched;
use crate::physics::Motion;

/// How long a contact squish lasts, in seconds.
const PULSE_SECONDS: f32 = 0.35;
/// At its widest, the object is this much wider and this much shorter.
const PULSE_WIDEN: f32 = 0.12;
const PULSE_FLATTEN: f32 = 0.08;

/// A running squish, if any: seconds since it started.
#[derive(Component, Debug, Default, Clone, Copy)]
pub(crate) struct ContactPulse(Option<f32>);

/// Visual reactions to gameplay events.
pub(crate) struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                (start_pulses.run_if(on_message::<ObjectTouched>), run_pulses).chain(),
                (show_shimmer, show_capture_bubble),
            ),
        );
    }
}

/// Scale of a squish `elapsed` seconds in: a smooth bump (half a sine wave)
/// from 1 out to the widest point and back, never a snap.
#[must_use]
pub(crate) fn pulse_scale(elapsed: f32) -> Vec2 {
    let t = (elapsed / PULSE_SECONDS).clamp(0.0, 1.0);
    let bump = (t * core::f32::consts::PI).sin();
    Vec2::new(1.0 + PULSE_WIDEN * bump, 1.0 - PULSE_FLATTEN * bump)
}

fn start_pulses(mut touched: MessageReader<ObjectTouched>, mut pulses: Query<&mut ContactPulse>) {
    for event in touched.read() {
        if let Ok(mut pulse) = pulses.get_mut(event.entity) {
            pulse.0 = Some(0.0);
        }
    }
}

/// Advances running squishes; objects at rest aren't touched at all.
fn run_pulses(time: Res<Time>, mut pulses: Query<(&mut ContactPulse, &mut Transform)>) {
    for (mut pulse, mut transform) in &mut pulses {
        let Some(elapsed) = pulse.0 else {
            continue;
        };
        let elapsed = elapsed + time.delta_secs();
        pulse.0 = (elapsed < PULSE_SECONDS).then_some(elapsed);
        transform.scale = pulse_scale(elapsed).extend(1.0);
    }
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
}
