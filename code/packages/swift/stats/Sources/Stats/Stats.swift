// ============================================================================
// Stats -- Descriptive Statistics, Frequency Analysis, and Cryptanalysis
// ============================================================================
//
// Overview
// --------
// This module provides three categories of pure functions:
//
// 1. **Descriptive statistics** (mean, median, mode, variance, standard
//    deviation, min, max, range) -- operate on arrays of Doubles.
// 2. **Frequency analysis** (frequencyCount, frequencyDistribution,
//    chiSquared, chiSquaredText) -- operate on text strings or arrays.
// 3. **Cryptanalysis helpers** (indexOfCoincidence, entropy,
//    englishFrequencies) -- tools for breaking classical ciphers.
//
// Design Principles
// -----------------
// - **Pure functions.** No side effects, no mutation of inputs.
// - **No external dependencies.** Pure math only (Foundation for sqrt/log).
// - **Population vs sample.** Variance and standard deviation default to
//   sample (Bessel-corrected, dividing by n-1). Pass population: true for
//   population statistics (dividing by n).
//
// Worked Example
// --------------
// Given the dataset [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]:
//
//   mean     = (2+4+4+4+5+5+7+9) / 8 = 40 / 8 = 5.0
//   median   = average of 4th and 5th values (sorted) = (4+5)/2 = 4.5
//   mode     = 4.0 (appears 3 times, more than any other)
//   variance = sample: sum of squared deviations / (n-1)
//            = [(2-5)^2 + (4-5)^2 + ... + (9-5)^2] / 7
//            = 32 / 7 = 4.571428...
//   std_dev  = sqrt(4.571428...) = 2.138...
//   min      = 2.0
//   max      = 9.0
//   range    = 9.0 - 2.0 = 7.0
// ============================================================================

import Foundation

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur when computing statistics on invalid input.
public enum StatsError: Error, Sendable {
    case emptyInput(String)
    case insufficientData(String)
    case mismatchedLengths(String)
}

// ============================================================================
// English Letter Frequencies
// ============================================================================
//
// Standard frequencies of each letter in English text, derived from large-
// corpus analysis. Used as the expected distribution for chi-squared tests
// in frequency analysis and cryptanalysis.
//
//   A = 0.08167  (about 8.2% of English text)
//   E = 0.12702  (the most common letter at ~12.7%)
//   Z = 0.00074  (the least common letter at ~0.07%)
// ============================================================================

/// Standard English letter frequencies (A-Z), derived from large-corpus
/// analysis. Keys are uppercase Character values.
public let englishFrequencies: [Character: Double] = [
    "A": 0.08167, "B": 0.01492, "C": 0.02782, "D": 0.04253,
    "E": 0.12702, "F": 0.02228, "G": 0.02015, "H": 0.06094,
    "I": 0.06966, "J": 0.00153, "K": 0.00772, "L": 0.04025,
    "M": 0.02406, "N": 0.06749, "O": 0.07507, "P": 0.01929,
    "Q": 0.00095, "R": 0.05987, "S": 0.06327, "T": 0.09056,
    "U": 0.02758, "V": 0.00978, "W": 0.02360, "X": 0.00150,
    "Y": 0.01974, "Z": 0.00074,
]

// ============================================================================
// Descriptive Statistics
// ============================================================================

/// Arithmetic mean: sum of all values divided by the count.
///
/// The mean is the most common measure of central tendency. It uses every
/// data point, which makes it sensitive to outliers. For example, the mean
/// of [1, 2, 3, 100] is 26.5, even though most values are small.
///
/// Formula: mean = (x_1 + x_2 + ... + x_n) / n
///
/// - Parameter values: An array of Doubles.
/// - Returns: The arithmetic mean.
/// - Throws: `StatsError.emptyInput` if the array is empty.
///
/// Example:
///   try mean([1, 2, 3, 4, 5])  // returns 3.0
public func mean(_ values: [Double]) throws -> Double {
    guard !values.isEmpty else {
        throw StatsError.emptyInput("mean requires at least one value")
    }
    return values.reduce(0.0, +) / Double(values.count)
}

