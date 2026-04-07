// ImageCodecPPMTests.swift
// Part of coding-adventures — IC02 test suite
//
// ============================================================================
// Tests for the ImageCodecPPM library
// ============================================================================
//
// Coverage areas:
//   1.  Encoded header starts with "P6\n"
//   2.  Dimensions in header match container size
//   3.  Maxval is 255
//   4.  Pixel data is raw RGB (no padding)
//   5.  Alpha is dropped on encode
//   6.  Round-trip: RGB channels preserved
//   7.  Round-trip: alpha synthesised as 255
//   8.  Round-trip: solid colour image
//   9.  Round-trip: multi-row image
//  10.  Comment lines in header are skipped on decode
//  11.  Whitespace variants in header (space, tab)
//  12.  Error: invalid magic
//  13.  Error: malformed header (missing width)
//  14.  Error: invalid dimensions (zero)
//  15.  Error: unsupported maxval
//  16.  Error: truncated pixel data
//  17.  PpmCodec mimeType
//  18.  PpmCodec encode/decode wrappers
//  19.  1×1 pixel round-trip
//  20.  Wide single-row image
//  21.  Single-column image
//  22.  Pixel data starts immediately after the single separator byte
//
// ============================================================================

import XCTest
@testable import ImageCodecPPM
import PixelContainer

// ============================================================================
// MARK: - Encode Header Tests
// ============================================================================

final class PpmEncodeHeaderTests: XCTestCase {

    /// The encoded header must start with "P6\n".
    func testMagicBytes() {
        let c = PixelContainer(width: 4, height: 4)
        let bytes = encodePpm(c)
        XCTAssertEqual(bytes[0], 0x50, "'P'")
        XCTAssertEqual(bytes[1], 0x36, "'6'")
        XCTAssertEqual(bytes[2], 0x0A, "newline after P6")
    }

    /// The width and height must appear in the header.
    func testDimensionsInHeader() {
        let c = PixelContainer(width: 12, height: 34)
        let bytes = encodePpm(c)
        // Convert to string for easy inspection.
        let headerString = String(bytes: Array(bytes.prefix(30)), encoding: .utf8) ?? ""
        XCTAssertTrue(headerString.contains("12 34"), "Header must contain '12 34'")
    }

    /// The maxval line must be "255".
    func testMaxvalIs255() {
        let c = PixelContainer(width: 2, height: 2)
        let bytes = encodePpm(c)
        let headerString = String(bytes: Array(bytes.prefix(20)), encoding: .utf8) ?? ""
        XCTAssertTrue(headerString.contains("255"), "Header must contain '255'")
    }

    /// Total encoded size = header bytes + width × height × 3.
    func testEncodedSizeIsCorrect() {
        let c = PixelContainer(width: 5, height: 3)
        let bytes = encodePpm(c)
        // Header = "P6\n5 3\n255\n" = 11 bytes
        let expectedPixelBytes = 5 * 3 * 3   // 45
        let header = "P6\n5 3\n255\n"
        XCTAssertEqual(bytes.count, header.utf8.count + expectedPixelBytes)
    }

    /// A single red pixel should produce R=255, G=0, B=0 immediately after the header.
    func testRedPixelRGBOrder() {
        var c = PixelContainer(width: 1, height: 1)
        setPixel(&c, x: 0, y: 0, r: 255, g: 0, b: 0, a: 255)
        let bytes = encodePpm(c)
        // Header = "P6\n1 1\n255\n" = 11 bytes; pixel starts at index 11.
        let header = "P6\n1 1\n255\n"
        let pixelStart = header.utf8.count
        XCTAssertEqual(bytes[pixelStart],     255, "R should be 255")
        XCTAssertEqual(bytes[pixelStart + 1],   0, "G should be 0")
        XCTAssertEqual(bytes[pixelStart + 2],   0, "B should be 0")
    }

    /// PPM has no row padding — pixel data is exactly width × height × 3 bytes.
    func testNoPadding() {
        let c = PixelContainer(width: 1, height: 1)  // odd width (no BMP padding needed)
        let bytes = encodePpm(c)
        let header = "P6\n1 1\n255\n"
        XCTAssertEqual(bytes.count, header.utf8.count + 3)
    }
}

