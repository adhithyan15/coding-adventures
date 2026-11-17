//! Statistical quality measures for hash functions: `avalanche_score` and
//! `distribution_test` (DT17, "Analysis helpers").
//!
//! This crate was split out of `coding_adventures_hash_functions` (WX-P1a2).  The
//! hash functions are pure computation and carry a Warrant document and a
//! no-capability declaration.  `avalanche_score` has to draw random inputs from the
//! operating system (`getrandom`), which is the `rand:read:*` capability.  Keeping
//! the two apart means the pure crate can say `"capabilities": []` truthfully, and
//! the randomness is declared, once, here.
//!
//! Both helpers take any closure `Fn(&[u8]) -> H` with `H: Into<u64>`, so they
//! measure any hash, not only the ones in `coding_adventures_hash_functions`:
//!
//! ```
//! use coding_adventures_hash_analysis::{avalanche_score, distribution_test};
//! use coding_adventures_hash_functions::fnv1a_32;
//!
//! // A constant-sized sample of random 8-byte inputs: every input bit is
//! // flipped and the fraction of output bits that change is averaged.
//! let score = avalanche_score(fnv1a_32, 32, 8);
//! assert!((0.0..=1.0).contains(&score));
//!
//! // Chi-squared of the bucket counts: 0 is perfectly even.
//! let chi2 = distribution_test(fnv1a_32, [&b"alpha"[..], &b"beta"[..]], 8);
//! assert!(chi2 >= 0.0);
//! ```

use getrandom::getrandom;

/// Estimate the average fraction of output bits that flip when a single input
/// bit is toggled.
pub fn avalanche_score<F, H>(hash_fn: F, output_bits: u32, sample_size: usize) -> f64
where
    F: Fn(&[u8]) -> H,
    H: Into<u64>,
{
    assert!(sample_size > 0, "sample_size must be positive");
    avalanche_score_with_source(hash_fn, output_bits, sample_size, |buf| {
        getrandom(buf).expect("failed to obtain random bytes for avalanche analysis")
    })
}

fn avalanche_score_with_source<F, H, R>(
    hash_fn: F,
    output_bits: u32,
    sample_size: usize,
    mut fill_random: R,
) -> f64
where
    F: Fn(&[u8]) -> H,
    H: Into<u64>,
    R: FnMut(&mut [u8]),
{
    assert!(output_bits > 0 && output_bits <= 64, "output_bits must be in 1..=64");

    let mut total_bit_flips = 0u64;
    let mut total_trials = 0u64;
    let mut input_bytes = [0u8; 8];

    for _ in 0..sample_size {
        fill_random(&mut input_bytes);
        let h1 = hash_fn(&input_bytes).into();

        for bit_pos in 0..(input_bytes.len() * 8) {
            let byte_idx = bit_pos / 8;
            let bit_mask = 1u8 << (bit_pos % 8);

            let mut flipped = input_bytes;
            flipped[byte_idx] ^= bit_mask;
            let h2 = hash_fn(&flipped).into();

            total_bit_flips += (h1 ^ h2).count_ones() as u64;
            total_trials += u64::from(output_bits);
        }
    }

    total_bit_flips as f64 / total_trials as f64
}

/// Measure how evenly hash outputs distribute across buckets.
pub fn distribution_test<F, H, I, B>(hash_fn: F, inputs: I, num_buckets: usize) -> f64
where
    F: Fn(&[u8]) -> H,
    H: Into<u64>,
    I: IntoIterator<Item = B>,
    B: AsRef<[u8]>,
{
    assert!(num_buckets > 0, "num_buckets must be positive");

    let mut counts = vec![0u64; num_buckets];
    for inp in inputs {
        let bucket = (hash_fn(inp.as_ref()).into() % num_buckets as u64) as usize;
        counts[bucket] += 1;
    }

    let total: u64 = counts.iter().sum();
    assert!(total > 0, "inputs must not be empty");
    let expected = total as f64 / num_buckets as f64;
    counts
        .into_iter()
        .map(|observed| {
            let observed = observed as f64;
            let delta = observed - expected;
            delta * delta / expected
        })
        .sum()
}

