// Main.kt — entry point for the VisiCalc Compose Desktop demo.
//
// Mounts the FormulaBar above a hand-written 5×5 grid.  Both are
// driven from local Compose state (`remember { mutableStateOf(...) }`);
// when the mosaic-emit-compose backend lands, the FormulaBar will be
// replaced by a generated composable and the local state will move
// to a `MosaicStore<AppState>` per the strict-Flux contract.
//
// Visual contract: identical to the React / HTML / WebComponent /
// SwiftUI / Qt / Flutter demos — dark theme, A1 selected (excel-blue
// highlight), formula bar reads "A1 / =SUM(B1:B5)".

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
private val sampleRows: List<List<String>> = listOf(
    listOf("15", "3",  "12", "8",  "5"),
    listOf("8",  "14", "7",  "22", "11"),
    listOf("12", "9",  "18", "6",  "25"),
    listOf("4",  "11", "3",  "17", "9"),
    listOf("7",  "5",  "13", "10", "19"),
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
    var selectedRow by remember { mutableStateOf(0) }
    var selectedCol by remember { mutableStateOf(0) }
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
                cellAddress = "${('A' + selectedCol)}${selectedRow + 1}",
                formula = formulaText,
                onFormulaChange = { newValue -> formulaText = newValue },
                onCommit = { /* no-op for v0.1.0 */ },
                onCancel = { formulaText = sampleRows[selectedRow][selectedCol] },
            )
        }

        Box(modifier = Modifier.padding(top = 16.dp)) {
            Grid(
                rows = sampleRows,
                selectedRow = selectedRow,
                selectedCol = selectedCol,
                onCellTap = { row, col ->
                    selectedRow = row
                    selectedCol = col
                    formulaText = sampleRows[row][col]
                },
            )
        }
    }
}
