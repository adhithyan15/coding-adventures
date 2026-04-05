// SHA256Tests.swift
// Tests for the SHA-256 hash function implementation (FIPS 180-4).

import XCTest
import Foundation
@testable import SHA256

final class SHA256Tests: XCTestCase {

    // ========================================================================
    // One-Shot sha256() -- FIPS 180-4 Test Vectors
    // ========================================================================

    func testEmptyString() {
        XCTAssertEqual(sha256Hex(Data()), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
    }

    func testABC() {
        XCTAssertEqual(sha256Hex(Data("abc".utf8)), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
    }

    func test448BitMessage() {
        let input = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
        XCTAssertEqual(sha256Hex(Data(input.utf8)), "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1")
    }

    // ========================================================================
    // One-Shot sha256() -- Additional Vectors
    // ========================================================================

    func testHello() {
        XCTAssertEqual(sha256Hex(Data("hello".utf8)), "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824")
    }

    func testSingleA() {
        XCTAssertEqual(sha256Hex(Data("a".utf8)), "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb")
    }

    func testPangram() {
        XCTAssertEqual(sha256Hex(Data("The quick brown fox jumps over the lazy dog".utf8)), "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592")
    }

    func testSingleZeroByte() {
        XCTAssertEqual(sha256Hex(Data([0x00])), "6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d")
    }

    func testSingleFFByte() {
        XCTAssertEqual(sha256Hex(Data([0xFF])), "a8100ae6aa1940d0b663bb31cd466142ebbdbd5187131b92d93818987832eb89")
    }

    // ========================================================================
    // Edge Cases: Block Boundary Tests
    // ========================================================================

    func testExact55Bytes() {
        let data = Data(repeating: 0x61, count: 55)
        XCTAssertEqual(sha256Hex(data), "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318")
    }

    func testExact56Bytes() {
        let data = Data(repeating: 0x61, count: 56)
        XCTAssertEqual(sha256Hex(data), "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a")
    }

    func testExact64Bytes() {
        let data = Data(repeating: 0x61, count: 64)
        XCTAssertEqual(sha256Hex(data), "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb")
    }

    func test127Bytes() {
        let data = Data(repeating: 0x61, count: 127)
        let hex = sha256Hex(data)
        XCTAssertEqual(hex.count, 64)
    }

    func test128Bytes() {
        let data = Data(repeating: 0x61, count: 128)
        let hex = sha256Hex(data)
        XCTAssertEqual(hex.count, 64)
    }

    func testBinaryData() {
        let data = Data((0..<256).map { UInt8($0) })
        let hex = sha256Hex(data)
        XCTAssertEqual(hex.count, 64)
    }

    // ========================================================================
    // Return Type and Property Tests
    // ========================================================================

    func testReturnTypeIs32Bytes() {
        let digest = sha256(Data("abc".utf8))
        XCTAssertEqual(digest.count, 32)
    }

    func testDeterministic() {
        let data = Data("hello world".utf8)
        XCTAssertEqual(sha256(data), sha256(data))
    }

    func testAvalanche() {
        XCTAssertNotEqual(sha256(Data("a".utf8)), sha256(Data("b".utf8)))
    }

    // ========================================================================
    // sha256Hex() Tests
    // ========================================================================

    func testHexLength64() {
        let hex = sha256Hex(Data("abc".utf8))
        XCTAssertEqual(hex.count, 64)
    }

    func testHexLowercase() {
        let hex = sha256Hex(Data("abc".utf8))
        XCTAssertEqual(hex, hex.lowercased())
    }

    func testHexMatchesDigest() {
        let data = Data("test".utf8)
        let digest = sha256(data)
        let hex = sha256Hex(data)
        let expected = digest.map { String(format: "%02x", $0) }.joined()
        XCTAssertEqual(hex, expected)
    }

    // ========================================================================
    // Streaming SHA256Hasher Tests
    // ========================================================================

    func testStreamingSingleUpdateEqualsOneShot() {
        var hasher = SHA256Hasher()
        hasher.update(Data("abc".utf8))
        XCTAssertEqual(hasher.hexDigest(), sha256Hex(Data("abc".utf8)))
    }

    func testStreamingSplitAtByteBoundary() {
        var hasher = SHA256Hasher()
        hasher.update(Data("ab".utf8))
        hasher.update(Data("c".utf8))
        XCTAssertEqual(hasher.hexDigest(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
    }

    func testStreamingSplitAtBlockBoundary() {
        let data = Data(repeating: 0x61, count: 128)
        let chunk1 = data.prefix(64)
        let chunk2 = data.suffix(64)

        var hasher = SHA256Hasher()
        hasher.update(chunk1)
        hasher.update(chunk2)
        XCTAssertEqual(hasher.hexDigest(), sha256Hex(data))
    }

    func testStreamingManyTinyUpdates() {
        let message = "abcdefghijklmnopqrstuvwxyz"
        var hasher = SHA256Hasher()
        for byte in message.utf8 {
            hasher.update(Data([byte]))
        }
        XCTAssertEqual(hasher.hexDigest(), sha256Hex(Data(message.utf8)))
    }

    func testDigestNonDestructive() {
        var hasher = SHA256Hasher()
        hasher.update(Data("abc".utf8))
        let digest1 = hasher.hexDigest()
        let digest2 = hasher.hexDigest()
        XCTAssertEqual(digest1, digest2)
    }

    func testContinueAfterDigest() {
        var hasher = SHA256Hasher()
        hasher.update(Data("abc".utf8))
        _ = hasher.hexDigest()
        hasher.update(Data("def".utf8))
        XCTAssertEqual(hasher.hexDigest(), sha256Hex(Data("abcdef".utf8)))
    }

    func testCopyIsIndependent() {
        var original = SHA256Hasher()
        original.update(Data("abc".utf8))
        var copied = original.copy()

        copied.update(Data("def".utf8))

        XCTAssertEqual(original.hexDigest(), sha256Hex(Data("abc".utf8)))
        XCTAssertEqual(copied.hexDigest(), sha256Hex(Data("abcdef".utf8)))
    }

    func testEmptyStreaming() {
        let hasher = SHA256Hasher()
        XCTAssertEqual(hasher.hexDigest(), sha256Hex(Data()))
    }

    func testStreamingLargeInput() {
        let fullData = Data(repeating: 0x42, count: 1000)
        var hasher = SHA256Hasher()
        var offset = 0
        while offset < fullData.count {
            let end = min(offset + 100, fullData.count)
            hasher.update(fullData[offset..<end])
            offset = end
        }
        XCTAssertEqual(hasher.hexDigest(), sha256Hex(fullData))
    }
}
