package com.codingadventures.derasn1

import com.codingadventures.derasn1.DerAsn1.Asn1Decoder
import com.codingadventures.derasn1.DerAsn1.Asn1Element
import com.codingadventures.derasn1.DerAsn1.Asn1Error
import com.codingadventures.derasn1.DerAsn1.Asn1ErrorKind
import com.codingadventures.derasn1.DerAsn1.Asn1Limits
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
import kotlin.test.assertNotNull
import kotlin.test.assertTrue
import org.junit.jupiter.api.DynamicTest
import org.junit.jupiter.api.TestFactory

class DerAsn1Test {
    private val mapper = ObjectMapper()
    private val fixture = readFixture("der-asn1-v1")
    private val upstream = readFixture("der-tlv-v1")

    @TestFactory
    fun portableConformance(): Stream<DynamicTest> {
        assertEquals(122, fixture.path("cases").size())
        assertEquals(22, fixture.path("error_ids").size())
        val references = fixture.path("cases").filter { it.has("der_tlv_case_id") }
        assertEquals(46, references.size)
        assertEquals(46, references.map { it.path("der_tlv_case_id").asText() }.toSet().size)
        return fixture.path("cases").map { testCase ->
            DynamicTest.dynamicTest(testCase.path("id").asText()) {
                val actual = runCase(testCase)
                assertEquals(testCase.path("expected"), mapper.readTree(mapper.writeValueAsString(actual)))
                if (testCase.has("redacted_input_hex")) {
                    assertFalse(mapper.writeValueAsString(actual).contains(testCase.path("redacted_input_hex").asText()))
                }
            }
        }.stream()
    }

    @Test
    fun nativeContractsCoverSnapshotsLimitsUnsignedValuesAndRedaction() {
        assertEquals(Asn1Limits(), Asn1Limits())
        assertFailsWith<IllegalArgumentException> { Asn1Limits(maxDepth = -1) }
        assertFailsWith<IllegalArgumentException> { Asn1Limits(maxTotalElements = -1) }
        assertFailsWith<IllegalArgumentException> { Asn1Limits(maxOidArcs = -1) }

        val input = fromHex("060b2a81ffffffffffffffff7f")
        val element = Asn1Decoder().decodeExact(input)
        val oid = DerAsn1.decodeObjectIdentifier(element)
        assertEquals(listOf(1uL, 2uL, ULong.MAX_VALUE), oid.arcs)
        assertTrue(oid.equalsArcs(oid.arcs))
        assertFalse(oid.equalsArcs(listOf(1uL, 2uL)))
        input[input.lastIndex] = 1
        assertEquals(0x7f, element.value.get(element.value.remaining() - 1).toInt() and 0xff)
        assertFailsWith<ReadOnlyBufferException> { element.value.put(0, 0) }
        assertFailsWith<ReadOnlyBufferException> { oid.encoded.put(0, 0) }
        @Suppress("UNCHECKED_CAST")
        val javaVisibleArcs = oid.arcs as MutableList<ULong>
        assertFailsWith<UnsupportedOperationException> { javaVisibleArcs.clear() }

        val integer = DerAsn1.decodeInteger(Asn1Decoder().decodeExact(fromHex("020101")))
        assertEquals(1uL, integer.toU64())
        assertFalse(integer.isNegative)
        assertFailsWith<ReadOnlyBufferException> { integer.signedBytes.put(0, 0) }

        val bits = DerAsn1.decodeBitString(Asn1Decoder().decodeExact(fromHex("03020180")))
        assertEquals(7L, bits.bitLength)
        assertEquals(1, bits.unusedBits)
        assertFailsWith<ReadOnlyBufferException> { bits.bytes.put(0, 0) }

        val hostile = Asn1Decoder().decodeExact(fromHex("160261ff"))
        val error = assertFailsWith<Asn1Error> { DerAsn1.decodeIA5String(hostile) }
        assertFalse(error.toString().contains("61ff"))
        assertFalse(error.toString().contains("255"))
    }

