import XCTest
@testable import VigenereCipher

final class VigenereCipherTests: XCTestCase {

    // A long English text for cryptanalysis testing. Needs 200+ characters
    // to give the IC analysis enough statistical data to work with.
    let longText = "The Vigenere cipher was long considered unbreakable and was known as "
        + "le chiffre indechiffrable for three hundred years until Friedrich "
        + "Kasiski published a general method of cryptanalysis in eighteen "
        + "sixty three which exploits the repeating nature of the keyword to "
        + "determine the key length and then uses frequency analysis on each "
        + "group of letters encrypted with the same key letter to recover the "
        + "original plaintext message without knowing the secret keyword at all "
        + "this technique works because each group of letters encrypted with the "
        + "same key letter forms a simple caesar cipher which can be broken by "
        + "comparing the frequency distribution of letters against the expected "
        + "frequencies found in normal english language text passages and "
        + "selecting the shift value that produces the closest match"

    // =====================================================================
    // Encryption Tests
    // =====================================================================

    func testEncryptParityVector() throws {
        XCTAssertEqual(try VigenereCipher.encrypt("ATTACKATDAWN", key: "LEMON"), "LXFOPVEFRNHR")
    }

    func testEncryptMixedCaseWithPunctuation() throws {
        XCTAssertEqual(try VigenereCipher.encrypt("Hello, World!", key: "key"), "Rijvs, Uyvjn!")
    }

    func testEncryptNonAlphaPassesThrough() throws {
        XCTAssertEqual(try VigenereCipher.encrypt("A-T-T", key: "LEM"), "L-X-F")
    }

    func testEncryptKeyWrapsAround() throws {
        // Key "AB" = shifts [0,1]. A(+0)=A, B(+1)=C, B(+0)=B, A(+1)=B
        XCTAssertEqual(try VigenereCipher.encrypt("ABBA", key: "AB"), "ACBB")
    }

    func testEncryptSingleCharacter() throws {
        XCTAssertEqual(try VigenereCipher.encrypt("A", key: "B"), "B")
    }

    func testEncryptLowercaseKey() throws {
        XCTAssertEqual(try VigenereCipher.encrypt("ATTACKATDAWN", key: "lemon"), "LXFOPVEFRNHR")
    }

    func testEncryptUppercaseKeyLowercaseText() throws {
        XCTAssertEqual(try VigenereCipher.encrypt("attackatdawn", key: "LEMON"), "lxfopvefrnhr")
    }

    func testEncryptEmptyPlaintext() throws {
        XCTAssertEqual(try VigenereCipher.encrypt("", key: "key"), "")
    }

    func testEncryptEmptyKeyThrows() {
        XCTAssertThrowsError(try VigenereCipher.encrypt("hello", key: ""))
    }

    func testEncryptNonAlphaKeyThrows() {
        XCTAssertThrowsError(try VigenereCipher.encrypt("hello", key: "key1"))
    }

    // =====================================================================
    // Decryption Tests
    // =====================================================================

    func testDecryptParityVector() throws {
        XCTAssertEqual(try VigenereCipher.decrypt("LXFOPVEFRNHR", key: "LEMON"), "ATTACKATDAWN")
    }

    func testDecryptMixedCaseWithPunctuation() throws {
        XCTAssertEqual(try VigenereCipher.decrypt("Rijvs, Uyvjn!", key: "key"), "Hello, World!")
    }

    func testDecryptNonAlphaPreserved() throws {
        XCTAssertEqual(try VigenereCipher.decrypt("L-X-F", key: "LEM"), "A-T-T")
    }

    func testDecryptSingleCharacter() throws {
        XCTAssertEqual(try VigenereCipher.decrypt("B", key: "B"), "A")
    }

    func testDecryptEmptyCiphertext() throws {
        XCTAssertEqual(try VigenereCipher.decrypt("", key: "key"), "")
    }

    func testDecryptEmptyKeyThrows() {
        XCTAssertThrowsError(try VigenereCipher.decrypt("hello", key: ""))
    }

    // =====================================================================
    // Round Trip Tests
    // =====================================================================

    func testRoundTripUppercase() throws {
        let text = "ATTACKATDAWN"
        XCTAssertEqual(try VigenereCipher.decrypt(try VigenereCipher.encrypt(text, key: "LEMON"), key: "LEMON"), text)
    }

