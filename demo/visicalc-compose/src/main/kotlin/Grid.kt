// Grid.kt — hand-written placeholder for the Grid composable.
//
// Renders a fixed-size 5×5 grid with column headers (A B C D E),
// row numbers (1 2 3 4 5), and one cell per (row, col).  The selected
// cell gets the excel-blue highlight (`#264F78` background +
// `#007ACC` border) matching Grid.dark.msl.
//
// Visual contract is shared with the other VisiCalc demos.  When
// mosaic-emit-compose grows a `Grid` primitive lowering, this file
// becomes the generated artifact; until then we hand-write it.

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

private val cellWidth = 96.dp
private val cellHeight = 22.dp
private val headerHeight = 24.dp
private val rowLabelWidth = 96.dp

// Excel-style theme tokens (mirror demo/visicalc/mosaic/Grid.dark.msl
// and the corresponding Qt / SwiftUI / Flutter palettes).
private val headerBg     = Color(0xFF2D2D30)
private val headerFg     = Color(0xFF9D9D9D)
private val cellBg       = Color(0xFF1E1E1E)
private val cellBgAlt    = Color(0xFF252526)
private val cellBorder   = Color(0xFF3F3F46)
private val cellFg       = Color(0xFFCCCCCC)
private val selectedBg   = Color(0xFF264F78)
private val selectedBdr  = Color(0xFF007ACC)
private val selectedFg   = Color.White

@Composable
fun Grid(
    rows: List<List<String>>,
    selectedRow: Int,
    selectedCol: Int,
    onCellTap: (row: Int, col: Int) -> Unit,
) {
    Column(
        modifier = Modifier
            .border(1.dp, cellBorder)
            .background(cellBg),
    ) {
        HeaderRow()
        rows.forEachIndexed { rowIdx, row ->
            DataRow(
                rowIdx = rowIdx,
                row = row,
                selectedRow = selectedRow,
                selectedCol = selectedCol,
                onCellTap = onCellTap,
            )
        }
    }
}

@Composable
private fun HeaderRow() {
    Row(modifier = Modifier.fillMaxWidth().height(headerHeight)) {
        // Top-left empty corner cell (above the row-number column).
        HeaderCell(text = "")
        listOf("A", "B", "C", "D", "E").forEach { letter ->
            HeaderCell(text = letter)
        }
    }
}

@Composable
private fun HeaderCell(text: String) {
    Box(
        modifier = Modifier
            .width(cellWidth)
            .height(headerHeight)
            .background(headerBg)
            .border(1.dp, cellBorder),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = text,
            color = headerFg,
            fontSize = 12.sp,
            fontFamily = FontFamily.Monospace,
        )
    }
}

@Composable
private fun DataRow(
    rowIdx: Int,
    row: List<String>,
    selectedRow: Int,
    selectedCol: Int,
    onCellTap: (Int, Int) -> Unit,
) {
    val zebraBg = if (rowIdx % 2 == 0) cellBg else cellBgAlt
    Row(modifier = Modifier.fillMaxWidth().height(cellHeight)) {
        // Row-number cell on the left.
        Box(
            modifier = Modifier
                .width(rowLabelWidth)
                .height(cellHeight)
                .background(headerBg)
                .border(1.dp, cellBorder),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = (rowIdx + 1).toString(),
                color = headerFg,
                fontSize = 12.sp,
                fontFamily = FontFamily.Monospace,
            )
        }
        row.forEachIndexed { colIdx, value ->
            val isSelected = rowIdx == selectedRow && colIdx == selectedCol
            DataCell(
                value = value,
                isSelected = isSelected,
                zebraBg = zebraBg,
                onClick = { onCellTap(rowIdx, colIdx) },
            )
        }
    }
}

@Composable
private fun DataCell(
    value: String,
    isSelected: Boolean,
    zebraBg: Color,
    onClick: () -> Unit,
) {
    Box(
        modifier = Modifier
            .width(cellWidth)
            .height(cellHeight)
            .background(if (isSelected) selectedBg else zebraBg)
            .border(1.dp, if (isSelected) selectedBdr else cellBorder)
            .clickable(onClick = onClick)
            .padding(end = 4.dp),
        contentAlignment = Alignment.CenterEnd,
    ) {
        Text(
            text = value,
            color = if (isSelected) selectedFg else cellFg,
            fontSize = 12.sp,
            fontFamily = FontFamily.Monospace,
        )
    }
}
