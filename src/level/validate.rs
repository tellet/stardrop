//! Level validation and its error type.
//!
//! [`check`] looks at a parsed [`LevelFile`] and returns the first rule it
//! breaks. Messages are written for whoever edits the level file, since they
//! appear on screen.

use core::fmt;

use super::data::{FilePoint, LEVEL_WIDTH, LevelFile};

/// A level must be at least one screen tall: shorter levels can't fill the
/// view (the camera would have nowhere valid to be).
pub const MIN_LEVEL_HEIGHT: f32 = 1080.0;

/// Which thing in the level an error is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    /// The astronaut's start point.
    Start,
    /// The landing beacon.
    Beacon,
    /// A checkpoint, by its position in the list (0-based).
    Checkpoint(usize),
    /// A stardust mote, by its position in the list (0-based).
    Stardust(usize),
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Humans count from 1, so the 0-based list index is shown +1.
        match self {
            Self::Start => write!(f, "the start"),
            Self::Beacon => write!(f, "the beacon"),
            Self::Checkpoint(i) => write!(f, "checkpoint #{}", i + 1),
            Self::Stardust(i) => write!(f, "stardust #{}", i + 1),
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
    /// The level has no name.
    EmptyName,
    /// A number is `NaN` or infinite.
    NotANumber(&'static str),
    /// The level is shorter than one screen.
    TooShort {
        /// The height in the file.
        height: f32,
    },
    /// Something is placed outside the level's edges.
    Outside(Item),
    /// The beacon's width or height isn't positive.
    BadBeaconSize,
    /// The beacon isn't below the start, so it can never be reached by falling.
    BeaconAboveStart,
    /// A checkpoint is above the start or below the beacon.
    CheckpointOutsideFall(usize),
    /// Checkpoints must be listed from top to bottom.
    CheckpointsOutOfOrder(usize),
    /// The target time isn't positive.
    BadTargetTime,
}

impl fmt::Display for LevelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax {
                line,
                column,
                message,
            } => write!(f, "line {line}, column {column}: {message}"),
            Self::EmptyName => write!(f, "the level needs a name"),
            Self::NotANumber(field) => write!(f, "{field} must be a normal number"),
            Self::TooShort { height } => write!(
                f,
                "height is {height}, but a level must be at least {MIN_LEVEL_HEIGHT} (one screen)"
            ),
            Self::Outside(item) => write!(
                f,
                "{item} is outside the level (x must be 0–{LEVEL_WIDTH}, depth 0–height)"
            ),
            Self::BadBeaconSize => write!(f, "the beacon's width and height must be positive"),
            Self::BeaconAboveStart => write!(f, "the beacon must be below the start"),
            Self::CheckpointOutsideFall(i) => write!(
                f,
                "{} must be between the start and the beacon",
                Item::Checkpoint(*i)
            ),
            Self::CheckpointsOutOfOrder(i) => write!(
                f,
                "{} is above the one before it; list checkpoints top to bottom",
                Item::Checkpoint(*i)
            ),
            Self::BadTargetTime => write!(f, "target_time must be more than 0 seconds"),
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
    finite("height", file.height)?;
    finite("target_time", file.target_time)?;
    if file.height < MIN_LEVEL_HEIGHT {
        return Err(LevelError::TooShort {
            height: file.height,
        });
    }
    if file.target_time <= 0.0 {
        return Err(LevelError::BadTargetTime);
    }

    inside(file, file.start, Item::Start)?;
    check_beacon(file)?;
    check_checkpoints(file)?;
    for (i, &mote) in file.stardust.iter().enumerate() {
        inside(file, mote, Item::Stardust(i))?;
    }
    Ok(())
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
        inside(file, corner, Item::Beacon)?;
    }
    if depth - height / 2.0 <= file.start.1 {
        return Err(LevelError::BeaconAboveStart);
    }
    Ok(())
}

