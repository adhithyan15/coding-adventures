import XCTest
@testable import AtbashCipher

/// Comprehensive tests for the Atbash cipher implementation.
///
/// These tests verify that the Atbash cipher correctly reverses the alphabet
/// for both uppercase and lowercase letters, preserves non-alphabetic
/// characters, and satisfies the self-inverse property.
final class AtbashCipherTests: XCTestCase {

    // MARK: - Basic Encryption

    /// H(7)->S(18), E(4)->V(21), L(11)->O(14), L(11)->O(14), O(14)->L(11)
    func testEncryptHelloUppercase() {
        XCTAssertEqual(AtbashCipher.encrypt("HELLO"), "SVOOL")
    }

    func testEncryptHelloLowercase() {
        XCTAssertEqual(AtbashCipher.encrypt("hello"), "svool")
    }

    func testEncryptMixedCaseWithPunctuation() {
        XCTAssertEqual(AtbashCipher.encrypt("Hello, World! 123"), "Svool, Dliow! 123")
    }

    func testEncryptFullUppercaseAlphabet() {
        XCTAssertEqual(
            AtbashCipher.encrypt("ABCDEFGHIJKLMNOPQRSTUVWXYZ"),
            "ZYXWVUTSRQPONMLKJIHGFEDCBA"
        )
    }

    func testEncryptFullLowercaseAlphabet() {
        XCTAssertEqual(
            AtbashCipher.encrypt("abcdefghijklmnopqrstuvwxyz"),
            "zyxwvutsrqponmlkjihgfedcba"
        )
    }

    // MARK: - Case Preservation

    func testUppercaseStaysUppercase() {
        XCTAssertEqual(AtbashCipher.encrypt("ABC"), "ZYX")
    }

    func testLowercaseStaysLowercase() {
        XCTAssertEqual(AtbashCipher.encrypt("abc"), "zyx")
    }

    func testMixedCasePreserved() {
        XCTAssertEqual(AtbashCipher.encrypt("AbCdEf"), "ZyXwVu")
    }

    // MARK: - Non-Alpha Passthrough

    func testDigitsUnchanged() {
        XCTAssertEqual(AtbashCipher.encrypt("12345"), "12345")
    }

    func testPunctuationUnchanged() {
        XCTAssertEqual(AtbashCipher.encrypt("!@#$%"), "!@#$%")
    }

    func testSpacesUnchanged() {
        XCTAssertEqual(AtbashCipher.encrypt("   "), "   ")
    }

    func testMixedAlphaAndDigits() {
        XCTAssertEqual(AtbashCipher.encrypt("A1B2C3"), "Z1Y2X3")
    }

    func testNewlinesAndTabs() {
        XCTAssertEqual(AtbashCipher.encrypt("A\nB\tC"), "Z\nY\tX")
    }

    // MARK: - Self-Inverse Property
    // The most important mathematical property: encrypt(encrypt(x)) == x

    func testSelfInverseHello() {
        XCTAssertEqual(AtbashCipher.encrypt(AtbashCipher.encrypt("HELLO")), "HELLO")
    }

    func testSelfInverseLowercase() {
        XCTAssertEqual(AtbashCipher.encrypt(AtbashCipher.encrypt("hello")), "hello")
    }

    func testSelfInverseMixed() {
        let input = "Hello, World! 123"
        XCTAssertEqual(AtbashCipher.encrypt(AtbashCipher.encrypt(input)), input)
    }

    func testSelfInverseFullAlphabet() {
        let alpha = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
        XCTAssertEqual(AtbashCipher.encrypt(AtbashCipher.encrypt(alpha)), alpha)
    }

    func testSelfInverseEmpty() {
        XCTAssertEqual(AtbashCipher.encrypt(AtbashCipher.encrypt("")), "")
    }

    func testSelfInverseLongText() {
        let text = "The quick brown fox jumps over the lazy dog! 42"
        XCTAssertEqual(AtbashCipher.encrypt(AtbashCipher.encrypt(text)), text)
    }

    // MARK: - Edge Cases

    func testEmptyString() {
        XCTAssertEqual(AtbashCipher.encrypt(""), "")
    }

    func testSingleUppercaseLetters() {
        XCTAssertEqual(AtbashCipher.encrypt("A"), "Z")
        XCTAssertEqual(AtbashCipher.encrypt("Z"), "A")
        XCTAssertEqual(AtbashCipher.encrypt("M"), "N")
        XCTAssertEqual(AtbashCipher.encrypt("N"), "M")
    }

    func testSingleLowercaseLetters() {
        XCTAssertEqual(AtbashCipher.encrypt("a"), "z")
        XCTAssertEqual(AtbashCipher.encrypt("z"), "a")
    }

    func testSingleDigit() {
        XCTAssertEqual(AtbashCipher.encrypt("5"), "5")
    }

    /// No letter in the alphabet should map to itself under Atbash.
    /// This is because 25 - p == p only when p == 12.5, which is not an integer.
    func testNoLetterMapsToItself() {
        for i in 0..<26 {
            let upper = String(UnicodeScalar(65 + i)!)
            XCTAssertNotEqual(AtbashCipher.encrypt(upper), upper, "\(upper) maps to itself!")

            let lower = String(UnicodeScalar(97 + i)!)
            XCTAssertNotEqual(AtbashCipher.encrypt(lower), lower, "\(lower) maps to itself!")
        }
    }

    // MARK: - Decrypt

    func testDecryptSvool() {
        XCTAssertEqual(AtbashCipher.decrypt("SVOOL"), "HELLO")
    }

    func testDecryptLowercase() {
        XCTAssertEqual(AtbashCipher.decrypt("svool"), "hello")
    }

    func testDecryptIsEncryptInverse() {
        let texts = ["HELLO", "hello", "Hello, World! 123", "", "42"]
        for text in texts {
            XCTAssertEqual(AtbashCipher.decrypt(AtbashCipher.encrypt(text)), text)
        }
    }

    func testEncryptDecryptEquivalence() {
        let texts = ["HELLO", "svool", "Test!", ""]
        for text in texts {
            XCTAssertEqual(AtbashCipher.encrypt(text), AtbashCipher.decrypt(text))
        }
    }
}
