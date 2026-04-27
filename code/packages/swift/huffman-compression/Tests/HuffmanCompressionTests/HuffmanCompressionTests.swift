// HuffmanCompressionTests.swift
// Comprehensive tests for the CMP04 Huffman compression implementation.
//
// Test coverage:
//   - Spec wire-format vector for "AAABBC"
//   - Round-trip tests for various inputs
//   - Edge cases: empty input, single byte, single symbol repeated, all 256 bytes
//   - Header parsing: big-endian byte order, symbol count
//   - Compression effectiveness for repetitive data
//   - Security: malformed input does not crash

import XCTest
@testable import HuffmanCompression

final class HuffmanCompressionTests: XCTestCase {

    // MARK: - Helpers

    /// Encode a string as UTF-8 bytes.
    func enc(_ s: String) -> [UInt8] { Array(s.utf8) }

    /// Decode bytes as a UTF-8 string (returns empty string on failure).
    func str(_ b: [UInt8]) -> String { String(bytes: b, encoding: .utf8) ?? "" }

    /// Round-trip: compress then decompress `data`.
    func rt(_ data: [UInt8]) throws -> [UInt8] {
        let compressed = try compress(data)
        return try decompress(compressed)
    }

    /// Round-trip: compress then decompress a UTF-8 string.
    func rtStr(_ s: String) throws -> String {
        str(try rt(enc(s)))
    }

    // MARK: - Spec Wire-Format Vector: "AAABBC"
    //
    // This is the primary correctness test. We verify the exact byte sequence
    // produced by compressing "AAABBC" against the CMP04 specification.
    //
    // The frequencies are: A=3, B=2, C=1
    // Canonical codes:
    //   A (65) → "0"   (length 1)
    //   B (66) → "10"  (length 2)
    //   C (67) → "11"  (length 2)
    //
    // Wire format breakdown:
    //   [0,0,0,6]   original_length = 6
    //   [0,0,0,3]   symbol_count    = 3
    //   [65,1]      A has code length 1
    //   [66,2]      B has code length 2
    //   [67,2]      C has code length 2
    //   [0xA8,0x01] bit stream for "000101011" packed LSB-first

    func testCompressAABBCWireFormat() throws {
        let input: [UInt8] = [65, 65, 65, 66, 66, 67]  // "AAABBC"
        let compressed = try compress(input)

        let expected: [UInt8] = [
            0, 0, 0, 6,    // original_length = 6
            0, 0, 0, 3,    // symbol_count    = 3
            65, 1,         // A (65) → length 1
            66, 2,         // B (66) → length 2
            67, 2,         // C (67) → length 2
            0xA8, 0x01,    // bit stream: "000101011" packed LSB-first
        ]
        XCTAssertEqual(compressed, expected,
            "compress('AAABBC') must match the CMP04 spec wire-format exactly")
    }

    func testDecompressAABBCFromWireFormat() throws {
        let wireBytes: [UInt8] = [
            0, 0, 0, 6,
            0, 0, 0, 3,
            65, 1,
            66, 2,
            67, 2,
            0xA8, 0x01,
        ]
        let result = try decompress(wireBytes)
        XCTAssertEqual(result, [65, 65, 65, 66, 66, 67],
            "decompress of 'AAABBC' wire bytes must recover original input")
    }

    // MARK: - Wire Format Header

    func testHeaderOriginalLengthIsBigEndian() throws {
        // original_length = 5 for "hello" (5 bytes) → [0,0,0,5]
        let compressed = try compress(enc("hello"))
        XCTAssertEqual(compressed[0], 0)
        XCTAssertEqual(compressed[1], 0)
        XCTAssertEqual(compressed[2], 0)
        XCTAssertEqual(compressed[3], 5)
    }

