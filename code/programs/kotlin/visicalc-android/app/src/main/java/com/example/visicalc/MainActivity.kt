// MainActivity.kt — VisiCalc Android entry point.
//
// Single Activity hosting the Compose surface.  Mirrors the
// `application { Window { ... } }` shape of visicalc-compose
// (Compose Desktop) but uses `setContent { ... }` from
// `androidx.activity.compose.ComponentActivity`, which is the
// idiomatic Compose-on-Android bootstrap.
//
// UI34 rewire — the VisiCalcApp composable's `FormulaBar` and
// `Grid` are now AUTO-GENERATED into FormulaBar.kt + Grid.kt
// next door by `bash scripts/build.sh`, which runs
// `mosaic-compile --backend compose` against the shared
// `code/programs/mosaic/visicalc/*` triple every other VC2-* demo
// consumes.  Grid.desktop.mll is a UI34
// `pkg::mosaic-pkg-grid::Grid` one-liner; the package resolver
// substitutes the canonical Grid + Cell composition before the
// Compose emitter runs.  No hand-written widgets in this demo.

package com.example.visicalc

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.ui.platform.LocalContext
import androidx.compose.material.MaterialTheme
import androidx.compose.material.Surface
import androidx.compose.material.Button
import androidx.compose.material.Text
import androidx.compose.material.darkColors
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

// The A1 address of a selected (row, col) in the grid's coordinate space:
// column 0 is the row-label gutter (no cell), column 1 → "A"; selectedRow is
// 0-based (0 → sheet row 1).
private fun cellAddress(selectedRow: Double, selectedCol: Double): String =
    "${('A' + selectedCol.toInt() - 1)}${selectedRow.toInt() + 1}"

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme(colors = darkColors(background = Color(0xFF1E1E1E))) {
                Surface(
                    color = Color(0xFF1E1E1E),
                    modifier = Modifier.fillMaxSize(),
                ) {
                    VisiCalcApp()
                }
            }
        }
    }
}

