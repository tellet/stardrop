//! The cartoon that plays when every level is done.
//!
//! An 8.5-second scene with nothing written on it: the astronaut comes down
//! on a planet nobody has seen before, the dust settles, and they plant a
//! flag. The beats below are the whole script, and every motion in it is a
//! pure function of how far through we are, so the systems only apply what
//! these decide.

use bevy::prelude::*;

use crate::app_state::AppState;
use crate::config::GameConfig;
use crate::particles::Burst;
use crate::settings::Settings;
use crate::ui::StarArt;
use art::{
    FLAG_TOP, cloth_mesh, color, flag_mesh, ground_mesh, pack_mesh, prop_mesh, sky_mesh, water_mesh,
};
use planet::{Planet, PropKind, SURFACE_Y};

mod art;
mod planet;

/// When the astronaut touches the ground.
const TOUCHDOWN: f32 = 2.2;
/// When the dust has settled and the pack comes off.
const SETTLED: f32 = 3.6;
/// When the flag starts rising out of the pack.
const FLAG: f32 = 5.0;
/// When they lean in and push it into the ground.
const PLANT: f32 = 6.4;
/// When they stand back and look at it.
const HOLD: f32 = 7.4;
/// The whole cartoon, in seconds.
const TOTAL: f32 = 8.5;
// The brief asks for five to ten seconds (Section 11), checked when compiling.
const _: () = assert!(TOTAL >= 5.0 && TOTAL <= 10.0, "the cartoon runs 5-10 s");
// The last beat needs time on screen to be a beat at all.
const _: () = assert!(HOLD < TOTAL);

/// Where the astronaut enters, in world y: above the top of the frame.
const ENTRY_Y: f32 = 120.0;
/// How far they drift sideways on the way down, in world units.
const SWAY: f32 = 90.0;
/// How long the landing squash lasts, in seconds.
const SQUASH_SECONDS: f32 = 0.35;
/// How much they give on impact, as a fraction of their height.
const SQUASH_DEPTH: f32 = 0.22;
/// How far the pack overshoots its size before settling into it.
const PACK_OVERSHOOT: f32 = 0.06;
/// How fast the planted flag waves, in radians per second.
const WAVE_SPEED: f32 = 3.4;
/// How far it swings, in radians, and how much of that survives when the
/// player has asked for less motion.
const WAVE_SWING: f32 = 0.22;
const WAVE_CALM: f32 = 0.35;

/// One beat of the script. Each one is announced once, when it begins, which
/// is what `audio.rs` listens for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Beat {
    /// Coming down.
    Descent,
    /// The moment of landing, and the dust dying away after it.
    Settle,
    /// The pack comes off, absurdly large.
    Pack,
    /// A very small flag rises out of it.
    Flag,
    /// It goes into the ground and unfurls.
    Plant,
    /// Standing beside it.
    Hold,
}

// --- The shapes of the motion ----------------------------------------------

/// Which beat is playing `t` seconds in.
#[must_use]
fn beat_at(t: f32) -> Beat {
    if t < TOUCHDOWN {
        Beat::Descent
    } else if t < SETTLED {
        Beat::Settle
    } else if t < FLAG {
        Beat::Pack
    } else if t < PLANT {
        Beat::Flag
    } else if t < HOLD {
        Beat::Plant
    } else {
        Beat::Hold
    }
}

/// How far through a stretch of the script we are, 0 to 1.
#[must_use]
fn through(t: f32, from: f32, to: f32) -> f32 {
    ((t - from) / (to - from)).clamp(0.0, 1.0)
}

/// The astronaut's height: down from above the frame to their feet on the
/// ground at `feet`, slowing into the landing rather than speeding up. A fall
/// this is not; they are bringing themselves down.
#[must_use]
fn descent_y(t: f32, feet: f32) -> f32 {
    let u = through(t, 0.0, TOUCHDOWN);
    let eased = 1.0 - (1.0 - u) * (1.0 - u);
    ENTRY_Y + (feet - ENTRY_Y) * eased
}

