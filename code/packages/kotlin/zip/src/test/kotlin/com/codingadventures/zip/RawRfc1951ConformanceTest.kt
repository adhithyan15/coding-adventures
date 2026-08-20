package com.codingadventures.zip

import com.fasterxml.jackson.databind.JsonNode
import com.fasterxml.jackson.databind.ObjectMapper
import java.io.ByteArrayOutputStream
import java.io.IOException
import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.util.HexFormat
import java.util.zip.DataFormatException
import java.util.zip.Deflater
import java.util.zip.Inflater
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

/** Runs the shared CMP09 raw RFC 1951 corpus against the Kotlin lane. */
class RawRfc1951ConformanceTest {
    @Test
    fun closedPortableCorpusPasses() {
        val root = json.readTree(Files.readString(findFixture()))
        assertEquals(1, root.path("schema_version").asInt())
        assertEquals("zip-owned-raw-rfc1951-v1", root.path("profile").asText())
        assertEquals(MAX_RAW_OUTPUT, root.path("limits").path("default_max_output").asInt())
        assertEquals(MAX_RAW_OUTPUT, root.path("limits").path("hard_max_output").asInt())
        assertEquals(RAW_INFLATE_ERROR_CODES, root.path("error_ids").map { it.asText() })
        assertEquals(34, root.path("cases").size())

        for (testCase in root.path("cases")) {
            val id = testCase.path("id").asText()
            val limit = if (testCase.has("max_output")) testCase.path("max_output").asInt() else MAX_RAW_OUTPUT
            when (testCase.path("operation").asText()) {
                "inflate" -> {
                    val input = fromHex(testCase.path("input_hex").asText())
                    val expected = materialize(testCase.path("expected").path("output"))
                    val result = rawInflateCounted(input, limit)
                    assertContentEquals(expected, result.output, id)
                    assertEquals(testCase.path("expected").path("bytes_consumed").asInt(), result.bytesConsumed, id)
                    assertContentEquals(expected, rawInflate(input, limit), id)
                }
                "inflate-error" -> {
                    val input = fromHex(testCase.path("input_hex").asText())
                    val expected = testCase.path("expected").path("error_id").asText()
                    val error = assertFailsWith<RawInflateError>(id) { rawInflateCounted(input, limit) }
                    assertEquals(expected, error.code, id)
                    assertEquals(expected, error.message, id)
                }
                "deflate-interoperability" -> {
                    val input = fromHex(testCase.path("input_hex").asText())
                    val expected = materialize(testCase.path("expected").path("output"))
                    assertContentEquals(expected, jdkCodec("decompress", rawDeflate(input)), id)
                }
                "crc32" -> {
                    var checksum = if (testCase.has("initial_crc32_hex"))
                        testCase.path("initial_crc32_hex").asText().toLong(16).toInt() else 0
                    for (chunk in testCase.path("chunks_hex")) checksum = crc32(fromHex(chunk.asText()), checksum)
                    assertEquals(testCase.path("expected").path("crc32_hex").asText(),
                        checksum.toUInt().toString(16).padStart(8, '0'), id)
                }
                else -> error("$id: unknown operation")
            }
        }
    }

    @Test
    fun foreignFullWindowAndHistoricalWrapperPass() {
        val expected = ByteArray(65_536)
        for (index in 0 until 32_768) {
            expected[index] = ((index * 73 + index / 251) and 0xff).toByte()
            expected[index + 32_768] = expected[index]
        }
        assertContentEquals(expected, rawInflate(jdkCodec("compress", expected), expected.size))
        val historical = "historical wrapper compatibility".toByteArray()
        assertContentEquals(historical, rawInflate(rawDeflate(historical)))
    }

    @Test
    fun zipReaderRequiresExactContainerBoundaries() {
        val compressed = fromHex("0dc28911c0200c03b0d8f97028ec3f6ed129cab7dd96a0c2445bdb93809663a5d303f6b265e20c2b79ea03379d227e")
        val plain = fromHex("0406030b000e070909010906010a04070007000000000501010908030108050302030401000401000207090009020a0a020605020d060c01020b020302090201")
        assertContentEquals(plain, ZipReader(rawZip("dynamic.bin", compressed, plain, plain.size, 8)).read("dynamic.bin"))
        assertMessage("zip: compressed payload contains trailing bytes") {
            ZipReader(rawZip("cavity.bin", compressed + byteArrayOf(0xde.toByte(), 0xad.toByte()), plain, plain.size, 8)).read("cavity.bin")
        }
        assertMessage("zip: uncompressed size does not match the directory") {
            ZipReader(rawZip("size.bin", compressed, plain, plain.size + 1, 8)).read("size.bin")
        }
        assertMessage("zip: stored entry sizes do not match") {
            ZipReader(rawZip("stored.bin", plain, plain, plain.size + 1, 0)).read("stored.bin")
        }
        assertMessage("zip: raw inflate failed: reserved-block-type") {
            ZipReader(rawZip("malformed.bin", byteArrayOf(0x07), byteArrayOf(), 0, 8)).read("malformed.bin")
        }

        val writer = ZipWriter()
        writer.addFile("a.bin", ByteArray(4), compress = false)
        writer.addFile("b.bin", ByteArray(4), compress = false)
        assertMessage("zip: aggregate decompressed size exceeds the 7-byte limit (decompression bomb guard)") {
            ZipArchive.unzip(writer.finish(), maxTotalBytes = 7)
        }

        val invalidLargeEntry = rawZip(
            "preflight.bin", byteArrayOf(0x07), byteArrayOf(), 8, 8
        )
        assertMessage("zip: aggregate decompressed size exceeds the 7-byte limit (decompression bomb guard)") {
            ZipArchive.unzip(invalidLargeEntry, maxTotalBytes = 7)
        }
    }

