//! Sparks: the short-lived soft dots behind the astronaut, at every contact
//! and around a landing.
//!
//! They come from a pool of a fixed size (Section 7.2): the entities are
//! spawned once and reused forever, so no effect can ever grow the world or
//! allocate mid-frame. When more sparks are asked for than the pool holds,
//! the oldest ones make way.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::rng::SplitMix64;
use crate::settings::{Quality, Settings};
use crate::soft_mesh::unit_disc;

/// The hard cap on sparks alive at once.
const POOL: usize = 192;
/// Corners of a spark. They're small, so a rough circle is plenty.
const SEGMENTS: usize = 8;
/// Sparks slow down as they go, like dust in air.
const DRAG: f32 = 1.8;
/// Drawn in front of the playfield's scenery, behind the astronaut.
const SPARK_DEPTH: f32 = 4.0;
/// Fixed seed: an effect looks the same every time it happens.
const SPARK_SEED: u64 = 0x5350_4152_4B53_0001;

/// Ask for a puff of sparks. Anything that wants an effect writes one of
/// these rather than spawning entities of its own.
#[derive(Message, Debug, Clone, Copy)]
pub(crate) struct Burst {
    /// Where the sparks come from.
    pub at: Vec2,
    pub color: Color,
    /// How many to send out, before the quality setting thins them.
    pub count: usize,
    /// How fast they fly outwards, in world units per second.
    pub speed: f32,
    /// Seconds each spark lives.
    pub life: f32,
    /// Radius of a spark at birth.
    pub size: f32,
    /// Added to every spark's velocity: the motion they inherit.
    pub drift: Vec2,
}

impl Default for Burst {
    fn default() -> Self {
        Self {
            at: Vec2::ZERO,
            color: Color::WHITE,
            count: 8,
            speed: 120.0,
            life: 0.6,
            size: 6.0,
            drift: Vec2::ZERO,
        }
    }
}

/// One spark in flight. A resting spark has no life left and is hidden.
#[derive(Component, Debug, Default, Clone, Copy)]
struct Spark {
    velocity: Vec2,
    /// Seconds left.
    life: f32,
    /// Seconds it had at birth.
    span: f32,
    /// Radius at birth.
    size: f32,
}

/// The pool: which slot to use next, and the shared mesh and materials.
#[derive(Resource)]
struct SparkPool {
    slots: Vec<Entity>,
    next: usize,
    /// One material per colour asked for, made once and kept. Sparks only
    /// ever use the handful of colours the objects are painted in.
    materials: HashMap<[u8; 4], Handle<ColorMaterial>>,
    rng: SplitMix64,
}

/// Pooled sparks.
pub(crate) struct ParticlesPlugin;

impl Plugin for ParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Burst>()
            .add_systems(Startup, fill_pool)
            .add_systems(Update, (emit_bursts, advance_sparks).chain());
    }
}

/// How much of an effect this quality setting draws.
fn density(settings: &Settings) -> f32 {
    match settings.quality {
        Quality::Full => 1.0,
        Quality::Plain => 0.5,
    }
}

/// A spark's radius, `life` seconds before it dies out of a `span`-second
/// life: it swells as it appears, then shrinks away to nothing.
fn spark_scale(size: f32, life: f32, span: f32) -> f32 {
    if span <= 0.0 {
        return 0.0;
    }
    let left = (life / span).clamp(0.0, 1.0);
    let age = 1.0 - left;
    // Smoothstep out, with a quick swell over the first tenth of its life.
    let swell = (age / 0.1).clamp(0.0, 1.0);
    size * swell * left * left * (3.0 - 2.0 * left)
}

fn fill_pool(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mesh = meshes.add(unit_disc(SEGMENTS));
    let white = materials.add(ColorMaterial::from_color(Color::WHITE));
    let resting = white.clone();
    let slots = (0..POOL)
        .map(|_| {
            commands
                .spawn((
                    Spark::default(),
                    Mesh2d(mesh.clone()),
                    MeshMaterial2d(resting.clone()),
                    Transform::default(),
                    Visibility::Hidden,
                ))
                .id()
        })
        .collect();
    // The resting material goes straight into the cache, so it stays in use
    // however the sparks are recoloured: the pool creates and frees nothing
    // once it exists.
    let mut cached = HashMap::new();
    cached.insert(Color::WHITE.to_srgba().to_u8_array(), white);
    commands.insert_resource(SparkPool {
        slots,
        next: 0,
        materials: cached,
        rng: SplitMix64::new(SPARK_SEED),
    });
}

