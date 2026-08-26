package com.codingadventures.canonicalcbor

import com.fasterxml.jackson.databind.ObjectMapper
import java.io.ByteArrayOutputStream
import java.io.IOException
import java.nio.file.Files
import java.nio.file.Path
import java.util.HexFormat
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

/** Executes every language-neutral CBR01 vector against the Kotlin lane. */
class PortableConformanceTest {
    @Test
    fun `exact portable bytes match shared oracle`() {
        val root = json.readTree(Files.readString(findFixture()))
        assertEquals(1, root.path("schema_version").asInt())
        assertEquals("rfc8949-section-4.2.3-length-first", root.path("profile").asText())
        assertEquals(CanonicalCbor.MAX_NESTING_DEPTH,
            root.path("limits").path("max_nesting_depth").asInt())
        assertEquals(CanonicalCbor.MAX_ENCODED_BYTES,
            root.path("limits").path("max_encoded_bytes").asInt())
        assertEquals(55, root.path("cases").size())

        for (testCase in root.path("cases")) {
            val id = testCase.path("id").asText()
            val operation = testCase.path("operation").asText()
            val input = testCase.path("input").asText()
            val expected = testCase.path("expected").asText()
            when (operation) {
                "round-trip" -> assertContentEquals(
                    fromHex(expected), CanonicalCbor.encodeChecked(CanonicalCbor.decode(fromHex(input))), id)
                "decode-error" -> {
                    val wire = if (input.startsWith("nested-array-wire:"))
                        nestedArrayWire(input.substringAfterLast(':').toInt()) else fromHex(input)
                    assertError(expected, id) { CanonicalCbor.decode(wire) }
                }
                "encode-map" -> assertContentEquals(
                    fromHex(expected), CanonicalCbor.encodeChecked(mapValue(input)), id)
                "generated-round-trip" -> assertContentEquals(
                    generatedWire(expected), CanonicalCbor.encodeChecked(generatedValue(input)), id)
                "encode-error" -> {
                    val value = if (input == "duplicate-map-key")
                        mapValue("6161=00;6161=01") else generatedValue(input)
                    assertError(expected, id) { CanonicalCbor.encodeChecked(value) }
                    val destination = ByteArrayOutputStream().apply { write(0xaa) }
                    assertError(expected, id) { CanonicalCbor.encodeIntoChecked(value, destination) }
                    assertContentEquals(byteArrayOf(0xaa.toByte()), destination.toByteArray(), id)
                }
                else -> error("$id: unknown operation")
            }
        }
    }

    @Test
    fun `unsigned maximum uses all eight argument bytes`() {
        val value = CborValue.Unsigned(ULong.MAX_VALUE)
        assertContentEquals(fromHex("1bffffffffffffffff"), CanonicalCbor.encodeChecked(value))
        assertEquals(value, CanonicalCbor.decode(fromHex("1bffffffffffffffff")))
    }

    @Test
    fun `errors never reflect payload bytes`() {
        val error = assertFailsWith<CborException> { CanonicalCbor.decode(fromHex("63e298")) }
        assertEquals("length-too-large", error.id)
        assertTrue(error.message!!.startsWith("canonical-cbor:"))
        assertTrue(!error.message!!.contains("e298"))
    }

    @Test
    fun `public values defend bytes and checked append publishes atomically`() {
        val source = byteArrayOf(1, 2, 3)
        val value = CborValue.Bytes(source)
        source[0] = 9
        assertContentEquals(byteArrayOf(1, 2, 3), value.value)
        assertEquals(CborValue.Bytes(byteArrayOf(1, 2, 3)), value)
        assertEquals(CborValue.Bytes(byteArrayOf(1, 2, 3)).hashCode(), value.hashCode())
        assertEquals("Bytes(length=3)", value.toString())
        assertEquals(CborValue.Array(listOf(CborValue.Null)), CborValue.Array(CborValue.Null))

        val destination = ByteArrayOutputStream().apply { write(0xaa) }
        CanonicalCbor.encodeIntoChecked(CborValue.Unsigned(24u), destination)
        assertContentEquals(fromHex("aa1818"), destination.toByteArray())
        assertFailsWith<IllegalArgumentException> { CborException("unknown-id") }
    }

    private fun assertError(id: String, caseId: String, action: () -> Unit) {
        val error = assertFailsWith<CborException>(caseId) { action() }
        assertEquals(id, error.id, caseId)
        assertTrue(error.message!!.startsWith("canonical-cbor:"), caseId)
    }

    private fun mapValue(specification: String): CborValue.Map = CborValue.Map(
        specification.split(';').map { fragment ->
            val (key, value) = fragment.split('=', limit = 2)
            CborValue.MapEntry(CanonicalCbor.decode(fromHex(key)), CanonicalCbor.decode(fromHex(value)))
        }
    )

    private fun generatedValue(specification: String): CborValue {
        if (specification.startsWith("nested-array:")) {
            var value: CborValue = CborValue.Null
            repeat(specification.substringAfterLast(':').toInt()) {
                value = CborValue.Array(listOf(value))
            }
            return value
        }
        val parts = specification.split(':')
        return CborValue.Bytes(ByteArray(parts[1].toInt()) { fromHex(parts[2])[0] })
    }

    private fun generatedWire(specification: String): ByteArray {
        if (specification.startsWith("wire:nested-array:")) {
            return nestedArrayWire(specification.substringAfterLast(':').toInt())
        }
        val parts = specification.split(':')
        val length = parts[2].toInt()
        val output = ByteArrayOutputStream(length + 9)
        when {
            length <= 23 -> output.write(0x40 or length)
            length <= 0xff -> { output.write(0x58); output.write(length) }
            length <= 0xffff -> { output.write(0x59); output.write(length ushr 8); output.write(length) }
            else -> {
                output.write(0x5a); output.write(length ushr 24); output.write(length ushr 16)
                output.write(length ushr 8); output.write(length)
            }
        }
        val repeated = fromHex(parts[3])[0]
        repeat(length) { output.write(repeated.toInt()) }
        return output.toByteArray()
    }

    private fun nestedArrayWire(depth: Int): ByteArray = ByteArray(depth + 1) { index ->
        if (index == depth) 0xf6.toByte() else 0x81.toByte()
    }

    companion object {
        private val json = ObjectMapper()
        private val hex = HexFormat.of()

        private fun fromHex(value: String): ByteArray = hex.parseHex(value)

        private fun findFixture(): Path {
            var directory: Path? = Path.of("").toAbsolutePath()
            while (directory != null) {
                val candidate = directory.resolve("code/specs/fixtures/canonical-cbor-v1/cases.json")
                if (Files.isRegularFile(candidate)) return candidate
                directory = directory.parent
            }
            throw IOException("canonical-cbor-v1 fixture not found")
        }
    }
}
