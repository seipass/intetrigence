//! Deterministic pseudo-random number generation.
//!
//! The engine must be reproducible from `(seed, action sequence)`, so the 7-bag
//! uses a fixed, self-contained algorithm rather than an OS or thread RNG.
//! `SplitMix64` is also exposed for tests, tools and future tuning code.

/// SplitMix64 (Steele, Lea & Flood). Fixed algorithm; changing it changes every
/// seeded bag sequence and therefore every replay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform integer in `0..upper` using rejection sampling (no modulo bias).
    pub fn below(&mut self, upper: u64) -> u64 {
        assert!(upper > 0, "upper bound must be positive");
        // `zone` is the largest multiple of `upper` representable below
        // `u64::MAX`; values at or above it are rejected.
        let zone = u64::MAX - (u64::MAX % upper);
        loop {
            let value = self.next_u64();
            if value < zone {
                return value % upper;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_first_output_for_seed_zero() {
        let mut rng = SplitMix64::new(0);
        assert_eq!(rng.next_u64(), 0xE220_A839_7B1D_CDAF);
    }

    #[test]
    fn below_stays_in_range() {
        let mut rng = SplitMix64::new(42);
        for upper in 1..=100 {
            for _ in 0..100 {
                assert!(rng.below(upper) < upper);
            }
        }
    }

    #[test]
    fn same_seed_same_stream() {
        let mut a = SplitMix64::new(12345);
        let mut b = SplitMix64::new(12345);
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }
}
