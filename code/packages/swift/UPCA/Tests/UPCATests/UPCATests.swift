import XCTest
@testable import UPCA

final class UPCATests: XCTestCase {
    func testComputeCheckDigitMatchesReference() throws {
        XCTAssertEqual(try computeUPCACheckDigit("03600029145"), "2")
    }

    func testNormalizeAppendsCheckDigit() throws {
        XCTAssertEqual(try normalizeUPCA("03600029145"), "036000291452")
    }

    func testExpandRunsTotal95Modules() throws {
        XCTAssertEqual(try expandUPCARuns("036000291452").reduce(0) { $0 + $1.modules }, 95)
    }

    func testLayoutUPCAIncludesMetadata() throws {
        let scene = try layoutUPCA("03600029145")
        XCTAssertEqual(scene.metadata["symbology"], "upc-a")
        XCTAssertEqual(scene.metadata["content_modules"], "95")
        XCTAssertEqual(scene.height, 120)
        XCTAssertFalse(scene.instructions.isEmpty)
    }
}
