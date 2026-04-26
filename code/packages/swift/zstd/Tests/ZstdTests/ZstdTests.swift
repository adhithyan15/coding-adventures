// ZstdTests.swift — Unit tests for the Zstd Swift package (CMP07).
//
// Test strategy:
//   TC-1  Empty round-trip          — empty input is the degenerate case.
//   TC-2  Single byte               — smallest non-empty input.
//   TC-3  All 256 byte values       — exercises non-ASCII and zero bytes.
//   TC-4  RLE block                 — 1024 identical bytes → RLE block.
//   TC-5  English prose             — repetitive text compresses > 20%.
//   TC-6  LCG random data           — pseudo-random; round-trip exact.
//   TC-7  200 KB single-byte run    — multi-block (crosses MAX_BLOCK_SIZE).
//   TC-8  300 KB repetitive text    — multi-block + strong matches.
//   TC-9  Bad magic                 — throws ZstdError.badMagic.
//   TC-10 Deterministic output      — same input always gives same bytes.
//   TC-11 All-zeros round-trip      — 1 KB of zeros.
//   TC-12 All-0xFF round-trip       — 1 KB of 0xFF.
//   TC-13 Binary ramp round-trip    — bytes 0..255 repeated.
//   TC-14 Alternating pattern       — 'X' runs + "ABCDEFGH"; compression > 30%.
//   TC-15 Wire-format decode        — manually crafted frame (no encoder dependency).

import XCTest
@testable import Zstd

final class ZstdTests: XCTestCase {

    // MARK: - Round-trip helper

    /// Compress then decompress; asserts equality and returns the compressed bytes.
    @discardableResult
    private func rt(_ data: [UInt8], file: StaticString = #file, line: UInt = #line) throws -> [UInt8] {
        let compressed = compress(data)
        let recovered = try decompress(compressed)
        XCTAssertEqual(recovered, data, "Round-trip mismatch", file: file, line: line)
        return compressed
    }

    // =========================================================================
    // TC-1: Empty round-trip
    // =========================================================================
    //
    // An empty input must produce a valid ZStd frame and decompress back to an
    // empty byte array without panicking or throwing.
    //
    // Expected frame layout for empty input:
    //   4 bytes magic + 1 byte FHD + 8 bytes FCS(0) + 3 bytes empty-raw-block header
    //   = 16 bytes total.

    func testTC1Empty() throws {
        let data: [UInt8] = []
        let compressed = compress(data)
        XCTAssertFalse(compressed.isEmpty, "Compressed output of empty input must not be empty (frame header is required)")
        let recovered = try decompress(compressed)
        XCTAssertEqual(recovered, data, "Empty round-trip failed")
    }

    // =========================================================================
    // TC-2: Single byte round-trip
    // =========================================================================
    //
    // The smallest non-empty input: exactly one byte.  This exercises the
    // single-byte literal path (no LZ77 matches possible).

    func testTC2SingleByte() throws {
        try rt([0x42])
    }

    // =========================================================================
    // TC-3: All 256 byte values
    // =========================================================================
    //
    // Every byte value 0x00..=0xFF in ascending order.  This exercises the
    // literal encoding of non-ASCII and null bytes, and ensures the decoder
    // handles all byte values correctly.

    func testTC3AllBytes() throws {
        let data = [UInt8](0...255)
        try rt(data)
    }

    // =========================================================================
    // TC-4: RLE block
    // =========================================================================
    //
    // 1024 identical bytes ('A') should be detected as an all-same block and
    // encoded as a single RLE block.
    //
    // Expected compressed size:
    //   4 (magic) + 1 (FHD) + 8 (FCS) + 3 (block header) + 1 (RLE byte) = 17 bytes.
    // Certainly < 30 bytes.

    func testTC4RLE() throws {
        let data = [UInt8](repeating: UInt8(ascii: "A"), count: 1024)
        let compressed = compress(data)
        let recovered = try decompress(compressed)
        XCTAssertEqual(recovered, data, "RLE round-trip failed")
        XCTAssertLessThan(
            compressed.count, 30,
            "RLE of 1024 identical bytes should compress to < 30 bytes, got \(compressed.count)")
    }

    // =========================================================================
    // TC-5: English prose compression ratio
    // =========================================================================
    //
    // Repeated English text has strong LZ77 matches and should achieve at least
    // 20% compression (output ≤ 80% of input).

