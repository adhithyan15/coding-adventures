// Main.kt — entry point for the VisiCalc Compose Desktop demo.
//
// Mounts a *generated* `FormulaBar` composable (from
// `src/main/kotlin/generated/FormulaBar.kt`, produced by
// `mosaic-compile --backend compose` against
// `demo/visicalc/mosaic/FormulaBar.{mil,desktop.mll,dark.msl}`)
// above a hand-written 5×5 grid.  Both surface and dispatch shape
// match what every sibling backend ships.
//
// Strict-Flux wiring: the FormulaBar receives a single
// `dispatch: (FormulaBarEvent) -> Unit` callback (per
// `code/specs/UI33-rewrite-unified-architecture.md` §6).  This file
// pattern-matches the event union back into local `mutableStateOf`
// for v0.1.0; a follow-up will swap the local state for a
// `MosaicStore<AppState>` from the `mosaic-flux-compose` runtime
// (already pulled in as an `includeBuild` composite-build dep).
//
// Visual contract: identical to the React / HTML / WebComponent /
// SwiftUI / Qt / Flutter demos — dark theme, A1 selected
// (excel-blue highlight), formula bar reads "A1 / =SUM(B1:B5)".

import generated.FormulaBar
import generated.FormulaBarEvent
import generated.Grid
import generated.GridEvent
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.MaterialTheme
import androidx.compose.material.Surface
import androidx.compose.material.Text
import androidx.compose.material.darkColors
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Window
import androidx.compose.ui.window.application
import androidx.compose.ui.window.WindowState
import androidx.compose.ui.window.rememberWindowState

// The 5×5 sample dataset shared with every other visicalc-* demo.
// Hard-coded here for the v0.1.0 visual parity exercise — when the
// strict-Flux contract is wired up, this moves into `AppState.cells`.
// The first cell of every row is the spreadsheet's row-number gutter
// ("1".."5"), mirroring the row-label column the React / HTML / SwiftUI
// demos render down the left edge.  The five data columns A–E follow.
private val sampleRows: List<List<String>> = listOf(
    listOf("1", "15", "3",  "12", "8",  "5"),
    listOf("2", "8",  "14", "7",  "22", "11"),
    listOf("3", "12", "9",  "18", "6",  "25"),
    listOf("4", "4",  "11", "3",  "17", "9"),
    listOf("5", "7",  "5",  "13", "10", "19"),
)

fun main() = application {
    Window(
        onCloseRequest = ::exitApplication,
        title = "VisiCalc — Mosaic Compose demo",
        state = rememberWindowState(width = 720.dp, height = 520.dp),
    ) {
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

@androidx.compose.runtime.Composable
private fun VisiCalcApp() {
    // Local state: selected cell + formula text.  Updates from the
    // formula bar feed `formulaText`; clicks on the grid update both
    // the selection and the formula text.  This is the strict-Flux
    // pattern at micro scale — the components don't mutate state
    // directly, they invoke callbacks that the host owns.
    // The generated Grid widget uses `Double` for every numeric
    // coordinate (mosaic-emit-compose lowers `number`-typed slots
    // to `Double` and casts For-loop indices to `Double` too so
    // verbatim Expr like `r == editRow` compiles cleanly).  The
    // host mirrors that on its state for a clean pass-through.
    var selectedRow by remember { mutableStateOf(0.0) }
    // Column 0 is the row-number gutter, so the first *data* column (A)
    // is index 1.  Start with A1 selected to match the sibling demos.
    var selectedCol by remember { mutableStateOf(1.0) }
    var editRow     by remember { mutableStateOf(-1.0) }
    var editCol     by remember { mutableStateOf(-1.0) }
    var editContent by remember { mutableStateOf("") }
    var formulaText by remember { mutableStateOf("=SUM(B1:B5)") }

    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        // Title row — kebab-cased label up top, matching the
        // sibling demos' "VISICALC · MOSAIC <BACKEND> DEMO" style.
        Text(
            text = "VISICALC · MOSAIC COMPOSE DEMO",
            color = Color(0xFF9D9D9D),
            fontSize = 11.sp,
            fontFamily = FontFamily.Monospace,
        )

        Box(modifier = Modifier.padding(top = 8.dp)) {
            FormulaBar(
                // Column 0 is the gutter, so subtract 1 to map the
                // selected data column back to a spreadsheet letter
                // (col 1 → 'A', col 2 → 'B', …).
                cellAddress = "${('A' + selectedCol.toInt() - 1)}${selectedRow.toInt() + 1}",
                formula = formulaText,
                readOnly = false,
                dispatch = { event ->
                    when (event) {
                        is FormulaBarEvent.FormulaChange -> formulaText = event.value
                        is FormulaBarEvent.Commit -> { /* no-op for v0.1.0 */ }
                        is FormulaBarEvent.Cancel ->
                            formulaText = sampleRows[selectedRow.toInt()][selectedCol.toInt()]
                    }
                },
            )
        }

        Box(modifier = Modifier.padding(top = 16.dp)) {
            // Grid — generated by mosaic-compile.  The signature
            // comes from `mosaic-pkg-grid::Grid` after UI34 PR-3's
            // resolver inlines the package composition.
            Grid(
                // Leading "" is the empty header above the row-number
                // gutter column; A–E label the five data columns.
                columnHeaders = listOf("", "A", "B", "C", "D", "E"),
                viewportRows = sampleRows,
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
                            val r = event.row.toInt()
                            val c = event.col.toInt()
                            if (r in sampleRows.indices &&
                                c in sampleRows[r].indices) {
                                formulaText = sampleRows[r][c]
                            }
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
