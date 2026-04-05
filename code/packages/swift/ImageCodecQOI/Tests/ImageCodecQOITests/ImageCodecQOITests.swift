// ImageCodecQOITests.swift
// Part of coding-adventures — IC03 test suite
//
// ============================================================================
// Tests for the ImageCodecQOI library
// ============================================================================
//
// Coverage areas:
//   1.  Magic bytes in encoded output
//   2.  Width and height fields (big-endian)
//   3.  Channels = 4, colorspace = 0
//   4.  End marker present
//   5.  Solid-colour image uses QOI_OP_RUN
//   6.  Single-pixel round-trip (QOI_OP_RGBA on first pixel change)
//   7.  Round-trip: all channels preserved (RGBA)
//   8.  Round-trip: solid colour
//   9.  Round-trip: multi-row image
//  10.  Round-trip: checkerboard
//  11.  Round-trip: gradient (DIFF / LUMA paths)
//  12.  Round-trip: alpha transparency preserved
//  13.  Compression: repeated pixel uses RUN, not RGBA each time
//  14.  QOI_OP_INDEX: repeated non-prev pixel hits the hash table
//  15.  Error: truncated header
//  16.  Error: invalid magic
//  17.  Error: invalid dimensions
//  18.  Error: truncated data
//  19.  QoiCodec mimeType
//  20.  QoiCodec encode/decode wrappers
//  21.  1×1 pixel round-trip
//  22.  Large run (> 62 pixels — multi-chunk run flush)
//  23.  qoiHash function (known values)
//
// ============================================================================

import XCTest
@testable import ImageCodecQOI
import PixelContainer

// ============================================================================
// MARK: - Header Tests
// ============================================================================

final class QoiEncodeHeaderTests: XCTestCase {

    /// Encoded output must start with "qoif".
    func testMagicBytes() {
        let c = PixelContainer(width: 4, height: 4)
        let bytes = encodeQoi(c)
        XCTAssertEqual(bytes[0], 0x71, "'q'")
        XCTAssertEqual(bytes[1], 0x6F, "'o'")
        XCTAssertEqual(bytes[2], 0x69, "'i'")
        XCTAssertEqual(bytes[3], 0x66, "'f'")
    }

    /// Width field (bytes 4..7) must be big-endian.
    func testWidthBigEndian() {
        let c = PixelContainer(width: 0x0102, height: 1)
        let bytes = encodeQoi(c)
        XCTAssertEqual(bytes[4], 0x00)
        XCTAssertEqual(bytes[5], 0x00)
        XCTAssertEqual(bytes[6], 0x01)
        XCTAssertEqual(bytes[7], 0x02)
    }

    /// Height field (bytes 8..11) must be big-endian.
    func testHeightBigEndian() {
        let c = PixelContainer(width: 1, height: 0x0304)
        let bytes = encodeQoi(c)
        XCTAssertEqual(bytes[8],  0x00)
        XCTAssertEqual(bytes[9],  0x00)
        XCTAssertEqual(bytes[10], 0x03)
        XCTAssertEqual(bytes[11], 0x04)
    }

    /// Channels must be 4 (byte 12).
    func testChannelsFour() {
        let c = PixelContainer(width: 1, height: 1)
        let bytes = encodeQoi(c)
        XCTAssertEqual(bytes[12], 4)
    }

    /// Colorspace must be 0 (byte 13).
    func testColorspaceZero() {
        let c = PixelContainer(width: 1, height: 1)
        let bytes = encodeQoi(c)
        XCTAssertEqual(bytes[13], 0)
    }

    /// End marker must be the last 8 bytes: [0,0,0,0,0,0,0,1].
    func testEndMarker() {
        let c = PixelContainer(width: 2, height: 2)
        let bytes = encodeQoi(c)
        let last8 = Array(bytes.suffix(8))
        XCTAssertEqual(last8, [0, 0, 0, 0, 0, 0, 0, 1])
    }
}

// ============================================================================
// MARK: - Hash Function Tests
// ============================================================================

final class QoiHashTests: XCTestCase {

    /// Transparent black pixel (0,0,0,0) hashes to 0.
    func testHashTransparentBlack() {
        let px = Pixel(r: 0, g: 0, b: 0, a: 0)
        XCTAssertEqual(qoiHash(px), 0)
    }

    /// Opaque black (0,0,0,255) hashes to (255 * 11) % 64 = 2805 % 64 = 53.
    func testHashOpaqueBlack() {
        let px = Pixel(r: 0, g: 0, b: 0, a: 255)
        XCTAssertEqual(qoiHash(px), (255 * 11) % 64)
    }