    func testTC5EnglishProse() throws {
        let text = String(repeating: "the quick brown fox jumps over the lazy dog ", count: 25)
        let data = Array(text.utf8)
        let compressed = try rt(data)
        let threshold = data.count * 80 / 100
        XCTAssertLessThan(
            compressed.count, threshold,
            "Prose compression: expected < \(threshold) bytes (80% of \(data.count)), got \(compressed.count)")
    }

    // =========================================================================
    // TC-6: LCG pseudo-random data
    // =========================================================================
    //
    // A Linear Congruential Generator produces bytes with no exploitable
    // repetition.  No significant compression is expected, but the round-trip
    // must be exactly correct regardless of which block type is chosen.
    //
    // LCG parameters: multiplier=1664525, increment=1013904223 (Numerical Recipes).

    func testTC6RandomData() throws {
        var seed: UInt32 = 42
        var data = [UInt8]()
        for _ in 0..<512 {
            seed = seed &* 1664525 &+ 1013904223
            data.append(UInt8(seed & 0xFF))
        }
        try rt(data)
    }

    // =========================================================================
    // TC-7: 200 KB single-byte run (multi-block)
    // =========================================================================
    //
    // 200 KB > maxBlockSize (128 KB), so this requires at least 2 blocks.
    // Both blocks are all the same byte, so both should be encoded as RLE blocks.
    // Final compressed size should be tiny (just a few dozen bytes).

    func testTC7MultiblockRLE() throws {
        let data = [UInt8](repeating: UInt8(ascii: "x"), count: 200 * 1024)
        let compressed = try rt(data)
        XCTAssertLessThan(
            compressed.count, 100,
            "200 KB of identical bytes should compress to < 100 bytes (multiple RLE blocks)")
    }

    // =========================================================================
    // TC-8: 300 KB repetitive text (multi-block)
    // =========================================================================
    //
    // Large repetitive text exercises both multi-block handling and LZ77 match
    // effectiveness.  Compression ratio should exceed 50%.

    func testTC8RepetitiveText() throws {
        let sentence = "ZStd is a fast, lossless compression algorithm using FSE and LZ77. "
        let text = String(repeating: sentence, count: 4500)  // ~≥300 KB
        let data = Array(text.utf8)
        let compressed = try rt(data)
        let threshold = data.count / 2
        XCTAssertLessThan(
            compressed.count, threshold,
            "Repetitive text should compress to < 50% of original; got \(compressed.count)/\(data.count)")
    }

    // =========================================================================
    // TC-9: Bad magic throws
    // =========================================================================
    //
    // A frame whose first 4 bytes are not 0xFD2FB528 must throw badMagic.

    func testTC9BadMagic() {
        let badFrame: [UInt8] = [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01, 0x02]
        XCTAssertThrowsError(try decompress(badFrame)) { error in
            guard case ZstdError.badMagic = error else {
                XCTFail("Expected ZstdError.badMagic, got \(error)")
                return
            }
        }
    }

    // =========================================================================
    // TC-10: Deterministic output
    // =========================================================================
    //
    // Compressing the same data twice must produce bit-for-bit identical output.
    // This is a critical requirement for reproducible builds and caching.

    func testTC10Deterministic() {
        let data = Array(String(repeating: "hello, ZStd world! ", count: 50).utf8)
        XCTAssertEqual(compress(data), compress(data), "Compression must be deterministic")
    }

    // =========================================================================
    // TC-11: All-zeros round-trip
    // =========================================================================

    func testTC11AllZeros() throws {
        let data = [UInt8](repeating: 0, count: 1000)
        try rt(data)
    }

    // =========================================================================
    // TC-12: All-0xFF round-trip
    // =========================================================================

    func testTC12AllFF() throws {
        let data = [UInt8](repeating: 0xFF, count: 1000)
        try rt(data)
    }

    // =========================================================================
    // TC-13: Binary ramp round-trip
    // =========================================================================
    //
    // Bytes 0..255 repeated across 300 bytes.  Exercises wrapping values and
    // ensures the encoder/decoder handle every byte value in a multi-byte stream.

    func testTC13BinaryRamp() throws {
        let data = [UInt8]((0..<300).map { UInt8($0 % 256) })
        try rt(data)
    }

    // =========================================================================
    // TC-14: Alternating pattern with long runs (compression ratio check)
    // =========================================================================
    //
    // Alternating runs of 'X' and the pattern "ABCDEFGH" produce both RLE-
    // amenable runs and LZ77-amenable repetition.

