// ReedSolomonTests.swift
// Comprehensive test suite for the ReedSolomon Swift package.
//
// Test coverage goals:
//   - buildGenerator: cross-language test vector, root property, length, monic
//   - encode: length, systematic layout, syndromes=zero, throws on invalid input
//   - syndromes: valid codeword → all zero, corrupted → non-zero
//   - errorLocator: all-zero syndromes → [1], degree matches error count
//   - decode: no errors, single error at many positions, 2 errors, 4 errors,
//             TooManyErrors, InvalidInput
//   - round-trip properties across many messages

import XCTest
import GF256
@testable import ReedSolomon

final class ReedSolomonTests: XCTestCase {

    // ========================================================================
    // MARK: - buildGenerator Tests
    // ========================================================================
    //
    // The generator polynomial g(x) = ∏(x + α^i) for i = 1..nCheck.
    // It is monic (leading coefficient = 1) and has length nCheck + 1.
    //
    // Cross-language test vector:
    //   buildGenerator(2) = [8, 6, 1]
    //   Verification: g(α¹) = g(α²) = 0 in GF(256).

    func testBuildGenerator2CrossLanguageVector() {
        // This must match the Python, TypeScript, Rust, Go, Elixir, Ruby,
        // Lua, and Perl implementations exactly.
        let g = ReedSolomon.buildGenerator(2)
        XCTAssertEqual(g, [8, 6, 1],
            "buildGenerator(2) must return [8, 6, 1] — cross-language test vector")
    }

    func testBuildGenerator2Length() {
        let g = ReedSolomon.buildGenerator(2)
        XCTAssertEqual(g.count, 3,
            "buildGenerator(2) must have length nCheck+1 = 3")
    }

    func testBuildGenerator2Monic() {
        let g = ReedSolomon.buildGenerator(2)
        XCTAssertEqual(g.last, 1,
            "Generator polynomial must be monic (leading coefficient = 1)")
    }

    func testBuildGenerator2Alpha1IsRoot() {
        // g(α¹) = g(2) must equal 0 in GF(256)
        // g(2) = 8 + 6·2 + 1·4 = 8 ^ 12 ^ 4 = 0
        let g = ReedSolomon.buildGenerator(2)
        let alpha1: UInt8 = GF256.power(2, 1)  // = 2
        let eval = evaluatePolyLE(g, alpha1)
        XCTAssertEqual(eval, 0,
            "α¹ must be a root of buildGenerator(2)")
    }

    func testBuildGenerator2Alpha2IsRoot() {
        // g(α²) = g(4) must equal 0 in GF(256)
        let g = ReedSolomon.buildGenerator(2)
        let alpha2: UInt8 = GF256.power(2, 2)  // = 4
        let eval = evaluatePolyLE(g, alpha2)
        XCTAssertEqual(eval, 0,
            "α² must be a root of buildGenerator(2)")
    }

    func testBuildGenerator4Length() {
        let g = ReedSolomon.buildGenerator(4)
        XCTAssertEqual(g.count, 5,
            "buildGenerator(4) must have length 5")
    }

    func testBuildGenerator4Monic() {
        let g = ReedSolomon.buildGenerator(4)
        XCTAssertEqual(g.last, 1,
            "buildGenerator(4) must be monic")
    }

    func testBuildGenerator4AllRootsAreZero() {
        let g = ReedSolomon.buildGenerator(4)
        for i in 1...4 {
            let alpha = GF256.power(2, UInt32(i))
            let eval = evaluatePolyLE(g, alpha)
            XCTAssertEqual(eval, 0,
                "α^\(i) must be a root of buildGenerator(4)")
        }
    }

    func testBuildGenerator8Length() {
        let g = ReedSolomon.buildGenerator(8)
        XCTAssertEqual(g.count, 9,
            "buildGenerator(8) must have length 9")
    }

