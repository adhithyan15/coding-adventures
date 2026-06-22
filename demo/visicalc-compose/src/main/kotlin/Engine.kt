// Engine.kt — the Compose/Kotlin host glue for the VisiCalc demo, computing on
// the shared Rust `spreadsheet-core` engine through its C ABI (spreadsheet-capi).
//
// This is the Kotlin sibling of the SwiftUI demo's Engine.swift, the Qt demo's
// SpreadsheetModel, and the Flutter demo's lib/engine.dart: it owns NO
// spreadsheet logic. The Rust engine (cells, dependency graph, recalc, formulas)
// lives behind the C ABI; this file marshals Kotlin Strings across it and maps
// the engine's JSON value shape into display text — the same engine, and the
// same JSON contract, the web demos drive as WebAssembly.
//
// It reaches the C ABI through the Java Foreign Function & Memory API
// (java.lang.foreign) — the same zero-JNI path Compose Desktop and Android use.
// On JDK 21 the FFM API is a preview feature, so run with
//   --enable-preview --enable-native-access=ALL-UNNAMED
// (it's stable from JDK 22). No third-party FFI dependency.

import java.lang.foreign.Arena
import java.lang.foreign.FunctionDescriptor
import java.lang.foreign.Linker
import java.lang.foreign.MemorySegment
import java.lang.foreign.SymbolLookup
import java.lang.foreign.ValueLayout
import java.io.File

/// A single spreadsheet session, owning the opaque C handle.
class SpreadsheetSession(libraryPath: String = resolveLibraryPath()) : AutoCloseable {
    private val linker = Linker.nativeLinker()
    private val lib = SymbolLookup.libraryLookup(libraryPath, Arena.global())

    private fun handle(name: String, desc: FunctionDescriptor) =
        linker.downcallHandle(lib.find(name).get(), desc)

    private val ptr = ValueLayout.ADDRESS
    private val scNew = handle("sc_session_new", FunctionDescriptor.of(ptr))
    private val scFree = handle("sc_session_free", FunctionDescriptor.ofVoid(ptr))
    // sc_set_cell(session, a1, raw) -> char*
    private val scSet = handle("sc_set_cell", FunctionDescriptor.of(ptr, ptr, ptr, ptr))
    // sc_get_value(session, a1) -> char*  /  sc_get_raw(session, a1) -> char*
    private val scGetValue = handle("sc_get_value", FunctionDescriptor.of(ptr, ptr, ptr))
    private val scGetRaw = handle("sc_get_raw", FunctionDescriptor.of(ptr, ptr, ptr))
    private val scStrFree = handle("sc_string_free", FunctionDescriptor.ofVoid(ptr))

