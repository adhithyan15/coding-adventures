import XCTest
import PixelContainer
@testable import ImagePointOps

// Helper: 1×1 image with a single colour.
func solid(_ r: UInt8, _ g: UInt8, _ b: UInt8, _ a: UInt8) -> PixelContainer {
    var img = PixelContainer(width: 1, height: 1)
    setPixel(&img, x: 0, y: 0, r: r, g: g, b: b, a: a)
    return img
}

func px(_ img: PixelContainer) -> (UInt8, UInt8, UInt8, UInt8) {
    pixelAt(img, x: 0, y: 0)
}

final class ImagePointOpsTests: XCTestCase {

    func testDimensionsPreserved() {
        let img = PixelContainer(width: 3, height: 5)
        let out = invert(img)
        XCTAssertEqual(out.width, 3)
        XCTAssertEqual(out.height, 5)
    }

    func testInvertRGB() {
        let out = invert(solid(10, 100, 200, 255))
        let (r, g, b, a) = px(out)
        XCTAssertEqual(r, 245)
        XCTAssertEqual(g, 155)
        XCTAssertEqual(b, 55)
        XCTAssertEqual(a, 255)
    }

    func testInvertPreservesAlpha() {
        let out = invert(solid(10, 100, 200, 128))
        XCTAssertEqual(px(out).3, 128)
    }

    func testDoubleInvertIdentity() {
        let img = solid(30, 80, 180, 255)
        let (r, g, b, a) = px(invert(invert(img)))
        let (ir, ig, ib, ia) = px(img)
        XCTAssertEqual(r, ir); XCTAssertEqual(g, ig)
        XCTAssertEqual(b, ib); XCTAssertEqual(a, ia)
    }

    func testThresholdAbove() {
        let (r, g, b, _) = px(threshold(solid(200, 200, 200, 255), value: 128))
        XCTAssertEqual(r, 255); XCTAssertEqual(g, 255); XCTAssertEqual(b, 255)
    }

    func testThresholdBelow() {
        let (r, g, b, _) = px(threshold(solid(50, 50, 50, 255), value: 128))
        XCTAssertEqual(r, 0); XCTAssertEqual(g, 0); XCTAssertEqual(b, 0)
    }

    func testThresholdLuminanceWhite() {
        let (r, _, _, _) = px(thresholdLuminance(solid(255, 255, 255, 255), value: 128))
        XCTAssertEqual(r, 255)
    }

    func testPosterizeTwoLevels() {
        let (r, _, _, _) = px(posterize(solid(50, 50, 50, 255), levels: 2))
        XCTAssertTrue(r == 0 || r == 255)
    }

    func testSwapRGBBGR() {
        let (r, g, b, _) = px(swapRGBBGR(solid(255, 0, 0, 255)))
        XCTAssertEqual(r, 0); XCTAssertEqual(g, 0); XCTAssertEqual(b, 255)
    }

    func testExtractChannelRed() {
        let (r, g, b, _) = px(extractChannel(solid(100, 150, 200, 255), channel: .r))
        XCTAssertEqual(r, 100); XCTAssertEqual(g, 0); XCTAssertEqual(b, 0)
    }

    func testBrightnessClampsHigh() {
        let (r, g, _, _) = px(brightness(solid(250, 10, 10, 255), offset: 20))
        XCTAssertEqual(r, 255)
        XCTAssertEqual(g, 30)
    }

    func testBrightnessClampsLow() {
        let (r, _, _, _) = px(brightness(solid(5, 10, 10, 255), offset: -20))
        XCTAssertEqual(r, 0)
    }

    func testContrastIdentity() {
        let img = solid(100, 150, 200, 255)
        let out = contrast(img, factor: 1.0)
        let (r, g, b, _) = px(out), (ir, ig, ib, _) = px(img)
        XCTAssertLessThanOrEqual(abs(Int(r) - Int(ir)), 1)
        XCTAssertLessThanOrEqual(abs(Int(g) - Int(ig)), 1)
        XCTAssertLessThanOrEqual(abs(Int(b) - Int(ib)), 1)
    }

