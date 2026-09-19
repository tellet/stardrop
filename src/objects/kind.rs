//! What objects are: their types, how they're placed, and the tuning for
//! each type. Plain Rust, no ECS.

use bevy::math::Vec2;

use crate::collision::{CircleShape, Shape};

/// Every object type in the game. Fieldless: it names a *type*, never a
/// particular object. Used for events, the per-type tuning and (later) the
/// journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectKind {
    /// Dark, crackling cloud: touching it ends the attempt.
    StaticCloud,
}

/// One object as placed in a level, in world coordinates. An enum *with
/// data*: each type carries exactly the placement fields it needs, and
/// nothing that could change its behavior (the consistency rule).
#[derive(Debug, Clone, PartialEq)]
pub enum Placement {
    /// A static cloud: a circle.
    StaticCloud {
        /// Center of the cloud.
        center: Vec2,
        /// Visible radius.
        radius: f32,
    },
}

impl Placement {
    /// This object's type.
    #[must_use]
    pub fn kind(&self) -> ObjectKind {
        match self {
            Self::StaticCloud { .. } => ObjectKind::StaticCloud,
        }
    }

    /// Where the object is drawn from (its center).
    #[must_use]
    pub fn center(&self) -> Vec2 {
        match self {
            Self::StaticCloud { center, .. } => *center,
        }
    }

    /// The object's full visible outline, used for level validation.
    #[must_use]
    pub fn outline(&self) -> CircleShape {
        match self {
            Self::StaticCloud { center, radius } => CircleShape::new(*center, *radius),
        }
    }

    /// What the astronaut actually collides with. Hazards are smaller than
    /// they look, so near misses feel fair (see `ObjectsConfig`).
    #[must_use]
    pub fn hitbox(&self, config: &ObjectsConfig) -> CircleShape {
        match self {
            Self::StaticCloud { center, radius } => {
                CircleShape::new(*center, radius * config.static_cloud.hitbox_scale)
            }
        }
    }

    /// Whether a circle (the astronaut) touches this object's hitbox.
    #[must_use]
    pub fn touches(&self, config: &ObjectsConfig, center: Vec2, radius: f32) -> bool {
        self.hitbox(config).overlaps_circle(center, radius)
    }
}

/// Static cloud tuning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StaticCloudConfig {
    /// Hitbox radius as a fraction of the visible radius.
    pub hitbox_scale: f32,
}

/// Behavior tuning for every object type, defined once (in `config.rs`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectsConfig {
    /// Static cloud.
    pub static_cloud: StaticCloudConfig,
}