    // Viewport primitive: integer coords (JAVA_INT) and a u64 revision
    // (JAVA_LONG); JSON char* results, except current_revision returns the long.
    private val i32 = ValueLayout.JAVA_INT
    private val i64 = ValueLayout.JAVA_LONG
    private val scGetWindow = handle("sc_get_window", FunctionDescriptor.of(ptr, ptr, i32, i32, i32, i32))
    private val scGetDisplayWindow = handle("sc_get_display_window", FunctionDescriptor.of(ptr, ptr, i32, i32, i32, i32))
    // sc_set_format(session, a1, code) -> void (empty code clears the format).
    private val scSetFormat = handle("sc_set_format", FunctionDescriptor.ofVoid(ptr, ptr, ptr))
    // sc_fill(session, src, dst_start, dst_end) -> void (drag-fill; three A1 strings).
    private val scFill = handle("sc_fill", FunctionDescriptor.ofVoid(ptr, ptr, ptr, ptr))
    // sc_sort_range(session, start, end, key_col, ascending) -> int (1 applied /
    // already sorted, 0 no-op). Two A1 strings + a 1-based key column + a flag.
    private val scSortRange = handle("sc_sort_range", FunctionDescriptor.of(i32, ptr, ptr, ptr, i32, i32))
    // sc_copy / sc_cut(session, start, end) -> void (clipboard capture; two A1 strings).
    private val scCopy = handle("sc_copy", FunctionDescriptor.ofVoid(ptr, ptr, ptr))
    private val scCut = handle("sc_cut", FunctionDescriptor.ofVoid(ptr, ptr, ptr))
    // sc_paste(session, dst_start) -> int (1 applied, 0 no-op).
    private val scPaste = handle("sc_paste", FunctionDescriptor.of(i32, ptr, ptr))
    // sc_insert_rows / sc_delete_rows / sc_insert_cols / sc_delete_cols(session,
    // at, count) -> void. Structural edits at a 1-based position; the engine
    // shifts every formula reference across the band.
    private val scInsertRows = handle("sc_insert_rows", FunctionDescriptor.ofVoid(ptr, i32, i32))
    private val scDeleteRows = handle("sc_delete_rows", FunctionDescriptor.ofVoid(ptr, i32, i32))
    private val scInsertCols = handle("sc_insert_cols", FunctionDescriptor.ofVoid(ptr, i32, i32))
    private val scDeleteCols = handle("sc_delete_cols", FunctionDescriptor.ofVoid(ptr, i32, i32))
    // sc_serialize(session) -> char* (workbook source + formats as JSON);
    // sc_deserialize(session, data) -> int (1 loaded, 0 malformed/unsupported).
    private val scSerialize = handle("sc_serialize", FunctionDescriptor.of(ptr, ptr))
    private val scDeserialize = handle("sc_deserialize", FunctionDescriptor.of(i32, ptr, ptr))
    // sc_undo / sc_redo / sc_can_undo / sc_can_redo(session) -> int (1/0).
    private val scUndo = handle("sc_undo", FunctionDescriptor.of(i32, ptr))
    private val scRedo = handle("sc_redo", FunctionDescriptor.of(i32, ptr))
    private val scCanUndo = handle("sc_can_undo", FunctionDescriptor.of(i32, ptr))
    private val scCanRedo = handle("sc_can_redo", FunctionDescriptor.of(i32, ptr))
    private val scUsedRange = handle("sc_used_range", FunctionDescriptor.of(ptr, ptr))
    private val scColumnLetters = handle("sc_column_letters", FunctionDescriptor.of(ptr, ptr, i32))
    private val scCurrentRevision = handle("sc_current_revision", FunctionDescriptor.of(i64, ptr))
    private val scChangedSince = handle("sc_changed_since", FunctionDescriptor.of(ptr, ptr, i64))

    private val session = scNew.invoke() as MemorySegment

    /// Read an engine-returned char* into a Kotlin String and free it with the
    /// engine's allocator (sc_string_free — NOT libc free). NULL becomes "".
    private fun take(p: MemorySegment): String {
        if (p.address() == 0L) return ""
        // The returned pointer is unbounded; reinterpret so we can read the
        // NUL-terminated string, then hand it back to the engine to free.
        val s = p.reinterpret(Long.MAX_VALUE).getUtf8String(0)
        scStrFree.invoke(p)
        return s
    }

    fun setCell(a1: String, raw: String): String = Arena.ofConfined().use { a ->
        take(scSet.invoke(session, a.allocateUtf8String(a1), a.allocateUtf8String(raw)) as MemorySegment)
    }

    fun getValueJson(a1: String): String = Arena.ofConfined().use { a ->
        take(scGetValue.invoke(session, a.allocateUtf8String(a1)) as MemorySegment)
    }

    fun getRaw(a1: String): String = Arena.ofConfined().use { a ->
        take(scGetRaw.invoke(session, a.allocateUtf8String(a1)) as MemorySegment)
    }

    /// The display string for a cell — what a spreadsheet should show. Parses
    /// the engine's JSON (the fixed shape the TS/WASM/Swift/Qt/Flutter engines
    /// emit); the keys are serde-sorted, so e.g. errors are {"code":…,"kind":…}.
    fun display(a1: String): String {
        val json = getValueJson(a1)
        val kind = Regex("\"kind\":\"(\\w+)\"").find(json)?.groupValues?.get(1) ?: return ""
        return when (kind) {
            "empty" -> ""
            "number" -> {
                val raw = Regex("\"value\":(-?[0-9.eE+]+)").find(json)?.groupValues?.get(1)
                    ?: return ""
                val d = raw.toDouble()
                // Show integers without a trailing ".0".
                if (d == Math.floor(d) && Math.abs(d) < 1e15) d.toLong().toString() else d.toString()
            }
            "text" -> Regex("\"value\":\"(.*)\"").find(json)?.groupValues?.get(1) ?: ""
            "boolean" -> if (json.contains("\"value\":true")) "TRUE" else "FALSE"
            "error" -> Regex("\"code\":\"([^\"]+)\"").find(json)?.groupValues?.get(1) ?: "#ERR"
            else -> ""
        }
    }

