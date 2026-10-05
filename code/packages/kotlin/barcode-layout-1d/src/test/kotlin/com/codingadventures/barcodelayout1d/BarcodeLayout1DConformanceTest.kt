package com.codingadventures.barcodelayout1d

import com.codingadventures.paintinstructions.PaintInstruction
import com.codingadventures.paintinstructions.PaintScene
import com.fasterxml.jackson.core.JsonParser
import com.fasterxml.jackson.databind.JsonNode
import com.fasterxml.jackson.databind.ObjectMapper
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DynamicTest
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.TestFactory
import java.nio.file.Files
import java.nio.file.Path
import java.security.MessageDigest
import java.util.HexFormat
import java.util.TreeMap

/** Runs the shared 56-case corpus through the independent Kotlin v1 core. */
class BarcodeLayout1DConformanceTest {
    private val maxBytes = 131_072
    private val corpusSha = "be95aa0381041ef3bd729b36bb4292f7a20692e139b53adca97af4e157cb7388"
    private val corpusPath = Path.of("..", "..", "..", "..", "code", "specs", "fixtures", "barcode-layout-1d-v1", "cases.json")
    private val json = ObjectMapper().apply { factory.enable(JsonParser.Feature.STRICT_DUPLICATE_DETECTION) }

    private fun sha256(bytes: ByteArray): String =
        HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(bytes))

    private fun load(bytes: ByteArray): JsonNode {
        require(bytes.size <= maxBytes) { "fixture-size-limit" }
        var depth = 0
        var quoted = false
        var escaped = false
        for (byte in bytes) {
            val ch = (byte.toInt() and 0xff).toChar()
            if (quoted) {
                if (escaped) escaped = false
                else if (ch == '\\') escaped = true
                else if (ch == '"') quoted = false
            } else if (ch == '"') quoted = true
            else if (ch == '[' || ch == '{') {
                depth++
                require(depth <= 8) { "fixture-depth-limit" }
            } else if (ch == ']' || ch == '}') depth--
        }
        val document = json.readTree(bytes)
        require(document != null && document.isObject && document.path("cases").isArray) { "fixture-schema-invalid" }
        validateScalars(document)
        return document
    }

    private fun validateScalars(node: JsonNode) {
        if (node.isTextual) {
            val value = node.textValue()
            var index = 0
            while (index < value.length) {
                val ch = value[index]
                if (ch.isHighSurrogate()) {
                    index++
                    require(index < value.length && value[index].isLowSurrogate()) { "fixture-invalid-scalar" }
                } else require(!ch.isLowSurrogate()) { "fixture-invalid-scalar" }
                index++
            }
        } else if (node.isContainerNode) {
            node.forEach(::validateScalars)
            if (node.isObject) node.fieldNames().forEachRemaining { validateScalars(json.valueToTree(it)) }
        }
    }

    private fun corpus(): JsonNode {
        val bytes = Files.newInputStream(corpusPath).use { it.readNBytes(maxBytes + 1) }
        require(bytes.size <= maxBytes) { "fixture-size-limit" }
        assertEquals(corpusSha, sha256(bytes))
        val document = load(bytes)
        assertEquals(56, document.path("cases").size())
        val ids = mutableSetOf<String>()
        document.path("cases").forEach { assertTrue(ids.add(it.path("id").asText())) }
        return document
    }

    private fun pattern(input: JsonNode): String {
        if (input.has("pattern")) return input.path("pattern").asText()
        val repeated = input.path("repeat")
        val count = repeated.path("count").asInt(-1)
        val token = repeated.path("token").asText()
        val suffix = repeated.path("suffix").asText("")
        require(count in 0..65_569 && token.codePointCount(0, token.length) in 1..2 &&
            suffix.codePointCount(0, suffix.length) <= 1) { "fixture-schema-invalid" }
        return token.repeat(count) + suffix
    }

    private fun runs(input: JsonNode): List<BarcodeLayout1DV1.Run> {
        if (input.has("runs")) return input.path("runs").map { row ->
            BarcodeLayout1DV1.Run(row.path("color").asText(), row.path("modules").asInt(),
                row.path("sourceLabel").asText(), row.path("sourceIndex").asLong(), row.path("role").asText())
        }
        val repeated = input.path("repeatRuns")
        val count = repeated.path("count").asInt(-1)
        require(count in 0..40_980) { "fixture-schema-invalid" }
        val first = repeated.path("firstColor").asText()
        return List(count) { index ->
            BarcodeLayout1DV1.Run(
                if (index % 2 == 0) first else if (first == "bar") "space" else "bar",
                repeated.path("modules").asInt(), repeated.path("sourceLabel").asText(),
                repeated.path("sourceIndex").asLong(), repeated.path("role").asText(),
            )
        }
    }

    private fun symbols(input: JsonNode): List<BarcodeLayout1DV1.Symbol>? {
        if (input.has("symbols")) return input.path("symbols").map { row ->
            BarcodeLayout1DV1.Symbol(row.path("label").asText(), row.path("modules").asInt(),
                row.path("sourceIndex").asLong(), row.path("role").asText())
        }
        if (!input.has("repeatSymbols")) return null
        val repeated = input.path("repeatSymbols")
        val count = repeated.path("count").asInt(-1)
        require(count in 0..40_980) { "fixture-schema-invalid" }
        return List(count) { index ->
            BarcodeLayout1DV1.Symbol(repeated.path("label").asText(), repeated.path("modules").asInt(),
                index.toLong(), repeated.path("role").asText())
        }
    }

    private fun integer(row: JsonNode, key: String, fallback: Int): Int =
        if (row.has(key)) row.path(key).asInt() else fallback

    private fun string(row: JsonNode, key: String, fallback: String): String =
        if (row.has(key)) row.path(key).asText() else fallback

    private fun execute(row: JsonNode): Any {
        val input = row.path("input")
        return when (row.path("operation").asText()) {
            "expand-binary" -> BarcodeLayout1DV1.expandBinary(pattern(input), input.path("sourceLabel").asText(),
                input.path("sourceIndex").asLong(), input.path("role").asText())
            "expand-width" -> BarcodeLayout1DV1.expandWidth(pattern(input),
                string(input, "narrowMarker", "N"), string(input, "wideMarker", "W"),
                integer(input, "narrowModules", 1), integer(input, "wideModules", 3),
                string(input, "startingColor", "bar"), input.path("sourceLabel").asText(),
                input.path("sourceIndex").asLong(), input.path("role").asText())
            "compute-layout" -> BarcodeLayout1DV1.computeLayout(runs(input),
                input.path("quietZoneModules").asInt(), symbols(input))
            "project-scene" -> {
                val render = input.path("renderConfig")
                val metadata = mutableMapOf<String, String>()
                input.path("metadata").fields().forEachRemaining { metadata[it.key] = it.value.asText() }
                BarcodeLayout1DV1.projectScene(runs(input), input.path("quietZoneModules").asInt(),
                    BarcodeLayout1DV1.SceneOptions(
                        moduleWidth = integer(render, "moduleWidth", 4),
                        barHeight = integer(render, "barHeight", 120),
                        foreground = string(render, "foreground", "#000000"),
                        background = string(render, "background", "#ffffff"),
                        label = string(input, "label", "1D barcode"),
                        metadata = metadata,
                        humanReadableText = if (input.has("humanReadableText")) input.path("humanReadableText").asText() else null,
                        includeHumanReadableText = render.path("includeHumanReadableText").asBoolean(false),
                        symbols = symbols(input),
                    ))
            }
            else -> error("unknown neutral operation")
        }
    }

    private fun runMap(run: BarcodeLayout1DV1.Run): Map<String, Any> = sortedMapOf(
        "color" to run.color, "modules" to run.modules, "role" to run.role,
        "sourceIndex" to run.sourceIndex.toInt(), "sourceLabel" to run.sourceLabel,
    )

    private fun project(actual: Any): Any = when (actual) {
        is List<*> -> actual.map { runMap(it as BarcodeLayout1DV1.Run) }
        is BarcodeLayout1DV1.Layout -> mapOf(
            "leftQuietZoneModules" to actual.leftQuietZoneModules,
            "rightQuietZoneModules" to actual.rightQuietZoneModules,
            "contentModules" to actual.contentModules,
            "totalModules" to actual.totalModules,
            "symbolLayouts" to actual.symbolLayouts.map { symbol -> mapOf(
                "label" to symbol.label, "startModule" to symbol.startModule,
                "endModule" to symbol.endModule, "sourceIndex" to symbol.sourceIndex,
                "role" to symbol.role,
            ) },
        )
        is PaintScene -> mapOf(
            "width" to actual.width, "height" to actual.height,
            "background" to actual.background,
            "rectangles" to actual.instructions.map { instruction ->
                val rect = instruction as PaintInstruction.PaintRect
                mapOf("x" to rect.x, "y" to rect.y, "width" to rect.width,
                    "height" to rect.height, "fill" to rect.fill, "metadata" to rect.metadata)
            },
            "metadata" to actual.metadata,
        )
        else -> error("unexpected neutral result")
    }

    @TestFactory
    fun allNeutralCases(): List<DynamicTest> = corpus().path("cases").map { row ->
        DynamicTest.dynamicTest(row.path("id").asText()) {
            val expected = row.path("expected")
            if (expected.has("error")) {
                val error = assertThrows(BarcodeLayout1DV1.V1Error::class.java) { execute(row) }
                assertEquals(expected.path("error").asText(), error.errorId)
            } else {
                val actual = project(execute(row))
                if (expected.has("runDigest")) {
                    val digest = expected.path("runDigest")
                    val projected = actual as List<*>
                    assertEquals(digest.path("runCount").asInt(), projected.size)
                    assertEquals(digest.path("contentModules").asInt(), projected.sumOf {
                        ((it as Map<*, *>)["modules"] as Number).toInt()
                    })
                    assertEquals(digest.path("firstRun"), json.valueToTree<JsonNode>(projected.first()))
                    assertEquals(digest.path("lastRun"), json.valueToTree<JsonNode>(projected.last()))
                    assertEquals(digest.path("runsSha256").asText(), sha256(json.writeValueAsBytes(projected)))
                } else {
                    val key = when {
                        expected.has("runs") -> "runs"
                        expected.has("layout") -> "layout"
                        else -> "scene"
                    }
                    assertEquals(expected.path(key), json.valueToTree<JsonNode>(actual))
                }
            }
        }
    }

    @Test
    fun hostileFixturesFailBeforeDispatch() {
        assertThrows(IllegalArgumentException::class.java) { load(ByteArray(maxBytes + 1)) }
        assertThrows(IllegalArgumentException::class.java) { load("[[[[[[[[[0]]]]]]]]]".toByteArray()) }
        assertThrows(Exception::class.java) { load("{\"cases\":[],\"cases\":[]}".toByteArray()) }
        assertThrows(IllegalArgumentException::class.java) {
            load("{\"cases\":[],\"bad\":\"\\uD800\"}".toByteArray())
        }
        assertThrows(Exception::class.java) { load(byteArrayOf(0xff.toByte())) }
        assertThrows(IllegalArgumentException::class.java) {
            pattern(json.valueToTree(mapOf("repeat" to mapOf("token" to "1", "count" to 65_570))))
        }
    }
}
