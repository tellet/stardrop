//! On-screen messages during play (level-file problems, the hint that tells
//! the player how to start falling, the level's name), and the soft fade
//! every screen change comes in through.

use bevy::prelude::*;

use crate::app_state::AppState;
use crate::collectibles::CurrentRun;
use crate::level::{ActiveLevel, LevelPhase, LevelProblem};
use crate::note_signs::NoteArt;
use crate::settings::Settings;

/// Soft coral: clearly a problem, but calm, like everything else.
const PROBLEM_COLOR: Color = Color::srgb(1.0, 0.72, 0.62);
const PROBLEM_FONT_SIZE: f32 = 22.0;

const HINT_COLOR: Color = Color::srgba(0.85, 0.85, 1.0, 0.7);
const HINT_FONT_SIZE: f32 = 26.0;
const NAME_COLOR: Color = Color::srgb(0.92, 0.93, 1.0);
const NAME_FONT_SIZE: f32 = 44.0;
/// The level's name fades in over this many seconds while the player looks.
const NAME_FADE_SECONDS: f32 = 1.2;

/// How long a screen takes to appear out of the dark.
const SCREEN_FADE_SECONDS: f32 = 0.28;
/// The colour screens fade in from: the night sky, never black.
const SCREEN_FADE_COLOR: Color = Color::srgb(0.02, 0.015, 0.06);

/// The text node that shows level problems.
#[derive(Component)]
struct ProblemText;

/// Anything that belongs to the level being played, and so has no business
/// on a menu, the journal or the level-complete screen.
#[derive(Component)]
struct PlayOnly;

/// One mark of the phrase, by its place in it.
#[derive(Component)]
struct PhraseMark(u32);

/// One mark of the level's stardust, by its place in the list.
#[derive(Component)]
struct MoteMark(usize);

/// How big the marks are, and how far apart.
const MARK_SIZE: f32 = 20.0;
const MARK_GAP: f32 = 10.0;
/// Lit once that note has been played, dim before.
const MARK_PLAYED: Color = Color::linear_rgba(2.4, 1.9, 0.9, 1.0);
const MARK_WAITING: Color = Color::srgba(0.7, 0.7, 0.85, 0.22);

/// The stardust marks: round, like the motes themselves, and smaller than
/// the notes so the two rows can never be read as one.
const MOTE_MARK_SIZE: f32 = 14.0;
const MOTE_MARK_GAP: f32 = 8.0;
/// How far the row sits from the corner, in pixels.
const MOTE_ROW_INSET: f32 = 20.0;
/// Collected motes keep the stardust's own gold; the rest are barely there.
const MOTE_COLLECTED: Color = Color::linear_rgba(2.4, 1.8, 0.6, 1.0);
const MOTE_WAITING: Color = Color::srgba(0.7, 0.7, 0.85, 0.18);

/// How long the phrase row stays up after a note lands, and how long it
/// takes to fade out again.
const PHRASE_HOLD: f32 = 1.5;
const PHRASE_FADE: f32 = 0.7;

/// Seconds of phrase row left to show. The row is the melody's progress,
/// which only matters just after a note: the rest of the time the sky is
/// better empty.
#[derive(Resource, Debug, Default)]
struct PhraseShown(f32);

/// The overlay every screen change fades in through.
#[derive(Component)]
struct ScreenFade;

/// How much of the current screen fade is left, in seconds.
#[derive(Resource, Debug, Default)]
struct ScreenFadeLeft(f32);

/// On-screen messages.
pub(crate) struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScreenFadeLeft>()
            .init_resource::<PhraseShown>()
            .add_systems(
                Startup,
                (spawn_problem_text, spawn_screen_fade, prepare_star_art),
            )
            .add_systems(
                Update,
                (start_screen_fade, show_play_ui_only_in_play).run_if(state_changed::<AppState>),
            )
            .add_systems(Update, run_screen_fade)
            .add_systems(
                Update,
                // Only when the problem changes, never every frame.
                show_problem.run_if(resource_changed::<LevelProblem>),
            )
            .add_systems(
                OnEnter(LevelPhase::Ready),
                (spawn_drop_hint, spawn_level_name, show_the_phrase_now),
            )
            .add_systems(
                Update,
                (
                    (rebuild_phrase_marks, rebuild_mote_marks)
                        .run_if(resource_exists_and_changed::<ActiveLevel>),
                    light_mote_marks,
                    (
                        show_the_phrase_on_a_note,
                        fade_the_phrase,
                        light_phrase_marks,
                    )
                        .chain(),
                ),
            )
            .add_systems(Update, fade_in_level_name);
    }
}