    func testTC14AlternatingPattern() throws {
        let pattern = Array("ABCDEFGH".utf8)
        var data = pattern
        for _ in 0..<10 {
            data.append(contentsOf: [UInt8](repeating: UInt8(ascii: "X"), count: 128))
            data.append(contentsOf: pattern)
        }
        let compressed = try rt(data)
        let threshold = data.count * 70 / 100
        XCTAssertLessThan(
            compressed.count, threshold,
            "Alternating pattern should compress to < 70%; got \(compressed.count)/\(data.count)")
    }

    // =========================================================================
    // TC-15: Wire-format decode (manual frame construction)
    // =========================================================================
    //
    // This test decodes a manually constructed ZStd frame without any dependency
    // on our encoder, verifying the decoder against the spec directly.
    //
    // Frame layout:
    //   [0..3]  Magic   = 0xFD2FB528 LE → [0x28, 0xB5, 0x2F, 0xFD]
    //   [4]     FHD     = 0x20:
    //                     bits [7:6] = 00 → FCS_flag = 0
    //                     bit  [5]   = 1  → Single_Segment = 1
    //                     bits [4:0] = 0  → no checksum, no dict
    //                   With Single_Segment=1 and FCS_flag=00, FCS is 1 byte.
    //   [5]     FCS     = 0x05 (content_size = 5)
    //   [6..8]  Block header: Last=1, Type=Raw(00), Size=5
    //                     = (5 << 3) | (0 << 1) | 1 = 41 = 0x29
    //                     → [0x29, 0x00, 0x00]
    //   [9..13] b"hello"

    func testTC15WireFormat() throws {
        let frame: [UInt8] = [
            0x28, 0xB5, 0x2F, 0xFD,   // magic
            0x20,                       // FHD: Single_Segment=1, FCS=1byte
            0x05,                       // FCS = 5
            0x29, 0x00, 0x00,           // block: last=1, raw, size=5
            UInt8(ascii: "h"), UInt8(ascii: "e"), UInt8(ascii: "l"),
            UInt8(ascii: "l"), UInt8(ascii: "o"),
        ]
        let recovered = try decompress(frame)
        XCTAssertEqual(recovered, Array("hello".utf8), "Wire-format decode failed")
    }

    // =========================================================================
    // Additional regression / edge-case tests
    // =========================================================================

    /// A short greeting — exercises the path where LZ77 finds no useful matches.
    func testRTHelloWorld() throws {
        try rt(Array("hello world".utf8))
    }

    /// Bytes 0-255 in a 300-element cycle.
    func testRTCyclicBytes() throws {
        let data = [UInt8]((0..<300).map { UInt8($0 % 256) })
        try rt(data)
    }

    /// Six-byte cycle repeated to 3000 bytes — strong LZ77 matches.
    func testRTRepeatedPattern() throws {
        let cycle: [UInt8] = [65, 66, 67, 68, 69, 70]  // "ABCDEF"
        let data = [UInt8]((0..<3000).map { cycle[$0 % cycle.count] })
        try rt(data)
    }

    /// A frame that is too short must throw frameTooShort.
    func testFrameTooShort() {
        XCTAssertThrowsError(try decompress([0x28, 0xB5])) { error in
            guard case ZstdError.frameTooShort = error else {
                XCTFail("Expected ZstdError.frameTooShort, got \(error)")
                return
            }
        }
    }

    /// Verify that compress does not grow a large random input by more than 5%.
    func testRandomDataOverhead() throws {
        var seed: UInt32 = 0xDEAD_BEEF
        var data = [UInt8]()
        data.reserveCapacity(4096)
        for _ in 0..<4096 {
            seed = seed &* 1664525 &+ 1013904223
            data.append(UInt8(seed & 0xFF))
        }
        let compressed = try rt(data)
        // Random data is incompressible; allow 5% overhead for frame + headers.
        let maxAllowed = data.count + data.count / 20 + 20
        XCTAssertLessThan(
            compressed.count, maxAllowed,
            "Random data should not expand by more than 5%")
    }

    /// A non-trivial sentence round-trips correctly.
    func testRTSentence() throws {
        let s = "The Zstandard algorithm (RFC 8878) achieves high compression ratios " +
                "using Finite State Entropy coding and LZ77 back-references."
        try rt(Array(s.utf8))
    }
}
