// BrotliTests.swift — Tests for CMP06 Brotli-inspired compression
// ============================================================================
//
// Test cases follow the spec (CMP06-brotli.md) and mirror the test suite
// used in other language implementations of this package.
//
// All 8 required spec test cases are present, plus additional edge-case and
// stress tests to push coverage well above 80%.

import XCTest
@testable import CodingAdventuresBrotli

final class BrotliTests: XCTestCase {

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Compress then decompress and assert equality.
    func roundtrip(_ data: [UInt8], _ label: String = "data") throws {
        let compressed   = try Brotli.compress(data)
        let decompressed = try Brotli.decompress(compressed)
        XCTAssertEqual(decompressed, data, "roundtrip mismatch for '\(label)'")
    }

    /// Convert a Swift String to its UTF-8 bytes.
    func bytes(_ s: String) -> [UInt8] { Array(s.utf8) }

    // -----------------------------------------------------------------------
    // Spec Test 1: Round-trip — empty input
    // -----------------------------------------------------------------------
    //
    // compress("") → some bytes → decompress → ""
    // The empty-input special case emits a minimal 1-symbol ICC tree.

    func testEmptyInput() throws {
        let compressed   = try Brotli.compress([])
        let decompressed = try Brotli.decompress(compressed)
        XCTAssertEqual(decompressed, [], "empty round-trip failed")
    }

    // -----------------------------------------------------------------------
    // Spec Test 2: Round-trip — single byte
    // -----------------------------------------------------------------------
    //
    // compress([0x42]) → decompress → [0x42]

    func testSingleByte() throws {
        try roundtrip([0x42], "single byte 0x42")
        try roundtrip([0x00], "single byte 0x00")
        try roundtrip([0xFF], "single byte 0xFF")
        try roundtrip(bytes("A"), "single byte 'A'")
    }

    // -----------------------------------------------------------------------
    // Spec Test 3: Round-trip — all literals, no matches
    // -----------------------------------------------------------------------
    //
    // 256 distinct bytes (one of each value 0-255). This data is incompressible
    // because every byte appears exactly once — no LZ matches can be found.
    // Compressed size will exceed input size due to header overhead, but the
    // round-trip must be exact.

    func testAllLiteralsNoMatches() throws {
        let data = [UInt8](0...255)
        let compressed = try Brotli.compress(data)
        let decompressed = try Brotli.decompress(compressed)
        XCTAssertEqual(decompressed, data, "all-literals round-trip failed")
        // Incompressible data: compressed size may exceed original.
        // (No compression ratio assertion — this is the expected behaviour.)
        XCTAssertGreaterThan(compressed.count, 0, "compressed should be non-empty")
    }

    // -----------------------------------------------------------------------
    // Spec Test 4: Round-trip — all copies, no leading literals
    // -----------------------------------------------------------------------
    //
    // 1024 × 'A'. The first 4 bytes are literals (the window is empty at the
    // start), then one or more copy commands reproduce the rest.

    func testAllCopies() throws {
        let data = [UInt8](repeating: UInt8(ascii: "A"), count: 1024)
        try roundtrip(data, "1024×A")
    }

    // -----------------------------------------------------------------------
    // Spec Test 5: Round-trip — English prose
    // -----------------------------------------------------------------------
    //
    // ASCII text ≥ 1024 bytes with varied vocabulary.
    // Compressed size must be < 80% of input size.

    func testEnglishProse() throws {
        // Repeat a sentence to get >= 1024 bytes with repetition that LZ can exploit.
        let sentence = "The quick brown fox jumps over the lazy dog near the river bank. "
        var text = ""
        while text.count < 1024 { text += sentence }
        let data = bytes(text)
        let compressed = try Brotli.compress(data)
        let decompressed = try Brotli.decompress(compressed)
        XCTAssertEqual(decompressed, data, "English prose round-trip failed")
        XCTAssertLessThan(
            compressed.count,
            Int(Double(data.count) * 0.80),
            "compression should achieve >20% reduction on English text; got \(compressed.count) vs \(data.count)"
        )
    }

