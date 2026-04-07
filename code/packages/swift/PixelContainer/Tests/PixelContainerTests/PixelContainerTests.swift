// PixelContainerTests.swift
// Part of coding-adventures — IC00 test suite
//
// ============================================================================
// Tests for the PixelContainer library
// ============================================================================
//
// Coverage areas:
//   1.  Initialization — size, zero-fill
//   2.  Buffer length — width × height × 4
//   3.  pixelAt — correct reads, out-of-bounds safety
//   4.  setPixel — correct writes, out-of-bounds no-op
//   5.  fillPixels — entire buffer overwritten
//   6.  Pixel offset formula — spot-checks via direct data access
//   7.  Row-major order — adjacent pixels in same row are adjacent in data
//   8.  Alpha channel — independent from RGB
//   9.  Edge dimensions — 1×1, 1×N, N×1
//  10.  PixelContainerError — enum equality
//
// ============================================================================

import XCTest
@testable import PixelContainer

// ============================================================================
// MARK: - Initialization Tests
// ============================================================================

final class InitTests: XCTestCase {

    /// A fresh container should have the correct dimensions.
    func testWidthAndHeight() {
        let c = PixelContainer(width: 10, height: 20)
        XCTAssertEqual(c.width, 10)
        XCTAssertEqual(c.height, 20)
    }

    /// Buffer length must be exactly width × height × 4 bytes.
    func testDataLength() {
        let c = PixelContainer(width: 3, height: 5)
        XCTAssertEqual(c.data.count, 3 * 5 * 4)
    }

    /// All bytes in a fresh container must be zero (transparent black).
    func testInitializesToZero() {
        let c = PixelContainer(width: 4, height: 4)
        XCTAssertTrue(c.data.allSatisfy { $0 == 0 })
    }

    /// A 1×1 container should have exactly 4 bytes.
    func testOneByOne() {
        let c = PixelContainer(width: 1, height: 1)
        XCTAssertEqual(c.data.count, 4)
    }

    /// A 256×256 container should have 262 144 bytes (256 × 256 × 4).
    func testLargeContainer() {
        let c = PixelContainer(width: 256, height: 256)
        XCTAssertEqual(c.data.count, 256 * 256 * 4)
    }
}

// ============================================================================
// MARK: - pixelAt Tests
// ============================================================================

final class PixelAtTests: XCTestCase {

    /// Reading from a freshly-created container yields (0,0,0,0) everywhere.
    func testReadFromFreshContainerIsZero() {
        let c = PixelContainer(width: 8, height: 8)
        let (r, g, b, a) = pixelAt(c, x: 3, y: 3)
        XCTAssertEqual(r, 0)
        XCTAssertEqual(g, 0)
        XCTAssertEqual(b, 0)
        XCTAssertEqual(a, 0)
    }

    /// Out-of-bounds access must return (0,0,0,0) without trapping.
    func testOutOfBoundsXReturnsZero() {
        let c = PixelContainer(width: 4, height: 4)
        let (r, g, b, a) = pixelAt(c, x: 100, y: 0)
        XCTAssertTrue((r, g, b, a) == (0, 0, 0, 0))
    }

    /// Out-of-bounds y must return (0,0,0,0) without trapping.
    func testOutOfBoundsYReturnsZero() {
        let c = PixelContainer(width: 4, height: 4)
        let (r, g, b, a) = pixelAt(c, x: 0, y: 100)
        XCTAssertTrue((r, g, b, a) == (0, 0, 0, 0))
    }

    /// pixelAt reads the correct channel values from a manually set pixel.
    func testReadsCorrectChannels() {
        // Build a container and manually set one pixel's bytes.
        var c = PixelContainer(width: 5, height: 5)
        // Pixel at (2, 3): offset = (3 * 5 + 2) * 4 = 17 * 4 = 68
        c.data[68] = 10   // R
        c.data[69] = 20   // G
        c.data[70] = 30   // B
        c.data[71] = 40   // A
        let (r, g, b, a) = pixelAt(c, x: 2, y: 3)
        XCTAssertEqual(r, 10)
        XCTAssertEqual(g, 20)
        XCTAssertEqual(b, 30)
        XCTAssertEqual(a, 40)
    }

