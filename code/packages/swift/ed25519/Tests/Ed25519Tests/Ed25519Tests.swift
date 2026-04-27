// Ed25519Tests.swift
// Tests against RFC 8032 Section 7.1 test vectors.

import XCTest
import Foundation
@testable import Ed25519

final class Ed25519Tests: XCTestCase {

    // ── RFC 8032 Test Vector 1: Empty message ──

    func testVector1EmptyMessage() {
        let seed = hexToData("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
        let expectedPub = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
        let expectedSig = "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"

        let kp = generateKeypair(seed: seed)
        XCTAssertEqual(dataToHex(kp.publicKey), expectedPub)

        let message = Data()
        let sig = ed25519Sign(message: message, secretKey: kp.secretKey)
        XCTAssertEqual(dataToHex(sig), expectedSig)

        XCTAssertTrue(ed25519Verify(message: message, signature: sig, publicKey: kp.publicKey))
    }

    // ── RFC 8032 Test Vector 2: One byte (0x72) ──

    func testVector2OneByte() {
        let seed = hexToData("4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb")
        let expectedPub = "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c"
        let expectedSig = "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00"

        let kp = generateKeypair(seed: seed)
        XCTAssertEqual(dataToHex(kp.publicKey), expectedPub)

        let message = hexToData("72")
        let sig = ed25519Sign(message: message, secretKey: kp.secretKey)
        XCTAssertEqual(dataToHex(sig), expectedSig)

        XCTAssertTrue(ed25519Verify(message: message, signature: sig, publicKey: kp.publicKey))
    }

    // ── RFC 8032 Test Vector 3: Two bytes (0xaf82) ──

    func testVector3TwoBytes() {
        let seed = hexToData("c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7")
        let expectedPub = "fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025"
        let expectedSig = "6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a"

        let kp = generateKeypair(seed: seed)
        XCTAssertEqual(dataToHex(kp.publicKey), expectedPub)

        let message = hexToData("af82")
        let sig = ed25519Sign(message: message, secretKey: kp.secretKey)
        XCTAssertEqual(dataToHex(sig), expectedSig)

        XCTAssertTrue(ed25519Verify(message: message, signature: sig, publicKey: kp.publicKey))
    }

    // ── Verification Failures ──

    func testRejectsTamperedMessage() {
        let seed = hexToData("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
        let kp = generateKeypair(seed: seed)
        let message = Data("Hello".utf8)
        let sig = ed25519Sign(message: message, secretKey: kp.secretKey)

        let tampered = Data("Hello!".utf8)
        XCTAssertFalse(ed25519Verify(message: tampered, signature: sig, publicKey: kp.publicKey))
    }

    func testRejectsWrongPublicKey() {
        let seed1 = hexToData("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
        let seed2 = hexToData("4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb")
        let kp1 = generateKeypair(seed: seed1)
        let kp2 = generateKeypair(seed: seed2)

        let message = Data("Hello".utf8)
        let sig = ed25519Sign(message: message, secretKey: kp1.secretKey)

        XCTAssertFalse(ed25519Verify(message: message, signature: sig, publicKey: kp2.publicKey))
    }

    func testRejectsTamperedSignatureR() {
        let seed = hexToData("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
        let kp = generateKeypair(seed: seed)
        let message = Data("Hello".utf8)
        var sig = ed25519Sign(message: message, secretKey: kp.secretKey)

        sig[0] ^= 1
        XCTAssertFalse(ed25519Verify(message: message, signature: sig, publicKey: kp.publicKey))
    }

    func testRejectsTamperedSignatureS() {
        let seed = hexToData("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
        let kp = generateKeypair(seed: seed)
        let message = Data("Hello".utf8)
        var sig = ed25519Sign(message: message, secretKey: kp.secretKey)

        sig[32] ^= 1
        XCTAssertFalse(ed25519Verify(message: message, signature: sig, publicKey: kp.publicKey))
    }

    func testRejectsWrongLengthSignature() {
        let seed = hexToData("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
        let kp = generateKeypair(seed: seed)
        let message = Data("Hello".utf8)

        XCTAssertFalse(ed25519Verify(message: message, signature: Data(count: 63), publicKey: kp.publicKey))
        XCTAssertFalse(ed25519Verify(message: message, signature: Data(count: 65), publicKey: kp.publicKey))
    }

    func testRejectsWrongLengthPublicKey() {
        let seed = hexToData("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
        let kp = generateKeypair(seed: seed)
        let message = Data("Hello".utf8)
        let sig = ed25519Sign(message: message, secretKey: kp.secretKey)

        XCTAssertFalse(ed25519Verify(message: message, signature: sig, publicKey: Data(count: 31)))
        XCTAssertFalse(ed25519Verify(message: message, signature: sig, publicKey: Data(count: 33)))
    }

    // ── Keypair Generation ──

    func testKeySizes() {
        let seed = hexToData("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
        let kp = generateKeypair(seed: seed)
        XCTAssertEqual(kp.publicKey.count, 32)
        XCTAssertEqual(kp.secretKey.count, 64)
    }

    func testSecretKeyStructure() {
        let seed = hexToData("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
        let kp = generateKeypair(seed: seed)
        XCTAssertEqual(Data(kp.secretKey.prefix(32)), seed)
        XCTAssertEqual(Data(kp.secretKey.suffix(32)), kp.publicKey)
    }

    func testDeterministicKeypair() {
        let seed = hexToData("c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7")
        let kp1 = generateKeypair(seed: seed)
        let kp2 = generateKeypair(seed: seed)
        XCTAssertEqual(kp1.publicKey, kp2.publicKey)
        XCTAssertEqual(kp1.secretKey, kp2.secretKey)
    }

    // ── Round-Trip ──

    func testSignVerifyRoundTrip() {
        let seed = hexToData("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
        let kp = generateKeypair(seed: seed)

        for len in [0, 1, 2, 16] {
            var msg = Data(count: len)
            for i in 0..<len { msg[i] = UInt8(i & 0xFF) }
            let sig = ed25519Sign(message: msg, secretKey: kp.secretKey)
            XCTAssertTrue(ed25519Verify(message: msg, signature: sig, publicKey: kp.publicKey),
                          "Failed for message length \(len)")
        }
    }

    func testDeterministicSignature() {
        let seed = hexToData("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
        let kp = generateKeypair(seed: seed)
        let msg = Data([1, 2, 3])
        let sig1 = ed25519Sign(message: msg, secretKey: kp.secretKey)
        let sig2 = ed25519Sign(message: msg, secretKey: kp.secretKey)
        XCTAssertEqual(sig1, sig2)
    }
}