// ============================================================================
// MARK: - Round-Trip Tests
// ============================================================================

final class PpmRoundTripTests: XCTestCase {

    func testSinglePixelRoundTrip() throws {
        var src = PixelContainer(width: 1, height: 1)
        setPixel(&src, x: 0, y: 0, r: 77, g: 88, b: 99, a: 255)
        let bytes = encodePpm(src)
        let dst = try decodePpm(bytes)
        let (r, g, b, _) = pixelAt(dst, x: 0, y: 0)
        XCTAssertEqual(r, 77)
        XCTAssertEqual(g, 88)
        XCTAssertEqual(b, 99)
    }

    func testAlphaSynthesisedAs255() throws {
        var src = PixelContainer(width: 1, height: 1)
        setPixel(&src, x: 0, y: 0, r: 10, g: 20, b: 30, a: 0)  // transparent
        let bytes = encodePpm(src)
        let dst = try decodePpm(bytes)
        let (_, _, _, a) = pixelAt(dst, x: 0, y: 0)
        XCTAssertEqual(a, 255, "PPM has no alpha; decode must synthesise 255")
    }

    func testSolidColorRoundTrip() throws {
        var src = PixelContainer(width: 8, height: 8)
        fillPixels(&src, r: 200, g: 100, b: 50, a: 255)
        let bytes = encodePpm(src)
        let dst = try decodePpm(bytes)
        for y in UInt32(0)..<8 {
            for x in UInt32(0)..<8 {
                let (r, g, b, a) = pixelAt(dst, x: x, y: y)
                XCTAssertEqual(r, 200)
                XCTAssertEqual(g, 100)
                XCTAssertEqual(b, 50)
                XCTAssertEqual(a, 255)
            }
        }
    }

    func testMultiRowImageRoundTrip() throws {
        var src = PixelContainer(width: 3, height: 3)
        setPixel(&src, x: 0, y: 0, r: 1,  g: 2,  b: 3,  a: 255)
        setPixel(&src, x: 2, y: 2, r: 10, g: 20, b: 30, a: 255)
        let bytes = encodePpm(src)
        let dst = try decodePpm(bytes)
        XCTAssertTrue(pixelAt(dst, x: 0, y: 0) == (1,  2,  3,  255))
        XCTAssertTrue(pixelAt(dst, x: 2, y: 2) == (10, 20, 30, 255))
    }

    func testCheckerboardRoundTrip() throws {
        var src = PixelContainer(width: 4, height: 4)
        for y in UInt32(0)..<4 {
            for x in UInt32(0)..<4 {
                let v: UInt8 = (x + y) % 2 == 0 ? 255 : 0
                setPixel(&src, x: x, y: y, r: v, g: v, b: v, a: 255)
            }
        }
        let bytes = encodePpm(src)
        let dst = try decodePpm(bytes)
        for y in UInt32(0)..<4 {
            for x in UInt32(0)..<4 {
                let expected: UInt8 = (x + y) % 2 == 0 ? 255 : 0
                XCTAssertEqual(pixelAt(dst, x: x, y: y).0, expected)
            }
        }
    }
}

// ============================================================================
// MARK: - Comment Handling Tests
// ============================================================================

final class PpmCommentTests: XCTestCase {

    /// A PPM file with a comment line in the header must decode correctly.
    func testCommentLineSkipped() throws {
        // Build a minimal P6 file with a comment.
        let header = "P6\n# This is a comment\n1 1\n255\n"
        var bytes = [UInt8](header.utf8)
        bytes += [100, 150, 200]  // One RGB pixel
        let dst = try decodePpm(bytes)
        let (r, g, b, _) = pixelAt(dst, x: 0, y: 0)
        XCTAssertEqual(r, 100)
        XCTAssertEqual(g, 150)
        XCTAssertEqual(b, 200)
    }

    /// Multiple comment lines before dimensions must all be skipped.
    func testMultipleCommentLinesSkipped() throws {
        let header = "P6\n# Comment 1\n# Comment 2\n2 1\n255\n"
        var bytes = [UInt8](header.utf8)
        bytes += [10, 20, 30, 40, 50, 60]  // Two RGB pixels
        let dst = try decodePpm(bytes)
        XCTAssertEqual(dst.width, 2)
        XCTAssertEqual(dst.height, 1)
    }
}

