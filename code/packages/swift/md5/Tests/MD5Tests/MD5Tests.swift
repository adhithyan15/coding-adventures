// MD5Tests.swift
// Tests for the MD5 message digest implementation (RFC 1321).

import XCTest
import Foundation
@testable import MD5

final class MD5Tests: XCTestCase {

    // ========================================================================
    // One-Shot md5() -- RFC 1321 Test Vectors
    // ========================================================================

    func testEmptyString() {
        XCTAssertEqual(md5Hex(Data()), "d41d8cd98f00b204e9800998ecf8427e")
    }

    func testSingleA() {
        XCTAssertEqual(md5Hex(Data("a".utf8)), "0cc175b9c0f1b6a831c399e269772661")
    }

    func testABC() {
        XCTAssertEqual(md5Hex(Data("abc".utf8)), "900150983cd24fb0d6963f7d28e17f72")
    }

    func testMessageDigest() {
        XCTAssertEqual(md5Hex(Data("message digest".utf8)), "f96b697d7cb7938d525a2f31aaf161d0")
    }

    func testLowercaseAlphabet() {
        XCTAssertEqual(
            md5Hex(Data("abcdefghijklmnopqrstuvwxyz".utf8)),
            "c3fcd3d76192e4007dfb496cca67e13b"
        )
    }

    func testAlphanumeric() {
        let input = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
        XCTAssertEqual(md5Hex(Data(input.utf8)), "d174ab98d277d9f5a5611c2c9f419d9f")
    }

    func testNumericRepeat() {
        let input = "12345678901234567890123456789012345678901234567890123456789012345678901234567890"
        XCTAssertEqual(md5Hex(Data(input.utf8)), "57edf4a22be3c955ac49da2e2107b67a")
    }

    // ========================================================================
    // One-Shot md5() -- Edge Cases
    // ========================================================================

    func testSingleZeroByte() {
        XCTAssertEqual(md5Hex(Data([0x00])), "93b885adfe0da089cdf634904fd59f71")
    }

    func testSingleFFByte() {
        XCTAssertEqual(md5Hex(Data([0xFF])), "00594fd4f42ba43fc1ca0427a0576295")
    }

    func testExact55Bytes() {
        // 55 bytes: one block with room for padding (55 + 1 + 0 + 8 = 64)
        let data = Data(repeating: 0x61, count: 55)
        XCTAssertEqual(md5Hex(data), "ef1772b6dff9a122358552954ad0df65")
    }

    func testExact56Bytes() {
        // 56 bytes: forces two blocks (56 + 1 > 56, need extra block)
        let data = Data(repeating: 0x61, count: 56)
        XCTAssertEqual(md5Hex(data), "3b0c8ac703f828b04c6c197006d17218")
    }

    func testExact64Bytes() {
        // 64 bytes: two blocks with padding in second
        let data = Data(repeating: 0x61, count: 64)
        XCTAssertEqual(md5Hex(data), "014842d480b571495a4a0363793f7367")
    }

    func test127Bytes() {
        let data = Data(repeating: 0x61, count: 127)
        XCTAssertEqual(md5Hex(data), "020406e1d05cdc2aa287641f7ae2cc39")
    }

    func test128Bytes() {
        let data = Data(repeating: 0x61, count: 128)
        XCTAssertEqual(md5Hex(data), "e510683b3f5ffe4093d021808bc6ff70")
    }

    func testBinaryData() {
        let data = Data((0..<256).map { UInt8($0) })
        XCTAssertEqual(md5Hex(data), "e2c865db4162bed963bfaa9ef6ac18f0")
    }

    // ========================================================================
    // Return Type and Property Tests
    // ========================================================================

    func testReturnTypeIs16Bytes() {
        let digest = md5(Data("abc".utf8))
        XCTAssertEqual(digest.count, 16)
    }

    func testDeterministic() {
        let data = Data("hello world".utf8)
        XCTAssertEqual(md5(data), md5(data))
    }