fn check_checkpoints(file: &LevelFile) -> Result<(), LevelError> {
    let beacon_top = file.beacon.center.1 - file.beacon.size.1 / 2.0;
    let mut previous_depth = file.start.1;
    for (i, &checkpoint) in file.checkpoints.iter().enumerate() {
        inside(file, checkpoint, Item::Checkpoint(i))?;
        let depth = checkpoint.1;
        if depth <= file.start.1 || depth >= beacon_top {
            return Err(LevelError::CheckpointOutsideFall(i));
        }
        if depth <= previous_depth {
            return Err(LevelError::CheckpointsOutOfOrder(i));
        }
        previous_depth = depth;
    }
    Ok(())
}

fn inside(file: &LevelFile, point: FilePoint, item: Item) -> Result<(), LevelError> {
    let FilePoint(x, depth) = point;
    finite("a position", x)?;
    finite("a position", depth)?;
    let within = (0.0..=LEVEL_WIDTH).contains(&x) && (0.0..=file.height).contains(&depth);
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
        height: 5400,
        theme: DeepSpace,
        start: (960, 300),
        beacon: (center: (960, 5380), size: (320, 40)),
        checkpoints: [(960, 2000), (500, 4000)],
        stardust: [(200, 1000), (1700, 3000)],
        target_time: 20.0,
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
        let error = error_after("height: 5400,", "height: 5400");
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
        let error = error_after("target_time: 20.0,", "target_time: 20.0, gravity: 900,");
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
        let error = error_after("target_time: 20.0,", "");
        assert!(error.to_string().contains("target_time"));
    }

    #[test]
    fn empty_name_is_rejected() {
        assert_eq!(error_after(r#""Test""#, r#""  ""#), LevelError::EmptyName);
    }

    #[test]
    fn too_short_level_is_rejected() {
        let error = error_after("height: 5400", "height: 900");
        assert!(matches!(error, LevelError::TooShort { .. }));
    }

    #[test]
    fn nan_is_rejected() {
        let error = error_after("height: 5400", "height: NaN");
        assert_eq!(error, LevelError::NotANumber("height"));
    }

    #[test]
    fn things_outside_the_level_are_rejected() {
        assert_eq!(
            error_after("start: (960, 300)", "start: (2000, 300)"),
            LevelError::Outside(Item::Start)
        );
        assert_eq!(
            error_after("(200, 1000)", "(200, 9000)"),
            LevelError::Outside(Item::Stardust(0))
        );
        assert_eq!(
            error_after("(500, 4000)", "(-5, 4000)"),
            LevelError::Outside(Item::Checkpoint(1))
        );
    }

    #[test]
    fn beacon_must_fit_inside_the_level() {
        // Center inside, but the pad pokes out past the right edge.
        let error = error_after("center: (960, 5380)", "center: (1900, 5380)");
        assert_eq!(error, LevelError::Outside(Item::Beacon));
    }

    #[test]
    fn beacon_must_have_a_size() {
        assert_eq!(
            error_after("size: (320, 40)", "size: (0, 40)"),
            LevelError::BadBeaconSize
        );
    }

    #[test]
    fn beacon_above_start_is_rejected() {
        let error = error_after("start: (960, 300)", "start: (960, 5390)");
        assert_eq!(error, LevelError::BeaconAboveStart);
    }

    #[test]
    fn checkpoints_must_be_between_start_and_beacon_in_order() {
        assert_eq!(
            error_after("(960, 2000)", "(960, 100)"),
            LevelError::CheckpointOutsideFall(0)
        );
        assert_eq!(
            error_after("(500, 4000)", "(500, 1500)"),
            LevelError::CheckpointsOutOfOrder(1)
        );
    }

    #[test]
    fn target_time_must_be_positive() {
        assert_eq!(
            error_after("target_time: 20.0", "target_time: 0.0"),
            LevelError::BadTargetTime
        );
    }

    #[test]
    fn messages_are_readable() {
        let message = LevelError::Outside(Item::Stardust(2)).to_string();
        assert!(message.starts_with("stardust #3 is outside the level"));
    }
}
