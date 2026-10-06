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
    /// Objects touched during this run: the length of the melody so far.
    pub touches: u32,
}

/// The three stars a level can award (Section 3.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stars {
    /// Reached the beacon.
    pub landed: bool,
    /// Collected every stardust mote in one attempt.
    pub all_stardust: bool,
    /// Finished the level's phrase: touched that many objects in the one
    /// attempt, so the melody ran its full length.
    pub phrase: bool,
}

impl Stars {
    /// What an attempt earned. Only a landed attempt earns anything: the
    /// other two stars are for *how* you landed.
    #[must_use]
    pub fn earned(run: RunStats, stardust_in_level: usize, phrase: u8) -> Self {
        Self {
            landed: true,
            all_stardust: run.stardust.count() >= stardust_in_level,
            phrase: run.touches >= u32::from(phrase),
        }
    }

    /// How many stars, 0 to 3.
    #[must_use]
    pub fn count(self) -> u8 {
        u8::from(self.landed) + u8::from(self.all_stardust) + u8::from(self.phrase)
    }

    /// The best of two results, star by star: stars from different attempts
    /// add up (Section 3.5 counts a level's best, not one perfect run).
    #[must_use]
    pub fn merged(self, other: Self) -> Self {
        Self {
            landed: self.landed || other.landed,
            all_stardust: self.all_stardust || other.all_stardust,
            phrase: self.phrase || other.phrase,
        }
    }
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

    fn run(collected: usize, touches: u32) -> RunStats {
        let mut stardust = StardustSet::default();
        for index in 0..collected {
            stardust.insert(index);
        }
        RunStats {
            elapsed: 10.0,
            stardust,
            touches,
        }
    }

    #[test]
    fn landing_earns_one_star_and_the_others_are_for_how_you_landed() {
        let straight_down = Stars::earned(run(1, 2), 3, 5);
        assert_eq!(
            straight_down,
            Stars {
                landed: true,
                all_stardust: false,
                phrase: false
            }
        );
        assert_eq!(straight_down.count(), 1);

        let perfect = Stars::earned(run(3, 5), 3, 5);
        assert_eq!(perfect.count(), 3);
        // A level with no stardust gives that star to anyone who lands.
        assert!(Stars::earned(run(0, 0), 0, 5).all_stardust);
        // Exactly the asked-for phrase counts, and more than it does too.
        assert!(Stars::earned(run(0, 5), 3, 5).phrase);
        assert!(Stars::earned(run(0, 9), 3, 5).phrase);
    }

    #[test]
    fn stars_from_different_attempts_add_up() {
        let musical_run = Stars::earned(run(0, 5), 3, 5);
        let thorough_run = Stars::earned(run(3, 1), 3, 5);
        assert_eq!(musical_run.merged(thorough_run).count(), 3);
    }
}
