// InfiniteSheet.kt — a virtualized, effectively-infinite spreadsheet view for
// the Compose Desktop demo, rendered on the shared Rust engine through the
// viewport primitive (the same get_display_window / used_range / changed_since
// the SwiftUI InfiniteGridView, the Qt InfiniteSheet.qml, and the Flutter
// InfiniteGrid drive).
//
// The sheet is u32 × u32 and sparse; only the cells in the VISIBLE rows are ever
// composed. The body is a `LazyColumn`, which natively virtualizes — it composes
// a row item only while it's near the viewport and recycles it as it scrolls
// off. So a 1000-row-tall sheet costs the handful of rows you can see, and each
// composed row makes ONE engine `get_display_window` over its 1×totalCols strip
// (InfiniteSheetModel.rowCells) — display strings, already rendered through each
// cell's format code. Per-frame engine work is proportional to *visible* rows,
// never to the sheet's height.
//
// Frozen chrome without a second scroller: the row-number gutter rides as each
// LazyColumn row's first child, OUTSIDE the horizontal scroll, so it stays
// pinned on the left and scrolls vertically with the body for free. Every row
// and the column-letter header share ONE horizontal `ScrollState`, so dragging
// any row pans them all in lockstep (the header is gesture-disabled — it only
// follows). Vertical drags fall through the per-row horizontalScroll to the
// LazyColumn.
//
// Visually this mirrors the reference design language from the web demo
// (demo/visicalc-html/infinite.html) — a dark, modern-spreadsheet surface built
// from a small set of color tokens: an address pill + `fx` marker + a formula
// field with an accent focus ring, segmented tool-button groups, zebra row
// banding, a 2px accent selection ring with accent-tinted row/col headers, and a
// hairline status footer. The same token set the Qt and Flutter ports use.

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.hoverable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsFocusedAsState
import androidx.compose.foundation.interaction.collectIsHoveredAsState
import androidx.compose.foundation.interaction.collectIsPressedAsState
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
import androidx.compose.foundation.shape.RoundedCornerShape
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
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

// Cell geometry (roomier, to match the web reference).
private val ROW_H = 26.dp
private val COL_W = 92.dp
private val GUTTER_W = 64.dp
private val HEAD_H = 28.dp

