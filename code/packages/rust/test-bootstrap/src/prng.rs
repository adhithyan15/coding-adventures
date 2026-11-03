//! # Deterministic randomness for test runs (TF01 §5.1)
//!
//! A run's order must be *random* (so tests cannot quietly depend on each
//! other) yet *reproducible* (so a failure seen once can be seen again). The
//! answer is a seeded pseudo-random generator whose every step is pinned by
//! the spec, so the same seed gives the same order in every language.
//!
//! Nothing here is suitable for cryptography; it only shuffles tests.

/// The SplitMix64 generator: 64 bits of state, advanced by a fixed odd
/// constant (the "golden gamma"), then scrambled by a finaliser.
///
/// ```text
///   state += 0x9e3779b97f4a7c15                 (wrapping)
///   z = state
///   z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9    (wrapping)
///   z = (z ^ (z >> 27)) * 0x94d049bb133111eb    (wrapping)
///   output z ^ (z >> 31)
/// ```
///
/// The published reference vector: seed 0 first outputs `0xe220a8397b1dcdaf`.
#[derive(Clone, Debug)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// A generator whose first output is derived from `seed`.
    pub fn new(seed: u64) -> Self {
        SplitMix64 { state: seed }
    }

    /// The next 64-bit output.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A uniformly distributed integer in `0..n`, for `1 <= n <= 2^32`.
    ///
    /// Plain `next_u64() % n` is biased whenever `n` does not divide 2^64:
    /// the low remainders get one extra chance each. So we discard the
    /// `t = 2^64 mod n` smallest outputs first, which leaves a range whose
    /// size is an exact multiple of `n`:
    ///
    /// ```text
    ///   n = 3:  2^64 mod 3 = 1  →  reject x = 0, keep 1 ..= 2^64-1
    ///           (2^64 - 1 values, divisible by 3)  →  return x % 3
    /// ```
    ///
    /// `(2^64 - n) mod n` equals `2^64 mod n`, and in wrapping arithmetic
    /// `2^64 - n` is just `n.wrapping_neg()`. At most half the outputs are ever
    /// rejected, so the loop ends quickly.
    ///
    /// # Panics
    ///
    /// If `n` is 0 or above 2^32. The bound is part of the protocol: every
    /// lane must draw identically, and 2^32 is the largest range every
    /// language can represent without a big-integer type.
    pub fn bounded(&mut self, n: u64) -> u64 {
        assert!(
            (1..=1u64 << 32).contains(&n),
            "bounded(n) needs 1 <= n <= 2^32"
        );
        let threshold = n.wrapping_neg() % n;
        loop {
            let x = self.next_u64();
            if x >= threshold {
                return x % n;
            }
        }
    }
}

/// FNV-1a, 64-bit: a tiny, well-known hash used to turn a test id into
/// seed material. Start from the offset basis; for each byte, XOR it in and
/// multiply by the FNV prime, wrapping.
///
/// ```text
///   fnv1a64(b"")  = 0xcbf29ce484222325   (the offset basis)
///   fnv1a64(b"a") = 0xaf63dc4c8601ec8c
/// ```
pub fn fnv1a64(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in data {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The run order for a set of test ids (TF01 §5.2).
///
/// 1. Sort by byte order. Registration order is never used: in some
///    languages (JVM and .NET reflection) it is unspecified, and the order
///    must be the same everywhere.
/// 2. Fisher–Yates, driven by `SplitMix64(seed)`: for `i` from the last
///    index down to 1, swap item `i` with item `bounded(i + 1)`.
///
/// # Panics
///
/// If there are more than 2^32 ids (the bounded draw's limit).
pub fn shuffled<S: AsRef<str>>(seed: u64, ids: &[S]) -> Vec<String> {
    let mut order: Vec<String> = ids.iter().map(|id| id.as_ref().to_owned()).collect();
    // `str`'s ordering is lexicographic over its UTF-8 bytes: byte order.
    order.sort();
    let mut rng = SplitMix64::new(seed);
    for i in (1..order.len()).rev() {
        let j = rng.bounded(i as u64 + 1) as usize;
        order.swap(i, j);
    }
    order
}

/// A test's own seed: the first output of
/// `SplitMix64(run_seed XOR fnv1a64(utf8(id)))`.
///
/// It depends only on the run seed and the test's id, never on where the
/// shuffle put the test, so adding a test never changes another's randomness.
pub fn sub_seed(run_seed: u64, test_id: &str) -> u64 {
    SplitMix64::new(run_seed ^ fnv1a64(test_id.as_bytes())).next_u64()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_zero_matches_the_published_reference() {
        assert_eq!(SplitMix64::new(0).next_u64(), 0xe220_a839_7b1d_cdaf);
    }

    #[test]
    fn fnv_known_values() {
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn bounded_stays_in_range_at_both_limits() {
        let mut rng = SplitMix64::new(9);
        for _ in 0..1000 {
            assert_eq!(rng.bounded(1), 0);
            assert!(rng.bounded(1 << 32) < 1 << 32);
            assert!(rng.bounded(3) < 3);
        }
    }

    #[test]
    #[should_panic(expected = "1 <= n <= 2^32")]
    fn bounded_rejects_zero() {
        SplitMix64::new(0).bounded(0);
    }

    #[test]
    #[should_panic(expected = "1 <= n <= 2^32")]
    fn bounded_rejects_more_than_two_to_the_32() {
        SplitMix64::new(0).bounded((1 << 32) + 1);
    }

    #[test]
    fn shuffle_ignores_registration_order() {
        let a = shuffled(5, &["b", "a", "c"]);
        let b = shuffled(5, &["c", "b", "a"]);
        assert_eq!(a, b);
        let mut sorted = a.clone();
        sorted.sort();
        assert_eq!(sorted, ["a", "b", "c"]);
    }

    #[test]
    fn sub_seed_depends_on_the_id() {
        assert_ne!(sub_seed(0, "a"), sub_seed(0, "b"));
        assert_ne!(sub_seed(0, "a"), sub_seed(1, "a"));
    }
}
