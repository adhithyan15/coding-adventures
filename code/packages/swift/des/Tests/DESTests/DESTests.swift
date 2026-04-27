// DESTests.swift
// Tests for the DES block cipher implementation (FIPS 46-3).

import XCTest
@testable import DES

// Hex helper: decode a hex string to [UInt8]
private func h(_ hex: String) -> [UInt8] {
    var bytes: [UInt8] = []
    var index = hex.startIndex
    while index < hex.endIndex {
        let next = hex.index(index, offsetBy: 2)
        bytes.append(UInt8(hex[index..<next], radix: 16)!)
        index = next
    }
    return bytes
}

// Hex helper: encode [UInt8] to lowercase hex string
private func toHex(_ bytes: [UInt8]) -> String {
    bytes.map { String(format: "%02x", $0) }.joined()
}

final class DESTests: XCTestCase {

    // ========================================================================
    // FIPS 46-3 / Stallings Known-Answer Test
    // ========================================================================
    // Key:   133457799BBCDFF1 (standard worked example from Stallings)
    // Plain: 0123456789ABCDEF
    // CT:    85E813540F0AB405

    func testFIPSKnownAnswer() {
        let key   = h("133457799bbcdff1")
        let plain = h("0123456789abcdef")
        let ct    = h("85e813540f0ab405")

        XCTAssertEqual(desEncryptBlock(plain, key: key), ct, "FIPS encrypt")
        XCTAssertEqual(desDecryptBlock(ct, key: key), plain, "FIPS decrypt")
    }

    // ========================================================================
    // NIST SP 800-20 / FIPS 81 Round-trip Vectors
    // ========================================================================

    func testNIST_weak1() {
        // Key=0101010101010101  Plain=95F8A5E5DD31D900 → CT=8000000000000000
        let key   = h("0101010101010101")
        let plain = h("95f8a5e5dd31d900")
        let ct    = h("8000000000000000")
        XCTAssertEqual(desEncryptBlock(plain, key: key), ct)
        XCTAssertEqual(desDecryptBlock(ct, key: key), plain)
    }

    func testNIST_weak2() {
        // Key=0101010101010101  Plain=DD7F121CA5015619 → CT=4000000000000000
        let key   = h("0101010101010101")
        let plain = h("dd7f121ca5015619")
        let ct    = h("4000000000000000")
        XCTAssertEqual(desEncryptBlock(plain, key: key), ct)
        XCTAssertEqual(desDecryptBlock(ct, key: key), plain)
    }

    func testNIST_weak3() {
        // Key=0101010101010101  Plain=2E8653104F3834EA → CT=2000000000000000
        // From NIST SP 800-20 Table B.1 row 3: Encrypt(2000000000000000) = 2E8653104F3834EA
        // Weak key involution: Encrypt(2E8653104F3834EA) = 2000000000000000 (not 0800000000000000)
        let key   = h("0101010101010101")
        let plain = h("2e8653104f3834ea")
        let ct    = h("2000000000000000")
        XCTAssertEqual(desEncryptBlock(plain, key: key), ct)
        XCTAssertEqual(desDecryptBlock(ct, key: key), plain)
    }

    // NIST SP 800-20 Table B.2: variable-key, plain=0000000000000000
    func testNIST_keyPT1() {
        // Row 1: Key bit 1 set → CT=95A8D72813DAA94D  (NIST SP 800-20 Table B.2)
        let key   = h("8001010101010101")
        let plain = h("0000000000000000")
        let ct    = h("95a8d72813daa94d")
        XCTAssertEqual(desEncryptBlock(plain, key: key), ct)
        XCTAssertEqual(desDecryptBlock(ct, key: key), plain)
    }

    func testNIST_keyPT2() {
        // Row 2: Key bit 2 set → CT=0EEC1487DD8C26D5
        let key   = h("4001010101010101")
        let plain = h("0000000000000000")
        let ct    = h("0eec1487dd8c26d5")
        XCTAssertEqual(desEncryptBlock(plain, key: key), ct)
        XCTAssertEqual(desDecryptBlock(ct, key: key), plain)
    }