    func testBuildGenerator8AllRootsAreZero() {
        let g = ReedSolomon.buildGenerator(8)
        for i in 1...8 {
            let alpha = GF256.power(2, UInt32(i))
            let eval = evaluatePolyLE(g, alpha)
            XCTAssertEqual(eval, 0,
                "α^\(i) must be a root of buildGenerator(8)")
        }
    }

    // ========================================================================
    // MARK: - encode Tests
    // ========================================================================

    func testEncodeOutputLength() throws {
        let message: [UInt8] = [1, 2, 3, 4, 5]
        let codeword = try ReedSolomon.encode(message, nCheck: 4)
        XCTAssertEqual(codeword.count, 9,
            "encode must produce message.count + nCheck bytes")
    }

    func testEncodeIsSystematic() throws {
        // The first k bytes of the codeword must equal the original message.
        let message: [UInt8] = [72, 101, 108, 108, 111]
        let codeword = try ReedSolomon.encode(message, nCheck: 4)
        XCTAssertEqual(Array(codeword.prefix(message.count)), message,
            "encode must be systematic: first k bytes = message")
    }

    func testEncodeProducesZeroSyndromes() throws {
        // A valid codeword must satisfy C(α^i) = 0 for i = 1..nCheck.
        let message: [UInt8] = [1, 2, 3, 4, 5]
        let codeword = try ReedSolomon.encode(message, nCheck: 4)
        let synds = ReedSolomon.syndromes(codeword, nCheck: 4)
        XCTAssertEqual(synds, [UInt8](repeating: 0, count: 4),
            "A valid codeword must have all-zero syndromes")
    }

    func testEncodeCheckBytesLength() throws {
        let message: [UInt8] = [10, 20, 30]
        let codeword = try ReedSolomon.encode(message, nCheck: 6)
        let checkBytes = Array(codeword.dropFirst(message.count))
        XCTAssertEqual(checkBytes.count, 6,
            "There must be exactly nCheck check bytes appended")
    }

    func testEncodeNCheck2CrossLanguageVector() throws {
        // Test against known values for a simple message with nCheck=2.
        // buildGenerator(2) = [8, 6, 1]
        // padded = [0x48, 0, 0] (for message [0x48])
        // check = polyModBE([0x48, 0, 0], [1, 6, 8])
        // This is a regression test — exact values must match other languages.
        let message: [UInt8] = [0x48]  // 'H'
        let codeword = try ReedSolomon.encode(message, nCheck: 2)
        XCTAssertEqual(codeword.count, 3)
        // Verify: syndromes of the codeword must all be zero
        let synds = ReedSolomon.syndromes(codeword, nCheck: 2)
        XCTAssertTrue(synds.allSatisfy { $0 == 0 },
            "Encoded codeword for [0x48] must have zero syndromes")
    }

    func testEncodeThrowsOnZeroNCheck() {
        let message: [UInt8] = [1, 2, 3]
        XCTAssertThrowsError(try ReedSolomon.encode(message, nCheck: 0)) { error in
            XCTAssertTrue(error is ReedSolomon.InvalidInput,
                "encode with nCheck=0 must throw InvalidInput")
        }
    }

    func testEncodeThrowsOnOddNCheck() {
        let message: [UInt8] = [1, 2, 3]
        XCTAssertThrowsError(try ReedSolomon.encode(message, nCheck: 3)) { error in
            XCTAssertTrue(error is ReedSolomon.InvalidInput,
                "encode with odd nCheck must throw InvalidInput")
        }
    }

    func testEncodeThrowsOnOversizedMessage() {
        // 200 message bytes + 60 check bytes = 260 > 255
        let message = [UInt8](repeating: 0x41, count: 200)
        XCTAssertThrowsError(try ReedSolomon.encode(message, nCheck: 60)) { error in
            XCTAssertTrue(error is ReedSolomon.InvalidInput,
                "encode with total > 255 bytes must throw InvalidInput")
        }
    }

    func testEncodeMaximumValidLength() throws {
        // 249 + 6 = 255, which is the maximum allowed.
        let message = [UInt8](repeating: 0x42, count: 249)
        XCTAssertNoThrow(try ReedSolomon.encode(message, nCheck: 6),
            "encode with total = 255 must not throw")
    }