/// Their sideways drift: a slow sway on the way down that is spent by the
/// time they arrive, so they land where they are going to stand.
#[must_use]
fn descent_x(t: f32, landing_x: f32) -> f32 {
    let u = through(t, 0.0, TOUCHDOWN);
    landing_x + SWAY * (1.0 - u) * (u * 6.0).sin()
}

/// The landing squash: they give on impact and spread sideways, keeping their
/// area, and it is over well before the dust is.
#[must_use]
fn squash(t: f32) -> Vec2 {
    let u = through(t, TOUCHDOWN, TOUCHDOWN + SQUASH_SECONDS);
    if u <= 0.0 || u >= 1.0 {
        return Vec2::ONE;
    }
    // Deepest at the moment of contact, easing back out.
    let give = SQUASH_DEPTH * (1.0 - u) * (1.0 - u);
    let y = 1.0 - give;
    Vec2::new(1.0 / y, y)
}

/// How big the pack is: nothing until its beat, then eased up to full size
/// with a little overshoot, because something that heavy should land.
#[must_use]
fn pack_scale(t: f32) -> f32 {
    let u = through(t, SETTLED, FLAG);
    if u <= 0.0 {
        return 0.0;
    }
    let eased = 1.0 - (1.0 - u) * (1.0 - u) * (1.0 - u);
    // The overshoot peaks four fifths of the way through and is gone by the end.
    let bump = PACK_OVERSHOOT * (u * core::f32::consts::PI).sin() * (1.0 - u);
    eased + bump
}

/// How far the flag has risen out of the pack, 0 to 1.
#[must_use]
fn flag_rise(t: f32) -> f32 {
    let u = through(t, FLAG, PLANT);
    1.0 - (1.0 - u) * (1.0 - u)
}

/// How far the flag climbs out of the pack, in world units.
const FLAG_LIFT: f32 = 62.0;
/// How long it takes to carry the flag across and push it in, in seconds.
const CARRY_SECONDS: f32 = 0.5;

/// Where the flag is: climbing out of the pack while it rises, then carried
/// across to where it is planted and pushed back down into the ground.
#[must_use]
fn flag_at(t: f32, pack_x: f32, plant_x: f32, ground_y: f32) -> Vec2 {
    let lifted = ground_y + FLAG_LIFT * flag_rise(t);
    let across = through(t, PLANT, PLANT + CARRY_SECONDS);
    let eased = across * across * (3.0 - 2.0 * across);
    Vec2::new(
        pack_x + (plant_x - pack_x) * eased,
        lifted + (ground_y - lifted) * eased,
    )
}

/// How far the planted cloth is swung, in radians. `motion` is the player's
/// motion setting: a calmer wave, never a still one.
#[must_use]
fn flag_wave(t: f32, motion: f32) -> f32 {
    if t < PLANT {
        return 0.0;
    }
    let swing = WAVE_SWING * (WAVE_CALM + (1.0 - WAVE_CALM) * motion.clamp(0.0, 1.0));
    (t * WAVE_SPEED).sin() * swing
}

// --- Where it all sits ------------------------------------------------------

/// Drawn in front of the game's own sky, veil and edge panels
/// (`background.rs`), so the cartoon covers the window rather than sharing it.
const BACKDROP_DEPTH: f32 = -520.0;
const SKY_DEPTH: f32 = -510.0;
const GROUND_DEPTH: f32 = -300.0;
const WATER_DEPTH: f32 = -290.0;
const PROP_DEPTH: f32 = -200.0;
const PACK_DEPTH: f32 = 8.0;
const FLAG_DEPTH: f32 = 9.0;

/// How far the flat backdrop reaches: more than any window can show.
const BACKDROP: f32 = 12_000.0;
/// The sky gradient, from above the frame down past the surface.
const SKY_TOP: f32 = 450.0;
const SKY_WIDTH: f32 = 6_000.0;

