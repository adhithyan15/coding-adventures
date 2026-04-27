// ImageGeometricTransformsTests.swift
// Part of coding-adventures — an educational computing stack.
//
// ============================================================================
// MARK: - IMG04 Test Suite
// ============================================================================
//
// Coverage targets:
//   - Every public function has at least one happy-path test.
//   - Round-trip identities (double-flip, CW+CCW, rotate 0, affine identity).
//   - Dimension assertions for all transforms that change canvas size.
//   - Pixel-value assertions for lossless transforms.
//   - Near-identity assertions (±2 per channel) for resampling transforms.
//   - OOB mode smoke tests (no crash; sensible values).
//   - Interpolation-specific tests (nearest exact, bilinear midpoint blend).

import XCTest
import PixelContainer
@testable import ImageGeometricTransforms

// ============================================================================
// MARK: - Test Helpers
// ============================================================================

/// Create a 1×1 image with a single RGBA pixel.
private func solid(_ r: UInt8, _ g: UInt8, _ b: UInt8, _ a: UInt8) -> PixelContainer {
    var img = PixelContainer(width: 1, height: 1)
    setPixel(&img, x: 0, y: 0, r: r, g: g, b: b, a: a)
    return img
}

/// Read the single pixel from a 1×1 image.
private func px(_ img: PixelContainer) -> (UInt8, UInt8, UInt8, UInt8) {
    pixelAt(img, x: 0, y: 0)
}

/// Create a W×H image with a horizontal gradient: pixel (x, y) has R = x * 255 / (W-1).
private func hGradient(width W: UInt32, height H: UInt32) -> PixelContainer {
    var img = PixelContainer(width: W, height: H)
    for y: UInt32 in 0..<H {
        for x: UInt32 in 0..<W {
            let v = W > 1 ? UInt8(UInt32(x) * 255 / (W - 1)) : UInt8(0)
            setPixel(&img, x: x, y: y, r: v, g: 0, b: 0, a: 255)
        }
    }
    return img
}

/// Create a small checkerboard: alternating white/black 1×1 tiles.
private func checker(width W: UInt32, height H: UInt32) -> PixelContainer {
    var img = PixelContainer(width: W, height: H)
    for y: UInt32 in 0..<H {
        for x: UInt32 in 0..<W {
            let v: UInt8 = ((x + y) % 2 == 0) ? 255 : 0
            setPixel(&img, x: x, y: y, r: v, g: v, b: v, a: 255)
        }
    }
    return img
}

// ============================================================================
// MARK: - Test Case
// ============================================================================

final class ImageGeometricTransformsTests: XCTestCase {

    // ── flipHorizontal ────────────────────────────────────────────────────

    /// After a horizontal flip the image dimensions are unchanged.
    func testFlipHorizontalPreservesDimensions() {
        let src = PixelContainer(width: 5, height: 3)
        let out = flipHorizontal(src)
        XCTAssertEqual(out.width, 5)
        XCTAssertEqual(out.height, 3)
    }

    /// Left-most column of the source becomes the right-most column of the output.
    func testFlipHorizontalMovesPixels() {
        var src = PixelContainer(width: 4, height: 1)
        setPixel(&src, x: 0, y: 0, r: 255, g: 0, b: 0, a: 255)
        setPixel(&src, x: 3, y: 0, r: 0,   g: 0, b: 255, a: 255)
        let out = flipHorizontal(src)
        // src[0] → out[3], src[3] → out[0]
        let (r0, _, b0, _) = pixelAt(out, x: 0, y: 0)
        let (r3, _, b3, _) = pixelAt(out, x: 3, y: 0)
        XCTAssertEqual(b0, 255)   // original right pixel moved to left
        XCTAssertEqual(r3, 255)   // original left pixel moved to right
        XCTAssertEqual(r0, 0)
        XCTAssertEqual(b3, 0)
    }