    func testGammaIdentity() {
        let img = solid(100, 150, 200, 255)
        let (r, _, _, _) = px(gamma(img, g: 1.0))
        let (ir, _, _, _) = px(img)
        XCTAssertLessThanOrEqual(abs(Int(r) - Int(ir)), 1)
    }

    func testGammaBrightensMidtones() {
        let (r, _, _, _) = px(gamma(solid(128, 128, 128, 255), g: 0.5))
        XCTAssertGreaterThan(r, 128)
    }

    func testExposurePlusOne() {
        let img = solid(100, 100, 100, 255)
        let (r, _, _, _) = px(exposure(img, stops: 1.0))
        let (ir, _, _, _) = px(img)
        XCTAssertGreaterThan(r, ir)
    }

    func testGreyscaleWhiteStaysWhite() {
        for method in [GreyscaleMethod.rec709, .bt601, .average] {
            let (r, g, b, _) = px(greyscale(solid(255, 255, 255, 255), method: method))
            XCTAssertEqual(r, 255); XCTAssertEqual(g, 255); XCTAssertEqual(b, 255)
        }
    }

    func testGreyscaleBlackStaysBlack() {
        let (r, g, b, _) = px(greyscale(solid(0, 0, 0, 255)))
        XCTAssertEqual(r, 0); XCTAssertEqual(g, 0); XCTAssertEqual(b, 0)
    }

    func testSepiaPreservesAlpha() {
        XCTAssertEqual(px(sepia(solid(128, 128, 128, 200))).3, 200)
    }

    func testColourMatrixIdentity() {
        let img = solid(80, 120, 200, 255)
        let id: [[Float]] = [[1,0,0],[0,1,0],[0,0,1]]
        let (r, g, b, _) = px(colourMatrix(img, matrix: id))
        let (ir, ig, ib, _) = px(img)
        XCTAssertLessThanOrEqual(abs(Int(r) - Int(ir)), 1)
        XCTAssertLessThanOrEqual(abs(Int(g) - Int(ig)), 1)
        XCTAssertLessThanOrEqual(abs(Int(b) - Int(ib)), 1)
    }

    func testSaturateZeroGivesGrey() {
        let (r, g, b, _) = px(saturate(solid(200, 100, 50, 255), factor: 0))
        XCTAssertEqual(r, g); XCTAssertEqual(g, b)
    }

    func testHueRotate360Identity() {
        let img = solid(200, 80, 40, 255)
        let (r, g, b, _) = px(hueRotate(img, degrees: 360))
        let (ir, ig, ib, _) = px(img)
        XCTAssertLessThanOrEqual(abs(Int(r) - Int(ir)), 2)
        XCTAssertLessThanOrEqual(abs(Int(g) - Int(ig)), 2)
        XCTAssertLessThanOrEqual(abs(Int(b) - Int(ib)), 2)
    }

    func testSRGBLinearRoundtrip() {
        let img = solid(100, 150, 200, 255)
        let (r, g, b, _) = px(linearToSRGBImage(srgbToLinearImage(img)))
        let (ir, ig, ib, _) = px(img)
        XCTAssertLessThanOrEqual(abs(Int(r) - Int(ir)), 2)
        XCTAssertLessThanOrEqual(abs(Int(g) - Int(ig)), 2)
        XCTAssertLessThanOrEqual(abs(Int(b) - Int(ib)), 2)
    }

    func testApplyLUT1DInvert() {
        let lut = (0..<256).map { UInt8(255 - $0) }
        let (r, g, b, _) = px(applyLUT1DU8(solid(100, 0, 200, 255), lutR: lut, lutG: lut, lutB: lut))
        XCTAssertEqual(r, 155); XCTAssertEqual(g, 255); XCTAssertEqual(b, 55)
    }

    func testBuildLUT1DU8Identity() {
        let lut = buildLUT1DU8 { v in v }
        for i in 0..<256 {
            XCTAssertLessThanOrEqual(abs(Int(lut[i]) - i), 1, "index \(i)")
        }
    }

    func testBuildGammaLUTIdentity() {
        let lut = buildGammaLUT(g: 1.0)
        for i in 0..<256 {
            XCTAssertLessThanOrEqual(abs(Int(lut[i]) - i), 1, "index \(i)")
        }
    }
}
