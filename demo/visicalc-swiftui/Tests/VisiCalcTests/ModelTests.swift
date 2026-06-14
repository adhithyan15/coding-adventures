// ModelTests.swift — headless proof that the SwiftUI demo's grid is driven by
// the Rust engine (not hard-coded), and that edits recompute. Runs via
// `swift test`, no GUI needed.
import XCTest
@testable import VisiCalc

final class ModelTests: XCTestCase {
    func testGridIsEngineComputed() {
        let m = SpreadsheetModel()
        // The seed puts formulas in column E and row 5; these values exist ONLY
        // if the Rust engine computed them. viewportRows[r] = [label, A..E].
        XCTAssertEqual(m.viewportRows[0][5], "38")   // E1 = SUM(A1:D1) = 15+3+12+8
        XCTAssertEqual(m.viewportRows[4][1], "39")   // A5 = SUM(A1:A4) = 15+8+12+4
        XCTAssertEqual(m.viewportRows[4][5], "169")  // E5 grand total (cross-foots)
    }

    func testEditRecomputesDependents() {
        let m = SpreadsheetModel()
        m.select(row: 0, col: 1)        // A1
        m.setSelected("115")            // 15 -> 115 (+100)
        XCTAssertEqual(m.viewportRows[0][5], "138")  // E1: 115+3+12+8
        XCTAssertEqual(m.viewportRows[4][1], "139")  // A5: 115+8+12+4
        XCTAssertEqual(m.viewportRows[4][5], "269")  // grand total: 169+100
    }

    func testFormulaEntryAndError() {
        let m = SpreadsheetModel()
        m.select(row: 0, col: 1)        // A1
        m.setSelected("=1/0")
        XCTAssertEqual(m.viewportRows[0][1], "#DIV/0!") // A1 itself
        XCTAssertEqual(m.selectedRaw, "=1/0")           // formula bar shows source
        // Binary-op dependents propagate the error: B1 = A1 + 1.
        m.select(row: 0, col: 2)        // B1
        m.setSelected("=A1+1")
        XCTAssertEqual(m.viewportRows[0][2], "#DIV/0!")
    }
}
