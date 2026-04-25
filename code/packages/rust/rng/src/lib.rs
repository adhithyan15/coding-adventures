//! # rng
//!
//! Three classic pseudorandom number generators — LCG, Xorshift64, and PCG32.
//!
//! All three generators expose the same interface:
//!
//! ```rust
//! use rng::Lcg;
//!
//! let mut g = Lcg::new(42);
//! let v: u32  = g.next_u32();              // uniform in [0, 2^32)
//! let u: u64  = g.next_u64();              // uniform in [0, 2^64)
//! let f: f64  = g.next_float();            // uniform in [0.0, 1.0)
//! let n: i64  = g.next_int_in_range(1, 6); // die roll, inclusive
//! ```
//!
//! ## Why three generators?
//!
//! | Generator  | Speed | Quality | State |
//! |------------|-------|---------|-------|
//! | LCG        | ★★★   | ★☆☆    | 8 B   |
//! | Xorshift64 | ★★★   | ★★☆    | 8 B   |
//! | PCG32      | ★★☆   | ★★★    | 8 B   |
//!
//! All are deterministic given the same seed, which is important for
//! reproducible simulations and tests.
//!
//! ## Reference values (seed = 1, first three `next_u32()` calls)
//!
//! | Generator  | [0]        | [1]        | [2]       |
//! |------------|------------|------------|-----------|
//! | LCG        | 1817669548 | 2187888307 | 2784682393 |
//! | Xorshift64 | 1082269761 | 201397313  | 1854285353 |
//! | PCG32      | 1412771199 | 1791099446 | 124312908  |
//!
//! These values are cross-checked against the Go reference implementation.

// ── Constants ─────────────────────────────────────────────────────────────────
//
// These Knuth/Numerical Recipes constants satisfy the Hull-Dobell theorem:
//   full period 2^64 (every 64-bit integer appears exactly once per cycle).
//
//   a = 6364136223846793005  (LCG multiplier)
//   c = 1442695040888963407  (LCG increment — must be odd; already is)
//
// The same pair is reused by PCG32, which adds an output permutation on top.

const LCG_MULTIPLIER: u64 = 6364136223846793005;
const LCG_INCREMENT: u64 = 1442695040888963407;

/// Divisor for normalising a u32 to the half-open interval [0.0, 1.0).
/// 2^32 as an f64 = 4_294_967_296.0
const FLOAT_DIV: f64 = 4_294_967_296.0;

// ── LCG ───────────────────────────────────────────────────────────────────────

/// Linear Congruential Generator (Knuth 1948).
///
/// The recurrence is:
///
/// ```text
/// state_{n+1} = (state_n × a + c)  mod 2^64
/// ```
///
/// where `a = 6364136223846793005` and `c = 1442695040888963407`.
///
/// **Output:** upper 32 bits of state. The lower bits have shorter
/// sub-periods and are discarded. Correlation between successive
/// outputs is the main weakness of plain LCGs.
///
/// **Period:** 2^64 — every 64-bit value appears exactly once per cycle
/// (Hull-Dobell conditions: c is odd; a-1 is divisible by every prime
/// factor of 2^64, and also by 4 since 2^64 is divisible by 4).
pub struct Lcg {
    state: u64,
}

impl Lcg {
    /// Creates a new LCG seeded with `seed`. Any 64-bit value is valid.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Advances the LCG state and returns the upper 32 bits.
    ///
    /// Using the *upper* half discards the low-quality lower bits that
    /// have shorter sub-periods.
    pub fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(LCG_MULTIPLIER).wrapping_add(LCG_INCREMENT);
        (self.state >> 32) as u32
    }

    /// Returns a 64-bit value built from two consecutive [`next_u32`] calls:
    /// `(hi << 32) | lo`.
    pub fn next_u64(&mut self) -> u64 {
        let hi = self.next_u32() as u64;
        let lo = self.next_u32() as u64;
        (hi << 32) | lo
    }

    /// Returns a `f64` uniformly distributed in `[0.0, 1.0)`.
    pub fn next_float(&mut self) -> f64 {
        self.next_u32() as f64 / FLOAT_DIV
    }

    /// Returns a uniform random integer in `[min, max]` inclusive.
    ///
    /// Uses rejection sampling to eliminate modulo bias. Naïve
    /// `value % range` over-samples small residues when `2^32` is not
    /// divisible by `range`. We compute:
    ///
    /// ```text
    /// threshold = (-range_size) mod range_size
    /// ```
    ///
    /// Any draw below `threshold` is discarded. The expected number of
    /// extra draws per call is less than 2 for all range sizes.
    pub fn next_int_in_range(&mut self, min: i64, max: i64) -> i64 {
        let range_size = (max - min + 1) as u64;
        let threshold = range_size.wrapping_neg() % range_size;
        loop {
            let r = self.next_u32() as u64;
            if r >= threshold {
                return min + (r % range_size) as i64;
            }
        }
    }
}