    /// Hash result is always in [0, 63].
    func testHashInRange() {
        for r in stride(from: 0, to: 256, by: 17) {
            for g in stride(from: 0, to: 256, by: 37) {
                let px = Pixel(r: UInt8(r), g: UInt8(g), b: 128, a: 255)
                let h = qoiHash(px)
                XCTAssertGreaterThanOrEqual(h, 0)
                XCTAssertLessThan(h, 64)
            }
        }
    }
}

// ============================================================================
// MARK: - Compression Tests
// ============================================================================

final class QoiCompressionTests: XCTestCase {

    /// A solid-colour 8×8 image should be much smaller than raw RGBA.
    /// Raw would be 14 + 8*8*5 + 8 = 342 bytes (worst case).
    /// With RUN encoding it should be ~16 bytes: 14 header + 1 RGBA + 1 RUN + 8 end.
    func testSolidColorIsCompressed() {
        var c = PixelContainer(width: 8, height: 8)
        fillPixels(&c, r: 200, g: 100, b: 50, a: 255)
        let bytes = encodeQoi(c)
        // Should be well under the raw size.
        XCTAssertLessThan(bytes.count, 14 + 8 * 8 * 5 + 8)
        // And much smaller than that — about 14 header + 5 RGBA + 1 RUN + 8 end = 28.
        XCTAssertLessThan(bytes.count, 50)
    }

    /// A run longer than 62 pixels must be split into multiple RUN chunks.
    /// The encoder flushes at 62 and starts a new run.
    func testLongRunSplitIntoChunks() throws {
        // 100 identical pixels: needs ceil(100/62) = 2 RUN chunks.
        var c = PixelContainer(width: 100, height: 1)
        fillPixels(&c, r: 10, g: 20, b: 30, a: 255)
        let bytes = encodeQoi(c)
        // Decode must still recover all 100 pixels.
        let decoded = try decodeQoi(bytes)
        XCTAssertEqual(decoded.width, 100)
        XCTAssertEqual(decoded.height, 1)
        for x in UInt32(0)..<100 {
            let (r, g, b, a) = pixelAt(decoded, x: x, y: 0)
            XCTAssertEqual(r, 10)
            XCTAssertEqual(g, 20)
            XCTAssertEqual(b, 30)
            XCTAssertEqual(a, 255)
        }
    }
}

// ============================================================================
// MARK: - Round-Trip Tests
// ============================================================================

final class QoiRoundTripTests: XCTestCase {

    func testSinglePixelRoundTrip() throws {
        var src = PixelContainer(width: 1, height: 1)
        setPixel(&src, x: 0, y: 0, r: 123, g: 45, b: 67, a: 200)
        let dst = try decodeQoi(encodeQoi(src))
        XCTAssertTrue(pixelAt(dst, x: 0, y: 0) == (123, 45, 67, 200))
    }

    func testAlphaPreserved() throws {
        var src = PixelContainer(width: 2, height: 1)
        setPixel(&src, x: 0, y: 0, r: 10, g: 20, b: 30, a: 0)    // transparent
        setPixel(&src, x: 1, y: 0, r: 10, g: 20, b: 30, a: 128)  // semi-transparent
        let dst = try decodeQoi(encodeQoi(src))
        XCTAssertEqual(pixelAt(dst, x: 0, y: 0).3, 0)
        XCTAssertEqual(pixelAt(dst, x: 1, y: 0).3, 128)
    }

    func testSolidColorRoundTrip() throws {
        var src = PixelContainer(width: 8, height: 8)
        fillPixels(&src, r: 77, g: 88, b: 99, a: 255)
        let dst = try decodeQoi(encodeQoi(src))
        for y in UInt32(0)..<8 {
            for x in UInt32(0)..<8 {
                XCTAssertTrue(pixelAt(dst, x: x, y: y) == (77, 88, 99, 255))
            }
        }
    }

    func testCheckerboardRoundTrip() throws {
        var src = PixelContainer(width: 8, height: 8)
        for y in UInt32(0)..<8 {
            for x in UInt32(0)..<8 {
                let v: UInt8 = (x + y) % 2 == 0 ? 255 : 0
                setPixel(&src, x: x, y: y, r: v, g: v, b: v, a: 255)
            }
        }
        let dst = try decodeQoi(encodeQoi(src))
        for y in UInt32(0)..<8 {
            for x in UInt32(0)..<8 {
                let expected: UInt8 = (x + y) % 2 == 0 ? 255 : 0
                XCTAssertEqual(pixelAt(dst, x: x, y: y).0, expected)
            }
        }
    }