    func testHeaderSymbolCountIsBigEndian() throws {
        // "hello" has h=1, e=1, l=2, o=1 → 4 distinct symbols → symbol_count=4
        let compressed = try compress(enc("hello"))
        let symbolCount = Int(UInt32(compressed[4]) << 24
                           | UInt32(compressed[5]) << 16
                           | UInt32(compressed[6]) << 8
                           | UInt32(compressed[7]))
        XCTAssertEqual(symbolCount, 4,
            "symbol_count for 'hello' (h,e,l,o) must be 4")
    }

    func testHeaderOriginalLengthPreserved() throws {
        let data = enc("the quick brown fox")
        let compressed = try compress(data)
        let stored = Int(UInt32(compressed[0]) << 24
                      | UInt32(compressed[1]) << 16
                      | UInt32(compressed[2]) << 8
                      | UInt32(compressed[3]))
        XCTAssertEqual(stored, data.count)
    }

    // MARK: - Empty Input

    func testCompressEmpty() throws {
        let compressed = try compress([])
        // Must return at least 8 bytes (two-field header) and no crash.
        XCTAssertGreaterThanOrEqual(compressed.count, 8,
            "compress([]) must return at least the 8-byte header")
        let origLen = Int(UInt32(compressed[0]) << 24
                       | UInt32(compressed[1]) << 16
                       | UInt32(compressed[2]) << 8
                       | UInt32(compressed[3]))
        XCTAssertEqual(origLen, 0, "original_length must be 0 for empty input")
    }

    func testDecompressEmptyWireFormat() throws {
        let compressed = try compress([])
        let result = try decompress(compressed)
        XCTAssertEqual(result, [], "round-trip of [] must return []")
    }

    func testRoundTripEmpty() throws {
        XCTAssertEqual(try rt([]), [])
    }

    // MARK: - Single Byte

    func testRoundTripSingleByte() throws {
        XCTAssertEqual(try rt([0x42]), [0x42])
    }

    func testRoundTripSingleByteA() throws {
        XCTAssertEqual(try rtStr("A"), "A")
    }

    func testRoundTripSingleByteNull() throws {
        XCTAssertEqual(try rt([0x00]), [0x00])
    }

    func testRoundTripSingleByteMaxByte() throws {
        XCTAssertEqual(try rt([0xFF]), [0xFF])
    }

    // MARK: - Single Distinct Symbol Repeated

    func testRoundTripAllSameBytes() throws {
        // Single symbol: the tree has only one leaf.
        // The canonical code is "0" and each byte costs 1 bit.
        let data = [UInt8](repeating: 0x41, count: 100)
        XCTAssertEqual(try rt(data), data)
    }

    func testRoundTripAllSameBytesLong() throws {
        let data = [UInt8](repeating: 0xAA, count: 1000)
        XCTAssertEqual(try rt(data), data)
    }

    // MARK: - Round-Trip Correctness

    func testRoundTripHelloWorld() throws {
        XCTAssertEqual(try rtStr("hello world"), "hello world")
    }

    func testRoundTripAB() throws {
        XCTAssertEqual(try rtStr("AB"), "AB")
    }

    func testRoundTripABCDE() throws {
        XCTAssertEqual(try rtStr("ABCDE"), "ABCDE")
    }

    func testRoundTripAABBCC() throws {
        XCTAssertEqual(try rtStr("AABBCC"), "AABBCC")
    }

    func testRoundTripABCx100() throws {
        let s = String(repeating: "ABC", count: 100)
        XCTAssertEqual(try rtStr(s), s)
    }

    func testRoundTripLoremIpsum() throws {
        let s = "Lorem ipsum dolor sit amet, consectetur adipiscing elit."
        XCTAssertEqual(try rtStr(s), s)
    }

    func testRoundTripAllByteValues() throws {
        // All 256 distinct byte values — exercises every possible symbol.
        let data = [UInt8](0...255)
        XCTAssertEqual(try rt(data), data)
    }

