// ImageCodecBMPTests.swift
// Part of coding-adventures — IC01 test suite
//
// ============================================================================
// Tests for the ImageCodecBMP library
// ============================================================================
//
// Coverage areas:
//   1.  Magic bytes in encoded output
//   2.  File size field matches actual output length
//   3.  Pixel data offset field = 54
//   4.  Width and height fields in DIB header
//   5.  Bits-per-pixel = 24
//   6.  Compression field = 0
//   7.  BGR byte order in pixel data
//   8.  Row stride padding to 4-byte boundary
//   9.  Encode/decode round-trip (RGB channels preserved)
//  10.  fillPixels + round-trip
//  11.  Error: truncated header
//  12.  Error: invalid signature
//  13.  Error: unsupported DIB header size
//  14.  Error: unsupported bit depth
//  15.  Error: unsupported compression
//  16.  Error: invalid dimensions
//  17.  Error: truncated pixel data
//  18.  BmpCodec mimeType
//  19.  BmpCodec encode/decode wrappers
//  20.  1×1 pixel round-trip
//  21.  Single-row wide image (stride padding)
//  22.  Alpha synthesised as 255 on decode
//
// ============================================================================

import XCTest
@testable import ImageCodecBMP
import PixelContainer

// ============================================================================
// MARK: - Helper
// ============================================================================

/// Create a small BMP from a PixelContainer and decode it back.
private func roundTrip(_ pixels: PixelContainer) throws -> PixelContainer {
    let bytes = encodeBmp(pixels)
    return try decodeBmp(bytes)
}

// ============================================================================
// MARK: - Encode Header Tests
// ============================================================================

final class BmpEncodeHeaderTests: XCTestCase {

    /// Encoded bytes must start with the BMP magic "BM".
    func testMagicBytes() {
        let c = PixelContainer(width: 4, height: 4)
        let bytes = encodeBmp(c)
        XCTAssertEqual(bytes[0], 0x42, "First byte must be 'B' (0x42)")
        XCTAssertEqual(bytes[1], 0x4D, "Second byte must be 'M' (0x4D)")
    }

    /// File size field (offset 2, LE uint32) must equal the actual array length.
    func testFileSizeField() {
        let c = PixelContainer(width: 3, height: 5)
        let bytes = encodeBmp(c)
        let fieldSize = Int(readLE32(bytes, at: 2))
        XCTAssertEqual(fieldSize, bytes.count)
    }

    /// Pixel data offset (offset 10) must always be 54.
    func testPixelDataOffset() {
        let c = PixelContainer(width: 7, height: 7)
        let bytes = encodeBmp(c)
        XCTAssertEqual(readLE32(bytes, at: 10), 54)
    }

    /// DIB header size (offset 14) must be 40.
    func testDibHeaderSize() {
        let c = PixelContainer(width: 2, height: 2)
        let bytes = encodeBmp(c)
        XCTAssertEqual(readLE32(bytes, at: 14), 40)
    }

    /// Width field (offset 18, LE int32) must match the container width.
    func testWidthField() {
        let c = PixelContainer(width: 11, height: 7)
        let bytes = encodeBmp(c)
        XCTAssertEqual(readLE32Signed(bytes, at: 18), 11)
    }

    /// Height field (offset 22) must be negative (top-down storage).
    func testHeightFieldIsNegative() {
        let c = PixelContainer(width: 4, height: 8)
        let bytes = encodeBmp(c)
        XCTAssertEqual(readLE32Signed(bytes, at: 22), -8)
    }

    /// Bits-per-pixel field (offset 28) must be 24.
    func testBitsPerPixel() {
        let c = PixelContainer(width: 4, height: 4)
        let bytes = encodeBmp(c)
        XCTAssertEqual(readLE16(bytes, at: 28), 24)
    }

    /// Compression field (offset 30) must be 0 (BI_RGB).
    func testCompressionIsZero() {
        let c = PixelContainer(width: 4, height: 4)
        let bytes = encodeBmp(c)
        XCTAssertEqual(readLE32(bytes, at: 30), 0)
    }

    /// Color planes (offset 26) must be 1.
    func testColorPlanes() {
        let c = PixelContainer(width: 4, height: 4)
        let bytes = encodeBmp(c)
        XCTAssertEqual(readLE16(bytes, at: 26), 1)
    }
}

// ============================================================================
// MARK: - Encode Pixel Data Tests
// ============================================================================

final class BmpEncodePixelTests: XCTestCase {

