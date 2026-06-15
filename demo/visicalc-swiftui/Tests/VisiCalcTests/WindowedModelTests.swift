// WindowedModelTests.swift — headless proof that the SwiftUI demo can render a
// VIRTUALIZED infinite sheet on the engine's viewport primitive, with no GUI.
// The view (InfiniteGridView) reads only the visible window via this model; here
// we drive that model directly and assert the windowed reads are correct,
// bounded, sparse, and diff on edit — the SwiftUI analog of the web demo's
// scripts/verify-infinite.mjs.
import XCTest
@testable import VisiCalc

final class WindowedModelTests: XCTestCase {
    func testExtentSizesToTheFarData() {
        let m = WindowedSheetModel()
        // The seed plants a formula at Z1000 (row 1000, col 26), so the virtual
        // grid must extend at least that far (plus the model's margin).
        XCTAssertGreaterThanOrEqual(m.totalRows, 1000)
        XCTAssertGreaterThanOrEqual(m.totalCols, 60)
    }

    func testTopWindowIsEngineComputedAndDense() {
        let m = WindowedSheetModel()
        // A small visible window over A1:E5 — the values are engine-computed.
        let w = m.window(rows: 1...5, cols: 1...5)
        XCTAssertEqual(w.count, 5)
        XCTAssertEqual(w[0][0], "15")   // A1
        XCTAssertEqual(w[0][4], "38")   // E1 = SUM(A1:D1)
        XCTAssertEqual(w[4][4], "169")  // E5 grand total
    }

    func testFarWindowReachesZ1000AndGapsAreSparse() {
        let m = WindowedSheetModel()
        // A window 1000 rows down, around column Z (26): the far formula shows.
        let far = m.window(rows: 998...1002, cols: 24...28)
        XCTAssertEqual(far[2][2], "39") // Z1000 = SUM(A1:A4), at row 1000 / col 26
        // The gap between the two data islands is empty (the sheet is sparse).
        let gap = m.window(rows: 100...110, cols: 1...10)
        for row in gap { for cell in row { XCTAssertEqual(cell, "") } }
    }

    func testColumnLettersPastZ() {
        let m = WindowedSheetModel()
        XCTAssertEqual(m.columnLetters(27), "AA")
        XCTAssertEqual(m.columnLetters(53), "BA")
        XCTAssertEqual(m.columnLetters(54), "BB")
    }

    func testEditDiffReachesTheFarDependent() {
        let m = WindowedSheetModel()
        // A1 feeds Z1000 (=SUM(A1:A4)); editing A1 must dirty the far cell too.
        let (changed, stale) = m.setCell("A1", "115")
        XCTAssertFalse(stale)
        XCTAssertTrue(changed.contains("A1"))
        XCTAssertTrue(changed.contains("Z1000"), "far dependent recomputed: \(changed)")
        // And the recomputed value is visible through a fresh window read.
        XCTAssertEqual(m.window(rows: 1000...1000, cols: 26...26)[0][0], "139") // 115+8+12+4
    }
}
