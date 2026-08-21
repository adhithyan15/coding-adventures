package com.codingadventures.imagecodecpng

import com.codingadventures.pixelcontainer.ImageCodec
import com.codingadventures.pixelcontainer.PixelContainer
import com.codingadventures.zip.crc32
import java.io.ByteArrayOutputStream
import org.junit.jupiter.api.Assertions.assertArrayEquals
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Test

class PngTest {
    @Test
    fun codecImplementsPixelContainerContract() {
        val codec: ImageCodec = PngCodec()
        assertEquals("image/png", codec.mimeType)

        val pixels = PixelContainer(1, 1, byteArrayOf(1, 2, 3, 4))
        assertArrayEquals(pixels.data, codec.decode(codec.encode(pixels)).data)

        val limited = PngCodec(1.0)
        assertArrayEquals(pixels.data, limited.decode(limited.encode(pixels)).data)
    }

    @Test
    fun callerPixelLimitIsValidatedWithoutCoercion() {
        val invalid = listOf(
            0.0,
            -1.0,
            1.5,
            DEFAULT_MAX_PIXELS.toDouble() + 1.0,
            Double.NaN,
            Double.POSITIVE_INFINITY,
            Double.NEGATIVE_INFINITY,
        )
        for (value in invalid) {
            requireCode("invalid-max-pixels") { PngCodec(value) }
            requireCode("invalid-max-pixels") { decodePng(byteArrayOf(), value) }
        }
    }

    @Test
    fun encoderValidatesExplicitPixelContainerStateBeforeAllocating() {
        requireCode("invalid-image-dimensions") { encodePng(null) }
        requireCode("invalid-image-dimensions") {
            encodePng(PixelContainer(0, 1, byteArrayOf()))
        }
        requireCode("invalid-image-dimensions") {
            encodePng(PixelContainer(MAX_DIMENSION + 1, 1, byteArrayOf()))
        }
        requireCode("invalid-image-dimensions") {
            encodePng(PixelContainer(8192, 4097, byteArrayOf()))
        }
        requireCode("invalid-pixel-data-length") {
            encodePng(PixelContainer(1, 1, byteArrayOf(1, 2, 3)))
        }
    }

    @Test
    fun errorTaxonomyIsImmutableAndPayloadBlind() {
        assertEquals(29, ERROR_CODES.size)
        assertThrows(UnsupportedOperationException::class.java) {
            @Suppress("UNCHECKED_CAST")
            (ERROR_CODES as MutableList<String>)[0] = "changed"
        }
        val error = PngError("invalid-filter")
        assertEquals("invalid-filter", error.code)
        assertEquals("invalid-filter", error.message)
    }

    @Test
    fun apngRefusalPreservesCrcAndFirstChunkPrecedence() {
        val encoded = encodePng(PixelContainer(1, 1))
        val valid = chunk("acTL", byteArrayOf())
        requireCode("unsupported-feature") { decodePng(insert(encoded, 33, valid)) }

        val corrupt = valid.copyOf()
        corrupt[corrupt.lastIndex] = (corrupt.last().toInt() xor 1).toByte()
        requireCode("chunk-crc-mismatch") { decodePng(insert(encoded, 33, corrupt)) }
        requireCode("chunk-before-ihdr") { decodePng(insert(encoded, 8, valid)) }
    }

    @Test
    fun adlerMatchesPublishedBoundaryVector() {
        assertEquals(0x11e60398L, adler32("Wikipedia".toByteArray(Charsets.US_ASCII)))
        val boundary = ByteArray(5553) { it.toByte() }
        assertEquals(0x2ccab2efL, adler32(boundary))
    }

    private fun chunk(type: String, payload: ByteArray): ByteArray {
        val typeBytes = type.toByteArray(Charsets.US_ASCII)
        var checksum = crc32(typeBytes)
        checksum = crc32(payload, checksum)
        return ByteArrayOutputStream().apply {
            writeU32(this, payload.size.toLong())
            writeBytes(typeBytes)
            writeBytes(payload)
            writeU32(this, checksum.toLong() and 0xffffffffL)
        }.toByteArray()
    }

    private fun insert(original: ByteArray, offset: Int, inserted: ByteArray): ByteArray =
        ByteArray(original.size + inserted.size).also { output ->
            original.copyInto(output, 0, 0, offset)
            inserted.copyInto(output, offset)
            original.copyInto(output, offset + inserted.size, offset)
        }

    private fun writeU32(out: ByteArrayOutputStream, value: Long) {
        out.write((value ushr 24).toInt())
        out.write((value ushr 16).toInt())
        out.write((value ushr 8).toInt())
        out.write(value.toInt())
    }

    private fun requireCode(expected: String, action: () -> Unit) {
        val error = assertThrows(PngError::class.java) { action() }
        assertEquals(expected, error.code)
        assertEquals(expected, error.message)
    }
}
