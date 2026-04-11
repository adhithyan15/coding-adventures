//! DT21 HyperLogLog approximate cardinality estimation.
//!
//! The implementation mirrors the Python spec closely enough for the
//! published behavior: fixed register count, FNV-1a hashing plus Murmur-style
//! avalanche mixing, harmonic-mean estimation, and linear-counting correction
//! for small cardinalities.

use core::fmt;

use coding_adventures_hash_functions::fnv1a_64;

const DEFAULT_PRECISION: u8 = 14;
const MIN_PRECISION: u8 = 4;
const MAX_PRECISION: u8 = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HyperLogLogError {
    InvalidPrecision { precision: u8 },
    PrecisionMismatch { left: u8, right: u8 },
}

impl fmt::Display for HyperLogLogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPrecision { precision } => write!(
                f,
                "precision must be between {MIN_PRECISION} and {MAX_PRECISION}, got {precision}"
            ),
            Self::PrecisionMismatch { left, right } => {
                write!(f, "precision mismatch: {left} vs {right}")
            }
        }
    }
}

impl std::error::Error for HyperLogLogError {}

#[derive(Clone, PartialEq, Eq)]
pub struct HyperLogLog {
    precision: u8,
    registers: Vec<u8>,
}

impl Default for HyperLogLog {
    fn default() -> Self {
        Self::new()
    }
}

impl HyperLogLog {
    pub fn new() -> Self {
        Self::with_precision(DEFAULT_PRECISION)
    }

    pub fn with_precision(precision: u8) -> Self {
        Self::try_with_precision(precision).expect("invalid HyperLogLog precision")
    }

    pub fn try_with_precision(precision: u8) -> Result<Self, HyperLogLogError> {
        if !(MIN_PRECISION..=MAX_PRECISION).contains(&precision) {
            return Err(HyperLogLogError::InvalidPrecision { precision });
        }
        let register_count = 1usize << precision;
        Ok(Self {
            precision,
            registers: vec![0; register_count],
        })
    }

    pub fn add<T: fmt::Debug>(&mut self, element: T) {
        self.add_bytes(format!("{:?}", element).as_bytes());
    }

    pub fn add_bytes(&mut self, bytes: &[u8]) {
        let mut h = fnv1a_64(bytes);
        h = fmix64(h);

        let precision = self.precision as u32;
        let bucket = (h >> (64 - precision)) as usize;
        let remaining_bits = 64 - precision;
        let remaining = if remaining_bits == 64 {
            h
        } else {
            h & ((1u64 << remaining_bits) - 1)
        };
        let rho = count_leading_zeros(remaining, remaining_bits as u8) + 1;

        if rho > self.registers[bucket] as u32 {
            self.registers[bucket] = rho as u8;
        }
    }

    pub fn count(&self) -> usize {
        let m = self.num_registers() as f64;
        let z_sum: f64 = self
            .registers
            .iter()
            .map(|&r| 2.0_f64.powi(-(r as i32)))
            .sum();
        let alpha = alpha_for_registers(self.num_registers());
        let mut estimate = alpha * m * m / z_sum;

        if estimate <= 2.5 * m {
            let zeros = self.registers.iter().filter(|&&r| r == 0).count();
            if zeros > 0 {
                estimate = m * (m / zeros as f64).ln();
            }
        }

        let two_32 = (1u64 << 32) as f64;
        if estimate > two_32 / 30.0 {
            let ratio = 1.0 - estimate / two_32;
            if ratio > 0.0 {
                estimate = -two_32 * ratio.ln();
            }
        }

        estimate.round().max(0.0) as usize
    }

    pub fn merge(&self, other: &Self) -> Self {
        self.try_merge(other)
            .expect("cannot merge HyperLogLog sketches with different precisions")
    }

    pub fn try_merge(&self, other: &Self) -> Result<Self, HyperLogLogError> {
        if self.precision != other.precision {
            return Err(HyperLogLogError::PrecisionMismatch {
                left: self.precision,
                right: other.precision,
            });
        }
        let mut merged = Self::with_precision(self.precision);
        merged.registers = self
            .registers
            .iter()
            .zip(other.registers.iter())
            .map(|(a, b)| (*a).max(*b))
            .collect();
        Ok(merged)
    }

