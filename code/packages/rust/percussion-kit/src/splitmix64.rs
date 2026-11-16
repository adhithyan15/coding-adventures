//! Pure-Rust deterministic SplitMix64 pseudo-random number generator.
//!
//! Generates reproducible 64-bit pseudo-random streams with zero ambient entropy
//! and zero third-party dependencies, discharging CWE-330 for deterministic
//! unpitched percussion synthesis and property-based fuzz testing.

/// Pure-Rust deterministic 64-bit pseudo-random number generator (Steele et al., 2014).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// Construct a new SplitMix64 generator with an initial seed.
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Advance internal state and return the next 64-bit pseudo-random word.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Return a uniform pseudo-random float in `[0.0, 1.0)`.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / 9007199254740992.0)
    }

    /// Return a uniform pseudo-random float in `[-1.0, 1.0]`.
    pub fn next_float_signed(&mut self) -> f64 {
        let val = self.next_u64();
        (val as f64 / 9223372036854775807.5) - 1.0
    }

    /// Return a uniform pseudo-random float in `[low, high)`.
    pub fn uniform(&mut self, low: f64, high: f64) -> f64 {
        low + (high - low) * self.next_f64()
    }

    /// Pick a uniform pseudo-random element from a non-empty slice.
    pub fn choice<'a, T>(&mut self, slice: &'a [T]) -> &'a T {
        assert!(!slice.is_empty(), "choice called on empty slice");
        let idx = (self.next_u64() as usize) % slice.len();
        &slice[idx]
    }
}

/// Statelessly evaluate a deterministic SplitMix64 noise sample for a given index and seed.
#[inline]
pub fn splitmix64_noise(sample_index: usize, seed: u64) -> f64 {
    // Hash (seed, sample_index) using SplitMix64 rounds without persistent heap state
    let state = seed.wrapping_add((sample_index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let mut z = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    let u = z ^ (z >> 31);
    (u as f64 / 9223372036854775807.5) - 1.0
}
