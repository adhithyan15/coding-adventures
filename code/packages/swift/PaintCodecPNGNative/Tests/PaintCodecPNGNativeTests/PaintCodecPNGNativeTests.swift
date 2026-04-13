import XCTest
import PixelContainer
@testable import PaintCodecPNGNative

final class PaintCodecPNGNativeTests: XCTestCase {
    func testEncodePNGSignature() throws {
        var pixels = PixelContainer(width: 1, height: 1)
        pixels.data = [0, 0, 0, 255]
        let png = try PaintCodecPNGNative.encode(pixels)
        XCTAssertEqual(Array(png.prefix(8)), [137, 80, 78, 71, 13, 10, 26, 10])
    }
}
