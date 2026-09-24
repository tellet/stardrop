//! What objects are: their types, how they're placed, their shapes, and the
//! tuning for each type. Plain Rust, no ECS.

use core::ops::RangeInclusive;

use bevy::math::Vec2;

use crate::collision::{CircleShape, PathShape, Shape};

/// Every object type in the game. Fieldless: it names a *type*, never a
/// particular object. Used for events, the per-type tuning and (later) the
/// journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectKind {
    /// Soft cloud: bounces the astronaut upward and away.
    CloudPuff,
    /// Glowing stream: carries the astronaut along its path.
    WindRibbon,
    /// Glassy diamond: reverses sideways motion.
    Crystal,
    /// Twin portals: entering one end sends the astronaut out of the other.
    TwinPortals,
    /// Gravity orb: pulls the astronaut toward its center within a field.
    GravityOrb,
    /// Floating bubble: catches the astronaut and carries it upward.
    Bubble,
    /// Mischievous flower: swaps left and right for a few seconds.
    SporeFlower,
    /// Heavy stone: throws the astronaut into a fast dive.
    CometStone,
    /// Dark, crackling cloud: touching it ends the attempt.
    StaticCloud,
}

impl ObjectKind {
    /// Every kind, in the order the journal lists them. Tests walk this, so
    /// a new kind can't be added without the look and feel to go with it.
    pub const ALL: [Self; 9] = [
        Self::CloudPuff,
        Self::WindRibbon,
        Self::Crystal,
        Self::TwinPortals,
        Self::GravityOrb,
        Self::Bubble,
        Self::SporeFlower,
        Self::CometStone,
        Self::StaticCloud,
    ];

    /// Allowed values for [`Placement::size`], so every object stays readable
    /// and touchable.
    #[must_use]
    pub fn size_range(self) -> RangeInclusive<f32> {
        match self {
            Self::CloudPuff
            | Self::TwinPortals
            | Self::GravityOrb
            | Self::Bubble
            | Self::Crystal
            | Self::SporeFlower
            | Self::CometStone
            | Self::StaticCloud => 20.0..=300.0,
            // Ribbon width.
            Self::WindRibbon => 30.0..=240.0,
        }
    }
}

/// Most points a wind ribbon's path may have.
pub const MAX_RIBBON_POINTS: usize = 16;

/// One object as placed in a level, in world coordinates. An enum *with
/// data*: each type carries exactly the placement fields it needs, and
/// nothing that could change its behavior (the consistency rule).
#[derive(Debug, Clone, PartialEq)]
pub enum Placement {
    /// A cloud puff: a circle.
    CloudPuff {
        /// Center of the cloud.
        center: Vec2,
        /// Visible radius.
        radius: f32,
    },
    /// A wind ribbon: a thick path the wind flows along, first point to last.
    WindRibbon {
        /// The centerline, in flow order.
        path: Vec<Vec2>,
        /// Full width of the ribbon.
        width: f32,
    },
    /// A pair of portals: two circles of the same size, always paired by
    /// construction (a level file can't write an unpaired portal).
    TwinPortals {
        /// One end.
        a: Vec2,
        /// The other end.
        b: Vec2,
        /// Radius of both ends.
        radius: f32,
    },
    /// A gravity orb: a core circle, with a pull field around it.
    GravityOrb {
        /// Center of the orb.
        center: Vec2,
        /// Radius of the visible core.
        radius: f32,
    },
    /// A bubble: a circle.
    Bubble {
        /// Center of the bubble.
        center: Vec2,
        /// Visible radius.
        radius: f32,
    },
    /// A spore flower: a circle.
    SporeFlower {
        /// Center of the flower.
        center: Vec2,
        /// Visible radius.
        radius: f32,
    },
    /// A comet stone: a circle.
    CometStone {
        /// Center of the stone.
        center: Vec2,
        /// Visible radius.
        radius: f32,
    },
    /// A crystal: a diamond, `size` from its center to each tip.
    Crystal {
        /// Center of the diamond.
        center: Vec2,
        /// Distance from the center to a tip.
        size: f32,
    },
    /// A static cloud: a circle.
    StaticCloud {
        /// Center of the cloud.
        center: Vec2,
        /// Visible radius.
        radius: f32,
    },
}