// Run the README's usage example as a doctest (`cargo test --doc`).
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_hash_functions::{fnv1a_32, murmur3_32, siphash_2_4};

    fn deterministic_fill(mut seed: u64) -> impl FnMut(&mut [u8]) {
        move |buf| {
            for byte in buf.iter_mut() {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                *byte = (seed >> 24) as u8;
            }
        }
    }

    #[test]
    fn avalanche_score_is_zero_for_constant_hash() {
        let score = avalanche_score_with_source(|_| 0u64, 32, 4, deterministic_fill(1));
        assert_eq!(score, 0.0);
    }

    #[test]
    fn distribution_test_matches_exact_constant_hash_math() {
        let inputs = [b"a".as_slice(), b"b".as_slice(), b"c".as_slice(), b"d".as_slice()];
        let chi2 = distribution_test(|_| 0u64, inputs, 4);
        assert_eq!(chi2, 12.0);
    }

    #[test]
    fn distribution_test_handles_non_slice_iterators() {
        let inputs = vec![b"hello".to_vec(), b"world".to_vec()];
        let chi2 = distribution_test(|data| data.len() as u64, inputs, 4);
        assert!(chi2 >= 0.0);
    }

    #[test]
    fn avalanche_score_with_a_real_hash_is_a_fraction() {
        let score =
            avalanche_score_with_source(fnv1a_32, 32, 64, deterministic_fill(7));
        assert!(score > 0.2 && score < 0.8, "fnv1a_32 avalanche {score}");
    }

    #[test]
    fn avalanche_score_of_a_strong_hash_is_close_to_one_half() {
        let key = [7u8; 16];
        let score = avalanche_score_with_source(
            |d| siphash_2_4(d, &key),
            64,
            64,
            deterministic_fill(11),
        );
        assert!((score - 0.5).abs() < 0.05, "siphash avalanche {score}");
    }

    #[test]
    fn avalanche_score_uses_the_operating_system_source() {
        // The public entry point draws real randomness; the result is still a
        // fraction and a constant hash still scores exactly zero.
        assert_eq!(avalanche_score(|_| 0u32, 32, 3), 0.0);
        let s = avalanche_score(murmur3_32, 32, 16);
        assert!((0.0..=1.0).contains(&s));
    }

    #[test]
    fn avalanche_score_counts_only_the_declared_output_bits() {
        // An identity-like hash of the first byte flips exactly one output bit
        // when that byte's bit flips, and none otherwise: 8 of 64 inputs bits
        // matter, each flips 1 of 8 output bits => 8 / (64 * 8).
        let score = avalanche_score_with_source(|d| d[0], 8, 5, deterministic_fill(3));
        assert!((score - 8.0 / (64.0 * 8.0)).abs() < 1e-12, "{score}");
    }

    #[test]
    #[should_panic(expected = "sample_size must be positive")]
    fn avalanche_score_rejects_zero_samples() {
        avalanche_score(fnv1a_32, 32, 0);
    }

    #[test]
    #[should_panic(expected = "output_bits must be in 1..=64")]
    fn avalanche_score_rejects_zero_output_bits() {
        avalanche_score_with_source(fnv1a_32, 0, 1, deterministic_fill(1));
    }

    #[test]
    #[should_panic(expected = "output_bits must be in 1..=64")]
    fn avalanche_score_rejects_more_than_64_output_bits() {
        avalanche_score_with_source(fnv1a_32, 65, 1, deterministic_fill(1));
    }

    #[test]
    fn distribution_test_is_zero_for_a_perfectly_even_spread() {
        let inputs: Vec<[u8; 1]> = (0u8..8).collect::<Vec<_>>().into_iter().map(|b| [b]).collect();
        let chi2 = distribution_test(|d| u64::from(d[0]), inputs, 8);
        assert_eq!(chi2, 0.0);
    }

    #[test]
    fn distribution_test_accepts_a_real_hash() {
        let inputs: Vec<Vec<u8>> = (0u32..1000).map(|i| i.to_le_bytes().to_vec()).collect();
        let chi2 = distribution_test(fnv1a_32, inputs, 10);
        assert!(chi2.is_finite() && chi2 >= 0.0);
    }

    #[test]
    #[should_panic(expected = "num_buckets must be positive")]
    fn distribution_test_rejects_zero_buckets() {
        distribution_test(fnv1a_32, [b"a"], 0);
    }

    #[test]
    #[should_panic(expected = "inputs must not be empty")]
    fn distribution_test_rejects_empty_input() {
        let none: [&[u8]; 0] = [];
        distribution_test(fnv1a_32, none, 4);
    }
}
