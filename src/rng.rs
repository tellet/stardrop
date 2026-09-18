//! A tiny deterministic random-number generator (`SplitMix64`).
//!
//! Used for placing decorative things like background stars. Seeded, so the
//! same seed always gives the same layout, and small enough that pulling in
//! the `rand` crate isn't worth it.

/// Seeded pseudo-random generator. Not for anything security-related.
#[derive(Debug, Clone)]
pub(crate) struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// A generator that always produces the same sequence for the same seed.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0.0..1.0`.
    pub fn next_f32(&mut self) -> f32 {
        /// 2^24: an f32 represents every integer up to this exactly.
        const TWO_POW_24: f32 = 16_777_216.0;
        // The top 24 bits fit an f32 mantissa exactly, so every value is equally likely.
        #[expect(clippy::cast_precision_loss, reason = "24-bit value is exact in f32")]
        let bits = (self.next_u64() >> 40) as f32;
        bits / TWO_POW_24
    }

    /// Uniform in `min..max`.
    pub fn range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.next_f32()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = SplitMix64::new(7);
        let mut b = SplitMix64::new(7);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn floats_stay_in_range() {
        let mut rng = SplitMix64::new(1);
        for _ in 0..10_000 {
            let value = rng.range(-2.0, 3.0);
            assert!((-2.0..3.0).contains(&value));
        }
    }
}
