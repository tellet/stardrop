//! Menus: the title screen, level select, pause and the level-complete
//! screen, plus the navigation shared by all of them.
//!
//! Every screen is a column of items. One item has the focus, shown brighter.
//! Up/down (or W/S, or a gamepad stick) move the focus, Enter/Space/gamepad-A
//! activate it, and Esc/gamepad-B goes back. The mouse works too: hovering
//! moves the focus and clicking activates.

use bevy::prelude::*;

use crate::app_state::AppState;
use crate::attempt::RestartRequested;
use crate::collectibles::CurrentRun;
use crate::level::{ActiveLevel, Levels, SelectedLevel};

const TITLE_SIZE: f32 = 64.0;
const ITEM_SIZE: f32 = 30.0;
const NOTE_SIZE: f32 = 22.0;
const TITLE_COLOR: Color = Color::srgb(0.92, 0.93, 1.0);
const FOCUSED: Color = Color::srgb(1.0, 0.95, 0.8);
const UNFOCUSED: Color = Color::srgba(0.78, 0.78, 0.92, 0.65);
const NOTE_COLOR: Color = Color::srgba(0.8, 0.8, 0.95, 0.75);
/// Menus dim whatever is behind them (the level, while paused).
const BACKDROP: Color = Color::srgba(0.02, 0.015, 0.06, 0.72);
/// Gamepad sticks count as a press past this much.
const STICK_THRESHOLD: f32 = 0.6;

/// What activating a menu item does.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
enum MenuAction {
    /// Go to the level select.
    ChooseLevel,
    /// Open the field journal.
    OpenJournal,
    /// Leave the game.
    Quit,
    /// Play the level with this index.
    StartLevel(usize),
    /// Back to playing (from pause).
    Resume,
    /// Start this level over.
    RetryLevel,
    /// Back to the level select.
    BackToLevels,
    /// Play the next level.
    NextLevel,
}

/// A menu item, by its position in the screen's list.
#[derive(Component, Debug, Clone, Copy)]
struct MenuItem(usize);

/// Which item is focused, and how many there are.
#[derive(Resource, Debug, Default, Clone, Copy)]
struct Focus {
    index: usize,
    count: usize,
}

impl Focus {
    /// Moves the focus by `delta`, wrapping around the ends.
    fn moved(self, delta: isize) -> Self {
        if self.count == 0 {
            return self;
        }
        // `rem_euclid` keeps the result positive when moving up from the top.
        let count = self.count as isize;
        let index = (self.index as isize + delta).rem_euclid(count);
        Self {
            index: index.unsigned_abs(),
            ..self
        }
    }
}

/// The menu screens.
pub(crate) struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Focus>()
            .add_systems(OnEnter(AppState::MainMenu), spawn_main_menu)
            .add_systems(OnEnter(AppState::LevelSelect), spawn_level_select)
            .add_systems(OnEnter(AppState::Paused), spawn_pause_menu)
            .add_systems(OnEnter(AppState::LevelComplete), spawn_level_complete)
            .add_systems(
                Update,
                (
                    (move_focus, hover_focus, show_focus).chain(),
                    activate,
                    go_back,
                )
                    .run_if(not(in_state(AppState::Playing))),
            )
            .add_systems(Update, pause_on_escape.run_if(in_state(AppState::Playing)));
    }
}

/// Builds one screen: a dimmed backdrop, a title, the items, and an optional
/// note under them. Everything is removed when the screen is left.
fn spawn_screen(
    commands: &mut Commands,
    focus: &mut Focus,
    state: AppState,
    title: &str,
    note: Option<String>,
    items: Vec<(String, MenuAction)>,
) {
    *focus = Focus {
        index: 0,
        count: items.len(),
    };
    commands
        .spawn((
            DespawnOnExit(state),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(BACKDROP),
            GlobalZIndex(5),
        ))
        .with_children(|screen| {
            screen.spawn((
                Text::new(title),
                TextFont::from_font_size(TITLE_SIZE),
                TextColor(TITLE_COLOR),
                Node {
                    margin: UiRect::bottom(Val::Px(24.0)),
                    ..default()
                },
            ));
            for (index, (label, action)) in items.into_iter().enumerate() {
                screen.spawn((
                    MenuItem(index),
                    action,
                    Button,
                    Text::new(label),
                    TextFont::from_font_size(ITEM_SIZE),
                    TextColor(if index == 0 { FOCUSED } else { UNFOCUSED }),
                ));
            }
            if let Some(note) = note {
                screen.spawn((
                    Text::new(note),
                    TextFont::from_font_size(NOTE_SIZE),
                    TextColor(NOTE_COLOR),
                    TextLayout::justify(Justify::Center),
                    Node {
                        margin: UiRect::top(Val::Px(24.0)),
                        ..default()
                    },
                ));
            }
        });
}