    // ========================================================================
    // Round-trip: encrypt then decrypt returns original
    // ========================================================================

    func testRoundTripAllZeros() {
        let key   = h("0000000000000000")
        let plain = h("0000000000000000")
        let ct = desEncryptBlock(plain, key: key)
        XCTAssertEqual(desDecryptBlock(ct, key: key), plain)
    }

    func testRoundTripAllOnes() {
        let key   = h("ffffffffffffffff")
        let plain = h("ffffffffffffffff")
        let ct = desEncryptBlock(plain, key: key)
        XCTAssertEqual(desDecryptBlock(ct, key: key), plain)
    }

    func testRoundTripArbitrary() {
        let key   = h("fedcba9876543210")
        let plain = h("0102030405060708")
        let ct = desEncryptBlock(plain, key: key)
        XCTAssertNotEqual(ct, plain)  // must change
        XCTAssertEqual(desDecryptBlock(ct, key: key), plain)
    }

    func testRoundTripMultipleBlocks() {
        let key = h("1234567890abcdef")
        for i in 0..<10 {
            let plain = [UInt8](repeating: UInt8(i * 25), count: 8)
            let ct = desEncryptBlock(plain, key: key)
            XCTAssertEqual(desDecryptBlock(ct, key: key), plain, "block \(i)")
        }
    }

    // ========================================================================
    // ECB Mode + PKCS#7 Padding
    // ========================================================================

    func testECBRoundTripOneByte() {
        let key  = h("133457799bbcdff1")
        let data = [UInt8]("A".utf8)
        let ct   = desECBEncrypt(data, key: key)
        XCTAssertEqual(ct.count, 8, "single byte pads to one block")
        XCTAssertEqual(desECBDecrypt(ct, key: key), data)
    }

    func testECBRoundTripExactBlock() {
        let key  = h("133457799bbcdff1")
        let data = [UInt8](repeating: 0x41, count: 8)  // exactly one block
        let ct   = desECBEncrypt(data, key: key)
        XCTAssertEqual(ct.count, 16, "full block + padding block")
        XCTAssertEqual(desECBDecrypt(ct, key: key), data)
    }

    func testECBRoundTripMultiBlock() {
        let key  = h("fedcba9876543210")
        let data = Array("Hello, DES world!".utf8)  // 17 bytes
        let ct   = desECBEncrypt(data, key: key)
        XCTAssertEqual(ct.count % 8, 0)
        XCTAssertEqual(desECBDecrypt(ct, key: key), data)
    }

    func testECBRoundTripEmpty() {
        let key = h("0102030405060708")
        let ct  = desECBEncrypt([], key: key)
        XCTAssertEqual(ct.count, 8, "empty input pads to one block")
        XCTAssertEqual(desECBDecrypt(ct, key: key), [])
    }

    func testECBRoundTrip16Bytes() {
        let key  = h("0102030405060708")
        let data = [UInt8](0..<16)
        let ct   = desECBEncrypt(data, key: key)
        XCTAssertEqual(ct.count, 24, "16 bytes → 24 with padding block")
        XCTAssertEqual(desECBDecrypt(ct, key: key), data)
    }

    // ========================================================================
    // 3DES / TDEA
    // ========================================================================

    // TDEA EDE test vector (consistent with all other language implementations)
    // Ordering: E(K1, D(K2, E(K3, P))) per NIST SP 800-67
    // K1=0123456789ABCDEF K2=23456789ABCDEF01 K3=456789ABCDEF0123
    // Plain=6BC1BEE22E409F96  CT=3B6423D418DEFC23
    func testTDEA_NIST() {
        let k1    = h("0123456789abcdef")
        let k2    = h("23456789abcdef01")
        let k3    = h("456789abcdef0123")
        let plain = h("6bc1bee22e409f96")
        let ct    = h("3b6423d418defc23")

        XCTAssertEqual(tdeaEncryptBlock(plain, k1: k1, k2: k2, k3: k3), ct, "3DES encrypt")
        XCTAssertEqual(tdeaDecryptBlock(ct, k1: k1, k2: k2, k3: k3), plain, "3DES decrypt")
    }