    func testLittleEndianFirstByte() {
        // md5("abc") = 900150983cd24fb0d6963f7d28e17f72
        // First byte should be 0x90
        let digest = md5(Data("abc".utf8))
        XCTAssertEqual(digest[0], 0x90)
    }

    func testAvalanche() {
        XCTAssertNotEqual(md5(Data("a".utf8)), md5(Data("b".utf8)))
    }

    // ========================================================================
    // md5Hex() Tests
    // ========================================================================

    func testHexLength32() {
        let hex = md5Hex(Data("abc".utf8))
        XCTAssertEqual(hex.count, 32)
    }

    func testHexLowercase() {
        let hex = md5Hex(Data("abc".utf8))
        XCTAssertEqual(hex, hex.lowercased())
    }

    func testHexMatchesDigest() {
        let data = Data("test".utf8)
        let digest = md5(data)
        let hex = md5Hex(data)
        let expected = digest.map { String(format: "%02x", $0) }.joined()
        XCTAssertEqual(hex, expected)
    }

    // ========================================================================
    // Streaming MD5Hasher Tests
    // ========================================================================

    func testStreamingSingleUpdateEqualsOneShot() {
        var hasher = MD5Hasher()
        hasher.update(Data("abc".utf8))
        XCTAssertEqual(hasher.hexDigest(), md5Hex(Data("abc".utf8)))
    }

    func testStreamingSplitAtByteBoundary() {
        var hasher = MD5Hasher()
        hasher.update(Data("ab".utf8))
        hasher.update(Data("c".utf8))
        XCTAssertEqual(hasher.hexDigest(), "900150983cd24fb0d6963f7d28e17f72")
    }

    func testStreamingSplitAtBlockBoundary() {
        let data = Data(repeating: 0x61, count: 128)
        let chunk1 = data.prefix(64)
        let chunk2 = data.suffix(64)

        var hasher = MD5Hasher()
        hasher.update(chunk1)
        hasher.update(chunk2)
        XCTAssertEqual(hasher.hexDigest(), md5Hex(data))
    }

    func testStreamingManyTinyUpdates() {
        let message = "abcdefghijklmnopqrstuvwxyz"
        var hasher = MD5Hasher()
        for byte in message.utf8 {
            hasher.update(Data([byte]))
        }
        XCTAssertEqual(hasher.hexDigest(), md5Hex(Data(message.utf8)))
    }

    func testDigestNonDestructive() {
        var hasher = MD5Hasher()
        hasher.update(Data("abc".utf8))
        let digest1 = hasher.hexDigest()
        let digest2 = hasher.hexDigest()
        XCTAssertEqual(digest1, digest2)
    }

    func testContinueAfterDigest() {
        var hasher = MD5Hasher()
        hasher.update(Data("abc".utf8))
        _ = hasher.hexDigest()
        hasher.update(Data("def".utf8))
        XCTAssertEqual(hasher.hexDigest(), md5Hex(Data("abcdef".utf8)))
    }

    func testCopyIsIndependent() {
        var original = MD5Hasher()
        original.update(Data("abc".utf8))
        var copied = original.copy()

        copied.update(Data("def".utf8))

        XCTAssertEqual(original.hexDigest(), md5Hex(Data("abc".utf8)))
        XCTAssertEqual(copied.hexDigest(), md5Hex(Data("abcdef".utf8)))
    }

    func testEmptyStreaming() {
        let hasher = MD5Hasher()
        XCTAssertEqual(hasher.hexDigest(), md5Hex(Data()))
    }

    func testStreamingLargeInput() {
        let fullData = Data(repeating: 0x42, count: 1000)
        var hasher = MD5Hasher()
        var offset = 0
        while offset < fullData.count {
            let end = min(offset + 100, fullData.count)
            hasher.update(fullData[offset..<end])
            offset = end
        }
        XCTAssertEqual(hasher.hexDigest(), md5Hex(fullData))
    }
}