    /// Applying flipHorizontal twice is the identity.
    func testFlipHorizontalDoubleIdentity() {
        let src = checker(width: 6, height: 4)
        let restored = flipHorizontal(flipHorizontal(src))
        for y: UInt32 in 0..<src.height {
            for x: UInt32 in 0..<src.width {
                let (r,  g,  b,  a)  = pixelAt(src,      x: x, y: y)
                let (r2, g2, b2, a2) = pixelAt(restored, x: x, y: y)
                XCTAssertEqual(r, r2, "r mismatch at (\(x),\(y))")
                XCTAssertEqual(g, g2, "g mismatch at (\(x),\(y))")
                XCTAssertEqual(b, b2, "b mismatch at (\(x),\(y))")
                XCTAssertEqual(a, a2, "a mismatch at (\(x),\(y))")
            }
        }
    }

    // ── flipVertical ──────────────────────────────────────────────────────

    /// Top row of the source becomes the bottom row of the output.
    func testFlipVerticalMovesPixels() {
        var src = PixelContainer(width: 1, height: 4)
        setPixel(&src, x: 0, y: 0, r: 255, g: 0, b: 0, a: 255)
        setPixel(&src, x: 0, y: 3, r: 0,   g: 0, b: 255, a: 255)
        let out = flipVertical(src)
        let (r0, _, b0, _) = pixelAt(out, x: 0, y: 0)
        let (r3, _, b3, _) = pixelAt(out, x: 0, y: 3)
        XCTAssertEqual(b0, 255)
        XCTAssertEqual(r3, 255)
        XCTAssertEqual(r0, 0)
        XCTAssertEqual(b3, 0)
    }

    /// Applying flipVertical twice is the identity.
    func testFlipVerticalDoubleIdentity() {
        let src = checker(width: 5, height: 5)
        let restored = flipVertical(flipVertical(src))
        for y: UInt32 in 0..<src.height {
            for x: UInt32 in 0..<src.width {
                let (r, g, b, a)     = pixelAt(src,      x: x, y: y)
                let (r2, g2, b2, a2) = pixelAt(restored, x: x, y: y)
                XCTAssertEqual(r, r2); XCTAssertEqual(g, g2)
                XCTAssertEqual(b, b2); XCTAssertEqual(a, a2)
            }
        }
    }

    // ── rotate90CW / rotate90CCW ──────────────────────────────────────────

    /// A 90° CW rotation swaps width and height.
    func testRotate90CWSwapsDimensions() {
        let src = PixelContainer(width: 7, height: 3)
        let out = rotate90CW(src)
        XCTAssertEqual(out.width,  3)  // new width  = old height
        XCTAssertEqual(out.height, 7)  // new height = old width
    }

    /// A 90° CCW rotation swaps width and height.
    func testRotate90CCWSwapsDimensions() {
        let src = PixelContainer(width: 7, height: 3)
        let out = rotate90CCW(src)
        XCTAssertEqual(out.width,  3)
        XCTAssertEqual(out.height, 7)
    }

    /// CW followed by CCW returns the original image exactly.
    func testRotate90CWThenCCWIsIdentity() {
        let src = checker(width: 4, height: 6)
        let restored = rotate90CCW(rotate90CW(src))
        XCTAssertEqual(restored.width,  src.width)
        XCTAssertEqual(restored.height, src.height)
        for y: UInt32 in 0..<src.height {
            for x: UInt32 in 0..<src.width {
                let (r,  g,  b,  a)  = pixelAt(src,      x: x, y: y)
                let (r2, g2, b2, a2) = pixelAt(restored, x: x, y: y)
                XCTAssertEqual(r, r2); XCTAssertEqual(g, g2)
                XCTAssertEqual(b, b2); XCTAssertEqual(a, a2)
            }
        }
    }

    /// Four 90° CW rotations return the original image.
    func testRotate90CWFourTimesIsIdentity() {
        let src = checker(width: 3, height: 3)
        let restored = rotate90CW(rotate90CW(rotate90CW(rotate90CW(src))))
        for y: UInt32 in 0..<src.height {
            for x: UInt32 in 0..<src.width {
                let (r,  g,  b,  a)  = pixelAt(src,      x: x, y: y)
                let (r2, g2, b2, a2) = pixelAt(restored, x: x, y: y)
                XCTAssertEqual(r, r2); XCTAssertEqual(g, g2)
                XCTAssertEqual(b, b2); XCTAssertEqual(a, a2)
            }
        }
    }

