import XCTest
@testable import CaesarCipher

// ============================================================================
// CaesarCipherTests — Comprehensive tests for the Caesar cipher library
// ============================================================================
//
// These tests cover:
//   1. Basic encryption and decryption
//   2. Round-trip (encrypt then decrypt returns original)
//   3. Case preservation
//   4. Non-alphabetic character passthrough
//   5. Empty string handling
//   6. Negative shifts
//   7. Shift wrapping (shifts >= 26)
//   8. ROT13 and its self-inverse property
//   9. Brute-force attack
//  10. Frequency analysis
//  11. Edge cases (all same letter, single character, etc.)
//
// ============================================================================

// MARK: - Encryption Tests

final class EncryptionTests: XCTestCase {

    /// The classic example: Caesar's own shift of 3.
    func testBasicEncryption() {
        XCTAssertEqual(encrypt("HELLO", shift: 3), "KHOOR")
    }

    /// Lowercase letters should produce lowercase output.
    func testLowercaseEncryption() {
        XCTAssertEqual(encrypt("hello", shift: 3), "khoor")
    }

    /// Mixed case should be preserved character by character.
    func testMixedCaseEncryption() {
        XCTAssertEqual(encrypt("Hello", shift: 3), "Khoor")
        XCTAssertEqual(encrypt("HeLLo WoRLd", shift: 5), "MjQQt BtWQi")
    }

    /// Shift of 0 should return the original text unchanged.
    func testZeroShift() {
        XCTAssertEqual(encrypt("Hello, World!", shift: 0), "Hello, World!")
    }

    /// Shift of 26 should also return the original text (full rotation).
    func testFullRotationShift() {
        XCTAssertEqual(encrypt("Hello", shift: 26), "Hello")
    }

    /// Shifts larger than 26 should wrap around.
    func testLargeShift() {
        // Shift 29 is equivalent to shift 3
        XCTAssertEqual(encrypt("HELLO", shift: 29), encrypt("HELLO", shift: 3))
        // Shift 52 is equivalent to shift 0 (two full rotations)
        XCTAssertEqual(encrypt("abc", shift: 52), "abc")
    }

    /// Wrapping at the end of the alphabet: X + 3 = A, Y + 3 = B, Z + 3 = C.
    func testAlphabetWrapping() {
        XCTAssertEqual(encrypt("XYZ", shift: 3), "ABC")
        XCTAssertEqual(encrypt("xyz", shift: 3), "abc")
    }

    /// The entire alphabet shifted by 1.
    func testFullAlphabetShift() {
        XCTAssertEqual(
            encrypt("abcdefghijklmnopqrstuvwxyz", shift: 1),
            "bcdefghijklmnopqrstuvwxyza"
        )
    }
}


// MARK: - Decryption Tests

final class DecryptionTests: XCTestCase {

    /// Basic decryption reverses encryption.
    func testBasicDecryption() {
        XCTAssertEqual(decrypt("KHOOR", shift: 3), "HELLO")
    }

    /// Decrypting lowercase ciphertext.
    func testLowercaseDecryption() {
        XCTAssertEqual(decrypt("khoor", shift: 3), "hello")
    }

    /// Decryption with wrapping: A - 3 = X.
    func testDecryptionWrapping() {
        XCTAssertEqual(decrypt("ABC", shift: 3), "XYZ")
    }
}


// MARK: - Round-Trip Tests

final class RoundTripTests: XCTestCase {

    /// Encrypt then decrypt with the same shift should return original text.
    func testRoundTrip() {
        let original = "The quick brown fox jumps over the lazy dog!"
        for shift in 0...25 {
            let encrypted = encrypt(original, shift: shift)
            let decrypted = decrypt(encrypted, shift: shift)
            XCTAssertEqual(decrypted, original, "Round-trip failed for shift \(shift)")
        }
    }

    /// Encrypt with positive shift, decrypt with negative shift of same magnitude.
    func testRoundTripNegativeDecrypt() {
        let text = "Hello, World!"
        let encrypted = encrypt(text, shift: 7)
        let decrypted = encrypt(encrypted, shift: -7)
        XCTAssertEqual(decrypted, text)
    }
}


// MARK: - Non-Alphabetic Passthrough Tests

final class PassthroughTests: XCTestCase {

    /// Digits should pass through unchanged.
    func testDigitsPassthrough() {
        XCTAssertEqual(encrypt("abc123", shift: 1), "bcd123")
    }

    /// Spaces should pass through unchanged.
    func testSpacesPassthrough() {
        XCTAssertEqual(encrypt("HELLO WORLD", shift: 3), "KHOOR ZRUOG")
    }

    /// Punctuation should pass through unchanged.
    func testPunctuationPassthrough() {
        XCTAssertEqual(encrypt("Hello, World!", shift: 3), "Khoor, Zruog!")
    }

    /// A string with only non-alphabetic characters should be unchanged.
    func testAllNonAlpha() {
        XCTAssertEqual(encrypt("123 !@#$%^&*()", shift: 5), "123 !@#$%^&*()")
    }

