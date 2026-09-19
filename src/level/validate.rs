//! Level validation and its error type.
//!
//! [`check`] looks at a parsed [`LevelFile`] and returns the first rule it
//! breaks. Messages are written for whoever edits the level file, since they
//! appear on screen.

use core::fmt;

use super::data::{FilePoint, LEVEL_HEIGHT, LEVEL_WIDTH, LevelFile, to_world};
use super::scenery::MOTE_RADIUS;
use crate::collision::{Shape, circle_overlaps_rect, circles_overlap};
use crate::config::ASTRONAUT_RADIUS;
use crate::objects::{MAX_RIBBON_POINTS, Placement};
use crate::scoring::MAX_STARDUST;

/// Which thing in the level an error is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    /// The astronaut's start point.
    Start,
    /// The landing beacon.
    Beacon,
    /// A stardust mote, by its position in the list (0-based).
    Stardust(usize),
    /// An object, by its position in the list (0-based).
    Object(usize),
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Humans count from 1, so the 0-based list index is shown +1.
        match self {
            Self::Start => write!(f, "the start"),
            Self::Beacon => write!(f, "the beacon"),
            Self::Stardust(i) => write!(f, "stardust #{}", i + 1),
            Self::Object(i) => write!(f, "object #{}", i + 1),
        }
    }
}

/// Everything that can be wrong with a level file.
#[derive(Debug, Clone, PartialEq)]
pub enum LevelError {
    /// The text isn't valid RON, or doesn't match the level format.
    Syntax {
        /// 1-based line of the problem.
        line: usize,
        /// 1-based column of the problem.
        column: usize,
        /// What the parser expected or found.
        message: String,
    },
    /// The file isn't UTF-8 text.
    NotText,
    /// The level has no name.
    EmptyName,
    /// A number is `NaN` or infinite.
    NotANumber(&'static str),
    /// Something is placed outside the level's edges.
    Outside(Item),
    /// The beacon's width or height isn't positive.
    BadBeaconSize,
    /// The beacon isn't below the start, so it can never be reached by falling.
    BeaconAboveStart,
    /// The target time isn't positive.
    BadTargetTime,
    /// More stardust than a run can track.
    TooMuchStardust(usize),
    /// An object's size is outside its type's range.
    BadObjectSize {
        /// Index of the object in the list.
        object: usize,
        /// Smallest allowed size for its type.
        min: f32,
        /// Largest allowed size for its type.
        max: f32,
    },
    /// A wind ribbon's path needs 2 to `MAX_RIBBON_POINTS` points.
    BadRibbonPath(usize),
    /// A pair of portals is too close together (they would overlap).
    PortalsTooClose(usize),
    /// An object covers something it must not (e.g. the start, or a hazard
    /// on the beacon).
    Overlaps {
        /// Index of the object in the list.
        object: usize,
        /// What it covers.
        item: Item,
    },
}

impl fmt::Display for LevelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax {
                line,
                column,
                message,
            } => write!(f, "line {line}, column {column}: {message}"),
            Self::NotText => write!(f, "the file isn't plain text (UTF-8)"),
            Self::EmptyName => write!(f, "the level needs a name"),
            Self::NotANumber(field) => write!(f, "{field} must be a normal number"),
            Self::Outside(item) => write!(
                f,
                "{item} is outside the level (x must be 0–{LEVEL_WIDTH}, depth 0–{LEVEL_HEIGHT})"
            ),
            Self::BadBeaconSize => write!(f, "the beacon's width and height must be positive"),
            Self::BeaconAboveStart => write!(f, "the beacon must be below the start"),
            Self::BadTargetTime => write!(f, "target_time must be more than 0 seconds"),
            Self::BadObjectSize { object, min, max } => write!(
                f,
                "{}'s size must be between {min} and {max}",
                Item::Object(*object)
            ),
            Self::PortalsTooClose(i) => write!(
                f,
                "{}'s two ends are too close together; leave at least three radii between them",
                Item::Object(*i)
            ),
            Self::BadRibbonPath(i) => write!(
                f,
                "{}'s path needs 2 to {MAX_RIBBON_POINTS} points",
                Item::Object(*i)
            ),
            Self::Overlaps { object, item } => {
                write!(f, "{} overlaps {item}", Item::Object(*object))
            }
            Self::TooMuchStardust(count) => write!(
                f,
                "{count} stardust motes; a level can have at most {MAX_STARDUST}"
            ),
        }
    }
}

