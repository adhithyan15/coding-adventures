// AESModesTests.swift
//
// Tests for AES modes of operation using NIST test vectors from:
//   - SP 800-38A (ECB, CBC, CTR)
//   - GCM specification (GCM)

import Testing
@testable import AESModes

// ─────────────────────────────────────────────────────────────────────────────
// Shared test data — NIST SP 800-38A
// ─────────────────────────────────────────────────────────────────────────────

private let nistKey = fromHex("2b7e151628aed2a6abf7158809cf4f3c")
private let nistPtBlock1 = fromHex("6bc1bee22e409f96e93d7e117393172a")
private let nistPtAll = fromHex(
    "6bc1bee22e409f96e93d7e117393172a" +
    "ae2d8a571e03ac9c9eb76fac45af8e51" +
    "30c81c46a35ce411e5fbc1191a0a52ef" +
    "f69f2445df4f9b17ad2b417be66c3710"
)

// ─────────────────────────────────────────────────────────────────────────────
// PKCS#7 Padding
// ─────────────────────────────────────────────────────────────────────────────

@Test func pkcs7PadAligned() throws {
    let input = [UInt8](repeating: 0xAA, count: 16)
    let padded = AESModes.pkcs7Pad(input)
    #expect(padded.count == 32)
    for i in 16..<32 { #expect(padded[i] == 16) }
}

@Test func pkcs7Pad13Bytes() throws {
    let input = [UInt8](repeating: 0xBB, count: 13)
    let padded = AESModes.pkcs7Pad(input)
    #expect(padded.count == 16)
    #expect(padded[13] == 3)
    #expect(padded[14] == 3)
    #expect(padded[15] == 3)
}

@Test func pkcs7Roundtrip() throws {
    let input: [UInt8] = [1, 2, 3, 4, 5]
    let result = try AESModes.pkcs7Unpad(AESModes.pkcs7Pad(input))
    #expect(result == input)
}

@Test func pkcs7RejectsInvalidValue() throws {
    var bad = [UInt8](repeating: 0, count: 16)
    bad[15] = 0
    #expect(throws: AESModesError.self) { try AESModes.pkcs7Unpad(bad) }
}

@Test func pkcs7RejectsInconsistent() throws {
    var bad = [UInt8](repeating: 0, count: 16)
    bad[15] = 2
    bad[14] = 3
    #expect(throws: AESModesError.self) { try AESModes.pkcs7Unpad(bad) }
}

// ─────────────────────────────────────────────────────────────────────────────
// ECB Mode
// ─────────────────────────────────────────────────────────────────────────────

@Test func ecbNistBlock1() throws {
    let ct = AESModes.ecbEncrypt(nistPtBlock1, key: nistKey)
    #expect(toHex(Array(ct[0..<16])) == "3ad77bb40d7a3660a89ecaf32466ef97")
}

@Test func ecbNistAllBlocks() throws {
    let ct = AESModes.ecbEncrypt(nistPtAll, key: nistKey)
    #expect(ct.count == 80)
    #expect(toHex(Array(ct[0..<16])) == "3ad77bb40d7a3660a89ecaf32466ef97")
    #expect(toHex(Array(ct[16..<32])) == "f5d3d58503b9699de785895a96fdbaaf")
    #expect(toHex(Array(ct[32..<48])) == "43b1cd7f598ece23881b00e3ed030688")
    #expect(toHex(Array(ct[48..<64])) == "7b0c785e27e8ad3f8223207104725dd4")
}

@Test func ecbRoundtrip() throws {
    let ct = AESModes.ecbEncrypt(nistPtAll, key: nistKey)
    let pt = try AESModes.ecbDecrypt(ct, key: nistKey)
    #expect(pt == nistPtAll)
}

@Test func ecbIdenticalBlocksProduceIdenticalCt() throws {
    var twoBlocks = nistPtBlock1
    twoBlocks.append(contentsOf: nistPtBlock1)
    let ct = AESModes.ecbEncrypt(twoBlocks, key: nistKey)
    #expect(toHex(Array(ct[0..<16])) == toHex(Array(ct[16..<32])))
}

// ─────────────────────────────────────────────────────────────────────────────
// CBC Mode
// ─────────────────────────────────────────────────────────────────────────────

private let cbcIV = fromHex("000102030405060708090a0b0c0d0e0f")

@Test func cbcNistBlock1() throws {
    let ct = try AESModes.cbcEncrypt(nistPtBlock1, key: nistKey, iv: cbcIV)
    #expect(toHex(Array(ct[0..<16])) == "7649abac8119b246cee98e9b12e9197d")
}

@Test func cbcNistAllBlocks() throws {
    let ct = try AESModes.cbcEncrypt(nistPtAll, key: nistKey, iv: cbcIV)
    #expect(ct.count == 80)
    #expect(toHex(Array(ct[0..<16])) == "7649abac8119b246cee98e9b12e9197d")
    #expect(toHex(Array(ct[16..<32])) == "5086cb9b507219ee95db113a917678b2")
    #expect(toHex(Array(ct[32..<48])) == "73bed6b8e3c1743b7116e69e22229516")
    #expect(toHex(Array(ct[48..<64])) == "3ff1caa1681fac09120eca307586e1a7")
}

@Test func cbcRoundtrip() throws {
    let ct = try AESModes.cbcEncrypt(nistPtAll, key: nistKey, iv: cbcIV)
    let pt = try AESModes.cbcDecrypt(ct, key: nistKey, iv: cbcIV)
    #expect(pt == nistPtAll)
}

@Test func cbcRejectsWrongIVLength() throws {
    #expect(throws: AESModesError.self) {
        try AESModes.cbcEncrypt(nistPtBlock1, key: nistKey, iv: [UInt8](repeating: 0, count: 8))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CTR Mode
// ─────────────────────────────────────────────────────────────────────────────

@Test func ctrRoundtrip() throws {
    let nonce = fromHex("f0f1f2f3f4f5f6f7f8f9fafb")
    let ct = try AESModes.ctrEncrypt(nistPtAll, key: nistKey, nonce: nonce)
    let pt = try AESModes.ctrDecrypt(ct, key: nistKey, nonce: nonce)
    #expect(pt == nistPtAll)
}

@Test func ctrNoPadding() throws {
    let nonce = fromHex("000000000000000000000000")
    let pt: [UInt8] = [0xDE, 0xAD]
    let ct = try AESModes.ctrEncrypt(pt, key: nistKey, nonce: nonce)
    #expect(ct.count == 2)
    let result = try AESModes.ctrDecrypt(ct, key: nistKey, nonce: nonce)
    #expect(result == pt)
}

@Test func ctrEmpty() throws {
    let nonce = fromHex("000000000000000000000000")
    let ct = try AESModes.ctrEncrypt([], key: nistKey, nonce: nonce)
    #expect(ct.count == 0)
}

@Test func ctrRejectsWrongNonceLength() throws {
    #expect(throws: AESModesError.self) {
        try AESModes.ctrEncrypt([0], key: nistKey, nonce: [UInt8](repeating: 0, count: 16))
    }
}

@Test func ctrEncryptEqualsDecrypt() throws {
    let nonce = fromHex("aabbccddeeff001122334455")
    let ct = try AESModes.ctrEncrypt(nistPtAll, key: nistKey, nonce: nonce)
    // CTR decrypt is same as encrypt
    let pt = try AESModes.ctrEncrypt(ct, key: nistKey, nonce: nonce)
    #expect(pt == nistPtAll)
}

// ─────────────────────────────────────────────────────────────────────────────
// GCM Mode
// ─────────────────────────────────────────────────────────────────────────────

@Test func gcmNistTestCase() throws {
    let key = fromHex("feffe9928665731c6d6a8f9467308308")
    let iv = fromHex("cafebabefacedbaddecaf888")
    let pt = fromHex(
        "d9313225f88406e5a55909c5aff5269a" +
        "86a7a9531534f7da2e4c303d8a318a72" +
        "1c3c0c95956809532fcf0e2449a6b525" +
        "b16aedf5aa0de657ba637b391aafd255"
    )
    let expectedCt = fromHex(
        "42831ec2217774244b7221b784d0d49c" +
        "e3aa212f2c02a4e035c17e2329aca12e" +
        "21d514b25466931c7d8f6a5aac84aa05" +
        "1ba30b396a0aac973d58e091473f5985"
    )
    let expectedTag = fromHex("4d5c2af327cd64a62cf35abd2ba6fab4")

    let (ct, tag) = try AESModes.gcmEncrypt(pt, key: key, iv: iv)
    #expect(toHex(ct) == toHex(expectedCt))
    #expect(toHex(tag) == toHex(expectedTag))
}

@Test func gcmRoundtripWithAAD() throws {
    let key = fromHex("feffe9928665731c6d6a8f9467308308")
    let iv = fromHex("cafebabefacedbaddecaf888")
    let pt = fromHex("d9313225f88406e5a55909c5aff5269a")
    let aad = fromHex("feedfacedeadbeeffeedfacedeadbeef")

    let (ct, tag) = try AESModes.gcmEncrypt(pt, key: key, iv: iv, aad: aad)
    let result = try AESModes.gcmDecrypt(ct, key: key, iv: iv, aad: aad, tag: tag)
    #expect(result == pt)
}

@Test func gcmRejectsTamperedCiphertext() throws {
    let key = fromHex("feffe9928665731c6d6a8f9467308308")
    let iv = fromHex("cafebabefacedbaddecaf888")
    let pt = fromHex("d9313225f88406e5a55909c5aff5269a")

    var (ct, tag) = try AESModes.gcmEncrypt(pt, key: key, iv: iv)
    ct[0] ^= 0x01

    #expect(throws: AESModesError.self) {
        try AESModes.gcmDecrypt(ct, key: key, iv: iv, aad: [], tag: tag)
    }
}

