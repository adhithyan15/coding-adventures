import XCTest
@testable import Code39

final class Code39Tests: XCTestCase {
    func testLayoutCode39IncludesMetadata() throws {
        let scene = try layoutCode39("HELLO-123")
        XCTAssertEqual(scene.metadata["symbology"], "code39")
        XCTAssertEqual(scene.height, 120)
        XCTAssertFalse(scene.instructions.isEmpty)
    }

    func testNormalizeRejectsStartStopMarker() {
        XCTAssertThrowsError(try normalizeCode39("*ABC"))
    }
}
