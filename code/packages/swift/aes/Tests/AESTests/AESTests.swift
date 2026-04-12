// AESTests.swift
// Tests for the AES block cipher implementation (FIPS 197).

import XCTest
@testable import AES

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

final class AESTests: XCTestCase {

    // ========================================================================
    // FIPS 197 Appendix B — AES-128
    // ========================================================================
    // Key:   2b7e151628aed2a6abf7158809cf4f3c
    // Plain: 3243f6a8885a308d313198a2e0370734
    // CT:    3925841d02dc09fbdc118597196a0b32

    func testFIPS197AppendixB_Encrypt() {
        let key   = h("2b7e151628aed2a6abf7158809cf4f3c")
        let plain = h("3243f6a8885a308d313198a2e0370734")
        let ct    = h("3925841d02dc09fbdc118597196a0b32")
        XCTAssertEqual(aesEncryptBlock(plain, key: key), ct)
    }

    func testFIPS197AppendixB_Decrypt() {
        let key   = h("2b7e151628aed2a6abf7158809cf4f3c")
        let plain = h("3243f6a8885a308d313198a2e0370734")
        let ct    = h("3925841d02dc09fbdc118597196a0b32")
        XCTAssertEqual(aesDecryptBlock(ct, key: key), plain)
    }

    // ========================================================================
    // FIPS 197 Appendix C.1 — AES-128 sequential key
    // ========================================================================
    // Key:   000102030405060708090a0b0c0d0e0f
    // Plain: 00112233445566778899aabbccddeeff
    // CT:    69c4e0d86a7b0430d8cdb78070b4c55a

    func testFIPS197_C1_Encrypt() {
        let key   = h("000102030405060708090a0b0c0d0e0f")
        let plain = h("00112233445566778899aabbccddeeff")
        let ct    = h("69c4e0d86a7b0430d8cdb78070b4c55a")
        XCTAssertEqual(aesEncryptBlock(plain, key: key), ct)
    }

    func testFIPS197_C1_Decrypt() {
        let key   = h("000102030405060708090a0b0c0d0e0f")
        let plain = h("00112233445566778899aabbccddeeff")
        let ct    = h("69c4e0d86a7b0430d8cdb78070b4c55a")
        XCTAssertEqual(aesDecryptBlock(ct, key: key), plain)
    }

    // ========================================================================
    // FIPS 197 Appendix C.2 — AES-192
    // ========================================================================
    // Key:   000102030405060708090a0b0c0d0e0f1011121314151617
    // Plain: 00112233445566778899aabbccddeeff
    // CT:    dda97ca4864cdfe06eaf70a0ec0d7191

    func testFIPS197_C2_Encrypt() {
        let key   = h("000102030405060708090a0b0c0d0e0f1011121314151617")
        let plain = h("00112233445566778899aabbccddeeff")
        let ct    = h("dda97ca4864cdfe06eaf70a0ec0d7191")
        XCTAssertEqual(aesEncryptBlock(plain, key: key), ct)
    }

    func testFIPS197_C2_Decrypt() {
        let key   = h("000102030405060708090a0b0c0d0e0f1011121314151617")
        let plain = h("00112233445566778899aabbccddeeff")
        let ct    = h("dda97ca4864cdfe06eaf70a0ec0d7191")
        XCTAssertEqual(aesDecryptBlock(ct, key: key), plain)
    }

    // ========================================================================
    // FIPS 197 Appendix C.3 — AES-256
    // ========================================================================
    // Key:   000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f
    // Plain: 00112233445566778899aabbccddeeff
    // CT:    8ea2b7ca516745bfeafc49904b496089