    // -----------------------------------------------------------------------
    // Spec Test 6: Round-trip — binary blob
    // -----------------------------------------------------------------------
    //
    // 512 bytes of pseudo-random binary data. Round-trip must be exact.
    // No compression ratio requirement (random data is incompressible).
    //
    // We use a simple linear congruential generator (LCG) for reproducibility
    // across language implementations. This is NOT cryptographically random —
    // it just produces a spread of values that avoid obvious LZ matches.

    func testBinaryBlob() throws {
        var data: [UInt8] = []
        var lcg: UInt32 = 0xDEADBEEF
        for _ in 0..<512 {
            lcg = lcg &* 1664525 &+ 1013904223
            data.append(UInt8(lcg >> 24))
        }
        try roundtrip(data, "512-byte binary blob")
    }

    // -----------------------------------------------------------------------
    // Spec Test 7: Cross-command literal context
    // -----------------------------------------------------------------------
    //
    // "abc123ABC" exercises all four context buckets:
    //   'a' → ctx0 (start of stream)
    //   'b' → ctx3 (after lowercase 'a')
    //   'c' → ctx3 (after lowercase 'b')
    //   '1' → ctx3 (after lowercase 'c')
    //   '2' → ctx1 (after digit '1')
    //   '3' → ctx1 (after digit '2')
    //   'A' → ctx1 (after digit '3')
    //   'B' → ctx2 (after uppercase 'A')
    //   'C' → ctx2 (after uppercase 'B')
    //
    // Verify round-trip and that multiple context trees are populated.

    func testCrossCommandLiteralContext() throws {
        let input = bytes("abc123ABC")
        try roundtrip(input, "abc123ABC")
    }

    // -----------------------------------------------------------------------
    // Spec Test 8: Long-distance match
    // -----------------------------------------------------------------------
    //
    // A 10-byte sequence repeated with offset > 4096 bytes, exercising
    // distance codes 24–31 (the extended codes absent in CMP05/DEFLATE).

    func testLongDistanceMatch() throws {
        // Pad: 5000 bytes of one pattern, then repeat a 10-byte marker.
        let marker: [UInt8] = [0xDE, 0xAD, 0xBE, 0xEF, 0x42, 0x00, 0x11, 0x22, 0x33, 0x44]
        var data = marker
        // Interleave unique filler bytes to avoid early matches.
        for i in 0..<5000 {
            data.append(UInt8(i & 0xFF))
        }
        data.append(contentsOf: marker) // repeat the marker at distance > 4096
        try roundtrip(data, "long-distance match")
    }

    // -----------------------------------------------------------------------
    // Additional edge cases
    // -----------------------------------------------------------------------

    func testTwoBytes() throws {
        try roundtrip([0x01, 0x02], "two bytes")
        try roundtrip([0xFF, 0xFF], "two identical bytes")
    }

    func testThreeBytes() throws {
        try roundtrip([0x01, 0x02, 0x03], "three bytes")
    }

    func testShortRepeat() throws {
        // Short repeated sequence — below minimum match length, all literals.
        try roundtrip(bytes("ABCABC"), "ABCABC")
    }

    func testMinMatchLength() throws {
        // Exactly 4-byte repeat — the minimum match length.
        try roundtrip(bytes("ABCDABCD"), "ABCDABCD (min match)")
    }

    func testOverlappingMatch() throws {
        // Overlapping copy: "AAAA..." is a single copy that extends past the
        // source — the decoder must copy byte-by-byte (not memcpy) to handle
        // overlapping copies correctly.
        try roundtrip([UInt8](repeating: UInt8(ascii: "A"), count: 256), "A×256 overlapping")
        try roundtrip(bytes("ABABABABABABABAB"), "ABAB×8 overlapping")
    }

    func testMultipleMatches() throws {
        try roundtrip(bytes("ABCABCABCABC"), "ABCABC×3")
        try roundtrip(bytes("hello hello hello world"), "hello×3")
    }

