//! Reusable sample median and exact order-statistic confidence intervals.
//!
//! The median and its interval use the original finite observations.  The
//! interval cutoff is compared with exact integers, so a rounded probability
//! can never move the selected rank across a confidence boundary.

use std::cmp::Ordering;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    EmptySample,
    NonFiniteSample { index: usize },
    InvalidConfidence { confidence_percent: u8 },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySample => write!(f, "sample must not be empty"),
            Self::NonFiniteSample { index } => {
                write!(f, "sample value at index {index} must be finite")
            }
            Self::InvalidConfidence { confidence_percent } => write!(
                f,
                "confidence percentage must be from 1 through 99, got {confidence_percent}"
            ),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    pub low: f64,
    pub high: f64,
}

fn sorted_finite(values: &[f64]) -> Result<Vec<f64>, Error> {
    if values.is_empty() {
        return Err(Error::EmptySample);
    }
    for (index, value) in values.iter().enumerate() {
        if !value.is_finite() {
            return Err(Error::NonFiniteSample { index });
        }
    }
    let mut ordered = values.to_vec();
    ordered.sort_by(f64::total_cmp);
    Ok(ordered)
}

/// Return the sample median without modifying the input.
///
/// Even-sized samples use their central observations' arithmetic midpoint.
/// The fallback halves each operand before adding when the direct sum would
/// overflow, keeping the result finite for every finite pair.
pub fn median(values: &[f64]) -> Result<f64, Error> {
    let ordered = sorted_finite(values)?;
    let middle = ordered.len() / 2;
    if ordered.len() % 2 == 1 {
        return Ok(ordered[middle]);
    }
    let (low, high) = (ordered[middle - 1], ordered[middle]);
    let sum = low + high;
    Ok(if sum.is_finite() {
        sum / 2.0
    } else {
        low / 2.0 + high / 2.0
    })
}

/// Return the exact two-sided order-statistic interval for a population median.
///
/// The selected one-based rank is the largest `k` satisfying
/// `200 * P-count(0..k-1) <= (100-confidence) * 2^n`, where the P-count is
/// `sum(C(n,j), j=0..k-1)`. `None` means no rank meets the requested cutoff.
/// Ties remain repeated observations, and the caller's slice is unchanged.
pub fn median_interval(values: &[f64], confidence_percent: u8) -> Result<Option<Interval>, Error> {
    if !(1..=99).contains(&confidence_percent) {
        return Err(Error::InvalidConfidence { confidence_percent });
    }
    let ordered = sorted_finite(values)?;
    let rank = exact_median_rank(ordered.len(), confidence_percent);
    if rank == 0 {
        return Ok(None);
    }
    Ok(Some(Interval {
        low: ordered[rank - 1],
        high: ordered[ordered.len() - rank],
    }))
}

/// A small unsigned integer used only by the exact binomial cutoff.
///
/// A 10,000-pair experiment needs about 10,000 bits. Keeping this tiny helper
/// local avoids pulling a general-purpose bignum or statistics dependency into
/// a two-operation package.
#[derive(Clone, Debug, PartialEq, Eq)]
struct BigNat(Vec<u64>);

impl BigNat {
    fn from_u64(value: u64) -> Self {
        Self(vec![value])
    }

    fn one_shifted(bits: usize) -> Self {
        let mut limbs = vec![0; bits / 64 + 1];
        limbs[bits / 64] = 1u64 << (bits % 64);
        Self(limbs)
    }

    fn normalize(&mut self) {
        while self.0.len() > 1 && self.0.last() == Some(&0) {
            self.0.pop();
        }
    }

    fn add_assign(&mut self, other: &Self) {
        let size = self.0.len().max(other.0.len());
        self.0.resize(size, 0);
        let mut carry = 0u128;
        for (index, left_limb) in self.0.iter_mut().enumerate() {
            let left = u128::from(*left_limb);
            let right = u128::from(*other.0.get(index).unwrap_or(&0));
            let total = left + right + carry;
            *left_limb = total as u64;
            carry = total >> 64;
        }
        if carry != 0 {
            self.0.push(carry as u64);
        }
    }

    fn multiply_small(&mut self, factor: u64) {
        let mut carry = 0u128;
        for limb in &mut self.0 {
            let product = u128::from(*limb) * u128::from(factor) + carry;
            *limb = product as u64;
            carry = product >> 64;
        }
        if carry != 0 {
            self.0.push(carry as u64);
        }
        self.normalize();
    }

    fn divide_small_exact(&mut self, divisor: u64) {
        debug_assert_ne!(divisor, 0);
        let mut remainder = 0u128;
        for limb in self.0.iter_mut().rev() {
            let dividend = (remainder << 64) | u128::from(*limb);
            *limb = (dividend / u128::from(divisor)) as u64;
            remainder = dividend % u128::from(divisor);
        }
        debug_assert_eq!(remainder, 0, "binomial recurrence divides exactly");
        self.normalize();
    }

    fn compare(&self, other: &Self) -> Ordering {
        self.0
            .len()
            .cmp(&other.0.len())
            .then_with(|| self.0.iter().rev().cmp(other.0.iter().rev()))
    }
}