    // ── Viewport primitive (virtualized infinite sheet) ──────────────
    // These mirror the engine's get_window / used_range / changed_since reads
    // (1-based inclusive coords) so a windowed Compose grid can render only the
    // visible rectangle of an unbounded sheet — the Compose sibling of the
    // web/SwiftUI/Qt/Flutter infinite views. The window JSON is nested, so these
    // use the small JSON parser below rather than display()'s per-value regex.

    /// Set a cell's display format code (an Excel-style code like "#,##0.00" or
    /// "0%"); an empty code clears it. Drives the engine's display path that
    /// [window] reads through sc_get_display_window.
    fun setFormat(a1: String, code: String): Unit = Arena.ofConfined().use { a ->
        scSetFormat.invoke(session, a.allocateUtf8String(a1), a.allocateUtf8String(code))
    }

    /// Drag-fill: replicate the `src` cell across the inclusive A1 rectangle
    /// `dstStart`..`dstEnd`. Relative references shift per target (`=A1` filled
    /// one row down becomes `=A2`), absolute (`$`) refs pin, off-grid refs become
    /// `#REF!`; the source's display format rides along. The engine recomputes
    /// every dependent. Reaches sc_fill — the same path the web/SwiftUI/Qt/Flutter
    /// demos drive.
    fun fill(src: String, dstStart: String, dstEnd: String): Unit = Arena.ofConfined().use { a ->
        scFill.invoke(
            session,
            a.allocateUtf8String(src),
            a.allocateUtf8String(dstStart),
            a.allocateUtf8String(dstEnd),
        )
    }

    /// Range sort: reorder the rows of the rectangle [start]..[end] by the
    /// computed values in [keyCol] (1-based, inside the rectangle), ascending or
    /// descending. Each row moves as a record; the engine shifts moved formulas'
    /// references with their row and carries formats. Returns true when a sort was
    /// applied (or the range was already sorted), false for a no-op. [keyCol] is
    /// clamped to ≥ 0 before the call (Kotlin Int maxes below u32, so no high-end
    /// truncation); the engine validates it lies inside the rectangle.
    fun sortRange(start: String, end: String, keyCol: Int, ascending: Boolean): Boolean =
        Arena.ofConfined().use { a ->
            (scSortRange.invoke(
                session,
                a.allocateUtf8String(start),
                a.allocateUtf8String(end),
                maxOf(0, keyCol),
                if (ascending) 1 else 0,
            ) as Int) != 0
        }

    /// Structural edits: insert / delete [count] rows or columns at the 1-based
    /// position [at]. The engine shifts every formula reference at or after the
    /// band (a reference whose whole band is deleted becomes `#REF!`), then
    /// recomputes. [at]/[count] are clamped to ≥ 0 before the call so a negative
    /// can't reach the u32 C ABI as a huge unsigned band (Kotlin Int maxes below
    /// u32, so no high-end truncation).
    fun insertRows(at: Int, count: Int) = scInsertRows.invoke(session, maxOf(0, at), maxOf(0, count))
    fun deleteRows(at: Int, count: Int) = scDeleteRows.invoke(session, maxOf(0, at), maxOf(0, count))
    fun insertCols(at: Int, count: Int) = scInsertCols.invoke(session, maxOf(0, at), maxOf(0, count))
    fun deleteCols(at: Int, count: Int) = scDeleteCols.invoke(session, maxOf(0, at), maxOf(0, count))

    /// Copy the inclusive rectangle [start]..[end] into the clipboard — a
    /// whole-block copy that pastes as a unit. The source is untouched; the
    /// buffer survives any number of pastes.
    fun copy(start: String, end: String): Unit = Arena.ofConfined().use { a ->
        scCopy.invoke(session, a.allocateUtf8String(start), a.allocateUtf8String(end))
    }

