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
    println(if (failures == 0) "\nALL PASS" else "\n$failures FAILURE(S)")
    kotlin.system.exitProcess(if (failures == 0) 0 else 1)
}
