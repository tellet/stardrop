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

/// The window the UI's pixel sizes were drawn for: the size the game opens at
/// on the desktop (`INITIAL_WINDOW_SIZE` in lib.rs). A browser tab is usually
/// smaller, so the UI is scaled to fit it.
const UI_REFERENCE: Vec2 = Vec2::new(675.0, 1200.0);

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
                (spawn_drop_hint, spawn_level_name),
            )
            .add_systems(Update, (fade_in_level_name, fit_ui_to_window));
    }
}

/// How much to shrink the UI so it still fits a window smaller than the one it
/// was drawn for. Never grows it: a bigger window gets the UI at its drawn
/// size, with the camera showing more of the world around the playfield.
#[must_use]
fn ui_fit(window: Vec2) -> f32 {
    (window / UI_REFERENCE).min_element().clamp(0.1, 1.0)
}

/// Keeps the UI inside whatever window the player (or the page) gives us.
fn fit_ui_to_window(window: Single<&Window>, mut scale: ResMut<UiScale>) {
    let fit = ui_fit(window.size());
    if (scale.0 - fit).abs() > f32::EPSILON {
        scale.0 = fit;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ui_keeps_its_drawn_size_on_a_big_enough_window() {
        assert!((ui_fit(UI_REFERENCE) - 1.0).abs() < 1e-6);
        assert!((ui_fit(UI_REFERENCE * 2.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn the_ui_shrinks_with_a_small_window() {
        // A browser panel 400x711: both sides are 59% of the drawn size.
        assert!((ui_fit(Vec2::new(400.0, 711.0)) - 0.5925).abs() < 0.001);
    }

    #[test]
    fn the_narrower_side_decides() {
        // Tall and narrow: the width is what the menu rows would overflow.
        assert!((ui_fit(Vec2::new(337.5, 1200.0)) - 0.5).abs() < 1e-6);
        assert!((ui_fit(Vec2::new(675.0, 600.0)) - 0.5).abs() < 1e-6);
    }
}
