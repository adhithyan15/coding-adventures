//! DT22 Bloom filter for probabilistic membership tests.

use core::fmt;

use coding_adventures_hash_functions::{djb2, fnv1a_32};

const DEFAULT_EXPECTED_ITEMS: usize = 1_000;
const DEFAULT_FALSE_POSITIVE_RATE: f64 = 0.01;
const MASK32: u32 = 0xFFFF_FFFF;

#[derive(Debug, Clone, PartialEq)]
pub enum BloomFilterError {
    InvalidExpectedItems { expected_items: usize },
    InvalidFalsePositiveRate { false_positive_rate: f64 },
    InvalidBitCount { bit_count: usize },
    InvalidHashCount { hash_count: usize },
}

impl fmt::Display for BloomFilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidExpectedItems { expected_items } => {
                write!(f, "expected_items must be positive, got {expected_items}")
            }
            Self::InvalidFalsePositiveRate { false_positive_rate } => write!(
                f,
                "false_positive_rate must be in the open interval (0, 1), got {false_positive_rate}"
            ),
            Self::InvalidBitCount { bit_count } => {
                write!(f, "bit_count must be positive, got {bit_count}")
            }
            Self::InvalidHashCount { hash_count } => {
                write!(f, "hash_count must be positive, got {hash_count}")
            }
        }
    }
}

impl std::error::Error for BloomFilterError {}

#[derive(Clone, PartialEq, Eq)]
pub struct BloomFilter {
    bit_count: usize,
    hash_count: usize,
    expected_items: usize,
    bits: Vec<u8>,
    bits_set: usize,
    items_added: usize,
}

impl Default for BloomFilter {
    fn default() -> Self {
        Self::new(DEFAULT_EXPECTED_ITEMS, DEFAULT_FALSE_POSITIVE_RATE)
    }
}

impl BloomFilter {
    pub fn new(expected_items: usize, false_positive_rate: f64) -> Self {
        Self::try_new(expected_items, false_positive_rate)
            .expect("invalid BloomFilter configuration")
    }

    pub fn try_new(
        expected_items: usize,
        false_positive_rate: f64,
    ) -> Result<Self, BloomFilterError> {
        if expected_items == 0 {
            return Err(BloomFilterError::InvalidExpectedItems { expected_items });
        }
        if !(0.0 < false_positive_rate && false_positive_rate < 1.0) {
            return Err(BloomFilterError::InvalidFalsePositiveRate {
                false_positive_rate,
            });
        }

        let bit_count = Self::optimal_m(expected_items, false_positive_rate);
        let hash_count = Self::optimal_k(bit_count, expected_items);
        Ok(Self::from_parts(bit_count, hash_count, expected_items))
    }

    pub fn from_params(bit_count: usize, hash_count: usize) -> Self {
        Self::try_from_params(bit_count, hash_count)
            .expect("invalid BloomFilter parameters")
    }

    pub fn try_from_params(
        bit_count: usize,
        hash_count: usize,
    ) -> Result<Self, BloomFilterError> {
        if bit_count == 0 {
            return Err(BloomFilterError::InvalidBitCount { bit_count });
        }
        if hash_count == 0 {
            return Err(BloomFilterError::InvalidHashCount { hash_count });
        }
        Ok(Self::from_parts(bit_count, hash_count, 0))
    }

    pub fn add<T: fmt::Debug>(&mut self, element: T) {
        for idx in self.hash_indices(element) {
            let byte_idx = idx / 8;
            let bit_mask = 1u8 << (idx % 8);
            if self.bits[byte_idx] & bit_mask == 0 {
                self.bits[byte_idx] |= bit_mask;
                self.bits_set += 1;
            }
        }
        self.items_added += 1;
    }

    pub fn contains<T: fmt::Debug>(&self, element: T) -> bool {
        self.hash_indices(element).into_iter().all(|idx| {
            let byte_idx = idx / 8;
            let bit_mask = 1u8 << (idx % 8);
            self.bits[byte_idx] & bit_mask != 0
        })
    }

    pub fn bit_count(&self) -> usize {
        self.bit_count
    }

    pub fn hash_count(&self) -> usize {
        self.hash_count
    }

    pub fn bits_set(&self) -> usize {
        self.bits_set
    }

    pub fn fill_ratio(&self) -> f64 {
        if self.bit_count == 0 {
            0.0
        } else {
            self.bits_set as f64 / self.bit_count as f64
        }
    }