/// How often a spark comes off the astronaut on the way down, in seconds.
const TRAIL_EVERY: f32 = 0.12;
/// How often a steam vent breathes, in seconds.
const STEAM_EVERY: f32 = 0.9;
/// How far props lean when they sway, in radians.
const SWAY_LEAN: f32 = 0.05;
/// And how fast.
const SWAY_SPEED: f32 = 0.7;

/// Sent once as each beat of the script begins. `audio.rs` listens.
#[derive(Message, Debug, Clone, Copy)]
pub(crate) struct EndingBeat(pub Beat);

/// Everything the cartoon spawns, so leaving can take all of it.
#[derive(Component)]
struct EndingEntity;

/// The astronaut in the cartoon. Deliberately not the game's `Astronaut`:
/// that one is hidden whenever no level is loaded.
#[derive(Component)]
struct Figure;

/// The pack, which scales up out of nothing.
#[derive(Component)]
struct Pack;

/// The flag, which rises out of the pack and then goes into the ground.
#[derive(Component)]
struct Flag;

/// The cloth on it, which waves.
#[derive(Component)]
struct Cloth;

/// A prop that leans in the wind.
#[derive(Component)]
struct Swaying {
    /// Where in the sway it starts, so no two move together.
    phase: f32,
}

/// A crack in the ground that breathes steam.
#[derive(Component)]
struct Vent;

/// The cartoon being watched.
#[derive(Resource)]
struct Showing {
    /// Seconds since it started.
    elapsed: f32,
    /// The beat on screen now.
    beat: Beat,
    planet: Planet,
    /// World y of the astronaut's middle once they are standing.
    feet: f32,
    /// Where the pack sits, and where the flag ends up.
    pack_x: f32,
    plant_x: f32,
    /// The ground under the landing spot.
    ground_y: f32,
    /// Set once the cartoon has asked to leave, so it only asks once.
    leaving: bool,
    /// Seconds until the next spark comes off the suit.
    trail: f32,
    /// Whether the pack has landed yet, which is worth one puff of dust.
    thumped: bool,
}

/// The cartoon after the last level.
pub(crate) struct EndingPlugin;

impl Plugin for EndingPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<EndingBeat>()
            .add_systems(OnEnter(AppState::Ending), open_the_scene)
            .add_systems(OnExit(AppState::Ending), close_the_scene)
            .add_systems(
                Update,
                (
                    // The clock first: everything else reads where we are.
                    advance,
                    skip,
                    bring_them_down,
                    settle_the_dust,
                    sway_the_props,
                    breathe_the_vents,
                    unpack,
                    raise_the_flag,
                )
                    .chain()
                    .run_if(in_state(AppState::Ending)),
            );
    }
}

// --- Building the scene -----------------------------------------------------