    /// Corner pixel (0,0) should be at offset 0.
    func testTopLeftCorner() {
        var c = PixelContainer(width: 4, height: 4)
        c.data[0] = 255  // R of top-left pixel
        let (r, _, _, _) = pixelAt(c, x: 0, y: 0)
        XCTAssertEqual(r, 255)
    }
}

// ============================================================================
// MARK: - setPixel Tests
// ============================================================================

final class SetPixelTests: XCTestCase {

    /// setPixel should write the correct RGBA bytes at the right offset.
    func testSetAndReadBack() {
        var c = PixelContainer(width: 6, height: 6)
        setPixel(&c, x: 1, y: 2, r: 100, g: 150, b: 200, a: 255)
        let (r, g, b, a) = pixelAt(c, x: 1, y: 2)
        XCTAssertEqual(r, 100)
        XCTAssertEqual(g, 150)
        XCTAssertEqual(b, 200)
        XCTAssertEqual(a, 255)
    }

    /// setPixel on an out-of-bounds coordinate must be a no-op.
    func testOutOfBoundsIsNoop() {
        var c = PixelContainer(width: 4, height: 4)
        let before = c.data
        setPixel(&c, x: 10, y: 10, r: 255, g: 255, b: 255, a: 255)
        XCTAssertEqual(c.data, before)
    }

    /// Writing to two different pixels should not overlap.
    func testTwoPixelsDontOverlap() {
        var c = PixelContainer(width: 4, height: 4)
        setPixel(&c, x: 0, y: 0, r: 10, g: 20, b: 30, a: 40)
        setPixel(&c, x: 1, y: 0, r: 50, g: 60, b: 70, a: 80)
        let (r0, g0, b0, a0) = pixelAt(c, x: 0, y: 0)
        let (r1, g1, b1, a1) = pixelAt(c, x: 1, y: 0)
        XCTAssertTrue((r0, g0, b0, a0) == (10, 20, 30, 40))
        XCTAssertTrue((r1, g1, b1, a1) == (50, 60, 70, 80))
    }

    /// Writing all extreme values (0 and 255) round-trips correctly.
    func testExtremeValues() {
        var c = PixelContainer(width: 2, height: 2)
        setPixel(&c, x: 0, y: 0, r: 0,   g: 0,   b: 0,   a: 0)
        setPixel(&c, x: 1, y: 0, r: 255, g: 255, b: 255, a: 255)
        XCTAssertTrue(pixelAt(c, x: 0, y: 0) == (0,   0,   0,   0))
        XCTAssertTrue(pixelAt(c, x: 1, y: 0) == (255, 255, 255, 255))
    }

    /// Bottom-right pixel (last pixel) writes and reads correctly.
    func testBottomRightCorner() {
        var c = PixelContainer(width: 3, height: 3)
        setPixel(&c, x: 2, y: 2, r: 1, g: 2, b: 3, a: 4)
        let (r, g, b, a) = pixelAt(c, x: 2, y: 2)
        XCTAssertTrue((r, g, b, a) == (1, 2, 3, 4))
    }
}

// ============================================================================
// MARK: - fillPixels Tests
// ============================================================================

final class FillPixelsTests: XCTestCase {

    /// fillPixels should set every pixel to the specified colour.
    func testFillSetsAllPixels() {
        var c = PixelContainer(width: 4, height: 4)
        fillPixels(&c, r: 255, g: 128, b: 64, a: 200)
        for y in UInt32(0)..<4 {
            for x in UInt32(0)..<4 {
                let px = pixelAt(c, x: x, y: y)
                XCTAssertEqual(px.0, 255, "R at (\(x),\(y))")
                XCTAssertEqual(px.1, 128, "G at (\(x),\(y))")
                XCTAssertEqual(px.2, 64,  "B at (\(x),\(y))")
                XCTAssertEqual(px.3, 200, "A at (\(x),\(y))")
            }
        }
    }

    /// Filling with all zeros should produce a transparent-black buffer.
    func testFillWithZero() {
        var c = PixelContainer(width: 2, height: 2)
        // First write some non-zero values.
        setPixel(&c, x: 0, y: 0, r: 10, g: 20, b: 30, a: 40)
        // Now fill with zeros.
        fillPixels(&c, r: 0, g: 0, b: 0, a: 0)
        XCTAssertTrue(c.data.allSatisfy { $0 == 0 })
    }