    @Test
    fun publishedJvmApiCannotForgeOrMutateValidatedValues() {
        assertTrue(Asn1ErrorKind.entries.contains(Asn1ErrorKind.FRAMING))
        val validatedTypes = listOf(
            Asn1Element::class.java,
            DerAsn1.Asn1Cursor::class.java,
            DerAsn1.DerInteger::class.java,
            DerAsn1.DerBitString::class.java,
            DerAsn1.ObjectIdentifier::class.java,
        )
        for (type in validatedTypes) {
            assertTrue(type.constructors.none { Modifier.isPublic(it.modifiers) && !it.isSynthetic }, type.name)
        }
        assertTrue(
            Asn1Element::class.java.methods.none {
                Modifier.isPublic(it.modifiers) && !it.isSynthetic && it.returnType == ByteArray::class.java
            },
        )
    }

    @Test
    fun oversizedInputIsRejectedWithoutChangingCallerBuffer() {
        val input = ByteArray(65) { 0x05 }
        val limits = Asn1Limits(der = DerTlv.Limits(maxInputLength = 64))
        val error = assertFailsWith<Asn1Error> { Asn1Decoder(limits).decodeExact(input) }
        assertEquals(Asn1ErrorKind.FRAMING, error.kind)
        assertEquals("input-limit-exceeded", error.framingKind)
        assertTrue(input.all { it == 0x05.toByte() })
    }

    @Test
    fun structurallyEqualLimitsShareCursorsAndViewsAreReadOnly() {
        val first = Asn1Decoder()
        val cursor = first.sequence(first.decodeExact(fromHex("30020500")))
        val second = Asn1Decoder(Asn1Limits())
        assertNotNull(cursor.read(second))
        assertEquals(1, second.elementsRead)
        assertEquals(0, cursor.remaining.remaining())
        assertFailsWith<ReadOnlyBufferException> { cursor.remaining.put(0, 0) }
    }

    @Test
    fun comparesEveryCursorLimitField() {
        val defaults = Asn1Limits()
        val variants = listOf(
            defaults.copy(maxDepth = 31),
            defaults.copy(maxTotalElements = 16_383),
            defaults.copy(maxOidArcs = 127),
            defaults.copy(der = defaults.der.copy(maxInputLength = 1_048_575)),
            defaults.copy(der = defaults.der.copy(maxValueLength = 1_048_575)),
            defaults.copy(der = defaults.der.copy(maxElements = 4_095)),
            defaults.copy(der = defaults.der.copy(maxTagNumber = 0xffff_fffeL)),
        )
        for (variant in variants) {
            val decoder = Asn1Decoder()
            val cursor = decoder.sequence(decoder.decodeExact(fromHex("30020500")))
            val error = assertFailsWith<Asn1Error> { cursor.read(Asn1Decoder(variant)) }
            assertEquals(Asn1ErrorKind.DECODER_LIMIT_MISMATCH, error.kind)
        }
    }

    @Test
    fun malformedContainerFinishMapsFramingFailure() {
        val decoder = Asn1Decoder()
        val cursor = decoder.sequence(decoder.decodeExact(fromHex("300101")))
        val error = assertFailsWith<Asn1Error> { cursor.finish() }
        assertEquals(Asn1ErrorKind.FRAMING, error.kind)
        assertEquals("trailing-data", error.framingKind)
        assertEquals(0, error.offset)
    }

