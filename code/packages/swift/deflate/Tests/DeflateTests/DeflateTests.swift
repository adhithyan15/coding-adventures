// DeflateTests.swift — Tests for CMP05 DEFLATE compression

import XCTest
@testable import Deflate

final class DeflateTests: XCTestCase {

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    func roundtrip(_ data: [UInt8], _ label: String = "data") throws {
        let compressed   = try Deflate.compress(data)
        let decompressed = try Deflate.decompress(compressed)
        XCTAssertEqual(decompressed, data, "roundtrip mismatch for \(label)")
    }

    func fromString(_ s: String) -> [UInt8] {
        Array(s.utf8)
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    func testEmpty() throws {
        let compressed   = try Deflate.compress([])
        let decompressed = try Deflate.decompress(compressed)
        XCTAssertEqual(decompressed, [])
    }

    func testSingleByteNull() throws {
        try roundtrip([0x00], "NUL")
    }

    func testSingleByteFF() throws {
        try roundtrip([0xFF], "0xFF")
    }

    func testSingleByteA() throws {
        try roundtrip(fromString("A"), "A")
    }

    func testSingleByteRepeated() throws {
        try roundtrip([UInt8](repeating: 65, count: 20), "A×20")
        try roundtrip([UInt8](repeating: 0, count: 100), "NUL×100")
    }

    // -----------------------------------------------------------------------
    // Spec examples
    // -----------------------------------------------------------------------

    func testAABBCAllLiterals() throws {
        let data = fromString("AAABBC")
        try roundtrip(data, "AAABBC")
        let compressed = try Deflate.compress(data)
        let distCount = Int(UInt16(compressed[6]) << 8 | UInt16(compressed[7]))
        XCTAssertEqual(distCount, 0, "expected dist_entry_count=0 for all-literals input")
    }

    func testAABCBBABCOneMatch() throws {
        let data = fromString("AABCBBABC")
        try roundtrip(data, "AABCBBABC")
        let compressed = try Deflate.compress(data)
        let origLen = Int(
            UInt32(compressed[0]) << 24 | UInt32(compressed[1]) << 16 |
            UInt32(compressed[2]) << 8  | UInt32(compressed[3])
        )
        let distCount = Int(UInt16(compressed[6]) << 8 | UInt16(compressed[7]))
        XCTAssertEqual(origLen, 9)
        XCTAssertGreaterThan(distCount, 0, "expected a match")
    }

    // -----------------------------------------------------------------------
    // Match tests
    // -----------------------------------------------------------------------

    func testOverlappingMatch() throws {
        try roundtrip(fromString("AAAAAAA"), "run of A")
        try roundtrip(fromString("ABABABABABAB"), "ABAB run")
    }

    func testMultipleMatches() throws {
        try roundtrip(fromString("ABCABCABCABC"), "ABCABC×3")
        try roundtrip(fromString("hello hello hello world"), "hello×3")
    }

    func testMaxMatchLength() throws {
        try roundtrip([UInt8](repeating: 65, count: 300), "A×300")
    }

    // -----------------------------------------------------------------------
    // Data variety
    // -----------------------------------------------------------------------

    func testAllByteValues() throws {
        let data = [UInt8](0...255)
        try roundtrip(data, "all-bytes")
    }

    func testBinaryData1000Bytes() throws {
        let data = [UInt8]((0..<1000).map { UInt8($0 % 256) })
        try roundtrip(data, "binary-1000")
    }

    func testLongerTextWithRepetition() throws {
        let base = fromString("the quick brown fox jumps over the lazy dog ")
        let data = [UInt8]((0..<10).flatMap { _ in base })
        try roundtrip(data, "pangram×10")
    }

    // -----------------------------------------------------------------------
    // Compression ratio
    // -----------------------------------------------------------------------

    func testCompressionRatio() throws {
        let base = fromString("ABCABC")
        let data = [UInt8]((0..<100).flatMap { _ in base })
        let compressed = try Deflate.compress(data)
        XCTAssertLessThan(
            compressed.count, data.count / 2,
            "expected significant compression: \(compressed.count) >= \(data.count)/2"
        )
    }

    // -----------------------------------------------------------------------
    // Various match lengths
    // -----------------------------------------------------------------------

    func testVariousMatchLengths() throws {
        for length in [3, 4, 10, 11, 13, 19, 35, 67, 131, 227, 255] {
            let prefix = [UInt8](repeating: 65, count: length)
            let separator = fromString("BBB")
            let data = prefix + separator + prefix
            try roundtrip(data, "length=\(length)")
        }
    }

    // -----------------------------------------------------------------------
    // Diverse round-trips
    // -----------------------------------------------------------------------

    func testDiverseInputs() throws {
        let inputs: [[UInt8]] = [
            [UInt8](repeating: 0, count: 100),
            [UInt8](repeating: 0xFF, count: 100),
            fromString("abcdefghijklmnopqrstuvwxyz"),
            [UInt8]((0..<20).flatMap { _ in fromString("The quick brown fox ") }),
        ]
        for (i, data) in inputs.enumerated() {
            try roundtrip(data, "diverse-\(i)")
        }
    }
}
