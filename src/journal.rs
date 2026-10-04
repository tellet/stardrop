//! The field journal: a page per object type, filled in the first time the
//! astronaut touches one.
//!
//! The pages are laid out in a grid in an unused corner of the world, and the
//! journal simply points the camera at them. That way each page shows the
//! object's real shape, drawn by the same code as in play, instead of a
//! separate set of pictures. Undiscovered types show as dark silhouettes.

use bevy::prelude::*;

use crate::app_state::AppState;

use crate::config::GameConfig;
use crate::objects::{ObjectAssets, ObjectKind, Placement, spawn_preview};
use crate::save::Progress;

/// Far from any level, so the journal and the level never share the screen.
/// The camera module points the camera here while the journal is open.
pub(crate) const JOURNAL_ORIGIN: Vec2 = Vec2::new(0.0, 40_000.0);
/// Grid of pages.
const COLUMNS: usize = 3;
const CELL: Vec2 = Vec2::new(320.0, 380.0);
/// Every object type, in the order the game introduces them (Section 4).
const PAGES: [ObjectKind; 9] = [
    ObjectKind::CloudPuff,
    ObjectKind::WindRibbon,
    ObjectKind::Crystal,
    ObjectKind::StaticCloud,
    ObjectKind::Bubble,
    ObjectKind::CometStone,
    ObjectKind::SporeFlower,
    ObjectKind::GravityOrb,
    ObjectKind::TwinPortals,
];

const TITLE_SIZE: f32 = 40.0;
const NAME_SIZE: f32 = 30.0;
const NOTE_SIZE: f32 = 24.0;
const HINT_SIZE: f32 = 20.0;
const TITLE_COLOR: Color = Color::srgb(0.92, 0.93, 1.0);
const NAME_COLOR: Color = Color::srgb(1.0, 0.95, 0.8);
const NOTE_COLOR: Color = Color::srgba(0.85, 0.85, 1.0, 0.85);
const HINT_COLOR: Color = Color::srgba(0.8, 0.8, 0.95, 0.55);
/// Undiscovered pages are drawn as flat shadows.
const SILHOUETTE: Color = Color::srgb(0.10, 0.09, 0.16);
/// The ring around the page being read.
const SELECTION: Color = Color::srgba(1.0, 0.95, 0.8, 0.5);
/// How long the "new entry" notice stays on screen, in seconds.
const NOTICE_SECONDS: f32 = 2.5;

/// Sent the first time an object type is ever touched (across all sessions).
#[derive(Message, Debug, Clone, Copy)]
pub(crate) struct ObjectDiscovered(pub ObjectKind);

/// Which page is being read.
#[derive(Resource, Debug, Default, Clone, Copy)]
struct Page(usize);

/// A page's preview shapes.
#[derive(Component)]
struct PageEntity;

/// The name and note of the page being read.
#[derive(Component)]
struct PageText;

/// The "new entry" notice shown during play.
#[derive(Component)]
struct NewEntryNotice(Timer);

/// The field journal.
pub(crate) struct JournalPlugin;

impl Plugin for JournalPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ObjectDiscovered>()
            .init_resource::<Page>()
            .add_systems(
                OnEnter(AppState::Journal),
                (open_journal, show_page).chain(),
            )
            .add_systems(
                Update,
                (turn_pages, show_page)
                    .chain()
                    .run_if(in_state(AppState::Journal)),
            )
            .add_systems(
                Update,
                (
                    open_on_key.run_if(not(in_state(AppState::Journal))),
                    announce_discovery.run_if(on_message::<ObjectDiscovered>),
                    fade_notice,
                ),
            );
    }
}

/// What the player is told about an object, once it's been discovered. Notes
/// are the astronaut's guesses, never explanations (Section 3.6).
fn page_text(kind: ObjectKind) -> (&'static str, &'static str) {
    match kind {
        ObjectKind::CloudPuff => (
            "Cloud puff",
            "Soft as a breath. I push off it and go up, not through.",
        ),
        ObjectKind::WindRibbon => (
            "Wind ribbon",
            "A river with no water. It carries me where it points.",
        ),
        ObjectKind::Crystal => (
            "Crystal",
            "Bright, and rude. It sends me back the way I came.",
        ),
        ObjectKind::SporeFlower => ("Spore flower", "Pretty. Then left is right for a while."),
        ObjectKind::CometStone => (
            "Comet stone",
            "Heavy little thing. It throws me down faster than falling.",
        ),
        ObjectKind::Bubble => ("Bubble", "Slow, cozy, never in a hurry."),
        ObjectKind::GravityOrb => ("Gravity orb", "It wants me. Swing wide and it lets go."),
        ObjectKind::TwinPortals => (
            "Twin portals",
            "Two mouths, one tunnel. In here, out there.",
        ),
        ObjectKind::StaticCloud => ("Static cloud", "Dark, and it crackles. Best not."),
    }
}