/// Marks `LevelError` as a standard Rust error, so it works with `?` and with
/// Bevy's asset system. `Display` above provides the message.
impl std::error::Error for LevelError {}

/// Lets `?` turn a RON parse error into our own error type automatically.
impl From<ron::error::SpannedError> for LevelError {
    fn from(error: ron::error::SpannedError) -> Self {
        Self::Syntax {
            line: error.span.start.line,
            column: error.span.start.col,
            message: error.code.to_string(),
        }
    }
}

/// Returns the first rule the level breaks, or `Ok(())` if it's valid.
///
/// # Errors
/// See [`LevelError`] for every rule.
pub(super) fn check(file: &LevelFile) -> Result<(), LevelError> {
    if file.name.trim().is_empty() {
        return Err(LevelError::EmptyName);
    }
    finite("target_time", file.target_time)?;
    if file.target_time <= 0.0 {
        return Err(LevelError::BadTargetTime);
    }

    inside(file.start, Item::Start)?;
    check_beacon(file)?;
    if file.stardust.len() > MAX_STARDUST {
        return Err(LevelError::TooMuchStardust(file.stardust.len()));
    }
    for (i, &mote) in file.stardust.iter().enumerate() {
        inside(mote, Item::Stardust(i))?;
    }
    check_objects(file)
}

fn check_objects(file: &LevelFile) -> Result<(), LevelError> {
    let start = to_world(file.start);
    let beacon_center = to_world(file.beacon.center);
    let beacon_size = bevy::math::Vec2::new(file.beacon.size.0, file.beacon.size.1);
    for (i, object) in file.objects.iter().enumerate() {
        let placed = object.clone().into_world();
        check_object_shape(&placed, i)?;
        // The astronaut must never start inside anything.
        if placed.outline().overlaps_circle(start, ASTRONAUT_RADIUS) {
            return Err(LevelError::Overlaps {
                object: i,
                item: Item::Start,
            });
        }
        // Hazards must leave the goal and every mote reachable.
        if let Placement::StaticCloud { center, radius } = placed {
            if circle_overlaps_rect(center, radius, beacon_center, beacon_size) {
                return Err(LevelError::Overlaps {
                    object: i,
                    item: Item::Beacon,
                });
            }
            for (m, &mote) in file.stardust.iter().enumerate() {
                if circles_overlap(center, radius, to_world(mote), MOTE_RADIUS) {
                    return Err(LevelError::Overlaps {
                        object: i,
                        item: Item::Stardust(m),
                    });
                }
            }
        }
    }
    Ok(())
}

/// Size and position rules for every object.
fn check_object_shape(placed: &Placement, index: usize) -> Result<(), LevelError> {
    let size = placed.size();
    finite("an object's size", size)?;
    let range = placed.kind().size_range();
    if !range.contains(&size) {
        return Err(LevelError::BadObjectSize {
            object: index,
            min: *range.start(),
            max: *range.end(),
        });
    }
    if let Placement::TwinPortals { a, b, radius } = placed {
        // Overlapping ends would send the astronaut straight back in.
        if a.distance(*b) < radius * 3.0 {
            return Err(LevelError::PortalsTooClose(index));
        }
        for end in [*a, *b] {
            inside(to_file(end), Item::Object(index))?;
        }
        return Ok(());
    }
    if let Placement::WindRibbon { path, .. } = placed {
        if !(2..=MAX_RIBBON_POINTS).contains(&path.len()) {
            return Err(LevelError::BadRibbonPath(index));
        }
        // Every point of the path, not just its middle, must be inside.
        for point in path {
            inside(to_file(*point), Item::Object(index))?;
        }
        return Ok(());
    }
    inside(to_file(placed.center()), Item::Object(index))
}

/// World coordinates back to file coordinates, for the shared "inside" rule.
fn to_file(point: bevy::math::Vec2) -> FilePoint {
    FilePoint(point.x + LEVEL_WIDTH / 2.0, -point.y)
}

fn check_beacon(file: &LevelFile) -> Result<(), LevelError> {
    let (width, height) = file.beacon.size;
    finite("the beacon's size", width)?;
    finite("the beacon's size", height)?;
    if width <= 0.0 || height <= 0.0 {
        return Err(LevelError::BadBeaconSize);
    }
    // The whole pad must fit, not just its center.
    let FilePoint(x, depth) = file.beacon.center;
    let corners = [
        FilePoint(x - width / 2.0, depth - height / 2.0),
        FilePoint(x + width / 2.0, depth + height / 2.0),
    ];
    for corner in corners {
        inside(corner, Item::Beacon)?;
    }
    if depth - height / 2.0 <= file.start.1 {
        return Err(LevelError::BeaconAboveStart);
    }
    Ok(())
}