fn exact_median_rank(n: usize, confidence_percent: u8) -> usize {
    let confidence_complement = u64::from(100 - confidence_percent);
    let mut cutoff = BigNat::one_shifted(n);
    cutoff.multiply_small(confidence_complement);

    let mut coefficient = BigNat::from_u64(1); // C(n, 0)
    let mut tail = BigNat::from_u64(0);
    let mut rank = 0;
    for j in 0..=n / 2 {
        tail.add_assign(&coefficient);
        let mut scaled_tail = tail.clone();
        scaled_tail.multiply_small(200);
        if scaled_tail.compare(&cutoff) == Ordering::Greater {
            break;
        }
        rank = j + 1;

        if j < n / 2 {
            coefficient.multiply_small((n - j) as u64);
            coefficient.divide_small_exact((j + 1) as u64);
        }
    }
    rank
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounded_oracle_rank(n: usize, confidence: u8) -> usize {
        assert!(n <= 20);
        let assignments = 1u64 << n;
        let mut rank = 0;
        let mut tail = 0u64;
        for j in 0..=n / 2 {
            // Enumerate sign assignments directly; this intentionally does not
            // use the production binomial-coefficient recurrence.
            tail += (0..assignments)
                .filter(|mask| mask.count_ones() as usize == j)
                .count() as u64;
            if 200 * tail <= u64::from(100 - confidence) * assignments {
                rank = j + 1;
            } else {
                break;
            }
        }
        rank
    }

    #[test]
    fn median_handles_odd_even_extreme_and_non_mutating_samples() {
        let values = [9.0, 1.0, 5.0, 3.0];
        assert_eq!(median(&values), Ok(4.0));
        assert_eq!(values, [9.0, 1.0, 5.0, 3.0]);
        assert_eq!(median(&[f64::MAX, f64::MAX]), Ok(f64::MAX));
        assert_eq!(median(&[-f64::MAX, f64::MAX]), Ok(0.0));
    }

    #[test]
    fn median_interval_matches_exact_bounded_oracle() {
        for n in 1..=14 {
            let values: Vec<f64> = (0..n).map(|index| ((index * 7) % 5) as f64).collect();
            for confidence in [1, 50, 80, 90, 95, 99] {
                let rank = bounded_oracle_rank(n, confidence);
                let interval = median_interval(&values, confidence).unwrap();
                assert_eq!(interval.is_some(), rank != 0, "n={n}, c={confidence}");
                if let Some(got) = interval {
                    let mut sorted = values.clone();
                    sorted.sort_by(f64::total_cmp);
                    assert_eq!(got.low, sorted[rank - 1]);
                    assert_eq!(got.high, sorted[n - rank]);
                }
            }
        }
    }

    #[test]
    fn interval_endpoints_are_observations_and_ties_are_preserved() {
        let values = [3.0, 1.0, 2.0, 2.0, 5.0, 4.0];
        let got = median_interval(&values, 95).unwrap().unwrap();
        assert_eq!(
            got,
            Interval {
                low: 1.0,
                high: 5.0
            }
        );
        assert!(values.contains(&got.low) && values.contains(&got.high));
        assert_eq!(values, [3.0, 1.0, 2.0, 2.0, 5.0, 4.0]);
    }

    #[test]
    fn rejects_empty_nonfinite_and_invalid_confidence() {
        assert_eq!(median(&[]), Err(Error::EmptySample));
        assert_eq!(median_interval(&[], 95), Err(Error::EmptySample));
        assert_eq!(
            median(&[1.0, f64::NAN]),
            Err(Error::NonFiniteSample { index: 1 })
        );
        assert_eq!(
            median_interval(&[f64::INFINITY], 95),
            Err(Error::NonFiniteSample { index: 0 })
        );
        assert!(matches!(
            median_interval(&[1.0], 0),
            Err(Error::InvalidConfidence { .. })
        ));
        assert!(matches!(
            median_interval(&[1.0], 100),
            Err(Error::InvalidConfidence { .. })
        ));
    }

    #[test]
    fn deterministic_generated_samples_keep_order_statistic_invariants() {
        let mut seed = 0x9e37_79b9_7f4a_7c15u64;
        for n in 1..=64 {
            let mut values = Vec::with_capacity(n);
            for _ in 0..n {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                values.push((seed % 101) as f64 - 50.0);
            }
            let mut sorted = values.clone();
            sorted.sort_by(f64::total_cmp);
            let m = median(&values).unwrap();
            assert!(sorted[0] <= m && m <= sorted[n - 1]);
            for confidence in [50, 80, 95, 99] {
                if let Some(interval) = median_interval(&values, confidence).unwrap() {
                    assert!(interval.low <= interval.high);
                    assert!(values.contains(&interval.low));
                    assert!(values.contains(&interval.high));
                }
            }
        }
    }

    #[test]
    fn exact_cutoff_handles_the_full_benchmark_pair_limit() {
        let values: Vec<f64> = (0..10_000).map(|index| index as f64).collect();
        let interval = median_interval(&values, 95).unwrap().unwrap();
        assert!(interval.low < 5_000.0);
        assert!(interval.high > 4_999.0);
        assert!(values.contains(&interval.low) && values.contains(&interval.high));
    }

    #[test]
    fn multi_limb_cutoffs_match_frozen_python_reference_vectors() {
        // These values were generated by the merged Python reference package.
        // They are cross-language differential vectors, not an independent
        // mathematical oracle; the bounded sign-assignment test above is that.
        let cases = [
            (64usize, 95u8, 23.0, 40.0),
            (65, 99, 21.0, 43.0),
            (127, 95, 51.0, 75.0),
            (128, 95, 52.0, 75.0),
            (256, 99, 106.0, 149.0),
            (1_000, 95, 468.0, 531.0),
            (10_000, 50, 4_965.0, 5_034.0),
            (10_000, 95, 4_901.0, 5_098.0),
            (10_000, 99, 4_870.0, 5_129.0),
        ];
        for (n, confidence, low, high) in cases {
            let values: Vec<f64> = (0..n).map(|index| index as f64).collect();
            assert_eq!(
                median_interval(&values, confidence).unwrap(),
                Some(Interval { low, high }),
                "n={n}, confidence={confidence}"
            );
        }
    }
}