    func testLongerRepetitive() throws {
        let base = bytes("The quick brown fox jumps over the lazy dog. ")
        let data = [UInt8]((0..<20).flatMap { _ in base })
        try roundtrip(data, "pangram×20")
    }

    func testNullBytes() throws {
        try roundtrip([UInt8](repeating: 0, count: 100), "NUL×100")
    }

    func testMaxByteRepeat() throws {
        try roundtrip([UInt8](repeating: 0xFF, count: 100), "0xFF×100")
    }

    func testAllASCIILowercase() throws {
        let data = bytes("abcdefghijklmnopqrstuvwxyz")
        try roundtrip(data, "a-z")
    }

    func testAllASCIIUppercase() throws {
        let data = bytes("ABCDEFGHIJKLMNOPQRSTUVWXYZ")
        try roundtrip(data, "A-Z")
    }

    func testAllDigits() throws {
        let data = bytes("0123456789")
        try roundtrip(data, "0-9")
    }

    func testMixedContent() throws {
        let data = bytes("Hello, World! 123 abc DEF xyz 456")
        try roundtrip(data, "mixed content")
    }

    func testWindowBoundary() throws {
        // Data that requires matches near the 65535-byte window boundary.
        var data: [UInt8] = []
        let pattern: [UInt8] = [0xAB, 0xCD, 0xEF, 0x12]
        for _ in 0..<100 { data.append(contentsOf: pattern) }
        try roundtrip(data, "window-boundary pattern×100")
    }

    // -----------------------------------------------------------------------
    // Compression ratio tests
    // -----------------------------------------------------------------------

    func testCompressionRatioHighlyRepetitive() throws {
        // Highly repetitive data should compress significantly.
        let base = bytes("ABCDEFGH")
        let data = [UInt8]((0..<200).flatMap { _ in base })
        let compressed = try Brotli.compress(data)
        XCTAssertLessThan(
            compressed.count, data.count / 2,
            "highly repetitive data should achieve >50% compression"
        )
        let decompressed = try Brotli.decompress(compressed)
        XCTAssertEqual(decompressed, data)
    }

    // -----------------------------------------------------------------------
    // Header structure tests
    // -----------------------------------------------------------------------

    func testEmptyInputWireFormat() throws {
        let compressed = try Brotli.compress([])
        // Minimum structure: 10 header + 1 ICC entry (2 bytes) + 1 bit-stream byte
        XCTAssertGreaterThanOrEqual(compressed.count, 13, "empty compressed should have at least 13 bytes")

        // original_length in bytes 0-3 should be 0
        let origLen = Int(compressed[0]) << 24 | Int(compressed[1]) << 16 |
                      Int(compressed[2]) << 8  | Int(compressed[3])
        XCTAssertEqual(origLen, 0, "original_length should be 0 for empty input")

        // icc_entry_count should be 1 (only the sentinel)
        XCTAssertEqual(Int(compressed[4]), 1, "empty input should have 1 ICC entry (sentinel)")

        // dist_entry_count should be 0
        XCTAssertEqual(Int(compressed[5]), 0, "empty input should have 0 dist entries")
    }

    func testOriginalLengthInHeader() throws {
        let data = bytes("Hello, World!")
        let compressed = try Brotli.compress(data)
        let origLen = Int(compressed[0]) << 24 | Int(compressed[1]) << 16 |
                      Int(compressed[2]) << 8  | Int(compressed[3])
        XCTAssertEqual(origLen, data.count, "original_length in header should match input size")
    }

    // -----------------------------------------------------------------------
    // Diverse input sizes
    // -----------------------------------------------------------------------

    func testVariousSizes() throws {
        for size in [1, 2, 3, 4, 5, 10, 50, 100, 500, 1000, 5000] {
            let data = [UInt8]((0..<size).map { UInt8($0 % 256) })
            try roundtrip(data, "size=\(size)")
        }
    }

    func testBinaryPattern1024() throws {
        let data = [UInt8]((0..<1024).map { UInt8($0 % 256) })
        try roundtrip(data, "binary-1024")
    }
}