// ── Design tokens ───────────────────────────────────────────────────────────
// Mirror demo/visicalc-html/infinite.html's palette so every VisiCalc backend
// reads as one considered surface (dark modern spreadsheet). Same token set as
// the Qt InfiniteSheet.qml / Flutter infinite_grid.dart ports.
private val BG = Color(0xFF16181D) // app / base cell
private val PANEL = Color(0xFF1B1E24) // toolbar + zebra band
private val SURFACE = Color(0xFF21252C) // buttons, pill
private val SURFACE_HOVER = Color(0xFF2B313A)
private val SURFACE_DOWN = Color(0xFF14171C)
private val FIELD = Color(0xFF0F1115) // formula input well
private val LINE = Color(0xFF2C313A) // hairline borders
private val LINE_STRONG = Color(0xFF3A404B) // control borders
private val HEAD = Color(0xFF20242B) // row/col headers
private val HEAD_SEL = Color(0xFF2B3340) // header of selected row/col
private val INK = Color(0xFFE8EAED) // primary text
private val MUTED = Color(0xFF9AA3B2) // labels, headers
private val ACCENT = Color(0xFF4AA3FF) // selection + focus
private val SEL = Color(0xFF21344A) // selected-cell fill
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
    // In-memory "saved file" slot for the Save / Load buttons: Save stows the
    // serialized workbook here, Load restores from it. (A real app would write
    // it to a file; the demo keeps the round trip self-contained.)
    var savedSnapshot by remember { mutableStateOf("") }

    // Find / replace fields + a short status echoed in the footer (match/replace
    // count). The query searches every cell's SOURCE (case-insensitive).
    var findText by remember { mutableStateOf("") }
    var replaceText by remember { mutableStateOf("") }
    var findStatus by remember { mutableStateOf("") }

    // Drives the formula field's accent focus ring.
    val fieldInteraction = remember { MutableInteractionSource() }
    val fieldFocused by fieldInteraction.collectIsFocusedAsState()

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

    // Drag-fill: replicate the selected cell into the 10 rows below it. The engine
    // shifts each copy's relative refs, pins absolute ($) refs, carries the format.
    fun fillDown() {
        model.fillDown(10)
        rev++
    }

    // Clipboard: copy/cut the selected cell, then paste at the selection. The
    // engine shifts the pasted formula's relative refs by the destination's
    // offset, pins absolute ($) refs, carries the format; a cut clears on paste.
    fun copyCell() = model.copyCell()
    fun cutCell() = model.cutCell()
    fun pasteCell() {
        model.pasteCell()
        rev++
    }

    // Save / load: serialize the whole workbook (formulas + formats) to a JSON
    // document held in memory, and restore it. Computed values recompute on load,
    // so a loaded formula stays live; the formula bar re-reads after a load.
    fun saveBook() {
        savedSnapshot = model.saveBook()
    }
    fun loadBook() {
        if (savedSnapshot.isEmpty()) return
        model.loadBook(savedSnapshot)
        formula = model.formula
        rev++
    }

    // Undo / redo: walk the engine's snapshot history. On success the grid
    // re-reads (rev++) and the formula bar re-syncs; the buttons gate off the
    // model's canUndo/canRedo (re-evaluated because rev is read in the layout).
    fun undo() {
        if (!model.undoEdit()) return
        formula = model.formula
        rev++
    }
    fun redo() {
        if (!model.redoEdit()) return
        formula = model.formula
        rev++
    }

    // Structural edits: insert / delete the selected cell's row or column. The
    // engine shifts every formula reference across the band and recomputes; a
    // reference whose whole band is deleted becomes #REF!. Bump rev so the
    // visible rows re-read and re-sync the formula bar.
    fun insertRow() { model.insertRow(); formula = model.formula; rev++ }
    fun deleteRow() { model.deleteRow(); formula = model.formula; rev++ }
    fun insertCol() { model.insertCol(); formula = model.formula; rev++ }
    fun deleteCol() { model.deleteCol(); formula = model.formula; rev++ }

    // Number formatting: apply an Excel-style code to the selected cell. Display-
    // only — bump rev so the visible rows re-read and show the formatted string.
    fun applyFormat(code: String) { model.applyFormat(code); rev++ }

    // Range sort: reorder the budget block A1:E4 by the selected column,
    // ascending/descending. Bump rev so the reordered rows re-read.
    fun sortBlock(ascending: Boolean) { model.sortBlock(ascending); rev++ }

    // Find: locate every cell whose source contains the query (case-insensitive)
    // and jump the selection to the first hit; the footer shows the match count.
    fun runFind() {
        val hits = model.findAll(findText)
        if (hits.isNotEmpty()) {
            model.selectA1(hits.first())
            selRow = model.selRow
            selCol = model.selCol
            formula = model.formula
        }
        findStatus = when {
            findText.isEmpty() -> ""
            hits.isEmpty() -> "no match"
            else -> "${hits.size} match${if (hits.size == 1) "" else "es"}"
        }
    }

    // Replace: rewrite the query → replacement in every cell's source and
    // recompute; the footer shows how many cells changed.
    fun runReplace() {
        val n = model.replaceAll(findText, replaceText)
        formula = model.formula
        rev++
        findStatus = "$n replaced"
    }

    // ── Multi-sheet workbook ──
    // `sheetRev` is bumped on every sheet op so the tab bar recomposes. After a
    // switch/add/delete the model re-primes the selection, so we mirror it back
    // into the Compose state. `renaming` (≥ 0) opens the rename dialog for that
    // tab index; `renameText` holds the in-progress name.
    var sheetRev by remember { mutableStateOf(0) }
    var renaming by remember { mutableStateOf(-1) }
    var renameText by remember { mutableStateOf("") }

    fun syncFromModel() {
        selRow = model.selRow
        selCol = model.selCol
        formula = model.formula
        rev++
        sheetRev++
    }

    fun switchSheet(i: Int) { model.selectSheet(i); syncFromModel() }
    fun deleteSheet(i: Int) { model.deleteSheet(i); syncFromModel() }
    fun addSheet() {
        // Name the new sheet "SheetN" where N avoids a clash with existing names.
        val existing = model.sheetNames().toSet()
        var n = existing.size + 1
        while (existing.contains("Sheet$n")) n++
        model.addSheet("Sheet$n")
        syncFromModel()
    }
    fun commitRename() {
        val i = renaming
        if (i >= 0 && renameText.isNotBlank()) {
            model.renameSheet(i, renameText.trim())
            syncFromModel()
        }
        renaming = -1
    }

    Column(modifier = Modifier.fillMaxSize().background(BG)) {
        // ── Formula bar: a panel holding the address pill, an `fx` marker, the
        // editable source line (with an accent focus ring), and segmented button
        // groups (drag-fill · clipboard · file · history) divided by thin rules.
        Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier
                .fillMaxWidth()
                .padding(start = 10.dp, end = 10.dp, top = 10.dp, bottom = 6.dp)
                .clip(RoundedCornerShape(8.dp))
                .background(PANEL)
                .border(1.dp, LINE, RoundedCornerShape(8.dp))
                .padding(8.dp),
        ) {
            // Address pill.
            Box(
                modifier = Modifier
                    .width(46.dp)
                    .height(30.dp)
                    .clip(RoundedCornerShape(5.dp))
                    .background(SURFACE)
                    .border(1.dp, LINE_STRONG, RoundedCornerShape(5.dp)),
                contentAlignment = Alignment.Center,
            ) {
                androidx.compose.material.Text(
                    model.infAddress(),
                    color = INK,
                    fontSize = 12.sp,
                    fontWeight = FontWeight.Bold,
                    fontFamily = MONO,
                )
            }
            Spacer(Modifier.width(6.dp))
            androidx.compose.material.Text(
                "fx",
                color = MUTED,
                fontSize = 12.sp,
                fontStyle = FontStyle.Italic,
                fontFamily = MONO,
            )
            Spacer(Modifier.width(6.dp))
            // Formula field — accent focus ring on edit.
            Box(
                modifier = Modifier
                    .weight(1f)
                    .height(30.dp)
                    .clip(RoundedCornerShape(5.dp))
                    .background(FIELD)
                    .border(
                        if (fieldFocused) 2.dp else 1.dp,
                        if (fieldFocused) ACCENT else LINE_STRONG,
                        RoundedCornerShape(5.dp),
                    )
                    .padding(horizontal = 8.dp),
                contentAlignment = Alignment.CenterStart,
            ) {
                BasicTextField(
                    value = formula,
                    onValueChange = { formula = it },
                    singleLine = true,
                    interactionSource = fieldInteraction,
                    textStyle = TextStyle(color = INK, fontSize = 13.sp, fontFamily = MONO),
                    cursorBrush = androidx.compose.ui.graphics.SolidColor(ACCENT),
                    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                    keyboardActions = KeyboardActions(onDone = { commit() }),
                    modifier = Modifier.fillMaxWidth(),
                )
            }
            Spacer(Modifier.width(6.dp))
            // ── Drag-fill ──
            toolButton("↓ Fill 10") { fillDown() }
            toolSep()
            // ── Clipboard ──
            toolButton("Copy") { copyCell() }
            Spacer(Modifier.width(6.dp))
            toolButton("Cut") { cutCell() }
            Spacer(Modifier.width(6.dp))
            toolButton("Paste") { pasteCell() }
            toolSep()
            // ── File (save / load) ──
            toolButton("Save") { saveBook() }
            Spacer(Modifier.width(6.dp))
            toolButton("Load", enabled = savedSnapshot.isNotEmpty()) { loadBook() }
            toolSep()
            // ── Structure (insert / delete the selected row or column) ──
            toolButton("+ Row") { insertRow() }
            Spacer(Modifier.width(6.dp))
            toolButton("− Row") { deleteRow() }
            Spacer(Modifier.width(6.dp))
            toolButton("+ Col") { insertCol() }
            Spacer(Modifier.width(6.dp))
            toolButton("− Col") { deleteCol() }
            toolSep()
            // ── Format (apply a number format to the selected cell) ──
            toolButton(".00") { applyFormat("#,##0.00") }
            Spacer(Modifier.width(6.dp))
            toolButton("%") { applyFormat("0.0%") }
            Spacer(Modifier.width(6.dp))
            toolButton("$") { applyFormat("\$#,##0.00") }
            Spacer(Modifier.width(6.dp))
            toolButton("Gen") { applyFormat("") }
            toolSep()
            // ── Sort (reorder the budget block A1:E4 by the selected column) ──
            toolButton("▲ Sort") { sortBlock(true) }
            Spacer(Modifier.width(6.dp))
            toolButton("▼ Sort") { sortBlock(false) }
            toolSep()
            // ── History (undo / redo). Reading `rev` re-evaluates canUndo/canRedo
            // on every edit so the buttons gate live.
            toolButton("↶ Undo", enabled = rev.let { model.canUndo() }) { undo() }
            Spacer(Modifier.width(6.dp))
            toolButton("↷ Redo", enabled = rev.let { model.canRedo() }) { redo() }
            toolSep()
            // ── Find / replace (search cell sources; rewrite matches) ──
            searchField(findText, "find", { findText = it }) { runFind() }
            Spacer(Modifier.width(6.dp))
            toolButton("Find") { runFind() }
            Spacer(Modifier.width(6.dp))
            searchField(replaceText, "replace", { replaceText = it }) { runReplace() }
            Spacer(Modifier.width(6.dp))
            toolButton("Replace") { runReplace() }
        }

        // ── Column-letter header (frozen vertically, follows horizontal pan) ──
        // The selected column's header tints to the accent.
        Row(modifier = Modifier.padding(horizontal = 10.dp)) {
            chromeCell(GUTTER_W, HEAD_H, "") // corner
            Box(modifier = Modifier.horizontalScroll(hScroll, enabled = false)) {
                Row {
                    for (c in 1..model.totalCols) {
                        chromeCell(COL_W, HEAD_H, model.columnLetters(c), selected = selCol == c)
                    }
                }
            }
        }

        // ── Body: virtualized rows, each with a frozen gutter + scrolling cells ──
        LazyColumn(modifier = Modifier.weight(1f).padding(horizontal = 10.dp)) {
            items(model.totalRows) { idx ->
                val rowNum = idx + 1
                // One engine read for the whole row; re-read when `rev` changes.
                val cells = remember(rowNum, rev) { model.rowCells(rowNum) }
                Row {
                    // Gutter — frozen left; the selected row's label tints to accent.
                    chromeCell(GUTTER_W, ROW_H, "$rowNum", selected = selRow == rowNum)
                    Box(modifier = Modifier.horizontalScroll(hScroll)) {
                        Row {
                            for (c in 1..model.totalCols) {
                                val text = if (c - 1 < cells.size) cells[c - 1] else ""
                                val selected = selRow == rowNum && selCol == c
                                dataCell(text, rowNum, selected) { select(rowNum, c) }
                            }
                        }
                    }
                }
            }
        }

        // ── Sheet tab bar (Excel-style, along the bottom) ──
        // One chip per sheet; the active one tints to the accent. Click a tab to
        // switch (bare-A1 ops then address it); the active tab carries inline
        // ✎ rename and ✕ delete affordances, and "+ Sheet" adds one. A formula
        // like `=Summary!B3` reaches across the tabs, so switching and editing
        // here updates every cross-sheet dependent live. Reading `sheetRev`
        // recomposes the bar after any sheet op.
        val (sheetNames, activeSheet) = sheetRev.let { model.sheetNames() to model.activeSheet() }
        Box(modifier = Modifier.fillMaxWidth().padding(horizontal = 10.dp).height(1.dp).background(LINE))
        Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier
                .fillMaxWidth()
                .horizontalScroll(rememberScrollState())
                .padding(start = 10.dp, end = 10.dp, top = 6.dp),
        ) {
            sheetNames.forEachIndexed { i, name ->
                val selected = i == activeSheet
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier
                        .padding(end = 4.dp)
                        .clip(RoundedCornerShape(6.dp))
                        .background(if (selected) SEL else SURFACE)
                        .border(
                            if (selected) 2.dp else 1.dp,
                            if (selected) ACCENT else LINE_STRONG,
                            RoundedCornerShape(6.dp),
                        )
                        .clickable { switchSheet(i) }
                        .padding(horizontal = 10.dp, vertical = 5.dp),
                ) {
                    androidx.compose.material.Text(
                        name,
                        color = if (selected) Color.White else INK,
                        fontSize = 12.sp,
                        fontWeight = if (selected) FontWeight.Bold else FontWeight.Normal,
                        fontFamily = MONO,
                    )
                    if (selected) {
                        // Inline rename / delete affordances on the active tab.
                        Spacer(Modifier.width(8.dp))
                        androidx.compose.material.Text(
                            "✎",
                            color = MUTED,
                            fontSize = 12.sp,
                            fontFamily = MONO,
                            modifier = Modifier
                                .clip(RoundedCornerShape(3.dp))
                                .clickable { renaming = i; renameText = name }
                                .padding(horizontal = 3.dp),
                        )
                        Spacer(Modifier.width(2.dp))
                        androidx.compose.material.Text(
                            "✕",
                            color = MUTED,
                            fontSize = 12.sp,
                            fontFamily = MONO,
                            modifier = Modifier
                                .clip(RoundedCornerShape(3.dp))
                                .clickable { deleteSheet(i) }
                                .padding(horizontal = 3.dp),
                        )
                    }
                }
            }
            Spacer(Modifier.width(6.dp))
            toolButton("+ Sheet") { addSheet() }
        }

        // Rename dialog — a small panel with a text field, opened when `renaming`
        // ≥ 0 (the ✎ on the active tab). Enter or "Rename" commits; "Cancel"
        // dismisses. The engine rewrites every referencing qualifier on commit.
        if (renaming >= 0) {
            androidx.compose.ui.window.Dialog(onDismissRequest = { renaming = -1 }) {
                Column(
                    modifier = Modifier
                        .clip(RoundedCornerShape(8.dp))
                        .background(PANEL)
                        .border(1.dp, LINE, RoundedCornerShape(8.dp))
                        .padding(16.dp),
                ) {
                    androidx.compose.material.Text(
                        "Rename sheet", color = INK, fontSize = 14.sp, fontFamily = MONO,
                    )
                    Spacer(Modifier.height(10.dp))
                    Box(
                        modifier = Modifier
                            .width(220.dp)
                            .height(30.dp)
                            .clip(RoundedCornerShape(5.dp))
                            .background(FIELD)
                            .border(1.dp, LINE_STRONG, RoundedCornerShape(5.dp))
                            .padding(horizontal = 8.dp),
                        contentAlignment = Alignment.CenterStart,
                    ) {
                        BasicTextField(
                            value = renameText,
                            onValueChange = { renameText = it },
                            singleLine = true,
                            textStyle = TextStyle(color = INK, fontSize = 13.sp, fontFamily = MONO),
                            cursorBrush = androidx.compose.ui.graphics.SolidColor(ACCENT),
                            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                            keyboardActions = KeyboardActions(onDone = { commitRename() }),
                            modifier = Modifier.fillMaxWidth(),
                        )
                    }
                    Spacer(Modifier.height(12.dp))
                    Row {
                        toolButton("Cancel") { renaming = -1 }
                        Spacer(Modifier.width(8.dp))
                        toolButton("Rename") { commitRename() }
                    }
                }
            }
        }

        // ── Status line: a hairline-separated footer echoing the live virtual-grid
        // size and the per-edit revision clock (mirrors the web/Qt/Flutter demos).
        Box(modifier = Modifier.fillMaxWidth().padding(horizontal = 10.dp).height(1.dp).background(LINE))
        androidx.compose.material.Text(
            "Virtual grid: ${model.totalRows} rows × ${model.totalCols} cols" +
                "  ·  revision ${rev.let { model.revision() }}" +
                (if (findStatus.isNotEmpty()) "  ·  $findStatus" else ""),
            color = MUTED,
            fontSize = 12.sp,
            fontFamily = MONO,
            modifier = Modifier.padding(start = 10.dp, end = 10.dp, top = 6.dp, bottom = 10.dp),
        )
    }
}