    private fun assertMessage(expected: String, action: () -> Unit) {
        assertEquals(expected, assertFailsWith<IOException> { action() }.message)
    }

    companion object {
        private val json = ObjectMapper()
        private val hex = HexFormat.of()

        private fun materialize(output: JsonNode): ByteArray {
            if (output.has("hex")) return fromHex(output.path("hex").asText())
            return ByteArray(output.path("count").asInt()) { fromHex(output.path("repeat_hex").asText())[0] }
        }

        private fun fromHex(value: String): ByteArray = hex.parseHex(value)

        private fun findFixture(): Path {
            var directory: Path? = Path.of("").toAbsolutePath()
            while (directory != null) {
                val candidate = directory.resolve("code/specs/fixtures/zip-raw-rfc1951-v1/cases.json")
                if (Files.isRegularFile(candidate)) return candidate
                directory = directory.parent
            }
            throw IOException("zip-raw-rfc1951-v1 fixture not found")
        }

        private fun jdkCodec(mode: String, input: ByteArray): ByteArray {
            val buffer = ByteArray(4096)
            val output = ByteArrayOutputStream()
            if (mode == "compress") {
                val codec = Deflater(9, true)
                try {
                    codec.setInput(input)
                    codec.finish()
                    while (!codec.finished()) output.write(buffer, 0, codec.deflate(buffer))
                } finally {
                    codec.end()
                }
                return output.toByteArray()
            }

            val codec = Inflater(true)
            try {
                // JDK's nowrap inflater requires one dummy byte after a raw stream.
                codec.setInput(input + byteArrayOf(0))
                while (!codec.finished()) {
                    val count = codec.inflate(buffer)
                    if (count == 0 && (codec.needsInput() || codec.needsDictionary())) {
                        throw DataFormatException("JDK raw inflater did not reach end-of-stream")
                    }
                    output.write(buffer, 0, count)
                }
            } finally {
                codec.end()
            }
            return output.toByteArray()
        }

        private fun rawZip(name: String, compressed: ByteArray, plain: ByteArray, declaredSize: Int, method: Int): ByteArray {
            val archive = ByteArrayOutputStream()
            val nameBytes = name.toByteArray(StandardCharsets.UTF_8)
            val checksum = crc32(plain).toUInt().toLong()
            u32(archive, 0x04034b50); u16(archive, 20); u16(archive, 0x0800); u16(archive, method)
            u16(archive, 0); u16(archive, 0); u32(archive, checksum); u32(archive, compressed.size.toLong())
            u32(archive, declaredSize.toLong()); u16(archive, nameBytes.size); u16(archive, 0); archive.writeBytes(nameBytes); archive.writeBytes(compressed)
            val centralOffset = archive.size()
            u32(archive, 0x02014b50); u16(archive, 0x031e); u16(archive, 20); u16(archive, 0x0800); u16(archive, method)
            u16(archive, 0); u16(archive, 0); u32(archive, checksum); u32(archive, compressed.size.toLong()); u32(archive, declaredSize.toLong())
            u16(archive, nameBytes.size); u16(archive, 0); u16(archive, 0); u16(archive, 0); u16(archive, 0)
            u32(archive, 0); u32(archive, 0); archive.writeBytes(nameBytes)
            val centralSize = archive.size() - centralOffset
            u32(archive, 0x06054b50); u16(archive, 0); u16(archive, 0); u16(archive, 1); u16(archive, 1)
            u32(archive, centralSize.toLong()); u32(archive, centralOffset.toLong()); u16(archive, 0)
            return archive.toByteArray()
        }

        private fun u16(out: ByteArrayOutputStream, value: Int) { out.write(value); out.write(value ushr 8) }
        private fun u32(out: ByteArrayOutputStream, value: Long) { u16(out, value.toInt()); u16(out, (value ushr 16).toInt()) }
    }
}