/// Median: the middle value when sorted.
///
/// The median splits the dataset in half -- 50% of values are below it and
/// 50% are above. Unlike the mean, the median is robust to outliers.
///
/// For odd-length arrays, the median is the middle element.
/// For even-length arrays, it is the average of the two middle elements.
///
/// - Parameter values: An array of Doubles.
/// - Returns: The median value.
/// - Throws: `StatsError.emptyInput` if the array is empty.
///
/// Examples:
///   try median([1, 2, 3, 4, 5])  // returns 3.0 (middle of 5 elements)
///   try median([1, 2, 3, 4])     // returns 2.5 (average of 2 and 3)
public func median(_ values: [Double]) throws -> Double {
    guard !values.isEmpty else {
        throw StatsError.emptyInput("median requires at least one value")
    }

    let sorted = values.sorted()
    let n = sorted.count
    let mid = n / 2

    // Odd length: single middle element
    if n % 2 == 1 {
        return sorted[mid]
    }

    // Even length: average of two middle elements
    return (sorted[mid - 1] + sorted[mid]) / 2.0
}

/// Mode: the most frequently occurring value.
///
/// If multiple values share the highest frequency, the one that appears
/// first in the original array wins. This "first occurrence" tie-breaking
/// rule ensures deterministic results across all languages in the repo.
///
/// How it works:
/// 1. Count occurrences of each value.
/// 2. Find the maximum count.
/// 3. Return the first value in the original array that has that count.
///
/// - Parameter values: An array of Doubles.
/// - Returns: The mode value.
/// - Throws: `StatsError.emptyInput` if the array is empty.
///
/// Example:
///   try mode([2, 4, 4, 4, 5, 5, 7, 9])  // returns 4.0
public func mode(_ values: [Double]) throws -> Double {
    guard !values.isEmpty else {
        throw StatsError.emptyInput("mode requires at least one value")
    }

    // Step 1: count occurrences
    var counts: [Double: Int] = [:]
    for v in values {
        counts[v, default: 0] += 1
    }

    // Step 2: find the maximum frequency
    let maxCount = counts.values.max()!

    // Step 3: return the first value with that frequency
    for v in values {
        if counts[v] == maxCount {
            return v
        }
    }

    // Unreachable, but satisfies the compiler
    return values[0]
}

/// Variance: average of squared deviations from the mean.
///
/// Variance measures how spread out the data is. A variance of 0 means
/// all values are identical.
///
/// Two flavors:
///   - **Sample variance** (default, population=false): divides by n-1.
///     Used when your data is a sample from a larger population. The n-1
///     correction (Bessel's correction) makes the estimate unbiased.
///   - **Population variance** (population=true): divides by n.
///     Used when your data IS the entire population.
///
/// Formula:
///   variance = Sum((x_i - mean)^2) / d
///   where d = n (population) or n-1 (sample)
///
/// - Parameters:
///   - values: An array of Doubles.
///   - population: If true, compute population variance. Default false.
/// - Returns: The variance.
/// - Throws: `StatsError.emptyInput` or `StatsError.insufficientData`.
///
/// Examples:
///   try variance([2,4,4,4,5,5,7,9])                    // 4.571428... (sample)
///   try variance([2,4,4,4,5,5,7,9], population: true)  // 4.0
public func variance(_ values: [Double], population: Bool = false) throws -> Double {
    guard !values.isEmpty else {
        throw StatsError.emptyInput("variance requires at least one value")
    }
    let n = values.count
    if !population && n == 1 {
        throw StatsError.insufficientData("sample variance requires at least two values")
    }

    let m = try mean(values)

    // Sum of squared deviations
    let squaredDiffs = values.reduce(0.0) { $0 + ($1 - m) * ($1 - m) }

    let divisor = Double(population ? n : (n - 1))
    return squaredDiffs / divisor
}