/// A compact search input for the find / replace group — a narrow well with the
/// same accent focus ring as the formula field, a muted placeholder when empty,
/// and Enter-to-submit. The Compose analog of the web demo's find/replace boxes
/// and the Qt/Flutter ports' search fields.
@Composable
private fun searchField(value: String, hint: String, onValueChange: (String) -> Unit, onSubmit: () -> Unit) {
    val interaction = remember { MutableInteractionSource() }
    val focused by interaction.collectIsFocusedAsState()
    Box(
        modifier = Modifier
            .width(96.dp)
            .height(30.dp)
            .clip(RoundedCornerShape(5.dp))
            .background(FIELD)
            .border(
                if (focused) 2.dp else 1.dp,
                if (focused) ACCENT else LINE_STRONG,
                RoundedCornerShape(5.dp),
            )
            .padding(horizontal = 8.dp),
        contentAlignment = Alignment.CenterStart,
    ) {
        if (value.isEmpty()) {
            androidx.compose.material.Text(hint, color = MUTED, fontSize = 12.sp, fontFamily = MONO)
        }
        BasicTextField(
            value = value,
            onValueChange = onValueChange,
            singleLine = true,
            interactionSource = interaction,
            textStyle = TextStyle(color = INK, fontSize = 12.sp, fontFamily = MONO),
            cursorBrush = androidx.compose.ui.graphics.SolidColor(ACCENT),
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
            keyboardActions = KeyboardActions(onDone = { onSubmit() }),
            modifier = Modifier.fillMaxWidth(),
        )
    }
}

