// # Descriptive Statistics
//
// This module provides the fundamental statistical functions that summarize
// a dataset with a single number: central tendency (mean, median, mode) and
// spread (variance, standard deviation, min, max, range).
//
// All functions take a slice of f64 values. They are pure functions that
// do not mutate their inputs.

/// # Mean (Arithmetic Average)
///
/// The mean answers: "If we spread the total equally among all values,
/// what would each value be?"
///
/// ## Formula
///
/// ```text
/// mean = sum(values) / n
/// ```
///
/// ## Example
///
/// ```text
/// mean(&[1.0, 2.0, 3.0, 4.0, 5.0]) => 3.0
/// ```
///
pub fn mean(values: &[f64]) -> f64 {
    assert!(!values.is_empty(), "Cannot compute mean of an empty slice");
    let sum: f64 = values.iter().sum();
    sum / values.len() as f64
}

/// # Median
///
/// The median is the "middle" value when data is sorted. Unlike the mean,
/// it is robust against outliers.
///
/// ## Algorithm
///
/// 1. Sort the values in ascending order.
/// 2. If the count is odd, return the middle element.
/// 3. If the count is even, return the average of the two middle elements.
///
/// ## Example
///
/// ```text
/// median(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]) => 4.5
/// ```
///
pub fn median(values: &[f64]) -> f64 {
    assert!(!values.is_empty(), "Cannot compute median of an empty slice");

    // Sort a copy so we don't mutate the input.
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mid = sorted.len() / 2;
    if sorted.len() % 2 != 0 {
        sorted[mid]
    } else {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    }
}

/// # Mode
///
/// The mode is the most frequently occurring value. When multiple values
/// share the highest frequency, the first occurrence in the original
/// array wins (deterministic tie-breaking).
///
/// ## Example
///
/// ```text
/// mode(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]) => 4.0
/// ```
///
pub fn mode(values: &[f64]) -> f64 {
    assert!(!values.is_empty(), "Cannot compute mode of an empty slice");

    // We use a Vec of (value, count) pairs to preserve insertion order,
    // which is needed for "first occurrence wins" tie-breaking.
    let mut counts: Vec<(f64, usize)> = Vec::new();

    for &val in values {
        if let Some(entry) = counts.iter_mut().find(|(v, _)| v.to_bits() == val.to_bits()) {
            entry.1 += 1;
        } else {
            counts.push((val, 1));
        }
    }

    let mut best_value = values[0];
    let mut best_count = 0;
    for &(val, count) in &counts {
        if count > best_count {
            best_count = count;
            best_value = val;
        }
    }

    best_value
}

/// # Variance
///
/// Variance measures how spread out the data is from the mean.
///
/// ## Formula
///
/// ```text
/// variance = sum((x_i - mean)^2) / d
/// ```
///
/// Where `d` is n (population) or n-1 (sample, Bessel's correction).
///
/// ## Why n-1? (Bessel's Correction)
///
/// When computing variance from a sample, the sample mean is "pulled toward"
/// the data points, systematically underestimating the true spread. Dividing
/// by n-1 compensates for this bias.
///
/// ## Example
///
/// ```text
/// values = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]
/// variance(values, false) => 4.571428571428571  (sample)
/// variance(values, true)  => 4.0                (population)
/// ```
///
pub fn variance(values: &[f64], population: bool) -> f64 {
    assert!(!values.is_empty(), "Cannot compute variance of an empty slice");
    if !population {
        assert!(
            values.len() >= 2,
            "Sample variance requires at least 2 values"
        );
    }

    let n = values.len() as f64;
    let avg = mean(values);

    let sum_sq_dev: f64 = values.iter().map(|&x| (x - avg).powi(2)).sum();

    let divisor = if population { n } else { n - 1.0 };
    sum_sq_dev / divisor
}

/// # Standard Deviation
///
/// The standard deviation is the square root of the variance. While variance
/// is in "squared units," standard deviation brings us back to the original
/// units, making it more interpretable.
///
/// The 68-95-99.7 rule: for normally distributed data, ~68% of values fall
/// within 1 standard deviation, ~95% within 2, and ~99.7% within 3.
///
pub fn standard_deviation(values: &[f64], population: bool) -> f64 {
    variance(values, population).sqrt()
}