/// Standard deviation: square root of variance.
///
/// The standard deviation has the same units as the original data (unlike
/// variance, which is in squared units). This makes it more interpretable.
///
/// For a normal distribution:
///   - ~68% of data falls within 1 standard deviation of the mean
///   - ~95% falls within 2 standard deviations
///   - ~99.7% falls within 3 standard deviations
///
/// - Parameters:
///   - values: An array of Doubles.
///   - population: If true, compute population std dev. Default false.
/// - Returns: The standard deviation.
/// - Throws: `StatsError.emptyInput` or `StatsError.insufficientData`.
public func standardDeviation(_ values: [Double], population: Bool = false) throws -> Double {
    return sqrt(try variance(values, population: population))
}

/// Minimum value in the dataset.
///
/// - Parameter values: An array of Doubles.
/// - Returns: The smallest value.
/// - Throws: `StatsError.emptyInput` if the array is empty.
public func statsMin(_ values: [Double]) throws -> Double {
    guard let result = values.min() else {
        throw StatsError.emptyInput("min requires at least one value")
    }
    return result
}

/// Maximum value in the dataset.
///
/// - Parameter values: An array of Doubles.
/// - Returns: The largest value.
/// - Throws: `StatsError.emptyInput` if the array is empty.
public func statsMax(_ values: [Double]) throws -> Double {
    guard let result = values.max() else {
        throw StatsError.emptyInput("max requires at least one value")
    }
    return result
}

/// Range: the difference between the maximum and minimum values.
///
/// The range is the simplest measure of spread. It only looks at the two
/// extreme values, so it is very sensitive to outliers.
///
/// Formula: range = max - min
///
/// - Parameter values: An array of Doubles.
/// - Returns: The range value.
/// - Throws: `StatsError.emptyInput` if the array is empty.
///
/// Example:
///   try statsRange([2, 4, 4, 4, 5, 5, 7, 9])  // returns 7.0
public func statsRange(_ values: [Double]) throws -> Double {
    return try statsMax(values) - statsMin(values)
}

// ============================================================================
// Frequency Analysis
// ============================================================================
//
// These functions analyze the frequency distribution of letters in text.
// They are the foundation of classical cipher analysis: by comparing
// observed letter frequencies against the known English distribution,
// we can detect whether a ciphertext was encrypted with a substitution
// cipher and potentially recover the key.
// ============================================================================

/// Count each letter in text (case-insensitive, A-Z only).
///
/// Non-alphabetic characters are ignored. The result is a dictionary mapping
/// uppercase Characters to their integer counts.
///
/// - Parameter text: A string to analyze.
/// - Returns: A dictionary mapping Characters to Int counts.
///
/// Example:
///   frequencyCount("Hello!")  // ["H": 1, "E": 1, "L": 2, "O": 1, ...]
public func frequencyCount(_ text: String) -> [Character: Int] {
    var counts: [Character: Int] = [:]

    // Initialize all 26 letters to 0
    for i in 0..<26 {
        let ch = Character(UnicodeScalar(65 + i)!)
        counts[ch] = 0
    }

    for ch in text.uppercased() {
        if ch >= "A" && ch <= "Z" {
            counts[ch, default: 0] += 1
        }
    }

    return counts
}

/// Frequency distribution: proportion of each letter in the text.
///
/// Like frequencyCount, but returns proportions (counts / total letters)
/// instead of raw counts.
///
/// - Parameter text: A string to analyze.
/// - Returns: A dictionary mapping Characters to Double proportions.
public func frequencyDistribution(_ text: String) -> [Character: Double] {
    let counts = frequencyCount(text)

    let total = counts.values.reduce(0, +)

    var dist: [Character: Double] = [:]
    for (letter, count) in counts {
        dist[letter] = total > 0 ? Double(count) / Double(total) : 0.0
    }

    return dist
}

