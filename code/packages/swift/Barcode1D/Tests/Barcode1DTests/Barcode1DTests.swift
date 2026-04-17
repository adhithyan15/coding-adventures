import XCTest
@testable import Barcode1D

final class Barcode1DTests: XCTestCase {
    func testBuildScene() throws {
        let scene = try Barcode1D.buildScene("HELLO-123")
        XCTAssertEqual(scene.metadata["symbology"], "code39")
        XCTAssertEqual(scene.height, 120)
    }

    func testBuildSceneAdditionalSymbologies() throws {
        let cases: [(String, String, String)] = [
            ("40156", "codabar", "codabar"),
            ("Code 128", "code128", "code128"),
            ("400638133393", "ean-13", "ean-13"),
            ("123456", "itf", "itf"),
            ("03600029145", "upc-a", "upc-a"),
        ]

        for (input, symbology, expected) in cases {
            let scene = try Barcode1D.buildScene(input, symbology: symbology)
            XCTAssertEqual(scene.metadata["symbology"], expected)
            XCTAssertGreaterThan(scene.width, 0)
        }
    }

    func testRenderPNGSignature() throws {
        #if os(Windows) || (os(macOS) && arch(arm64))
        let png = try Barcode1D.renderPNG("HELLO-123")
        XCTAssertEqual(Array(png.prefix(8)), [137, 80, 78, 71, 13, 10, 26, 10])
        #else
        throw XCTSkip("Native barcode rendering is only exercised on macOS arm64 and Windows")
        #endif
    }
}
