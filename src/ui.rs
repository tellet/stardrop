//! On-screen messages during play (level-file problems, the hint that tells
//! the player how to start falling, the level's name), and the soft fade
//! every screen change comes in through.

use bevy::prelude::*;

use crate::app_state::AppState;
use crate::level::{ActiveLevel, LevelPhase, LevelProblem};
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
            .add_systems(Startup, (spawn_problem_text, spawn_screen_fade))
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
                (spawn_drop_hint, spawn_level_name),
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
