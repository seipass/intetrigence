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

/// MINSTD / Park–Miller constants used by the garbage hole RNG
/// (`docs/s2-spec.md` §7.7).
pub const MINSTD_MODULUS: u64 = 2_147_483_647;
pub const MINSTD_MULTIPLIER: u64 = 16_807;
pub const MINSTD_MAX_FLOAT: f64 = 2_147_483_646.0;

/// MINSTD (Park–Miller) RNG, kept separate from the 7-bag's SplitMix64.
///
/// The garbage queue must reproduce TETR.IO's hole-column draw order, so this
/// algorithm and its seed initialization are fixed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MinStd {
    state: u64,
}

impl MinStd {
    pub fn new(seed: u64) -> Self {
        let mut state = seed % MINSTD_MODULUS;
        if state == 0 {
            state = MINSTD_MODULUS - 1;
        }
        Self { state }
    }

    /// Current MINSTD state (the seed used by a fresh `MINSTD(state)`).
    pub fn state(&self) -> u64 {
        self.state
    }

    pub fn next_u32(&mut self) -> u32 {
        self.state = (MINSTD_MULTIPLIER * self.state) % MINSTD_MODULUS;
        self.state as u32
    }

    /// Uniform float in `[0, 1)`.
    pub fn next_float(&mut self) -> f64 {
        (self.next_u32() as f64 - 1.0) / MINSTD_MAX_FLOAT
    }
}

/// Seed derivation for one versus round (`docs/s2-spec.md` v1.3.1 §11-U7).
///
/// `piece_seed` drives the shared 7-bag; `garbage_seeds[0]` and
/// `garbage_seeds[1]` drive the two bots' hole RNGs. Slots are bot identities
/// (0 = intetrigence, 1 = attax), not seats. The old v1.2 xor derivation is
/// deliberately not used: it made slot 0 share the bag stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeedDerivation {
    pub piece_seed: u64,
    pub garbage_seeds: [u64; 2],
}

/// Derives the piece and garbage seeds by advancing one MINSTD stream.
pub fn derive_seeds(seed: u64) -> SeedDerivation {
    let mut base = MinStd::new(seed);
    let piece_seed = base.state();
    let garbage_seed_0 = base.next_u32() as u64;
    let garbage_seed_1 = base.next_u32() as u64;
    SeedDerivation {
        piece_seed,
        garbage_seeds: [garbage_seed_0, garbage_seed_1],
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

    #[test]
    fn minstd_known_values() {
        let mut rng = MinStd::new(1);
        assert_eq!(rng.next_u32(), 16_807);
        assert_eq!(rng.next_u32(), 282_475_249);
    }

    #[test]
    fn minstd_float_is_in_unit_interval() {
        let mut rng = MinStd::new(123);
        for _ in 0..10_000 {
            let value = rng.next_float();
            assert!((0.0..1.0).contains(&value), "{value}");
        }
    }

    #[test]
    fn minstd_zero_seed_is_remapped() {
        let mut zero = MinStd::new(0);
        let mut max = MinStd::new(MINSTD_MODULUS - 1);
        assert_eq!(zero.next_u32(), max.next_u32());
    }
}