    pub fn len(&self) -> usize {
        self.count()
    }

    pub fn precision(&self) -> u8 {
        self.precision
    }

    pub fn num_registers(&self) -> usize {
        self.registers.len()
    }

    pub fn error_rate(&self) -> f64 {
        Self::error_rate_for_precision(self.precision)
    }

    pub fn error_rate_for_precision(precision: u8) -> f64 {
        let m = 1usize << precision;
        1.04 / (m as f64).sqrt()
    }

    pub fn memory_bytes(precision: u8) -> usize {
        let m = 1usize << precision;
        (m * 6) / 8
    }

    pub fn optimal_precision(desired_error: f64) -> u8 {
        let min_m = (1.04 / desired_error).powi(2);
        let precision = min_m.log2().ceil() as u8;
        precision.clamp(MIN_PRECISION, MAX_PRECISION)
    }

    fn error_rate_pct(&self) -> f64 {
        self.error_rate() * 100.0
    }
}

impl fmt::Debug for HyperLogLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HyperLogLog(precision={}, registers={}, error_rate={:.2}%)",
            self.precision,
            self.num_registers(),
            self.error_rate_pct()
        )
    }
}

impl fmt::Display for HyperLogLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

fn count_leading_zeros(value: u64, bit_width: u8) -> u32 {
    if value == 0 {
        return bit_width as u32;
    }
    let total_width = 64u32;
    let leading_zeros = value.leading_zeros();
    leading_zeros.saturating_sub(total_width - bit_width as u32)
}

fn alpha_for_registers(registers: usize) -> f64 {
    match registers {
        16 => 0.673,
        32 => 0.697,
        64 => 0.709,
        m => 0.7213 / (1.0 + 1.079 / m as f64),
    }
}

fn fmix64(mut k: u64) -> u64 {
    k ^= k >> 33;
    k = k.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    k ^= k >> 33;
    k = k.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    k ^= k >> 33;
    k
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_count_is_zero() {
        let hll = HyperLogLog::new();
        assert_eq!(hll.count(), 0);
    }

    #[test]
    fn duplicates_do_not_change_the_estimate_much() {
        let mut hll = HyperLogLog::new();
        for _ in 0..1_000 {
            hll.add("same");
        }
        assert!(hll.count() < 10);
    }

    #[test]
    fn small_accuracy_is_reasonable() {
        let mut hll = HyperLogLog::new();
        for i in 0..1_000 {
            hll.add(format!("item-{i}"));
        }
        let estimate = hll.count();
        assert!((900..=1_100).contains(&estimate), "estimate={estimate}");
    }

    #[test]
    fn merge_combines_registers() {
        let mut left = HyperLogLog::with_precision(10);
        let mut right = HyperLogLog::with_precision(10);
        for i in 0..200 {
            left.add(i);
            right.add(i + 1000);
        }
        let merged = left.merge(&right);
        assert!(merged.count() >= left.count());
        assert!(merged.count() >= right.count());
    }

    #[test]
    fn merge_precision_mismatch_errors() {
        let left = HyperLogLog::with_precision(10);
        let right = HyperLogLog::with_precision(14);
        assert!(left.try_merge(&right).is_err());
    }

    #[test]
    fn helper_functions_work() {
        assert_eq!(HyperLogLog::memory_bytes(14), 12_288);
        assert_eq!(HyperLogLog::optimal_precision(0.01), 14);
        assert!((HyperLogLog::error_rate_for_precision(14) - 0.00812).abs() < 0.001);
        assert_eq!(count_leading_zeros(0, 8), 8);
        assert_eq!(count_leading_zeros(0b0010, 4), 2);
    }

    #[test]
    fn invalid_precision_is_rejected() {
        assert!(HyperLogLog::try_with_precision(3).is_err());
        assert!(HyperLogLog::try_with_precision(17).is_err());
    }
}
