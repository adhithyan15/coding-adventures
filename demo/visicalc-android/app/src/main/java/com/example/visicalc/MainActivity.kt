// MainActivity.kt — VisiCalc Android entry point.
//
// Single Activity hosting the Compose surface.  Mirrors the
// `application { Window { ... } }` shape of visicalc-compose
// (Compose Desktop) but uses `setContent { ... }` from
// `androidx.activity.compose.ComponentActivity`, which is the
// idiomatic Compose-on-Android bootstrap.
//
// The VisiCalcApp composable, plus FormulaBar.kt and Grid.kt
// next door, are byte-for-byte identical to their Compose Desktop
// siblings — Jetpack Compose for Android and Compose for Desktop
// share the `androidx.compose.*` package and runtime, so the same
// composables render unmodified on both platforms.

package com.example.visicalc

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.MaterialTheme
import androidx.compose.material.Surface
import androidx.compose.material.Text
import androidx.compose.material.darkColors
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

// 5×5 sample dataset shared with every other visicalc-* demo.
private val sampleRows: List<List<String>> = listOf(
    listOf("15", "3",  "12", "8",  "5"),
    listOf("8",  "14", "7",  "22", "11"),
    listOf("12", "9",  "18", "6",  "25"),
    listOf("4",  "11", "3",  "17", "9"),
    listOf("7",  "5",  "13", "10", "19"),
)

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
    var selectedRow by remember { mutableStateOf(0) }
    var selectedCol by remember { mutableStateOf(0) }
    var formulaText by remember { mutableStateOf("=SUM(B1:B5)") }

    Column(modifier = Modifier.fillMaxSize().padding(16.dp)) {
        Text(
            text = "VISICALC · MOSAIC ANDROID DEMO",
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