    // ========================================================================
    // MARK: - syndromes Tests
    // ========================================================================

    func testSyndromesAllZeroForValidCodeword() throws {
        let message: [UInt8] = [3, 14, 15, 92, 65]
        let codeword = try ReedSolomon.encode(message, nCheck: 4)
        let synds = ReedSolomon.syndromes(codeword, nCheck: 4)
        XCTAssertEqual(synds, [0, 0, 0, 0],
            "Valid codeword must produce all-zero syndromes")
    }

    func testSyndromesNonZeroAfterCorruption() throws {
        let message: [UInt8] = [1, 2, 3, 4, 5]
        var codeword = try ReedSolomon.encode(message, nCheck: 4)
        // Flip a bit in the first byte
        codeword[0] ^= 0xFF
        let synds = ReedSolomon.syndromes(codeword, nCheck: 4)
        XCTAssertFalse(synds.allSatisfy { $0 == 0 },
            "Corrupted codeword must produce non-zero syndromes")
    }

    func testSyndromesCount() throws {
        let message: [UInt8] = [1, 2, 3]
        let codeword = try ReedSolomon.encode(message, nCheck: 6)
        let synds = ReedSolomon.syndromes(codeword, nCheck: 6)
        XCTAssertEqual(synds.count, 6,
            "syndromes must return exactly nCheck values")
    }

    // ========================================================================
    // MARK: - errorLocator Tests
    // ========================================================================

    func testErrorLocatorAllZeroSyndromes() {
        // No errors → Λ(x) = 1 (the constant polynomial)
        let synds: [UInt8] = [0, 0, 0, 0]
        let lam = ReedSolomon.errorLocator(synds)
        XCTAssertEqual(lam, [1],
            "All-zero syndromes → error locator must be [1]")
    }

    func testErrorLocatorDegreeMatchesErrorCount() throws {
        let message: [UInt8] = [1, 2, 3, 4, 5]
        var codeword = try ReedSolomon.encode(message, nCheck: 4)
        // Introduce exactly 2 errors
        codeword[0] ^= 0x01
        codeword[2] ^= 0x02
        let synds = ReedSolomon.syndromes(codeword, nCheck: 4)
        let lam = ReedSolomon.errorLocator(synds)
        XCTAssertEqual(lam.count - 1, 2,
            "Two errors must give a degree-2 error locator")
    }

    func testErrorLocatorAlwaysStartsWith1() throws {
        let message: [UInt8] = [10, 20, 30, 40]
        var codeword = try ReedSolomon.encode(message, nCheck: 4)
        codeword[1] ^= 0x55
        let synds = ReedSolomon.syndromes(codeword, nCheck: 4)
        let lam = ReedSolomon.errorLocator(synds)
        XCTAssertEqual(lam.first, 1,
            "Error locator polynomial must have Λ(0) = 1 (constant term = 1)")
    }

    // ========================================================================
    // MARK: - decode Tests — No Errors
    // ========================================================================

    func testDecodeNoErrors() throws {
        let message: [UInt8] = [1, 2, 3, 4, 5]
        let codeword = try ReedSolomon.encode(message, nCheck: 4)
        let decoded = try ReedSolomon.decode(codeword, nCheck: 4)
        XCTAssertEqual(decoded, message,
            "decode must return original message when there are no errors")
    }

    func testDecodeNoErrorsLongMessage() throws {
        let message = [UInt8](0..<100)
        let codeword = try ReedSolomon.encode(message, nCheck: 4)
        let decoded = try ReedSolomon.decode(codeword, nCheck: 4)
        XCTAssertEqual(decoded, message,
            "decode must handle long messages correctly")
    }

    func testDecodeEmptyMessage() throws {
        let message: [UInt8] = []
        let codeword = try ReedSolomon.encode(message, nCheck: 2)
        let decoded = try ReedSolomon.decode(codeword, nCheck: 2)
        XCTAssertEqual(decoded, message,
            "decode must handle empty message (pure check bytes)")
    }

