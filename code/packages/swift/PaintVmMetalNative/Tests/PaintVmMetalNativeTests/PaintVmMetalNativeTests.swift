import XCTest
import PixelContainer
@testable import PaintInstructions
@testable import PaintVmMetalNative

final class PaintVmMetalNativeTests: XCTestCase {
    func testRenderSceneToPixels() throws {
        #if os(macOS) && arch(arm64)
        let scene = paintScene(
            width: 8,
            height: 4,
            instructions: [
                paintRect(x: 0, y: 0, width: 4, height: 4, fill: "#000000"),
            ]
        )
        let pixels = try PaintVmMetalNative.render(scene)
        XCTAssertEqual(pixels.width, 8)
        XCTAssertEqual(pixels.height, 4)
        XCTAssertEqual(pixels.data.count, 8 * 4 * 4)
        #else
        throw XCTSkip("Metal Paint VM is only exercised on macOS arm64")
        #endif
    }
}
