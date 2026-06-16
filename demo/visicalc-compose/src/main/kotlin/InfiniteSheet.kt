// InfiniteSheet.kt — a virtualized, effectively-infinite spreadsheet view for
// the Compose Desktop demo, rendered on the shared Rust engine through the
// viewport primitive (the same get_window / used_range / changed_since the
// SwiftUI InfiniteGridView, the Qt InfiniteSheet.qml, and the Flutter
// InfiniteGrid drive).
//
// The sheet is u32 × u32 and sparse; only the cells in the VISIBLE rows are ever
// composed. The body is a `LazyColumn`, which natively virtualizes — it composes
// a row item only while it's near the viewport and recycles it as it scrolls
// off. So a 1000-row-tall sheet costs the handful of rows you can see, and each
// composed row makes ONE engine `get_window` over its 1×totalCols strip
// (InfiniteSheetModel.rowCells). Per-frame engine work is proportional to
// *visible* rows, never to the sheet's height.
//
// Frozen chrome without a second scroller: the row-number gutter rides as each
// LazyColumn row's first child, OUTSIDE the horizontal scroll, so it stays
// pinned on the left and scrolls vertically with the body for free. Every row
// and the column-letter header share ONE horizontal `ScrollState`, so dragging
// any row pans them all in lockstep (the header is gesture-disabled — it only
// follows). Vertical drags fall through the per-row horizontalScroll to the
// LazyColumn.

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

// Cell geometry + palette, matching the sibling infinite views.
private val ROW_H = 24.dp
private val COL_W = 90.dp
private val GUTTER_W = 64.dp
private val HEAD_H = 26.dp

private val BG = Color(0xFF1E1E1E)
private val CHROME = Color(0xFF2D2D30)
private val BORDER = Color(0xFF3F3F46)
private val INK = Color(0xFFCCCCCC)
private val DIM = Color(0xFF9D9D9D)
private val SEL = Color(0xFF094771)
private val MONO = FontFamily.Monospace

/// The virtualized infinite-sheet view. Owns its [InfiniteSheetModel] and the
/// single horizontal scroll state shared by the header and every body row.
@Composable
fun InfiniteSheet() {
    val model = remember { InfiniteSheetModel() }
    DisposableEffect(Unit) { onDispose { model.close() } }

    // The body scrolls horizontally; the header follows the same state.
    val hScroll = rememberScrollState()

    // Compose state that mirrors the model, so edits/selections recompose.
    // `rev` is bumped on commit to force visible rows to re-read rowCells.
    var selRow by remember { mutableStateOf(model.selRow) }
    var selCol by remember { mutableStateOf(model.selCol) }
    var formula by remember { mutableStateOf(model.formula) }
    var rev by remember { mutableStateOf(0) }

    fun select(row: Int, col: Int) {
        model.selectInf(row, col)
        selRow = model.selRow
        selCol = model.selCol
        formula = model.formula
    }

    fun commit() {
        model.commitInf(formula)
        formula = model.formula
        rev++
    }

    Column(modifier = Modifier.fillMaxSize().padding(8.dp)) {
        // ── Formula bar: the selected cell's address + an editable source ──
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(model.infAddress(), DIM, GUTTER_W)
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(28.dp)
                    .background(CHROME)
                    .border(1.dp, BORDER)
                    .padding(horizontal = 6.dp),
                contentAlignment = Alignment.CenterStart,
            ) {
                BasicTextField(
                    value = formula,
                    onValueChange = { formula = it },
                    singleLine = true,
                    textStyle = TextStyle(color = INK, fontSize = 13.sp, fontFamily = MONO),
                    cursorBrush = androidx.compose.ui.graphics.SolidColor(INK),
                    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                    keyboardActions = KeyboardActions(onDone = { commit() }),
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        }

        Spacer(Modifier.height(6.dp))

        // ── Column-letter header (frozen vertically, follows horizontal pan) ──
        Row {
            chromeCell(GUTTER_W, HEAD_H, "") // corner
            Box(modifier = Modifier.horizontalScroll(hScroll, enabled = false)) {
                Row {
                    for (c in 1..model.totalCols) {
                        chromeCell(COL_W, HEAD_H, model.columnLetters(c))
                    }
                }
            }
        }

        // ── Body: virtualized rows, each with a frozen gutter + scrolling cells ──
        LazyColumn(modifier = Modifier.fillMaxSize()) {
            items(model.totalRows) { idx ->
                val rowNum = idx + 1
                // One engine read for the whole row; re-read when `rev` changes.
                val cells = remember(rowNum, rev) { model.rowCells(rowNum) }
                Row {
                    chromeCell(GUTTER_W, ROW_H, "$rowNum") // gutter — frozen left
                    Box(modifier = Modifier.horizontalScroll(hScroll)) {
                        Row {
                            for (c in 1..model.totalCols) {
                                val text = if (c - 1 < cells.size) cells[c - 1] else ""
                                val selected = selRow == rowNum && selCol == c
                                dataCell(text, selected) { select(rowNum, c) }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// A right-aligned, tappable data cell.
@Composable
private fun dataCell(text: String, selected: Boolean, onTap: () -> Unit) {
    Box(
        modifier = Modifier
            .size(COL_W, ROW_H)
            .background(if (selected) SEL else BG)
            .border(0.5.dp, BORDER)
            .clickable(onClick = onTap)
            .padding(end = 4.dp),
        contentAlignment = Alignment.CenterEnd,
    ) {
        androidx.compose.material.Text(
            text = text,
            color = INK,
            fontSize = 12.sp,
            fontFamily = MONO,
            maxLines = 1,
            textAlign = TextAlign.End,
        )
    }
}

/// A frozen header/gutter cell (column letter, row number, or the corner).
@Composable
private fun chromeCell(w: androidx.compose.ui.unit.Dp, h: androidx.compose.ui.unit.Dp, text: String) {
    Box(
        modifier = Modifier.size(w, h).background(CHROME).border(0.5.dp, BORDER),
        contentAlignment = Alignment.Center,
    ) {
        androidx.compose.material.Text(text, color = DIM, fontSize = 12.sp, fontFamily = MONO)
    }
}

/// The formula-bar address label (fixed width).
@Composable
private fun Text(text: String, color: Color, width: androidx.compose.ui.unit.Dp) {
    Box(modifier = Modifier.width(width)) {
        androidx.compose.material.Text(text, color = color, fontSize = 12.sp, fontFamily = MONO)
    }
}
