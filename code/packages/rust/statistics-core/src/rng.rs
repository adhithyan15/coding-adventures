//! Mersenne Twister 19937 (MT19937) pseudo-random number generator.
//!
//! Pure-Rust faithful port of the Matsumoto-Nishimura 1998 reference
//! implementation. R's default RNG follows the same recurrence and
//! initialization, so seeded sequences round-trip across the two
//! systems.
//!
//! `RngState::new(seed)` calls `init_genrand(seed as u32)` which is
//! R's `Init_R_seed` initialization:
//!
//! ```text
//!   state[0] = seed
//!   state[i] = 1812433253 * (state[i-1] XOR (state[i-1] >> 30)) + i   for i in 1..624
//! ```

const N: usize = 624;
const M: usize = 397;
const MATRIX_A: u32 = 0x9908b0df;
const UPPER_MASK: u32 = 0x80000000;
const LOWER_MASK: u32 = 0x7fffffff;

/// Mersenne Twister 19937 state.
#[derive(Debug, Clone)]
pub struct RngState {
    state: [u32; N],
    index: usize,
    /// The seed last used to initialize this state (preserved so the
    /// caller can echo it back via the public API).
    seed_value: u64,
}

impl RngState {
    /// Create a new generator from a seed.
    pub fn new(seed: u64) -> Self {
        let mut s = Self {
            state: [0; N],
            index: N,
            seed_value: seed,
        };
        s.seed(seed);
        s
    }

    /// Re-seed in place. Equivalent to R's `set.seed(seed)`.
    pub fn seed(&mut self, seed: u64) {
        self.seed_value = seed;
        self.state[0] = seed as u32;
        for i in 1..N {
            // Same multiplier R uses, mask to u32 implicit in arithmetic.
            self.state[i] = 1812433253_u32
                .wrapping_mul(self.state[i - 1] ^ (self.state[i - 1] >> 30))
                .wrapping_add(i as u32);
        }
        self.index = N;
    }

    /// The seed value last passed to `new` / `seed`. Read-only.
    pub fn seed_value(&self) -> u64 {
        self.seed_value
    }

    /// Generate the next 32-bit unsigned integer.
    pub fn next_u32(&mut self) -> u32 {
        if self.index >= N {
            self.refill();
        }
        let mut y = self.state[self.index];
        self.index += 1;
        // Tempering — verbatim from Matsumoto-Nishimura.
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c5680;
        y ^= (y << 15) & 0xefc60000;
        y ^= y >> 18;
        y
    }

    /// Generate a uniform double in `[0, 1)`.
    pub fn next_f64(&mut self) -> f64 {
        // 53-bit precision; (next_u32 >> 5) gives 27 bits, (next_u32 >> 6) gives 26.
        let a = (self.next_u32() >> 5) as u64; // 27 bits
        let b = (self.next_u32() >> 6) as u64; // 26 bits
        (a * 67_108_864 + b) as f64 / 9_007_199_254_740_992.0
    }

    fn refill(&mut self) {
        for i in 0..(N - M) {
            let y = (self.state[i] & UPPER_MASK) | (self.state[i + 1] & LOWER_MASK);
            self.state[i] = self.state[i + M] ^ (y >> 1) ^ if y & 1 == 1 { MATRIX_A } else { 0 };
        }
        for i in (N - M)..(N - 1) {
            let y = (self.state[i] & UPPER_MASK) | (self.state[i + 1] & LOWER_MASK);
            self.state[i] = self.state[i + M - N]
                ^ (y >> 1)
                ^ if y & 1 == 1 { MATRIX_A } else { 0 };
        }
        let y = (self.state[N - 1] & UPPER_MASK) | (self.state[0] & LOWER_MASK);
        self.state[N - 1] = self.state[M - 1] ^ (y >> 1) ^ if y & 1 == 1 { MATRIX_A } else { 0 };
        self.index = 0;
    }
}

/// Excel `RAND()` would be `set_seed` then `next_f64` — but `RAND()`
/// has no explicit seed in the spreadsheet model. The reproducibility
/// is the *engine's* responsibility, not the function's.
pub fn set_seed(state: &mut RngState, seed: u64) {
    state.seed(seed);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference values from the MT19937 documentation file
    /// `mt19937ar.out` after calling `init_genrand(5489)` and pulling
    /// the first six outputs.
    #[test]
    fn known_seed_produces_reference_sequence() {
        let mut rng = RngState::new(5489);
        let expected: [u32; 6] = [
            3499211612, 581869302, 3890346734, 3586334585, 545404204, 4161255391,
        ];
        for (i, &exp) in expected.iter().enumerate() {
            let actual = rng.next_u32();
            assert_eq!(actual, exp, "mismatch at position {i}");
        }
    }

    #[test]
    fn seeded_streams_are_independent() {
        let mut a = RngState::new(42);
        let mut b = RngState::new(99);
        let a_first = a.next_u32();
        let b_first = b.next_u32();
        assert_ne!(a_first, b_first);
    }

    #[test]
    fn reseeding_resets_stream() {
        let mut rng = RngState::new(7);
        let first_run: Vec<u32> = (0..5).map(|_| rng.next_u32()).collect();
        rng.seed(7);
        let second_run: Vec<u32> = (0..5).map(|_| rng.next_u32()).collect();
        assert_eq!(first_run, second_run);
    }

    #[test]
    fn set_seed_function_works() {
        let mut rng = RngState::new(0);
        let _ = rng.next_u32();
        set_seed(&mut rng, 42);
        let a = rng.next_u32();
        let mut rng2 = RngState::new(42);
        let b = rng2.next_u32();
        assert_eq!(a, b);
    }

    #[test]
    fn next_f64_in_unit_range() {
        let mut rng = RngState::new(1234);
        for _ in 0..10_000 {
            let v = rng.next_f64();
            assert!((0.0..1.0).contains(&v), "out-of-range: {v}");
        }
    }

    #[test]
    fn next_f64_reasonable_distribution() {
        // Empirical mean over 10k samples should be ~0.5.
        let mut rng = RngState::new(2024);
        let n = 10_000;
        let sum: f64 = (0..n).map(|_| rng.next_f64()).sum();
        let mean = sum / n as f64;
        assert!((mean - 0.5).abs() < 0.02, "mean={mean}");
    }

    #[test]
    fn seed_value_round_trips() {
        let rng = RngState::new(31_415_926_535);
        assert_eq!(rng.seed_value(), 31_415_926_535);
    }
}