/// Any object's collision shape. Implements [`Shape`] by asking whichever
/// shape it holds; `'a` is the level data a path borrows from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ObjectShape<'a> {
    /// Clouds and crystals.
    Circle(CircleShape),
    /// Wind ribbons.
    Path(PathShape<'a>),
    /// Twin portals: either end counts.
    Pair([CircleShape; 2]),
}

impl Shape for ObjectShape<'_> {
    fn overlaps_circle(&self, center: Vec2, radius: f32) -> bool {
        match self {
            Self::Circle(circle) => circle.overlaps_circle(center, radius),
            Self::Path(path) => path.overlaps_circle(center, radius),
            Self::Pair(ends) => ends.iter().any(|end| end.overlaps_circle(center, radius)),
        }
    }
}

impl Placement {
    /// This object's type.
    #[must_use]
    pub fn kind(&self) -> ObjectKind {
        match self {
            Self::CloudPuff { .. } => ObjectKind::CloudPuff,
            Self::TwinPortals { .. } => ObjectKind::TwinPortals,
            Self::GravityOrb { .. } => ObjectKind::GravityOrb,
            Self::Bubble { .. } => ObjectKind::Bubble,
            Self::SporeFlower { .. } => ObjectKind::SporeFlower,
            Self::CometStone { .. } => ObjectKind::CometStone,
            Self::WindRibbon { .. } => ObjectKind::WindRibbon,
            Self::Crystal { .. } => ObjectKind::Crystal,
            Self::StaticCloud { .. } => ObjectKind::StaticCloud,
        }
    }

    /// The object's middle: where it's drawn from and what its squish
    /// centers on. For a ribbon, the average of its points.
    #[must_use]
    pub fn center(&self) -> Vec2 {
        match self {
            Self::CloudPuff { center, .. }
            | Self::GravityOrb { center, .. }
            | Self::Bubble { center, .. }
            | Self::Crystal { center, .. }
            | Self::SporeFlower { center, .. }
            | Self::CometStone { center, .. }
            | Self::StaticCloud { center, .. } => *center,
            // Halfway between the ends: what the pair's squish centers on.
            Self::TwinPortals { a, b, .. } => a.midpoint(*b),
            Self::WindRibbon { path, .. } => {
                // A ribbon has at most MAX_RIBBON_POINTS points, so this
                // conversion is exact; `max(1)` avoids dividing by zero.
                #[expect(clippy::cast_precision_loss, reason = "at most 16 points")]
                let count = path.len().max(1) as f32;
                path.iter().copied().sum::<Vec2>() / count
            }
        }
    }

    /// The number a level sets for this object's size: radius, tip distance
    /// or ribbon width. Checked against [`ObjectKind::size_range`].
    #[must_use]
    pub fn size(&self) -> f32 {
        match self {
            Self::CloudPuff { radius, .. }
            | Self::GravityOrb { radius, .. }
            | Self::Bubble { radius, .. }
            | Self::SporeFlower { radius, .. }
            | Self::CometStone { radius, .. }
            | Self::StaticCloud { radius, .. }
            | Self::TwinPortals { radius, .. } => *radius,
            Self::Crystal { size, .. } => *size,
            Self::WindRibbon { width, .. } => *width,
        }
    }

