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
    inf.close()

    println(if (failures == 0) "\nALL PASS" else "\n$failures FAILURE(S)")
    kotlin.system.exitProcess(if (failures == 0) 0 else 1)
}