@Test func gcmRejectsTamperedTag() throws {
    let key = fromHex("feffe9928665731c6d6a8f9467308308")
    let iv = fromHex("cafebabefacedbaddecaf888")
    let pt = fromHex("d9313225f88406e5a55909c5aff5269a")

    let (ct, origTag) = try AESModes.gcmEncrypt(pt, key: key, iv: iv)
    var tag = origTag
    tag[0] ^= 0x01

    #expect(throws: AESModesError.self) {
        try AESModes.gcmDecrypt(ct, key: key, iv: iv, aad: [], tag: tag)
    }
}

@Test func gcmRejectsWrongAAD() throws {
    let key = fromHex("feffe9928665731c6d6a8f9467308308")
    let iv = fromHex("cafebabefacedbaddecaf888")
    let pt = fromHex("d9313225f88406e5a55909c5aff5269a")
    let aad = fromHex("feedfacedeadbeef")

    let (ct, tag) = try AESModes.gcmEncrypt(pt, key: key, iv: iv, aad: aad)
    let wrongAad = fromHex("deadbeeffeedface")

    #expect(throws: AESModesError.self) {
        try AESModes.gcmDecrypt(ct, key: key, iv: iv, aad: wrongAad, tag: tag)
    }
}

@Test func gcmEmptyPtEmptyAAD() throws {
    let key = fromHex("00000000000000000000000000000000")
    let iv = fromHex("000000000000000000000000")

    let (ct, tag) = try AESModes.gcmEncrypt([], key: key, iv: iv)
    #expect(ct.count == 0)
    #expect(toHex(tag) == "58e2fccefa7e3061367f1d57a4e7455a")
}

@Test func gcmEmptyPtWithAAD() throws {
    let key = fromHex("feffe9928665731c6d6a8f9467308308")
    let iv = fromHex("cafebabefacedbaddecaf888")
    let aad = fromHex("feedfacedeadbeef")

    let (ct, tag) = try AESModes.gcmEncrypt([], key: key, iv: iv, aad: aad)
    #expect(ct.count == 0)
    #expect(tag.count == 16)

    let result = try AESModes.gcmDecrypt(ct, key: key, iv: iv, aad: aad, tag: tag)
    #expect(result.count == 0)
}

@Test func gcmRejectsWrongIVLength() throws {
    let key = fromHex("feffe9928665731c6d6a8f9467308308")
    #expect(throws: AESModesError.self) {
        try AESModes.gcmEncrypt([], key: key, iv: [UInt8](repeating: 0, count: 16))
    }
}

@Test func gcmRejectsWrongTagLength() throws {
    let key = fromHex("feffe9928665731c6d6a8f9467308308")
    let iv = fromHex("cafebabefacedbaddecaf888")
    #expect(throws: AESModesError.self) {
        try AESModes.gcmDecrypt([], key: key, iv: iv, aad: [], tag: [UInt8](repeating: 0, count: 8))
    }
}