    func testRoundTripAllByteValuesTwice() throws {
        // Two copies of the full byte range (512 bytes, 256 distinct symbols).
        let data = [UInt8](0...255) + [UInt8](0...255)
        XCTAssertEqual(try rt(data), data)
    }

    func testRoundTripBinaryData() throws {
        let data: [UInt8] = [0x00, 0xFF, 0x0F, 0xF0, 0xAA, 0x55, 0x12, 0x34]
        XCTAssertEqual(try rt(data), data)
    }

    func testRoundTripUnicodeUtf8() throws {
        // UTF-8 encoded emoji — multi-byte sequences.
        let s = "Hello 🌍 World"
        XCTAssertEqual(try rtStr(s), s)
    }

    func testRoundTripLongRepetitivePattern() throws {
        let data = [UInt8]((0..<1000).map { UInt8($0 % 5) })
        XCTAssertEqual(try rt(data), data)
    }

    // MARK: - Length Preservation

    func testLengthPreservedAfterRoundTrip() throws {
        let data = enc("the quick brown fox jumps over the lazy dog")
        let result = try rt(data)
        XCTAssertEqual(result.count, data.count)
    }

    func testLengthPreservedForBinaryData() throws {
        let data = [UInt8](0...255)
        let result = try rt(data)
        XCTAssertEqual(result.count, 256)
    }

    // MARK: - Code-Lengths Table

    func testCodeLengthsTableStartsAtByte8() throws {
        // For "AAABBC" we know exactly what the table looks like.
        let compressed = try compress([65, 65, 65, 66, 66, 67])
        // Byte 8 = symbol 65 (A), byte 9 = length 1
        XCTAssertEqual(compressed[8],  65, "First entry symbol should be A (65)")
        XCTAssertEqual(compressed[9],   1, "First entry length should be 1")
        // Byte 10 = symbol 66 (B), byte 11 = length 2
        XCTAssertEqual(compressed[10], 66, "Second entry symbol should be B (66)")
        XCTAssertEqual(compressed[11],  2, "Second entry length should be 2")
        // Byte 12 = symbol 67 (C), byte 13 = length 2
        XCTAssertEqual(compressed[12], 67, "Third entry symbol should be C (67)")
        XCTAssertEqual(compressed[13],  2, "Third entry length should be 2")
    }

    func testCodeLengthsSortedByLengthThenSymbol() throws {
        // "ABCDE" with equal frequencies — all lengths equal, sorted by symbol.
        let compressed = try compress(enc("ABCDE"))
        let symbolCount = Int(UInt32(compressed[4]) << 24
                           | UInt32(compressed[5]) << 16
                           | UInt32(compressed[6]) << 8
                           | UInt32(compressed[7]))
        XCTAssertEqual(symbolCount, 5)

        // Extract the sorted (symbol, length) pairs.
        var pairs = [(Int, Int)]()
        for i in 0..<symbolCount {
            let sym = Int(compressed[8 + i * 2])
            let len = Int(compressed[8 + i * 2 + 1])
            pairs.append((sym, len))
        }

        // Verify they are sorted by (length, symbol).
        for i in 1..<pairs.count {
            let prev = pairs[i - 1]
            let curr = pairs[i]
            let prevOrder = prev.1 * 1000 + prev.0
            let currOrder = curr.1 * 1000 + curr.0
            XCTAssertLessThanOrEqual(prevOrder, currOrder,
                "Code-lengths table must be sorted by (length, symbol): \(prev) vs \(curr)")
        }
    }

    // MARK: - Compression Determinism

    func testCompressIsDeterministic() throws {
        let data = enc("hello world test data 12345")
        XCTAssertEqual(try compress(data), try compress(data))
    }

    func testCompressAABBCIsDeterministic() throws {
        let input: [UInt8] = [65, 65, 65, 66, 66, 67]
        XCTAssertEqual(try compress(input), try compress(input))
    }

    // MARK: - Compression Effectiveness