    /// A second fillPixels call should completely overwrite the first.
    func testFillOverwritesPreviousFill() {
        var c = PixelContainer(width: 3, height: 3)
        fillPixels(&c, r: 255, g: 0, b: 0, a: 255)  // red
        fillPixels(&c, r: 0, g: 255, b: 0, a: 255)  // green
        for y in UInt32(0)..<3 {
            for x in UInt32(0)..<3 {
                let (r, g, b, _) = pixelAt(c, x: x, y: y)
                XCTAssertEqual(r, 0)
                XCTAssertEqual(g, 255)
                XCTAssertEqual(b, 0)
            }
        }
    }

    /// fillPixels on a 1×1 buffer touches exactly 4 bytes.
    func testFillOneByOne() {
        var c = PixelContainer(width: 1, height: 1)
        fillPixels(&c, r: 7, g: 8, b: 9, a: 10)
        XCTAssertEqual(c.data, [7, 8, 9, 10])
    }
}

// ============================================================================
// MARK: - Row-Major Order Tests
// ============================================================================

final class RowMajorTests: XCTestCase {

    /// Pixels in the same row are adjacent in the data array.
    /// offset(x, y) and offset(x+1, y) should differ by exactly 4 bytes.
    func testAdjacentPixelsInSameRow() {
        var c = PixelContainer(width: 5, height: 5)
        setPixel(&c, x: 0, y: 2, r: 1, g: 0, b: 0, a: 0)
        setPixel(&c, x: 1, y: 2, r: 2, g: 0, b: 0, a: 0)
        // Row y=2, x=0: offset = (2 * 5 + 0) * 4 = 40
        // Row y=2, x=1: offset = (2 * 5 + 1) * 4 = 44
        XCTAssertEqual(c.data[40], 1)
        XCTAssertEqual(c.data[44], 2)
    }

    /// The first pixel of row y is at offset y * width * 4.
    func testFirstPixelOfRowOffset() {
        var c = PixelContainer(width: 6, height: 6)
        // Row 3, column 0: offset = 3 * 6 * 4 = 72
        setPixel(&c, x: 0, y: 3, r: 99, g: 0, b: 0, a: 0)
        XCTAssertEqual(c.data[72], 99)
    }
}

// ============================================================================
// MARK: - PixelContainerError Tests
// ============================================================================

final class ErrorTests: XCTestCase {

    /// Error enum cases must compare equal to themselves.
    func testInvalidDimensionsEquality() {
        XCTAssertEqual(PixelContainerError.invalidDimensions,
                       PixelContainerError.invalidDimensions)
    }

    func testInvalidDataEquality() {
        XCTAssertEqual(PixelContainerError.invalidData,
                       PixelContainerError.invalidData)
    }

    /// The two error cases must be distinct.
    func testErrorCasesAreDifferent() {
        XCTAssertNotEqual(PixelContainerError.invalidDimensions,
                          PixelContainerError.invalidData)
    }
}

// ============================================================================
// MARK: - Dimension Edge Case Tests
// ============================================================================

final class EdgeDimensionTests: XCTestCase {

    /// A 1×100 (single-column) container should have 400 bytes.
    func testSingleColumnContainer() {
        let c = PixelContainer(width: 1, height: 100)
        XCTAssertEqual(c.data.count, 400)
    }

    /// A 100×1 (single-row) container should have 400 bytes.
    func testSingleRowContainer() {
        let c = PixelContainer(width: 100, height: 1)
        XCTAssertEqual(c.data.count, 400)
    }

    /// Setting the only pixel in a 1×1 container works correctly.
    func testSinglePixelRoundTrip() {
        var c = PixelContainer(width: 1, height: 1)
        setPixel(&c, x: 0, y: 0, r: 42, g: 43, b: 44, a: 45)
        XCTAssertTrue(pixelAt(c, x: 0, y: 0) == (42, 43, 44, 45))
    }

    /// The last pixel of a wide container writes to the right offset.
    func testLastPixelInWideRow() {
        var c = PixelContainer(width: 100, height: 1)
        setPixel(&c, x: 99, y: 0, r: 77, g: 0, b: 0, a: 0)
        // offset = (0 * 100 + 99) * 4 = 396
        XCTAssertEqual(c.data[396], 77)
    }
}
