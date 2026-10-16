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
use crate::audio::Interface;
use crate::collectibles::CurrentRun;
use crate::level::{ActiveLevel, Levels, SelectedLevel};
use crate::save::Progress;
use crate::scoring::Stars;
use crate::settings::{Settings, next_volume};

const TITLE_SIZE: f32 = 64.0;
const ITEM_SIZE: f32 = 30.0;
const NOTE_SIZE: f32 = 22.0;
const TITLE_COLOR: Color = Color::srgb(0.92, 0.93, 1.0);
const FOCUSED: Color = Color::srgb(1.0, 0.95, 0.8);
const UNFOCUSED: Color = Color::srgba(0.78, 0.78, 0.92, 0.65);
const NOTE_COLOR: Color = Color::srgba(0.8, 0.8, 0.95, 0.75);
/// Menus dim whatever is behind them (the level, while paused).
const BACKDROP: Color = Color::srgba(0.02, 0.015, 0.06, 0.72);
/// How big a star is on the finish screen, and how far apart they sit.
const STAR_SIZE: f32 = 56.0;
const STAR_GAP: f32 = 10.0;
/// Earned stars are warm and bright; the rest are barely there.
const STAR_EARNED: Color = Color::linear_rgba(2.6, 2.0, 0.9, 1.0);
const STAR_MISSING: Color = Color::srgba(0.45, 0.45, 0.58, 0.35);
/// Every row is this wide, so the settings' values line up in a column.
const ROW_WIDTH: f32 = 440.0;
/// How far the focused row slides in from the others, and how fast the slide
/// catches up (per second).
const SLIDE_DISTANCE: f32 = 14.0;
const SLIDE_RATE: f32 = 18.0;
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
    /// A level that hasn't been unlocked yet: does nothing.
    Locked,
    /// Open the settings screen.
    OpenSettings,
    /// Watch the landing cartoon.
    WatchEnding,
    /// Change one setting.
    Change(Setting),
}

/// The settings a row can change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Setting {
    MasterVolume,
    MusicVolume,
    SfxVolume,
    AmbienceVolume,
    Fullscreen,
    Quality,
    ReduceMotion,
}

