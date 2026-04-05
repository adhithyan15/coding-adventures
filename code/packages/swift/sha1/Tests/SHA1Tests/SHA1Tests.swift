// SHA1Tests.swift
// Tests for the SHA-1 hash function implementation (FIPS 180-4).

import XCTest
import Foundation
@testable import SHA1

final class SHA1Tests: XCTestCase {

    // ========================================================================
    // One-Shot sha1() -- FIPS 180-4 Test Vectors
    // ========================================================================

    func testEmptyString() {
        XCTAssertEqual(sha1Hex(Data()), "da39a3ee5e6b4b0d3255bfef95601890afd80709")
    }

    func testABC() {
        XCTAssertEqual(sha1Hex(Data("abc".utf8)), "a9993e364706816aba3e25717850c26c9cd0d89d")
    }

    func test56ByteMessage() {
        // "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq" = 56 bytes
        let input = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
        XCTAssertEqual(sha1Hex(Data(input.utf8)), "84983e441c3bd26ebaae4aa1f95129e5e54670f1")
    }

    // ========================================================================
    // One-Shot sha1() -- Additional Vectors
    // ========================================================================

    func testSingleA() {
        XCTAssertEqual(sha1Hex(Data("a".utf8)), "86f7e437faa5a7fce15d1ddcb9eaeaea377667b8")
    }

    func testSingleZeroByte() {
        XCTAssertEqual(sha1Hex(Data([0x00])), "5ba93c9db0cff93f52b521d7420e43f6eda2784f")
    }

    func testSingleFFByte() {
        XCTAssertEqual(sha1Hex(Data([0xFF])), "85e53271e14006f0265921d02d4d736cdc580b0b")
    }

    // ========================================================================
    // Edge Cases: Block Boundary Tests
    // ========================================================================

    func testExact55Bytes() {
        let data = Data(repeating: 0x61, count: 55)
        XCTAssertEqual(sha1Hex(data), "c1c8bbdc22796e28c0e15163d20899b65621d65a")
    }

    func testExact56Bytes() {
        let data = Data(repeating: 0x61, count: 56)
        XCTAssertEqual(sha1Hex(data), "c2db330f6083854c99d4b5bfb6e8f29f201be699")
    }

    func testExact64Bytes() {
        let data = Data(repeating: 0x61, count: 64)
        XCTAssertEqual(sha1Hex(data), "0098ba824b5c16427bd7a1122a5a442a25ec644d")
    }

    func test127Bytes() {
        let data = Data(repeating: 0x61, count: 127)
        XCTAssertEqual(sha1Hex(data), "89d95fa32ed44a7c610b7ee38517ddf57e0bb975")
    }

    func test128Bytes() {
        let data = Data(repeating: 0x61, count: 128)
        XCTAssertEqual(sha1Hex(data), "ad5b3fdbcb526778c2839d2f151ea753995e26a0")
    }

    func testBinaryData() {
        let data = Data((0..<256).map { UInt8($0) })
        XCTAssertEqual(sha1Hex(data), "4916d6bdb7f78e6803698cab32d1586ea457dfc8")
    }

    // ========================================================================
    // Return Type and Property Tests
    // ========================================================================

    func testReturnTypeIs20Bytes() {
        let digest = sha1(Data("abc".utf8))
        XCTAssertEqual(digest.count, 20)
    }

    func testDeterministic() {
        let data = Data("hello world".utf8)
        XCTAssertEqual(sha1(data), sha1(data))
    }

    func testAvalanche() {
        XCTAssertNotEqual(sha1(Data("a".utf8)), sha1(Data("b".utf8)))
    }

    // ========================================================================
    // sha1Hex() Tests
    // ========================================================================

    func testHexLength40() {
        let hex = sha1Hex(Data("abc".utf8))
        XCTAssertEqual(hex.count, 40)
    }

    func testHexLowercase() {
        let hex = sha1Hex(Data("abc".utf8))
        XCTAssertEqual(hex, hex.lowercased())
    }

    func testHexMatchesDigest() {
        let data = Data("test".utf8)
        let digest = sha1(data)
        let hex = sha1Hex(data)
        let expected = digest.map { String(format: "%02x", $0) }.joined()
        XCTAssertEqual(hex, expected)
    }

    // ========================================================================
    // Streaming SHA1Hasher Tests
    // ========================================================================

    func testStreamingSingleUpdateEqualsOneShot() {
        var hasher = SHA1Hasher()
        hasher.update(Data("abc".utf8))
        XCTAssertEqual(hasher.hexDigest(), sha1Hex(Data("abc".utf8)))
    }

    func testStreamingSplitAtByteBoundary() {
        var hasher = SHA1Hasher()
        hasher.update(Data("ab".utf8))
        hasher.update(Data("c".utf8))
        XCTAssertEqual(hasher.hexDigest(), "a9993e364706816aba3e25717850c26c9cd0d89d")
    }

    func testStreamingSplitAtBlockBoundary() {
        let data = Data(repeating: 0x61, count: 128)
        let chunk1 = data.prefix(64)
        let chunk2 = data.suffix(64)

        var hasher = SHA1Hasher()
        hasher.update(chunk1)
        hasher.update(chunk2)
        XCTAssertEqual(hasher.hexDigest(), sha1Hex(data))
    }

    func testStreamingManyTinyUpdates() {
        let message = "abcdefghijklmnopqrstuvwxyz"
        var hasher = SHA1Hasher()
        for byte in message.utf8 {
            hasher.update(Data([byte]))
        }
        XCTAssertEqual(hasher.hexDigest(), sha1Hex(Data(message.utf8)))
    }

    func testDigestNonDestructive() {
        var hasher = SHA1Hasher()
        hasher.update(Data("abc".utf8))
        let digest1 = hasher.hexDigest()
        let digest2 = hasher.hexDigest()
        XCTAssertEqual(digest1, digest2)
    }

    func testContinueAfterDigest() {
        var hasher = SHA1Hasher()
        hasher.update(Data("abc".utf8))
        _ = hasher.hexDigest()
        hasher.update(Data("def".utf8))
        XCTAssertEqual(hasher.hexDigest(), sha1Hex(Data("abcdef".utf8)))
    }

    func testCopyIsIndependent() {
        var original = SHA1Hasher()
        original.update(Data("abc".utf8))
        var copied = original.copy()

        copied.update(Data("def".utf8))

        XCTAssertEqual(original.hexDigest(), sha1Hex(Data("abc".utf8)))
        XCTAssertEqual(copied.hexDigest(), sha1Hex(Data("abcdef".utf8)))
    }

    func testEmptyStreaming() {
        let hasher = SHA1Hasher()
        XCTAssertEqual(hasher.hexDigest(), sha1Hex(Data()))
    }

    func testStreamingLargeInput() {
        let fullData = Data(repeating: 0x42, count: 1000)
        var hasher = SHA1Hasher()
        var offset = 0
        while offset < fullData.count {
            let end = min(offset + 100, fullData.count)
            hasher.update(fullData[offset..<end])
            offset = end
        }
        XCTAssertEqual(hasher.hexDigest(), sha1Hex(fullData))
    }
}
