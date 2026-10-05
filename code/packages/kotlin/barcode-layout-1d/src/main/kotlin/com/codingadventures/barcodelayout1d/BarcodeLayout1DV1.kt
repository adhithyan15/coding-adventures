package com.codingadventures.barcodelayout1d

import com.codingadventures.paintinstructions.PaintInstruction
import com.codingadventures.paintinstructions.PaintScene
import java.nio.charset.StandardCharsets

/** Pure bounded module geometry and paint-rectangle projection. */
object BarcodeLayout1DV1 {
    private const val maxPattern = 65_567
    private const val maxRuns = 40_979
    private const val maxContent = 65_567
    private const val maxQuiet = 4_096
    private val roles = setOf("data", "start", "stop", "guard", "check", "inter-character-gap")

    data class Run(val color: String, val modules: Int, val sourceLabel: String, val sourceIndex: Long, val role: String)
    data class Symbol(val label: String, val modules: Int, val sourceIndex: Long, val role: String)
    data class SymbolLayout(val label: String, val startModule: Int, val endModule: Int,
                            val sourceIndex: Int, val role: String)
    data class Layout(val leftQuietZoneModules: Int, val rightQuietZoneModules: Int,
                      val contentModules: Int, val totalModules: Int, val symbolLayouts: List<SymbolLayout>)
    data class SceneOptions(
        val moduleWidth: Int = 4, val barHeight: Int = 120,
        val foreground: String = "#000000", val background: String = "#ffffff",
        val label: String = "1D barcode", val metadata: Map<String, String> = emptyMap(),
        val humanReadableText: String? = null, val includeHumanReadableText: Boolean = false,
        val symbols: List<Symbol>? = null,
    )
    class V1Error(val errorId: String) : IllegalArgumentException(errorId)
    private fun fail(id: String): Nothing = throw V1Error(id)

    private fun scalars(value: String, error: String): Int {
        var index = 0
        while (index < value.length) {
            val ch = value[index]
            if (ch.isHighSurrogate()) {
                index++
                if (index >= value.length || !value[index].isLowSurrogate()) fail(error)
            } else if (ch.isLowSurrogate()) fail(error)
            index++
        }
        return value.codePointCount(0, value.length)
    }

    private fun source(label: String, index: Long, role: String) {
        if (scalars(label, "invalid-source-attribution") > 4_096 ||
            index !in Int.MIN_VALUE.toLong()..Int.MAX_VALUE.toLong() || role !in roles) {
            fail("invalid-source-attribution")
        }
    }

    fun expandBinary(pattern: String, sourceLabel: String, sourceIndex: Long, role: String): List<Run> {
        val length = scalars(pattern, "invalid-binary-token")
        if (length > maxPattern) fail("pattern-too-long")
        if (length == 0) fail("empty-pattern")
        if (pattern.any { it != '0' && it != '1' }) fail("invalid-binary-token")
        source(sourceLabel, sourceIndex, role)
        val result = mutableListOf<Run>()
        var current = pattern[0]
        var count = 1
        for (token in pattern.drop(1)) {
            if (token == current) { count++; continue }
            if (result.size >= maxRuns) fail("too-many-runs")
            result += Run(if (current == '1') "bar" else "space", count, sourceLabel, sourceIndex, role)
            current = token
            count = 1
        }
        if (result.size >= maxRuns) fail("too-many-runs")
        result += Run(if (current == '1') "bar" else "space", count, sourceLabel, sourceIndex, role)
        return result.toList()
    }

    fun expandWidth(pattern: String, narrowMarker: String, wideMarker: String,
                    narrowModules: Int, wideModules: Int, startingColor: String,
                    sourceLabel: String, sourceIndex: Long, role: String): List<Run> {
        val length = scalars(pattern, "invalid-width-token")
        if (length > maxPattern) fail("pattern-too-long")
        if (length == 0) fail("empty-pattern")
        if (scalars(narrowMarker, "invalid-marker-configuration") != 1 ||
            scalars(wideMarker, "invalid-marker-configuration") != 1 ||
            narrowMarker == wideMarker) fail("invalid-marker-configuration")
        val narrow = narrowMarker.codePointAt(0)
        val wide = wideMarker.codePointAt(0)
        val tokens = pattern.codePoints().toArray()
        if (tokens.any { it != narrow && it != wide }) fail("invalid-width-token")
        source(sourceLabel, sourceIndex, role)
        if (narrowModules <= 0 || wideModules <= 0) fail("invalid-module-count")
        if (startingColor != "bar" && startingColor != "space") fail("invalid-marker-configuration")
        if (length > maxRuns) fail("too-many-runs")
        val result = ArrayList<Run>(length)
        var content = 0
        tokens.forEachIndexed { index, token ->
            val modules = if (token == wide) wideModules else narrowModules
            if (modules > maxContent - content) fail("content-too-wide")
            content += modules
            val color = if (index % 2 == 0) startingColor else if (startingColor == "bar") "space" else "bar"
            result += Run(color, modules, sourceLabel, sourceIndex, role)
        }
        return result.toList()
    }

