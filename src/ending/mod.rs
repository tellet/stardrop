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
use art::{
    CHUTE_LIFT, FLAG_TOP, Q_AT, chute_mesh, cloth_mesh, color, flag_mesh, ground_mesh,
    letter_q_mesh, pack_mesh, prop_mesh, sky_mesh, water_mesh,
};
use planet::{Planet, PropKind, SURFACE_Y};

mod art;
mod planet;

/// When the astronaut touches the ground. They come down under a canopy, so
/// this is a long, slow arrival rather than a drop.
const TOUCHDOWN: f32 = 3.6;
/// When the dust has settled and the pack comes off.
const SETTLED: f32 = 5.0;
/// When the flag starts rising out of the pack.
const FLAG: f32 = 6.4;
/// When they lean in and push it into the ground.
const PLANT: f32 = 7.8;
/// When they stand back and look at it.
const HOLD: f32 = 8.8;
/// The whole cartoon, in seconds.
const TOTAL: f32 = 9.9;
// The brief asks for five to ten seconds (Section 11), checked when compiling.
const _: () = assert!(TOTAL >= 5.0 && TOTAL <= 10.0, "the cartoon runs 5-10 s");
// The last beat needs time on screen to be a beat at all.
const _: () = assert!(HOLD < TOTAL);
// They come down under a canopy, which is a slow way to arrive.
const _: () = assert!(TOUCHDOWN >= 3.0, "the descent is a slow one");

/// Where the astronaut enters, in world y: above the top of the frame.
const ENTRY_Y: f32 = 120.0;
/// How far they drift sideways on the way down, in world units.
const SWAY: f32 = 90.0;
/// How long the cut-away canopy takes to sink to the ground, in seconds, and
/// how far it drifts off while it does.
const CUTAWAY_SECONDS: f32 = 1.2;
const CUTAWAY_DRIFT: f32 = 160.0;
/// How much the open canopy breathes on the way down, and how fast.
const CHUTE_BREATH: f32 = 0.035;
const CHUTE_BREATH_SPEED: f32 = 2.0;
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

/// Eased 0 to 1: still at both ends, quickest in the middle.
#[must_use]
fn smooth(u: f32) -> f32 {
    u * u * (3.0 - 2.0 * u)
}

/// The astronaut's height: down from above the frame to their feet on the
/// ground at `feet`, slowing into the landing rather than speeding up. A fall
/// this is not; they are bringing themselves down.
#[must_use]
fn descent_y(t: f32, feet: f32) -> f32 {
    let u = through(t, 0.0, TOUCHDOWN);
    // Mostly even, easing off into the ground: a canopy brings someone down
    // at a steady rate, where a fall would keep gathering speed.
    let eased = 0.7 * u + 0.3 * (1.0 - (1.0 - u) * (1.0 - u));
    ENTRY_Y + (feet - ENTRY_Y) * eased
}

/// Their sideways drift: a slow sway on the way down that is spent by the
/// time they arrive, so they land where they are going to stand.
#[must_use]
fn descent_x(t: f32, landing_x: f32) -> f32 {
    let u = through(t, 0.0, TOUCHDOWN);
    landing_x + SWAY * (1.0 - u) * (u * 6.0).sin()
}

/// Where the canopy is: over their shoulders on the way down, then cut away,
/// sinking to the ground and drifting off as the dust settles.
#[must_use]
fn chute_at(t: f32, over: Vec2, ground: f32) -> Vec2 {
    let gone = smooth(through(t, TOUCHDOWN, TOUCHDOWN + CUTAWAY_SECONDS));
    Vec2::new(
        over.x - CUTAWAY_DRIFT * gone,
        over.y + (ground - over.y) * gone,
    )
}

/// How solid the canopy is. It is gone before the pack comes off, so the
/// joke lands on a clean screen.
#[must_use]
fn chute_fade(t: f32) -> f32 {
    1.0 - through(t, TOUCHDOWN + 0.3, SETTLED)
}

