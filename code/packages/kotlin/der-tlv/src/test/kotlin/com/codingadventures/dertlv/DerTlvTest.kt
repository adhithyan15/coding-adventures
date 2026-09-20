package com.codingadventures.dertlv

import com.fasterxml.jackson.databind.JsonNode
import com.fasterxml.jackson.databind.ObjectMapper
import java.nio.file.Files
import java.nio.file.Path
import org.junit.jupiter.api.DynamicTest
import org.junit.jupiter.api.TestFactory
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class DerTlvTest {
    private val mapper = ObjectMapper()
    private val fixture = mapper.readTree(
        Files.readString(Path.of("..", "..", "..", "specs", "fixtures", "der-tlv-v1", "cases.json")),
    )

    @TestFactory
    fun portableConformance(): List<DynamicTest> = fixture.path("cases").map { testCase ->
        DynamicTest.dynamicTest(testCase.path("id").asText()) {
            val input = materialize(testCase.path("input"))
            val actual = if (testCase.path("operation").asText() == "cursor") {
                runCursor(testCase, input)
            } else {
                runDecode(testCase, input)
            }
            assertEquals(
                testCase.path("expected"),
                mapper.readTree(mapper.writeValueAsString(actual)),
            )
            if (testCase.has("redacted_input_hex")) {
                assertEquals(-1, mapper.writeValueAsString(actual).indexOf(testCase.path("redacted_input_hex").asText()))
            }
        }
    }

    @Test
    fun viewsShareCallerArrayAndDefaultsArePublic() {
        val input = byteArrayOf(4, 1, 42)
        val element = DerTlv.decodeExact(input)
        input[2] = 127
        val value = ByteArray(element.value.remaining()).also { element.value.get(it) }
        assertContentEquals(byteArrayOf(127), value)
        assertEquals(4_096, DerTlv.Limits().maxElements)
        assertEquals(2, DerTlv.Cursor(byteArrayOf(5, 0)).remaining.remaining())
    }

    @Test
    fun rejectsInvalidLimits() {
        assertFailsWith<IllegalArgumentException> { DerTlv.Limits(maxElements = -1) }
        assertFailsWith<IllegalArgumentException> { DerTlv.Limits(maxTagNumber = 0x1_0000_0000L) }
    }

    private fun runDecode(testCase: JsonNode, input: ByteArray): Any = try {
        if (testCase.path("operation").asText() == "decode-one") {
            val decoded = DerTlv.decodeOne(input, limits(testCase))
            elementProjection(decoded.element, 0).also {
                assertEquals(it["remainder_offset"], input.size - decoded.remainder.remaining())
            }
        } else {
            elementProjection(DerTlv.decodeExact(input, limits(testCase)), 0)
        }
    } catch (failure: DerTlv.DerError) {
        errorProjection(failure)
    }

    private fun runCursor(testCase: JsonNode, input: ByteArray): Any {
        val cursor = DerTlv.Cursor(input, limits(testCase))
        val events = testCase.path("actions").map { action ->
            if (action.asText() == "finish") {
                try {
                    cursor.finish()
                    mapOf("outcome" to "finished")
                } catch (failure: DerTlv.DerError) {
                    errorProjection(failure)
                }
            } else {
                val offset = input.size - cursor.remaining.remaining()
                try {
                    cursor.read()?.let { elementProjection(it, offset) } ?: mapOf("outcome" to "end")
                } catch (failure: DerTlv.DerError) {
                    errorProjection(failure)
                }
            }
        }
        return mapOf(
            "events" to events,
            "elements_read" to cursor.elementsRead,
            "remaining_offset" to input.size - cursor.remaining.remaining(),
        )
    }

    private fun elementProjection(element: DerTlv.Element, offset: Int): Map<String, Any> = mapOf(
        "outcome" to "element",
        "element_offset" to offset,
        "tag" to mapOf(
            "class" to element.tag.tagClass,
            "constructed" to element.tag.constructed,
            "number" to element.tag.number,
        ),
        "header_len" to element.header.remaining(),
        "encoded_len" to element.encoded.remaining(),
        "remainder_offset" to offset + element.encoded.remaining(),
    )

    private fun errorProjection(error: DerTlv.DerError): Map<String, Any> = mapOf(
        "outcome" to "error",
        "error_id" to error.kind,
        "offset" to error.offset,
    )

    private fun limits(testCase: JsonNode): DerTlv.Limits {
        val defaults = fixture.path("defaults")
        val overrides = testCase.path("limits")
        fun value(name: String): Long {
            val node = if (overrides.has(name)) overrides.path(name) else defaults.path(name)
            return if (node.isTextual) Long.MAX_VALUE else node.asLong()
        }
        return DerTlv.Limits(
            value("max_input_len"),
            value("max_value_len"),
            value("max_elements"),
            value("max_tag_number"),
        )
    }

    private fun materialize(segments: JsonNode): ByteArray = buildList {
        segments.forEach { segment ->
            val value = hex(if (segment.has("hex")) segment.path("hex").asText() else segment.path("repeat_hex").asText())
            repeat(if (segment.has("hex")) 1 else segment.path("count").asInt()) { addAll(value.toList()) }
        }
    }.toByteArray()

    private fun hex(value: String): ByteArray = ByteArray(value.length / 2) { index ->
        value.substring(index * 2, index * 2 + 2).toInt(16).toByte()
    }
}