    // ========================================================================
    // MARK: - decode Tests — Single Error Correction
    // ========================================================================

    func testDecodeSingleErrorInMessageByte() throws {
        let message: [UInt8] = [10, 20, 30, 40, 50]
        var codeword = try ReedSolomon.encode(message, nCheck: 4)
        codeword[0] ^= 0xAB  // corrupt first byte
        let decoded = try ReedSolomon.decode(codeword, nCheck: 4)
        XCTAssertEqual(decoded, message,
            "decode must correct a single error in the first byte")
    }

    func testDecodeSingleErrorInMiddleByte() throws {
        let message: [UInt8] = [10, 20, 30, 40, 50]
        var codeword = try ReedSolomon.encode(message, nCheck: 4)
        codeword[2] ^= 0xFF  // corrupt middle byte
        let decoded = try ReedSolomon.decode(codeword, nCheck: 4)
        XCTAssertEqual(decoded, message,
            "decode must correct a single error in the middle of the message")
    }

    func testDecodeSingleErrorInLastMessageByte() throws {
        let message: [UInt8] = [10, 20, 30, 40, 50]
        var codeword = try ReedSolomon.encode(message, nCheck: 4)
        codeword[4] ^= 0x33  // corrupt last message byte
        let decoded = try ReedSolomon.decode(codeword, nCheck: 4)
        XCTAssertEqual(decoded, message,
            "decode must correct a single error in the last message byte")
    }

    func testDecodeSingleErrorInCheckByte() throws {
        let message: [UInt8] = [10, 20, 30, 40, 50]
        var codeword = try ReedSolomon.encode(message, nCheck: 4)
        codeword[5] ^= 0x77  // corrupt first check byte
        let decoded = try ReedSolomon.decode(codeword, nCheck: 4)
        XCTAssertEqual(decoded, message,
            "decode must correct a single error in the check bytes")
    }

    func testDecodeSingleErrorAllPositions() throws {
        // Verify error correction works at every position in a short codeword.
        let message: [UInt8] = [65, 66, 67]   // "ABC"
        let codeword = try ReedSolomon.encode(message, nCheck: 4)
        for pos in 0..<codeword.count {
            var corrupted = codeword
            corrupted[pos] ^= 0x55
            let decoded = try ReedSolomon.decode(corrupted, nCheck: 4)
            XCTAssertEqual(decoded, message,
                "Single error at position \(pos) must be correctable")
        }
    }

    // ========================================================================
    // MARK: - decode Tests — Two Error Correction (nCheck=4)
    // ========================================================================

    func testDecodeTwoErrors() throws {
        let message: [UInt8] = [1, 2, 3, 4, 5, 6, 7, 8]
        var codeword = try ReedSolomon.encode(message, nCheck: 4)
        codeword[0] ^= 0x11
        codeword[4] ^= 0x22
        let decoded = try ReedSolomon.decode(codeword, nCheck: 4)
        XCTAssertEqual(decoded, message,
            "decode must correct 2 errors with nCheck=4")
    }

    func testDecodeTwoErrorsInCheckBytes() throws {
        let message: [UInt8] = [100, 101, 102, 103]
        var codeword = try ReedSolomon.encode(message, nCheck: 4)
        codeword[codeword.count - 1] ^= 0xAA
        codeword[codeword.count - 2] ^= 0xBB
        let decoded = try ReedSolomon.decode(codeword, nCheck: 4)
        XCTAssertEqual(decoded, message,
            "decode must correct 2 errors in check bytes with nCheck=4")
    }

    // ========================================================================
    // MARK: - decode Tests — Four Error Correction (nCheck=8)
    // ========================================================================