/// A thin vertical rule between toolbar button groups.
@Composable
private fun toolSep() {
    Spacer(Modifier.width(6.dp))
    Box(modifier = Modifier.width(1.dp).height(22.dp).background(LINE))
    Spacer(Modifier.width(6.dp))
}

/// A compact, modern toolbar button — a rounded chip with hover / pressed /
/// disabled states, the Compose analog of the web demo's segmented controls and
/// the Qt port's `component ToolButton`.
@Composable
private fun toolButton(label: String, enabled: Boolean = true, onClick: () -> Unit) {
    val interaction = remember { MutableInteractionSource() }
    val hovered by interaction.collectIsHoveredAsState()
    val pressed by interaction.collectIsPressedAsState()
    val bg = when {
        !enabled -> SURFACE
        pressed -> SURFACE_DOWN
        hovered -> SURFACE_HOVER
        else -> SURFACE
    }
    val fg = if (!enabled) MUTED else if (hovered) Color.White else INK
    Box(
        modifier = Modifier
            .height(30.dp)
            .alpha(if (enabled) 1f else 0.6f)
            .clip(RoundedCornerShape(5.dp))
            .background(bg)
            .border(1.dp, LINE_STRONG, RoundedCornerShape(5.dp))
            .hoverable(interaction, enabled = enabled)
            .clickable(interactionSource = interaction, indication = null, enabled = enabled) { onClick() }
            .padding(horizontal = 11.dp),
        contentAlignment = Alignment.Center,
    ) {
        androidx.compose.material.Text(label, color = fg, fontSize = 12.sp, fontFamily = MONO)
    }
}

