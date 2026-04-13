// ============================================================================
// X25519Tests.swift — Test suite for X25519 (RFC 7748)
// ============================================================================
//
// These tests verify the X25519 implementation against the official test
// vectors from RFC 7748 Section 6.1, plus the iterated test and the
// full Diffie-Hellman key exchange test.
// ============================================================================

import XCTest
@testable import X25519

final class X25519Tests: XCTestCase {

    // -----------------------------------------------------------------------
    // Helper: convert hex string to [UInt8]
    // -----------------------------------------------------------------------

    func hexToBytes(_ hex: String) -> [UInt8] {
        var bytes = [UInt8]()
        var index = hex.startIndex
        while index < hex.endIndex {
            let nextIndex = hex.index(index, offsetBy: 2)
            let byteString = hex[index..<nextIndex]
            bytes.append(UInt8(byteString, radix: 16)!)
            index = nextIndex
        }
        return bytes
    }

    // -----------------------------------------------------------------------
    // Helper: convert [UInt8] to hex string
    // -----------------------------------------------------------------------

    func bytesToHex(_ bytes: [UInt8]) -> String {
        return bytes.map { String(format: "%02x", $0) }.joined()
    }

    // ===================================================================
    // RFC 7748 Section 6.1 — Test Vectors
    // ===================================================================

    // Test Vector 1 — generic scalar multiplication
    func testRFC7748Vector1() throws {
        let scalar = hexToBytes("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4")
        let u = hexToBytes("e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c")
        let expected = "c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552"

        let result = try X25519.x25519(scalar: scalar, u: u)
        XCTAssertEqual(bytesToHex(result), expected)
    }

    // Test Vector 2 — generic scalar multiplication
    func testRFC7748Vector2() throws {
        let scalar = hexToBytes("4b66e9d4d1b4673c5ad22691957d6af5c11b6421e0ea01d42ca4169e7918ba0d")
        let u = hexToBytes("e5210f12786811d3f4b7959d0538ae2c31dbe7106fc03c3efc4cd549c715a493")
        let expected = "95cbde9476e8907d7aade45cb4b873f88b595a68799fa152e6f8f7647aac7957"

        let result = try X25519.x25519(scalar: scalar, u: u)
        XCTAssertEqual(bytesToHex(result), expected)
    }

    // Alice's public key from base point
    func testAlicePublicKey() throws {
        let alicePrivate = hexToBytes("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a")
        let expected = "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a"

        let result = try X25519.x25519Base(scalar: alicePrivate)
        XCTAssertEqual(bytesToHex(result), expected)
    }

    // Bob's public key from base point
    func testBobPublicKey() throws {
        let bobPrivate = hexToBytes("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb")
        let expected = "de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f"

        let result = try X25519.x25519Base(scalar: bobPrivate)
        XCTAssertEqual(bytesToHex(result), expected)
    }

    // Diffie-Hellman shared secret
    func testSharedSecret() throws {
        let alicePrivate = hexToBytes("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a")
        let bobPrivate = hexToBytes("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb")

        let alicePublic = try X25519.x25519Base(scalar: alicePrivate)
        let bobPublic = try X25519.x25519Base(scalar: bobPrivate)

        let aliceShared = try X25519.x25519(scalar: alicePrivate, u: bobPublic)
        let bobShared = try X25519.x25519(scalar: bobPrivate, u: alicePublic)

        let expected = "4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742"

        XCTAssertEqual(bytesToHex(aliceShared), expected)
        XCTAssertEqual(bytesToHex(bobShared), expected)
        XCTAssertEqual(bytesToHex(aliceShared), bytesToHex(bobShared))
    }

    // generateKeypair alias
    func testGenerateKeypair() throws {
        let privateKey = hexToBytes("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a")
        let fromBase = try X25519.x25519Base(scalar: privateKey)
        let fromKeypair = try X25519.generateKeypair(privateKey: privateKey)
        XCTAssertEqual(bytesToHex(fromBase), bytesToHex(fromKeypair))
    }

    // Iterated test: 1 iteration
    func testIterated1() throws {
        var k: [UInt8] = [UInt8](repeating: 0, count: 32)
        k[0] = 9
        var u: [UInt8] = [UInt8](repeating: 0, count: 32)
        u[0] = 9

        let oldK = k
        k = try X25519.x25519(scalar: k, u: u)
        u = oldK

        XCTAssertEqual(bytesToHex(k),
            "422c8e7a6227d7bca1350b3e2bb7279f7897b87bb6854b783c60e80311ae3079")
    }

    // Iterated test: 1000 iterations
    func testIterated1000() throws {
        var k: [UInt8] = [UInt8](repeating: 0, count: 32)
        k[0] = 9
        var u: [UInt8] = [UInt8](repeating: 0, count: 32)
        u[0] = 9

        for _ in 0..<1000 {
            let oldK = k
            k = try X25519.x25519(scalar: k, u: u)
            u = oldK
        }

        XCTAssertEqual(bytesToHex(k),
            "684cf59ba83309552800ef566f2f4d3c1c3887c49360e3875f2eb94d99532c51")
    }

    // Input validation: wrong scalar length
    func testInvalidScalarLength() {
        XCTAssertThrowsError(try X25519.x25519(scalar: [UInt8](repeating: 0, count: 16),
                                                  u: [UInt8](repeating: 0, count: 32)))
    }

    // Input validation: wrong u-coordinate length
    func testInvalidULength() {
        XCTAssertThrowsError(try X25519.x25519(scalar: [UInt8](repeating: 0, count: 32),
                                                  u: [UInt8](repeating: 0, count: 16)))
    }

    // Edge case: u = 0 (low-order point, should throw)
    func testLowOrderPoint() {
        let scalar = hexToBytes("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4")
        let u = [UInt8](repeating: 0, count: 32)

        XCTAssertThrowsError(try X25519.x25519(scalar: scalar, u: u))
    }

    // Edge case: u = 1 is a low-order point (should throw)
    func testUEquals1LowOrder() {
        let scalar = hexToBytes("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4")
        var u = [UInt8](repeating: 0, count: 32)
        u[0] = 1

        XCTAssertThrowsError(try X25519.x25519(scalar: scalar, u: u))
    }
}
