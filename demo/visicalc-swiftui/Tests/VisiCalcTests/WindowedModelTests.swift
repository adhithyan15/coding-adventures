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
        XCTAssertEqual(w[0][0], "15")       // A1 (unformatted)
        // E1/E5 carry the "#,##0.00" seed format → the engine renders the
        // formatted display string (window() now reads sc_get_display_window).
        XCTAssertEqual(w[0][4], "38.00")    // E1 = SUM(A1:D1)
        XCTAssertEqual(w[4][4], "169.00")   // E5 grand total
    }

    func testFarWindowReachesZ1000AndGapsAreSparse() {
        let m = WindowedSheetModel()
        // A window 1000 rows down, around column Z (26): the far formula shows.
        let far = m.window(rows: 998...1002, cols: 24...28)
        // Z1000 = SUM(A1:A4) = 39 at row 1000 / col 26, with the "0.0%" seed
        // format → 39 × 100 = "3900.0%": the format applies 1000 rows off-origin.
        XCTAssertEqual(far[2][2], "3900.0%")
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
        // And the recomputed value is visible through a fresh window read:
        // 115+8+12+4 = 139, formatted as a percent ("0.0%") → "13900.0%".
        XCTAssertEqual(m.window(rows: 1000...1000, cols: 26...26)[0][0], "13900.0%")
    }

    /// Drag-fill replicates the selected cell, shifting relative refs per target.
    func testFillDownShiftsRelativeReferences() {
        let m = WindowedSheetModel()
        // Seed a fresh column: H1=2, H2=3, H3=4 (col 8 = H); I1 = H1*10 (col 9 = I).
        m.setCell("H1", "2")
        m.setCell("H2", "3")
        m.setCell("H3", "4")
        m.setCell("I1", "=H1*10") // 20
        // Select I1 and fill down 10 — each filled formula tracks its row.
        m.select(row: 1, col: 9)
        m.fillDown(10)
        XCTAssertEqual(m.rowCells(2)[8], "30") // I2 = H2*10
        XCTAssertEqual(m.rowCells(3)[8], "40") // I3 = H3*10
        XCTAssertEqual(m.rowCells(1)[8], "20") // I1 source untouched
    }
}