/// Chi-squared goodness-of-fit test for parallel arrays.
///
/// The chi-squared statistic measures how well observed data fits an
/// expected distribution:
///
///   chi2 = Sum((O_i - E_i)^2 / E_i)
///
/// - Parameters:
///   - observed: Array of observed counts.
///   - expected: Array of expected counts (same length as observed).
/// - Returns: The chi-squared statistic.
/// - Throws: `StatsError.mismatchedLengths` or `StatsError.emptyInput`.
///
/// Example:
///   try chiSquared(observed: [10, 20, 30], expected: [20, 20, 20])  // 10.0
public func chiSquared(observed: [Double], expected: [Double]) throws -> Double {
    guard observed.count == expected.count else {
        throw StatsError.mismatchedLengths("observed and expected must have same length")
    }
    guard !observed.isEmpty else {
        throw StatsError.emptyInput("arrays must not be empty")
    }

    var chi2 = 0.0
    for i in 0..<observed.count {
        if expected[i] > 1e-10 {
            let diff = observed[i] - expected[i]
            chi2 += (diff * diff) / expected[i]
        }
    }

    return chi2
}

/// Chi-squared test of text against an expected frequency table.
///
/// Combines frequencyCount with chiSquared. Counts the letters in the text,
/// then compares those counts against expected frequencies scaled to the
/// text length.
///
/// - Parameters:
///   - text: A string to analyze.
///   - expectedFreq: A dictionary mapping Characters to frequency proportions.
/// - Returns: The chi-squared statistic.
public func chiSquaredText(_ text: String, expectedFreq: [Character: Double]) -> Double {
    let counts = frequencyCount(text)

    let total = counts.values.reduce(0, +)

    if total == 0 {
        return 0.0
    }

    var chi2 = 0.0
    for i in 0..<26 {
        let letter = Character(UnicodeScalar(65 + i)!)
        let observed = Double(counts[letter] ?? 0)
        let expected = Double(total) * (expectedFreq[letter] ?? 0.0)
        if expected > 1e-10 {
            let diff = observed - expected
            chi2 += (diff * diff) / expected
        }
    }

    return chi2
}

// ============================================================================
// Cryptanalysis Helpers
// ============================================================================
//
// Index of Coincidence (IC):
//   Measures the probability that two randomly chosen letters from the
//   text are the same. English text has IC ~0.0667, while random text
//   has IC ~0.0385 (1/26).
//
// Shannon Entropy:
//   Measures the information content (in bits) of the text's letter
//   distribution. Uniform distribution gives maximum entropy (log2(26)
//   ~ 4.700 bits).
// ============================================================================

/// Index of Coincidence: probability that two random letters match.
///
/// Formula:
///   IC = Sum(n_i * (n_i - 1)) / (N * (N - 1))
///
/// Reference values:
///   - English text:  IC ~ 0.0667
///   - Random text:   IC ~ 0.0385 (1/26)
///   - "AABB":        IC = (2*1 + 2*1) / (4*3) = 4/12 = 0.333...
///
/// - Parameter text: A string to analyze.
/// - Returns: The index of coincidence.
public func indexOfCoincidence(_ text: String) -> Double {
    let counts = frequencyCount(text)

    // N = total alphabetic characters
    let n = counts.values.reduce(0, +)

    // Need at least 2 letters
    guard n >= 2 else {
        return 0.0
    }

    // Sum(n_i * (n_i - 1))
    var numerator = 0
    for c in counts.values {
        numerator += c * (c - 1)
    }

    return Double(numerator) / Double(n * (n - 1))
}

/// Shannon entropy of the letter distribution in text.
///
/// Formula:
///   H = -Sum(p_i * log2(p_i))
///
/// Reference values:
///   - 26 equal letters: H = log2(26) ~ 4.700 bits
///   - English text:     H ~ 4.1 bits
///   - "AAAA":           H = 0.0 bits (no surprise)
///
/// - Parameter text: A string to analyze.
/// - Returns: The Shannon entropy in bits.
public func entropy(_ text: String) -> Double {
    let dist = frequencyDistribution(text)

    var h = 0.0
    for p in dist.values {
        if p > 0 {
            h -= p * (log(p) / log(2.0))
        }
    }

    return h
}
