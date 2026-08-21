package com.codingadventures.imagecodecpng

import com.codingadventures.pixelcontainer.PixelContainer
import com.fasterxml.jackson.databind.JsonNode
import com.fasterxml.jackson.databind.ObjectMapper
import java.awt.image.BufferedImage
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.nio.file.Files
import java.nio.file.Path
import java.util.HexFormat
import java.util.Locale
import java.util.zip.InflaterInputStream
import javax.imageio.ImageIO
import org.junit.jupiter.api.Assertions.assertArrayEquals
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNotNull
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Assertions.fail
import org.junit.jupiter.api.Test

class PortableConformanceTest {
    private val json = ObjectMapper()
    private val hex = HexFormat.of()

    private data class Chunk(val type: String, val data: ByteArray)

    @Test
    fun consumesEveryPortableCaseThroughPublicApis() {
        val document = json.readTree(Files.readString(fixturePath()))
        val cases = document.required("cases")

        assertEquals(1, document.required("schema_version").intValue())
        assertEquals("image-codec-png-v1", document.required("profile").textValue())
        assertEquals(85, cases.size())
        assertEquals(MAX_DIMENSION, document.at("/limits/max_dimension").intValue())
        assertEquals(DEFAULT_MAX_PIXELS, document.at("/limits/default_max_pixels").longValue())
        assertEquals(
            document.required("error_ids").map { it.textValue() },
            ERROR_CODES,
        )

        for (fixture in cases) {
            val id = fixture.required("id").textValue()
            try {
                when (fixture.required("operation").textValue()) {
                    "decode" -> assertDecode(fixture)
                    "decode-error" -> assertDecodeError(fixture)
                    "encode" -> assertEncode(fixture)
                    "encode-error" -> assertEncodeError(fixture)
                    "adler32" -> assertAdler(fixture)
                    else -> fail<Unit>("unknown fixture operation for $id")
                }
            } catch (error: AssertionError) {
                throw AssertionError("portable case failed: $id", error)
            } catch (error: RuntimeException) {
                throw AssertionError("portable case failed: $id", error)
            }
        }
    }

    private fun assertDecode(fixture: JsonNode) {
        val actual = decode(fixture)
        val expected = fixture.required("expected")
        assertEquals(expected.required("width").intValue(), actual.width)
        assertEquals(expected.required("height").intValue(), actual.height)
        assertArrayEquals(bytes(expected.required("rgba_hex").textValue()), actual.data)
    }

    private fun assertDecodeError(fixture: JsonNode) {
        val error = assertThrows(PngError::class.java) { decode(fixture) }
        val expected = fixture.at("/expected/error_id").textValue()
        assertEquals(expected, error.code)
        assertEquals(expected, error.message)
    }

    private fun decode(fixture: JsonNode): PixelContainer {
        val png = bytes(fixture.required("png_hex").textValue())
        val options = fixture.get("options")
        return if (options == null) {
            decodePng(png)
        } else {
            decodePng(png, options.required("max_pixels").doubleValue())
        }
    }

    private fun assertEncode(fixture: JsonNode) {
        val input = fixture.required("input")
        val encoded = encodeFixture(input)
        val expected = fixture.required("expected")
        val chunks = parseChunks(encoded)

        assertEquals(
            expected.required("chunk_types").map { it.textValue() },
            chunks.map { it.type },
        )
        assertEquals(expected.required("bit_depth").intValue(), encoded[24].toInt() and 0xff)
        assertEquals(expected.required("colour_type").intValue(), encoded[25].toInt() and 0xff)
        assertEquals(expected.required("interlace").intValue(), encoded[28].toInt() and 0xff)

        val filtered = inflateIdat(chunks)
        val width = exactFixtureDimension(input.required("width"))
        val height = exactFixtureDimension(input.required("height"))
        val rowSize = width * 4 + 1
        val actualFilters = (0 until height).map { filtered[it * rowSize].toInt() and 0xff }
        assertEquals(expected.required("filter_types").map { it.intValue() }, actualFilters)

        val foreign = ImageIO.read(ByteArrayInputStream(encoded))
        assertNotNull(foreign)
        assertEquals(width, foreign.width)
        assertEquals(height, foreign.height)
        assertArrayEquals(bytes(input.required("rgba_hex").textValue()), rgbaBytes(foreign))
    }

