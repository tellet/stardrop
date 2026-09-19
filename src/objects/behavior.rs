//! What touching an object does. Pure functions of the object type's tuning
//! and the astronaut's state, so every object of a type behaves identically
//! (the consistency rule) and each effect is unit-tested.

use super::kind::{ObjectKind, ObjectsConfig};
use crate::physics::movement::Body;

/// The result of touching an object.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outcome {
    /// The attempt ends (hazard).
    FailAttempt,
}

/// What entering contact with an object of `kind` does to `body`.
///
/// One exhaustive `match`: adding an object type makes the compiler point
/// here until its behavior is written.
#[must_use]
pub fn on_enter(kind: ObjectKind, _config: &ObjectsConfig, _body: Body) -> Outcome {
    match kind {
        ObjectKind::StaticCloud => Outcome::FailAttempt,
    }
}

#[cfg(test)]
mod tests {
    use bevy::math::Vec2;

    use super::*;
    use crate::objects::kind::StaticCloudConfig;

    const CONFIG: ObjectsConfig = ObjectsConfig {
        static_cloud: StaticCloudConfig { hitbox_scale: 0.8 },
    };

    #[test]
    fn static_cloud_always_ends_the_attempt() {
        for velocity in [
            Vec2::ZERO,
            Vec2::new(500.0, -450.0),
            Vec2::new(-300.0, 200.0),
        ] {
            let body = Body {
                velocity,
                ..Body::default()
            };
            assert_eq!(
                on_enter(ObjectKind::StaticCloud, &CONFIG, body),
                Outcome::FailAttempt
            );
        }
    }
}