@Composable
private fun VisiCalcApp() {
    // The generated Grid widget uses `Double` for every numeric
    // coordinate (mosaic-emit-compose lowers `number`-typed slots
    // to `Double` so verbatim Expr like `r == editRow` compiles).
    // Column 0 is the row-label gutter; the first data cell is column 1.
    // The Rust engine, reached over JNI (see Engine.kt). Seeded with the shared
    // cross-footing budget, it holds the cells / dependency graph / recalc — the
    // grid below renders ITS computed values, and edits write back through it.
    val engine = remember { Engine() }
    // Free the native session (the boxed SpreadsheetSession on the Rust heap) when
    // this composable leaves the tree, so the handle doesn't leak.
    DisposableEffect(Unit) { onDispose { engine.close() } }
    var viewportRows by remember { mutableStateOf(engine.viewportRows()) }

    var selectedRow by remember { mutableStateOf(0.0) }
    var selectedCol by remember { mutableStateOf(1.0) }
    var editRow     by remember { mutableStateOf(-1.0) }
    var editCol     by remember { mutableStateOf(-1.0) }
    var editContent by remember { mutableStateOf("") }
    // Start showing A1's source ("15") in the bar, like the sibling demos.
    var formulaText by remember { mutableStateOf(engine.rawAt("A1")) }

    // Which FormulaBar LAYOUT is showing: the desktop Row (address label left of
    // the input) or the UI30 touch Column (address label stacked ABOVE a
    // full-width input — the phone arrangement, most apt on Android). Both are
    // generated from the SAME FormulaBar.mil interface, so they share
    // FormulaBarEvent + the dispatch below. "One component, many layouts" made a
    // runtime toggle — the Android sibling of the Qt/Compose/Flutter toggles.
    var touch by remember { mutableStateOf(true) }

    // File open / save via the Storage Access Framework: a document picker for
    // Open, a "create document" flow for Save. The bytes cross the Rust engine's
    // byte codecs (engine.exportBytes / importBytes → the nativeLoad* / nativeSave*
    // JNI methods). One `pendingFormat` selects which codec a launched dialog uses;
    // `fileStatus` echoes the result under the toolbar.
    val context = LocalContext.current
    var pendingFormat by remember { mutableStateOf("xlsx") }
    var fileStatus by remember { mutableStateOf("") }
    val openLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument(),
    ) { uri ->
        if (uri != null) {
            val bytes = context.contentResolver.openInputStream(uri)?.use { it.readBytes() }
                ?: ByteArray(0)
            val ok = engine.importBytes(pendingFormat, bytes)
            if (ok) {
                // The load replaced the whole workbook — re-read the grid + bar.
                viewportRows = engine.viewportRows()
                formulaText =
                    if (selectedCol.toInt() >= 1) engine.rawAt(cellAddress(selectedRow, selectedCol))
                    else ""
            }
            fileStatus = if (ok) "opened .$pendingFormat" else "not a valid .$pendingFormat file"
        }
    }
    val saveLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument("application/octet-stream"),
    ) { uri ->
        if (uri != null) {
            context.contentResolver.openOutputStream(uri)
                ?.use { it.write(engine.exportBytes(pendingFormat)) }
            fileStatus = "saved .$pendingFormat"
        }
    }

    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        Text(
            text = "VISICALC · MOSAIC ANDROID DEMO",
            color = Color(0xFF9D9D9D),
            fontSize = 11.sp,
            fontFamily = FontFamily.Monospace,
        )

        // Flip the formula-bar layout between the desktop Row and the touch
        // Column at runtime — both drive the same engine identically.
        Button(onClick = { touch = !touch }) {
            Text(if (touch) "Desktop bar" else "Touch bar")
        }

        // File: open / save a REAL spreadsheet file via the Storage Access
        // Framework, over the engine's byte codecs (.xlsx keeps live formulas;
        // .csv is values only). Horizontally scrollable so the row fits a phone.
        Row(
            modifier = Modifier.horizontalScroll(rememberScrollState()).padding(top = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Button(onClick = { pendingFormat = "xlsx"; saveLauncher.launch("visicalc-demo.xlsx") }) { Text("Save .xlsx") }
            Button(onClick = { pendingFormat = "xlsx"; openLauncher.launch(arrayOf("*/*")) }) { Text("Open .xlsx") }
            Button(onClick = { pendingFormat = "csv"; saveLauncher.launch("visicalc-demo.csv") }) { Text("Save .csv") }
            Button(onClick = { pendingFormat = "csv"; openLauncher.launch(arrayOf("*/*")) }) { Text("Open .csv") }
        }
        if (fileStatus.isNotEmpty()) {
            Text(
                text = fileStatus,
                color = Color(0xFF9D9D9D),
                fontSize = 11.sp,
                fontFamily = FontFamily.Monospace,
                modifier = Modifier.padding(top = 4.dp),
            )
        }

        Box(modifier = Modifier.padding(top = 8.dp)) {
            // Desktop (Row) vs touch (Column) — both generated from the same
            // FormulaBar.mil, sharing FormulaBarEvent + this dispatch. Only the
            // active variant is composed.
            val fbCellAddress =
                if (selectedCol.toInt() < 1) "${selectedRow.toInt() + 1}"
                else "${('A' + selectedCol.toInt() - 1)}${selectedRow.toInt() + 1}"
            val fbDispatch: (FormulaBarEvent) -> Unit = { event ->
                    when (event) {
                        is FormulaBarEvent.FormulaChange -> formulaText = event.value
                        is FormulaBarEvent.Commit -> {
                            // Write the edit through to the engine and recompute;
                            // every dependent cell (E-column row sums, row-5 column
                            // sums, the grand total) updates in the refreshed grid.
                            if (selectedCol.toInt() >= 1) {
                                val a1 = cellAddress(selectedRow, selectedCol)
                                engine.setCell(a1, formulaText)
                                viewportRows = engine.viewportRows()
                                formulaText = engine.rawAt(a1)
                            }
                        }
                        is FormulaBarEvent.Cancel ->
                            formulaText =
                                if (selectedCol.toInt() >= 1)
                                    engine.rawAt(cellAddress(selectedRow, selectedCol))
                                else ""
                    }
            }
            if (touch) {
                FormulaBarTouch(cellAddress = fbCellAddress, formula = formulaText, readOnly = false, dispatch = fbDispatch)
            } else {
                FormulaBar(cellAddress = fbCellAddress, formula = formulaText, readOnly = false, dispatch = fbDispatch)
            }
        }

        Box(modifier = Modifier.padding(top = 16.dp)) {
            // Grid — generated by mosaic-compile from
            // code/programs/mosaic/visicalc/Grid.*.
            Grid(
                // Leading "" is the empty corner above the row-label
                // gutter; A–E label the five data columns.
                columnHeaders = listOf("", "A", "B", "C", "D", "E"),
                viewportRows = viewportRows,
                columnWidths = listOf(48.0, 96.0, 96.0, 96.0, 96.0, 96.0),
                totalHeight = 0.0,
                selectedRow = selectedRow,
                selectedCol = selectedCol,
                editRow = editRow,
                editCol = editCol,
                editContent = editContent,
                dispatch = { event ->
                    when (event) {
                        is GridEvent.Navigate -> {
                            selectedRow = event.row
                            selectedCol = event.col
                            // Load the newly-selected cell's SOURCE into the bar
                            // (A1 → "15", E1 → "=SUM(A1:D1)") from the engine.
                            formulaText =
                                if (event.col.toInt() >= 1)
                                    engine.rawAt(cellAddress(event.row, event.col))
                                else ""
                        }
                        is GridEvent.FormulaChange -> editContent = event.value
                        is GridEvent.EditCommit -> {
                            editRow = -1.0
                            editCol = -1.0
                        }
                        is GridEvent.EditCancel -> {
                            editRow = -1.0
                            editCol = -1.0
                            editContent = ""
                        }
                    }
                },
            )
        }
    }
}