    private fun assertEncodeError(fixture: JsonNode) {
        val error = assertThrows(PngError::class.java) {
            encodeFixture(fixture.required("input"))
        }
        val expected = fixture.at("/expected/error_id").textValue()
        assertEquals(expected, error.code)
        assertEquals(expected, error.message)
    }

    private fun encodeFixture(input: JsonNode): ByteArray {
        val width = exactFixtureDimension(input.required("width"))
        val height = exactFixtureDimension(input.required("height"))
        return encodePng(
            PixelContainer(width, height, bytes(input.required("rgba_hex").textValue())),
        )
    }

    private fun exactFixtureDimension(node: JsonNode): Int {
        if (!node.isNumber) throw PngError("invalid-image-dimensions")
        val value = node.doubleValue()
        if (!value.isFinite() || value != kotlin.math.floor(value)
            || value < Int.MIN_VALUE.toDouble() || value > Int.MAX_VALUE.toDouble()
        ) {
            throw PngError("invalid-image-dimensions")
        }
        return value.toInt()
    }

    private fun assertAdler(fixture: JsonNode) {
        val actual = adler32(bytes(fixture.required("input_hex").textValue()))
        assertEquals(
            fixture.at("/expected/adler32_hex").textValue(),
            String.format(Locale.ROOT, "%08x", actual),
        )
    }

    private fun parseChunks(png: ByteArray): List<Chunk> {
        val chunks = mutableListOf<Chunk>()
        var offset = 8
        while (offset < png.size) {
            val length = readU32(png, offset)
            val end = offset.toLong() + 12L + length
            if (length > Int.MAX_VALUE || end > png.size) {
                fail<Unit>("encoder produced a truncated chunk")
            }
            val size = length.toInt()
            val type = String(png, offset + 4, 4, Charsets.US_ASCII)
            chunks += Chunk(type, png.copyOfRange(offset + 8, offset + 8 + size))
            offset = end.toInt()
        }
        return chunks
    }

    private fun inflateIdat(chunks: List<Chunk>): ByteArray {
        val idat = ByteArrayOutputStream()
        chunks.filter { it.type == "IDAT" }.forEach { idat.writeBytes(it.data) }
        return InflaterInputStream(ByteArrayInputStream(idat.toByteArray())).use { it.readAllBytes() }
    }

    private fun rgbaBytes(image: BufferedImage): ByteArray {
        val rgba = ByteArray(Math.multiplyExact(Math.multiplyExact(image.width, image.height), 4))
        var offset = 0
        for (y in 0 until image.height) {
            for (x in 0 until image.width) {
                val argb = image.getRGB(x, y)
                rgba[offset++] = (argb ushr 16).toByte()
                rgba[offset++] = (argb ushr 8).toByte()
                rgba[offset++] = argb.toByte()
                rgba[offset++] = (argb ushr 24).toByte()
            }
        }
        return rgba
    }

    private fun readU32(data: ByteArray, offset: Int): Long =
        ((data[offset].toLong() and 0xff) shl 24) or
            ((data[offset + 1].toLong() and 0xff) shl 16) or
            ((data[offset + 2].toLong() and 0xff) shl 8) or
            (data[offset + 3].toLong() and 0xff)

    private fun bytes(value: String): ByteArray = hex.parseHex(value)

    private fun fixturePath(): Path {
        var current: Path? = Path.of("").toAbsolutePath()
        while (current != null) {
            val candidate = current.resolve("code/specs/fixtures/image-codec-png-v1/cases.json")
            if (Files.isRegularFile(candidate)) return candidate
            current = current.parent
        }
        throw AssertionError("could not locate IC18 portable fixture corpus")
    }
}