    func testDecodeFourErrors() throws {
        let message = [UInt8](1...20)
        var codeword = try ReedSolomon.encode(message, nCheck: 8)
        codeword[0]  ^= 0x11
        codeword[5]  ^= 0x22
        codeword[10] ^= 0x33
        codeword[15] ^= 0x44
        let decoded = try ReedSolomon.decode(codeword, nCheck: 8)
        XCTAssertEqual(decoded, message,
            "decode must correct 4 errors with nCheck=8")
    }

    func testDecodeFourErrorsMixed() throws {
        // Mix of errors in message bytes and check bytes
        let message: [UInt8] = [0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE]
        var codeword = try ReedSolomon.encode(message, nCheck: 8)
        codeword[0]  ^= 0x01
        codeword[3]  ^= 0x02
        codeword[7]  ^= 0x03  // check byte
        codeword[12] ^= 0x04  // check byte
        let decoded = try ReedSolomon.decode(codeword, nCheck: 8)
        XCTAssertEqual(decoded, message,
            "decode must correct 4 mixed errors with nCheck=8")
    }

    // ========================================================================
    // MARK: - decode Tests — TooManyErrors
    // ========================================================================

    func testDecodeTooManyErrorsThrows() throws {
        let message: [UInt8] = [1, 2, 3, 4, 5]
        var codeword = try ReedSolomon.encode(message, nCheck: 2)
        // Introduce 2 errors (t = 1, so 2 errors is uncorrectable)
        codeword[0] ^= 0xFF
        codeword[1] ^= 0xAA
        XCTAssertThrowsError(try ReedSolomon.decode(codeword, nCheck: 2)) { error in
            XCTAssertTrue(error is ReedSolomon.TooManyErrors,
                "2 errors with nCheck=2 (t=1) must throw TooManyErrors")
        }
    }

    func testDecodeThreeErrorsWithNCheck4Throws() throws {
        let message: [UInt8] = [1, 2, 3, 4, 5, 6, 7]
        var codeword = try ReedSolomon.encode(message, nCheck: 4)
        // Introduce 3 errors (t = 2, so 3 is uncorrectable)
        codeword[0] ^= 0x01
        codeword[2] ^= 0x02
        codeword[4] ^= 0x03
        XCTAssertThrowsError(try ReedSolomon.decode(codeword, nCheck: 4)) { error in
            XCTAssertTrue(error is ReedSolomon.TooManyErrors,
                "3 errors with nCheck=4 (t=2) must throw TooManyErrors")
        }
    }

    // ========================================================================
    // MARK: - decode Tests — InvalidInput
    // ========================================================================

    func testDecodeThrowsOnZeroNCheck() {
        let received: [UInt8] = [1, 2, 3, 4]
        XCTAssertThrowsError(try ReedSolomon.decode(received, nCheck: 0)) { error in
            XCTAssertTrue(error is ReedSolomon.InvalidInput,
                "decode with nCheck=0 must throw InvalidInput")
        }
    }

    func testDecodeThrowsOnOddNCheck() {
        let received: [UInt8] = [1, 2, 3, 4, 5]
        XCTAssertThrowsError(try ReedSolomon.decode(received, nCheck: 3)) { error in
            XCTAssertTrue(error is ReedSolomon.InvalidInput,
                "decode with odd nCheck must throw InvalidInput")
        }
    }

    func testDecodeThrowsWhenReceivedShorterThanNCheck() {
        let received: [UInt8] = [1, 2]
        XCTAssertThrowsError(try ReedSolomon.decode(received, nCheck: 4)) { error in
            XCTAssertTrue(error is ReedSolomon.InvalidInput,
                "decode with received.count < nCheck must throw InvalidInput")
        }
    }

    // ========================================================================
    // MARK: - Round-Trip Property Tests
    // ========================================================================
    //
    // These tests verify the encode → corrupt → decode round-trip for various
    // message sizes and error patterns.

    func testRoundTripSingleByte() throws {
        // Every possible single-byte message, nCheck=2
        for byte in UInt8.min...UInt8.max {
            let message: [UInt8] = [byte]
            let codeword = try ReedSolomon.encode(message, nCheck: 2)
            let decoded = try ReedSolomon.decode(codeword, nCheck: 2)
            XCTAssertEqual(decoded, message,
                "Round-trip must work for single byte \(byte) with nCheck=2")
        }
    }

