// Package stats provides descriptive statistics, frequency analysis, and
// cryptanalysis helpers.
//
// # Overview
//
// This package serves two purposes:
//
//  1. General-purpose statistics: Mean, median, mode, variance, standard
//     deviation, min, max, range — usable by any package in the repo.
//  2. Cryptanalysis toolkit: Chi-squared tests, index of coincidence, Shannon
//     entropy, and English frequency tables — for breaking classical ciphers.
//
// Every function is a pure function with no side effects and no mutation of
// inputs.
//
// # Population vs Sample
//
// Variance and standard deviation accept a boolean `population` parameter.
// When false (the default), they use Bessel's correction (dividing by n-1)
// for sample statistics. When true, they divide by n for population statistics.
//
// # Frequency Analysis
//
// The frequency functions operate on text strings, counting only ASCII letters
// A-Z (case-insensitive). Non-alphabetic characters are silently ignored.
package stats

import (
	"math"
	"sort"
	"strings"
	"unicode"
)

// ── Descriptive Statistics ─────────────────────────────────────────────

// Mean computes the arithmetic mean (average) of a slice of floats.
//
// Formula: mean = (x_1 + x_2 + ... + x_n) / n
//
// The mean is the most common measure of central tendency. It uses every
// data point, making it sensitive to outliers.
//
// Panics if values is empty.
func Mean(values []float64) float64 {
	if len(values) == 0 {
		panic("stats.Mean: values must not be empty")
	}
	sum := 0.0
	for _, v := range values {
		sum += v
	}
	return sum / float64(len(values))
}

// Median returns the middle value when the data is sorted.
//
// For odd-length slices, it returns the single middle element.
// For even-length slices, it returns the average of the two middle elements.
//
// The median is robust to outliers — unlike the mean, extreme values do not
// pull it away from the center.
//
// Panics if values is empty.
func Median(values []float64) float64 {
	if len(values) == 0 {
		panic("stats.Median: values must not be empty")
	}
	// Sort a copy so we don't mutate the caller's slice.
	sorted := make([]float64, len(values))
	copy(sorted, values)
	sort.Float64s(sorted)

	n := len(sorted)
	mid := n / 2

	// Odd length: single middle element.
	if n%2 == 1 {
		return sorted[mid]
	}
	// Even length: average of the two middle elements.
	return (sorted[mid-1] + sorted[mid]) / 2.0
}

// Mode returns the most frequently occurring value.
//
// If multiple values share the highest frequency, the one that appears first
// in the original slice wins. This "first occurrence" tie-breaking rule
// ensures deterministic results across all language implementations.
//
// Panics if values is empty.
func Mode(values []float64) float64 {
	if len(values) == 0 {
		panic("stats.Mode: values must not be empty")
	}

	// Step 1: count occurrences.
	counts := make(map[float64]int)
	for _, v := range values {
		counts[v]++
	}

	// Step 2: find the maximum frequency.
	maxCount := 0
	for _, c := range counts {
		if c > maxCount {
			maxCount = c
		}
	}

	// Step 3: return the first value with that frequency.
	for _, v := range values {
		if counts[v] == maxCount {
			return v
		}
	}

	// Unreachable, but satisfies the compiler.
	return values[0]
}

// Variance measures how spread out the data is.
//
// Formula: variance = Sum((x_i - mean)^2) / d
// where d = n (population) or n-1 (sample)
//
// Sample variance (population=false) uses Bessel's correction, dividing by
// n-1 to produce an unbiased estimator. Population variance (population=true)
// divides by n.
//
// Panics if values is empty, or if sample variance is requested with n < 2.
func Variance(values []float64, population bool) float64 {
	if len(values) == 0 {
		panic("stats.Variance: values must not be empty")
	}
	n := len(values)
	if n == 1 && !population {
		panic("stats.Variance: sample variance requires at least two values")
	}

	m := Mean(values)

	// Sum of squared deviations from the mean.
	sumSq := 0.0
	for _, v := range values {
		diff := v - m
		sumSq += diff * diff
	}

	divisor := float64(n)
	if !population {
		divisor = float64(n - 1)
	}
	return sumSq / divisor
}

// StandardDeviation is the square root of variance.
//
// It has the same units as the original data, making it more interpretable
// than variance. For a normal distribution, about 68% of data falls within
// one standard deviation of the mean.
func StandardDeviation(values []float64, population bool) float64 {
	return math.Sqrt(Variance(values, population))
}

// Min returns the minimum value in the slice.
//
// Panics if values is empty.
func Min(values []float64) float64 {
	if len(values) == 0 {
		panic("stats.Min: values must not be empty")
	}
	result := values[0]
	for _, v := range values[1:] {
		if v < result {
			result = v
		}
	}
	return result
}

// Max returns the maximum value in the slice.
//
// Panics if values is empty.
func Max(values []float64) float64 {
	if len(values) == 0 {
		panic("stats.Max: values must not be empty")
	}
	result := values[0]
	for _, v := range values[1:] {
		if v > result {
			result = v
		}
	}
	return result
}