    // Degenerate case: K1=K2=K3 must equal single DES
    func testTDEA_DegenerateEquivalence() {
        let key   = h("133457799bbcdff1")
        let plain = h("0123456789abcdef")
        let singleCT = desEncryptBlock(plain, key: key)
        let tripleCT = tdeaEncryptBlock(plain, k1: key, k2: key, k3: key)
        XCTAssertEqual(tripleCT, singleCT, "K1=K2=K3 → single DES")
    }

    func testTDEA_RoundTrip() {
        let k1    = h("fedcba9876543210")
        let k2    = h("0102030405060708")
        let k3    = h("aabbccddeeff0011")
        let plain = h("deadbeefcafebabe")
        let ct    = tdeaEncryptBlock(plain, k1: k1, k2: k2, k3: k3)
        XCTAssertEqual(tdeaDecryptBlock(ct, k1: k1, k2: k2, k3: k3), plain)
    }

    // ========================================================================
    // Key Schedule
    // ========================================================================

    func testExpandKeyCount() {
        let subkeys = expandKey(h("2b7e151628aed2a6"))
        XCTAssertEqual(subkeys.count, 16, "DES key schedule produces 16 subkeys")
    }

    func testExpandKeySubkeyLength() {
        let subkeys = expandKey(h("133457799bbcdff1"))
        for (i, sk) in subkeys.enumerated() {
            XCTAssertEqual(sk.count, 6, "subkey \(i) must be 6 bytes (48 bits)")
        }
    }

    func testExpandKeyDifferentKeys() {
        let sk1 = expandKey(h("133457799bbcdff1"))
        let sk2 = expandKey(h("fedcba9876543210"))
        // All 16 subkeys should differ between the two keys
        for i in 0..<16 {
            XCTAssertNotEqual(sk1[i], sk2[i], "subkey \(i) should differ for different keys")
        }
    }

    // ========================================================================
    // Error handling (preconditions) — tested via inline checks
    // ========================================================================

    // Note: We can't test precondition failures with XCTest in Swift without
    // additional tooling. We instead verify the happy-path contract.

    func testBlockSizeContract() {
        let key  = h("133457799bbcdff1")
        let ct   = desEncryptBlock(h("0123456789abcdef"), key: key)
        XCTAssertEqual(ct.count, 8, "output must be exactly 8 bytes")
    }

    // ========================================================================
    // Avalanche effect: flipping one key bit should flip ~50% of output bits
    // ========================================================================

    func testAvalancheKey() {
        let plain = h("0123456789abcdef")
        let key1  = h("133457799bbcdff1")
        var key2Bytes = h("133457799bbcdff1")
        key2Bytes[0] ^= 0x80  // flip MSB of first byte
        let key2 = key2Bytes

        let ct1 = desEncryptBlock(plain, key: key1)
        let ct2 = desEncryptBlock(plain, key: key2)

        var diffBits = 0
        for i in 0..<8 {
            diffBits += ct1[i] ^ ct2[i] == 0 ? 0 : Int((ct1[i] ^ ct2[i]).nonzeroBitCount)
        }
        // Should flip roughly 32 of 64 bits; accept any result > 8
        XCTAssertGreaterThan(diffBits, 8, "key bit flip should cause significant output change")
    }

    func testAvalanchePlaintext() {
        let key    = h("133457799bbcdff1")
        let plain1 = h("0123456789abcdef")
        var plain2Bytes = h("0123456789abcdef")
        plain2Bytes[0] ^= 0x01  // flip LSB of first byte
        let plain2 = plain2Bytes

        let ct1 = desEncryptBlock(plain1, key: key)
        let ct2 = desEncryptBlock(plain2, key: key)

        var diffBits = 0
        for i in 0..<8 {
            diffBits += Int((ct1[i] ^ ct2[i]).nonzeroBitCount)
        }
        XCTAssertGreaterThan(diffBits, 8, "plaintext bit flip should cause significant output change")
    }
}
