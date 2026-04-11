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

#[cfg(test)]
mod tests {
    use super::*;

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
}