    @Test
    fun exercisesTypedHelpersAndErrorPrecedence() {
        val oid = DerAsn1.decodeObjectIdentifier(Asn1Decoder().decodeExact(fromHex("06032a0304")))
        assertTrue(oid.equalsArcs(listOf(1uL, 2uL, 3uL, 4uL)))
        assertFalse(oid.equalsArcs(listOf(1uL, 2uL, 3uL)))
        assertEquals("2a0304", toHex(bytes(oid.encoded)))
        assertEquals(4, oid.arcCount)

        val negative = DerAsn1.decodeInteger(Asn1Decoder().decodeExact(fromHex("0201ff")))
        assertTrue(negative.isNegative)
        assertEquals(Asn1ErrorKind.NEGATIVE_INTEGER, assertFailsWith<Asn1Error> { negative.toU64() }.kind)
        assertEquals(
            Asn1ErrorKind.INTEGER_OVERFLOW,
            assertFailsWith<Asn1Error> {
                DerAsn1.decodeInteger(Asn1Decoder().decodeExact(fromHex("0209010102030405060708"))).toU64()
            }.kind,
        )
        assertEquals("hi", DerAsn1.decodeIA5String(Asn1Decoder().decodeExact(fromHex("16026869"))))
        assertEquals("hi", DerAsn1.decodeImplicitIA5String(Asn1Decoder().decodeExact(fromHex("80026869")), 0))
        assertEquals("2a", toHex(bytes(DerAsn1.decodeOctetString(Asn1Decoder().decodeExact(fromHex("04012a"))))))
        assertEquals("2a", toHex(bytes(DerAsn1.decodeImplicitOctetString(Asn1Decoder().decodeExact(fromHex("80012a")), 0))))

        fun kind(hex: String, action: (Asn1Element) -> Unit): Asn1ErrorKind =
            assertFailsWith<Asn1Error> { action(Asn1Decoder().decodeExact(fromHex(hex))) }.kind
        assertEquals(Asn1ErrorKind.INVALID_BOOLEAN_VALUE, kind("010101") { DerAsn1.decodeBoolean(it) })
        assertEquals(Asn1ErrorKind.NON_MINIMAL_INTEGER, kind("02020001") { DerAsn1.decodeInteger(it) })
        assertEquals(Asn1ErrorKind.INVALID_UNUSED_BIT_COUNT, kind("030108") { DerAsn1.decodeBitString(it) })
        assertEquals(Asn1ErrorKind.NON_ZERO_BIT_PADDING, kind("03020181") { DerAsn1.decodeBitString(it) })
        assertEquals(Asn1ErrorKind.NON_EMPTY_NULL, kind("050100") { DerAsn1.decodeNull(it) })
        assertEquals(Asn1ErrorKind.EMPTY_OBJECT_IDENTIFIER, kind("0600") { DerAsn1.decodeObjectIdentifier(it) })
        assertEquals(Asn1ErrorKind.UNEXPECTED_TAG, kind("0500") { DerAsn1.decodeBoolean(it) })
        assertEquals(
            listOf(1uL, 2uL),
            DerAsn1.decodeImplicitObjectIdentifier(Asn1Decoder().decodeExact(fromHex("80012a")), 0).arcs,
        )
    }

    private fun runCase(testCase: JsonNode): Any {
        if (testCase.has("der_tlv_case_id")) return verifyUpstream(testCase.path("der_tlv_case_id").asText())
        val configured = limits(testCase)
        val decoder = Asn1Decoder(configured)
        val operation = testCase.path("operation").asText()
        return try {
            val root = decoder.decodeExact(materialize(testCase.path("input")))
            when (operation) {
                "decode-exact" -> linkedMapOf(
                    "outcome" to "value", "tag" to tag(root),
                    "header_hex" to toHex(bytes(root.header)), "value_hex" to toHex(bytes(root.value)),
                    "encoded_hex" to toHex(bytes(root.encoded)), "depth" to root.depth,
                    "elements_read" to decoder.elementsRead,
                )
                "cursor-script" -> cursorResult(testCase, decoder, root)
                "sequence", "set" -> {
                    val cursor = if (operation == "sequence") decoder.sequence(root) else decoder.set(root)
                    linkedMapOf(
                        "outcome" to "value", "elements_read" to decoder.elementsRead,
                        "remaining_offset" to root.value.remaining() - cursor.remaining.remaining(),
                    )
                }
                "explicit" -> {
                    val child = decoder.explicit(root, testCase.path("tag_number").asLong())
                    linkedMapOf(
                        "outcome" to "value", "tag" to tag(child), "value_hex" to toHex(bytes(child.value)),
                        "depth" to child.depth, "elements_read" to decoder.elementsRead,
                    )
                }
                else -> primitive(operation, root, configured, testCase.path("tag_number").asLong())
            }
        } catch (error: Asn1Error) {
            val scope = if (operation == "explicit" && error.kind == Asn1ErrorKind.FRAMING) {
                "container-value"
            } else {
                "operation-input"
            }
            failure(error, scope)
        }
    }