    /// Unicode characters beyond ASCII letters should pass through.
    /// Note: "cafe\u{0301}" is "caf" + "e\u{0301}" (e + combining accent).
    /// Swift treats "e\u{0301}" as a single Character (grapheme cluster).
    /// Since it's not a simple ASCII letter, it passes through unchanged.
    func testUnicodePassthrough() {
        // "cafe\u{0301}" → "c","a","f","e\u{0301}" as Characters
        // "c" → "f", "a" → "d", "f" → "i", "e\u{0301}" → passes through (grapheme cluster, not plain ASCII)
        XCTAssertEqual(encrypt("caf\u{00e9}", shift: 3), "fdi\u{00e9}")
    }
}


// MARK: - Empty String Tests

final class EmptyStringTests: XCTestCase {

    /// Encrypting an empty string should return an empty string.
    func testEncryptEmpty() {
        XCTAssertEqual(encrypt("", shift: 5), "")
    }

    /// Decrypting an empty string should return an empty string.
    func testDecryptEmpty() {
        XCTAssertEqual(decrypt("", shift: 5), "")
    }

    /// ROT13 of empty string should return empty string.
    func testRot13Empty() {
        XCTAssertEqual(rot13(""), "")
    }
}


// MARK: - Negative Shift Tests

final class NegativeShiftTests: XCTestCase {

    /// Negative shift should shift backward in the alphabet.
    func testNegativeShift() {
        XCTAssertEqual(encrypt("HELLO", shift: -3), "EBIIL")
    }

    /// A negative shift of -1 on 'a' should give 'z'.
    func testNegativeShiftWrapping() {
        XCTAssertEqual(encrypt("abc", shift: -1), "zab")
    }

    /// Negative shift is equivalent to the corresponding positive shift.
    func testNegativeEquivalence() {
        // Shift of -3 is the same as shift of 23
        XCTAssertEqual(
            encrypt("HELLO", shift: -3),
            encrypt("HELLO", shift: 23)
        )
    }

    /// Large negative shifts should wrap correctly.
    func testLargeNegativeShift() {
        // -29 mod 26 = -3 mod 26 = 23
        XCTAssertEqual(
            encrypt("HELLO", shift: -29),
            encrypt("HELLO", shift: -3)
        )
    }
}


// MARK: - ROT13 Tests

final class ROT13Tests: XCTestCase {

    /// Basic ROT13 transformation.
    func testBasicRot13() {
        XCTAssertEqual(rot13("Hello"), "Uryyb")
    }

    /// ROT13 is its own inverse: applying it twice returns the original.
    func testRot13SelfInverse() {
        let original = "The quick brown fox jumps over the lazy dog!"
        XCTAssertEqual(rot13(rot13(original)), original)
    }

    /// ROT13 applied to the full alphabet.
    func testRot13FullAlphabet() {
        XCTAssertEqual(
            rot13("abcdefghijklmnopqrstuvwxyz"),
            "nopqrstuvwxyzabcdefghijklm"
        )
    }

    /// ROT13 preserves non-alphabetic characters.
    func testRot13Passthrough() {
        XCTAssertEqual(rot13("Hello, World! 123"), "Uryyb, Jbeyq! 123")
    }

    /// ROT13 preserves case.
    func testRot13CasePreservation() {
        XCTAssertEqual(rot13("ABCabc"), "NOPnop")
    }
}


// MARK: - Brute Force Tests

final class BruteForceTests: XCTestCase {

    /// Brute force should return exactly 26 results.
    func testBruteForceReturns26Results() {
        let results = bruteForce("KHOOR")
        XCTAssertEqual(results.count, 26)
    }

    /// Each result should have the correct shift value.
    func testBruteForceShiftValues() {
        let results = bruteForce("KHOOR")
        for (index, result) in results.enumerated() {
            XCTAssertEqual(result.shift, index)
        }
    }

    /// The correct decryption should appear at the right shift.
    func testBruteForceFindsCorrectDecryption() {
        let results = bruteForce("KHOOR")
        // "KHOOR" was encrypted with shift 3 from "HELLO"
        XCTAssertEqual(results[3].plaintext, "HELLO")
    }

    /// Shift 0 should return the ciphertext unchanged.
    func testBruteForceShiftZero() {
        let results = bruteForce("KHOOR")
        XCTAssertEqual(results[0].plaintext, "KHOOR")
    }

    /// Brute force on empty string should return 26 empty results.
    func testBruteForceEmptyString() {
        let results = bruteForce("")
        XCTAssertEqual(results.count, 26)
        for result in results {
            XCTAssertEqual(result.plaintext, "")
        }
    }

    /// Brute force preserves non-alpha characters in all results.
    func testBruteForcePreservesNonAlpha() {
        let results = bruteForce("HI! 123")
        for result in results {
            XCTAssertTrue(result.plaintext.contains("! 123"))
        }
    }
}


// MARK: - Frequency Analysis Tests

final class FrequencyAnalysisTests: XCTestCase {

