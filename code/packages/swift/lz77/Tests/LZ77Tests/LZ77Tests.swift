// LZ77Tests.swift
// Comprehensive tests for the LZ77 compression implementation.
//
// Test vectors come from the CMP00 specification. Covers: literals,
// backreferences, overlapping matches, edge cases, and round-trip invariants.

import XCTest
@testable import LZ77

final class LZ77Tests: XCTestCase {

    // MARK: - Helpers

    func enc(_ s: String) -> [UInt8] { Array(s.utf8) }
    func dec(_ b: [UInt8]) -> String { String(bytes: b, encoding: .utf8) ?? "" }

    // Round-trip helper: compress then decompress.
    func rt(_ s: String) -> String { dec(decompress(compress(enc(s)))) }

    // MARK: - Specification Test Vectors

    func testEmptyInput() {
        XCTAssertEqual(encode([]).count, 0)
        XCTAssertEqual(decode([]).count, 0)
    }

    func testNoRepetition() {
        // "ABCDE" → all literal tokens.
        let tokens = encode(enc("ABCDE"))
        XCTAssertEqual(tokens.count, 5)
        for t in tokens {
            XCTAssertEqual(t.offset, 0)
            XCTAssertEqual(t.length, 0)
        }
    }

    func testAllIdenticalBytes() {
        // "AAAAAAA" → literal A + backreference (offset=1, length=5, nextChar=A).
        let tokens = encode(enc("AAAAAAA"))
        XCTAssertEqual(tokens.count, 2)
        XCTAssertEqual(tokens[0], Token(offset: 0, length: 0, nextChar: 65))
        XCTAssertEqual(tokens[1].offset, 1)
        XCTAssertEqual(tokens[1].length, 5)
        XCTAssertEqual(tokens[1].nextChar, 65)
        XCTAssertEqual(dec(decode(tokens)), "AAAAAAA")
    }

    func testRepeatedPair() {
        // "ABABABAB" → [A literal, B literal, (offset=2, length=5, nextChar='B')].
        let tokens = encode(enc("ABABABAB"))
        XCTAssertEqual(tokens.count, 3)
        XCTAssertEqual(tokens[0], Token(offset: 0, length: 0, nextChar: 65))
        XCTAssertEqual(tokens[1], Token(offset: 0, length: 0, nextChar: 66))
        XCTAssertEqual(tokens[2].offset, 2)
        XCTAssertEqual(tokens[2].length, 5)
        XCTAssertEqual(tokens[2].nextChar, 66)
        XCTAssertEqual(dec(decode(tokens)), "ABABABAB")
    }

    func testSubstringReuseNoMatch() {
        // "AABCBBABC" with min_match=3 → all literals.
        let tokens = encode(enc("AABCBBABC"))
        XCTAssertEqual(tokens.count, 9)
        for t in tokens { XCTAssertEqual(t.offset, 0); XCTAssertEqual(t.length, 0) }
        XCTAssertEqual(dec(decode(tokens)), "AABCBBABC")
    }

    func testSubstringReuseWithLowerMinMatch() {
        let tokens = encode(enc("AABCBBABC"), minMatch: 2)
        XCTAssertEqual(dec(decode(tokens)), "AABCBBABC")
    }

    // MARK: - Round-Trip Tests

    func testRoundTrip() {
        let cases = ["", "A", "hello world", "the quick brown fox",
                     "ababababab", "aaaaaaaaaa"]
        for s in cases {
            let tokens = encode(enc(s))
            XCTAssertEqual(dec(decode(tokens)), s, "Round-trip failed for '\(s)'")
        }
    }

    func testBinaryRoundTrip() {
        let cases: [[UInt8]] = [
            [0, 0, 0],
            [255, 255, 255],
            Array(0...255),
            [0, 1, 2, 0, 1, 2],
        ]
        for data in cases {
            XCTAssertEqual(decode(encode(data)), data)
        }
    }

    func testCompressDecompressRoundTrip() {
        let cases = ["", "A", "ABCDE", "AAAAAAA", "ABABABAB", "hello world"]
        for s in cases {
            XCTAssertEqual(rt(s), s, "Compress/decompress failed for '\(s)'")
        }
    }

    // MARK: - Parameter Tests

    func testWindowSizeLimit() {
        var data: [UInt8] = [88]
        data += [UInt8](repeating: 89, count: 5000)
        data.append(88)
        let tokens = encode(data, windowSize: 100)
        for t in tokens {
            XCTAssertLessThanOrEqual(Int(t.offset), 100, "offset \(t.offset) exceeds windowSize 100")
        }
    }