/// Turns each request into sparks, taking pool slots in turn.
fn emit_bursts(
    mut bursts: MessageReader<Burst>,
    settings: Res<Settings>,
    pool: Option<ResMut<SparkPool>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut sparks: Query<(
        &mut Spark,
        &mut Transform,
        &mut MeshMaterial2d<ColorMaterial>,
    )>,
    mut visibility: Query<&mut Visibility, With<Spark>>,
) {
    let Some(mut pool) = pool else {
        return;
    };
    let density = density(&settings);
    let calm = settings.motion_scale();
    for burst in bursts.read() {
        // Quality thins effects out rather than shortening them.
        #[expect(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "small counts"
        )]
        let count = ((burst.count as f32 * density).round() as usize).min(POOL);
        let material = spark_material(&mut pool, &mut materials, burst.color);
        for _ in 0..count {
            let slot = pool.slots[pool.next % pool.slots.len()];
            pool.next = pool.next.wrapping_add(1);
            let angle = pool.rng.range(0.0, core::f32::consts::TAU);
            let speed = pool.rng.range(0.35, 1.0) * burst.speed * calm;
            let life = pool.rng.range(0.7, 1.3) * burst.life;
            let size = pool.rng.range(0.7, 1.3) * burst.size;
            let Ok((mut spark, mut transform, mut slot_material)) = sparks.get_mut(slot) else {
                continue;
            };
            *spark = Spark {
                velocity: Vec2::new(angle.cos(), angle.sin()) * speed + burst.drift,
                life,
                span: life,
                size,
            };
            transform.translation = burst.at.extend(SPARK_DEPTH);
            transform.scale = Vec3::splat(spark_scale(size, life, life));
            if slot_material.0 != material {
                slot_material.0 = material.clone();
            }
            if let Ok(mut visible) = visibility.get_mut(slot) {
                visible.set_if_neq(Visibility::Inherited);
            }
        }
    }
}

/// The material for one colour, made the first time that colour is asked for.
fn spark_material(
    pool: &mut SparkPool,
    materials: &mut Assets<ColorMaterial>,
    color: Color,
) -> Handle<ColorMaterial> {
    let key = color.to_srgba().to_u8_array();
    pool.materials
        .entry(key)
        .or_insert_with(|| materials.add(ColorMaterial::from_color(color)))
        .clone()
}

/// Moves every living spark and shrinks it away. Hidden slots cost one
/// comparison each.
fn advance_sparks(
    time: Res<Time>,
    mut sparks: Query<(&mut Spark, &mut Transform, &mut Visibility)>,
) {
    let delta = time.delta_secs();
    for (mut spark, mut transform, mut visibility) in &mut sparks {
        if spark.life <= 0.0 {
            continue;
        }
        spark.life -= delta;
        if spark.life <= 0.0 {
            spark.life = 0.0;
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        }
        let velocity = spark.velocity;
        spark.velocity = velocity * (1.0 - DRAG * delta).max(0.0);
        transform.translation += (velocity * delta).extend(0.0);
        transform.scale = Vec3::splat(spark_scale(spark.size, spark.life, spark.span));
    }
}

#[cfg(test)]
mod tests {
    use bevy::asset::AssetPlugin;

    use super::*;

    #[test]
    fn a_spark_swells_then_shrinks_to_nothing() {
        let (size, span) = (6.0, 0.6);
        assert!(spark_scale(size, span, span) < size * 0.2, "born small");
        let quarter = spark_scale(size, span * 0.75, span);
        assert!(quarter > size * 0.5, "grown by a quarter of the way in");
        assert!(spark_scale(size, 0.0, span).abs() < 1e-6, "gone at the end");
        // Never bigger than it was asked to be.
        for step in 0..=100_u8 {
            let life = span * f32::from(step) / 100.0;
            assert!(spark_scale(size, life, span) <= size + 1e-4);
        }
    }

    /// The pool is the whole point: asking for more sparks than it holds must
    /// reuse slots, never spawn.
    #[test]
    fn asking_for_too_many_sparks_reuses_the_pool() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<ColorMaterial>()
            .init_resource::<Settings>()
            .add_plugins(ParticlesPlugin);
        app.update();
        let before = app.world().entities().count_spawned();
        for _ in 0..50 {
            app.world_mut().write_message(Burst {
                count: 64,
                ..default()
            });
            app.update();
        }
        assert_eq!(app.world().entities().count_spawned(), before);
        let world = app.world_mut();
        let alive = world
            .query::<&Spark>()
            .iter(world)
            .filter(|spark| spark.life > 0.0)
            .count();
        assert!(alive <= POOL, "{alive} sparks alive, pool holds {POOL}");
    }
}
