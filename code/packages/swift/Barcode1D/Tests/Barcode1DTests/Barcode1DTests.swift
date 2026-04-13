import XCTest
@testable import Barcode1D

final class Barcode1DTests: XCTestCase {
    func testBuildScene() throws {
        let scene = try Barcode1D.buildScene("HELLO-123")
        XCTAssertEqual(scene.metadata["symbology"], "code39")
        XCTAssertEqual(scene.height, 120)
    }

    func testRenderPNGSignature() throws {
        #if os(macOS) && arch(arm64)
        let png = try Barcode1D.renderPNG("HELLO-123")
        XCTAssertEqual(Array(png.prefix(8)), [137, 80, 78, 71, 13, 10, 26, 10])
        #else
        throw XCTSkip("Metal-backed barcode rendering is only exercised on macOS arm64")
        #endif
    }
}