/// How dark the screen still is, `left` seconds into a fade: full at the
/// moment the screen changes, gone smoothly by the end.
#[must_use]
fn screen_fade_alpha(left: f32) -> f32 {
    let t = (left / SCREEN_FADE_SECONDS).clamp(0.0, 1.0);
    t * t
}

fn spawn_screen_fade(mut commands: Commands) {
    commands.spawn((
        ScreenFade,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(SCREEN_FADE_COLOR.with_alpha(0.0)),
        // Under the attempt's own fade, over everything else.
        GlobalZIndex(9),
        // Never swallow a click meant for the menu underneath.
        Pickable::IGNORE,
    ));
}

/// Keeps the level's own words (its name, the hint) off every other screen.
/// They belong to the fall, not to the menu in front of it.
fn show_play_ui_only_in_play(
    state: Res<State<AppState>>,
    mut nodes: Query<&mut Visibility, With<PlayOnly>>,
) {
    let playing = *state.get() == AppState::Playing;
    for mut visibility in &mut nodes {
        visibility.set_if_neq(if playing {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

fn start_screen_fade(settings: Res<Settings>, mut fade: ResMut<ScreenFadeLeft>) {
    fade.0 = SCREEN_FADE_SECONDS * settings.motion_scale();
}

/// Lifts the fade off the new screen. Does nothing at all between screens.
fn run_screen_fade(
    time: Res<Time>,
    mut fade: ResMut<ScreenFadeLeft>,
    mut overlay: Query<&mut BackgroundColor, With<ScreenFade>>,
) {
    if fade.0 <= 0.0 {
        return;
    }
    fade.0 = (fade.0 - time.delta_secs()).max(0.0);
    let alpha = screen_fade_alpha(fade.0);
    for mut color in &mut overlay {
        color.0.set_alpha(alpha);
    }
}

/// A note's place in the row. Phrases are a handful of notes long.
fn note_offset(note: u32) -> f32 {
    u16::try_from(note).map_or(0.0, f32::from)
}

/// The phrase, as a row of notes along the top: one per note the level asks
/// for, lighting up as each object is touched.
///
/// Notes, not stars. A star on screen means a star earned, and the row was
/// read as a stardust counter while it counted something else entirely.
///
/// The melody is the real feedback; the row is there for the player who
/// hasn't noticed yet that the notes mean something, which is why it shows
/// itself when one lands and then gets out of the way.
fn rebuild_phrase_marks(
    mut commands: Commands,
    level: Res<ActiveLevel>,
    art: Option<Res<NoteArt>>,
    old: Query<Entity, With<PhraseMark>>,
) {
    for entity in &old {
        commands.entity(entity).despawn();
    }
    let Some(art) = art else {
        return;
    };
    for note in 0..u32::from(level.phrase) {
        commands.spawn((
            PlayOnly,
            PhraseMark(note),
            ImageNode::new(art.0.clone()).with_color(MARK_WAITING),
            Node {
                position_type: PositionType::Absolute,
                // Along the top, out of the way of the beacon and of where
                // the astronaut spends its time.
                top: Val::Percent(3.0),
                // Centred as a group: each mark steps right from the middle.
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(
                    // Step right from the middle, so the row sits centred.
                    (note_offset(note) - (f32::from(level.phrase) - 1.0) / 2.0)
                        * (MARK_SIZE + MARK_GAP)
                        - MARK_SIZE / 2.0,
                )),
                width: Val::Px(MARK_SIZE),
                height: Val::Px(MARK_SIZE),
                ..default()
            },
        ));
    }
}

/// How much of the phrase row is showing, `left` seconds before it is gone:
/// solid while it is held, then eased away.
#[must_use]
fn phrase_row_alpha(left: f32) -> f32 {
    let t = (left / PHRASE_FADE).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Brings the row back whenever a note lands.
fn show_the_phrase_on_a_note(
    run: Res<CurrentRun>,
    mut shown: ResMut<PhraseShown>,
    mut last: Local<u32>,
) {
    if run.touches > *last {
        shown.0 = PHRASE_HOLD + PHRASE_FADE;
    }
    *last = run.touches;
}

/// And shows it once at the start, so the player sees what the level asks
/// for before they drop.
fn show_the_phrase_now(mut shown: ResMut<PhraseShown>) {
    shown.0 = PHRASE_HOLD + PHRASE_FADE;
}

fn fade_the_phrase(time: Res<Time>, mut shown: ResMut<PhraseShown>) {
    if shown.0 > 0.0 {
        shown.0 = (shown.0 - time.delta_secs()).max(0.0);
    }
}

/// Lights one mark per object touched this attempt, under the row's own
/// fade. Does nothing at all once the row is gone and the run is quiet.
fn light_phrase_marks(
    run: Res<CurrentRun>,
    shown: Res<PhraseShown>,
    mut marks: Query<(&PhraseMark, &mut ImageNode)>,
) {
    if !run.is_changed() && !shown.is_changed() {
        return;
    }
    let alpha = phrase_row_alpha(shown.0);
    for (mark, mut image) in &mut marks {
        let base = if run.touches > mark.0 {
            MARK_PLAYED
        } else {
            MARK_WAITING
        };
        let wanted = base.with_alpha(base.alpha() * alpha);
        if image.color != wanted {
            image.color = wanted;
        }
    }
}

/// The stardust, as a row of motes in the top corner: one per mote in the
/// level, filling as they are collected. The only counter that stays up,
/// because stardust is the thing the player picks up.
fn rebuild_mote_marks(
    mut commands: Commands,
    level: Res<ActiveLevel>,
    old: Query<Entity, With<MoteMark>>,
) {
    for entity in &old {
        commands.entity(entity).despawn();
    }
    for index in 0..level.stardust.len() {
        commands.spawn((
            PlayOnly,
            MoteMark(index),
            BackgroundColor(MOTE_WAITING),
            Node {
                position_type: PositionType::Absolute,
                // A circle: the same shape as the mote in the level.
                border_radius: BorderRadius::MAX,
                top: Val::Percent(3.0),
                left: Val::Px(
                    MOTE_ROW_INSET + mote_offset(index) * (MOTE_MARK_SIZE + MOTE_MARK_GAP),
                ),
                width: Val::Px(MOTE_MARK_SIZE),
                height: Val::Px(MOTE_MARK_SIZE),
                ..default()
            },
        ));
    }
}

/// A mote's place in the row. A level holds at most 64.
fn mote_offset(index: usize) -> f32 {
    u16::try_from(index).map_or(0.0, f32::from)
}

/// Fills the marks of the motes collected this attempt.
fn light_mote_marks(run: Res<CurrentRun>, mut marks: Query<(&MoteMark, &mut BackgroundColor)>) {
    if !run.is_changed() {
        return;
    }
    for (mark, mut color) in &mut marks {
        let wanted = if run.stardust.contains(mark.0) {
            MOTE_COLLECTED
        } else {
            MOTE_WAITING
        };
        if color.0 != wanted {
            color.0 = wanted;
        }
    }
}

fn spawn_problem_text(mut commands: Commands) {
    commands.spawn((
        ProblemText,
        Text::default(),
        TextFont::from_font_size(PROBLEM_FONT_SIZE),
        TextColor(PROBLEM_COLOR),
        TextLayout::justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(40.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        Visibility::Hidden,
    ));
}

fn show_problem(
    problem: Res<LevelProblem>,
    mut text: Single<(&mut Text, &mut Visibility), With<ProblemText>>,
) {
    let (text, visibility) = &mut *text;
    match &problem.0 {
        Some(message) => {
            text.0.clone_from(message);
            **visibility = Visibility::Visible;
        }
        None => **visibility = Visibility::Hidden,
    }
}

/// The level's name, which fades in gently while the astronaut waits.
#[derive(Component)]
struct LevelName(f32);

/// Shows the level's name at the top while the player studies the level.
fn spawn_level_name(mut commands: Commands, level: Option<Res<ActiveLevel>>) {
    let Some(level) = level else {
        return;
    };
    commands.spawn((
        LevelName(0.0),
        PlayOnly,
        DespawnOnExit(LevelPhase::Ready),
        Text::new(level.name.clone()),
        TextFont::from_font_size(NAME_FONT_SIZE),
        TextColor(NAME_COLOR.with_alpha(0.0)),
        TextLayout::justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(8.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
    ));
}

/// Eases the name in (smoothstep), then leaves it be.
fn fade_in_level_name(time: Res<Time>, mut names: Query<(&mut LevelName, &mut TextColor)>) {
    for (mut name, mut color) in &mut names {
        if name.0 >= NAME_FADE_SECONDS {
            continue;
        }
        name.0 += time.delta_secs();
        let t = (name.0 / NAME_FADE_SECONDS).clamp(0.0, 1.0);
        color.0.set_alpha(t * t * (3.0 - 2.0 * t));
    }
}

/// "How to start" hint while the astronaut waits at the start. Removed as soon
/// as the fall begins. Plain words: Bevy's built-in font has no arrow symbols.
fn spawn_drop_hint(mut commands: Commands) {
    commands.spawn((
        PlayOnly,
        DespawnOnExit(LevelPhase::Ready),
        Text::new("press left or right to drop"),
        TextFont::from_font_size(HINT_FONT_SIZE),
        TextColor(HINT_COLOR),
        TextLayout::justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Percent(12.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
    ));
}

// --- Stars -----------------------------------------------------------------

/// How big the star picture is, in pixels. Drawn once, shown at whatever
/// size a screen asks for.
const STAR_PIXELS: u32 = 64;
/// A five-pointed star: how deep the notches between its points go.
const STAR_WAIST: f32 = 0.38;
/// How far the glow reaches past the star's points.
const STAR_GLOW: f32 = 0.30;

/// The star picture, made once and used by every screen that shows stars.
#[derive(Resource)]
pub(crate) struct StarArt(pub Handle<Image>);

/// Draws a soft five-pointed star: white, with its shape in the alpha, so a
/// screen can tint it gold for earned and grey for not.
fn star_image() -> Image {
    let size = STAR_PIXELS as usize;
    // Pixel coordinates in a 64-wide picture: exact in f32.
    let at = |value: usize| u16::try_from(value).map_or(0.0, f32::from);
    let mut pixels = Vec::with_capacity(size * size * 4);
    let middle = (at(size) - 1.0) / 2.0;
    for y in 0..size {
        for x in 0..size {
            let from_middle = Vec2::new(at(x) - middle, at(y) - middle) / middle;
            let distance = from_middle.length();
            // Five points. The radius allowed at this angle runs straight
            // from a tip down to a notch and back: a curve here would make
            // petals, and this is a star.
            // Picture rows run downwards, so up is negative y. The half
            // turn puts a point straight up instead of off to one side.
            let angle = (-from_middle.y).atan2(from_middle.x);
            let around =
                (angle - core::f32::consts::FRAC_PI_2) / (core::f32::consts::TAU / 5.0) + 0.5;
            let within = around - around.floor();
            let point = 1.0 - 2.0 * (within - 0.5).abs();
            let reach = STAR_WAIST + (1.0 - STAR_WAIST) * point;
            // Solid inside the shape, fading out over a pixel or two, with a
            // wider halo beyond it so it sits in light rather than on top.
            let edge = 2.0 / middle;
            let solid = (1.0 - (distance - reach * (1.0 - edge)) / edge.max(1e-4)).clamp(0.0, 1.0);
            let glow = (1.0 - (distance - reach) / STAR_GLOW).clamp(0.0, 1.0) * 0.35;
            let alpha = solid.max(glow * glow);
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
            width: STAR_PIXELS,
            height: STAR_PIXELS,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        pixels,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    )
}

fn prepare_star_art(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.insert_resource(StarArt(images.add(star_image())));
}