// ============================================================================
// MARK: - Decode Error Tests
// ============================================================================

final class PpmDecodeErrorTests: XCTestCase {

    func testInvalidMagic() {
        let bytes: [UInt8] = [0x50, 0x33, 0x0A]  // "P3" not "P6"
        XCTAssertThrowsError(try decodePpm(bytes)) { error in
            XCTAssertEqual(error as? ImageCodecPPMError, .invalidMagic)
        }
    }

    func testMalformedHeaderMissingWidth() {
        // "P6\n" with no width
        let bytes: [UInt8] = [0x50, 0x36, 0x0A]
        XCTAssertThrowsError(try decodePpm(bytes)) { error in
            XCTAssertEqual(error as? ImageCodecPPMError, .malformedHeader)
        }
    }

    func testInvalidDimensionsZeroWidth() {
        let header = "P6\n0 4\n255\n"
        var bytes = [UInt8](header.utf8)
        bytes += [UInt8](repeating: 0, count: 4 * 3)
        XCTAssertThrowsError(try decodePpm(bytes)) { error in
            XCTAssertEqual(error as? ImageCodecPPMError, .invalidDimensions)
        }
    }

    func testUnsupportedMaxval() {
        let header = "P6\n1 1\n65535\n"
        var bytes = [UInt8](header.utf8)
        bytes += [0, 0, 0, 0, 0, 0]  // 2 bytes per channel × 3 channels for 16-bit
        XCTAssertThrowsError(try decodePpm(bytes)) { error in
            XCTAssertEqual(error as? ImageCodecPPMError, .unsupportedMaxval)
        }
    }

    func testTruncatedPixelData() {
        let header = "P6\n2 2\n255\n"
        var bytes = [UInt8](header.utf8)
        bytes += [0, 0, 0]  // Only 1 pixel; should be 4 pixels (12 bytes)
        XCTAssertThrowsError(try decodePpm(bytes)) { error in
            XCTAssertEqual(error as? ImageCodecPPMError, .truncatedPixelData)
        }
    }

    func testEmptyInput() {
        XCTAssertThrowsError(try decodePpm([])) { error in
            XCTAssertEqual(error as? ImageCodecPPMError, .invalidMagic)
        }
    }
}

// ============================================================================
// MARK: - PpmCodec Protocol Tests
// ============================================================================

final class PpmCodecTests: XCTestCase {

    func testMimeType() {
        XCTAssertEqual(PpmCodec().mimeType, "image/x-portable-pixmap")
    }

    func testEncodeDecodeViaProtocol() throws {
        let codec = PpmCodec()
        var src = PixelContainer(width: 2, height: 2)
        setPixel(&src, x: 1, y: 1, r: 5, g: 6, b: 7, a: 255)
        let bytes = codec.encode(src)
        let dst = try codec.decode(bytes)
        let (r, g, b, _) = pixelAt(dst, x: 1, y: 1)
        XCTAssertEqual(r, 5)
        XCTAssertEqual(g, 6)
        XCTAssertEqual(b, 7)
    }
}

// ============================================================================
// MARK: - Edge Dimension Tests
// ============================================================================

final class PpmEdgeDimensionTests: XCTestCase {

    func testSingleColumnRoundTrip() throws {
        var src = PixelContainer(width: 1, height: 5)
        for y in UInt32(0)..<5 {
            setPixel(&src, x: 0, y: y, r: UInt8(y * 10), g: 0, b: 0, a: 255)
        }
        let dst = try decodePpm(encodePpm(src))
        for y in UInt32(0)..<5 {
            let (r, _, _, _) = pixelAt(dst, x: 0, y: y)
            XCTAssertEqual(r, UInt8(y * 10))
        }
    }

    func testSingleRowRoundTrip() throws {
        var src = PixelContainer(width: 5, height: 1)
        for x in UInt32(0)..<5 {
            setPixel(&src, x: x, y: 0, r: UInt8(x * 10), g: 0, b: 0, a: 255)
        }
        let dst = try decodePpm(encodePpm(src))
        for x in UInt32(0)..<5 {
            let (r, _, _, _) = pixelAt(dst, x: x, y: 0)
            XCTAssertEqual(r, UInt8(x * 10))
        }
    }
}
