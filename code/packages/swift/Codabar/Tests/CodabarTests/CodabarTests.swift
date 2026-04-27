import XCTest
@testable import Codabar

final class CodabarTests: XCTestCase {
    func testNormalizeCodabarAddsDefaultGuards() throws {
        XCTAssertEqual(try normalizeCodabar("40156"), "A40156A")
    }

    func testNormalizeCodabarPreservesExplicitGuards() throws {
        XCTAssertEqual(try normalizeCodabar("B1234D"), "B1234D")
    }

    func testExpandCodabarRunsIncludesInterCharacterGap() throws {
        let runs = try expandCodabarRuns("40156")
        XCTAssertTrue(runs.contains { $0.role == "inter-character-gap" })
    }

    func testLayoutCodabarIncludesMetadata() throws {
        let scene = try layoutCodabar("40156")
        XCTAssertEqual(scene.metadata["symbology"], "codabar")
        XCTAssertEqual(scene.metadata["start"], "A")
        XCTAssertEqual(scene.metadata["stop"], "A")
        XCTAssertEqual(scene.height, 120)
        XCTAssertFalse(scene.instructions.isEmpty)
    }
}
