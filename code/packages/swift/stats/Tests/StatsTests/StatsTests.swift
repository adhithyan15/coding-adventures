// ============================================================================
// Tests for Statistics Package
// ============================================================================
// These tests cover:
//   - Descriptive statistics (mean, median, mode, variance, std_dev, min, max, range)
//   - Frequency analysis (frequencyCount, frequencyDistribution, chiSquared,
//     chiSquaredText)
//   - Cryptanalysis helpers (indexOfCoincidence, entropy, englishFrequencies)
//   - Cross-language parity test vectors
// ============================================================================

import Foundation
import Testing
@testable import Stats

// ============================================================================
// Helper: compare floats within a tolerance (epsilon)
// ============================================================================

func approx(_ a: Double, _ b: Double, eps: Double = 1e-6) -> Bool {
    return abs(a - b) < eps
}

// ============================================================================
// Descriptive Statistics Tests
// ============================================================================

@Suite("Descriptive Statistics")
struct DescriptiveTests {

    // -- Mean ---------------------------------------------------------------

    @Test("mean of simple list")
    func testMeanSimple() throws {
        // Parity test vector: mean([1,2,3,4,5]) = 3.0
        #expect(approx(try mean([1, 2, 3, 4, 5]), 3.0))
    }

    @Test("mean of worked example")
    func testMeanWorkedExample() throws {
        #expect(approx(try mean([2, 4, 4, 4, 5, 5, 7, 9]), 5.0))
    }

    @Test("mean of single value")
    func testMeanSingle() throws {
        #expect(approx(try mean([42]), 42.0))
    }

    @Test("mean of negative values")
    func testMeanNegative() throws {
        #expect(approx(try mean([-1, -2, -3]), -2.0))
    }

    @Test("mean errors on empty input")
    func testMeanEmpty() {
        #expect(throws: StatsError.self) {
            try mean([])
        }
    }

    // -- Median -------------------------------------------------------------

    @Test("median of odd-length list")
    func testMedianOdd() throws {
        #expect(approx(try median([1, 2, 3, 4, 5]), 3.0))
    }

    @Test("median of even-length list")
    func testMedianEven() throws {
        #expect(approx(try median([2, 4, 4, 4, 5, 5, 7, 9]), 4.5))
    }

    @Test("median of single value")
    func testMedianSingle() throws {
        #expect(approx(try median([7]), 7.0))
    }

    @Test("median sorts unsorted input")
    func testMedianUnsorted() throws {
        #expect(approx(try median([5, 1, 3]), 3.0))
    }

    @Test("median errors on empty input")
    func testMedianEmpty() {
        #expect(throws: StatsError.self) {
            try median([])
        }
    }

    // -- Mode ---------------------------------------------------------------

    @Test("mode finds most frequent value")
    func testModeFrequent() throws {
        #expect(approx(try mode([2, 4, 4, 4, 5, 5, 7, 9]), 4.0))
    }

    @Test("mode returns first occurrence on tie")
    func testModeTie() throws {
        #expect(approx(try mode([1, 2, 1, 2, 3]), 1.0))
    }

    @Test("mode of single value")
    func testModeSingle() throws {
        #expect(approx(try mode([99]), 99.0))
    }

    @Test("mode errors on empty input")
    func testModeEmpty() {
        #expect(throws: StatsError.self) {
            try mode([])
        }
    }

    // -- Variance -----------------------------------------------------------

    @Test("sample variance of worked example")
    func testVarianceSample() throws {
        // Parity test vector: 4.571428571428571
        #expect(approx(try variance([2, 4, 4, 4, 5, 5, 7, 9]), 4.571428571428571))
    }

    @Test("population variance of worked example")
    func testVariancePopulation() throws {
        // Parity test vector: 4.0
        #expect(approx(try variance([2, 4, 4, 4, 5, 5, 7, 9], population: true), 4.0))
    }

    @Test("sample variance of two values")
    func testVarianceTwo() throws {
        #expect(approx(try variance([1, 3]), 2.0))
    }

    @Test("sample variance errors on single value")
    func testVarianceSingleSample() {
        #expect(throws: StatsError.self) {
            try variance([42])
        }
    }

    @Test("population variance of single value")
    func testVarianceSinglePopulation() throws {
        #expect(approx(try variance([42], population: true), 0.0))
    }

    // -- Standard Deviation -------------------------------------------------

    @Test("sample standard deviation")
    func testStdDevSample() throws {
        #expect(approx(try standardDeviation([2, 4, 4, 4, 5, 5, 7, 9]), 2.13809, eps: 1e-4))
    }

    @Test("population standard deviation")
    func testStdDevPopulation() throws {
        #expect(approx(try standardDeviation([2, 4, 4, 4, 5, 5, 7, 9], population: true), 2.0))
    }

    // -- Min / Max / Range --------------------------------------------------

    @Test("min of worked example")
    func testMin() throws {
        #expect(approx(try statsMin([2, 4, 4, 4, 5, 5, 7, 9]), 2.0))
    }

    @Test("min with negative values")
    func testMinNegative() throws {
        #expect(approx(try statsMin([3, -1, 7]), -1.0))
    }

    @Test("min errors on empty input")
    func testMinEmpty() {
        #expect(throws: StatsError.self) {
            try statsMin([])
        }
    }

    @Test("max of worked example")
    func testMax() throws {
        #expect(approx(try statsMax([2, 4, 4, 4, 5, 5, 7, 9]), 9.0))
    }

    @Test("max errors on empty input")
    func testMaxEmpty() {
        #expect(throws: StatsError.self) {
            try statsMax([])
        }
    }

    @Test("range of worked example")
    func testRange() throws {
        #expect(approx(try statsRange([2, 4, 4, 4, 5, 5, 7, 9]), 7.0))
    }

    @Test("range of identical values")
    func testRangeIdentical() throws {
        #expect(approx(try statsRange([5, 5, 5]), 0.0))
    }
}