    // ── rotate180 ─────────────────────────────────────────────────────────

    /// Applying rotate180 twice is the identity.
    func testRotate180TwiceIsIdentity() {
        let src = checker(width: 5, height: 5)
        let restored = rotate180(rotate180(src))
        for y: UInt32 in 0..<src.height {
            for x: UInt32 in 0..<src.width {
                let (r,  g,  b,  a)  = pixelAt(src,      x: x, y: y)
                let (r2, g2, b2, a2) = pixelAt(restored, x: x, y: y)
                XCTAssertEqual(r, r2); XCTAssertEqual(g, g2)
                XCTAssertEqual(b, b2); XCTAssertEqual(a, a2)
            }
        }
    }

    /// rotate180 is equivalent to flipH composed with flipV.
    func testRotate180EqualsDoubleFlip() {
        let src = checker(width: 4, height: 4)
        let via180  = rotate180(src)
        let viaFlip = flipVertical(flipHorizontal(src))
        for y: UInt32 in 0..<src.height {
            for x: UInt32 in 0..<src.width {
                let (r1, g1, b1, a1) = pixelAt(via180,  x: x, y: y)
                let (r2, g2, b2, a2) = pixelAt(viaFlip, x: x, y: y)
                XCTAssertEqual(r1, r2); XCTAssertEqual(g1, g2)
                XCTAssertEqual(b1, b2); XCTAssertEqual(a1, a2)
            }
        }
    }

    // ── crop ──────────────────────────────────────────────────────────────

    /// Crop returns the requested dimensions.
    func testCropDimensions() {
        let src = PixelContainer(width: 10, height: 10)
        let out = crop(src, x: 2, y: 3, w: 4, h: 5)
        XCTAssertEqual(out.width,  4)
        XCTAssertEqual(out.height, 5)
    }

    /// Crop preserves pixel values from the correct source location.
    func testCropPixelValues() {
        var src = PixelContainer(width: 5, height: 5)
        // Paint a single distinguishable pixel at (3, 2).
        setPixel(&src, x: 3, y: 2, r: 200, g: 100, b: 50, a: 255)
        // Crop from (2, 1) with size 3×3 — the marked pixel is now at (1, 1).
        let out = crop(src, x: 2, y: 1, w: 3, h: 3)
        let (r, g, b, a) = pixelAt(out, x: 1, y: 1)
        XCTAssertEqual(r, 200)
        XCTAssertEqual(g, 100)
        XCTAssertEqual(b, 50)
        XCTAssertEqual(a, 255)
    }

    // ── pad ───────────────────────────────────────────────────────────────

    /// Pad returns the correct expanded dimensions.
    func testPadDimensions() {
        let src = PixelContainer(width: 4, height: 4)
        let out = pad(src, top: 1, right: 2, bottom: 3, left: 4)
        XCTAssertEqual(out.width,  4 + 4 + 2)   // left + src + right
        XCTAssertEqual(out.height, 1 + 4 + 3)   // top  + src + bottom
    }

    /// Border pixels have the fill colour.
    func testPadFillBorder() {
        let src = PixelContainer(width: 2, height: 2)
        let out = pad(src, top: 1, right: 1, bottom: 1, left: 1,
                      fill: (255, 0, 128, 255))
        // Top-left corner is a border pixel.
        let (r, g, b, a) = pixelAt(out, x: 0, y: 0)
        XCTAssertEqual(r, 255)
        XCTAssertEqual(g, 0)
        XCTAssertEqual(b, 128)
        XCTAssertEqual(a, 255)
    }

    /// Interior pixels are copied unchanged from the source.
    func testPadInteriorPreserved() {
        var src = PixelContainer(width: 2, height: 2)
        setPixel(&src, x: 0, y: 0, r: 77, g: 88, b: 99, a: 200)
        let out = pad(src, top: 2, right: 0, bottom: 0, left: 3,
                      fill: (0, 0, 0, 255))
        // The (0,0) source pixel is now at (3, 2) in the output.
        let (r, g, b, a) = pixelAt(out, x: 3, y: 2)
        XCTAssertEqual(r, 77)
        XCTAssertEqual(g, 88)
        XCTAssertEqual(b, 99)
        XCTAssertEqual(a, 200)
    }