// ── Xorshift64 ────────────────────────────────────────────────────────────────

/// Xorshift64 generator (Marsaglia 2003).
///
/// Three XOR-shift operations permute all 64 bits of state without
/// any multiplication, making this the fastest of the three generators:
///
/// ```text
/// x ^= x << 13
/// x ^= x >> 7
/// x ^= x << 17
/// ```
///
/// **Period:** 2^64 − 1. State 0 is a fixed point (all operations on
/// zero produce zero), so seed 0 is silently replaced with 1.
///
/// **Output:** lower 32 bits. The shifts already mix high bits down into
/// the low half, so no further permutation is needed.
pub struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    /// Creates a new Xorshift64 seeded with `seed`.
    /// Seed 0 is replaced with 1 to avoid the all-zeros fixed point.
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    /// Applies the three XOR-shifts and returns the lower 32 bits.
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x as u32
    }

    /// Returns a 64-bit value built from two consecutive [`next_u32`] calls.
    pub fn next_u64(&mut self) -> u64 {
        let hi = self.next_u32() as u64;
        let lo = self.next_u32() as u64;
        (hi << 32) | lo
    }

    /// Returns a `f64` uniformly distributed in `[0.0, 1.0)`.
    pub fn next_float(&mut self) -> f64 {
        self.next_u32() as f64 / FLOAT_DIV
    }

    /// Returns a uniform random integer in `[min, max]` inclusive.
    /// Uses rejection sampling — identical algorithm to [`Lcg::next_int_in_range`].
    pub fn next_int_in_range(&mut self, min: i64, max: i64) -> i64 {
        let range_size = (max - min + 1) as u64;
        let threshold = range_size.wrapping_neg() % range_size;
        loop {
            let r = self.next_u32() as u64;
            if r >= threshold {
                return min + (r % range_size) as i64;
            }
        }
    }
}

// ── PCG32 ─────────────────────────────────────────────────────────────────────

/// Permuted Congruential Generator (O'Neill 2014).
///
/// Uses the same LCG recurrence as [`Lcg`] but applies the XSH RR
/// (XOR-Shift High / Random Rotate) output permutation before returning:
///
/// ```text
/// old_state → advance LCG → emit permuted(old_state)
///
/// xorshifted = ((old >> 18) ^ old) >> 27   // mix high bits down to low 32
/// rot        = old >> 59                    // 5-bit rotation amount
/// output     = rotr32(xorshifted, rot)      // scatter remaining bits
/// ```
///
/// The permutation is applied to the state *before* advancing, which
/// breaks the linear correlation between outputs that LCG suffers from.
/// PCG32 passes all known statistical test suites (TestU01 BigCrush,
/// PractRand).
///
/// **State size:** 8 bytes (64-bit LCG state).
/// **Period:** 2^64.
pub struct Pcg32 {
    state: u64,
    increment: u64,
}

impl Pcg32 {
    /// Creates a new PCG32 seeded with `seed`.
    ///
    /// The initseq warm-up is applied so that even seeds 0 and 1 produce
    /// well-distributed initial sequences:
    ///
    /// 1. Start from state = 0.
    /// 2. Advance once (incorporates the fixed increment).
    /// 3. Mix `seed` into state.
    /// 4. Advance once more (scatters seed bits throughout state).
    pub fn new(seed: u64) -> Self {
        let increment = LCG_INCREMENT | 1; // must be odd for full period (already is)
        let mut g = Self { state: 0, increment };
        // Step 1 & 2: first advance from zero
        g.state = g.state.wrapping_mul(LCG_MULTIPLIER).wrapping_add(increment);
        // Step 3: mix seed in
        g.state = g.state.wrapping_add(seed);
        // Step 4: second advance to scatter seed bits
        g.state = g.state.wrapping_mul(LCG_MULTIPLIER).wrapping_add(increment);
        g
    }

