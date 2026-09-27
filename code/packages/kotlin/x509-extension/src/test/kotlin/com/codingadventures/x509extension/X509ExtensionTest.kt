package com.codingadventures.x509extension

import com.codingadventures.derasn1.DerAsn1
import com.codingadventures.dertlv.DerTlv
import com.fasterxml.jackson.databind.JsonNode
import com.fasterxml.jackson.databind.ObjectMapper
import java.io.ByteArrayOutputStream
import java.lang.reflect.Modifier
import java.nio.ByteBuffer
import java.nio.ReadOnlyBufferException
import java.nio.file.Files
import java.nio.file.Path
import java.util.stream.Stream
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import org.junit.jupiter.api.DynamicTest
import org.junit.jupiter.api.TestFactory

class X509ExtensionTest {
    private val mapper = ObjectMapper()
    private val contract = readFixture("x509-extension-v1")
    private val upstream = readFixture("der-asn1-v1")

    @TestFactory
    fun portableConformance(): Stream<DynamicTest> {
        assertEquals(48, contract.path("cases").size())
        assertEquals(8, contract.path("error_ids").size())
        return contract.path("cases").map { testCase ->
            DynamicTest.dynamicTest(testCase.path("id").asText()) {
                val decoder = DerAsn1.Asn1Decoder(limits(testCase))
                val root = decoder.decodeExact(materialize(testCase.path("input")))
                val actual: Any = if (testCase.path("operation").asText() == "extension-script") {
                    linkedMapOf(
                        "outcome" to "script",
                        "events" to testCase.path("actions").map { attempt(decoder, root) },
                    )
                } else {
                    attempt(decoder, root)
                }
                val serialized = mapper.writeValueAsString(actual)
                assertEquals(testCase.path("expected"), mapper.readTree(serialized))
                if (testCase.has("redacted_input_hex")) {
                    assertFalse(serialized.contains(testCase.path("redacted_input_hex").asText()))
                }
            }
        }.stream()
    }

    @Test
    fun publishedContractCannotForgeOrMutateValidatedValues() {
        assertEquals(8, X509Extension.ErrorKind.entries.size)
        assertEquals(
            8,
            X509Extension.ErrorKind.entries.map(X509Extension.ErrorKind::id).toSet().size,
        )
        assertTrue(
            X509Extension.Value::class.java.constructors.none {
                Modifier.isPublic(it.modifiers) && !it.isSynthetic
            },
        )
        val input = fromHex("30090603551d1104023000")
        val decoder = DerAsn1.Asn1Decoder()
        val value = X509Extension.decodeX509Extension(decoder, decoder.decodeExact(input))
        input[9] = 0xff.toByte()
        assertEquals("3000", toHex(bytes(value.extensionValue)))
        assertFailsWith<ReadOnlyBufferException> { value.extensionValue.put(0, 0) }
        assertEquals("3000", toHex(bytes(value.extensionValue)))

        val hostileDecoder = DerAsn1.Asn1Decoder()
        val hostileRoot = hostileDecoder.decodeExact(fromHex("30080601800403deadbe"))
        val error = assertFailsWith<X509Extension.Error> {
            X509Extension.decodeX509Extension(hostileDecoder, hostileRoot)
        }
        assertEquals(X509Extension.ErrorKind.INVALID_EXTENSION_ID, error.kind)
        assertEquals(DerAsn1.Asn1ErrorKind.NON_MINIMAL_OBJECT_IDENTIFIER, error.asn1Kind)
        assertEquals(4, error.offset)
        assertEquals(2, hostileDecoder.elementsRead)
        assertFalse(error.toString().contains("deadbe"))
    }

    private fun attempt(
        decoder: DerAsn1.Asn1Decoder,
        root: DerAsn1.Asn1Element,
    ): LinkedHashMap<String, Any?> = try {
        val value = X509Extension.decodeX509Extension(decoder, root)
        linkedMapOf(
            "outcome" to "value",
            "extension_id_arcs_decimal" to value.extensionId.arcs.map(ULong::toString),
            "critical" to value.critical,
            "extension_value_hex" to toHex(bytes(value.extensionValue)),
            "elements_read" to decoder.elementsRead,
        )
    } catch (error: X509Extension.Error) {
        linkedMapOf<String, Any?>(
            "outcome" to "error",
            "error_id" to error.kind.id,
            "offset" to error.offset,
            "offset_scope" to "extension-element",
            "elements_read" to decoder.elementsRead,
        ).also { result ->
            if (error.asn1Kind != null) result["asn1_error_id"] = error.asn1Kind.id
            if (error.framingKind != null) result["framing_error_id"] = error.framingKind
        }
    }

    private fun limits(testCase: JsonNode): DerAsn1.Asn1Limits {
        val defaults = upstream.path("defaults")
        val overrides = testCase.path("limits")
        return DerAsn1.Asn1Limits(
            der = DerTlv.Limits(
                maxInputLength = longLimit(defaults.path("der"), overrides.path("der"), "max_input_len"),
                maxValueLength = longLimit(defaults.path("der"), overrides.path("der"), "max_value_len"),
                maxElements = longLimit(defaults.path("der"), overrides.path("der"), "max_elements"),
                maxTagNumber = longLimit(defaults.path("der"), overrides.path("der"), "max_tag_number"),
            ),
            maxDepth = intLimit(defaults, overrides, "max_depth"),
            maxTotalElements = longLimit(defaults, overrides, "max_total_elements"),
            maxOidArcs = intLimit(defaults, overrides, "max_oid_arcs"),
        )
    }

    private fun intLimit(defaults: JsonNode, overrides: JsonNode, name: String): Int =
        (if (overrides.has(name)) overrides.path(name) else defaults.path(name)).asInt()

    private fun longLimit(defaults: JsonNode, overrides: JsonNode, name: String): Long {
        val value = if (overrides.has(name)) overrides.path(name) else defaults.path(name)
        return if (value.isTextual) Long.MAX_VALUE else value.asLong()
    }

    private fun materialize(segments: JsonNode): ByteArray {
        val output = ByteArrayOutputStream()
        for (segment in segments) {
            val value = fromHex(
                if (segment.has("hex")) segment.path("hex").asText()
                else segment.path("repeat_hex").asText(),
            )
            repeat(if (segment.has("hex")) 1 else segment.path("count").asInt()) {
                output.writeBytes(value)
            }
        }
        return output.toByteArray()
    }

    private fun fromHex(value: String): ByteArray = ByteArray(value.length / 2) { index ->
        value.substring(index * 2, index * 2 + 2).toInt(16).toByte()
    }

    private fun toHex(value: ByteArray): String =
        value.joinToString("") { "%02x".format(it.toInt() and 0xff) }

    private fun bytes(buffer: ByteBuffer): ByteArray {
        val copy = buffer.duplicate()
        return ByteArray(copy.remaining()).also(copy::get)
    }

    private fun readFixture(name: String): JsonNode {
        val path = Path.of("..", "..", "..", "specs", "fixtures", name, "cases.json")
        return mapper.readTree(Files.readString(path))
    }
}