    /// Frequency analysis should detect a shift of 3 on a long enough text.
    func testFrequencyAnalysisBasic() {
        // A sufficiently long English plaintext
        let plaintext = "the quick brown fox jumps over the lazy dog and the cat sat on the mat"
        let ciphertext = encrypt(plaintext, shift: 3)
        let result = frequencyAnalysis(ciphertext)
        XCTAssertEqual(result.shift, 3)
        XCTAssertEqual(result.plaintext, plaintext)
    }

    /// Frequency analysis on a longer text with shift 17.
    func testFrequencyAnalysisLargerShift() {
        let plaintext = "in the beginning there was nothing and then there was something and it was good and the people rejoiced"
        let ciphertext = encrypt(plaintext, shift: 17)
        let result = frequencyAnalysis(ciphertext)
        XCTAssertEqual(result.shift, 17)
        XCTAssertEqual(result.plaintext, plaintext)
    }

    /// Frequency analysis with shift 0 should detect shift 0.
    func testFrequencyAnalysisShiftZero() {
        let plaintext = "the quick brown fox jumps over the lazy dog which is a common pangram"
        let result = frequencyAnalysis(plaintext)
        XCTAssertEqual(result.shift, 0)
        XCTAssertEqual(result.plaintext, plaintext)
    }

    /// Frequency analysis on text with no letters returns shift 0.
    func testFrequencyAnalysisNoLetters() {
        let result = frequencyAnalysis("12345 !@#$%")
        XCTAssertEqual(result.shift, 0)
        XCTAssertEqual(result.plaintext, "12345 !@#$%")
    }

    /// Frequency analysis on empty string returns shift 0.
    func testFrequencyAnalysisEmptyString() {
        let result = frequencyAnalysis("")
        XCTAssertEqual(result.shift, 0)
        XCTAssertEqual(result.plaintext, "")
    }

    /// Frequency analysis on mixed-case text with punctuation.
    func testFrequencyAnalysisMixedCase() {
        let plaintext = "Hello, World! The quick brown fox jumps over the lazy dog. This is a test of the frequency analysis."
        let ciphertext = encrypt(plaintext, shift: 7)
        let result = frequencyAnalysis(ciphertext)
        XCTAssertEqual(result.shift, 7)
        XCTAssertEqual(result.plaintext, plaintext)
    }
}


// MARK: - English Frequencies Tests

final class EnglishFrequenciesTests: XCTestCase {

    /// The frequency table should have entries for all 26 letters.
    func testFrequencyTableCompleteness() {
        XCTAssertEqual(englishFrequencies.count, 26)
        for scalar in UnicodeScalar("a").value...UnicodeScalar("z").value {
            let char = Character(UnicodeScalar(scalar)!)
            XCTAssertNotNil(
                englishFrequencies[char],
                "Missing frequency for '\(char)'"
            )
        }
    }

    /// All frequencies should be positive.
    func testFrequenciesArePositive() {
        for (char, freq) in englishFrequencies {
            XCTAssertGreaterThan(freq, 0.0, "Frequency for '\(char)' should be positive")
        }
    }

    /// The frequencies should approximately sum to 1.0.
    func testFrequenciesSumToOne() {
        let total = englishFrequencies.values.reduce(0.0, +)
        XCTAssertEqual(total, 1.0, accuracy: 0.01, "Frequencies should sum to approximately 1.0")
    }

    /// 'E' should be the most frequent letter.
    func testMostFrequentIsE() {
        let maxEntry = englishFrequencies.max(by: { $0.value < $1.value })!
        XCTAssertEqual(maxEntry.key, "e")
    }

    /// 'Z' should be the least frequent letter.
    func testLeastFrequentIsZ() {
        let minEntry = englishFrequencies.min(by: { $0.value < $1.value })!
        XCTAssertEqual(minEntry.key, "z")
    }
}


// MARK: - Edge Case Tests

final class EdgeCaseTests: XCTestCase {

    /// Single character encryption.
    func testSingleCharacter() {
        XCTAssertEqual(encrypt("A", shift: 1), "B")
        XCTAssertEqual(encrypt("Z", shift: 1), "A")
        XCTAssertEqual(encrypt("a", shift: 1), "b")
        XCTAssertEqual(encrypt("z", shift: 1), "a")
    }

    /// A string of all the same letter.
    func testRepeatedCharacter() {
        XCTAssertEqual(encrypt("AAAA", shift: 5), "FFFF")
    }

    /// Very large shift values.
    func testVeryLargeShift() {
        // 1000 mod 26 = 12
        XCTAssertEqual(
            encrypt("HELLO", shift: 1000),
            encrypt("HELLO", shift: 1000 % 26)
        )
    }

    /// BruteForceResult equality.
    func testBruteForceResultEquality() {
        let a = BruteForceResult(shift: 3, plaintext: "HELLO")
        let b = BruteForceResult(shift: 3, plaintext: "HELLO")
        let c = BruteForceResult(shift: 4, plaintext: "HELLO")
        XCTAssertEqual(a, b)
        XCTAssertNotEqual(a, c)
    }
}
