//! On-screen messages. For now: what's wrong with the level file, if anything.

use bevy::prelude::*;

use crate::level::LevelProblem;

/// Soft coral: clearly a problem, but calm, like everything else.
const PROBLEM_COLOR: Color = Color::srgb(1.0, 0.72, 0.62);
const PROBLEM_FONT_SIZE: f32 = 22.0;

/// The text node that shows level problems.
#[derive(Component)]
struct ProblemText;

/// On-screen messages.
pub(crate) struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_problem_text).add_systems(
            Update,
            // Only when the problem changes, never every frame.
            show_problem.run_if(resource_changed::<LevelProblem>),
        );
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
