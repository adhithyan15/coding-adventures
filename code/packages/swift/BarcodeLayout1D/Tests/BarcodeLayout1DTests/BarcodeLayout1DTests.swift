import XCTest
@testable import BarcodeLayout1D

final class BarcodeLayout1DTests: XCTestCase {
    func testLayoutProducesScene() throws {
        let runs = try runsFromBinaryPattern("11001", sourceCharacter: "A", sourceIndex: 0)
        let scene = try layoutBarcode1D(runs)

        XCTAssertEqual(scene.height, 120)
        XCTAssertEqual(scene.instructions.count, 2)
        XCTAssertEqual(scene.metadata["module_unit"], "4")
    }
}