fn spawn_main_menu(mut commands: Commands, mut focus: ResMut<Focus>) {
    spawn_screen(
        &mut commands,
        &mut focus,
        AppState::MainMenu,
        "Stardrop",
        Some("arrows or mouse to choose, Enter to pick".to_owned()),
        vec![
            ("Descend".to_owned(), MenuAction::ChooseLevel),
            ("Journal".to_owned(), MenuAction::OpenJournal),
            ("Quit".to_owned(), MenuAction::Quit),
        ],
    );
}

fn spawn_level_select(
    mut commands: Commands,
    mut focus: ResMut<Focus>,
    levels: Option<Res<Levels>>,
) {
    let items = levels.map_or_else(Vec::new, |levels| {
        levels
            .levels
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                (
                    format!("{}. {}", index + 1, entry.name),
                    MenuAction::StartLevel(index),
                )
            })
            .collect()
    });
    let note = items
        .is_empty()
        .then(|| "the level list could not be read".to_owned());
    spawn_screen(
        &mut commands,
        &mut focus,
        AppState::LevelSelect,
        "Choose a fall",
        note,
        items,
    );
}

fn spawn_pause_menu(mut commands: Commands, mut focus: ResMut<Focus>) {
    spawn_screen(
        &mut commands,
        &mut focus,
        AppState::Paused,
        "Paused",
        None,
        vec![
            ("Resume".to_owned(), MenuAction::Resume),
            ("Retry".to_owned(), MenuAction::RetryLevel),
            ("Journal".to_owned(), MenuAction::OpenJournal),
            ("Level select".to_owned(), MenuAction::BackToLevels),
        ],
    );
}

fn spawn_level_complete(
    mut commands: Commands,
    mut focus: ResMut<Focus>,
    run: Res<CurrentRun>,
    level: Option<Res<ActiveLevel>>,
    levels: Option<Res<Levels>>,
    selected: Res<SelectedLevel>,
) {
    let total_stardust = level.map_or(0, |level| level.stardust.len());
    let note = format!(
        "time {:.1} s\nstardust {}/{}",
        run.elapsed,
        run.stardust.count(),
        total_stardust
    );
    let has_next = levels.is_some_and(|levels| selected.0 + 1 < levels.levels.len());
    let mut items = vec![("Retry".to_owned(), MenuAction::RetryLevel)];
    if has_next {
        items.insert(0, ("Next level".to_owned(), MenuAction::NextLevel));
    }
    items.push(("Level select".to_owned(), MenuAction::BackToLevels));
    spawn_screen(
        &mut commands,
        &mut focus,
        AppState::LevelComplete,
        "Landed",
        Some(note),
        items,
    );
}

/// Keyboard and gamepad move the focus up and down.
fn move_focus(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut focus: ResMut<Focus>,
    mut stick_held: Local<bool>,
) {
    let mut delta = 0_isize;
    if keys.any_just_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) {
        delta += 1;
    }
    if keys.any_just_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) {
        delta -= 1;
    }
    // A stick counts once per push, not once per frame it's held.
    let stick = gamepads.iter().next().map_or(0.0, |pad| pad.left_stick().y);
    if stick.abs() > STICK_THRESHOLD {
        if !*stick_held {
            delta -= if stick > 0.0 { 1 } else { -1 };
        }
        *stick_held = true;
    } else {
        *stick_held = false;
    }
    if delta != 0 {
        let moved = focus.moved(delta);
        *focus = moved;
    }
}