    // ── scale ─────────────────────────────────────────────────────────────

    /// Scaling up doubles the dimensions.
    func testScaleUpDoublesDimensions() {
        let src = PixelContainer(width: 3, height: 5)
        let out = scale(src, width: 6, height: 10)
        XCTAssertEqual(out.width,  6)
        XCTAssertEqual(out.height, 10)
    }

    /// Scaling down halves the dimensions.
    func testScaleDownHalvesDimensions() {
        let src = PixelContainer(width: 8, height: 6)
        let out = scale(src, width: 4, height: 3)
        XCTAssertEqual(out.width,  4)
        XCTAssertEqual(out.height, 3)
    }

    /// Scaling with replicate OOB does not crash.
    func testScaleReplicateOOBNoCrash() {
        let src = solid(128, 64, 32, 255)
        let out = scale(src, width: 3, height: 3, mode: .bilinear)
        XCTAssertEqual(out.width, 3)
        XCTAssertEqual(out.height, 3)
    }

    // ── rotate (arbitrary angle) ──────────────────────────────────────────

    /// Rotating by 0 radians is approximately the identity (±2 per channel).
    func testRotateZeroApproxIdentity() {
        let src = checker(width: 5, height: 5)
        let out = rotate(src, radians: 0.0, mode: .bilinear, bounds: .crop)
        // With .crop bounds rotate(0) returns the same size.
        XCTAssertEqual(out.width,  src.width)
        XCTAssertEqual(out.height, src.height)
        for y: UInt32 in 0..<src.height {
            for x: UInt32 in 0..<src.width {
                let (ir, ig, ib, _) = pixelAt(src, x: x, y: y)
                let ( r,  g,  b, _) = pixelAt(out, x: x, y: y)
                XCTAssertLessThanOrEqual(abs(Int(r) - Int(ir)), 2, "r at (\(x),\(y))")
                XCTAssertLessThanOrEqual(abs(Int(g) - Int(ig)), 2, "g at (\(x),\(y))")
                XCTAssertLessThanOrEqual(abs(Int(b) - Int(ib)), 2, "b at (\(x),\(y))")
            }
        }
    }

    /// Rotating by .fit expands the canvas (for a 45° rotation of a square).
    func testRotateFitExpandsCanvas() {
        let src = PixelContainer(width: 10, height: 10)
        let out = rotate(src, radians: Float.pi / 4, bounds: .fit)
        // Diagonal of a 10×10 is ~14.14, so both dimensions should be >10.
        XCTAssertGreaterThan(out.width,  src.width)
        XCTAssertGreaterThan(out.height, src.height)
    }

    // ── affine ────────────────────────────────────────────────────────────

    /// Affine identity matrix returns an image approximately equal to the source.
    func testAffineIdentityApproxIdentity() {
        let src = checker(width: 4, height: 4)
        let identity: [[Float]] = [[1, 0, 0], [0, 1, 0]]
        let out = affine(src, matrix: identity, width: src.width, height: src.height,
                         mode: .nearest, oob: .replicate)
        for y: UInt32 in 0..<src.height {
            for x: UInt32 in 0..<src.width {
                let (ir, ig, ib, _) = pixelAt(src, x: x, y: y)
                let ( r,  g,  b, _) = pixelAt(out, x: x, y: y)
                XCTAssertLessThanOrEqual(abs(Int(r) - Int(ir)), 2)
                XCTAssertLessThanOrEqual(abs(Int(g) - Int(ig)), 2)
                XCTAssertLessThanOrEqual(abs(Int(b) - Int(ib)), 2)
            }
        }
    }

