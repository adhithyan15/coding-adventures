// ============================================================================
// ChaCha20-Poly1305 Tests
// ============================================================================
//
// These tests verify correctness against the official RFC 8439 test vectors.
//
// ============================================================================

import XCTest
@testable import ChaCha20Poly1305

final class ChaCha20Poly1305Tests: XCTestCase {

    // MARK: - Helpers

    /// Convert a hex string to a byte array.
    private func fromHex(_ hex: String) -> [UInt8] {
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

    /// Convert a byte array to a hex string.
    private func toHex(_ bytes: [UInt8]) -> String {
        return bytes.map { String(format: "%02x", $0) }.joined()
    }

    /// Convert a string to UTF-8 bytes.
    private func fromString(_ str: String) -> [UInt8] {
        return Array(str.utf8)
    }

    // MARK: - ChaCha20 Tests

    func testChaCha20RFC8439Section242() {
        // RFC 8439 Section 2.4.2 — the canonical "sunscreen" test vector.
        let key = fromHex(
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
        )
        let nonce = fromHex("000000000000004a00000000")
        let plaintext = fromString(
            "Ladies and Gentlemen of the class of \'99: If I could offer you only one tip for the future, sunscreen would be it."
        )

        let expectedCt = fromHex(
            "6e2e359a2568f98041ba0728dd0d6981"
            + "e97e7aec1d4360c20a27afccfd9fae0b"
            + "f91b65c5524733ab8f593dabcd62b357"
            + "1639d624e65152ab8f530c359f0861d8"
            + "07ca0dbf500d6a6156a38e088a22b65e"
            + "52bc514d16ccf806818ce91ab7793736"
            + "5af90bbf74a35be6b40b8eedf2785e42"
            + "874d"
        )

        let ciphertext = ChaCha20Poly1305.chacha20Encrypt(
            plaintext: plaintext, key: key, nonce: nonce, counter: 1
        )
        XCTAssertEqual(toHex(ciphertext), toHex(expectedCt))

        // Verify round-trip
        let decrypted = ChaCha20Poly1305.chacha20Encrypt(
            plaintext: ciphertext, key: key, nonce: nonce, counter: 1
        )
        XCTAssertEqual(decrypted, plaintext)
    }

    func testChaCha20Empty() {
        let key = [UInt8](repeating: 0, count: 32)
        let nonce = [UInt8](repeating: 0, count: 12)
        let result = ChaCha20Poly1305.chacha20Encrypt(
            plaintext: [], key: key, nonce: nonce, counter: 0
        )
        XCTAssertTrue(result.isEmpty)
    }

    func testChaCha20SingleByte() {
        var key = [UInt8](repeating: 0, count: 32)
        key[0] = 1
        let nonce = [UInt8](repeating: 0, count: 12)
        let ct = ChaCha20Poly1305.chacha20Encrypt(
            plaintext: [0x42], key: key, nonce: nonce, counter: 0
        )
        XCTAssertEqual(ct.count, 1)
        let pt = ChaCha20Poly1305.chacha20Encrypt(
            plaintext: ct, key: key, nonce: nonce, counter: 0
        )
        XCTAssertEqual(pt, [0x42])
    }

    func testChaCha20Multiblock() {
        var key = [UInt8](repeating: 0, count: 32)
        for i in 0..<32 { key[i] = UInt8(i) }
        var nonce = [UInt8](repeating: 0, count: 12)
        nonce[0] = 0x09
        var plaintext = [UInt8](repeating: 0, count: 200)
        for i in 0..<200 { plaintext[i] = UInt8(i % 256) }

        let ct = ChaCha20Poly1305.chacha20Encrypt(
            plaintext: plaintext, key: key, nonce: nonce, counter: 0
        )
        XCTAssertEqual(ct.count, 200)
        let pt = ChaCha20Poly1305.chacha20Encrypt(
            plaintext: ct, key: key, nonce: nonce, counter: 0
        )
        XCTAssertEqual(pt, plaintext)
    }

    func testChaCha20DifferentKeys() {
        var key1 = [UInt8](repeating: 0, count: 32)
        key1[0] = 1
        var key2 = [UInt8](repeating: 0, count: 32)
        key2[0] = 2
        let nonce = [UInt8](repeating: 0, count: 12)
        let plaintext = fromString("Hello, World!")
        let ct1 = ChaCha20Poly1305.chacha20Encrypt(
            plaintext: plaintext, key: key1, nonce: nonce, counter: 0
        )
        let ct2 = ChaCha20Poly1305.chacha20Encrypt(
            plaintext: plaintext, key: key2, nonce: nonce, counter: 0
        )
        XCTAssertNotEqual(ct1, ct2)
    }

    // MARK: - Poly1305 Tests

    func testPoly1305RFC8439Section252() {
        let key = fromHex(
            "85d6be7857556d337f4452fe42d506a80103808afb0db2fd4abff6af4149f51b"
        )
        let message = fromString("Cryptographic Forum Research Group")
        let expectedTag = fromHex("a8061dc1305136c6c22b8baf0c0127a9")

        let tag = ChaCha20Poly1305.poly1305Mac(message: message, key: key)
        XCTAssertEqual(toHex(tag), toHex(expectedTag))
    }

    func testPoly1305Empty() {
        var key = [UInt8](repeating: 0, count: 32)
        for i in 0..<32 { key[i] = UInt8(i) }
        let tag = ChaCha20Poly1305.poly1305Mac(message: [], key: key)
        XCTAssertEqual(tag.count, 16)
    }

    func testPoly1305SingleByte() {
        var key = [UInt8](repeating: 0, count: 32)
        for i in 0..<32 { key[i] = UInt8(i) }
        let tag = ChaCha20Poly1305.poly1305Mac(message: [0x42], key: key)
        XCTAssertEqual(tag.count, 16)
    }

    func testPoly1305DifferentMessages() {
        let key = fromHex(
            "85d6be7857556d337f4452fe42d506a80103808afb0db2fd4abff6af4149f51b"
        )
        let tag1 = ChaCha20Poly1305.poly1305Mac(
            message: fromString("Message A"), key: key
        )
        let tag2 = ChaCha20Poly1305.poly1305Mac(
            message: fromString("Message B"), key: key
        )
        XCTAssertNotEqual(tag1, tag2)
    }

    func testPoly1305Exactly16Bytes() {
        var key = [UInt8](repeating: 0, count: 32)
        for i in 0..<32 { key[i] = UInt8(i) }
        var msg = [UInt8](repeating: 0, count: 16)
        for i in 0..<16 { msg[i] = UInt8(i) }
        let tag = ChaCha20Poly1305.poly1305Mac(message: msg, key: key)
        XCTAssertEqual(tag.count, 16)
    }

    // MARK: - AEAD Tests

    func testAEADRFC8439Section282() {
        let key = fromHex(
            "808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f"
        )
        let nonce = fromHex("070000004041424344454647")
        let aad = fromHex("50515253c0c1c2c3c4c5c6c7")
        let plaintext = fromString(
            "Ladies and Gentlemen of the class of \'99: If I could offer you only one tip for the future, sunscreen would be it."
        )

        let expectedCt = fromHex(
            "d31a8d34648e60db7b86afbc53ef7ec2"
            + "a4aded51296e08fea9e2b5a736ee62d6"
            + "3dbea45e8ca9671282fafb69da92728b"
            + "1a71de0a9e060b2905d6a5b67ecd3b36"
            + "92ddbd7f2d778b8c9803aee328091b58"
            + "fab324e4fad675945585808b4831d7bc"
            + "3ff4def08e4b7a9de576d26586cec64b"
            + "6116"
        )
        let expectedTag = fromHex("1ae10b594f09e26a7e902ecbd0600691")

        let (ciphertext, tag) = ChaCha20Poly1305.aeadEncrypt(
            plaintext: plaintext, key: key, nonce: nonce, aad: aad
        )
        XCTAssertEqual(toHex(ciphertext), toHex(expectedCt))
        XCTAssertEqual(toHex(tag), toHex(expectedTag))
    }

    func testAEADRoundTrip() {
        let key = fromHex(
            "808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f"
        )
        let nonce = fromHex("070000004041424344454647")
        let aad = fromHex("50515253c0c1c2c3c4c5c6c7")
        let plaintext = fromString(
            "Ladies and Gentlemen of the class of \'99: If I could offer you only one tip for the future, sunscreen would be it."
        )

        let (ciphertext, tag) = ChaCha20Poly1305.aeadEncrypt(
            plaintext: plaintext, key: key, nonce: nonce, aad: aad
        )
        let decrypted = ChaCha20Poly1305.aeadDecrypt(
            ciphertext: ciphertext, key: key, nonce: nonce, aad: aad, tag: tag
        )
        XCTAssertEqual(decrypted, plaintext)
    }

    func testAEADTamperedCiphertext() {
        let key = fromHex(
            "808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f"
        )
        let nonce = fromHex("070000004041424344454647")
        let aad = fromHex("50515253c0c1c2c3c4c5c6c7")
        let plaintext = fromString("Secret message")

        var (ciphertext, tag) = ChaCha20Poly1305.aeadEncrypt(
            plaintext: plaintext, key: key, nonce: nonce, aad: aad
        )
        ciphertext[0] ^= 0x01
        let result = ChaCha20Poly1305.aeadDecrypt(
            ciphertext: ciphertext, key: key, nonce: nonce, aad: aad, tag: tag
        )
        XCTAssertNil(result)
    }

    func testAEADTamperedAAD() {
        let key = fromHex(
            "808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f"
        )
        let nonce = fromHex("070000004041424344454647")
        var aad = fromHex("50515253c0c1c2c3c4c5c6c7")
        let plaintext = fromString("Secret message")

        let (ciphertext, tag) = ChaCha20Poly1305.aeadEncrypt(
            plaintext: plaintext, key: key, nonce: nonce, aad: aad
        )
        aad[0] ^= 0x01
        let result = ChaCha20Poly1305.aeadDecrypt(
            ciphertext: ciphertext, key: key, nonce: nonce, aad: aad, tag: tag
        )
        XCTAssertNil(result)
    }

    func testAEADEmptyPlaintext() {
        var key = [UInt8](repeating: 0, count: 32)
        for i in 0..<32 { key[i] = UInt8(i) }
        var nonce = [UInt8](repeating: 0, count: 12)
        nonce[0] = 7
        let aad = fromString("header data")

        let (ct, tag) = ChaCha20Poly1305.aeadEncrypt(
            plaintext: [], key: key, nonce: nonce, aad: aad
        )
        XCTAssertTrue(ct.isEmpty)
        XCTAssertEqual(tag.count, 16)

        let pt = ChaCha20Poly1305.aeadDecrypt(
            ciphertext: ct, key: key, nonce: nonce, aad: aad, tag: tag
        )
        XCTAssertEqual(pt, [])
    }

    func testAEADEmptyAAD() {
        var key = [UInt8](repeating: 0, count: 32)
        for i in 0..<32 { key[i] = UInt8(i) }
        let nonce = [UInt8](repeating: 0, count: 12)
        let plaintext = fromString("Hello, World!")

        let (ct, tag) = ChaCha20Poly1305.aeadEncrypt(
            plaintext: plaintext, key: key, nonce: nonce, aad: []
        )
        let pt = ChaCha20Poly1305.aeadDecrypt(
            ciphertext: ct, key: key, nonce: nonce, aad: [], tag: tag
        )
        XCTAssertEqual(pt, plaintext)
    }

    func testAEADWrongTag() {
        let key = [UInt8](repeating: 0, count: 32)
        let nonce = [UInt8](repeating: 0, count: 12)
        let plaintext = fromString("test")

        let (ciphertext, _) = ChaCha20Poly1305.aeadEncrypt(
            plaintext: plaintext, key: key, nonce: nonce, aad: []
        )
        let wrongTag = [UInt8](repeating: 0, count: 16)
        let result = ChaCha20Poly1305.aeadDecrypt(
            ciphertext: ciphertext, key: key, nonce: nonce, aad: [], tag: wrongTag
        )
        XCTAssertNil(result)
    }

    func testAEADLargePlaintext() {
        var key = [UInt8](repeating: 0, count: 32)
        for i in 0..<32 { key[i] = UInt8(i) }
        var nonce = [UInt8](repeating: 0, count: 12)
        nonce[4] = 0xAB
        let aad = fromString("extra data")
        var plaintext = [UInt8](repeating: 0, count: 500)
        for i in 0..<500 { plaintext[i] = UInt8(i % 256) }

        let (ct, tag) = ChaCha20Poly1305.aeadEncrypt(
            plaintext: plaintext, key: key, nonce: nonce, aad: aad
        )
        XCTAssertEqual(ct.count, 500)
        let pt = ChaCha20Poly1305.aeadDecrypt(
            ciphertext: ct, key: key, nonce: nonce, aad: aad, tag: tag
        )
        XCTAssertEqual(pt, plaintext)
    }
}