// Range returns max - min, the simplest measure of spread.
//
// Panics if values is empty.
func Range(values []float64) float64 {
	return Max(values) - Min(values)
}

// ── Frequency Analysis ─────────────────────────────────────────────────

// FrequencyCount counts occurrences of each letter (A-Z) in the text,
// case-insensitive. Non-alphabetic characters are silently ignored.
//
// Returns a map from uppercase letter strings to counts. Only letters that
// actually appear in the text are included.
func FrequencyCount(text string) map[string]int {
	counts := make(map[string]int)
	for _, ch := range strings.ToUpper(text) {
		if unicode.IsLetter(ch) && ch >= 'A' && ch <= 'Z' {
			counts[string(ch)]++
		}
	}
	return counts
}

// FrequencyDistribution returns the proportion of each letter in the text.
//
// Each proportion is count / total_letters. The proportions sum to
// approximately 1.0.
func FrequencyDistribution(text string) map[string]float64 {
	counts := FrequencyCount(text)
	total := 0
	for _, c := range counts {
		total += c
	}
	if total == 0 {
		return make(map[string]float64)
	}
	dist := make(map[string]float64, len(counts))
	for letter, count := range counts {
		dist[letter] = float64(count) / float64(total)
	}
	return dist
}

// ChiSquared computes the chi-squared statistic for two parallel slices.
//
// Formula: chi_squared = Sum((O_i - E_i)^2 / E_i)
//
// A value of 0 means a perfect match. Larger values mean worse match.
// Panics if the slices have different lengths.
func ChiSquared(observed, expected []float64) float64 {
	if len(observed) != len(expected) {
		panic("stats.ChiSquared: observed and expected must have the same length")
	}
	result := 0.0
	for i := range observed {
		diff := observed[i] - expected[i]
		result += (diff * diff) / expected[i]
	}
	return result
}

// ChiSquaredText computes chi-squared comparing text letter frequencies to
// an expected frequency distribution.
//
// Steps:
//  1. Count letters in text (A-Z, case-insensitive).
//  2. For each letter in expectedFreq, compute expected_count = freq * total.
//  3. Compute chi-squared over all letters in the expected map.
func ChiSquaredText(text string, expectedFreq map[string]float64) float64 {
	counts := FrequencyCount(text)
	total := 0
	for _, c := range counts {
		total += c
	}
	if total == 0 {
		return 0.0
	}

	result := 0.0
	for letter, freq := range expectedFreq {
		observed := float64(counts[strings.ToUpper(letter)])
		expected := freq * float64(total)
		if expected > 0 {
			diff := observed - expected
			result += (diff * diff) / expected
		}
	}
	return result
}

// ── Cryptanalysis Helpers ──────────────────────────────────────────────

// IndexOfCoincidence measures the probability that two randomly chosen letters
// from a text are the same.
//
// Formula: IC = Sum(n_i * (n_i - 1)) / (N * (N - 1))
//
// Key values:
//   - English text:  IC ~ 0.0667
//   - Random text:   IC ~ 0.0385 (1/26)
//
// Returns 0.0 for texts with fewer than 2 letters.
func IndexOfCoincidence(text string) float64 {
	counts := FrequencyCount(text)
	n := 0
	for _, c := range counts {
		n += c
	}
	if n < 2 {
		return 0.0
	}

	// Numerator: Sum of n_i * (n_i - 1) for each letter.
	numerator := 0
	for _, count := range counts {
		numerator += count * (count - 1)
	}

	// Denominator: N * (N - 1), total ways to pick 2 letters.
	denominator := n * (n - 1)

	return float64(numerator) / float64(denominator)
}

// Entropy computes the Shannon entropy of the letter distribution in bits.
//
// Formula: H = -Sum(p_i * log2(p_i))
//
// Higher entropy means a more uniform distribution (harder to break).
// Maximum for 26 letters is log2(26) ~ 4.700 bits.
// Returns 0.0 for empty text.
func Entropy(text string) float64 {
	counts := FrequencyCount(text)
	total := 0
	for _, c := range counts {
		total += c
	}
	if total == 0 {
		return 0.0
	}

	h := 0.0
	for _, count := range counts {
		if count > 0 {
			p := float64(count) / float64(total)
			h -= p * math.Log2(p)
		}
	}
	return h
}

// EnglishFrequencies contains the standard English letter frequency table.
// Source: Lewand (2000). Keys are uppercase single-character strings.
var EnglishFrequencies = map[string]float64{
	"A": 0.08167, "B": 0.01492, "C": 0.02782, "D": 0.04253,
	"E": 0.12702, "F": 0.02228, "G": 0.02015, "H": 0.06094,
	"I": 0.06966, "J": 0.00153, "K": 0.00772, "L": 0.04025,
	"M": 0.02406, "N": 0.06749, "O": 0.07507, "P": 0.01929,
	"Q": 0.00095, "R": 0.05987, "S": 0.06327, "T": 0.09056,
	"U": 0.02758, "V": 0.00978, "W": 0.02360, "X": 0.00150,
	"Y": 0.01974, "Z": 0.00074,
}
