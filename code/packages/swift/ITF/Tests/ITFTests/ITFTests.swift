import XCTest
@testable import ITF

final class ITFTests: XCTestCase {
    func testNormalizeRejectsOddInput() {
        XCTAssertThrowsError(try normalizeITF("12345"))
    }

    func testEncodeInterleavesDigitPairs() throws {
        let encoded = try encodeITF("123456")
        XCTAssertEqual(encoded.count, 3)
        XCTAssertEqual(encoded.first?.pair, "12")
    }

    func testExpandRunsIncludeStartAndStop() throws {
        let roles = try expandITFRuns("123456").map(\.role)
        XCTAssertTrue(roles.contains("start"))
        XCTAssertTrue(roles.contains("stop"))
    }

    func testLayoutITFIncludesMetadata() throws {
        let scene = try layoutITF("123456")
        XCTAssertEqual(scene.metadata["symbology"], "itf")
        XCTAssertEqual(scene.metadata["pair_count"], "3")
        XCTAssertEqual(scene.height, 120)
        XCTAssertFalse(scene.instructions.isEmpty)
    }
}