    fun computeLayout(runs: List<Run>, quietZoneModules: Int, symbols: List<Symbol>? = null): Layout {
        if (runs.size > maxRuns) fail("too-many-runs")
        var content = 0
        var previous: String? = null
        for (run in runs) {
            if (run.color !in setOf("bar", "space") || run.role !in roles) fail("invalid-source-attribution")
            if (run.modules <= 0) fail("invalid-module-count")
            source(run.sourceLabel, run.sourceIndex, run.role)
            if (run.modules > maxContent - content) fail("content-too-wide")
            content += run.modules
            if (run.color == previous) fail("non-alternating-runs")
            previous = run.color
        }
        if (quietZoneModules !in 1..maxQuiet) fail("invalid-quiet-zone")
        val layouts = mutableListOf<SymbolLayout>()
        if (symbols != null) {
            if (symbols.size > maxRuns) fail("too-many-symbols")
            var cursor = 0
            for (symbol in symbols) {
                if (symbol.modules <= 0) fail("invalid-module-count")
                source(symbol.label, symbol.sourceIndex, symbol.role)
                if (symbol.role == "inter-character-gap") fail("invalid-source-attribution")
                if (symbol.modules > maxContent - cursor) fail("symbol-width-mismatch")
                val end = cursor + symbol.modules
                layouts += SymbolLayout(symbol.label, cursor, end, symbol.sourceIndex.toInt(), symbol.role)
                cursor = end
            }
            if (cursor != content) fail("symbol-width-mismatch")
        } else {
            var cursor = 0
            var active: Run? = null
            var start = 0
            for (run in runs) {
                if (run.role != "inter-character-gap") {
                    val changed = active == null || active.sourceLabel != run.sourceLabel ||
                        active.sourceIndex != run.sourceIndex || active.role != run.role
                    if (changed) {
                        if (active != null) layouts += SymbolLayout(active.sourceLabel, start, cursor,
                            active.sourceIndex.toInt(), active.role)
                        active = run
                        start = cursor
                    }
                }
                cursor += run.modules
            }
            if (active != null) layouts += SymbolLayout(active.sourceLabel, start, cursor,
                active.sourceIndex.toInt(), active.role)
            if (layouts.size > maxRuns) fail("too-many-symbols")
        }
        return Layout(quietZoneModules, quietZoneModules, content,
            quietZoneModules + content + quietZoneModules, layouts.toList())
    }

    fun projectScene(runs: List<Run>, quietZoneModules: Int, options: SceneOptions = SceneOptions()): PaintScene {
        if (options.includeHumanReadableText || options.humanReadableText != null) {
            fail("human-readable-text-unsupported")
        }
        if (options.moduleWidth !in 1..8_192 || options.barHeight !in 1..8_192 ||
            scalars(options.foreground, "invalid-render-config") > 128 ||
            scalars(options.background, "invalid-render-config") > 128) fail("invalid-render-config")
        val layout = computeLayout(runs, quietZoneModules, options.symbols)
        if (options.metadata.size > 64) fail("metadata-too-large")
        val metadata = mutableMapOf<String, String>()
        var bytes = 0
        for ((key, value) in options.metadata) {
            if (scalars(key, "metadata-too-large") > 128 ||
                scalars(value, "metadata-too-large") > 4_096) fail("metadata-too-large")
            bytes += key.toByteArray(StandardCharsets.UTF_8).size + value.toByteArray(StandardCharsets.UTF_8).size
            if (bytes > 65_536) fail("metadata-too-large")
            metadata[key] = value
        }
        if (scalars(options.label, "metadata-too-large") > 4_096) fail("metadata-too-large")
        val rectangles = mutableListOf<PaintInstruction>()
        var cursor = quietZoneModules
        for (run in runs) {
            val end = cursor + run.modules
            if (run.color == "bar") rectangles += PaintInstruction.PaintRect(
                cursor * options.moduleWidth, 0, run.modules * options.moduleWidth,
                options.barHeight, options.foreground,
                mapOf("sourceLabel" to run.sourceLabel, "sourceIndex" to run.sourceIndex.toString(),
                    "role" to run.role, "moduleStart" to cursor.toString(), "moduleEnd" to end.toString()),
            )
            cursor = end
        }
        val sceneWidth = layout.totalModules * options.moduleWidth
        metadata += mapOf(
            "label" to options.label,
            "leftQuietZoneModules" to layout.leftQuietZoneModules.toString(),
            "rightQuietZoneModules" to layout.rightQuietZoneModules.toString(),
            "contentModules" to layout.contentModules.toString(),
            "totalModules" to layout.totalModules.toString(),
            "moduleWidthPx" to options.moduleWidth.toString(),
            "barHeightPx" to options.barHeight.toString(),
            "sceneWidthPx" to sceneWidth.toString(),
            "sceneHeightPx" to options.barHeight.toString(),
            "symbolCount" to layout.symbolLayouts.size.toString(),
        )
        return PaintScene(sceneWidth, options.barHeight, options.background,
            rectangles.toList(), metadata.toMap())
    }
}
