package com.example.visicalc

import org.json.JSONObject

/**
 * Engine — the Android host's binding to the shared Rust spreadsheet engine.
 *
 * The Compose *Desktop* sibling reaches the engine through the JVM Foreign
 * Function & Memory API (JDK 21+). Android's ART runtime has no FFM, so here the
 * engine is reached the classic way: a cross-compiled native library
 * (`libspreadsheet_android_jni.so`, built from the `spreadsheet-android-jni` Rust
 * crate on top of `spreadsheet-core`) loaded via `System.loadLibrary`, with
 * `native` methods whose JNI symbols are `Java_com_example_visicalc_Engine_*`.
 *
 * The session lives on the native heap; this object holds it as an opaque `Long`
 * handle and frees it in [close]. All calls happen on the UI thread.
 */
class Engine : AutoCloseable {
    private val ptr: Long = nativeNewSession()

    init {
        // The classic cross-footing budget — the identical seed every other
        // VisiCalc demo uses (E column = row sums, row 5 = column sums, E5 = grand
        // total 169), so the Android grid shows the SAME engine-COMPUTED values,
        // not the hard-coded placeholders it used before.
        val cells = arrayOf(
            "A1" to "15", "B1" to "3", "C1" to "12", "D1" to "8", "E1" to "=SUM(A1:D1)",
            "A2" to "8", "B2" to "14", "C2" to "7", "D2" to "22", "E2" to "=SUM(A2:D2)",
            "A3" to "12", "B3" to "9", "C3" to "18", "D3" to "6", "E3" to "=SUM(A3:D3)",
            "A4" to "4", "B4" to "11", "C4" to "3", "D4" to "17", "E4" to "=SUM(A4:D4)",
            "A5" to "=SUM(A1:A4)", "B5" to "=SUM(B1:B4)", "C5" to "=SUM(C1:C4)",
            "D5" to "=SUM(D1:D4)", "E5" to "=SUM(E1:E4)",
        )
        for ((a1, raw) in cells) nativeSetCell(ptr, a1, raw)
    }

    /** Write a cell's source and recompute; returns the engine's JSON status. */
    fun setCell(a1: String, raw: String): String = nativeSetCell(ptr, a1, raw)

    /** A cell's typed source string (for the formula bar). */
    fun rawAt(a1: String): String = nativeGetRaw(ptr, a1)

    /** The A1-style letters for a 1-based column index (1 → "A"). */
    fun columnLetters(index: Int): String = nativeColumnLetters(ptr, index)

    /**
     * The demo's 5×5 grid as the host expects it: one row per sheet row 1..5, each
     * `[rowLabel, A, B, C, D, E]` where A..E are the engine's format-rendered
     * display strings. Parses the engine's `{"cells":[[…],…]}` window JSON.
     */
    fun viewportRows(rows: Int = 5, cols: Int = 5): List<List<String>> {
        val json = nativeGetDisplayWindow(ptr, 1, 1, rows, cols)
        val cells = JSONObject(json).optJSONArray("cells") ?: return emptyList()
        return (0 until cells.length()).map { r ->
            val row = cells.getJSONArray(r)
            val values = (0 until row.length()).map { c -> row.getString(c) }
            listOf((r + 1).toString()) + values
        }
    }

    // ── File open / save — bytes in, bytes out ──────────────────────────────
    // Open and save a REAL spreadsheet file over the one engine. A Java `byte[]`
    // carries the raw file bytes intact — an .xlsx is a ZIP, an .xls an OLE2 file,
    // either may hold a 0x00 — so nothing goes through the String/char* path a NUL
    // would truncate. The JNI side (spreadsheet-android-jni) marshals the byte[]
    // via jni-bridge's jni_get_byte_array / jni_new_byte_array_from. `.xlsx` keeps
    // live formulas; `.xls`/CSV/TSV/JSON are values-only per spreadsheet-io.

    /** Open a file's [bytes] as [format] (one of [fileFormats]), replacing the
     *  workbook. Returns `true` if opened, `false` if the bytes aren't a readable
     *  file of that format (the document is untouched) or the input is empty. The
     *  caller re-reads [viewportRows] / [rawAt] afterwards. */
    fun importBytes(format: String, bytes: ByteArray): Boolean {
        if (bytes.isEmpty()) return false
        return when (format) {
            "xlsx" -> nativeLoadXlsx(ptr, bytes)
            "xls" -> nativeLoadXls(ptr, bytes)
            "csv" -> nativeLoadCsv(ptr, bytes)
            "tsv" -> nativeLoadTsv(ptr, bytes)
            "json" -> nativeLoadJson(ptr, bytes)
            else -> false
        }
    }

    /** Serialize the current document to [format]'s file bytes (one of
     *  [fileFormats]) — the bytes a host writes to the file the user picked. An
     *  unknown format yields an empty array. */
    fun exportBytes(format: String): ByteArray = when (format) {
        "xlsx" -> nativeSaveXlsx(ptr)
        "xls" -> nativeSaveXls(ptr)
        "csv" -> nativeSaveCsv(ptr)
        "tsv" -> nativeSaveTsv(ptr)
        "json" -> nativeSaveJson(ptr)
        else -> ByteArray(0)
    }

    override fun close() {
        nativeFree(ptr)
    }

    // ── JNI bindings ────────────────────────────────────────────────────────
    // Instance `external` methods so the JNI symbols are exactly
    // `Java_com_example_visicalc_Engine_native*` (the implicit `this`/jclass second
    // argument is ignored on the Rust side). See spreadsheet-android-jni.
    private external fun nativeNewSession(): Long
    private external fun nativeFree(ptr: Long)
    private external fun nativeSetCell(ptr: Long, a1: String, raw: String): String
    private external fun nativeGetDisplay(ptr: Long, a1: String): String
    private external fun nativeGetRaw(ptr: Long, a1: String): String
    private external fun nativeGetDisplayWindow(ptr: Long, row0: Int, col0: Int, row1: Int, col1: Int): String
    private external fun nativeColumnLetters(ptr: Long, index: Int): String
    // File open / save (bytes in, bytes out) — added in spreadsheet-android-jni
    // 0.2.0; the byte[] crosses via jni-bridge's byte-array helpers.
    private external fun nativeLoadXlsx(ptr: Long, data: ByteArray): Boolean
    private external fun nativeSaveXlsx(ptr: Long): ByteArray
    private external fun nativeLoadXls(ptr: Long, data: ByteArray): Boolean
    private external fun nativeSaveXls(ptr: Long): ByteArray
    private external fun nativeLoadCsv(ptr: Long, data: ByteArray): Boolean
    private external fun nativeSaveCsv(ptr: Long): ByteArray
    private external fun nativeLoadTsv(ptr: Long, data: ByteArray): Boolean
    private external fun nativeSaveTsv(ptr: Long): ByteArray
    private external fun nativeLoadJson(ptr: Long, data: ByteArray): Boolean
    private external fun nativeSaveJson(ptr: Long): ByteArray

    companion object {
        /** The spreadsheet file formats the demo can open / save, in menu order. */
        val fileFormats = listOf("xlsx", "xls", "csv", "tsv", "json")

        init {
            System.loadLibrary("spreadsheet_android_jni")
        }
    }
}