/// How the canopy breathes on the way down, and collapses once it is cut.
#[must_use]
fn chute_scale(t: f32) -> Vec2 {
    let breath = 1.0 + CHUTE_BREATH * (t * CHUTE_BREATH_SPEED).sin();
    let gone = smooth(through(t, TOUCHDOWN, TOUCHDOWN + CUTAWAY_SECONDS));
    Vec2::new(breath * (1.0 - 0.55 * gone), 1.0 - 0.7 * gone)
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
/// How long the pack takes to slide off a shoulder, in seconds.
const SLIDE_SECONDS: f32 = 0.5;
/// How far the astronaut leans into their drift, in radians.
const LEAN: f32 = 0.25;
/// And in towards the pole as they push it in.
const PLANT_LEAN: f32 = 0.3;
/// How far they bob as they straighten up, in world units.
const BOB: f32 = 8.0;
/// How hard the landing pushes nearby props away, in radians, how long that
/// takes to die away, and how far it carries.
const RECOIL: f32 = 0.35;
const RECOIL_SECONDS: f32 = 0.9;
const RECOIL_REACH: f32 = 500.0;
/// How long it takes to carry the flag across and push it in, in seconds.
const CARRY_SECONDS: f32 = 0.5;

/// Where the flag is: climbing out of the pack while it rises, then carried
/// across to where it is planted and pushed into the ground there. `from` and
/// `to` each sit on their own ground, which is rarely the same height.
#[must_use]
fn flag_at(t: f32, from: Vec2, to: Vec2) -> Vec2 {
    let lifted = from.y + FLAG_LIFT * flag_rise(t);
    let eased = smooth(through(t, PLANT, PLANT + CARRY_SECONDS));
    Vec2::new(
        from.x + (to.x - from.x) * eased,
        lifted + (to.y - lifted) * eased,
    )
}

/// Where the pack is: it slides off their shoulder and settles on the ground
/// beside them.
#[must_use]
fn pack_at(t: f32, shoulder: Vec2, ground: Vec2) -> Vec2 {
    shoulder.lerp(ground, smooth(through(t, SETTLED, SETTLED + SLIDE_SECONDS)))
}

/// How far the astronaut leans: into their own drift on the way down, upright
/// to land, and in towards the pole while they push it into the ground.
#[must_use]
fn figure_lean(t: f32) -> f32 {
    if t < TOUCHDOWN {
        let u = through(t, 0.0, TOUCHDOWN);
        // The same curve their drift follows, so the lean belongs to it.
        return -LEAN * (1.0 - u) * (u * 6.0).sin();
    }
    let into = through(t, PLANT, PLANT + CARRY_SECONDS);
    let out = through(t, HOLD, HOLD + 0.4);
    PLANT_LEAN * into * (1.0 - out)
}

/// The one bob as they straighten up after landing, in world units.
#[must_use]
fn figure_bob(t: f32) -> f32 {
    let u = through(t, TOUCHDOWN + 0.2, SETTLED);
    if u <= 0.0 || u >= 1.0 {
        return 0.0;
    }
    BOB * (u * core::f32::consts::PI).sin() * (1.0 - u)
}

/// How far the landing pushes a prop `distance` away from it: hardest right
/// next to the touchdown, gone a few hundred units out, and over quickly.
#[must_use]
fn prop_recoil(t: f32, distance: f32) -> f32 {
    let u = through(t, TOUCHDOWN, TOUCHDOWN + RECOIL_SECONDS);
    if u <= 0.0 || u >= 1.0 {
        return 0.0;
    }
    let reach = (1.0 - distance / RECOIL_REACH).clamp(0.0, 1.0);
    RECOIL * reach * (1.0 - u) * (1.0 - u)
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
/// How far from the astronaut the pack lands, and how far the other way the
/// flag goes in, both in astronaut radii: they stand on opposite sides, so
/// neither hides the other.
const PACK_RADII: f32 = -4.2;
const PLANT_RADII: f32 = 1.6;
/// The canopy hangs behind everyone, so its lines pass behind the astronaut.
const CHUTE_DEPTH: f32 = 7.0;
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

/// The parachute that brings them down.
#[derive(Component)]
struct Chute;

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
    /// How far it stands from the landing spot, for the recoil.
    from_landing: f32,
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
    /// Where the pack ends up, and where the flag is planted. Each sits on
    /// the ground under itself, not the ground under the astronaut.
    pack: Vec2,
    plant: Vec2,
    /// Set once the cartoon has asked to leave, so it only asks once.
    leaving: bool,
    /// Seconds until the next spark comes off the suit.
    trail: f32,
    /// Seconds until the vents breathe again.
    steam: f32,
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
                    fly_the_chute,
                    settle_the_dust,
                    sway_the_props,
                    breathe_the_vents,
                    unpack,
                    raise_the_flag,
                )
                    .chain()
                    .run_if(in_state(AppState::Ending)),
            );
        #[cfg(feature = "dev")]
        app.add_systems(
            Update,
            open_it_for_a_look.run_if(not(in_state(AppState::Ending))),
        );
    }
}

