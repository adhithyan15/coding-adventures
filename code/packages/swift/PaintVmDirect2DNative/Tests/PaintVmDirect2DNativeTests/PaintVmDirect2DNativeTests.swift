import XCTest
import PixelContainer
@testable import PaintInstructions
@testable import PaintVmDirect2DNative

final class PaintVmDirect2DNativeTests: XCTestCase {
    func testRenderSceneToPixels() throws {
        #if os(Windows)
        let scene = paintScene(
            width: 8,
            height: 4,
            instructions: [
                paintRect(x: 0, y: 0, width: 4, height: 4, fill: "#000000"),
            ]
        )
        let pixels = try PaintVmDirect2DNative.render(scene)
        XCTAssertEqual(pixels.width, 8)
        XCTAssertEqual(pixels.height, 4)
        XCTAssertEqual(pixels.data.count, 8 * 4 * 4)
        #else
        throw XCTSkip("Direct2D/GDI Paint VM is only exercised on Windows")
        #endif
    }
}