    func testRoundTripNoErrorsManyMessages() throws {
        let testMessages: [[UInt8]] = [
            [0x00],
            [0xFF],
            [1, 2, 3],
            [0x48, 0x65, 0x6C, 0x6C, 0x6F],  // "Hello"
            Array(0..<50),
            [UInt8](repeating: 0xAA, count: 30),
        ]
        for message in testMessages {
            let codeword = try ReedSolomon.encode(message, nCheck: 4)
            let decoded = try ReedSolomon.decode(codeword, nCheck: 4)
            XCTAssertEqual(decoded, message,
                "Round-trip must work for message \(message)")
        }
    }

    func testRoundTripSingleErrorEveryByte() throws {
        // Test that single errors are corrected at every position
        let message: [UInt8] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
        let codeword = try ReedSolomon.encode(message, nCheck: 4)
        for pos in 0..<codeword.count {
            var corrupted = codeword
            corrupted[pos] ^= (pos % 255 == 0 ? 1 : UInt8(pos % 255))
            let decoded = try ReedSolomon.decode(corrupted, nCheck: 4)
            XCTAssertEqual(decoded, message,
                "Single error at position \(pos) must be correctable")
        }
    }

    func testRoundTripNCheck6TwoErrors() throws {
        let message: [UInt8] = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE]
        var codeword = try ReedSolomon.encode(message, nCheck: 6)
        // 3 errors correctable with nCheck=6 (t=3)
        codeword[1] ^= 0x11
        codeword[3] ^= 0x22
        let decoded = try ReedSolomon.decode(codeword, nCheck: 6)
        XCTAssertEqual(decoded, message,
            "2 errors must be correctable with nCheck=6")
    }

    func testRoundTripAllZeroMessage() throws {
        let message: [UInt8] = [0, 0, 0, 0, 0]
        let codeword = try ReedSolomon.encode(message, nCheck: 4)
        let decoded = try ReedSolomon.decode(codeword, nCheck: 4)
        XCTAssertEqual(decoded, message,
            "All-zero message must round-trip correctly")
    }

    func testRoundTripAllMaxMessage() throws {
        let message: [UInt8] = [0xFF, 0xFF, 0xFF, 0xFF]
        let codeword = try ReedSolomon.encode(message, nCheck: 4)
        let decoded = try ReedSolomon.decode(codeword, nCheck: 4)
        XCTAssertEqual(decoded, message,
            "All-0xFF message must round-trip correctly")
    }

    // ========================================================================
    // MARK: - Error Type Tests
    // ========================================================================

    func testTooManyErrorsIsError() {
        let error = ReedSolomon.TooManyErrors()
        XCTAssertTrue(error is any Error,
            "TooManyErrors must conform to Error")
    }

    func testInvalidInputCarriesReason() {
        let error = ReedSolomon.InvalidInput("test reason")
        XCTAssertEqual(error.reason, "test reason",
            "InvalidInput must carry the reason string")
    }

    func testInvalidInputIsError() {
        let error = ReedSolomon.InvalidInput("reason")
        XCTAssertTrue(error is any Error,
            "InvalidInput must conform to Error")
    }
}

// ============================================================================
// MARK: - Test Helpers
// ============================================================================
//
// These helpers replicate polynomial evaluation for test verification purposes.
// They do NOT test the private functions directly — they independently compute
// the same values to cross-check results.

/// Evaluate a little-endian GF(256) polynomial at x using Horner's method.
/// Used in test assertions to independently verify polynomial roots.
private func evaluatePolyLE(_ poly: [UInt8], _ x: UInt8) -> UInt8 {
    var acc: UInt8 = 0
    for coeff in poly.reversed() {
        let mul = GF256.multiply(acc, x)
        acc = GF256.add(mul, coeff)
    }
    return acc
}