/// F3 (with `--features dev`): watch the cartoon from any screen, without
/// finishing every level first.
#[cfg(feature = "dev")]
fn open_it_for_a_look(
    keys: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keys.just_pressed(KeyCode::F3) {
        next_state.set(AppState::Ending);
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
            entity.insert(Swaying {
                phase,
                from_landing: (prop.at.x - planet.landing_x).abs(),
            });
        }
    }

    // The pack, at nothing until its beat, beside where they will stand.
    let landing_x = planet.landing_x;
    let ground = planet.height_at(landing_x);
    let radius = config.astronaut_radius;
    // The pack and the pole stand on the ground beneath each of them, which
    // on a rocky planet is nowhere near the height under the astronaut.
    let pack = Vec2::new(landing_x + radius * PACK_RADII, 0.0);
    let pack = Vec2::new(pack.x, planet.height_at(pack.x));
    let plant = Vec2::new(landing_x + radius * PLANT_RADII, 0.0);
    let plant = Vec2::new(plant.x, planet.height_at(plant.x));
    commands.spawn((
        EndingEntity,
        Pack,
        Mesh2d(meshes.add(pack_mesh(&palette))),
        MeshMaterial2d(tint.clone()),
        Transform::from_translation(pack.extend(PACK_DEPTH)).with_scale(Vec3::splat(0.0)),
    ));
    // The flag, still inside it.
    let flag = commands
        .spawn((
            EndingEntity,
            Flag,
            Mesh2d(meshes.add(flag_mesh(&palette))),
            MeshMaterial2d(tint.clone()),
            Transform::from_translation(pack.extend(FLAG_DEPTH)),
            Visibility::Hidden,
        ))
        .id();
    let cloth = commands
        .spawn((
            Cloth,
            Mesh2d(meshes.add(cloth_mesh(&palette))),
            MeshMaterial2d(tint.clone()),
            Transform::from_xyz(0.0, FLAG_TOP, 0.1),
        ))
        .id();
    commands.entity(flag).add_child(cloth);
    // A letter on the cloth, drawn like everything else here.
    let mark = commands
        .spawn((
            Mesh2d(meshes.add(letter_q_mesh(&palette))),
            MeshMaterial2d(tint),
            Transform::from_translation(Q_AT.extend(0.1)),
        ))
        .id();
    commands.entity(cloth).add_child(mark);

    let feet = ground + radius;
    commands.insert_resource(Showing {
        elapsed: 0.0,
        beat: Beat::Descent,
        planet,
        feet,
        pack,
        plant,
        leaving: false,
        trail: 0.0,
        steam: 0.0,
        thumped: false,
    });
    // The canopy carries its own material, because it is the one thing here
    // that fades out.
    let here = Vec2::new(descent_x(0.0, landing_x), descent_y(0.0, feet));
    commands.spawn((
        EndingEntity,
        Chute,
        Mesh2d(meshes.add(chute_mesh(&palette))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::WHITE))),
        Transform::from_translation((here + Vec2::Y * (radius + CHUTE_LIFT)).extend(CHUTE_DEPTH)),
    ));
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