/// Hovering with the mouse moves the focus to that item.
fn hover_focus(
    hovered: Query<(&Interaction, &MenuItem), Changed<Interaction>>,
    mut focus: ResMut<Focus>,
) {
    for (interaction, item) in &hovered {
        if matches!(interaction, Interaction::Hovered | Interaction::Pressed) {
            focus.index = item.0;
        }
    }
}

/// Brightens the focused item. Only runs when the focus actually changes.
fn show_focus(focus: Res<Focus>, mut items: Query<(&MenuItem, &mut TextColor)>) {
    if !focus.is_changed() {
        return;
    }
    for (item, mut color) in &mut items {
        let wanted = if item.0 == focus.index {
            FOCUSED
        } else {
            UNFOCUSED
        };
        color.set_if_neq(TextColor(wanted));
    }
}

/// Enter, Space, gamepad A or a mouse click run the focused item.
fn activate(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    clicks: Query<(&Interaction, &MenuAction), Changed<Interaction>>,
    items: Query<(&MenuItem, &MenuAction)>,
    focus: Res<Focus>,
    mut next_state: ResMut<NextState<AppState>>,
    mut selected: ResMut<SelectedLevel>,
    mut restart: MessageWriter<RestartRequested>,
    mut exit: MessageWriter<AppExit>,
) {
    let clicked = clicks
        .iter()
        .find(|(interaction, _)| matches!(interaction, Interaction::Pressed))
        .map(|(_, action)| *action);
    let confirmed = keys.any_just_pressed([KeyCode::Enter, KeyCode::Space])
        || gamepads
            .iter()
            .any(|pad| pad.just_pressed(GamepadButton::South));
    let action = clicked.or_else(|| {
        confirmed
            .then(|| {
                items
                    .iter()
                    .find(|(item, _)| item.0 == focus.index)
                    .map(|(_, action)| *action)
            })
            .flatten()
    });
    let Some(action) = action else {
        return;
    };
    match action {
        MenuAction::ChooseLevel | MenuAction::BackToLevels => {
            next_state.set(AppState::LevelSelect);
        }
        MenuAction::OpenJournal => next_state.set(AppState::Journal),
        MenuAction::Quit => {
            exit.write(AppExit::Success);
        }
        MenuAction::StartLevel(index) => {
            *selected = SelectedLevel(index);
            next_state.set(AppState::Playing);
        }
        MenuAction::Resume => next_state.set(AppState::Playing),
        MenuAction::RetryLevel => {
            // The same path as pressing R in the level.
            restart.write(RestartRequested);
            next_state.set(AppState::Playing);
        }
        MenuAction::NextLevel => {
            selected.0 += 1;
            next_state.set(AppState::Playing);
        }
    }
}

/// Esc (or gamepad B) leaves the current screen.
fn go_back(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let back = keys.just_pressed(KeyCode::Escape)
        || gamepads
            .iter()
            .any(|pad| pad.just_pressed(GamepadButton::East));
    if !back {
        return;
    }
    match state.get() {
        AppState::LevelSelect | AppState::Journal => next_state.set(AppState::MainMenu),
        AppState::Paused => next_state.set(AppState::Playing),
        AppState::LevelComplete => next_state.set(AppState::LevelSelect),
        AppState::MainMenu | AppState::Playing => {}
    }
}

/// Esc during play opens the pause menu.
fn pause_on_escape(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let pause = keys.just_pressed(KeyCode::Escape)
        || gamepads
            .iter()
            .any(|pad| pad.just_pressed(GamepadButton::Start));
    if pause {
        next_state.set(AppState::Paused);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_wraps_around_the_ends() {
        let focus = Focus { index: 0, count: 3 };
        assert_eq!(focus.moved(1).index, 1);
        assert_eq!(
            focus.moved(-1).index,
            2,
            "up from the top wraps to the bottom"
        );
        assert_eq!(focus.moved(3).index, 0);
        // An empty screen can't move anywhere.
        assert_eq!(Focus::default().moved(1).index, 0);
    }
}