    /// Affine translate shifts pixels by the specified offset.
    func testAffineTranslate() {
        var src = PixelContainer(width: 5, height: 5)
        setPixel(&src, x: 2, y: 2, r: 200, g: 0, b: 0, a: 255)
        // Translate by (-1, -1): output pixel at (1,1) should sample source (2,2).
        let translate: [[Float]] = [[1, 0, 1], [0, 1, 1]]
        let out = affine(src, matrix: translate, width: 5, height: 5,
                         mode: .nearest, oob: .zero)
        let (r, _, _, _) = pixelAt(out, x: 1, y: 1)
        XCTAssertEqual(r, 200)
    }

    // ── perspectiveWarp ───────────────────────────────────────────────────

    /// A perspective identity matrix (bottom row [0,0,1]) is approximately identity.
    func testPerspectiveIdentityApproxIdentity() {
        let src = checker(width: 4, height: 4)
        // Identity perspective: H = 3×3 with diagonal 1s.
        let h: [[Float]] = [[1, 0, 0], [0, 1, 0], [0, 0, 1]]
        let out = perspectiveWarp(src, matrix: h, width: src.width, height: src.height,
                                  mode: .nearest, oob: .replicate)
        for y: UInt32 in 0..<src.height {
            for x: UInt32 in 0..<src.width {
                let (ir, ig, ib, _) = pixelAt(src, x: x, y: y)
                let ( r,  g,  b, _) = pixelAt(out, x: x, y: y)
                XCTAssertLessThanOrEqual(abs(Int(r) - Int(ir)), 2)
                XCTAssertLessThanOrEqual(abs(Int(g) - Int(ig)), 2)
                XCTAssertLessThanOrEqual(abs(Int(b) - Int(ib)), 2)
            }
        }
    }

    // ── Nearest-neighbour exact values ────────────────────────────────────

    /// Nearest sampler returns exact (un-blended) source pixel values.
    func testNearestReturnsExactValues() {
        var src = PixelContainer(width: 3, height: 1)
        setPixel(&src, x: 0, y: 0, r: 10,  g: 20,  b: 30,  a: 255)
        setPixel(&src, x: 1, y: 0, r: 100, g: 110, b: 120, a: 255)
        setPixel(&src, x: 2, y: 0, r: 200, g: 210, b: 220, a: 255)
        // Scale 2× with nearest: output [0]=src[0], [1]=src[0], [2]=src[1], etc.
        let out = scale(src, width: 6, height: 1, mode: .nearest)
        // Output pixel 0 should be src pixel 0.
        let (r0, g0, b0, _) = pixelAt(out, x: 0, y: 0)
        XCTAssertEqual(r0, 10)
        XCTAssertEqual(g0, 20)
        XCTAssertEqual(b0, 30)
        // Output pixel 4 or 5 should be src pixel 2.
        let (r5, g5, b5, _) = pixelAt(out, x: 5, y: 0)
        XCTAssertEqual(r5, 200)
        XCTAssertEqual(g5, 210)
        XCTAssertEqual(b5, 220)
    }

    // ── Bilinear midpoint blend ────────────────────────────────────────────

    /// Bilinear interpolation at the midpoint of two pixels should blend them.
    ///
    /// For a 1×2 source [left=0, right=255] scaled to 1×4 with bilinear,
    /// the middle output pixels should have intermediate values.
    func testBilinearBlendsMidpoint() {
        var src = PixelContainer(width: 2, height: 1)
        setPixel(&src, x: 0, y: 0, r: 0,   g: 0, b: 0, a: 255)
        setPixel(&src, x: 1, y: 0, r: 255, g: 0, b: 0, a: 255)
        let out = scale(src, width: 4, height: 1, mode: .bilinear)
        // The middle two output pixels should be between 0 and 255.
        let (r1, _, _, _) = pixelAt(out, x: 1, y: 0)
        let (r2, _, _, _) = pixelAt(out, x: 2, y: 0)
        XCTAssertGreaterThan(r1, 0,   "bilinear blend should be > 0 at x=1")
        XCTAssertLessThan   (r1, 255, "bilinear blend should be < 255 at x=1")
        XCTAssertGreaterThan(r2, 0,   "bilinear blend should be > 0 at x=2")
        XCTAssertLessThan   (r2, 255, "bilinear blend should be < 255 at x=2")
    }

