// EngineSmoke.kt — headless proof that the Compose VisiCalc demo does REAL
// formula work on the shared Rust engine, with no Compose UI in the loop. This
// is the Kotlin sibling of the SwiftUI demo's `swift test`, the Qt demo's
// tst_model, and the Flutter demo's engine_test.dart: it drives the same
// engine-backed SpreadsheetModel the Compose UI binds to (Engine.kt) and asserts
// the values are engine-computed and recompute on edit.
//
// It's a plain `fun main()` — NOT a Compose app — so it compiles with kotlinc
// and runs on the JVM with the Java FFM API enabled, loading the vendored engine
// library. scripts/verify.sh wires it up:
//
//   kotlinc src/main/kotlin/Engine.kt test/EngineSmoke.kt -include-runtime -d t.jar
//   java --enable-preview --enable-native-access=ALL-UNNAMED -cp t.jar EngineSmokeKt
//
// A green run proves the Kotlin ↔ Java FFM ↔ C ABI ↔ Rust path end-to-end.

private var failures = 0

private fun check(label: String, got: String, want: String) {
    val ok = got == want
    if (!ok) failures++
    println("${if (ok) "ok  " else "FAIL"}  $label: got=\"$got\" want=\"$want\"")
}

private fun checkContains(label: String, got: String, needle: String) {
    val ok = got.contains(needle)
    if (!ok) failures++
    println("${if (ok) "ok  " else "FAIL"}  $label: \"$got\" contains \"$needle\"")
}