/// A right-aligned, tappable data cell. Selected → accent fill + 2px accent ring;
/// otherwise a zebra band (even rows take the panel tint).
@Composable
private fun dataCell(text: String, rowNum: Int, selected: Boolean, onTap: () -> Unit) {
    val band = if (rowNum % 2 == 0) PANEL else BG
    Box(
        modifier = Modifier
            .size(COL_W, ROW_H)
            .background(if (selected) SEL else band)
            .border(if (selected) 2.dp else 0.5.dp, if (selected) ACCENT else LINE)
            .clickable(onClick = onTap)
            .padding(end = 6.dp),
        contentAlignment = Alignment.CenterEnd,
    ) {
        androidx.compose.material.Text(
            text = text,
            color = if (selected) Color.White else INK,
            fontSize = 12.sp,
            fontWeight = if (selected) FontWeight.Bold else FontWeight.Normal,
            fontFamily = MONO,
            maxLines = 1,
            textAlign = TextAlign.End,
        )
    }
}

/// A frozen header/gutter cell (column letter, row number, or the corner).
/// When [selected] (its row/column holds the cursor) it tints to the accent.
@Composable
private fun chromeCell(w: Dp, h: Dp, text: String, selected: Boolean = false) {
    Box(
        modifier = Modifier
            .size(w, h)
            .background(if (selected) HEAD_SEL else HEAD)
            .border(0.5.dp, LINE),
        contentAlignment = Alignment.Center,
    ) {
        androidx.compose.material.Text(
            text,
            color = if (selected) ACCENT else MUTED,
            fontSize = 11.sp,
            fontWeight = FontWeight.Bold,
            fontFamily = MONO,
        )
    }
}