/// A seed nobody can predict: every watch is a planet nobody has seen.
fn fresh_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0x5354_4152_4452_4F50, |since| since.subsec_nanos().into())
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Bevy systems take their resources by value"
)]
fn open_the_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    config: Res<GameConfig>,
    star: Option<Res<StarArt>>,
) {
    let planet = Planet::generate(fresh_seed());
    let palette = planet.palette;
    // One material for the lot: every colour is in the vertices.
    let tint = materials.add(ColorMaterial::from_color(Color::WHITE));
    let flat = meshes.add(Rectangle::new(1.0, 1.0));

    // Whatever the window shows beyond the scene is the colour of its sky.
    commands.spawn((
        EndingEntity,
        Mesh2d(flat.clone()),
        MeshMaterial2d(materials.add(color(palette.sky[0]))),
        Transform::from_xyz(0.0, 0.0, BACKDROP_DEPTH)
            .with_scale(Vec3::new(BACKDROP, BACKDROP, 1.0)),
    ));
    let sky_bottom = SURFACE_Y - 100.0;
    commands.spawn((
        EndingEntity,
        Mesh2d(meshes.add(sky_mesh(&palette))),
        MeshMaterial2d(tint.clone()),
        Transform::from_xyz(0.0, (SKY_TOP + sky_bottom) / 2.0, SKY_DEPTH).with_scale(Vec3::new(
            SKY_WIDTH,
            SKY_TOP - sky_bottom,
            1.0,
        )),
    ));
    commands.spawn((
        EndingEntity,
        Mesh2d(meshes.add(ground_mesh(&planet))),
        MeshMaterial2d(tint.clone()),
        Transform::from_xyz(0.0, 0.0, GROUND_DEPTH),
    ));
    if let Some(water) = water_mesh(&planet) {
        commands.spawn((
            EndingEntity,
            Mesh2d(meshes.add(water)),
            MeshMaterial2d(tint.clone()),
            Transform::from_xyz(0.0, 0.0, WATER_DEPTH),
        ));
    }
    for (index, prop) in planet.props.iter().enumerate() {
        #[expect(clippy::cast_precision_loss, reason = "at most 24 props")]
        let phase = index as f32 * 1.3;
        let mut entity = commands.spawn((
            EndingEntity,
            Mesh2d(meshes.add(prop_mesh(prop, &palette))),
            MeshMaterial2d(tint.clone()),
            Transform::from_translation(prop.at.extend(PROP_DEPTH)),
        ));
        // A vent is a hole in the ground: it breathes instead of leaning.
        if prop.kind == PropKind::SteamVent {
            entity.insert(Vent);
        } else {
            entity.insert(Swaying { phase });
        }
    }

    // The pack, at nothing until its beat, beside where they will stand.
    let landing_x = planet.landing_x;
    let ground = planet.height_at(landing_x);
    let radius = config.astronaut_radius;
    let pack_x = landing_x + radius * 2.6;
    commands.spawn((
        EndingEntity,
        Pack,
        Mesh2d(meshes.add(pack_mesh(&palette))),
        MeshMaterial2d(tint.clone()),
        Transform::from_xyz(pack_x, ground, PACK_DEPTH).with_scale(Vec3::splat(0.0)),
    ));
    // The flag, still inside it.
    let flag = commands
        .spawn((
            EndingEntity,
            Flag,
            Mesh2d(meshes.add(flag_mesh(&palette))),
            MeshMaterial2d(tint.clone()),
            Transform::from_xyz(pack_x, ground, FLAG_DEPTH),
            Visibility::Hidden,
        ))
        .id();
    let cloth = commands
        .spawn((
            Cloth,
            Mesh2d(meshes.add(cloth_mesh(&palette))),
            MeshMaterial2d(tint),
            Transform::from_xyz(0.0, FLAG_TOP, 0.1),
        ))
        .id();
    commands.entity(flag).add_child(cloth);
    // The game's own star, on the cloth. Without the UI plugin there is no
    // star to hang, and the flag is simply plain.
    if let Some(star) = star {
        let mark = commands
            .spawn((
                Sprite {
                    image: star.0.clone(),
                    custom_size: Some(Vec2::splat(radius * 0.5)),
                    color: color(palette.sky[0]),
                    ..default()
                },
                Transform::from_xyz(radius * 0.5, 0.0, 0.1),
            ))
            .id();
        commands.entity(cloth).add_child(mark);
    }

    let feet = ground + radius;
    commands.insert_resource(Showing {
        elapsed: 0.0,
        beat: Beat::Descent,
        planet,
        feet,
        pack_x,
        plant_x: landing_x + radius * 1.4,
        ground_y: ground,
        leaving: false,
        trail: 0.0,
        thumped: false,
    });
    // Spawn the astronaut last: `spawn_figure` takes the asset stores.
    crate::astronaut::spawn_figure(
        &mut commands,
        meshes,
        materials,
        radius,
        Vec2::new(descent_x(0.0, landing_x), descent_y(0.0, feet)),
        (EndingEntity, Figure),
    );
}

