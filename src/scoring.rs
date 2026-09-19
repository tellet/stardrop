//! Run tracking: how long the current run has taken and which stardust has
//! been collected. Plain Rust, no ECS. Star scoring builds on this in
//! Milestone 5.

/// The most stardust motes a level may have (one bit each in [`StardustSet`]).
pub const MAX_STARDUST: usize = 64;

/// Which stardust motes are collected, one bit per mote index.
///
/// A single `u64` instead of a list: copying, clearing and counting it never
/// allocates, and it's all a run needs to remember.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StardustSet(u64);

impl StardustSet {
    /// Marks mote `index` as collected. Returns `true` only the first time.
    /// Indices of `MAX_STARDUST` or more are ignored (validation rules them out).
    pub fn insert(&mut self, index: usize) -> bool {
        let Some(bit) = Self::bit(index) else {
            return false;
        };
        let newly = self.0 & bit == 0;
        self.0 |= bit;
        newly
    }

    /// Whether mote `index` has been collected.
    #[must_use]
    pub fn contains(self, index: usize) -> bool {
        Self::bit(index).is_some_and(|bit| self.0 & bit != 0)
    }

    /// How many motes have been collected.
    #[must_use]
    pub fn count(self) -> usize {
        // At most 64, so the conversion can't fail; 0 is a safe fallback anyway.
        usize::try_from(self.0.count_ones()).unwrap_or(0)
    }

    fn bit(index: usize) -> Option<u64> {
        u32::try_from(index)
            .ok()
            .and_then(|shift| 1_u64.checked_shl(shift))
    }
}

/// The current run: from the start of the level (or the last retry) to landing.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RunStats {
    /// Seconds since the run started (respawn fades included).
    pub elapsed: f32,
    /// Stardust collected during this run.
    pub stardust: StardustSet,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collecting_counts_each_mote_once() {
        let mut set = StardustSet::default();
        assert!(set.insert(3));
        assert!(!set.insert(3), "second pickup of the same mote");
        assert!(set.insert(0));
        assert_eq!(set.count(), 2);
        assert!(set.contains(3) && set.contains(0) && !set.contains(1));
    }

    #[test]
    fn all_64_indices_work_and_larger_ones_are_ignored() {
        let mut set = StardustSet::default();
        for index in 0..MAX_STARDUST {
            assert!(set.insert(index));
        }
        assert_eq!(set.count(), MAX_STARDUST);
        assert!(!set.insert(MAX_STARDUST));
        assert!(!set.contains(1000));
    }
}