    /// A single red pixel should be encoded as BGR = (0, 0, 255) at offset 54.
    func testRedPixelBGROrder() {
        var c = PixelContainer(width: 1, height: 1)
        setPixel(&c, x: 0, y: 0, r: 255, g: 0, b: 0, a: 255)
        let bytes = encodeBmp(c)
        // stride for 1px wide = ceil(3/4)*4 = 4 bytes; offset 54..56 = B,G,R
        XCTAssertEqual(bytes[54], 0,   "B should be 0 for red pixel")
        XCTAssertEqual(bytes[55], 0,   "G should be 0 for red pixel")
        XCTAssertEqual(bytes[56], 255, "R should be 255 for red pixel")
    }

    /// A 1px-wide image row must be padded to 4 bytes
    /// (1 pixel × 3 bytes = 3 bytes → padded to 4 bytes → 1 padding byte).
    func testRowPaddingOnePixelWide() {
        let c = PixelContainer(width: 1, height: 1)
        let bytes = encodeBmp(c)
        // Total size = 54 (header) + 4 (padded stride) = 58
        XCTAssertEqual(bytes.count, 58)
    }

    /// A 4px-wide image row needs no padding (4×3 = 12 bytes, already multiple of 4).
    func testNoRowPaddingFourPixelsWide() {
        let c = PixelContainer(width: 4, height: 1)
        let bytes = encodeBmp(c)
        // stride = 12, no padding. Total = 54 + 12 = 66
        XCTAssertEqual(bytes.count, 66)
    }

    /// A 2px-wide image has stride = ceil(6/4)*4 = 8 (2 padding bytes per row).
    func testTwoPixelWideStride() {
        let c = PixelContainer(width: 2, height: 3)
        let bytes = encodeBmp(c)
        // stride = 8; total = 54 + 8*3 = 78
        XCTAssertEqual(bytes.count, 78)
    }
}

// ============================================================================
// MARK: - Round-Trip Tests
// ============================================================================

final class BmpRoundTripTests: XCTestCase {

    /// A single pixel's RGB channels must survive an encode/decode cycle.
    func testSinglePixelRoundTrip() throws {
        var src = PixelContainer(width: 1, height: 1)
        setPixel(&src, x: 0, y: 0, r: 200, g: 100, b: 50, a: 255)
        let dst = try roundTrip(src)
        let (r, g, b, _) = pixelAt(dst, x: 0, y: 0)
        XCTAssertEqual(r, 200)
        XCTAssertEqual(g, 100)
        XCTAssertEqual(b, 50)
    }

    /// Alpha is dropped on encode and synthesised as 255 on decode.
    func testAlphaSynthesisedAs255() throws {
        var src = PixelContainer(width: 1, height: 1)
        setPixel(&src, x: 0, y: 0, r: 10, g: 20, b: 30, a: 128)  // semi-transparent
        let dst = try roundTrip(src)
        let (_, _, _, a) = pixelAt(dst, x: 0, y: 0)
        XCTAssertEqual(a, 255, "BMP 24-bit has no alpha; decode must synthesise 255")
    }

    /// All pixels in a 4×4 checkerboard must be preserved.
    func testCheckerboardRoundTrip() throws {
        var src = PixelContainer(width: 4, height: 4)
        for y in UInt32(0)..<4 {
            for x in UInt32(0)..<4 {
                let isWhite = (x + y) % 2 == 0
                let v: UInt8 = isWhite ? 255 : 0
                setPixel(&src, x: x, y: y, r: v, g: v, b: v, a: 255)
            }
        }
        let dst = try roundTrip(src)
        for y in UInt32(0)..<4 {
            for x in UInt32(0)..<4 {
                let expected: UInt8 = (x + y) % 2 == 0 ? 255 : 0
                let (r, g, b, _) = pixelAt(dst, x: x, y: y)
                XCTAssertEqual(r, expected, "R at (\(x),\(y))")
                XCTAssertEqual(g, expected, "G at (\(x),\(y))")
                XCTAssertEqual(b, expected, "B at (\(x),\(y))")
            }
        }
    }

    /// Filled solid-colour image round-trips correctly.
    func testSolidColorRoundTrip() throws {
        var src = PixelContainer(width: 10, height: 10)
        fillPixels(&src, r: 100, g: 150, b: 200, a: 255)
        let dst = try roundTrip(src)
        for y in UInt32(0)..<10 {
            for x in UInt32(0)..<10 {
                let (r, g, b, a) = pixelAt(dst, x: x, y: y)
                XCTAssertEqual(r, 100)
                XCTAssertEqual(g, 150)
                XCTAssertEqual(b, 200)
                XCTAssertEqual(a, 255)
            }
        }
    }
}

