//! Pure-Rust deterministic SplitMix64 pseudo-random number generator.
//!
//! Generates reproducible 64-bit pseudo-random streams with zero ambient entropy
//! and zero third-party dependencies, discharging CWE-330 for non-cryptographic
//! deterministic timbre variation (VER05 §2.1 / TF01 §5.1).

/// Pure-Rust deterministic SplitMix64 PRNG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// Construct a new SplitMix64 generator with an initial seed.
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Advance internal state and return next 64-bit pseudo-random word.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Return a uniform pseudo-random float in `[-1.0, 1.0]`.
    pub fn next_float_signed(&mut self) -> f64 {
        let val = self.next_u64();
        (val as f64 / 9223372036854775807.5) - 1.0
    }

    /// Return a uniform pseudo-random float in `[0.0, 1.0]`.
    pub fn next_float_unit(&mut self) -> f64 {
        let val = self.next_u64();
        (val as f64) / 18446744073709551615.0
    }
}