fun main() {
    val model = SpreadsheetModel()

    // Seeded cross-footing budget — computed by the engine, not hard-coded.
    var rows = model.viewportRows()
    check("E1 row total", rows[0][5], "38")   // 15+3+12+8
    check("E2 row total", rows[1][5], "51")   // 8+14+7+22
    check("A5 col total", rows[4][1], "39")   // 15+8+12+4
    check("E5 grand total", rows[4][5], "169")
    check("row-label gutter", rows[0][0], "1")

    // Editing A1 (display row 0, col 1) 15 -> 115 recomputes every dependent.
    model.setCell(0, 1, "115")
    rows = model.viewportRows()
    check("A1 after edit", rows[0][1], "115")
    check("E1 after edit", rows[0][5], "138") // 115+3+12+8
    check("A5 after edit", rows[4][1], "139") // 115+8+12+4
    check("E5 after edit", rows[4][5], "269") // 138+51+45+35

    // A formula that divides by zero, and an error propagating through a binary op.
    model.setCell(0, 1, "=1/0")               // A1
    checkContains("A1 div-by-0", model.valueJson("A1"), "#DIV/0!")
    model.setCell(0, 2, "=A1+1")              // B1
    checkContains("B1 propagated", model.valueJson("B1"), "#DIV/0!")
    check("B1 display", model.viewportRows()[0][2], "#DIV/0!")
    model.close()

    // ── Viewport primitive (virtualized infinite sheet) ──────────────
    // A fresh session seeded with the cross-foot budget + a far-flung formula at
    // Z1000 (row 1000, col 26), to exercise the windowed reads.
    val s = SpreadsheetSession()
    for ((a, v) in listOf(
        "A1" to "15", "B1" to "3", "C1" to "12", "D1" to "8", "E1" to "=SUM(A1:D1)",
        "A2" to "8", "B2" to "14", "C2" to "7", "D2" to "22", "E2" to "=SUM(A2:D2)",
        "A3" to "12", "B3" to "9", "C3" to "18", "D3" to "6", "E3" to "=SUM(A3:D3)",
        "A4" to "4", "B4" to "11", "C4" to "3", "D4" to "17", "E4" to "=SUM(A4:D4)",
        "A5" to "=SUM(A1:A4)", "E5" to "=SUM(E1:E4)", "Z1000" to "=SUM(A1:A4)",
    )) s.setCell(a, v)

    // Window over A1:E5 — engine-computed and dense.
    val w = s.window(1, 1, 5, 5)
    check("window A1", w[0][0], "15")
    check("window E1", w[0][4], "38")
    check("window E5", w[4][4], "169")
    // A window 1000 rows down reaches the far formula; the gap is empty.
    check("window Z1000", s.window(998, 24, 1002, 28)[2][2], "39")
    check("window gap empty", s.window(100, 1, 110, 10)[5][5], "")
    // Extent + letters past Z.
    val u = s.usedRange()!!
    check("usedRange maxRow", u["maxRow"].toString(), "1000")
    check("usedRange maxCol", u["maxCol"].toString(), "26")
    check("columnLetters 27", s.columnLetters(27), "AA")
    check("columnLetters 53", s.columnLetters(53), "BA")
    // Editing A1 dirties the far dependent Z1000 via changedSince.
    val rev = s.currentRevision()
    s.setCell("A1", "115")
    val (changed, stale) = s.changedSince(rev)
    check("changedSince not stale", stale.toString(), "false")
    checkContains("changedSince has A1", changed.joinToString(","), "A1")
    checkContains("changedSince reaches Z1000", changed.joinToString(","), "Z1000")
    check("window Z1000 after edit", s.window(1000, 26, 1000, 26)[0][0], "139")
    s.close()

    // ── Infinite-view binding layer (InfiniteSheetModel) ─────────────
    // The model InfiniteSheet.kt drives: one engine read per visible row via
    // rowCells, tap-to-select via selectInf (loading the cell's source into the
    // formula bar), and write-through via commitInf (recompute + regrow extent).
    val inf = InfiniteSheetModel()
    // The constructor seeds the budget PLUS far-flung cells (Z1000, BA50/BB50)
    // and computes the extent: at least 1000×60, grown to reach the far cells.
    check("inf totalRows >= 1000", (inf.totalRows >= 1000).toString(), "true")
    check("inf totalCols >= 60", (inf.totalCols >= 60).toString(), "true")

    // rowCells returns one row's display strings (columns 1..totalCols).
    val row1 = inf.rowCells(1)
    check("inf rowCells width", (row1.size == inf.totalCols).toString(), "true")
    check("inf rowCells A1", row1[0], "15") // unformatted
    check("inf rowCells E1", row1[4], "38.00")  // SUM(A1:D1), "#,##0.00" formatted
    check("inf rowCells J1 empty", row1[9], "") // sparse
    check("inf gap row blank", inf.rowCells(200).all { it.isEmpty() }.toString(), "true")

    // selectInf loads the cell SOURCE (A5 is a formula) and clamps to the grid.
    inf.selectInf(5, 1)
    check("inf select A5 addr", inf.infAddress(), "A5")
    check("inf select A5 formula", inf.formula, "=SUM(A1:A4)")
    inf.selectInf(-3, 0) // clamps to (1,1)
    check("inf clamp row", inf.selRow.toString(), "1")
    check("inf clamp col", inf.selCol.toString(), "1")

    // commitInf writes through and recomputes every dependent.
    inf.selectInf(2, 1)          // A2
    inf.commitInf("108")         // 8 -> 108
    check("inf commit A2", inf.rowCells(2)[0], "108") // unformatted
    check("inf commit E2", inf.rowCells(2)[4], "151.00") // 108+14+7+22, formatted
    check("inf commit A5", inf.rowCells(5)[0], "139.00") // 15+108+12+4, formatted
    check("inf commit E5", inf.rowCells(5)[4], "269.00") // grand total, formatted

    // fillDown replicates the selected cell, shifting relative refs per target.
    // Seed a fresh column via select+commit: H1=2, H2=3, H3=4 (col 8 = H);
    // I1 = H1*10 (col 9 = I). Select I1 and fill down 10 — each filled formula
    // tracks its row (I2 = H2*10 = 30, I3 = H3*10 = 40), and I1 stays untouched.
    inf.selectInf(1, 8); inf.commitInf("2")        // H1
    inf.selectInf(2, 8); inf.commitInf("3")        // H2
    inf.selectInf(3, 8); inf.commitInf("4")        // H3
    inf.selectInf(1, 9); inf.commitInf("=H1*10")   // I1 = 20
    inf.selectInf(1, 9)
    inf.fillDown(10)
    check("inf fillDown I2", inf.rowCells(2)[8], "30") // I2 = H2*10
    check("inf fillDown I3", inf.rowCells(3)[8], "40") // I3 = H3*10
    check("inf fillDown I1 source", inf.rowCells(1)[8], "20") // I1 untouched

    // Clipboard: copy I1 (= H1*10) and paste at I4 — the relative ref shifts by
    // the destination's offset, so I4 = H4*10. (H4 unset → 0, so seed it.)
    inf.selectInf(4, 8); inf.commitInf("6")        // H4 = 6
    inf.selectInf(1, 9); inf.copyCell()            // copy I1
    inf.selectInf(4, 9); check("inf pasteCell applied", inf.pasteCell().toString(), "true")
    check("inf paste I4 = H4*10", inf.rowCells(4)[8], "60") // I4 = H4*10 = 60
    // Cut A1 and move it to C1: source clears, a second paste is a no-op.
    inf.selectInf(1, 1); inf.commitInf("99")       // A1
    inf.selectInf(1, 1); inf.cutCell()
    inf.selectInf(1, 3); check("inf cut paste applied", inf.pasteCell().toString(), "true")
    check("inf cut moved C1", inf.rowCells(1)[2], "99") // C1 (col 3, index 2)
    check("inf cut cleared A1", inf.rowCells(1)[0], "") // A1 cleared
    inf.selectInf(1, 5); check("inf cut buffer consumed", inf.pasteCell().toString(), "false")
    inf.close()

    // Save / load (the Save / Load buttons drive saveBook/loadBook): serialize a
    // fresh seeded workbook, mutate it, restore the snapshot, and confirm the
    // loaded formula stays LIVE (the document stores source + formats, not values).
    val sl = InfiniteSheetModel()
    val snapshot = sl.saveBook()
    check("serialize non-empty", snapshot.isNotEmpty().toString(), "true")
    sl.selectInf(1, 1); sl.commitInf("500")              // E1 → 500+3+12+8 = 523
    check("mutated E1", sl.rowCells(1)[4], "523.00")
    check("loadBook ok", sl.loadBook(snapshot).toString(), "true")
    check("loaded A1", sl.rowCells(1)[0], "15")          // restored
    check("loaded E1 formatted", sl.rowCells(1)[4], "38.00") // recomputed through format
    sl.selectInf(1, 1); sl.commitInf("5")                // live: 5+3+12+8 = 28
    check("loaded formula live", sl.rowCells(1)[4], "28.00")
    check("loadBook rejects garbage", sl.loadBook("not a workbook").toString(), "false")
    check("workbook intact after reject", sl.rowCells(1)[4], "28.00")
    sl.close()

    // Undo / redo (the Undo / Redo buttons drive undoEdit/redoEdit): a fresh,
    // unseeded session so the initial history is empty. Two edits, walk back and
    // forward, and confirm a restored formula recomputes live.
    val ur = SpreadsheetSession()
    check("fresh canUndo false", ur.canUndo().toString(), "false")
    ur.setCell("A1", "1")
    ur.setCell("B1", "=A1*10") // 10
    check("after edits canUndo true", ur.canUndo().toString(), "true")
    check("undo formula", ur.undo().toString(), "true")
    check("B1 cleared by undo", ur.window(1, 2, 1, 2)[0][0], "")
    check("undo literal", ur.undo().toString(), "true")
    check("A1 cleared by undo", ur.window(1, 1, 1, 1)[0][0], "")
    check("canUndo false at bottom", ur.canUndo().toString(), "false")
    check("undo at bottom is noop", ur.undo().toString(), "false")
    check("redo literal", ur.redo().toString(), "true")
    check("redo formula", ur.redo().toString(), "true")
    check("B1 live after redo", ur.window(1, 2, 1, 2)[0][0], "10")
    check("canRedo false at top", ur.canRedo().toString(), "false")
    // A fresh edit forks history (drops the redo branch).
    ur.undo() // back: B1 gone
    check("canRedo true before fork", ur.canRedo().toString(), "true")
    ur.setCell("C1", "9")
    check("fresh edit clears redo", ur.canRedo().toString(), "false")
    ur.close()

    // Structural edits (the + Row / − Row / + Col / − Col buttons drive
    // insertRows/deleteRows/insertCols/deleteCols): inserting and deleting
    // rows/columns shifts every formula reference across the band, and deleting a
    // referenced band turns that reference into #REF!. The engine parenthesizes
    // binary ops on re-emit ("=(A1+A3)"), so compare with parens stripped.
    val st = SpreadsheetSession()
    st.setCell("A1", "10"); st.setCell("A2", "20"); st.setCell("A3", "=A1+A2") // 30
    check("struct A3 before", st.window(3, 1, 3, 1)[0][0], "30")
    st.insertRows(2, 1)
    check("struct inserted row blank", st.window(2, 1, 2, 1)[0][0], "")
    check("struct formula at A4", st.window(4, 1, 4, 1)[0][0], "30")
    check("struct insert shifted refs", st.getRaw("A4").replace("(", "").replace(")", ""), "=A1+A3")
    st.deleteRows(2, 1)
    check("struct delete shifted back", st.getRaw("A3").replace("(", "").replace(")", ""), "=A1+A2")
    st.deleteRows(1, 1) // delete the referenced row 1 → A1 ref destroyed
    check("struct deleted ref is #REF!", st.window(2, 1, 2, 1)[0][0], "#REF!")
    st.close()
    // Columns shift the same way: K1=5, L1 = K1*3 = 15; insert a col at K → M1.
    val sc = SpreadsheetSession()
    sc.setCell("K1", "5"); sc.setCell("L1", "=K1*3")
    sc.insertCols(11, 1) // col 11 = K
    check("struct insertCol value", sc.window(1, 13, 1, 13)[0][0], "15") // M1
    check("struct insertCol shifted refs", sc.getRaw("M1").replace("(", "").replace(")", ""), "=L1*3")
    sc.close()

    // Number formatting (the .00 / % / $ / Gen buttons drive setFormat): applying
    // a format code changes only how the cell DISPLAYS; the stored value is
    // unchanged. An empty code clears the format back to General.
    val fmt = SpreadsheetSession()
    fmt.setCell("A1", "1234")
    check("fmt unformatted", fmt.window(1, 1, 1, 1)[0][0], "1234")
    fmt.setFormat("A1", "#,##0.00")
    check("fmt #,##0.00", fmt.window(1, 1, 1, 1)[0][0], "1,234.00")
    fmt.setFormat("A1", "0.0%")
    check("fmt 0.0%", fmt.window(1, 1, 1, 1)[0][0], "123400.0%")
    fmt.setFormat("A1", "\$#,##0.00")
    check("fmt $", fmt.window(1, 1, 1, 1)[0][0], "\$1,234.00")
    fmt.setFormat("A1", "")
    check("fmt cleared", fmt.window(1, 1, 1, 1)[0][0], "1234")
    check("fmt raw untouched", fmt.getRaw("A1"), "1234") // display-only
    fmt.close()

    // Range sort (the ▲/▼ Sort buttons drive sortRange): reorder the budget block
    // A1:E4 by a key column. Each row moves as a record — the E-column SUM formulas
    // travel with their row (the engine shifts the refs), so totals stay correct.
    val so = SpreadsheetSession()
    for ((a, v) in listOf(
        "A1" to "15", "B1" to "3", "C1" to "12", "D1" to "8", "E1" to "=SUM(A1:D1)",
        "A2" to "8", "B2" to "14", "C2" to "7", "D2" to "22", "E2" to "=SUM(A2:D2)",
        "A3" to "12", "B3" to "9", "C3" to "18", "D3" to "6", "E3" to "=SUM(A3:D3)",
        "A4" to "4", "B4" to "11", "C4" to "3", "D4" to "17", "E4" to "=SUM(A4:D4)",
    )) so.setCell(a, v)
    check("sort pre A1", so.window(1, 1, 1, 1)[0][0], "15")
    check("sort applied asc", so.sortRange("A1", "E4", 1, true).toString(), "true")
    check("sort A1 asc", so.window(1, 1, 1, 1)[0][0], "4")    // col A → 4,8,12,15
    check("sort A4 asc", so.window(4, 1, 4, 1)[0][0], "15")
    check("sort E1 asc", so.window(1, 5, 1, 5)[0][0], "35")   // E tracks row: 4+11+3+17
    check("sort E4 asc", so.window(4, 5, 4, 5)[0][0], "38")   // 15+3+12+8
    check("sort applied desc", so.sortRange("A1", "E4", 1, false).toString(), "true")
    check("sort A1 desc", so.window(1, 1, 1, 1)[0][0], "15")
    check("sort single-row no-op", so.sortRange("A1", "A1", 1, true).toString(), "false")
    check("sort bad key no-op", so.sortRange("A1", "E4", 9, true).toString(), "false")
    so.close()

    // Find / replace (the find/replace fields + Find/Replace buttons drive
    // findAll/replaceAll): findAll returns the A1 addresses whose SOURCE contains
    // the query (case-insensitive); replaceAll rewrites the query in every cell's
    // source and recomputes, returning the count. A rewritten formula stays live;
    // a rewritten literal stays typed.
    val fr = InfiniteSheetModel()
    // The seed has the literal "15" only at A1, and "=SUM(" in every total formula.
    check("find literal 15", fr.findAll("15").joinToString(","), "A1")
    checkContains("find formula SUM has E1", fr.findAll("sum").joinToString(","), "E1")
    check("find empty query", fr.findAll("").size.toString(), "0")
    check("find no match", fr.findAll("zzz").size.toString(), "0")
    // selectA1 moves the cursor onto a hit (parsing column letters past Z).
    fr.selectA1("Z1000")
    check("selectA1 Z1000 addr", fr.infAddress(), "Z1000")
    // Replace a literal: A1 "15" → "99"; E1 = 99+3+12+8 = 122 (#,##0.00 format).
    check("replace 15→99 count", fr.replaceAll("15", "99").toString(), "1")
    fr.selectA1("A1")
    check("replaced A1 value", fr.rowCells(1)[0], "99")
    check("replaced E1 recomputed", fr.rowCells(1)[4], "122.00")
    // Replace inside a formula reference keeps it LIVE: H1=10, H2=20, H3 = =H1+5
    // (15). Rewrite "H1" → "H2" → H3 becomes =H2+5 = 25, recomputed by the engine.
    fr.selectInf(1, 8); fr.commitInf("10")     // H1
    fr.selectInf(2, 8); fr.commitInf("20")     // H2
    fr.selectInf(3, 8); fr.commitInf("=H1+5")  // H3 = 15
    check("pre-replace H3", fr.rowCells(3)[7], "15")
    check("replace H1→H2 count", fr.replaceAll("H1", "H2").toString(), "1")
    check("H3 recomputed live", fr.rowCells(3)[7], "25") // =H2+5
    fr.close()

    println(if (failures == 0) "\nALL PASS" else "\n$failures FAILURE(S)")
    kotlin.system.exitProcess(if (failures == 0) 0 else 1)
}