// ============================================================================
// Frequency Analysis Tests
// ============================================================================

@Suite("Frequency Analysis")
struct FrequencyTests {

    // -- frequencyCount -----------------------------------------------------

    @Test("frequency count counts letters case-insensitively")
    func testFrequencyCountBasic() {
        let counts = frequencyCount("Hello")
        #expect(counts["H"] == 1)
        #expect(counts["E"] == 1)
        #expect(counts["L"] == 2)
        #expect(counts["O"] == 1)
    }

    @Test("frequency count ignores non-alphabetic")
    func testFrequencyCountNonAlpha() {
        let counts = frequencyCount("A1B2C3!!!")
        #expect(counts["A"] == 1)
        #expect(counts["B"] == 1)
        #expect(counts["C"] == 1)
        #expect(counts["D"] == 0)
    }

    @Test("frequency count of empty string")
    func testFrequencyCountEmpty() {
        let counts = frequencyCount("")
        #expect(counts["A"] == 0)
    }

    // -- frequencyDistribution ----------------------------------------------

    @Test("frequency distribution computes proportions")
    func testFrequencyDistribution() {
        let dist = frequencyDistribution("AABB")
        #expect(approx(dist["A"]!, 0.5))
        #expect(approx(dist["B"]!, 0.5))
        #expect(approx(dist["C"]!, 0.0))
    }

    @Test("frequency distribution of empty string")
    func testFrequencyDistributionEmpty() {
        let dist = frequencyDistribution("")
        #expect(approx(dist["A"]!, 0.0))
    }

    // -- chiSquared ---------------------------------------------------------

    @Test("chi-squared for parallel arrays")
    func testChiSquared() throws {
        // Parity test vector: 10.0
        let result = try chiSquared(observed: [10, 20, 30], expected: [20, 20, 20])
        #expect(approx(result, 10.0))
    }

    @Test("chi-squared for identical distributions")
    func testChiSquaredIdentical() throws {
        let result = try chiSquared(observed: [10, 10, 10], expected: [10, 10, 10])
        #expect(approx(result, 0.0))
    }

    @Test("chi-squared errors on mismatched lengths")
    func testChiSquaredMismatch() {
        #expect(throws: StatsError.self) {
            try chiSquared(observed: [1, 2], expected: [1, 2, 3])
        }
    }

    // -- chiSquaredText -----------------------------------------------------

    @Test("chi-squared of text against English")
    func testChiSquaredText() {
        let result = chiSquaredText("AABB", expectedFreq: englishFrequencies)
        #expect(result > 0)
    }

    @Test("chi-squared of empty text")
    func testChiSquaredTextEmpty() {
        let result = chiSquaredText("", expectedFreq: englishFrequencies)
        #expect(approx(result, 0.0))
    }

    @Test("chi-squared of non-alpha text")
    func testChiSquaredTextNonAlpha() {
        let result = chiSquaredText("12345!!!", expectedFreq: englishFrequencies)
        #expect(approx(result, 0.0))
    }
}

// ============================================================================
// Cryptanalysis Helpers Tests
// ============================================================================

@Suite("Cryptanalysis Helpers")
struct CryptanalysisTests {

    // -- indexOfCoincidence -------------------------------------------------

    @Test("IC of AABB")
    func testICAABB() {
        // Parity test vector: 4/12 = 0.333...
        #expect(approx(indexOfCoincidence("AABB"), 1.0 / 3.0))
    }

    @Test("IC of single letter is 0")
    func testICSingle() {
        #expect(approx(indexOfCoincidence("A"), 0.0))
    }

    @Test("IC of empty string is 0")
    func testICEmpty() {
        #expect(approx(indexOfCoincidence(""), 0.0))
    }

    @Test("IC of same letter is 1.0")
    func testICSameLetter() {
        #expect(approx(indexOfCoincidence("AAAA"), 1.0))
    }

    @Test("IC of pangram is positive")
    func testICEnglish() {
        // A pangram has near-uniform distribution so IC is low
        let result = indexOfCoincidence("THEQUICKBROWNFOXJUMPSOVERTHELAZYDOG")
        #expect(result > 0)
    }

    // -- entropy ------------------------------------------------------------

    @Test("entropy of uniform 26-letter text")
    func testEntropyUniform() {
        let text = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
        let result = entropy(text)
        #expect(approx(result, log(26.0) / log(2.0), eps: 0.01))
    }

    @Test("entropy of single-letter text")
    func testEntropySingle() {
        #expect(approx(entropy("AAAA"), 0.0))
    }

    @Test("entropy of two-letter uniform distribution")
    func testEntropyTwoLetter() {
        // H("AABB") = 1.0
        #expect(approx(entropy("AABB"), 1.0, eps: 0.01))
    }

    @Test("entropy of empty text")
    func testEntropyEmpty() {
        #expect(approx(entropy(""), 0.0))
    }

    // -- englishFrequencies -------------------------------------------------

    @Test("english frequencies has 26 entries")
    func testEnglishFreqCount() {
        #expect(englishFrequencies.count == 26)
    }

    @Test("english frequencies sum to approximately 1.0")
    func testEnglishFreqSum() {
        let total = englishFrequencies.values.reduce(0.0, +)
        #expect(approx(total, 1.0, eps: 0.01))
    }

    @Test("E is the most common letter")
    func testEnglishFreqEMostCommon() {
        let maxEntry = englishFrequencies.max(by: { $0.value < $1.value })!
        #expect(maxEntry.key == "E")
    }
}