    func testMaxMatchLimit() {
        let data = [UInt8](repeating: 65, count: 1000)
        let tokens = encode(data, maxMatch: 50)
        for t in tokens {
            XCTAssertLessThanOrEqual(Int(t.length), 50, "length \(t.length) exceeds maxMatch 50")
        }
    }

    func testMinMatchThreshold() {
        let tokens = encode(enc("AABAA"), minMatch: 2)
        for t in tokens {
            XCTAssertTrue(t.length == 0 || t.length >= 2)
        }
    }

    // MARK: - Edge Cases

    func testSingleByteLiteral() {
        let tokens = encode(enc("X"))
        XCTAssertEqual(tokens.count, 1)
        XCTAssertEqual(tokens[0], Token(offset: 0, length: 0, nextChar: 88))
    }

    func testExactWindowBoundary() {
        let data = [UInt8](repeating: 88, count: 11)
        let tokens = encode(data, windowSize: 10)
        XCTAssertTrue(tokens.contains { $0.offset > 0 }, "Expected at least one match")
        XCTAssertEqual(decode(tokens), data)
    }

    func testOverlappingMatchDecode() {
        // [A, B] + (offset=2, length=5, nextChar='Z') → ABABABAZ
        let tokens: [Token] = [
            Token(offset: 0, length: 0, nextChar: 65),
            Token(offset: 0, length: 0, nextChar: 66),
            Token(offset: 2, length: 5, nextChar: 90),
        ]
        XCTAssertEqual(dec(decode(tokens)), "ABABABAZ")
    }

    func testBinaryWithNulls() {
        let data: [UInt8] = [0, 0, 0, 255, 255]
        XCTAssertEqual(decode(encode(data)), data)
    }

    func testVeryLongInput() {
        var data: [UInt8] = []
        for _ in 0..<100 { data += enc("Hello, World! ") }
        data += [UInt8](repeating: 88, count: 500)
        XCTAssertEqual(decode(encode(data)), data)
    }

    func testAllSameByteCompresses() {
        let data = [UInt8](repeating: 65, count: 10000)
        let tokens = encode(data)
        // ~41 tokens: 1 literal + ~39 × 255 + 1 partial.
        XCTAssertLessThan(tokens.count, 50, "Expected compression")
        XCTAssertEqual(decode(tokens), data)
    }

    func testInitialBuffer() {
        // Seed [A, B] and apply (offset=2, length=3, nextChar='Z') → ABABAZ.
        let tokens = [Token(offset: 2, length: 3, nextChar: 90)]
        let result = decode(tokens, initialBuffer: [65, 66])
        XCTAssertEqual(dec(result), "ABABAZ")
    }

    // MARK: - Serialisation Tests

    func testSerialiseFormatSize() {
        let tokens = [Token(offset: 0, length: 0, nextChar: 65),
                      Token(offset: 2, length: 5, nextChar: 66)]
        let serialised = serialiseTokens(tokens)
        XCTAssertEqual(serialised.count, 4 + 2 * 4)
    }

    func testSerialiseDeserialiseRoundTrip() {
        let tokens = [Token(offset: 0, length: 0, nextChar: 65),
                      Token(offset: 1, length: 3, nextChar: 66),
                      Token(offset: 2, length: 5, nextChar: 67)]
        XCTAssertEqual(deserialiseTokens(serialiseTokens(tokens)), tokens)
    }

    func testDeserialiseEmpty() {
        XCTAssertEqual(deserialiseTokens([]).count, 0)
    }

    func testCompressDecompressAllVectors() {
        let vectors = ["", "ABCDE", "AAAAAAA", "ABABABAB", "AABCBBABC"]
        for s in vectors {
            XCTAssertEqual(rt(s), s, "Failed for '\(s)'")
        }
    }

    // MARK: - Behaviour Tests

    func testNoExpansionOnIncompressibleData() {
        let data = Array<UInt8>(0...255)
        let compressed = compress(data)
        XCTAssertLessThanOrEqual(compressed.count, 4 * data.count + 10)
    }

    func testRepetitiveDataCompresses() {
        let chunk = enc("ABC")
        var data: [UInt8] = []
        for _ in 0..<100 { data += chunk }
        XCTAssertLessThan(compress(data).count, data.count)
    }

    func testDeterministicCompression() {
        let data = enc("hello world test")
        XCTAssertEqual(compress(data), compress(data))
    }
}
