import XCTest
@testable import ScytaleCipher

final class ScytaleCipherTests: XCTestCase {

    // --- Encryption Tests ---

    func testEncryptHelloWorldKey3() throws {
        XCTAssertEqual(try ScytaleCipher.encrypt("HELLO WORLD", key: 3), "HLWLEOODL R ")
    }

    func testEncryptABCDEFKey2() throws {
        XCTAssertEqual(try ScytaleCipher.encrypt("ABCDEF", key: 2), "ACEBDF")
    }

    func testEncryptABCDEFKey3() throws {
        XCTAssertEqual(try ScytaleCipher.encrypt("ABCDEF", key: 3), "ADBECF")
    }

    func testEncryptKeyEqualsLength() throws {
        XCTAssertEqual(try ScytaleCipher.encrypt("ABCD", key: 4), "ABCD")
    }

    func testEncryptEmptyString() throws {
        XCTAssertEqual(try ScytaleCipher.encrypt("", key: 2), "")
    }

    func testEncryptKeyTooSmall() {
        XCTAssertThrowsError(try ScytaleCipher.encrypt("HELLO", key: 1))
    }

    func testEncryptKeyTooLarge() {
        XCTAssertThrowsError(try ScytaleCipher.encrypt("HI", key: 3))
    }

    // --- Decryption Tests ---

    func testDecryptHelloWorldKey3() throws {
        XCTAssertEqual(try ScytaleCipher.decrypt("HLWLEOODL R ", key: 3), "HELLO WORLD")
    }

    func testDecryptACEBDFKey2() throws {
        XCTAssertEqual(try ScytaleCipher.decrypt("ACEBDF", key: 2), "ABCDEF")
    }

    func testDecryptEmptyString() throws {
        XCTAssertEqual(try ScytaleCipher.decrypt("", key: 2), "")
    }

    func testPortableScalarRaggedAndPaddingVectorsMatch() throws {
        XCTAssertEqual(try ScytaleCipher.encrypt("A😀Bé", key: 3), "Aé😀 B ")
        XCTAssertEqual(try ScytaleCipher.decrypt("Aé😀 B ", key: 3), "A😀Bé")
        XCTAssertEqual(try ScytaleCipher.encrypt("Ae\u{0301}B", key: 3), "ABe \u{0301} ")
        XCTAssertEqual(try ScytaleCipher.decrypt("ABe \u{0301} ", key: 3), "Ae\u{0301}B")
        XCTAssertEqual(try ScytaleCipher.decrypt("ABCDEF", key: 4), "ACEFBD")
        XCTAssertEqual(try ScytaleCipher.decrypt("A\tB ", key: 2), "AB\t")
        XCTAssertEqual(try ScytaleCipher.decrypt("A\u{00A0}\t \n ", key: 3), "A\t\n\u{00A0}")
    }

    func testDecryptInvalidKey() {
        XCTAssertThrowsError(try ScytaleCipher.decrypt("HELLO", key: 0))
        XCTAssertThrowsError(try ScytaleCipher.decrypt("HI", key: 3))
    }

    // --- Round Trip Tests ---

    func testRoundTripHelloWorld() throws {
        let text = "HELLO WORLD"
        let ct = try ScytaleCipher.encrypt(text, key: 3)
        let pt = try ScytaleCipher.decrypt(ct, key: 3)
        XCTAssertEqual(pt, text)
    }

    func testRoundTripVariousKeys() throws {
        let text = "The quick brown fox jumps over the lazy dog!"
        let n = text.count
        for key in 2...(n / 2) {
            let ct = try ScytaleCipher.encrypt(text, key: key)
            let pt = try ScytaleCipher.decrypt(ct, key: key)
            XCTAssertEqual(pt, text, "Round trip failed for key=\(key)")
        }
    }

    func testRoundTripWithPunctuation() throws {
        let text = "Hello, World! 123"
        let ct = try ScytaleCipher.encrypt(text, key: 4)
        let pt = try ScytaleCipher.decrypt(ct, key: 4)
        XCTAssertEqual(pt, text)
    }

    // --- Brute Force Tests ---

    func testBruteForceFindsOriginal() throws {
        let original = "HELLO WORLD"
        let ct = try ScytaleCipher.encrypt(original, key: 3)
        let results = try ScytaleCipher.bruteForce(ct)
        let found = results.first(where: { $0.key == 3 })
        XCTAssertNotNil(found)
        XCTAssertEqual(found?.text, original)
    }

    func testBruteForceReturnsAllKeys() throws {
        let results = try ScytaleCipher.bruteForce("ABCDEFGHIJ")
        let keys = results.map { $0.key }
        XCTAssertEqual(keys, [2, 3, 4, 5])
    }

    func testBruteForceShortText() throws {
        XCTAssertTrue(try ScytaleCipher.bruteForce("AB").isEmpty)
    }

    func testBruteForceRejectsOversizedScalarInput() {
        let oversized = String(repeating: "A", count: ScytaleCipher.maxBruteForceTextLength + 1)
        XCTAssertThrowsError(try ScytaleCipher.bruteForce(oversized)) { error in
            guard case ScytaleCipherError.bruteForceLimit = error else {
                return XCTFail("expected bruteForceLimit, got \(error)")
            }
        }
    }

    // --- Padding Tests ---

    func testPaddingStripped() throws {
        let ct = try ScytaleCipher.encrypt("HELLO", key: 3)
        XCTAssertEqual(try ScytaleCipher.decrypt(ct, key: 3), "HELLO")
    }

    func testNoPaddingNeeded() throws {
        let ct = try ScytaleCipher.encrypt("ABCDEF", key: 2)
        XCTAssertEqual(ct.count, 6)
    }
}