    private fun verifyUpstream(id: String): Any {
        val referenced = upstream.path("cases").firstOrNull { it.path("id").asText() == id }
            ?: error("missing upstream case $id")
        val decoder = Asn1Decoder(Asn1Limits(der = derLimits(upstream.path("defaults"), referenced.path("limits"))))
        val expected = referenced.path("expected")
        val input = materialize(referenced.path("input"))
        try {
            val element = decoder.decodeExact(input)
            assertEquals("element", expected.path("outcome").asText(), id)
            assertEquals(expected.path("tag").path("class").asText(), element.tag.tagClass, id)
            assertEquals(expected.path("tag").path("constructed").asBoolean(), element.tag.constructed, id)
            assertEquals(expected.path("tag").path("number").asLong(), element.tag.number, id)
            assertEquals(expected.path("header_len").asInt(), element.header.remaining(), id)
            assertEquals(expected.path("encoded_len").asInt(), element.encoded.remaining(), id)
            assertEquals(input.size, element.encoded.remaining(), id)
        } catch (failure: Asn1Error) {
            assertEquals("error", expected.path("outcome").asText(), id)
            assertEquals(Asn1ErrorKind.FRAMING, failure.kind, id)
            assertEquals(expected.path("error_id").asText(), failure.framingKind, id)
            assertEquals(expected.path("offset").asInt(), failure.offset, id)
            if (referenced.has("redacted_input_hex")) {
                assertFalse(failure.toString().contains(referenced.path("redacted_input_hex").asText()))
                assertFalse(mapper.writeValueAsString(failure(failure, "operation-input"))
                    .contains(referenced.path("redacted_input_hex").asText()))
            }
        }
        return mapOf("outcome" to "upstream")
    }

    private fun primitive(operation: String, element: Asn1Element, configured: Asn1Limits, tagNumber: Long): Any =
        when (operation) {
            "decode-boolean" -> linkedMapOf(
                "outcome" to "value", "boolean" to DerAsn1.decodeBoolean(element), "elements_read" to 1,
            )
            "decode-integer", "integer-to-u64" -> {
                val integer = DerAsn1.decodeInteger(element)
                val result = linkedMapOf<String, Any?>(
                    "outcome" to "value", "signed_hex" to toHex(bytes(integer.signedBytes)),
                    "negative" to integer.isNegative,
                )
                if (operation == "integer-to-u64") result["u64_decimal"] = integer.toU64().toString()
                result
            }
            "decode-bit-string" -> {
                val bits = DerAsn1.decodeBitString(element)
                linkedMapOf(
                    "outcome" to "value", "bytes_hex" to toHex(bytes(bits.bytes)),
                    "unused_bits" to bits.unusedBits, "bit_length" to bits.bitLength,
                )
            }
            "decode-octet-string" -> linkedMapOf("outcome" to "value", "bytes_hex" to toHex(bytes(DerAsn1.decodeOctetString(element))))
            "decode-implicit-octet-string" -> linkedMapOf("outcome" to "value", "bytes_hex" to toHex(bytes(DerAsn1.decodeImplicitOctetString(element, tagNumber))))
            "decode-ia5-string" -> linkedMapOf("outcome" to "value", "text" to DerAsn1.decodeIA5String(element))
            "decode-implicit-ia5-string" -> linkedMapOf("outcome" to "value", "text" to DerAsn1.decodeImplicitIA5String(element, tagNumber))
            "decode-null" -> { DerAsn1.decodeNull(element); linkedMapOf("outcome" to "value") }
            "decode-object-identifier", "decode-implicit-object-identifier" -> {
                val oid = if (operation == "decode-object-identifier") {
                    DerAsn1.decodeObjectIdentifier(element, configured)
                } else {
                    DerAsn1.decodeImplicitObjectIdentifier(element, tagNumber, configured)
                }
                linkedMapOf(
                    "outcome" to "value", "bytes_hex" to toHex(bytes(oid.encoded)),
                    "arcs_decimal" to oid.arcs.map(ULong::toString), "arc_count" to oid.arcCount,
                )
            }
            else -> error("unsupported operation $operation")
        }