    /// Cut the inclusive rectangle [start]..[end]. Like [copy] but a one-shot
    /// move: the [paste] that places it clears the source it didn't overwrite.
    fun cut(start: String, end: String): Unit = Arena.ofConfined().use { a ->
        scCut.invoke(session, a.allocateUtf8String(start), a.allocateUtf8String(end))
    }

    /// Paste the clipboard so its top-left lands at [dstStart]. Returns `true`
    /// when applied, `false` (a no-op) for an empty clipboard, malformed address,
    /// or off-grid destination. The block's references shift by the destination's
    /// offset; content and format ride along.
    fun paste(dstStart: String): Boolean = Arena.ofConfined().use { a ->
        (scPaste.invoke(session, a.allocateUtf8String(dstStart)) as Int) != 0
    }

    /// Serialize the whole workbook to a self-contained JSON document — the
    /// SOURCE (formula text + typed literals) + per-cell formats, not the
    /// computed values (those recompute on load, so the document is small and
    /// can't disagree with itself). [take] frees the engine's char*; the host
    /// persists the returned string wherever it likes.
    fun serialize(): String = take(scSerialize.invoke(session) as MemorySegment)

    /// Replace the workbook from a document produced by [serialize]. Returns
    /// `true` on success, `false` for malformed / unsupported input (the workbook
    /// is left untouched — the engine validates before it mutates). Formulas
    /// reload live.
    fun deserialize(data: String): Boolean = Arena.ofConfined().use { a ->
        (scDeserialize.invoke(session, a.allocateUtf8String(data)) as Int) != 0
    }

    /// Undo / redo: walk the engine's snapshot history. Each returns `true` if it
    /// changed the document (the host then re-reads the viewport), `false` if
    /// there was nothing to do. canUndo/canRedo gate a host's Undo/Redo controls.
    fun undo(): Boolean = (scUndo.invoke(session) as Int) != 0
    fun redo(): Boolean = (scRedo.invoke(session) as Int) != 0
    fun canUndo(): Boolean = (scCanUndo.invoke(session) as Int) != 0
    fun canRedo(): Boolean = (scCanRedo.invoke(session) as Int) != 0

    /// Dense display strings for the inclusive 1-based rectangle, row-major
    /// (empty cells become ""). Empty list on a bad/oversized request.
    ///
    /// Reads sc_get_display_window: each cell arrives already rendered through its
    /// format code as a display string, so the host paints it directly and never
    /// re-derives number formatting. The format-aware sibling of sc_get_window;
    /// the JSON is {...,"cells":[["1,234.50",…],…]}.
    fun window(row0: Int, col0: Int, row1: Int, col1: Int): List<List<String>> {
        val json = take(scGetDisplayWindow.invoke(session, row0, col0, row1, col1) as MemorySegment)
        val obj = parseJson(json) as? Map<*, *> ?: return emptyList()
        val cells = obj["cells"] as? List<*> ?: return emptyList()
        return cells.map { row ->
            (row as List<*>).map { it as? String ?: "" }
        }
    }

    /// The data extent {minRow,minCol,maxRow,maxCol}, or null if the sheet is
    /// empty (the engine returns the JSON literal `null`).
    fun usedRange(): Map<String, Int>? {
        val obj = parseJson(take(scUsedRange.invoke(session) as MemorySegment)) as? Map<*, *>
            ?: return null
        return mapOf(
            "minRow" to (obj["minRow"] as Number).toInt(),
            "minCol" to (obj["minCol"] as Number).toInt(),
            "maxRow" to (obj["maxRow"] as Number).toInt(),
            "maxCol" to (obj["maxCol"] as Number).toInt(),
        )
    }

    /// Column letters for a 1-based index (1 -> "A", 27 -> "AA").
    fun columnLetters(index: Int): String =
        take(scColumnLetters.invoke(session, index) as MemorySegment)

    /// The per-edit revision clock. Snapshot it, then pass to changedSince.
    fun currentRevision(): Long = scCurrentRevision.invoke(session) as Long