/// A small version of each object, for its page.
fn preview_of(kind: ObjectKind, at: Vec2) -> Placement {
    match kind {
        ObjectKind::CloudPuff => Placement::CloudPuff {
            center: at,
            radius: 70.0,
        },
        ObjectKind::WindRibbon => Placement::WindRibbon {
            path: vec![at - Vec2::new(90.0, 40.0), at + Vec2::new(90.0, 40.0)],
            width: 70.0,
        },
        ObjectKind::Crystal => Placement::Crystal {
            center: at,
            size: 60.0,
        },
        ObjectKind::SporeFlower => Placement::SporeFlower {
            center: at,
            radius: 65.0,
        },
        ObjectKind::CometStone => Placement::CometStone {
            center: at,
            radius: 55.0,
        },
        ObjectKind::Bubble => Placement::Bubble {
            center: at,
            radius: 70.0,
        },
        ObjectKind::GravityOrb => Placement::GravityOrb {
            center: at,
            radius: 30.0,
        },
        ObjectKind::TwinPortals => Placement::TwinPortals {
            a: at - Vec2::new(70.0, 0.0),
            b: at + Vec2::new(70.0, 0.0),
            radius: 45.0,
        },
        ObjectKind::StaticCloud => Placement::StaticCloud {
            center: at,
            radius: 70.0,
        },
    }
}

/// Where a page sits in the grid.
fn page_position(index: usize) -> Vec2 {
    // Small counts; the conversions are exact.
    #[expect(clippy::cast_precision_loss, reason = "nine pages")]
    let (column, row) = ((index % COLUMNS) as f32, (index / COLUMNS) as f32);
    #[expect(clippy::cast_precision_loss, reason = "three columns")]
    let middle = (COLUMNS - 1) as f32 / 2.0;
    JOURNAL_ORIGIN + Vec2::new((column - middle) * CELL.x, (1.0 - row) * CELL.y)
}

/// Points the camera at the journal and builds the pages.
fn open_journal(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    assets: Res<ObjectAssets>,
    config: Res<GameConfig>,
    progress: Res<Progress>,
) {
    let silhouette = materials.add(SILHOUETTE);
    for (index, kind) in PAGES.into_iter().enumerate() {
        let at = page_position(index);
        // The same idle motion as in play, so a page looks alive and shows
        // exactly what the player will meet.
        let marker = (
            PageEntity,
            DespawnOnExit(AppState::Journal),
            crate::effects::ContactPulse::default(),
            #[expect(clippy::cast_precision_loss, reason = "nine pages")]
            crate::effects::Idle {
                kind,
                phase: index as f32 * 1.3,
            },
        );
        if progress.has_discovered(kind) {
            spawn_preview(
                &mut commands,
                &assets,
                &mut meshes,
                &config.objects,
                &preview_of(kind, at),
                marker,
            );
        } else {
            // A flat disc stands in for a shape nobody has met yet.
            commands.spawn((
                marker,
                Mesh2d(meshes.add(Circle::new(70.0))),
                MeshMaterial2d(silhouette.clone()),
                Transform::from_translation(at.extend(0.0)),
            ));
        }
    }
    spawn_journal_ui(&mut commands);
}

fn spawn_journal_ui(commands: &mut Commands) {
    commands
        .spawn((
            DespawnOnExit(AppState::Journal),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::all(Val::Px(28.0)),
                ..default()
            },
        ))
        .with_children(|screen| {
            screen.spawn((
                Text::new("Field journal"),
                TextFont::from_font_size(TITLE_SIZE),
                TextColor(TITLE_COLOR),
            ));
            screen
                .spawn((Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(8.0),
                    ..default()
                },))
                .with_children(|bottom| {
                    bottom.spawn((
                        PageText,
                        Text::default(),
                        TextFont::from_font_size(NAME_SIZE),
                        TextColor(NAME_COLOR),
                        TextLayout::justify(Justify::Center),
                    ));
                    bottom.spawn((
                        Text::new("arrows to turn pages, Esc to close"),
                        TextFont::from_font_size(HINT_SIZE),
                        TextColor(HINT_COLOR),
                    ));
                });
        });
}