    private fun cursorResult(testCase: JsonNode, decoder: Asn1Decoder, root: Asn1Element): Any {
        val cursor = decoder.sequence(root)
        val total = cursor.remaining.remaining()
        val events = mutableListOf<Map<String, Any?>>()
        for (raw in testCase.path("actions")) {
            val action = raw.asText()
            if (action == "finish") {
                try { cursor.finish(); events += linkedMapOf("outcome" to "finished") }
                catch (error: Asn1Error) { events += failure(error, "container-value") }
                continue
            }
            check(action == "read" || action == "read-with-different-limits" || action == "read-nested-sequence")
            val active = if (action != "read-with-different-limits") decoder else Asn1Decoder(
                decoder.limits.copy(maxTotalElements = decoder.limits.maxTotalElements + 1),
            )
            try {
                val child = cursor.read(active)
                if (action == "read-nested-sequence") {
                    checkNotNull(child)
                    val nested = decoder.sequence(child)
                    val grandchild = checkNotNull(nested.read(decoder))
                    nested.finish()
                    events += linkedMapOf("outcome" to "value", "tag" to tag(grandchild), "depth" to grandchild.depth)
                    continue
                }
                events += if (child == null) {
                    linkedMapOf("outcome" to "end")
                } else {
                    linkedMapOf("outcome" to "value", "tag" to tag(child), "depth" to child.depth)
                }
            } catch (error: Asn1Error) {
                events += failure(error, "container-value")
            }
        }
        return linkedMapOf(
            "outcome" to "value", "elements_read" to decoder.elementsRead,
            "remaining_offset" to total - cursor.remaining.remaining(), "events" to events,
        )
    }

    private fun limits(testCase: JsonNode): Asn1Limits {
        val defaults = fixture.path("defaults")
        val overrides = testCase.path("limits")
        return Asn1Limits(
            der = derLimits(defaults.path("der"), overrides.path("der")),
            maxDepth = intLimit(defaults, overrides, "max_depth"),
            maxTotalElements = longLimit(defaults, overrides, "max_total_elements"),
            maxOidArcs = intLimit(defaults, overrides, "max_oid_arcs"),
        )
    }

    private fun derLimits(defaults: JsonNode, overrides: JsonNode): DerTlv.Limits = DerTlv.Limits(
        maxInputLength = longLimit(defaults, overrides, "max_input_len"),
        maxValueLength = longLimit(defaults, overrides, "max_value_len"),
        maxElements = longLimit(defaults, overrides, "max_elements"),
        maxTagNumber = longLimit(defaults, overrides, "max_tag_number"),
    )

    private fun intLimit(defaults: JsonNode, overrides: JsonNode, name: String): Int =
        (if (overrides.has(name)) overrides.path(name) else defaults.path(name)).asInt()

    private fun longLimit(defaults: JsonNode, overrides: JsonNode, name: String): Long {
        val value = if (overrides.has(name)) overrides.path(name) else defaults.path(name)
        return if (value.isTextual) Long.MAX_VALUE else value.asLong()
    }

    private fun tag(element: Asn1Element): Map<String, Any> = linkedMapOf(
        "class" to element.tag.tagClass, "constructed" to element.tag.constructed, "number" to element.tag.number,
    )

    private fun failure(error: Asn1Error, scope: String): LinkedHashMap<String, Any?> = linkedMapOf<String, Any?>(
        "outcome" to "error", "error_id" to error.kind.id, "offset" to error.offset, "offset_scope" to scope,
    ).also { if (error.framingKind != null) it["framing_error_id"] = error.framingKind }

    private fun materialize(segments: JsonNode): ByteArray {
        val output = ByteArrayOutputStream()
        for (segment in segments) {
            val value = fromHex(if (segment.has("hex")) segment.path("hex").asText() else segment.path("repeat_hex").asText())
            repeat(if (segment.has("hex")) 1 else segment.path("count").asInt()) { output.writeBytes(value) }
        }
        return output.toByteArray()
    }

    private fun fromHex(value: String): ByteArray = ByteArray(value.length / 2) { index ->
        value.substring(index * 2, index * 2 + 2).toInt(16).toByte()
    }

    private fun toHex(value: ByteArray): String = value.joinToString("") { "%02x".format(it.toInt() and 0xff) }

    private fun bytes(buffer: ByteBuffer): ByteArray {
        val copy = buffer.duplicate()
        return ByteArray(copy.remaining()).also(copy::get)
    }

    private fun readFixture(name: String): JsonNode {
        val path = Path.of("..", "..", "..", "specs", "fixtures", name, "cases.json")
        return mapper.readTree(Files.readString(path))
    }
}