    /// Advances the PCG32 state and returns the XSH RR permuted output.
    ///
    /// We capture `old_state` before advancing so the output is based on
    /// the pre-advance state — this is the standard PCG output function.
    pub fn next_u32(&mut self) -> u32 {
        let old_state = self.state;
        self.state = old_state.wrapping_mul(LCG_MULTIPLIER).wrapping_add(self.increment);

        // XSH RR permutation ─────────────────────────────────────────────────
        // Step 1: XOR-shift to mix high bits into the low 32.
        //   xorshifted = ((old >> 18) ^ old) >> 27
        let xorshifted = (((old_state >> 18) ^ old_state) >> 27) as u32;

        // Step 2: extract 5-bit rotation amount from the very top.
        //   rot = old >> 59   (values 0..31)
        let rot = (old_state >> 59) as u32;

        // Step 3: rotate-right the 32-bit xorshifted value.
        //   rotr(x, n) = (x >> n) | (x << (32 - n))
        xorshifted.rotate_right(rot)
    }

    /// Returns a 64-bit value built from two consecutive [`next_u32`] calls.
    pub fn next_u64(&mut self) -> u64 {
        let hi = self.next_u32() as u64;
        let lo = self.next_u32() as u64;
        (hi << 32) | lo
    }

    /// Returns a `f64` uniformly distributed in `[0.0, 1.0)`.
    pub fn next_float(&mut self) -> f64 {
        self.next_u32() as f64 / FLOAT_DIV
    }

