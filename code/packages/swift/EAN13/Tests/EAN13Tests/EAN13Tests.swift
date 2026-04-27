import XCTest
@testable import EAN13

final class EAN13Tests: XCTestCase {
    func testComputeCheckDigitMatchesReference() throws {
        XCTAssertEqual(try computeEAN13CheckDigit("400638133393"), "1")
    }

    func testNormalizeAppendsCheckDigit() throws {
        XCTAssertEqual(try normalizeEAN13("400638133393"), "4006381333931")
    }

    func testLeftParityPatternMatchesReference() throws {
        XCTAssertEqual(try leftParityPattern("4006381333931"), "LGLLGG")
    }

    func testExpandRunsTotal95Modules() throws {
        XCTAssertEqual(try expandEAN13Runs("4006381333931").reduce(0) { $0 + $1.modules }, 95)
    }

    func testLayoutEAN13IncludesMetadata() throws {
        let scene = try layoutEAN13("400638133393")
        XCTAssertEqual(scene.metadata["symbology"], "ean-13")
        XCTAssertEqual(scene.metadata["content_modules"], "95")
        XCTAssertEqual(scene.height, 120)
        XCTAssertFalse(scene.instructions.isEmpty)
    }
}
