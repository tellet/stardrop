//! Frame-time statistics for the F1 overlay. Plain Rust, no Bevy.

/// Summary of recent frame times, in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameTimeSummary {
    /// The most recent frame.
    pub current_ms: f64,
    /// Mean over the whole window.
    pub average_ms: f64,
    /// Slowest frame in the window: what the player feels as a hitch.
    pub worst_ms: f64,
}

/// Summarizes a window of frame times, oldest first.
///
/// Returns `None` for an empty window. Takes an iterator rather than a slice so
/// callers can pass Bevy's diagnostic history directly, without copying it
/// into a new `Vec` every frame.
#[must_use]
pub fn summarize(frame_times_ms: impl IntoIterator<Item = f64>) -> Option<FrameTimeSummary> {
    let mut count = 0_u32;
    let mut sum = 0.0;
    let mut worst = f64::MIN;
    let mut current = None;
    for ms in frame_times_ms {
        count += 1;
        sum += ms;
        worst = worst.max(ms);
        current = Some(ms);
    }
    let current_ms = current?;
    Some(FrameTimeSummary {
        current_ms,
        average_ms: sum / f64::from(count),
        worst_ms: worst,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_window_has_no_summary() {
        assert_eq!(summarize([]), None);
    }

    #[test]
    fn summary_tracks_current_average_and_worst() {
        let summary = summarize([16.0, 20.0, 12.0]);
        assert_eq!(
            summary,
            Some(FrameTimeSummary {
                current_ms: 12.0,
                average_ms: 16.0,
                worst_ms: 20.0,
            })
        );
    }
}