/// "on" or "off", for a setting row.
fn on_off(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

/// Where the first row of the level-complete screen leads. Finishing the
/// last level leads to the cartoon; anything else carries on as usual.
fn after_landing(all_done: bool, has_next: bool) -> MenuAction {
    if has_next {
        MenuAction::NextLevel
    } else if all_done {
        MenuAction::WatchEnding
    } else {
        MenuAction::BackToLevels
    }
}

/// Three slots, filled in as the stars are earned: `*` earned, `-` not.
fn star_marks(stars: Stars) -> String {
    [stars.landed, stars.all_stardust, stars.phrase]
        .map(|earned| if earned { '*' } else { '-' })
        .iter()
        .collect()
}

/// The root of a menu screen.
#[derive(Component)]
struct MenuScreen;

/// A menu item, by its position in the screen's list.
#[derive(Component, Debug, Clone, Copy)]
struct MenuItem(usize);

/// How far a row has slid towards the focus, in pixels. Eased, so the focus
/// glides between rows instead of jumping.
#[derive(Component, Debug, Default, Clone, Copy)]
struct Slide(f32);

/// One row of a screen: what it says, what it shows on the right (settings
/// rows only), and what it does.
struct Row {
    label: String,
    /// Shown in a second column, right-aligned, so values line up.
    value: Option<String>,
    action: MenuAction,
}

impl Row {
    fn new(label: &str, action: MenuAction) -> Self {
        Self {
            label: label.to_owned(),
            value: None,
            action,
        }
    }

    fn with_value(label: &str, value: String, action: MenuAction) -> Self {
        Self {
            label: label.to_owned(),
            value: Some(value),
            action,
        }
    }
}

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
        keep_the_main_menu_current(app);
        app.init_resource::<Focus>()
            .add_systems(OnEnter(AppState::MainMenu), spawn_main_menu)
            .add_systems(OnEnter(AppState::LevelSelect), spawn_level_select)
            .add_systems(OnEnter(AppState::Paused), spawn_pause_menu)
            .add_systems(OnEnter(AppState::Settings), spawn_settings_menu)
            // Changing a setting redraws the rows, keeping the focus put.
            .add_systems(
                Update,
                rebuild_settings
                    .run_if(in_state(AppState::Settings).and_then(resource_changed::<Settings>)),
            )
            .add_systems(OnEnter(AppState::LevelComplete), spawn_level_complete)
            .add_systems(
                Update,
                (
                    (move_focus, hover_focus, show_focus, slide_focus).chain(),
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
/// What a screen shows above its items: the stars earned, and the facts
/// behind them.
struct Summary {
    stars: Stars,
    /// Label and value, one per line, laid out in two columns.
    facts: Vec<(String, String)>,
}

fn spawn_screen(
    commands: &mut Commands,
    focus: &mut Focus,
    state: AppState,
    title: &str,
    note: Option<String>,
    items: Vec<Row>,
) {
    spawn_screen_with(commands, focus, state, title, note, items, None, None);
}

#[expect(clippy::too_many_arguments, reason = "one screen, one call")]
fn spawn_screen_with(
    commands: &mut Commands,
    focus: &mut Focus,
    state: AppState,
    title: &str,
    note: Option<String>,
    items: Vec<Row>,
    summary: Option<Summary>,
    star_art: Option<&Handle<Image>>,
) {
    *focus = Focus {
        index: 0,
        count: items.len(),
    };
    // Every screen says where Esc goes, because a key nobody mentions is a
    // key nobody presses.
    let note = match (note, back_from(state)) {
        (Some(note), Some((_, where_to))) => Some(format!("{note}\nEsc goes back to {where_to}")),
        (None, Some((_, where_to))) => Some(format!("Esc goes back to {where_to}")),
        (note, None) => note,
    };
    commands
        .spawn((
            MenuScreen,
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
            if let Some(summary) = summary {
                // Three stars, earned ones lit, then the facts behind them.
                screen
                    .spawn(Node {
                        column_gap: Val::Px(STAR_GAP),
                        margin: UiRect::bottom(Val::Px(18.0)),
                        ..default()
                    })
                    .with_children(|row| {
                        for earned in [
                            summary.stars.landed,
                            summary.stars.all_stardust,
                            summary.stars.phrase,
                        ] {
                            let mut star = row.spawn(Node {
                                width: Val::Px(STAR_SIZE),
                                height: Val::Px(STAR_SIZE),
                                ..default()
                            });
                            if let Some(art) = star_art {
                                star.insert(ImageNode::new(art.clone()).with_color(if earned {
                                    STAR_EARNED
                                } else {
                                    STAR_MISSING
                                }));
                            }
                        }
                    });
                for (label, value) in summary.facts {
                    screen
                        .spawn(Node {
                            width: Val::Px(ROW_WIDTH * 0.8),
                            justify_content: JustifyContent::SpaceBetween,
                            ..default()
                        })
                        .with_children(|line| {
                            line.spawn((
                                Text::new(label),
                                TextFont::from_font_size(NOTE_SIZE),
                                TextColor(NOTE_COLOR),
                            ));
                            line.spawn((
                                Text::new(value),
                                TextFont::from_font_size(NOTE_SIZE),
                                TextColor(TITLE_COLOR),
                            ));
                        });
                }
                // Space before the choices.
                screen.spawn(Node {
                    height: Val::Px(16.0),
                    ..default()
                });
            }
            for (index, row) in items.into_iter().enumerate() {
                let color = TextColor(if index == 0 { FOCUSED } else { UNFOCUSED });
                // A row is a fixed-width line so that every value column lands
                // in the same place, whatever the labels say.
                screen
                    .spawn((
                        MenuItem(index),
                        Slide::default(),
                        row.action,
                        Button,
                        Node {
                            width: Val::Px(ROW_WIDTH),
                            justify_content: if row.value.is_some() {
                                JustifyContent::SpaceBetween
                            } else {
                                JustifyContent::Center
                            },
                            ..default()
                        },
                    ))
                    .with_children(|line| {
                        line.spawn((
                            Text::new(row.label),
                            TextFont::from_font_size(ITEM_SIZE),
                            color,
                        ));
                        if let Some(value) = row.value {
                            line.spawn((
                                Text::new(value),
                                TextFont::from_font_size(ITEM_SIZE),
                                color,
                            ));
                        }
                    });
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

fn spawn_main_menu(
    mut commands: Commands,
    mut focus: ResMut<Focus>,
    levels: Option<Res<Levels>>,
    progress: Res<Progress>,
) {
    let mut items = vec![
        Row::new("Descend", MenuAction::ChooseLevel),
        Row::new("Journal", MenuAction::OpenJournal),
    ];
    // Once every level is done, the cartoon can be watched again.
    if progress.finished_everything(&level_order(levels.as_deref())) {
        items.insert(1, Row::new("The landing", MenuAction::WatchEnding));
    }
    items.push(Row::new("Settings", MenuAction::OpenSettings));
    items.push(Row::new("Quit", MenuAction::Quit));
    spawn_screen(
        &mut commands,
        &mut focus,
        AppState::MainMenu,
        "Stardrop",
        Some("arrows or mouse to choose, Enter to pick".to_owned()),
        items,
    );
}

/// The level files in the order they are played, for asking `Progress` what
/// has been finished.
fn level_order(levels: Option<&Levels>) -> Vec<String> {
    levels.map_or_else(Vec::new, |levels| {
        levels
            .levels
            .iter()
            .map(|entry| entry.file.clone())
            .collect()
    })
}

fn spawn_level_select(
    mut commands: Commands,
    mut focus: ResMut<Focus>,
    levels: Option<Res<Levels>>,
    progress: Res<Progress>,
) {
    let items = levels.map_or_else(Vec::new, |levels| {
        let order: Vec<String> = levels
            .levels
            .iter()
            .map(|entry| entry.file.clone())
            .collect();
        levels
            .levels
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                if progress.is_unlocked(index, &order) {
                    let record = progress.level(&entry.file);
                    Row::with_value(
                        &format!("{}. {}", index + 1, entry.name),
                        star_marks(record.stars()),
                        MenuAction::StartLevel(index),
                    )
                } else {
                    Row::with_value(
                        &format!("{}. locked", index + 1),
                        star_marks(Stars::default()),
                        MenuAction::Locked,
                    )
                }
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
            Row::new("Resume", MenuAction::Resume),
            Row::new("Retry", MenuAction::RetryLevel),
            Row::new("Journal", MenuAction::OpenJournal),
            Row::new("Settings", MenuAction::OpenSettings),
            Row::new("Level select", MenuAction::BackToLevels),
        ],
    );
}

fn spawn_settings_menu(mut commands: Commands, mut focus: ResMut<Focus>, settings: Res<Settings>) {
    let keep = focus.index;
    spawn_screen(
        &mut commands,
        &mut focus,
        AppState::Settings,
        "Settings",
        Some("Enter changes a setting".to_owned()),
        vec![
            Row::with_value(
                "Master volume",
                format!("{}%", settings.master_volume),
                MenuAction::Change(Setting::MasterVolume),
            ),
            Row::with_value(
                "Music",
                format!("{}%", settings.music_volume),
                MenuAction::Change(Setting::MusicVolume),
            ),
            Row::with_value(
                "Sound effects",
                format!("{}%", settings.sfx_volume),
                MenuAction::Change(Setting::SfxVolume),
            ),
            Row::with_value(
                "Ambience",
                format!("{}%", settings.ambience_volume),
                MenuAction::Change(Setting::AmbienceVolume),
            ),
            Row::with_value(
                "Fullscreen",
                on_off(settings.fullscreen).to_owned(),
                MenuAction::Change(Setting::Fullscreen),
            ),
            Row::with_value(
                "Graphics",
                settings.quality.label().to_owned(),
                MenuAction::Change(Setting::Quality),
            ),
            Row::with_value(
                "Reduce motion",
                on_off(settings.reduce_motion).to_owned(),
                MenuAction::Change(Setting::ReduceMotion),
            ),
        ],
    );
    focus.index = keep.min(focus.count.saturating_sub(1));
}

/// The level list is an asset, so it lands a frame or more after the main
/// menu is first built, and whether every level is finished cannot be known
/// until it does. Redraw the menu when it arrives.
fn keep_the_main_menu_current(app: &mut App) {
    app.add_systems(
        Update,
        rebuild_main_menu.run_if(in_state(AppState::MainMenu).and_then(resource_added::<Levels>)),
    );
}

/// Redraws the main menu, keeping the focused row.
fn rebuild_main_menu(
    mut commands: Commands,
    focus: ResMut<Focus>,
    levels: Option<Res<Levels>>,
    progress: Res<Progress>,
    screens: Query<Entity, With<MenuScreen>>,
) {
    for screen in &screens {
        commands.entity(screen).despawn();
    }
    spawn_main_menu(commands, focus, levels, progress);
}

/// Redraws the settings rows after a change, keeping the focused row.
fn rebuild_settings(
    mut commands: Commands,
    focus: ResMut<Focus>,
    settings: Res<Settings>,
    screens: Query<Entity, With<MenuScreen>>,
) {
    for screen in &screens {
        commands.entity(screen).despawn();
    }
    spawn_settings_menu(commands, focus, settings);
}

fn spawn_level_complete(
    mut commands: Commands,
    mut focus: ResMut<Focus>,
    star_art: Option<Res<crate::ui::StarArt>>,
    run: Res<CurrentRun>,
    level: Option<Res<ActiveLevel>>,
    levels: Option<Res<Levels>>,
    selected: Res<SelectedLevel>,
    progress: Res<Progress>,
) {
    let total_stardust = level.as_ref().map_or(0, |level| level.stardust.len());
    let phrase = level.map_or(0, |level| level.phrase);
    let earned = Stars::earned(run.0, total_stardust, phrase);
    let record = levels
        .as_ref()
        .and_then(|levels| levels.entry(*selected))
        .map(|entry| progress.level(&entry.file))
        .unwrap_or_default();
    let best = record
        .best_time
        .map_or_else(|| "-".to_owned(), |best| format!("{best:.1} s"));
    let summary = Summary {
        stars: earned,
        facts: vec![
            (
                "time".to_owned(),
                format!("{:.1} s   (best {best})", run.elapsed),
            ),
            (
                "stardust".to_owned(),
                format!("{}/{total_stardust}", run.stardust.count()),
            ),
            (
                "phrase".to_owned(),
                format!("{}/{phrase}", run.touches.min(u32::from(phrase))),
            ),
        ],
    };
    let order = level_order(levels.as_deref());
    let has_next = levels.is_some_and(|levels| selected.0 + 1 < levels.levels.len());
    let onwards = after_landing(progress.finished_everything(&order), has_next);
    let mut items = vec![Row::new("Retry", MenuAction::RetryLevel)];
    if onwards != MenuAction::BackToLevels {
        let label = if onwards == MenuAction::WatchEnding {
            "Onwards"
        } else {
            "Next level"
        };
        items.insert(0, Row::new(label, onwards));
    }
    items.push(Row::new("Level select", MenuAction::BackToLevels));
    let star_art = star_art.map(|art| art.0.clone());
    spawn_screen_with(
        &mut commands,
        &mut focus,
        AppState::LevelComplete,
        "Landed",
        None,
        items,
        Some(summary),
        star_art.as_ref(),
    );
}

/// Keyboard and gamepad move the focus up and down.
fn move_focus(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut focus: ResMut<Focus>,
    mut stick_held: Local<bool>,
    mut sound: MessageWriter<Interface>,
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
        sound.write(Interface::Moved(moved.index));
    }
}

/// Hovering with the mouse moves the focus to that item.
fn hover_focus(
    hovered: Query<(&Interaction, &MenuItem), Changed<Interaction>>,
    mut focus: ResMut<Focus>,
    mut sound: MessageWriter<Interface>,
) {
    for (interaction, item) in &hovered {
        if matches!(interaction, Interaction::Hovered | Interaction::Pressed)
            && focus.index != item.0
        {
            focus.index = item.0;
            sound.write(Interface::Moved(item.0));
        }
    }
}

/// Brightens the focused row. Only runs when the focus actually changes.
fn show_focus(
    focus: Res<Focus>,
    rows: Query<(&MenuItem, &Children)>,
    mut colors: Query<&mut TextColor>,
) {
    if !focus.is_changed() {
        return;
    }
    for (item, children) in &rows {
        let wanted = if item.0 == focus.index {
            FOCUSED
        } else {
            UNFOCUSED
        };
        for child in children {
            if let Ok(mut color) = colors.get_mut(*child) {
                color.set_if_neq(TextColor(wanted));
            }
        }
    }
}

/// Glides the focused row a little out of the column, so the focus moves
/// instead of jumping between rows.
fn slide_focus(
    time: Res<Time>,
    focus: Res<Focus>,
    settings: Res<Settings>,
    mut rows: Query<(&MenuItem, &mut Slide, &mut Node)>,
) {
    let delta = time.delta_secs();
    for (item, mut slide, mut node) in &mut rows {
        let target = if item.0 == focus.index {
            SLIDE_DISTANCE * settings.motion_scale()
        } else {
            0.0
        };
        if (slide.0 - target).abs() < 0.2 {
            continue;
        }
        slide.0 = crate::astronaut::approach(slide.0, target, SLIDE_RATE, delta);
        node.left = Val::Px(slide.0);
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
    mut settings: ResMut<Settings>,
    mut exit: MessageWriter<AppExit>,
    mut sound: MessageWriter<Interface>,
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
    // A locked level is the one thing that answers with nothing at all.
    if action != MenuAction::Locked {
        sound.write(Interface::Chose);
    }
    match action {
        MenuAction::ChooseLevel | MenuAction::BackToLevels => {
            next_state.set(AppState::LevelSelect);
        }
        MenuAction::OpenJournal => next_state.set(AppState::Journal),
        MenuAction::OpenSettings => next_state.set(AppState::Settings),
        MenuAction::WatchEnding => next_state.set(AppState::Ending),
        MenuAction::Change(setting) => change_setting(setting, &mut settings),
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
        // Locked levels stay put until the one before them is finished.
        MenuAction::Locked => {}
    }
}

/// Activating a settings row steps it to its next value.
fn change_setting(setting: Setting, settings: &mut Settings) {
    match setting {
        Setting::MasterVolume => settings.master_volume = next_volume(settings.master_volume),
        Setting::MusicVolume => settings.music_volume = next_volume(settings.music_volume),
        Setting::SfxVolume => settings.sfx_volume = next_volume(settings.sfx_volume),
        Setting::AmbienceVolume => settings.ambience_volume = next_volume(settings.ambience_volume),
        Setting::Fullscreen => settings.fullscreen = !settings.fullscreen,
        Setting::Quality => settings.quality = settings.quality.next(),
        Setting::ReduceMotion => settings.reduce_motion = !settings.reduce_motion,
    }
}

/// Esc (or gamepad B) leaves the current screen.
/// Where Esc leads from each screen, and what to call that in a hint.
/// `go_back` and the hint under every screen both read this, so what the
/// screen says and what the key does can never drift apart.
fn back_from(state: AppState) -> Option<(AppState, &'static str)> {
    match state {
        AppState::LevelSelect | AppState::Journal | AppState::Settings => {
            Some((AppState::MainMenu, "the main menu"))
        }
        AppState::Paused => Some((AppState::Playing, "the fall")),
        AppState::LevelComplete => Some((AppState::LevelSelect, "the level list")),
        // Nowhere left to go back to. The cartoon is left by any key at all,
        // which `ending.rs` handles itself.
        AppState::MainMenu | AppState::Playing | AppState::Ending => None,
    }
}

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
    if let Some((destination, _)) = back_from(*state.get()) {
        next_state.set(destination);
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
    use bevy::state::app::StatesPlugin;

    use super::*;
    use crate::level::{LevelEntry, LevelList};
    use crate::scoring::Stars;

    /// Landing the last level leads to the cartoon. Replaying an earlier one
    /// after the game is finished stays ordinary.
    #[test]
    fn the_last_landing_leads_to_the_cartoon() {
        assert_eq!(after_landing(true, false), MenuAction::WatchEnding);
        assert_eq!(after_landing(false, true), MenuAction::NextLevel);
        assert_eq!(after_landing(false, false), MenuAction::BackToLevels);
        assert_eq!(after_landing(true, true), MenuAction::NextLevel);
    }

    /// The level list is an asset, so it arrives a frame or more after the
    /// main menu is first built. The rewatch row has to appear when it does,
    /// or a player who finished the game and relaunched never sees it.
    #[test]
    fn the_landing_row_appears_when_the_level_list_turns_up() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<AppState>()
            .init_resource::<Focus>()
            .init_resource::<Progress>();
        keep_the_main_menu_current(&mut app);
        app.add_systems(OnEnter(AppState::MainMenu), spawn_main_menu);
        app.update();
        assert_eq!(
            rows_leading_to(&mut app, MenuAction::WatchEnding),
            0,
            "nothing is finished yet"
        );
        // The save says the one level is done, and the list arrives.
        app.world_mut().resource_mut::<Progress>().record_landing(
            "only.ron",
            1.0,
            0,
            Stars {
                landed: true,
                ..Stars::default()
            },
        );
        app.insert_resource(Levels(LevelList {
            levels: vec![LevelEntry {
                name: "Only".to_owned(),
                file: "only.ron".to_owned(),
            }],
        }));
        app.update();
        assert_eq!(
            rows_leading_to(&mut app, MenuAction::WatchEnding),
            1,
            "the landing is missing from the menu"
        );
    }

    fn rows_leading_to(app: &mut App, action: MenuAction) -> usize {
        app.world_mut()
            .query::<&MenuAction>()
            .iter(app.world())
            .filter(|row| **row == action)
            .count()
    }

    /// Esc must do something on every screen that isn't the top one or the
    /// fall itself, and the hint must name where it goes.
    #[test]
    fn every_screen_says_where_escape_goes() {
        for state in [
            AppState::LevelSelect,
            AppState::Journal,
            AppState::Settings,
            AppState::Paused,
            AppState::LevelComplete,
        ] {
            let (destination, where_to) =
                back_from(state).unwrap_or_else(|| panic!("{state:?} has no way back"));
            assert_ne!(destination, state, "{state:?} goes back to itself");
            assert!(!where_to.is_empty());
        }
        // The main menu and the fall have nowhere to go back to: on the fall
        // Esc pauses instead (`pause_on_escape`).
        assert!(back_from(AppState::MainMenu).is_none());
        assert!(back_from(AppState::Playing).is_none());
    }

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