    // ── OOB mode smoke tests ──────────────────────────────────────────────

    /// Scaling a 1-pixel image with nearest sampler and all four OOB modes
    /// does not crash and produces a valid image.
    func testScaleOOBModesNoCrash() {
        let src = solid(100, 150, 200, 255)
        for oob: OutOfBounds in [.zero, .replicate, .reflect, .wrap] {
            let identity: [[Float]] = [[1, 0, 0], [0, 1, 0]]
            let out = affine(src, matrix: identity, width: 3, height: 3,
                             mode: .nearest, oob: oob)
            XCTAssertEqual(out.width, 3)
            XCTAssertEqual(out.height, 3)
        }
    }

    // ── Additional dimension / round-trip tests ───────────────────────────

    /// rotate180 preserves dimensions.
    func testRotate180PreservesDimensions() {
        let src = PixelContainer(width: 7, height: 5)
        let out = rotate180(src)
        XCTAssertEqual(out.width, 7)
        XCTAssertEqual(out.height, 5)
    }

    /// crop then pad round-trips the original content.
    func testCropThenPadRoundTrip() {
        var src = PixelContainer(width: 6, height: 6)
        // Fill with a known pattern.
        for y: UInt32 in 0..<6 {
            for x: UInt32 in 0..<6 {
                setPixel(&src, x: x, y: y, r: UInt8(x * 40), g: UInt8(y * 40), b: 0, a: 255)
            }
        }
        // Crop out the centre 4×4 and then pad back to 6×6.
        let cropped = crop(src, x: 1, y: 1, w: 4, h: 4)
        let padded  = pad(cropped, top: 1, right: 1, bottom: 1, left: 1)
        // Centre region should match source.
        for y: UInt32 in 1..<5 {
            for x: UInt32 in 1..<5 {
                let (sr, sg, sb, _) = pixelAt(src, x: x, y: y)
                let (pr, pg, pb, _) = pixelAt(padded, x: x, y: y)
                XCTAssertEqual(sr, pr, "r at (\(x),\(y))")
                XCTAssertEqual(sg, pg, "g at (\(x),\(y))")
                XCTAssertEqual(sb, pb, "b at (\(x),\(y))")
            }
        }
    }

    /// scale with bicubic mode does not crash and returns correct dimensions.
    func testScaleBicubicDimensions() {
        let src = hGradient(width: 4, height: 4)
        let out = scale(src, width: 8, height: 8, mode: .bicubic)
        XCTAssertEqual(out.width, 8)
        XCTAssertEqual(out.height, 8)
    }

    /// perspectiveWarp with a scale matrix halves dimensions of the content.
    func testPerspectiveWarpScaleMatrix() {
        let src = checker(width: 4, height: 4)
        // Scale-by-2 perspective: sample from 2x source coords.
        let h: [[Float]] = [[2, 0, 0], [0, 2, 0], [0, 0, 1]]
        let out = perspectiveWarp(src, matrix: h, width: 4, height: 4,
                                  mode: .nearest, oob: .replicate)
        XCTAssertEqual(out.width,  4)
        XCTAssertEqual(out.height, 4)
    }

    /// rotate 2π (full circle) is approximately identity.
    func testRotateTwoPiApproxIdentity() {
        let src = checker(width: 5, height: 5)
        let out = rotate(src, radians: 2 * Float.pi, mode: .bilinear, bounds: .crop)
        for y: UInt32 in 1..<4 {   // avoid corner pixels clipped by .crop
            for x: UInt32 in 1..<4 {
                let (ir, ig, ib, _) = pixelAt(src, x: x, y: y)
                let ( r,  g,  b, _) = pixelAt(out, x: x, y: y)
                XCTAssertLessThanOrEqual(abs(Int(r) - Int(ir)), 2)
                XCTAssertLessThanOrEqual(abs(Int(g) - Int(ig)), 2)
                XCTAssertLessThanOrEqual(abs(Int(b) - Int(ib)), 2)
            }
        }
    }
}