fn inside(point: FilePoint, item: Item) -> Result<(), LevelError> {
    let FilePoint(x, depth) = point;
    finite("a position", x)?;
    finite("a position", depth)?;
    let within = (0.0..=LEVEL_WIDTH).contains(&x) && (0.0..=LEVEL_HEIGHT).contains(&depth);
    if within {
        Ok(())
    } else {
        Err(LevelError::Outside(item))
    }
}

fn finite(field: &'static str, value: f32) -> Result<(), LevelError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(LevelError::NotANumber(field))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::data::Level;

    const VALID: &str = r#"(
        name: "Test",
        theme: DeepSpace,
        start: (540, 200),
        beacon: (center: (540, 1880), size: (300, 40)),
        stardust: [(200, 1000), (900, 1500)],
        target_time: 8.0,
    )"#;

    /// Parses `VALID` with one piece of text swapped, and returns the error.
    fn error_after(from: &str, to: &str) -> LevelError {
        assert!(VALID.contains(from), "test typo: {from:?} not in VALID");
        match Level::parse(&VALID.replace(from, to)) {
            Ok(_) => panic!("expected an error after replacing {from:?} with {to:?}"),
            Err(error) => error,
        }
    }

    #[test]
    fn valid_level_passes() {
        assert!(Level::parse(VALID).is_ok());
    }

    #[test]
    fn syntax_errors_report_line_and_column() {
        let error = error_after("theme: DeepSpace,", "theme: DeepSpace");
        match error {
            LevelError::Syntax { line, column, .. } => {
                assert_eq!(line, 4);
                assert!(column > 0);
            }
            other => panic!("expected a syntax error, got {other:?}"),
        }
    }

    #[test]
    fn behavior_fields_are_rejected() {
        // Consistency rule: a level can't change how anything behaves.
        let error = error_after("target_time: 8.0,", "target_time: 8.0, gravity: 900,");
        assert!(matches!(error, LevelError::Syntax { .. }));
        assert!(error.to_string().contains("gravity"));
    }

    #[test]
    fn unknown_theme_is_rejected() {
        let error = error_after("DeepSpace", "Underwater");
        assert!(matches!(error, LevelError::Syntax { .. }));
    }

    #[test]
    fn missing_required_field_is_rejected() {
        let error = error_after("target_time: 8.0,", "");
        assert!(error.to_string().contains("target_time"));
    }

    #[test]
    fn empty_name_is_rejected() {
        assert_eq!(error_after(r#""Test""#, r#""  ""#), LevelError::EmptyName);
    }

    #[test]
    fn nan_is_rejected() {
        let error = error_after("target_time: 8.0", "target_time: NaN");
        assert_eq!(error, LevelError::NotANumber("target_time"));
    }

    #[test]
    fn things_outside_the_level_are_rejected() {
        assert_eq!(
            error_after("start: (540, 200)", "start: (1100, 200)"),
            LevelError::Outside(Item::Start)
        );
        assert_eq!(
            error_after("(200, 1000)", "(200, 2000)"),
            LevelError::Outside(Item::Stardust(0))
        );
        assert_eq!(
            error_after("(900, 1500)", "(-5, 1500)"),
            LevelError::Outside(Item::Stardust(1))
        );
    }

    #[test]
    fn beacon_must_fit_inside_the_level() {
        // Center inside, but the pad pokes out past the right edge.
        let error = error_after("center: (540, 1880)", "center: (1000, 1880)");
        assert_eq!(error, LevelError::Outside(Item::Beacon));
    }

    #[test]
    fn beacon_must_have_a_size() {
        assert_eq!(
            error_after("size: (300, 40)", "size: (0, 40)"),
            LevelError::BadBeaconSize
        );
    }

    #[test]
    fn beacon_above_start_is_rejected() {
        let error = error_after("start: (540, 200)", "start: (540, 1870)");
        assert_eq!(error, LevelError::BeaconAboveStart);
    }

    #[test]
    fn target_time_must_be_positive() {
        assert_eq!(
            error_after("target_time: 8.0", "target_time: 0.0"),
            LevelError::BadTargetTime
        );
    }

    #[test]
    fn too_much_stardust_is_rejected() {
        let motes = vec!["(100, 1000)"; MAX_STARDUST + 1].join(", ");
        let error = error_after(
            "stardust: [(200, 1000), (900, 1500)]",
            &format!("stardust: [{motes}]"),
        );
        assert_eq!(error, LevelError::TooMuchStardust(MAX_STARDUST + 1));
    }

    fn with_objects(objects: &str) -> String {
        VALID.replace(
            "target_time: 8.0,",
            &format!("objects: [{objects}], target_time: 8.0,"),
        )
    }

    #[test]
    fn valid_objects_pass() {
        assert!(Level::parse(&with_objects("StaticCloud(center: (300, 800), radius: 80)")).is_ok());
    }

    #[test]
    fn objects_cannot_set_behavior() {
        // Consistency rule: a level can't make one cloud stronger than another.
        let text = with_objects("StaticCloud(center: (300, 800), radius: 80, hitbox_scale: 0.1)");
        let error = Level::parse(&text).unwrap_err();
        assert!(error.to_string().contains("hitbox_scale"), "{error}");
    }

    #[test]
    fn object_rules_are_enforced() {
        let cases = [
            (
                "StaticCloud(center: (300, 2500), radius: 80)",
                LevelError::Outside(Item::Object(0)),
            ),
            (
                "StaticCloud(center: (300, 800), radius: 5)",
                LevelError::BadObjectSize {
                    object: 0,
                    min: 20.0,
                    max: 300.0,
                },
            ),
            (
                "StaticCloud(center: (300, 800), radius: 900)",
                LevelError::BadObjectSize {
                    object: 0,
                    min: 20.0,
                    max: 300.0,
                },
            ),
            (
                "StaticCloud(center: (560, 240), radius: 60)",
                LevelError::Overlaps {
                    object: 0,
                    item: Item::Start,
                },
            ),
            (
                "StaticCloud(center: (540, 1800), radius: 60)",
                LevelError::Overlaps {
                    object: 0,
                    item: Item::Beacon,
                },
            ),
            (
                "StaticCloud(center: (220, 1000), radius: 60)",
                LevelError::Overlaps {
                    object: 0,
                    item: Item::Stardust(0),
                },
            ),
        ];
        for (objects, expected) in cases {
            let error = Level::parse(&with_objects(objects)).unwrap_err();
            assert_eq!(error, expected, "for {objects}");
        }
    }

    #[test]
    fn ribbon_rules_are_enforced() {
        let ok = "WindRibbon(path: [(100, 600), (500, 700), (900, 650)], width: 80)";
        assert!(Level::parse(&with_objects(ok)).is_ok());
        let cases = [
            (
                "WindRibbon(path: [(100, 600)], width: 80)",
                LevelError::BadRibbonPath(0),
            ),
            (
                "WindRibbon(path: [(100, 600), (1200, 700)], width: 80)",
                LevelError::Outside(Item::Object(0)),
            ),
            (
                "WindRibbon(path: [(100, 600), (500, 700)], width: 10)",
                LevelError::BadObjectSize {
                    object: 0,
                    min: 30.0,
                    max: 240.0,
                },
            ),
            (
                "WindRibbon(path: [(400, 150), (700, 250)], width: 80)",
                LevelError::Overlaps {
                    object: 0,
                    item: Item::Start,
                },
            ),
        ];
        for (objects, expected) in cases {
            let error = Level::parse(&with_objects(objects)).unwrap_err();
            assert_eq!(error, expected, "for {objects}");
        }
    }

    #[test]
    fn portal_rules_are_enforced() {
        let ok = "TwinPortals(a: (200, 700), b: (800, 1400), radius: 60)";
        assert!(Level::parse(&with_objects(ok)).is_ok());
        let cases = [
            (
                "TwinPortals(a: (200, 700), b: (300, 760), radius: 60)",
                LevelError::PortalsTooClose(0),
            ),
            (
                "TwinPortals(a: (200, 700), b: (800, 2200), radius: 60)",
                LevelError::Outside(Item::Object(0)),
            ),
            (
                "TwinPortals(a: (540, 230), b: (800, 1400), radius: 60)",
                LevelError::Overlaps {
                    object: 0,
                    item: Item::Start,
                },
            ),
        ];
        for (objects, expected) in cases {
            let error = Level::parse(&with_objects(objects)).unwrap_err();
            assert_eq!(error, expected, "for {objects}");
        }
    }

    #[test]
    fn messages_are_readable() {
        let message = LevelError::Outside(Item::Stardust(2)).to_string();
        assert!(message.starts_with("stardust #3 is outside the level"));
    }
}