    /// The object's full visible outline, used for level validation.
    #[must_use]
    pub fn outline(&self) -> ObjectShape<'_> {
        match self {
            Self::CloudPuff { center, radius }
            | Self::GravityOrb { center, radius }
            | Self::Bubble { center, radius }
            | Self::SporeFlower { center, radius }
            | Self::CometStone { center, radius }
            | Self::StaticCloud { center, radius } => {
                ObjectShape::Circle(CircleShape::new(*center, *radius))
            }
            // The circle through the diamond's tips: generous, like every
            // helpful or mixed object.
            Self::Crystal { center, size } => ObjectShape::Circle(CircleShape::new(*center, *size)),
            Self::TwinPortals { a, b, radius } => {
                ObjectShape::Pair([CircleShape::new(*a, *radius), CircleShape::new(*b, *radius)])
            }
            Self::WindRibbon { path, width } => ObjectShape::Path(PathShape {
                points: path,
                half_width: width / 2.0,
            }),
        }
    }

    /// What the astronaut actually collides with. Hazards are smaller than
    /// they look, so near misses feel fair (see `ObjectsConfig`); everything
    /// else uses its full outline.
    #[must_use]
    pub fn hitbox(&self, config: &ObjectsConfig) -> ObjectShape<'_> {
        match self {
            Self::CloudPuff { .. }
            | Self::Bubble { .. }
            | Self::Crystal { .. }
            | Self::TwinPortals { .. }
            | Self::SporeFlower { .. }
            | Self::CometStone { .. }
            | Self::WindRibbon { .. } => self.outline(),
            // An orb acts throughout its field, which is larger than its core.
            Self::GravityOrb { center, radius } => ObjectShape::Circle(CircleShape::new(
                *center,
                radius * config.gravity_orb.field_scale,
            )),
            Self::StaticCloud { center, radius } => ObjectShape::Circle(CircleShape::new(
                *center,
                radius * config.static_cloud.hitbox_scale,
            )),
        }
    }

    /// Whether a circle (the astronaut) touches this object's hitbox.
    #[must_use]
    pub fn touches(&self, config: &ObjectsConfig, center: Vec2, radius: f32) -> bool {
        self.hitbox(config).overlaps_circle(center, radius)
    }
}

/// Cloud puff tuning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CloudPuffConfig {
    /// Speed of the bounce, in units/s.
    pub bounce_speed: f32,
    /// The bounce always points at least this close to straight up, in
    /// radians: "upward and away", never sideways or down.
    pub max_tilt: f32,
}

/// Wind ribbon tuning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindRibbonConfig {
    /// Speed the wind carries the astronaut at, along the path, in units/s.
    pub flow_speed: f32,
    /// How strongly the wind pulls the astronaut's velocity toward the flow,
    /// in units/s². Stronger than gravity, so ribbons can carry you upward.
    pub pull: f32,
}

/// Twin portal tuning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TwinPortalsConfig {
    /// How far outside the exit the astronaut appears, beyond the portal's
    /// radius, in world units.
    pub exit_clearance: f32,
    /// Seconds before portals accept the astronaut again.
    pub cooldown: f32,
}

/// Gravity orb tuning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GravityOrbConfig {
    /// The pull field's radius, as a multiple of the core radius.
    pub field_scale: f32,
    /// Pull strength: the acceleration toward the center is this divided by
    /// the distance, so orbits at any radius feel similar.
    pub pull: f32,
    /// Largest pull acceleration, in units/s² (close to the core).
    pub max_pull: f32,
    /// Speed limit inside the field, in units/s: the brief forbids extreme speeds.
    pub max_speed: f32,
    /// Speed of the soft bounce when touching the core, in units/s.
    pub core_bounce_speed: f32,
}

/// Bubble tuning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BubbleConfig {
    /// How fast the bubble floats upward, in units/s.
    pub rise_speed: f32,
    /// How long it carries the astronaut before popping, in seconds.
    pub seconds: f32,
    /// Fastest sideways nudge inside the bubble, in units/s.
    pub nudge_speed: f32,
}

/// Spore flower tuning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SporeFlowerConfig {
    /// How long left and right stay swapped, in seconds.
    pub swap_seconds: f32,
}

/// Comet stone tuning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CometStoneConfig {
    /// Dive speed, in units/s (well beyond terminal velocity).
    pub dive_speed: f32,
    /// How long the dive speed is held, in seconds, before easing back.
    pub dive_seconds: f32,
}

/// Crystal tuning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrystalConfig {
    /// After reversing, sideways speed is at least this, in units/s, so the
    /// effect is always noticeable.
    pub min_sideways_speed: f32,
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
    /// Cloud puff.
    pub cloud_puff: CloudPuffConfig,
    /// Wind ribbon.
    pub wind_ribbon: WindRibbonConfig,
    /// Crystal.
    pub crystal: CrystalConfig,
    /// Twin portals.
    pub twin_portals: TwinPortalsConfig,
    /// Gravity orb.
    pub gravity_orb: GravityOrbConfig,
    /// Bubble.
    pub bubble: BubbleConfig,
    /// Spore flower.
    pub spore_flower: SporeFlowerConfig,
    /// Comet stone.
    pub comet_stone: CometStoneConfig,
    /// Static cloud.
    pub static_cloud: StaticCloudConfig,
}
