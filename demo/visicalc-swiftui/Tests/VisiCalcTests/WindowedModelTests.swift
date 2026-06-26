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

    /// Structural edits (the + Row / − Row / + Col / − Col buttons drive
    /// insertRow/deleteRow/insertCol/deleteCol): inserting and deleting
    /// rows/columns shifts every formula reference across the band, and deleting a
    /// referenced band turns that reference into #REF!.
    func testStructuralInsertDeleteShiftsReferences() {
        let m = WindowedSheetModel()
        // The engine parenthesizes binary ops on re-emit ("=(H1+H3)"), so strip
        // parens before comparing the source the formula bar shows.
        func bare(_ s: String) -> String { s.replacingOccurrences(of: "(", with: "").replacingOccurrences(of: ")", with: "") }
        m.setCell("H1", "10")
        m.setCell("H2", "20")
        m.setCell("H3", "=H1+H2") // 30
        XCTAssertEqual(m.rowCells(3)[7], "30")

        // Insert a row at row 2: H2/H3 shift down to H3/H4, row 2 is blank, and
        // the formula's refs shift with their cells (=H1+H2 → =H1+H3).
        m.select(row: 2, col: 8); m.insertRow()
        XCTAssertEqual(m.rowCells(2)[7], "")    // inserted row blank
        XCTAssertEqual(m.rowCells(4)[7], "30")  // formula at H4
        m.select(row: 4, col: 8)
        XCTAssertEqual(bare(m.formulaText), "=H1+H3")

        // Delete that inserted row: everything shifts back.
        m.select(row: 2, col: 8); m.deleteRow()
        XCTAssertEqual(m.rowCells(3)[7], "30")
        m.select(row: 3, col: 8)
        XCTAssertEqual(bare(m.formulaText), "=H1+H2")

        // Delete row 1 (referenced by the formula): H2 shifts up to H1, the formula
        // shifts up to H2, and its destroyed H1 reference becomes #REF!.
        m.select(row: 1, col: 8); m.deleteRow()
        XCTAssertEqual(m.rowCells(2)[7], "#REF!")

        // Columns shift the same way: K1=5, L1 = K1*3 = 15; insert a column at K
        // and the formula (now at M1) keeps pointing at its precedent (now L1).
        let c = WindowedSheetModel()
        c.setCell("K1", "5")
        c.setCell("L1", "=K1*3")
        c.select(row: 1, col: 11); c.insertCol() // col 11 = K
        XCTAssertEqual(c.rowCells(1)[12], "15")  // M1 (col 13 → index 12)
        c.select(row: 1, col: 13)
        XCTAssertEqual(bare(c.formulaText), "=L1*3")
    }

    /// Clipboard copy/cut/paste shifts a formula and moves a cut.
    func testClipboardCopyCutPaste() {
        let m = WindowedSheetModel()
        // Seed H1=5, H2=7; I1 = H1*2 (col 8 = H, col 9 = I). Copy I1, paste at I2 —
        // the relative ref shifts by the destination's offset, so I2 = H2*2.
        m.setCell("H1", "5")
        m.setCell("H2", "7")
        m.setCell("I1", "=H1*2") // 10
        m.select(row: 1, col: 9); m.copyCell() // copy I1
        m.select(row: 2, col: 9)
        XCTAssertTrue(m.pasteCell())           // paste at I2
        XCTAssertEqual(m.rowCells(2)[8], "14") // I2 = H2*2 = 14
        // Cut A1, move it to C1: source clears, a second paste is a no-op.
        m.setCell("A1", "99")
        m.select(row: 1, col: 1); m.cutCell()
        m.select(row: 1, col: 3)
        XCTAssertTrue(m.pasteCell())           // paste at C1
        XCTAssertEqual(m.rowCells(1)[2], "99") // C1 moved
        XCTAssertEqual(m.rowCells(1)[0], "")   // A1 cleared
        m.select(row: 1, col: 5)
        XCTAssertFalse(m.pasteCell())          // buffer consumed
    }

    /// Save / load: serialize the workbook, mutate it, then restore the snapshot
    /// and confirm the workbook comes back — and that a loaded formula stays LIVE
    /// (the document stores source + formats, not computed values).
    func testSaveLoadRoundTrips() {
        let m = WindowedSheetModel()
        // The default seed has A1=15, E1 = SUM(A1:D1) = 38 (format "#,##0.00").
        let snapshot = m.saveBook()
        XCTAssertFalse(snapshot.isEmpty, "serialize produced a JSON document")
        // Mutate away from the saved state so a load has to visibly undo it.
        m.setCell("A1", "500") // E1 → 500+3+12+8 = 523
        XCTAssertEqual(m.window(rows: 1...1, cols: 5...5)[0][0], "523.00")
        // Restore: A1 → 15, E1 recomputes through its format back to "38.00".
        XCTAssertTrue(m.loadBook(snapshot))
        XCTAssertEqual(m.window(rows: 1...1, cols: 1...1)[0][0], "15")
        XCTAssertEqual(m.window(rows: 1...1, cols: 5...5)[0][0], "38.00")
        // The loaded formula is live, not frozen: edit a precedent and E1 recomputes.
        m.setCell("A1", "5") // 5+3+12+8 = 28
        XCTAssertEqual(m.window(rows: 1...1, cols: 5...5)[0][0], "28.00")
        // Garbage in is rejected (false), leaving the workbook intact.
        XCTAssertFalse(m.loadBook("not a workbook"))
        XCTAssertEqual(m.window(rows: 1...1, cols: 5...5)[0][0], "28.00")
    }

    /// Undo / redo: make two edits, walk history back and forward, and confirm a
    /// restored formula recomputes live. (The model seeds its budget via setCell,
    /// so history is non-empty from construction — undoing into the seed is
    /// expected; this test only asserts the round trip of its own two edits.)
    func testUndoRedoWalksHistory() {
        let m = WindowedSheetModel()
        // Two fresh edits on a clear column: H1 = 2 (col 8), I1 = H1*5 = 10 (col 9).
        m.setCell("H1", "2")
        m.setCell("I1", "=H1*5")
        XCTAssertEqual(m.window(rows: 1...1, cols: 9...9)[0][0], "10")
        XCTAssertTrue(m.canUndo())

        // Undo the formula, then the literal.
        XCTAssertTrue(m.undoEdit())
        XCTAssertEqual(m.window(rows: 1...1, cols: 9...9)[0][0], "") // I1 gone
        XCTAssertTrue(m.undoEdit())
        XCTAssertEqual(m.window(rows: 1...1, cols: 8...8)[0][0], "") // H1 gone

        // Redo both: I1 recomputes live (10).
        XCTAssertTrue(m.canRedo())
        XCTAssertTrue(m.redoEdit())
        XCTAssertTrue(m.redoEdit())
        XCTAssertEqual(m.window(rows: 1...1, cols: 9...9)[0][0], "10")
        XCTAssertFalse(m.canRedo())
        XCTAssertFalse(m.redoEdit()) // nothing left to redo

        // A fresh edit forks history (drops the redo branch).
        XCTAssertTrue(m.undoEdit()) // back: I1 gone
        XCTAssertTrue(m.canRedo())
        m.setCell("A1", "7")
        XCTAssertFalse(m.canRedo())
    }

    /// Number format applies to the selected cell's DISPLAY only — the stored
    /// value is untouched, so `getRaw` still returns the source and the
    /// display window renders the cell through the format code. The SwiftUI
    /// proof of the cross-backend "Format" toolbar group.
    func testApplyFormatChangesDisplayNotValue() {
        let m = WindowedSheetModel()
        // Put a plain number in a clear cell (H1, col 8). Unformatted it reads "1234".
        m.select(row: 1, col: 8)
        m.formulaText = "1234"
        m.commitFormula()
        XCTAssertEqual(m.rowCells(1)[7], "1234")

        // Thousands + 2 decimals.
        m.select(row: 1, col: 8); m.applyFormat("#,##0.00")
        XCTAssertEqual(m.rowCells(1)[7], "1,234.00")
        // Percent (1234 → 123400.0%).
        m.applyFormat("0.0%")
        XCTAssertEqual(m.rowCells(1)[7], "123400.0%")
        // Currency.
        m.applyFormat("$#,##0.00")
        XCTAssertEqual(m.rowCells(1)[7], "$1,234.00")
        // The raw stored value is never mutated by formatting: re-selecting H1
        // loads its source (not its formatted display) into the formula bar.
        m.select(row: 1, col: 8)
        XCTAssertEqual(m.formulaText, "1234")
        // Clearing the format (empty code) returns to General.
        m.applyFormat("")
        XCTAssertEqual(m.rowCells(1)[7], "1234")
    }

    /// Range sort (the ▲/▼ Sort buttons drive sortBlock): reorder the seeded
    /// budget block A1:E4 by the selected column. The default seed has column A =
    /// 15,8,12,4 (rows 1..4, unformatted) and each E cell = SUM(row) formatted
    /// "#,##0.00". Sorting by column A ascending moves each row as a record — col
    /// A becomes 4,8,12,15 and every E total travels with its row (the engine
    /// shifts the moved SUM formulas' refs). Descending reverses it.
    func testSortRangeReordersRowsByKeyColumn() {
        let m = WindowedSheetModel()
        // Pre-sort: column A rows 1..4 = 15,8,12,4 (selection defaults to A1).
        XCTAssertEqual(m.window(rows: 1...4, cols: 1...1).map { $0[0] }, ["15", "8", "12", "4"])
        // Ascending by column A → 4,8,12,15.
        XCTAssertTrue(m.sortBlock(true))
        XCTAssertEqual(m.window(rows: 1...4, cols: 1...1).map { $0[0] }, ["4", "8", "12", "15"])
        // Each row's E total tracked its row (formatted "#,##0.00").
        XCTAssertEqual(m.window(rows: 1...1, cols: 5...5)[0][0], "35.00")  // 4+11+3+17
        XCTAssertEqual(m.window(rows: 4...4, cols: 5...5)[0][0], "38.00")  // 15+3+12+8
        // Descending reverses the key order.
        XCTAssertTrue(m.sortBlock(false))
        XCTAssertEqual(m.window(rows: 1...1, cols: 1...1)[0][0], "15")
        XCTAssertEqual(m.window(rows: 4...4, cols: 1...1)[0][0], "4")
    }

    /// Find / replace (the find/replace boxes + Find/Replace buttons drive
    /// findAll/replaceAll): findAll returns the A1 addresses whose SOURCE contains
    /// the query (case-insensitive); replaceAll rewrites the query in every cell's
    /// source and recomputes, returning the count. A rewritten formula stays live;
    /// a rewritten literal stays typed.
    func testFindAndReplaceLocatesAndRewritesCells() {
        let m = WindowedSheetModel()
        // The seed has the literal "15" only at A1, and "=SUM(" in every total formula.
        XCTAssertEqual(m.findAll("15"), ["A1"])
        XCTAssertTrue(m.findAll("sum").contains("E1"))  // case-insensitive
        XCTAssertEqual(m.findAll(""), [])
        XCTAssertEqual(m.findAll("zzz"), [])
        // selectA1 moves the cursor onto a hit (parsing column letters past Z).
        m.selectA1("Z1000")
        XCTAssertEqual(m.selectedRow, 1000)
        XCTAssertEqual(m.selectedCol, 26)
        // Replace a literal: A1 "15" → "99"; E1 = 99+3+12+8 = 122 ("#,##0.00").
        XCTAssertEqual(m.replaceAll("15", "99"), 1)
        XCTAssertEqual(m.rowCells(1)[0], "99")
        XCTAssertEqual(m.rowCells(1)[4], "122.00")
        // Replace inside a formula reference keeps it LIVE: H1=10, H2=20, H3 = =H1+5
        // (15). Rewrite "H1" → "H2" → H3 becomes =H2+5 = 25, recomputed by the engine.
        m.setCell("H1", "10")
        m.setCell("H2", "20")
        m.setCell("H3", "=H1+5") // 15
        XCTAssertEqual(m.rowCells(3)[7], "15")
        XCTAssertEqual(m.replaceAll("H1", "H2"), 1)
        XCTAssertEqual(m.rowCells(3)[7], "25") // =H2+5
    }
}
