//! On-screen messages during play: level-file problems and the hint that
//! tells the player how to start falling.

use bevy::prelude::*;

use crate::level::{LevelPhase, LevelProblem};

/// Soft coral: clearly a problem, but calm, like everything else.
const PROBLEM_COLOR: Color = Color::srgb(1.0, 0.72, 0.62);
const PROBLEM_FONT_SIZE: f32 = 22.0;

const HINT_COLOR: Color = Color::srgba(0.85, 0.85, 1.0, 0.7);
const HINT_FONT_SIZE: f32 = 26.0;

/// The text node that shows level problems.
#[derive(Component)]
struct ProblemText;

/// On-screen messages.
pub(crate) struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_problem_text)
            .add_systems(
                Update,
                // Only when the problem changes, never every frame.
                show_problem.run_if(resource_changed::<LevelProblem>),
            )
            .add_systems(OnEnter(LevelPhase::Ready), spawn_drop_hint);
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

/// "How to start" hint while the astronaut waits at the start. Removed as soon
/// as the fall begins. Plain words: Bevy's built-in font has no arrow symbols.
fn spawn_drop_hint(mut commands: Commands) {
    commands.spawn((
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