    /// Cells changed since `since`; the Boolean is `stale` (re-read everything).
    fun changedSince(since: Long): Pair<List<String>, Boolean> {
        val obj = parseJson(take(scChangedSince.invoke(session, since) as MemorySegment))
            as? Map<*, *> ?: return Pair(emptyList(), false)
        if (obj["stale"] == true) return Pair(emptyList(), true)
        val changed = (obj["changed"] as? List<*>)?.map { it as String } ?: emptyList()
        return Pair(changed, false)
    }

    override fun close() {
        scFree.invoke(session)
    }

    companion object {
        /// Resolve the vendored engine library: an explicit CAPI_LIB env var, or
        /// `native/libspreadsheet_capi.*` found by walking up from the working dir.
        fun resolveLibraryPath(): String {
            val os = System.getProperty("os.name").lowercase()
            val name = when {
                os.contains("mac") -> "libspreadsheet_capi.dylib"
                os.contains("win") -> "spreadsheet_capi.dll"
                else -> "libspreadsheet_capi.so"
            }
            System.getenv("CAPI_LIB")?.let { if (File(it).exists()) return it }
            var dir: File? = File(System.getProperty("user.dir"))
            repeat(5) {
                val candidate = File(dir, "native/$name")
                if (candidate.exists()) return candidate.absolutePath
                dir = dir?.parentFile
            }
            return "native/$name"
        }
    }
}

/// An engine-backed 5×5 spreadsheet the Compose host drives. Mirrors the
/// SwiftUI / Qt / Flutter models: seeds the cross-footing budget, exposes the
/// computed display matrix, and writes through to the engine on edit.
class SpreadsheetModel(
    private val session: SpreadsheetSession = SpreadsheetSession(),
) : AutoCloseable {

    init {
        seed()
    }

    /// The classic cross-footing budget — identical seed to every other demo:
    /// column E totals each row, row 5 totals each column, E5 the grand total.
    private fun seed() {
        val cells = listOf(
            "A1" to "15", "B1" to "3", "C1" to "12", "D1" to "8", "E1" to "=SUM(A1:D1)",
            "A2" to "8", "B2" to "14", "C2" to "7", "D2" to "22", "E2" to "=SUM(A2:D2)",
            "A3" to "12", "B3" to "9", "C3" to "18", "D3" to "6", "E3" to "=SUM(A3:D3)",
            "A4" to "4", "B4" to "11", "C4" to "3", "D4" to "17", "E4" to "=SUM(A4:D4)",
            "A5" to "=SUM(A1:A4)", "B5" to "=SUM(B1:B4)", "C5" to "=SUM(C1:C4)",
            "D5" to "=SUM(D1:D4)", "E5" to "=SUM(E1:E4)",
        )
        for ((a1, raw) in cells) session.setCell(a1, raw)
    }

    /// Display matrix fed to the Grid: each row is [rowLabel, A, B, C, D, E].
    fun viewportRows(): List<List<String>> = (0 until ROWS).map { r ->
        listOf("${r + 1}") + (1..COLS).map { c -> session.display(address(r, c)) }
    }

    /// The raw source of the cell at display row/col (col 1..5; 0 = gutter).
    fun rawAt(r: Int, c: Int): String = if (c < 1) "" else session.getRaw(address(r, c))

    /// The value JSON of a cell — used by the verify harness to assert directly.
    fun valueJson(a1: String): String = session.getValueJson(a1)

    /// Write `raw` into the cell at display row/col. The caller rebuilds the
    /// display matrix afterwards via [viewportRows].
    fun setCell(r: Int, c: Int, raw: String) {
        if (c >= 1) session.setCell(address(r, c), raw)
    }

    override fun close() = session.close()

    companion object {
        const val ROWS = 5
        const val COLS = 5 // A..E

        /// A1 address for grid display row `r` (0-based) and column `c` (1..5).
        fun address(r: Int, c: Int): String = "${'A' + c - 1}${r + 1}"
    }
}