// ============================================================================
// MARK: - Decode Error Tests
// ============================================================================

final class BmpDecodeErrorTests: XCTestCase {

    func testTruncatedHeader() {
        let bytes = [UInt8](repeating: 0, count: 10)  // too short
        XCTAssertThrowsError(try decodeBmp(bytes)) { error in
            XCTAssertEqual(error as? ImageCodecBMPError, .truncatedHeader)
        }
    }

    func testInvalidSignature() {
        var bytes = [UInt8](repeating: 0, count: 54)
        bytes[0] = 0x50  // 'P', not 'B'
        bytes[1] = 0x36
        XCTAssertThrowsError(try decodeBmp(bytes)) { error in
            XCTAssertEqual(error as? ImageCodecBMPError, .invalidSignature)
        }
    }

    func testUnsupportedDibHeader() {
        var bytes = [UInt8](repeating: 0, count: 54)
        bytes[0] = 0x42; bytes[1] = 0x4D  // "BM"
        writeLE32(12, into: &bytes, at: 14)  // DIB header size = 12 (BITMAPCOREHEADER, not 40)
        writeLE32(54, into: &bytes, at: 10)  // pixel data offset
        XCTAssertThrowsError(try decodeBmp(bytes)) { error in
            XCTAssertEqual(error as? ImageCodecBMPError, .unsupportedDibHeader)
        }
    }

    func testUnsupportedBitDepth() {
        var bytes = [UInt8](repeating: 0, count: 54)
        bytes[0] = 0x42; bytes[1] = 0x4D
        writeLE32(40, into: &bytes, at: 14)
        writeLE32(54, into: &bytes, at: 10)
        writeLE16(32, into: &bytes, at: 28)  // 32-bit — we don't support this
        XCTAssertThrowsError(try decodeBmp(bytes)) { error in
            XCTAssertEqual(error as? ImageCodecBMPError, .unsupportedBitDepth)
        }
    }

    func testUnsupportedCompression() {
        var bytes = [UInt8](repeating: 0, count: 54)
        bytes[0] = 0x42; bytes[1] = 0x4D
        writeLE32(40, into: &bytes, at: 14)
        writeLE32(54, into: &bytes, at: 10)
        writeLE16(24, into: &bytes, at: 28)   // bpp = 24
        writeLE32(1, into: &bytes, at: 30)    // compression = 1 (BI_RLE8) — unsupported
        XCTAssertThrowsError(try decodeBmp(bytes)) { error in
            XCTAssertEqual(error as? ImageCodecBMPError, .unsupportedCompression)
        }
    }

    func testInvalidDimensionsZeroWidth() {
        var bytes = [UInt8](repeating: 0, count: 54)
        bytes[0] = 0x42; bytes[1] = 0x4D
        writeLE32(40, into: &bytes, at: 14)
        writeLE32(54, into: &bytes, at: 10)
        writeLE16(24, into: &bytes, at: 28)
        writeLE32(0, into: &bytes, at: 30)    // BI_RGB
        writeLE32Signed(0, into: &bytes, at: 18)   // width = 0
        writeLE32Signed(4, into: &bytes, at: 22)   // height = 4
        XCTAssertThrowsError(try decodeBmp(bytes)) { error in
            XCTAssertEqual(error as? ImageCodecBMPError, .invalidDimensions)
        }
    }
}

// ============================================================================
// MARK: - BmpCodec Protocol Tests
// ============================================================================

final class BmpCodecTests: XCTestCase {

    func testMimeType() {
        XCTAssertEqual(BmpCodec().mimeType, "image/bmp")
    }

    func testEncodeDecodeViaProtocol() throws {
        let codec = BmpCodec()
        var src = PixelContainer(width: 2, height: 2)
        setPixel(&src, x: 0, y: 0, r: 1, g: 2, b: 3, a: 255)
        let bytes = codec.encode(src)
        let dst = try codec.decode(bytes)
        let (r, g, b, _) = pixelAt(dst, x: 0, y: 0)
        XCTAssertEqual(r, 1)
        XCTAssertEqual(g, 2)
        XCTAssertEqual(b, 3)
    }
}

// The little-endian read/write helpers (readLE16, readLE32, readLE32Signed,
// writeLE16, writeLE32, writeLE32Signed) are internal to ImageCodecBMP and
// are made available to this test target via @testable import above.