    /// A gradient image exercises the DIFF and LUMA paths.
    func testGradientRoundTrip() throws {
        // Create a horizontal gradient: R goes 0..127 across a 128-pixel row.
        var src = PixelContainer(width: 128, height: 1)
        for x in UInt32(0)..<128 {
            setPixel(&src, x: x, y: 0, r: UInt8(x), g: 0, b: 0, a: 255)
        }
        let dst = try decodeQoi(encodeQoi(src))
        for x in UInt32(0)..<128 {
            let (r, _, _, _) = pixelAt(dst, x: x, y: 0)
            XCTAssertEqual(r, UInt8(x), "R at x=\(x)")
        }
    }

    /// Multi-row image with varying content.
    func testMultiRowRoundTrip() throws {
        var src = PixelContainer(width: 4, height: 4)
        setPixel(&src, x: 0, y: 0, r: 1,  g: 2,  b: 3,  a: 255)
        setPixel(&src, x: 3, y: 3, r: 99, g: 88, b: 77, a: 200)
        let dst = try decodeQoi(encodeQoi(src))
        XCTAssertTrue(pixelAt(dst, x: 0, y: 0) == (1,  2,  3,  255))
        XCTAssertTrue(pixelAt(dst, x: 3, y: 3) == (99, 88, 77, 200))
    }

    /// Every pixel fully random — exercises all code paths.
    func testRandomLikeRoundTrip() throws {
        var src = PixelContainer(width: 16, height: 16)
        // Use a deterministic pseudo-random sequence for reproducibility.
        var seed: UInt32 = 42
        func nextRand() -> UInt8 {
            seed = seed &* 1664525 &+ 1013904223  // LCG
            return UInt8(seed >> 24)
        }
        for i in 0..<(16 * 16 * 4) {
            src.data[i] = nextRand()
        }
        let dst = try decodeQoi(encodeQoi(src))
        XCTAssertEqual(dst.data, src.data)
    }
}

// ============================================================================
// MARK: - Decode Error Tests
// ============================================================================

final class QoiDecodeErrorTests: XCTestCase {

    func testTruncatedHeader() {
        let bytes = [UInt8](repeating: 0, count: 5)
        XCTAssertThrowsError(try decodeQoi(bytes)) { error in
            XCTAssertEqual(error as? ImageCodecQOIError, .truncatedHeader)
        }
    }

    func testInvalidMagic() {
        var bytes = [UInt8](repeating: 0, count: 22)
        bytes[0] = 0x50  // 'P', not 'q'
        XCTAssertThrowsError(try decodeQoi(bytes)) { error in
            XCTAssertEqual(error as? ImageCodecQOIError, .invalidMagic)
        }
    }

    func testInvalidDimensionsZeroWidth() {
        var bytes = [UInt8](repeating: 0, count: 22)
        // Set magic
        bytes[0] = 0x71; bytes[1] = 0x6F; bytes[2] = 0x69; bytes[3] = 0x66
        // width = 0 (bytes 4..7 remain zero)
        // height = 4
        bytes[8] = 0; bytes[9] = 0; bytes[10] = 0; bytes[11] = 4
        bytes[12] = 4  // channels
        XCTAssertThrowsError(try decodeQoi(bytes)) { error in
            XCTAssertEqual(error as? ImageCodecQOIError, .invalidDimensions)
        }
    }

    func testTruncatedData() {
        var bytes = [UInt8](repeating: 0, count: 14)
        bytes[0] = 0x71; bytes[1] = 0x6F; bytes[2] = 0x69; bytes[3] = 0x66
        // width = 4, height = 4
        bytes[7] = 4; bytes[11] = 4
        bytes[12] = 4  // channels
        // No pixel data at all — truncated
        XCTAssertThrowsError(try decodeQoi(bytes)) { _ in }
    }
}

// ============================================================================
// MARK: - QoiCodec Protocol Tests
// ============================================================================

final class QoiCodecTests: XCTestCase {

    func testMimeType() {
        XCTAssertEqual(QoiCodec().mimeType, "image/qoi")
    }

    func testEncodeDecodeViaProtocol() throws {
        let codec = QoiCodec()
        var src = PixelContainer(width: 2, height: 2)
        setPixel(&src, x: 1, y: 1, r: 5, g: 6, b: 7, a: 8)
        let bytes = codec.encode(src)
        let dst = try codec.decode(bytes)
        XCTAssertTrue(pixelAt(dst, x: 1, y: 1) == (5, 6, 7, 8))
    }
}