/// Engine-backed model for the VIRTUALIZED infinite sheet — the Kotlin sibling
/// of the SwiftUI `WindowedSheetModel`, the Qt `SpreadsheetModel` infinite-view
/// state, and the Flutter `InfiniteSheetModel`. It seeds a deliberately
/// far-flung, sparse dataset and exposes one-row windowed reads plus the data
/// extent, so a `LazyColumn`-virtualized Compose grid can render only the
/// visible rectangle of an effectively-unbounded (u32 × u32) sheet.
///
/// Plain Kotlin (no Compose types): the host `@Composable` mutates it and bumps
/// a `revision` state to recompose, exactly as `Main.kt` drives [SpreadsheetModel].
/// All coordinates here are 1-based (row/col ≥ 1, col 1 = "A"), matching the engine.
class InfiniteSheetModel(
    private val session: SpreadsheetSession = SpreadsheetSession(),
) : AutoCloseable {

    /// The virtual grid size, derived from the data extent plus a margin so you
    /// can scroll past the data into blank space.
    var totalRows: Int = 1000
        private set
    var totalCols: Int = 60
        private set

    /// The selected cell (1-based) and the formula-bar text (its raw source).
    var selRow: Int = 1
        private set
    var selCol: Int = 1
        private set
    var formula: String = ""
        private set

    init {
        seed()
        computeExtent()
        selectInf(1, 1) // prime the selection + formula bar at A1
    }

    /// The classic cross-footing budget PLUS far-flung cells (a formula at
    /// `Z1000`, a couple near `BA50`/`BB50`) to prove the sheet is sparse and
    /// unbounded — identical seed to the SwiftUI/Qt/Flutter infinite views.
    private fun seed() {
        val cells = listOf(
            "A1" to "15", "B1" to "3", "C1" to "12", "D1" to "8", "E1" to "=SUM(A1:D1)",
            "A2" to "8", "B2" to "14", "C2" to "7", "D2" to "22", "E2" to "=SUM(A2:D2)",
            "A3" to "12", "B3" to "9", "C3" to "18", "D3" to "6", "E3" to "=SUM(A3:D3)",
            "A4" to "4", "B4" to "11", "C4" to "3", "D4" to "17", "E4" to "=SUM(A4:D4)",
            "A5" to "=SUM(A1:A4)", "B5" to "=SUM(B1:B4)", "C5" to "=SUM(C1:C4)",
            "D5" to "=SUM(D1:D4)", "E5" to "=SUM(E1:E4)",
            "Z1000" to "=SUM(A1:A4)", // 1000 rows down: 39
            "BA50" to "far cell", "BB50" to "=Z1000*2", // col 53/54, row 50: 78
        )
        for ((a1, raw) in cells) session.setCell(a1, raw)

        // Attach Excel-style format codes so the engine's display path is visible
        // in the windowed view (which renders via sc_get_display_window): the
        // cross-foot totals read with thousands grouping + two decimals, and the
        // far-flung Z1000 total as a percent. Values are unchanged — only how the
        // display strings render. Identical to the web/Qt/Flutter demos' formats.
        val formats = listOf(
            "E1" to "#,##0.00", "E2" to "#,##0.00", "E3" to "#,##0.00",
            "E4" to "#,##0.00", "E5" to "#,##0.00",
            "A5" to "#,##0.00", "B5" to "#,##0.00", "C5" to "#,##0.00", "D5" to "#,##0.00",
            "Z1000" to "0.0%", // 39 -> "3900.0%": proves the format applies far off-origin
        )
        for ((a1, code) in formats) session.setFormat(a1, code)
    }

    /// Re-derive the virtual grid size from the engine's data extent plus a
    /// comfortable margin. Mirrors `WindowedSheetModel.resize()`.
    ///
    /// The `+ margin` is done in `Long` and saturated back into `Int` before use:
    /// the engine is u32-backed, so a `maxRow`/`maxCol` near `Int.MAX_VALUE`
    /// would otherwise overflow the 32-bit add to a negative size — which would
    /// feed `LazyColumn(items(...))` a negative count and invert the `coerceIn`
    /// range. Not reachable in this demo (the only far cell is the fixed Z1000),
    /// but the saturation makes the model safe against any sheet, per the
    /// recorded "u32-overflow-defeats-cap" lesson.
    fun computeExtent() {
        val u = session.usedRange()
        totalRows = saturate((u?.get("maxRow") ?: 1).toLong() + 200, floor = 1000)
        totalCols = saturate((u?.get("maxCol") ?: 1).toLong() + 30, floor = 60)
    }

    /// Clamp a widened (`Long`) extent into a sane positive `Int`: at least
    /// [floor], at most [Int.MAX_VALUE].
    private fun saturate(value: Long, floor: Int): Int =
        value.coerceIn(floor.toLong(), Int.MAX_VALUE.toLong()).toInt()

    /// Column letters for a 1-based index (1 -> "A", 27 -> "AA").
    fun columnLetters(index: Int): String = session.columnLetters(index)

    /// The A1 address of the selected cell (e.g. "Z1000").
    fun infAddress(): String = "${session.columnLetters(selCol)}$selRow"

    /// The engine's per-edit revision clock — bumps on every mutation. The
    /// status footer shows it so the live recompute is visible while scrolling.
    fun revision(): Long = session.currentRevision()

    /// One row's display strings (columns 1..totalCols) — what a virtualized
    /// `LazyColumn` item renders. A single engine `get_window` over a 1×N strip;
    /// returns an empty list if the request was rejected/oversized.
    fun rowCells(row: Int): List<String> {
        if (row < 1) return emptyList()
        val w = session.window(row, 1, row, totalCols)
        return if (w.isEmpty()) emptyList() else w[0]
    }

    /// Move the selection (clamped to the virtual grid; row/col ≥ 1) and pull the
    /// selected cell's raw source into the formula bar.
    fun selectInf(row: Int, col: Int) {
        selRow = row.coerceIn(1, totalRows)
        selCol = col.coerceIn(1, totalCols)
        formula = session.getRaw(infAddress())
    }

    /// Commit the formula bar into the selected cell: write through to the engine
    /// (which recomputes every dependent), grow the extent if the edit reached new
    /// ground, and re-read the canonicalised source back into the bar.
    fun commitInf(raw: String) {
        session.setCell(infAddress(), raw)
        computeExtent()
        formula = session.getRaw(infAddress())
    }

    /// Drag-fill: replicate the selected cell into the [rows] rows below it. The
    /// engine shifts each copy's relative references (`=A1`→`=A2`, …), pins
    /// absolute (`$`) refs, carries the format, and recomputes every dependent.
    /// Regrows the extent if the fill reached new ground. The Kotlin sibling of
    /// the Flutter `InfiniteSheetModel.fillDown` and the Qt "Fill ↓ 10" button.
    fun fillDown(rows: Int) {
        val col = session.columnLetters(selCol)
        val first = "$col${selRow + 1}"
        val last = "$col${selRow + rows}"
        session.fill(infAddress(), first, last)
        computeExtent()
    }

    /// Structural edits: insert / delete the selected cell's row or column. The
    /// engine shifts every formula reference at or after the band (a reference
    /// whose whole band is deleted becomes `#REF!`) and recomputes; regrow the
    /// extent so the view re-reads. Operate on a single row/column at the cursor.
    fun insertRow() { session.insertRows(selRow, 1); computeExtent() }
    fun deleteRow() { session.deleteRows(selRow, 1); computeExtent() }
    fun insertCol() { session.insertCols(selCol, 1); computeExtent() }
    fun deleteCol() { session.deleteCols(selCol, 1); computeExtent() }

    /// Number formatting: attach an Excel-style format code to the selected cell
    /// ("#,##0.00", "0.0%", "$#,##0.00", or "" to clear). Display-only — the
    /// stored value is unchanged; the engine renders it through the code, so a
    /// fresh rowCells read shows the formatted string.
    fun applyFormat(code: String) = session.setFormat(infAddress(), code)

    /// Range sort: reorder the rows of the seeded budget block A1:E4 by the
    /// SELECTED column (clamped into the block's columns A..E = 1..5), ascending
    /// or descending. Each row moves as a record; the E-column SUM formulas travel
    /// with their row (the engine shifts their refs), so every total stays correct.
    /// Returns false for a no-op (already sorted / bad args). Regrows the extent.
    fun sortBlock(ascending: Boolean): Boolean {
        val keyCol = selCol.coerceIn(1, 5)
        val ok = session.sortRange("A1", "E4", keyCol, ascending)
        computeExtent()
        return ok
    }

    /// Clipboard: copy/cut the selected cell, then paste it at the selection. The
    /// engine shifts the pasted formula's relative references by the
    /// destination's offset, pins absolute (`$`) refs, carries the format; a cut
    /// clears the source on paste. [pasteCell] returns false (a no-op) when the
    /// clipboard is empty, and regrows the extent on success.
    fun copyCell() = session.copy(infAddress(), infAddress())
    fun cutCell() = session.cut(infAddress(), infAddress())
    fun pasteCell(): Boolean {
        val ok = session.paste(infAddress())
        if (ok) computeExtent()
        return ok
    }

    /// Save / load: serialize the whole workbook to a JSON document, and restore
    /// it. The document stores only the source + formats — computed values
    /// recompute on load, so a loaded formula stays live. [loadBook] returns
    /// false (workbook untouched) for malformed input; on success it regrows the
    /// extent and refreshes the formula bar so the view re-reads.
    fun saveBook(): String = session.serialize()
    fun loadBook(data: String): Boolean {
        val ok = session.deserialize(data)
        if (ok) {
            computeExtent()
            formula = session.getRaw(infAddress())
        }
        return ok
    }

    /// Undo / redo: walk the engine's snapshot history. On success the extent
    /// regrows and the formula bar refreshes (any cell could have changed); a
    /// restored formula stays live. canUndo/canRedo gate the buttons.
    fun canUndo(): Boolean = session.canUndo()
    fun canRedo(): Boolean = session.canRedo()
    fun undoEdit(): Boolean {
        val ok = session.undo()
        if (ok) {
            computeExtent()
            formula = session.getRaw(infAddress())
        }
        return ok
    }
    fun redoEdit(): Boolean {
        val ok = session.redo()
        if (ok) {
            computeExtent()
            formula = session.getRaw(infAddress())
        }
        return ok
    }

    override fun close() = session.close()
}

