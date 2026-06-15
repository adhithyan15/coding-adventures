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

    /// Dense display strings for the inclusive 1-based rectangle, row-major
    /// (empty cells become ""). Empty list on a bad/oversized request.
    fun window(row0: Int, col0: Int, row1: Int, col1: Int): List<List<String>> {
        val json = take(scGetWindow.invoke(session, row0, col0, row1, col1) as MemorySegment)
        val obj = parseJson(json) as? Map<*, *> ?: return emptyList()
        val values = obj["values"] as? List<*> ?: return emptyList()
        return values.map { row ->
            (row as List<*>).map { valueToDisplay(it as Map<*, *>) }
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

// ─────────────────────────────────────────────────────────────────────
// A small JSON reader, just enough for the engine's output (objects,
// arrays, strings, numbers, true/false/null). The window read is nested
// (`{"values":[[{…},…],…]}`), which display()'s per-value regex can't
// handle; rather than add a dependency, we parse it here. The engine emits
// ASCII cell text and no \uXXXX escapes, so the minimal escape handling is
// sufficient for this trusted input.
// ─────────────────────────────────────────────────────────────────────

/// Map one decoded value object (`{"kind":...}`) to the display string.
private fun valueToDisplay(obj: Map<*, *>): String = when (obj["kind"]) {
    "empty" -> ""
    "number" -> {
        val d = (obj["value"] as Number).toDouble()
        if (d == Math.floor(d) && Math.abs(d) < 1e15) d.toLong().toString() else d.toString()
    }
    "text" -> obj["value"] as? String ?: ""
    "boolean" -> if (obj["value"] == true) "TRUE" else "FALSE"
    "error" -> obj["code"] as? String ?: "#ERR"
    else -> ""
}

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