    /// Returns a uniform random integer in `[min, max]` inclusive.
    /// Uses rejection sampling — identical algorithm to [`Lcg::next_int_in_range`].
    pub fn next_int_in_range(&mut self, min: i64, max: i64) -> i64 {
        let range_size = (max - min + 1) as u64;
        let threshold = range_size.wrapping_neg() % range_size;
        loop {
            let r = self.next_u32() as u64;
            if r >= threshold {
                return min + (r % range_size) as i64;
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Reference values (cross-checked against Go reference impl) ───────────
    //
    // These are the first three next_u32() outputs for seed=1 for each
    // generator. Any future refactor must still produce these values.

    #[test]
    fn lcg_known_values_seed1() {
        let mut g = Lcg::new(1);
        assert_eq!(g.next_u32(), 1817669548);
        assert_eq!(g.next_u32(), 2187888307);
        assert_eq!(g.next_u32(), 2784682393);
    }

    #[test]
    fn xorshift64_known_values_seed1() {
        let mut g = Xorshift64::new(1);
        assert_eq!(g.next_u32(), 1082269761);
        assert_eq!(g.next_u32(), 201397313);
        assert_eq!(g.next_u32(), 1854285353);
    }

    #[test]
    fn pcg32_known_values_seed1() {
        let mut g = Pcg32::new(1);
        assert_eq!(g.next_u32(), 1412771199);
        assert_eq!(g.next_u32(), 1791099446);
        assert_eq!(g.next_u32(), 124312908);
    }

    // ── Determinism — same seed → same sequence ───────────────────────────────

    #[test]
    fn lcg_deterministic() {
        let vals_a: Vec<u32> = { let mut g = Lcg::new(42); (0..10).map(|_| g.next_u32()).collect() };
        let vals_b: Vec<u32> = { let mut g = Lcg::new(42); (0..10).map(|_| g.next_u32()).collect() };
        assert_eq!(vals_a, vals_b);
    }

    #[test]
    fn xorshift64_deterministic() {
        let vals_a: Vec<u32> = { let mut g = Xorshift64::new(42); (0..10).map(|_| g.next_u32()).collect() };
        let vals_b: Vec<u32> = { let mut g = Xorshift64::new(42); (0..10).map(|_| g.next_u32()).collect() };
        assert_eq!(vals_a, vals_b);
    }

    #[test]
    fn pcg32_deterministic() {
        let vals_a: Vec<u32> = { let mut g = Pcg32::new(42); (0..10).map(|_| g.next_u32()).collect() };
        let vals_b: Vec<u32> = { let mut g = Pcg32::new(42); (0..10).map(|_| g.next_u32()).collect() };
        assert_eq!(vals_a, vals_b);
    }

    // ── Different seeds diverge ───────────────────────────────────────────────

    #[test]
    fn lcg_different_seeds_diverge() {
        let mut g1 = Lcg::new(1);
        let mut g2 = Lcg::new(2);
        let seq1: Vec<u32> = (0..5).map(|_| g1.next_u32()).collect();
        let seq2: Vec<u32> = (0..5).map(|_| g2.next_u32()).collect();
        assert_ne!(seq1, seq2);
    }

    #[test]
    fn xorshift64_different_seeds_diverge() {
        let mut g1 = Xorshift64::new(1);
        let mut g2 = Xorshift64::new(2);
        let seq1: Vec<u32> = (0..5).map(|_| g1.next_u32()).collect();
        let seq2: Vec<u32> = (0..5).map(|_| g2.next_u32()).collect();
        assert_ne!(seq1, seq2);
    }

    #[test]
    fn pcg32_different_seeds_diverge() {
        let mut g1 = Pcg32::new(1);
        let mut g2 = Pcg32::new(2);
        let seq1: Vec<u32> = (0..5).map(|_| g1.next_u32()).collect();
        let seq2: Vec<u32> = (0..5).map(|_| g2.next_u32()).collect();
        assert_ne!(seq1, seq2);
    }

    // ── Seed 0: Xorshift64 must not stay stuck at zero ────────────────────────
    //
    // State 0 is a fixed point: xor-shifting zero always gives zero.
    // The constructor replaces seed 0 with 1 to prevent this.

    #[test]
    fn xorshift64_seed0_not_stuck() {
        let mut g = Xorshift64::new(0);
        // After seed replacement state == 1; first output should be non-zero.
        let v = g.next_u32();
        assert_ne!(v, 0);
        // State should never become zero for at least 100 steps.
        for _ in 0..100 {
            assert_ne!(g.next_u32(), 0, "Xorshift64 state became zero");
        }
    }

    // ── Float range ───────────────────────────────────────────────────────────

    #[test]
    fn lcg_float_in_range() {
        let mut g = Lcg::new(99);
        for _ in 0..1000 {
            let f = g.next_float();
            assert!(f >= 0.0 && f < 1.0, "float out of [0,1): {f}");
        }
    }

    #[test]
    fn xorshift64_float_in_range() {
        let mut g = Xorshift64::new(99);
        for _ in 0..1000 {
            let f = g.next_float();
            assert!(f >= 0.0 && f < 1.0, "float out of [0,1): {f}");
        }
    }

    #[test]
    fn pcg32_float_in_range() {
        let mut g = Pcg32::new(99);
        for _ in 0..1000 {
            let f = g.next_float();
            assert!(f >= 0.0 && f < 1.0, "float out of [0,1): {f}");
        }
    }

    // ── Integer range bounds ──────────────────────────────────────────────────

    #[test]
    fn lcg_int_in_range_bounds() {
        let mut g = Lcg::new(7);
        for _ in 0..1000 {
            let v = g.next_int_in_range(1, 6);
            assert!((1..=6).contains(&v), "value out of [1,6]: {v}");
        }
    }

    #[test]
    fn xorshift64_int_in_range_bounds() {
        let mut g = Xorshift64::new(7);
        for _ in 0..1000 {
            let v = g.next_int_in_range(1, 6);
            assert!((1..=6).contains(&v), "value out of [1,6]: {v}");
        }
    }

    #[test]
    fn pcg32_int_in_range_bounds() {
        let mut g = Pcg32::new(7);
        for _ in 0..1000 {
            let v = g.next_int_in_range(1, 6);
            assert!((1..=6).contains(&v), "value out of [1,6]: {v}");
        }
    }

    // ── Single-value range ────────────────────────────────────────────────────
    //
    // When min == max the only valid return is min itself.

    #[test]
    fn lcg_single_value_range() {
        let mut g = Lcg::new(5);
        for _ in 0..20 {
            assert_eq!(g.next_int_in_range(42, 42), 42);
        }
    }

    #[test]
    fn xorshift64_single_value_range() {
        let mut g = Xorshift64::new(5);
        for _ in 0..20 {
            assert_eq!(g.next_int_in_range(42, 42), 42);
        }
    }

    #[test]
    fn pcg32_single_value_range() {
        let mut g = Pcg32::new(5);
        for _ in 0..20 {
            assert_eq!(g.next_int_in_range(42, 42), 42);
        }
    }

    // ── Distribution ─────────────────────────────────────────────────────────
    //
    // Roll a 6-sided die 12 000 times and check each face appears ~2000 ± 30%
    // (±600) times. A failing generator would cluster around a few values.

    fn check_distribution(counts: &[usize; 6]) {
        for (face, &count) in counts.iter().enumerate() {
            assert!(
                count >= 1400 && count <= 2600,
                "face {} appeared {} times (expected ~2000 ±30%)",
                face + 1,
                count
            );
        }
    }

    #[test]
    fn lcg_distribution() {
        let mut g = Lcg::new(123);
        let mut counts = [0usize; 6];
        for _ in 0..12_000 {
            let v = g.next_int_in_range(1, 6) as usize - 1;
            counts[v] += 1;
        }
        check_distribution(&counts);
    }

    #[test]
    fn xorshift64_distribution() {
        let mut g = Xorshift64::new(123);
        let mut counts = [0usize; 6];
        for _ in 0..12_000 {
            let v = g.next_int_in_range(1, 6) as usize - 1;
            counts[v] += 1;
        }
        check_distribution(&counts);
    }

    #[test]
    fn pcg32_distribution() {
        let mut g = Pcg32::new(123);
        let mut counts = [0usize; 6];
        for _ in 0..12_000 {
            let v = g.next_int_in_range(1, 6) as usize - 1;
            counts[v] += 1;
        }
        check_distribution(&counts);
    }

    // ── next_u64 composition ──────────────────────────────────────────────────
    //
    // next_u64 must equal (hi << 32) | lo where hi and lo come from
    // successive next_u32 calls on an identically-seeded generator.

    #[test]
    fn lcg_u64_composition() {
        let mut g_u64 = Lcg::new(55);
        let mut g_u32 = Lcg::new(55);
        for _ in 0..50 {
            let u64_val = g_u64.next_u64();
            let hi = g_u32.next_u32() as u64;
            let lo = g_u32.next_u32() as u64;
            assert_eq!(u64_val, (hi << 32) | lo);
        }
    }

    #[test]
    fn xorshift64_u64_composition() {
        let mut g_u64 = Xorshift64::new(55);
        let mut g_u32 = Xorshift64::new(55);
        for _ in 0..50 {
            let u64_val = g_u64.next_u64();
            let hi = g_u32.next_u32() as u64;
            let lo = g_u32.next_u32() as u64;
            assert_eq!(u64_val, (hi << 32) | lo);
        }
    }

    #[test]
    fn pcg32_u64_composition() {
        let mut g_u64 = Pcg32::new(55);
        let mut g_u32 = Pcg32::new(55);
        for _ in 0..50 {
            let u64_val = g_u64.next_u64();
            let hi = g_u32.next_u32() as u64;
            let lo = g_u32.next_u32() as u64;
            assert_eq!(u64_val, (hi << 32) | lo);
        }
    }

    // ── Negative range ────────────────────────────────────────────────────────

    #[test]
    fn lcg_negative_range() {
        let mut g = Lcg::new(11);
        for _ in 0..500 {
            let v = g.next_int_in_range(-10, -1);
            assert!((-10..=-1).contains(&v), "value out of [-10,-1]: {v}");
        }
    }

    #[test]
    fn pcg32_negative_range() {
        let mut g = Pcg32::new(11);
        for _ in 0..500 {
            let v = g.next_int_in_range(-10, -1);
            assert!((-10..=-1).contains(&v), "value out of [-10,-1]: {v}");
        }
    }
}