// ─────────────────────────────────────────────────────────────────────
// A small JSON reader, just enough for the engine's output (objects,
// arrays, strings, numbers, true/false/null). The window read is nested
// (`{"values":[[{…},…],…]}`), which display()'s per-value regex can't
// handle; rather than add a dependency, we parse it here. The engine emits
// ASCII cell text and no \uXXXX escapes, so the minimal escape handling is
// sufficient for this trusted input.
// ─────────────────────────────────────────────────────────────────────

private fun parseJson(s: String): Any? = if (s.isEmpty()) null else JsonReader(s).readValue()

private class JsonReader(private val s: String) {
    private var i = 0
    private fun ws() { while (i < s.length && s[i].isWhitespace()) i++ }

    fun readValue(): Any? {
        ws()
        return when (s[i]) {
            '{' -> readObject()
            '[' -> readArray()
            '"' -> readString()
            't' -> { i += 4; true }
            'f' -> { i += 5; false }
            'n' -> { i += 4; null }
            else -> readNumber()
        }
    }

    private fun readObject(): Map<String, Any?> {
        val m = LinkedHashMap<String, Any?>()
        i++ // {
        ws()
        if (s[i] == '}') { i++; return m }
        while (true) {
            ws()
            val k = readString()
            ws(); i++ // :
            m[k] = readValue()
            ws()
            if (s[i] == ',') { i++; continue }
            i++ // }
            break
        }
        return m
    }

    private fun readArray(): List<Any?> {
        val l = ArrayList<Any?>()
        i++ // [
        ws()
        if (s[i] == ']') { i++; return l }
        while (true) {
            l.add(readValue())
            ws()
            if (s[i] == ',') { i++; continue }
            i++ // ]
            break
        }
        return l
    }

    private fun readString(): String {
        val sb = StringBuilder()
        i++ // opening quote
        while (s[i] != '"') {
            if (s[i] == '\\') {
                i++
                sb.append(
                    when (s[i]) {
                        'n' -> '\n'; 't' -> '\t'; 'r' -> '\r'
                        '"' -> '"'; '\\' -> '\\'; '/' -> '/'
                        else -> s[i]
                    }
                )
            } else {
                sb.append(s[i])
            }
            i++
        }
        i++ // closing quote
        return sb.toString()
    }

    private fun readNumber(): Double {
        val start = i
        while (i < s.length && (s[i].isDigit() || s[i] in "-+.eE")) i++
        return s.substring(start, i).toDouble()
    }
}