/// # Minimum
///
/// Returns the smallest value in a slice.
///
pub fn min(values: &[f64]) -> f64 {
    assert!(!values.is_empty(), "Cannot compute min of an empty slice");
    values
        .iter()
        .copied()
        .fold(f64::INFINITY, |a, b| if b < a { b } else { a })
}

/// # Maximum
///
/// Returns the largest value in a slice.
///
pub fn max(values: &[f64]) -> f64 {
    assert!(!values.is_empty(), "Cannot compute max of an empty slice");
    values
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, |a, b| if b > a { b } else { a })
}

/// # Range
///
/// The range is the simplest measure of spread: max - min.
///
/// ## Example
///
/// ```text
/// range(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]) => 7.0
/// ```
///
pub fn range(values: &[f64]) -> f64 {
    max(values) - min(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mean_parity() {
        assert!((mean(&[1.0, 2.0, 3.0, 4.0, 5.0]) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_mean_worked_example() {
        assert!(
            (mean(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]) - 5.0).abs() < 1e-10
        );
    }

    #[test]
    fn test_mean_single() {
        assert_eq!(mean(&[42.0]), 42.0);
    }

    #[test]
    #[should_panic(expected = "empty")]
    fn test_mean_empty() {
        mean(&[]);
    }

    #[test]
    fn test_median_odd() {
        assert_eq!(median(&[1.0, 3.0, 5.0]), 3.0);
    }

    #[test]
    fn test_median_even() {
        assert!(
            (median(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]) - 4.5).abs() < 1e-10
        );
    }

    #[test]
    fn test_median_unsorted() {
        assert_eq!(median(&[5.0, 1.0, 3.0]), 3.0);
    }

    #[test]
    #[should_panic(expected = "empty")]
    fn test_median_empty() {
        median(&[]);
    }

    #[test]
    fn test_mode_most_frequent() {
        assert_eq!(mode(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]), 4.0);
    }

    #[test]
    fn test_mode_tie_first_wins() {
        assert_eq!(mode(&[1.0, 2.0, 1.0, 2.0, 3.0]), 1.0);
    }

    #[test]
    fn test_mode_single() {
        assert_eq!(mode(&[99.0]), 99.0);
    }

    #[test]
    #[should_panic(expected = "empty")]
    fn test_mode_empty() {
        mode(&[]);
    }

    #[test]
    fn test_variance_sample_parity() {
        let vals = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        assert!((variance(&vals, false) - 4.571428571428571).abs() < 1e-10);
    }

    #[test]
    fn test_variance_population_parity() {
        let vals = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        assert!((variance(&vals, true) - 4.0).abs() < 1e-10);
    }

    #[test]
    #[should_panic(expected = "empty")]
    fn test_variance_empty() {
        variance(&[], false);
    }

    #[test]
    #[should_panic(expected = "at least 2")]
    fn test_variance_sample_single() {
        variance(&[5.0], false);
    }

    #[test]
    fn test_variance_population_single() {
        assert_eq!(variance(&[5.0], true), 0.0);
    }

    #[test]
    fn test_std_dev_sample() {
        let vals = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        assert!((standard_deviation(&vals, false) - 4.571428571428571_f64.sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_std_dev_population() {
        let vals = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        assert!((standard_deviation(&vals, true) - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_min() {
        assert_eq!(min(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]), 2.0);
    }

    #[test]
    fn test_min_negative() {
        assert_eq!(min(&[-3.0, -1.0, 0.0, 5.0]), -3.0);
    }

    #[test]
    #[should_panic(expected = "empty")]
    fn test_min_empty() {
        min(&[]);
    }

    #[test]
    fn test_max() {
        assert_eq!(max(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]), 9.0);
    }

    #[test]
    fn test_max_negative() {
        assert_eq!(max(&[-3.0, -1.0, 0.0, 5.0]), 5.0);
    }

    #[test]
    #[should_panic(expected = "empty")]
    fn test_max_empty() {
        max(&[]);
    }

    #[test]
    fn test_range() {
        assert!((range(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]) - 7.0).abs() < 1e-10);
    }

    #[test]
    fn test_range_identical() {
        assert_eq!(range(&[5.0, 5.0, 5.0]), 0.0);
    }
}