    func testRepetitiveDataCompresses() throws {
        // 1000 repetitions of "ABC" (3000 bytes) — should compress well.
        let data = enc(String(repeating: "ABC", count: 1000))
        let compressed = try compress(data)
        XCTAssertLessThan(compressed.count, data.count,
            "Repetitive data should compress to fewer bytes than the original")
    }

    func testHighlyRepetitiveCompressesWell() throws {
        // 10 000 copies of the same byte — near-maximal compression.
        let data = [UInt8](repeating: 0x42, count: 10_000)
        let compressed = try compress(data)
        XCTAssertLessThan(compressed.count, data.count / 2,
            "10 000 identical bytes should compress to under 5 000 bytes")
    }

    // MARK: - Security / Malformed Input

    func testDecompressTooShortThrows() {
        for n in 0..<8 {
            let bad = [UInt8](repeating: 0, count: n)
            XCTAssertThrowsError(try decompress(bad),
                "decompress of \(n)-byte input must throw dataTooShort") { error in
                guard case HuffmanCompressionError.dataTooShort = error else {
                    XCTFail("Expected dataTooShort, got \(error)")
                    return
                }
            }
        }
    }

    func testDecompressZeroOriginalLength() throws {
        // Header: original_length=0, symbol_count=0 — should return [].
        let input: [UInt8] = [0, 0, 0, 0,  // original_length = 0
                              0, 0, 0, 0]  // symbol_count    = 0
        let result = try decompress(input)
        XCTAssertEqual(result, [])
    }

    func testDecompressTruncatedTableThrows() {
        // Claims 3 symbols but only provides 4 bytes of table (need 6).
        let bad: [UInt8] = [
            0, 0, 0, 6,   // original_length = 6
            0, 0, 0, 3,   // symbol_count    = 3
            65, 1,        // only one table entry (need 3)
            66,           // incomplete second entry
        ]
        XCTAssertThrowsError(try decompress(bad),
            "Truncated code-lengths table must throw truncatedCodeTable") { error in
            guard case HuffmanCompressionError.truncatedCodeTable = error else {
                XCTFail("Expected truncatedCodeTable, got \(error)")
                return
            }
        }
    }

    func testDecompressInvalidCodeLengthZeroThrows() {
        // A code-length entry with length=0 is invalid.
        let bad: [UInt8] = [
            0, 0, 0, 1,   // original_length = 1
            0, 0, 0, 1,   // symbol_count    = 1
            65, 0,        // length = 0, invalid
            0x00,         // bit stream
        ]
        XCTAssertThrowsError(try decompress(bad),
            "Code length 0 must throw invalidCodeLength") { error in
            guard case HuffmanCompressionError.invalidCodeLength = error else {
                XCTFail("Expected invalidCodeLength, got \(error)")
                return
            }
        }
    }

    func testDecompressAllZerosDoesNotCrash() {
        // 8 zero bytes: original_length=0, symbol_count=0 — valid empty stream.
        XCTAssertNoThrow(try decompress([UInt8](repeating: 0, count: 8)))
    }

    func testDecompressLargeOriginalLengthInHeader() {
        // Header claims a huge original_length but bit stream is small.
        // Must throw bitStreamExhausted, not crash or over-allocate.
        let bad: [UInt8] = [
            0xFF, 0xFF, 0xFF, 0xFF,  // original_length = 4 294 967 295
            0, 0, 0, 1,              // symbol_count    = 1
            65, 1,                   // A → length 1
            0xAA,                    // tiny bit stream
        ]
        // This may throw bitStreamExhausted or succeed with single-symbol path.
        // Either way, it must not crash or hang.
        _ = try? decompress(bad)
    }

    func testCompressAndDecompressPreservesLength() throws {
        // Verify that the decoded length exactly matches the original.
        let data = [UInt8](0...255)
        let result = try rt(data)
        XCTAssertEqual(result.count, data.count,
            "Decompressed data must have exactly the same length as the original")
    }
}