/// The canopy: over their shoulders on the way down, cut away when they
/// land, and faded out before the pack comes off.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Bevy systems take their resources by value"
)]
fn fly_the_chute(
    showing: Res<Showing>,
    config: Res<GameConfig>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    canopy: Single<
        (
            &mut Transform,
            &mut Visibility,
            &MeshMaterial2d<ColorMaterial>,
        ),
        With<Chute>,
    >,
) {
    let (mut transform, mut visibility, material) = canopy.into_inner();
    let t = showing.elapsed;
    let fade = chute_fade(t);
    if fade <= 0.0 {
        *visibility = Visibility::Hidden;
        return;
    }
    let radius = config.astronaut_radius;
    let shoulders = Vec2::new(
        descent_x(t, showing.planet.landing_x),
        descent_y(t, showing.feet) + radius,
    );
    let at = chute_at(t, shoulders + Vec2::Y * CHUTE_LIFT, showing.feet - radius);
    transform.translation = at.extend(CHUTE_DEPTH);
    let scale = chute_scale(t);
    transform.scale = Vec3::new(scale.x, scale.y, 1.0);
    if let Some(mut canopy) = materials.get_mut(&material.0) {
        canopy.color = canopy.color.with_alpha(fade);
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
        let bob = figure_bob(elapsed);
        transform.translation = (at + Vec2::new(0.0, bob)).extend(transform.translation.z);
        let squash = squash(elapsed);
        transform.scale = Vec3::new(squash.x, squash.y, 1.0);
        transform.rotation = Quat::from_rotation_z(figure_lean(elapsed));
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
                    at: showing.plant,
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
                    at: showing.plant + Vec2::new(0.0, 70.0),
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
            at: showing.pack,
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
    showing: Res<Showing>,
    mut props: Query<(&Swaying, &mut Transform)>,
) {
    let motion = settings.motion_scale();
    let now = time.elapsed_secs();
    let landing_x = showing.planet.landing_x;
    for (swaying, mut transform) in &mut props {
        let sway = SWAY_LEAN * motion * (now * SWAY_SPEED + swaying.phase).sin();
        // Whatever is near the touchdown bends away from it.
        let away = if transform.translation.x < landing_x {
            -1.0
        } else {
            1.0
        };
        let flinch = away * prop_recoil(showing.elapsed, swaying.from_landing) * motion;
        transform.rotation = Quat::from_rotation_z(sway + flinch);
    }
}

/// The vents breathe all the way through, and harder when the dust is up.
fn breathe_the_vents(
    time: Res<Time>,
    mut showing: ResMut<Showing>,
    vents: Query<&Transform, With<Vent>>,
    mut bursts: MessageWriter<Burst>,
) {
    showing.steam -= time.delta_secs();
    if showing.steam > 0.0 {
        return;
    }
    showing.steam = STEAM_EVERY;
    let hard = showing.beat == Beat::Settle;
    let steam = color(showing.planet.palette.accent);
    for transform in &vents {
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
    // It comes off their shoulder, so it starts where they are standing.
    let shoulder = Vec2::new(showing.planet.landing_x, showing.feet);
    let at = pack_at(showing.elapsed, shoulder, showing.pack);
    for mut transform in &mut pack {
        transform.scale = Vec3::new(scale, scale, 1.0);
        transform.translation = at.extend(transform.translation.z);
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Bevy systems take their resources by value"
)]
fn raise_the_flag(
    settings: Res<Settings>,
    showing: Res<Showing>,
    mut flag: Query<(&mut Transform, &mut Visibility), With<Flag>>,
    mut cloth: Query<&mut Transform, (With<Cloth>, Without<Flag>)>,
) {
    let risen = flag_rise(showing.elapsed);
    let at = flag_at(showing.elapsed, showing.pack, showing.plant);
    for (mut transform, mut visibility) in &mut flag {
        visibility.set_if_neq(if risen > 0.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        transform.translation = at.extend(transform.translation.z);
    }
    let wave = flag_wave(showing.elapsed, settings.motion_scale());
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
    use crate::config::ASTRONAUT_RADIUS;
    use crate::ending::art::{CLOTH_WIDTH, PACK_REACH};
    use crate::level::{ActiveLevel, LEVEL_WIDTH};
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

    /// The game's screens and the plugins the cartoon leans on, before any
    /// cartoon has been watched.
    fn app_before_the_cartoon() -> App {
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
        app.update();
        app
    }

    fn cartoon_app() -> App {
        let mut app = app_before_the_cartoon();
        enter(&mut app, AppState::Ending);
        app
    }

    fn enter(app: &mut App, state: AppState) {
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(state);
        app.update();
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
    fn the_beats_tile_the_whole_cartoon() {
        let mut seen = Vec::new();
        let mut t = 0.0;
        while t <= TOTAL {
            let beat = beat_at(t);
            if seen.last() != Some(&beat) {
                seen.push(beat);
            }
            t += 1.0 / 240.0;
        }
        assert_eq!(
            seen,
            vec![
                Beat::Descent,
                Beat::Settle,
                Beat::Pack,
                Beat::Flag,
                Beat::Plant,
                Beat::Hold
            ],
            "the beats must come once each, in order"
        );
    }

    #[test]
    fn the_astronaut_arrives_as_the_dust_does() {
        let feet = -1400.0;
        assert!(
            descent_y(0.0, feet) >= ENTRY_Y,
            "they start above the frame"
        );
        assert!(
            (descent_y(TOUCHDOWN, feet) - feet).abs() < 0.5,
            "and land on the ground"
        );
        assert!(
            (descent_y(TOTAL, feet) - feet).abs() < 0.5,
            "and stay there"
        );
        // Slowing into the landing, not speeding up: the first half covers more.
        let half = descent_y(0.0, feet) - descent_y(TOUCHDOWN / 2.0, feet);
        let all = descent_y(0.0, feet) - feet;
        assert!(half > all / 2.0, "{half} of {all}");
        // They come down where they are going to stand.
        assert!((descent_x(TOUCHDOWN, 120.0) - 120.0).abs() < 0.5);
    }

    /// They are brought down by a canopy, not dropped: the descent is long,
    /// and no part of it is much faster than any other.
    #[test]
    fn the_descent_is_slow_and_even() {
        let feet = -1400.0;
        let average = (descent_y(0.0, feet) - feet) / TOUCHDOWN;
        let mut fastest: f32 = 0.0;
        let mut t = 0.0;
        while t + 0.1 <= TOUCHDOWN {
            let speed = (descent_y(t, feet) - descent_y(t + 0.1, feet)) / 0.1;
            fastest = fastest.max(speed);
            t += 0.05;
        }
        assert!(
            fastest < average * 1.6,
            "{fastest} at the fastest against {average} on average"
        );
    }

    #[test]
    fn the_canopy_carries_them_down_and_is_cut_away() {
        let over = Vec2::new(40.0, -200.0);
        let ground = -520.0;
        assert!(
            (chute_at(0.0, over, ground) - over).length() < 0.5,
            "it flies over them"
        );
        assert!(
            (chute_at(TOUCHDOWN, over, ground) - over).length() < 0.5,
            "and stays there until they land"
        );
        let settled = chute_at(TOUCHDOWN + CUTAWAY_SECONDS, over, ground);
        assert!(
            (settled.y - ground).abs() < 1.0,
            "it should come to rest on the ground, not at {}",
            settled.y
        );
        assert!(
            settled.x < over.x - CUTAWAY_DRIFT * 0.9,
            "it should drift clear of them, not stop at {}",
            settled.x
        );
        assert!(
            chute_fade(TOUCHDOWN) > 0.9,
            "it is solid while it carries them"
        );
        assert!(
            chute_fade(SETTLED) <= 0.0,
            "it should be gone before the pack comes off"
        );
        assert!(
            chute_scale(TOUCHDOWN + CUTAWAY_SECONDS).y < 0.4,
            "it should collapse as it falls"
        );
    }

    #[test]
    fn the_squash_is_one_bounce_and_then_nothing() {
        assert!(
            (squash(0.0) - Vec2::ONE).length() < 1e-6,
            "nothing before the landing"
        );
        assert!(squash(TOUCHDOWN + 0.05).y < 0.9, "it gives on impact");
        assert!(squash(TOUCHDOWN + 0.05).x > 1.05, "and spreads sideways");
        assert!(
            (squash(SETTLED).y - 1.0).abs() < 0.01,
            "and is over by the settle"
        );
    }

    #[test]
    fn the_pack_and_the_flag_wait_their_turn() {
        assert!(pack_scale(SETTLED - 0.01) < 1e-6, "no pack before its beat");
        assert!(
            pack_scale(FLAG) > 0.99,
            "full size by the time the flag comes"
        );
        assert!(flag_rise(FLAG - 0.01) < 1e-6);
        assert!(flag_rise(PLANT) > 0.99, "up before it is planted");
        assert!(
            flag_wave(PLANT - 0.01, 1.0).abs() < 1e-6,
            "still in the pack"
        );
        // Reduce motion calms the wave instead of stopping it.
        let calm = flag_wave(HOLD + 0.2, 0.0);
        let full = flag_wave(HOLD + 0.2, 1.0);
        assert!(calm.abs() > 1e-3, "it still moves when motion is reduced");
        assert!(calm.abs() < full.abs(), "{calm} against {full}");
    }

    #[test]
    fn the_flag_climbs_out_then_goes_into_the_ground() {
        // The pack and the planting spot sit on ground of their own heights.
        let from = Vec2::new(100.0, -1400.0);
        let to = Vec2::new(200.0, -1430.0);
        let start = flag_at(FLAG, from, to);
        assert!((start - from).length() < 1.0, "it starts in the pack");
        let up = flag_at(PLANT, from, to);
        assert!(up.y > from.y + 20.0, "it climbed to {}", up.y);
        assert!((up.x - from.x).abs() < 0.5, "and is still at {}", up.x);
        // Then it is carried across and pushed into the ground it ends on.
        let planted = flag_at(HOLD, from, to);
        assert!(
            (planted - to).length() < 1.0,
            "planted at {planted}, not {to}"
        );
    }

    #[test]
    fn the_pack_slides_off_their_shoulder_onto_the_ground() {
        let shoulder = Vec2::new(0.0, -1380.0);
        let ground = Vec2::new(70.0, -1420.0);
        assert!(
            (pack_at(SETTLED, shoulder, ground) - shoulder).length() < 1.0,
            "it starts on their back"
        );
        assert!(
            (pack_at(FLAG, shoulder, ground) - ground).length() < 1.0,
            "and is down by the time the flag comes"
        );
    }

    #[test]
    fn they_lean_on_the_way_down_and_straighten_to_land() {
        assert!(figure_lean(0.6).abs() > 0.02, "leaning into the drift");
        assert!(figure_lean(TOUCHDOWN).abs() < 0.01, "upright to land");
        assert!(figure_lean(PLANT + 0.2) > 0.05, "leaning in to plant it");
        assert!(figure_lean(TOTAL).abs() < 0.01, "standing beside it again");
    }

    #[test]
    fn they_straighten_up_with_one_bob() {
        assert!(
            figure_bob(TOUCHDOWN - 0.1).abs() < 1e-6,
            "not before they land"
        );
        let mid = figure_bob((TOUCHDOWN + SETTLED) / 2.0);
        assert!(mid > 1.0, "they come back up: {mid}");
        assert!(figure_bob(SETTLED).abs() < 1e-6, "and are still by the end");
        assert!(figure_bob(TOTAL).abs() < 1e-6);
    }

    #[test]
    fn the_landing_pushes_the_nearest_props_away() {
        assert!(
            prop_recoil(TOUCHDOWN - 0.1, 50.0).abs() < 1e-6,
            "nothing before it lands"
        );
        let near = prop_recoil(TOUCHDOWN + 0.05, 50.0);
        let far = prop_recoil(TOUCHDOWN + 0.05, 400.0);
        assert!(near > far, "{near} against {far}");
        assert!(
            prop_recoil(TOUCHDOWN + 0.05, 2_000.0).abs() < 1e-6,
            "and nothing to anything far away"
        );
        assert!(
            prop_recoil(SETTLED, 50.0).abs() < 1e-3,
            "over well before the pack comes off"
        );
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
        enter(&mut app, AppState::MainMenu);
        app.update();
        assert_eq!(scene_size(&mut app), 0, "entities outlived the cartoon");
        assert!(app.world().get_resource::<Showing>().is_none());
    }

    /// The pack stands clear of the astronaut, the flag stands clear of the
    /// pack, and wherever a planet puts the landing spot, the whole tableau
    /// is still on a portrait screen.
    #[test]
    fn the_pack_and_the_flag_stand_clear_of_everything() {
        let radius = ASTRONAUT_RADIUS;
        let pack_near = PACK_RADII * radius + PACK_REACH;
        assert!(
            pack_near < -radius,
            "the pack stands over the astronaut, up to {pack_near}"
        );
        let flag_near = PLANT_RADII * radius;
        assert!(
            flag_near > radius,
            "the flag goes in at the astronaut's feet, at {flag_near}"
        );
        for seed in 0..2_000 {
            let landing = Planet::generate(seed).landing_x;
            let left = landing + PACK_RADII * radius - PACK_REACH;
            let right = landing + PLANT_RADII * radius + CLOTH_WIDTH;
            assert!(
                left >= -LEVEL_WIDTH / 2.0,
                "seed {seed}: the pack hangs off the screen at {left}"
            );
            assert!(
                right <= LEVEL_WIDTH / 2.0,
                "seed {seed}: the flag hangs off the screen at {right}"
            );
        }
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

    /// In dev builds F3 opens the cartoon from any screen, so it can be
    /// watched without finishing every level first.
    #[cfg(feature = "dev")]
    #[test]
    fn f3_opens_the_cartoon() {
        let mut app = app_before_the_cartoon();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F3);
        app.update();
        // The real game clears `just_pressed` every frame; this app has no
        // input plugin to do it, and a key still down would skip the cartoon.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();
        assert_eq!(state(&app), AppState::Ending);
        assert!(scene_size(&mut app) > 0, "the planet is standing");
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

    /// Nothing hovers: the pack and the pole stand on the ground beneath
    /// them, which is not the ground under the astronaut's feet.
    #[test]
    fn the_pack_and_the_pole_stand_on_their_own_ground() {
        let mut app = cartoon_app();
        for _ in 0..frames(TOTAL) - 4 {
            app.update();
        }
        let (pack_ground, plant_ground) = {
            let showing = app.world().resource::<Showing>();
            (
                showing.planet.height_at(showing.pack.x),
                showing.planet.height_at(showing.plant.x),
            )
        };
        let pack = app
            .world_mut()
            .query_filtered::<&Transform, With<Pack>>()
            .iter(app.world())
            .next()
            .map(|transform| transform.translation.y);
        assert!(
            pack.is_some_and(|y| (y - pack_ground).abs() < 1.0),
            "the pack is at {pack:?}, the ground under it at {pack_ground}"
        );
        let flag = app
            .world_mut()
            .query_filtered::<&Transform, With<Flag>>()
            .iter(app.world())
            .next()
            .map(|transform| transform.translation.y);
        assert!(
            flag.is_some_and(|y| (y - plant_ground).abs() < 1.0),
            "the pole is at {flag:?}, the ground under it at {plant_ground}"
        );
    }

    /// The menu offers the cartoon again, so watching it five times must
    /// cost no more than watching it once: nothing left in the world and no
    /// meshes left in the store. The count after a watch is no guide (every
    /// planet grows a different number of props); the count with no cartoon
    /// loaded is.
    #[test]
    fn watching_it_again_leaves_nothing_behind() {
        let mut app = app_before_the_cartoon();
        let baseline = app.world().resource::<Assets<Mesh>>().len();
        for watch in 1..=5 {
            enter(&mut app, AppState::Ending);
            assert!(scene_size(&mut app) > 10, "watch {watch} drew nothing");
            enter(&mut app, AppState::MainMenu);
            // One more frame, so the dropped handles are collected.
            app.update();
            assert_eq!(scene_size(&mut app), 0, "watch {watch} left entities");
            assert_eq!(
                app.world().resource::<Assets<Mesh>>().len(),
                baseline,
                "watch {watch} left meshes behind"
            );
        }
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
            (feet - ground - ASTRONAUT_RADIUS).abs() < 0.5,
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