    func testRoundTripMixedCase() throws {
        let text = "Hello, World! This is a test of the Vigenere cipher."
        XCTAssertEqual(try VigenereCipher.decrypt(try VigenereCipher.encrypt(text, key: "secret"), key: "secret"), text)
    }

    func testRoundTripVariousKeys() throws {
        let keys = ["A", "KEY", "LONGER", "VERYLONGKEYWORD"]
        for k in keys {
            let ct = try VigenereCipher.encrypt(longText, key: k)
            let pt = try VigenereCipher.decrypt(ct, key: k)
            XCTAssertEqual(pt, longText, "Round trip failed for key=\(k)")
        }
    }

    // =====================================================================
    // Key Length Detection Tests
    // =====================================================================

    func testFindKeyLengthSix() throws {
        let ct = try VigenereCipher.encrypt(longText, key: "SECRET")
        let detected = VigenereCipher.findKeyLength(ct)
        XCTAssertEqual(detected % 6, 0, "Expected key length 6 or multiple, got \(detected)")
    }

    func testFindKeyLengthThree() throws {
        let ct = try VigenereCipher.encrypt(longText, key: "KEY")
        let detected = VigenereCipher.findKeyLength(ct)
        XCTAssertEqual(detected % 3, 0, "Expected key length 3 or multiple, got \(detected)")
    }

    func testFindKeyLengthRespectsMaxLength() throws {
        let ct = try VigenereCipher.encrypt(longText, key: "SECRET")
        let detected = VigenereCipher.findKeyLength(ct, maxLength: 4)
        XCTAssertTrue(detected >= 1 && detected <= 4, "Expected 1..4, got \(detected)")
    }

    // =====================================================================
    // Key Finding Tests
    // =====================================================================

    func testFindKeySECRET() throws {
        let ct = try VigenereCipher.encrypt(longText, key: "SECRET")
        let key = VigenereCipher.findKey(ct, keyLength: 6)
        XCTAssertEqual(key, "SECRET")
    }

    func testFindKeyKEY() throws {
        let ct = try VigenereCipher.encrypt(longText, key: "KEY")
        let key = VigenereCipher.findKey(ct, keyLength: 3)
        XCTAssertEqual(key, "KEY")
    }

    // =====================================================================
    // Full Break Tests
    // =====================================================================

    func testBreakCipherSECRET() throws {
        let ct = try VigenereCipher.encrypt(longText, key: "SECRET")
        let (key, pt) = try VigenereCipher.breakCipher(ct)
        // Key may be repeated (IC can find multiples of true length)
        XCTAssertEqual(pt, longText)
        XCTAssertEqual(key.count % 6, 0, "Key length should be multiple of 6, got \(key.count)")
    }

    func testBreakCipherKEY() throws {
        let ct = try VigenereCipher.encrypt(longText, key: "KEY")
        let (key, pt) = try VigenereCipher.breakCipher(ct)
        XCTAssertEqual(pt, longText)
        XCTAssertEqual(key.count % 3, 0, "Key length should be multiple of 3, got \(key.count)")
    }

    // =====================================================================
    // Edge Cases
    // =====================================================================

    func testKeyAIsIdentity() throws {
        let text = "Hello, World!"
        XCTAssertEqual(try VigenereCipher.encrypt(text, key: "A"), text)
    }

    func testKeyZShifts() throws {
        XCTAssertEqual(try VigenereCipher.encrypt("A", key: "Z"), "Z")
        XCTAssertEqual(try VigenereCipher.encrypt("B", key: "Z"), "A")
    }

    func testNumbersAndSymbolsRoundTrip() throws {
        let text = "Test 123 !@# end"
        let ct = try VigenereCipher.encrypt(text, key: "KEY")
        let pt = try VigenereCipher.decrypt(ct, key: "KEY")
        XCTAssertEqual(pt, text)
    }

    func testKeyDoesNotAdvanceOnNonAlpha() throws {
        // With key "AB" (shifts 0,1), spaces should not advance key
        // "A B" -> A(shift 0)=A, space, B(shift 1)=C
        XCTAssertEqual(try VigenereCipher.encrypt("A B", key: "AB"), "A C")
    }
}