fn close_the_scene(mut commands: Commands, scene: Query<Entity, With<EndingEntity>>) {
    for entity in &scene {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<Showing>();
}

// --- Watching it ------------------------------------------------------------

/// The clock, and the beats it passes through.
fn advance(
    time: Res<Time>,
    mut showing: ResMut<Showing>,
    mut beats: MessageWriter<EndingBeat>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    showing.elapsed += time.delta_secs();
    let beat = beat_at(showing.elapsed);
    if beat != showing.beat {
        showing.beat = beat;
        beats.write(EndingBeat(beat));
    }
    if showing.elapsed >= TOTAL && !showing.leaving {
        showing.leaving = true;
        next_state.set(AppState::MainMenu);
    }
}

/// Any key, button or stick press ends it early.
fn skip(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    gamepads: Query<&Gamepad>,
    mut showing: ResMut<Showing>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if showing.leaving {
        return;
    }
    let asked = keys.get_just_pressed().next().is_some()
        || mouse.get_just_pressed().next().is_some()
        || gamepads
            .iter()
            .any(|pad| pad.get_just_pressed().next().is_some());
    if asked {
        showing.leaving = true;
        next_state.set(AppState::MainMenu);
    }
}

/// The descent, the landing squash, and the sparks that come off on the way.
fn bring_them_down(
    time: Res<Time>,
    mut showing: ResMut<Showing>,
    mut figure: Query<&mut Transform, With<Figure>>,
    mut bursts: MessageWriter<Burst>,
) {
    let landing_x = showing.planet.landing_x;
    let feet = showing.feet;
    let elapsed = showing.elapsed;
    let at = Vec2::new(descent_x(elapsed, landing_x), descent_y(elapsed, feet));
    for mut transform in &mut figure {
        transform.translation = at.extend(transform.translation.z);
        let squash = squash(elapsed);
        transform.scale = Vec3::new(squash.x, squash.y, 1.0);
    }
    // A trail of sparks while they are still coming down.
    if elapsed < TOUCHDOWN {
        showing.trail -= time.delta_secs();
        if showing.trail <= 0.0 {
            showing.trail = TRAIL_EVERY;
            bursts.write(Burst {
                at,
                color: color(showing.planet.palette.accent),
                count: 3,
                speed: 30.0,
                life: 0.7,
                size: 4.0,
                drift: Vec2::new(0.0, 40.0),
            });
        }
    }
}

/// The dust of the landing, and the puffs the pack and the flagpole make.
fn settle_the_dust(
    mut showing: ResMut<Showing>,
    mut beats: MessageReader<EndingBeat>,
    mut bursts: MessageWriter<Burst>,
) {
    let ground = Vec2::new(showing.planet.landing_x, showing.feet - 20.0);
    let dust = color(showing.planet.palette.ground);
    for EndingBeat(beat) in beats.read() {
        match beat {
            // The landing itself.
            Beat::Settle => {
                bursts.write(Burst {
                    at: ground,
                    color: dust,
                    count: 28,
                    speed: 260.0,
                    life: 1.1,
                    size: 9.0,
                    drift: Vec2::new(0.0, 20.0),
                });
            }
            // The pole going in.
            Beat::Plant => {
                bursts.write(Burst {
                    at: ground + Vec2::new(60.0, 0.0),
                    color: dust,
                    count: 10,
                    speed: 90.0,
                    life: 0.7,
                    size: 6.0,
                    drift: Vec2::ZERO,
                });
            }
            // Standing back from it.
            Beat::Hold => {
                bursts.write(Burst {
                    at: ground + Vec2::new(60.0, 70.0),
                    color: color(showing.planet.palette.accent),
                    count: 8,
                    speed: 60.0,
                    life: 1.2,
                    size: 5.0,
                    drift: Vec2::new(0.0, 30.0),
                });
            }
            Beat::Descent | Beat::Pack | Beat::Flag => {}
        }
    }
    // The pack is heavy, and the ground says so when it lands.
    if !showing.thumped && pack_scale(showing.elapsed) > 0.95 {
        showing.thumped = true;
        bursts.write(Burst {
            at: ground + Vec2::new(70.0, 0.0),
            color: dust,
            count: 16,
            speed: 140.0,
            life: 0.9,
            size: 8.0,
            drift: Vec2::ZERO,
        });
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Bevy systems take their resources by value"
)]
fn sway_the_props(
    time: Res<Time>,
    settings: Res<Settings>,
    mut props: Query<(&Swaying, &mut Transform)>,
) {
    let motion = settings.motion_scale();
    let now = time.elapsed_secs();
    for (swaying, mut transform) in &mut props {
        let lean = SWAY_LEAN * motion * (now * SWAY_SPEED + swaying.phase).sin();
        transform.rotation = Quat::from_rotation_z(lean);
    }
}

/// The vents breathe all the way through, and harder when the dust is up.
fn breathe_the_vents(
    time: Res<Time>,
    showing: Res<Showing>,
    mut vents: Query<(&Transform, &mut Vent)>,
    mut timer: Local<f32>,
    mut bursts: MessageWriter<Burst>,
) {
    *timer -= time.delta_secs();
    if *timer > 0.0 {
        return;
    }
    *timer = STEAM_EVERY;
    let hard = showing.beat == Beat::Settle;
    let steam = color(showing.planet.palette.accent);
    for (transform, _) in &mut vents {
        bursts.write(Burst {
            at: transform.translation.truncate(),
            color: steam,
            count: if hard { 10 } else { 4 },
            speed: if hard { 120.0 } else { 40.0 },
            life: 1.6,
            size: 10.0,
            drift: Vec2::new(0.0, 60.0),
        });
    }
}

fn unpack(showing: Res<Showing>, mut pack: Query<&mut Transform, With<Pack>>) {
    let scale = pack_scale(showing.elapsed);
    for mut transform in &mut pack {
        transform.scale = Vec3::new(scale, scale, 1.0);
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Bevy systems take their resources by value"
)]
fn raise_the_flag(
    time: Res<Time>,
    settings: Res<Settings>,
    showing: Res<Showing>,
    mut flag: Query<(&mut Transform, &mut Visibility), With<Flag>>,
    mut cloth: Query<&mut Transform, (With<Cloth>, Without<Flag>)>,
) {
    let risen = flag_rise(showing.elapsed);
    let at = flag_at(
        showing.elapsed,
        showing.pack_x,
        showing.plant_x,
        showing.ground_y,
    );
    for (mut transform, mut visibility) in &mut flag {
        visibility.set_if_neq(if risen > 0.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        transform.translation = at.extend(transform.translation.z);
    }
    let wave = flag_wave(time.elapsed_secs(), settings.motion_scale());
    for mut transform in &mut cloth {
        transform.rotation = Quat::from_rotation_z(wave);
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use bevy::asset::AssetPlugin;
    use bevy::state::app::StatesPlugin;
    use bevy::time::TimeUpdateStrategy;

    use super::*;
    use crate::app_state::AppState;
    use crate::level::ActiveLevel;
    use crate::particles::ParticlesPlugin;

    /// A frame of the cartoon, in the test's own time.
    const FRAME: Duration = Duration::from_millis(16);

    /// How many of those frames cover `seconds` of the script.
    fn frames(seconds: f32) -> usize {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a positive handful of seconds"
        )]
        let count = (seconds / FRAME.as_secs_f32()).ceil() as usize;
        count
    }

    fn cartoon_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<ColorMaterial>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<Settings>()
            .init_resource::<GameConfig>()
            .insert_resource(TimeUpdateStrategy::ManualDuration(FRAME))
            .init_state::<AppState>()
            .add_plugins((ParticlesPlugin, EndingPlugin));
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Ending);
        app.update();
        app
    }

    fn scene_size(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<(), With<EndingEntity>>()
            .iter(app.world())
            .count()
    }

    fn state(app: &App) -> AppState {
        *app.world().resource::<State<AppState>>().get()
    }

    /// The cartoon is watched from the main menu as well, where no level is
    /// loaded, so it must never reach for one.
    #[test]
    fn the_flag_climbs_out_then_goes_into_the_ground() {
        let pack = 100.0;
        let plant = 200.0;
        let ground = -1400.0;
        // It starts inside the pack, on the ground.
        let start = flag_at(FLAG, pack, plant, ground);
        assert!((start.x - pack).abs() < 0.5);
        assert!((start.y - ground).abs() < 1.0);
        // By its own beat it has climbed, and is still over the pack.
        let up = flag_at(PLANT, pack, plant, ground);
        assert!(up.y > ground + 20.0, "it climbed to {}", up.y);
        assert!((up.x - pack).abs() < 0.5, "and is at {}", up.x);
        // Then it is carried across and pushed into the ground.
        let planted = flag_at(HOLD, pack, plant, ground);
        assert!((planted.x - plant).abs() < 0.5, "at {}", planted.x);
        assert!((planted.y - ground).abs() < 1.0, "at {}", planted.y);
    }

    #[test]
    fn the_cartoon_needs_no_level() {
        let mut app = cartoon_app();
        assert!(app.world().get_resource::<ActiveLevel>().is_none());
        let scene = scene_size(&mut app);
        assert!(scene > 10, "the scene spawned {scene} things");
        assert!(scene < 50, "the scene spawned {scene} things, over budget");
    }

    #[test]
    fn leaving_takes_the_whole_scene_with_it() {
        let mut app = cartoon_app();
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::MainMenu);
        app.update();
        app.update();
        assert_eq!(scene_size(&mut app), 0, "entities outlived the cartoon");
        assert!(app.world().get_resource::<Showing>().is_none());
    }

    #[test]
    fn it_ends_at_the_main_menu_on_its_own() {
        let mut app = cartoon_app();
        for _ in 0..frames(TOTAL) + 10 {
            app.update();
        }
        assert_eq!(state(&app), AppState::MainMenu);
    }

    /// A key held down from the very first frame skips once, and does not
    /// leave the scene standing.
    #[test]
    fn a_key_on_the_first_frame_skips_it() {
        let mut app = cartoon_app();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
        app.update();
        app.update();
        assert_eq!(state(&app), AppState::MainMenu);
        assert_eq!(scene_size(&mut app), 0);
    }

    /// Bevy 0.19 renamed events to messages. If `Messages<T>` is not how they
    /// are read back, a system with a `MessageReader<EndingBeat>` pushing into
    /// a resource would do the same job.
    #[test]
    fn every_beat_is_announced_once() {
        let mut app = cartoon_app();
        let mut beats = vec![app.world().resource::<Showing>().beat];
        for _ in 0..frames(TOTAL) {
            app.update();
            if let Some(showing) = app.world().get_resource::<Showing>()
                && beats.last() != Some(&showing.beat)
            {
                beats.push(showing.beat);
            }
        }
        assert_eq!(
            beats,
            vec![
                Beat::Descent,
                Beat::Settle,
                Beat::Pack,
                Beat::Flag,
                Beat::Plant,
                Beat::Hold
            ],
            "every beat once, in order"
        );
    }

    /// The astronaut's feet meet the ground they land on.
    #[test]
    fn they_land_on_the_ground_and_not_in_it() {
        let mut app = cartoon_app();
        let (feet, ground) = {
            let showing = app.world().resource::<Showing>();
            (
                showing.feet,
                showing.planet.height_at(showing.planet.landing_x),
            )
        };
        assert!(
            (feet - ground - crate::config::ASTRONAUT_RADIUS).abs() < 0.5,
            "feet at {feet} over ground at {ground}"
        );
        for _ in 0..frames(TOUCHDOWN) + 2 {
            app.update();
        }
        let standing = app
            .world_mut()
            .query_filtered::<&Transform, With<Figure>>()
            .iter(app.world())
            .next()
            .map(|transform| transform.translation.y);
        assert!(
            standing.is_some_and(|y| (y - feet).abs() < 1.0),
            "{standing:?} against {feet}"
        );
    }
}