/// Arrows or a stick move between pages.
fn turn_pages(
    keys: Res<ButtonInput<KeyCode>>,
    mut page: ResMut<Page>,
    mut sound: MessageWriter<crate::audio::Interface>,
) {
    let mut delta = 0_isize;
    if keys.any_just_pressed([KeyCode::ArrowRight, KeyCode::KeyD]) {
        delta += 1;
    }
    if keys.any_just_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]) {
        delta -= 1;
    }
    if keys.any_just_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) {
        delta += COLUMNS as isize;
    }
    if keys.any_just_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) {
        delta -= COLUMNS as isize;
    }
    if delta != 0 {
        let count = PAGES.len() as isize;
        let index = (page.0 as isize + delta).rem_euclid(count);
        page.0 = index.unsigned_abs();
        sound.write(crate::audio::Interface::TurnedPage);
    }
}

/// Writes the current page's name and note, and rings the page being read.
fn show_page(
    page: Res<Page>,
    progress: Res<Progress>,
    mut text: Single<&mut Text, With<PageText>>,
    mut commands: Commands,
    rings: Query<Entity, With<PageRing>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if !page.is_changed() {
        return;
    }
    let kind = PAGES[page.0.min(PAGES.len() - 1)];
    let (name, note) = page_text(kind);
    text.0 = if progress.has_discovered(kind) {
        format!("{name}\n{note}")
    } else {
        "???".to_owned()
    };
    for ring in &rings {
        commands.entity(ring).despawn();
    }
    commands.spawn((
        PageRing,
        DespawnOnExit(AppState::Journal),
        Mesh2d(meshes.add(Annulus::new(130.0, 134.0))),
        MeshMaterial2d(materials.add(SELECTION)),
        Transform::from_translation(page_position(page.0).extend(-0.1)),
    ));
}

/// The ring drawn around the page being read.
#[derive(Component)]
struct PageRing;

/// J opens the journal from the menus and from a paused level.
fn open_on_key(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let opens_here = matches!(
        state.get(),
        AppState::MainMenu | AppState::LevelSelect | AppState::Paused
    );
    if opens_here && keys.just_pressed(KeyCode::KeyJ) {
        next_state.set(AppState::Journal);
    }
}

/// A small notice during play when something new is discovered.
fn announce_discovery(mut discovered: MessageReader<ObjectDiscovered>, mut commands: Commands) {
    let Some(ObjectDiscovered(kind)) = discovered.read().last() else {
        return;
    };
    let (name, _) = page_text(*kind);
    commands.spawn((
        NewEntryNotice(Timer::from_seconds(NOTICE_SECONDS, TimerMode::Once)),
        Text::new(format!("new journal entry: {name}")),
        TextFont::from_font_size(NOTE_SIZE),
        TextColor(NOTE_COLOR),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Percent(6.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
    ));
}

/// Fades the notice out and removes it.
fn fade_notice(
    time: Res<Time>,
    mut commands: Commands,
    mut notices: Query<(Entity, &mut NewEntryNotice, &mut TextColor)>,
) {
    for (entity, mut notice, mut color) in &mut notices {
        if notice.0.tick(time.delta()).just_finished() {
            commands.entity(entity).despawn();
            continue;
        }
        // Hold, then fade over the last second.
        let left = notice.0.remaining_secs();
        color.0.set_alpha(left.min(1.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::PLAYFIELD;

    #[test]
    fn every_object_type_has_a_page() {
        assert_eq!(PAGES.len(), 9);
        for kind in PAGES {
            let (name, note) = page_text(kind);
            assert!(!name.is_empty() && !note.is_empty(), "{kind:?}");
        }
    }

    #[test]
    fn pages_are_laid_out_in_a_grid_around_the_journal() {
        let first = page_position(0);
        let second = page_position(1);
        assert!((second.x - first.x - CELL.x).abs() < 1e-3, "next column");
        let fourth = page_position(3);
        assert!(
            (first.y - fourth.y - CELL.y).abs() < 1e-3,
            "next row, lower"
        );
        // The whole grid fits on screen.
        for index in 0..PAGES.len() {
            let offset = page_position(index) - JOURNAL_ORIGIN;
            assert!(offset.x.abs() < PLAYFIELD.x / 2.0);
            assert!(offset.y.abs() < PLAYFIELD.y / 2.0);
        }
    }
}
