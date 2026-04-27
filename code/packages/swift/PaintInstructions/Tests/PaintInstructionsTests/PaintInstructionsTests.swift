import XCTest
@testable import PaintInstructions

final class PaintInstructionsTests: XCTestCase {
    func testParseHexColor() {
        XCTAssertEqual(
            parsePaintColor("#336699"),
            PaintColorRGBA8(r: 0x33, g: 0x66, b: 0x99, a: 255)
        )
    }

    func testPaintSceneDefaults() {
        let scene = paintScene(width: 32, height: 16, instructions: [])
        XCTAssertEqual(scene.background, "#ffffff")
        XCTAssertEqual(scene.width, 32)
        XCTAssertEqual(scene.height, 16)
    }
}