    pub fn estimated_false_positive_rate(&self) -> f64 {
        if self.bits_set == 0 {
            0.0
        } else {
            self.fill_ratio().powi(self.hash_count as i32)
        }
    }

    pub fn is_over_capacity(&self) -> bool {
        if self.expected_items == 0 {
            return false;
        }
        self.items_added > self.expected_items
    }

    pub fn size_bytes(&self) -> usize {
        self.bits.len()
    }

    pub fn optimal_m(n: usize, p: f64) -> usize {
        ((-(n as f64) * p.ln()) / (2.0_f64.ln().powi(2))).ceil() as usize
    }

    pub fn optimal_k(m: usize, n: usize) -> usize {
        ((m as f64 / n as f64) * 2.0_f64.ln()).round().max(1.0) as usize
    }

    pub fn capacity_for_memory(memory_bytes: usize, p: f64) -> usize {
        let m = memory_bytes * 8;
        (-(m as f64) * 2.0_f64.ln().powi(2) / p.ln()).floor() as usize
    }

    fn from_parts(bit_count: usize, hash_count: usize, expected_items: usize) -> Self {
        let byte_count = (bit_count + 7) / 8;
        Self {
            bit_count,
            hash_count,
            expected_items,
            bits: vec![0; byte_count],
            bits_set: 0,
            items_added: 0,
        }
    }

    fn hash_indices<T: fmt::Debug>(&self, element: T) -> Vec<usize> {
        let raw = format!("{:?}", element);
        let h1 = fmix32(fnv1a_32(raw.as_bytes()));
        let h2_raw = djb2(raw.as_bytes());
        let folded = ((h2_raw ^ (h2_raw >> 32)) as u32) & MASK32;
        let mut h2 = fmix32(folded);
        h2 |= 1;

        (0..self.hash_count)
            .map(|i| {
                let idx = (h1 as u64 + i as u64 * h2 as u64) % self.bit_count as u64;
                idx as usize
            })
            .collect()
    }
}

impl fmt::Debug for BloomFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pct_set = self.fill_ratio() * 100.0;
        let est_fp = self.estimated_false_positive_rate() * 100.0;
        write!(
            f,
            "BloomFilter(m={}, k={}, bits_set={}/{}, ({pct_set:.2}%), ~fp={est_fp:.4}%)",
            self.bit_count, self.hash_count, self.bits_set, self.bit_count
        )
    }
}

impl fmt::Display for BloomFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

fn fmix32(mut h: u32) -> u32 {
    h ^= h >> 16;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 16;
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_filter_is_empty() {
        let bf = BloomFilter::default();
        assert_eq!(bf.bits_set(), 0);
        assert_eq!(bf.fill_ratio(), 0.0);
        assert!(!bf.is_over_capacity());
    }

    #[test]
    fn add_and_contains_work() {
        let mut bf = BloomFilter::default();
        bf.add("hello");
        assert!(bf.contains("hello"));
    }

    #[test]
    fn no_false_negatives_for_inserted_values() {
        let mut bf = BloomFilter::new(1_000, 0.01);
        for i in 0..200 {
            bf.add(format!("item-{i}"));
        }
        for i in 0..200 {
            assert!(bf.contains(format!("item-{i}")));
        }
    }

    #[test]
    fn stats_are_consistent() {
        let mut bf = BloomFilter::new(100, 0.01);
        bf.add("alpha");
        assert!(bf.bit_count() > 0);
        assert!(bf.hash_count() >= 1);
        assert!(bf.bits_set() > 0);
        assert!(bf.size_bytes() > 0);
    }

    #[test]
    fn parameter_helpers_match_expectations() {
        let m = BloomFilter::optimal_m(1_000_000, 0.01);
        let k = BloomFilter::optimal_k(m, 1_000_000);
        assert!(m > 9_000_000);
        assert_eq!(k, 7);
        assert_eq!(BloomFilter::capacity_for_memory(1_000_000, 0.01) > 0, true);
    }

    #[test]
    fn invalid_params_are_rejected() {
        assert!(BloomFilter::try_new(0, 0.01).is_err());
        assert!(BloomFilter::try_new(1, 0.0).is_err());
        assert!(BloomFilter::try_from_params(0, 1).is_err());
        assert!(BloomFilter::try_from_params(1, 0).is_err());
    }
}