    func testFIPS197_C3_Encrypt() {
        let key   = h("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
        let plain = h("00112233445566778899aabbccddeeff")
        let ct    = h("8ea2b7ca516745bfeafc49904b496089")
        XCTAssertEqual(aesEncryptBlock(plain, key: key), ct)
    }

    func testFIPS197_C3_Decrypt() {
        let key   = h("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
        let plain = h("00112233445566778899aabbccddeeff")
        let ct    = h("8ea2b7ca516745bfeafc49904b496089")
        XCTAssertEqual(aesDecryptBlock(ct, key: key), plain)
    }

    // ========================================================================
    // AES-256 SE01 spec vector
    // ========================================================================
    // Key:   603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4
    // Plain: 6bc1bee22e409f96e93d7e117393172a
    // CT:    f3eed1bdb5d2a03c064b5a7e3db181f8

    func testAES256_SE01_Encrypt() {
        let key   = h("603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4")
        let plain = h("6bc1bee22e409f96e93d7e117393172a")
        let ct    = h("f3eed1bdb5d2a03c064b5a7e3db181f8")
        XCTAssertEqual(aesEncryptBlock(plain, key: key), ct)
    }

    func testAES256_SE01_Decrypt() {
        let key   = h("603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4")
        let plain = h("6bc1bee22e409f96e93d7e117393172a")
        let ct    = h("f3eed1bdb5d2a03c064b5a7e3db181f8")
        XCTAssertEqual(aesDecryptBlock(ct, key: key), plain)
    }

    // ========================================================================
    // S-box Properties
    // ========================================================================

    func testSBoxHas256Elements() {
        XCTAssertEqual(sbox.count, 256)
    }

    func testSBoxIsBijection() {
        var seen = Set<UInt8>()
        for b in sbox { seen.insert(b) }
        XCTAssertEqual(seen.count, 256, "SBOX must be a bijection (all 256 outputs distinct)")
    }

    func testInvSBoxInverseOfSBox() {
        for b in 0..<256 {
            XCTAssertEqual(Int(invSbox[Int(sbox[b])]), b, "INV_SBOX[SBOX[\(b)]] must equal \(b)")
        }
    }

    func testSBoxKnownValues_FIPS197Figure7() {
        // FIPS 197 Figure 7: specific known S-box values
        XCTAssertEqual(sbox[0x00], 0x63)
        XCTAssertEqual(sbox[0x01], 0x7c)
        XCTAssertEqual(sbox[0xff], 0x16)
        XCTAssertEqual(sbox[0x53], 0xed)
    }

    func testSBoxHasNoFixedPoints() {
        for b in 0..<256 {
            XCTAssertNotEqual(sbox[b], UInt8(b), "SBOX[\(b)] must not equal \(b)")
        }
    }

    // ========================================================================
    // Key Schedule
    // ========================================================================

    func testExpandKey128RoundCount() {
        let rks = expandKey(h("2b7e151628aed2a6abf7158809cf4f3c"))
        XCTAssertEqual(rks.count, 11, "AES-128 produces 11 round keys")
    }

    func testExpandKey192RoundCount() {
        let rks = expandKey(h("000102030405060708090a0b0c0d0e0f1011121314151617"))
        XCTAssertEqual(rks.count, 13, "AES-192 produces 13 round keys")
    }

    func testExpandKey256RoundCount() {
        let rks = expandKey(h("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"))
        XCTAssertEqual(rks.count, 15, "AES-256 produces 15 round keys")
    }

    func testExpandKeyRoundKeyLength() {
        let rks = expandKey(h("2b7e151628aed2a6abf7158809cf4f3c"))
        for (i, rk) in rks.enumerated() {
            XCTAssertEqual(rk.count, 16, "round key \(i) must be 16 bytes")
        }
    }

    // ========================================================================
    // Round-trip tests
    // ========================================================================

    func testRoundTrip_AES128() {
        let key = h("fedcba9876543210fedcba9876543210")
        let starts = [0, 32, 64, 128, 192, 224]
        for start in starts {
            let plain = (0..<16).map { UInt8((start + $0) % 256) }
            let ct = aesEncryptBlock(plain, key: key)
            XCTAssertEqual(aesDecryptBlock(ct, key: key), plain, "round-trip start=\(start)")
        }
    }

    func testRoundTrip_AES192() {
        let key   = h("000102030405060708090a0b0c0d0e0f1011121314151617")
        let plain = h("deadbeefcafebabe0123456789abcdef")
        let ct    = aesEncryptBlock(plain, key: key)
        XCTAssertEqual(aesDecryptBlock(ct, key: key), plain)
    }

    func testRoundTrip_AES256_AllZeros() {
        let key   = h("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
        let plain = h("00000000000000000000000000000000")
        let ct    = aesEncryptBlock(plain, key: key)
        XCTAssertEqual(aesDecryptBlock(ct, key: key), plain)
    }

    func testEncryptChangesBlock() {
        let key   = h("2b7e151628aed2a6abf7158809cf4f3c")
        let plain = h("3243f6a8885a308d313198a2e0370734")
        let ct    = aesEncryptBlock(plain, key: key)
        XCTAssertNotEqual(ct, plain, "ciphertext must differ from plaintext")
    }
}
