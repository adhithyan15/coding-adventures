package com.codingadventures.imagecodecpng

import com.codingadventures.pixelcontainer.ImageCodec
import com.codingadventures.pixelcontainer.PixelContainer
import com.codingadventures.zip.RawInflateError
import com.codingadventures.zip.crc32
import com.codingadventures.zip.rawDeflate
import com.codingadventures.zip.rawInflateCounted
import java.io.ByteArrayOutputStream
import java.io.IOException
import java.util.Collections
import kotlin.math.abs
import kotlin.math.floor

/** Largest accepted width or height. */
const val MAX_DIMENSION: Int = 16_384

/** Default and hard total-pixel ceiling. */
const val DEFAULT_MAX_PIXELS: Long = 32L * 1024L * 1024L

/** Closed IC18 error taxonomy in normative order. */
val ERROR_CODES: List<String> = Collections.unmodifiableList(
    listOf(
        "invalid-max-pixels",
        "invalid-image-dimensions",
        "invalid-pixel-data-length",
        "file-too-short",
        "invalid-signature",
        "truncated-chunk",
        "invalid-chunk-type",
        "chunk-crc-mismatch",
        "chunk-before-ihdr",
        "duplicate-ihdr",
        "invalid-ihdr-length",
        "invalid-dimensions",
        "dimension-limit",
        "pixel-limit",
        "unsupported-feature",
        "invalid-plte",
        "invalid-trns",
        "nonconsecutive-idat",
        "invalid-iend",
        "trailing-data",
        "unknown-critical-chunk",
        "missing-required-chunk",
        "invalid-zlib-header",
        "preset-dictionary",
        "inflate-failed",
        "inflated-length-mismatch",
        "idat-cavity",
        "adler-mismatch",
        "invalid-filter",
    ),
)

/** A stable, payload-blind IC18 PNG failure. */
class PngError(val code: String) : RuntimeException(code)

/** PixelContainer adapter for the bounded IC18 PNG profile. */
class PngCodec(maxPixels: Double? = null) : ImageCodec {
    private val activeLimit = validateMaxPixels(maxPixels)

    override val mimeType: String = "image/png"

    override fun encode(container: PixelContainer): ByteArray = encodePng(container)

    override fun decode(data: ByteArray): PixelContainer = decodePngWithLimit(data, activeLimit)
}

private val SIGNATURE = byteArrayOf(
    0x89.toByte(), 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a,
)
private const val ADLER_MOD = 65_521L

/** Compute the RFC 1950 Adler-32 checksum. */
fun adler32(data: ByteArray): Long {
    var a = 1L
    var b = 0L
    var start = 0
    while (start < data.size) {
        val end = minOf(start + 5552, data.size)
        for (index in start until end) {
            a += data[index].toInt() and 0xff
            b += a
        }
        a %= ADLER_MOD
        b %= ADLER_MOD
        start = end
    }
    return ((b shl 16) or a) and 0xffff_ffffL
}

/** Encode RGBA8 pixels as a bounded colour-type-6 PNG. */
fun encodePng(pixels: PixelContainer?): ByteArray {
    if (pixels == null || pixels.width <= 0 || pixels.height <= 0
        || pixels.width > MAX_DIMENSION || pixels.height > MAX_DIMENSION
    ) {
        fail("invalid-image-dimensions")
    }
    val pixelCount = pixels.width.toLong() * pixels.height.toLong()
    if (pixelCount > DEFAULT_MAX_PIXELS) fail("invalid-image-dimensions")
    val expectedPixels = Math.multiplyExact(pixelCount, 4L)
    if (pixels.data.size.toLong() != expectedPixels) fail("invalid-pixel-data-length")

    val output = ByteArrayOutputStream()
    output.writeBytes(SIGNATURE)
    val ihdr = ByteArray(13)
    writeU32(ihdr, 0, pixels.width.toLong())
    writeU32(ihdr, 4, pixels.height.toLong())
    ihdr[8] = 8
    ihdr[9] = 6
    appendChunk(output, "IHDR", ihdr)

    val stride = Math.multiplyExact(pixels.width, 4)
    val filteredLength = Math.toIntExact(
        Math.multiplyExact(pixels.height.toLong(), stride.toLong() + 1L),
    )
    val filtered = ByteArray(filteredLength)
    val prior = ByteArray(stride)
    val scratch = ByteArray(stride)
    val best = ByteArray(stride)
    for (row in 0 until pixels.height) {
        val source = row * stride
        val destination = row * (stride + 1)
        filtered[destination] = chooseFilter(pixels.data, source, prior, scratch, best, 4).toByte()
        best.copyInto(filtered, destination + 1)
        pixels.data.copyInto(prior, 0, source, source + stride)
    }

    val deflated = try {
        rawDeflate(filtered)
    } catch (error: IOException) {
        throw IllegalStateException("deflate-failed", error)
    }
    val idat = ByteArray(Math.addExact(deflated.size, 6))
    idat[0] = 0x78
    idat[1] = 0x9c.toByte()
    deflated.copyInto(idat, 2)
    writeU32(idat, idat.size - 4, adler32(filtered))
    appendChunk(output, "IDAT", idat)
    appendChunk(output, "IEND", byteArrayOf())
    return output.toByteArray()
}

/** Decode a PNG using the default or a caller-lowered pixel ceiling. */
fun decodePng(data: ByteArray, maxPixels: Double? = null): PixelContainer =
    decodePngWithLimit(data, validateMaxPixels(maxPixels))

internal fun validateMaxPixels(value: Double?): Long {
    if (value == null) return DEFAULT_MAX_PIXELS
    if (!value.isFinite() || value != floor(value)
        || value <= 0 || value > DEFAULT_MAX_PIXELS.toDouble()
    ) {
        fail("invalid-max-pixels")
    }
    return value.toLong()
}

internal fun decodePngWithLimit(data: ByteArray, maxPixels: Long): PixelContainer {
    if (data.size < SIGNATURE.size) fail("file-too-short")
    if (!data.copyOfRange(0, SIGNATURE.size).contentEquals(SIGNATURE)) fail("invalid-signature")

    var width = 0L
    var height = 0L
    var bitDepth = 0
    var colourType = 0
    var sawIhdr = false
    var sawIend = false
    var sawPlte = false
    var sawTrns = false
    var inIdat = false
    var idatEnded = false
    var transparentGrey: Int? = null
    var transparentRgb: IntArray? = null
    val idatParts = mutableListOf<ByteArray>()

    var position = SIGNATURE.size
    while (position < data.size) {
        if (data.size - position < 8) fail("truncated-chunk")
        val length = readU32(data, position)
        val chunkEnd = position.toLong() + 12L + length
        if (chunkEnd > data.size || length > Int.MAX_VALUE) fail("truncated-chunk")
        val size = length.toInt()
        val typeStart = position + 4
        val dataStart = position + 8
        val dataEnd = dataStart + size
        val typeBytes = data.copyOfRange(typeStart, dataStart)
        if (!validChunkType(typeBytes)) fail("invalid-chunk-type")
        val chunkData = data.copyOfRange(dataStart, dataEnd)
        var checksum = crc32(typeBytes)
        checksum = crc32(chunkData, checksum)
        if ((checksum.toLong() and 0xffff_ffffL) != readU32(data, dataEnd)) {
            fail("chunk-crc-mismatch")
        }
        val type = String(typeBytes, Charsets.US_ASCII)
        if (!sawIhdr && type != "IHDR") fail("chunk-before-ihdr")

        when (type) {
            "IHDR" -> {
                if (sawIhdr) fail("duplicate-ihdr")
                if (size != 13) fail("invalid-ihdr-length")
                width = readU32(chunkData, 0)
                height = readU32(chunkData, 4)
                bitDepth = chunkData[8].toInt() and 0xff
                colourType = chunkData[9].toInt() and 0xff
                if (width == 0L || height == 0L) fail("invalid-dimensions")
                if (width > MAX_DIMENSION || height > MAX_DIMENSION) fail("dimension-limit")
                if (Math.multiplyExact(width, height) > maxPixels) fail("pixel-limit")
                if ((chunkData[10].toInt() and 0xff) != 0
                    || (chunkData[11].toInt() and 0xff) != 0
                    || (chunkData[12].toInt() and 0xff) != 0
                ) {
                    fail("unsupported-feature")
                }
                if (bitDepth != 8 || colourType !in setOf(0, 2, 4, 6)) {
                    fail("unsupported-feature")
                }
                sawIhdr = true
            }
            "PLTE" -> {
                if (sawPlte || idatParts.isNotEmpty() || sawTrns
                    || colourType !in setOf(2, 6)
                    || size < 3 || size > 768 || size % 3 != 0
                ) {
                    fail("invalid-plte")
                }
                sawPlte = true
            }
            "tRNS" -> {
                if (sawTrns || idatParts.isNotEmpty()) fail("invalid-trns")
                when (colourType) {
                    0 -> {
                        if (size != 2 || readU16(chunkData, 0) > 255) fail("invalid-trns")
                        transparentGrey = readU16(chunkData, 0)
                    }
                    2 -> {
                        if (size != 6) fail("invalid-trns")
                        transparentRgb = IntArray(3) { index ->
                            readU16(chunkData, index * 2).also { sample ->
                                if (sample > 255) fail("invalid-trns")
                            }
                        }
                    }
                    else -> fail("invalid-trns")
                }
                sawTrns = true
            }
            "IDAT" -> {
                if (idatEnded) fail("nonconsecutive-idat")
                idatParts += chunkData
                inIdat = true
            }
            "IEND" -> {
                if (size != 0) fail("invalid-iend")
                if (chunkEnd != data.size.toLong()) fail("trailing-data")
                sawIend = true
                position = chunkEnd.toInt()
                continue
            }
            "acTL", "fcTL", "fdAT" -> fail("unsupported-feature")
            else -> if ((typeBytes[0].toInt() and 0x20) == 0) fail("unknown-critical-chunk")
        }

        if (type != "IDAT" && inIdat) {
            inIdat = false
            idatEnded = true
        }
        position = chunkEnd.toInt()
    }

    if (!sawIhdr || !sawIend || idatParts.isEmpty()) fail("missing-required-chunk")
    var zlibLength = 0L
    for (part in idatParts) zlibLength = Math.addExact(zlibLength, part.size.toLong())
    if (zlibLength > data.size || zlibLength > Int.MAX_VALUE) fail("truncated-chunk")
    val zlib = ByteArray(zlibLength.toInt())
    var zlibOffset = 0
    for (part in idatParts) {
        part.copyInto(zlib, zlibOffset)
        zlibOffset += part.size
    }
    if (zlib.size < 6) fail("invalid-zlib-header")
    val cmf = zlib[0].toInt() and 0xff
    val flg = zlib[1].toInt() and 0xff
    if ((cmf and 0x0f) != 8 || (cmf ushr 4) > 7 || (((cmf shl 8) or flg) % 31) != 0) {
        fail("invalid-zlib-header")
    }
    if ((flg and 0x20) != 0) fail("preset-dictionary")

    val channels = when (colourType) {
        0 -> 1
        2 -> 3
        4 -> 2
        else -> 4
    }
    val strideLong = Math.multiplyExact(width, channels.toLong())
    val expectedLong = Math.multiplyExact(height, Math.addExact(strideLong, 1L))
    if (expectedLong > Int.MAX_VALUE) fail("pixel-limit")
    val expected = expectedLong.toInt()
    val deflate = zlib.copyOfRange(2, zlib.size - 4)
    val inflated = try {
        rawInflateCounted(deflate, expected)
    } catch (error: RawInflateError) {
        if (error.code == "output-limit-exceeded") fail("inflated-length-mismatch")
        fail("inflate-failed")
    }
    if (inflated.output.size != expected) fail("inflated-length-mismatch")
    if (inflated.bytesConsumed != deflate.size) fail("idat-cavity")
    if (adler32(inflated.output) != readU32(zlib, zlib.size - 4)) fail("adler-mismatch")

    val stride = Math.toIntExact(strideLong)
    val rowSize = stride + 1
    for (row in 0 until height.toInt()) {
        if ((inflated.output[row * rowSize].toInt() and 0xff) > 4) fail("invalid-filter")
    }

    val rgba = ByteArray(Math.toIntExact(Math.multiplyExact(width * height, 4L)))
    val prior = ByteArray(stride)
    for (rowIndex in 0 until height.toInt()) {
        val source = rowIndex * rowSize
        val row = inflated.output.copyOfRange(source + 1, source + rowSize)
        undoFilter(inflated.output[source].toInt() and 0xff, row, prior, channels)
        val destination = rowIndex * width.toInt() * 4
        for (x in 0 until width.toInt()) {
            val from = x * channels
            val to = destination + x * 4
            val first = row[from].toInt() and 0xff
            when (channels) {
                1 -> {
                    rgba[to] = row[from]
                    rgba[to + 1] = row[from]
                    rgba[to + 2] = row[from]
                    rgba[to + 3] = if (transparentGrey != null && first == transparentGrey) 0 else 0xff.toByte()
                }
                2 -> {
                    rgba[to] = row[from]
                    rgba[to + 1] = row[from]
                    rgba[to + 2] = row[from]
                    rgba[to + 3] = row[from + 1]
                }
                3 -> {
                    val green = row[from + 1].toInt() and 0xff
                    val blue = row[from + 2].toInt() and 0xff
                    rgba[to] = row[from]
                    rgba[to + 1] = row[from + 1]
                    rgba[to + 2] = row[from + 2]
                    rgba[to + 3] = if (transparentRgb != null
                        && first == transparentRgb[0]
                        && green == transparentRgb[1]
                        && blue == transparentRgb[2]
                    ) 0 else 0xff.toByte()
                }
                else -> row.copyInto(rgba, to, from, from + 4)
            }
        }
        row.copyInto(prior)
    }
    return PixelContainer(width.toInt(), height.toInt(), rgba)
}

private fun chooseFilter(
    raw: ByteArray,
    rawOffset: Int,
    prior: ByteArray,
    scratch: ByteArray,
    best: ByteArray,
    bytesPerPixel: Int,
): Int {
    var bestFilter = 0
    var bestScore = Int.MAX_VALUE
    for (filter in 0..4) {
        applyFilter(filter, raw, rawOffset, prior, scratch, bytesPerPixel)
        var score = 0
        for (value in scratch) {
            val unsigned = value.toInt() and 0xff
            score += if (unsigned < 128) unsigned else 256 - unsigned
        }
        if (score < bestScore) {
            bestScore = score
            bestFilter = filter
            scratch.copyInto(best)
        }
    }
    return bestFilter
}

private fun applyFilter(
    filter: Int,
    raw: ByteArray,
    rawOffset: Int,
    prior: ByteArray,
    output: ByteArray,
    bytesPerPixel: Int,
) {
    for (index in output.indices) {
        val value = raw[rawOffset + index].toInt() and 0xff
        val left = if (index >= bytesPerPixel) raw[rawOffset + index - bytesPerPixel].toInt() and 0xff else 0
        val above = prior[index].toInt() and 0xff
        val aboveLeft = if (index >= bytesPerPixel) prior[index - bytesPerPixel].toInt() and 0xff else 0
        val prediction = when (filter) {
            1 -> left
            2 -> above
            3 -> (left + above) / 2
            4 -> paeth(left, above, aboveLeft)
            else -> 0
        }
        output[index] = (value - prediction).toByte()
    }
}

private fun undoFilter(filter: Int, row: ByteArray, prior: ByteArray, bytesPerPixel: Int) {
    for (index in row.indices) {
        val left = if (index >= bytesPerPixel) row[index - bytesPerPixel].toInt() and 0xff else 0
        val above = prior[index].toInt() and 0xff
        val aboveLeft = if (index >= bytesPerPixel) prior[index - bytesPerPixel].toInt() and 0xff else 0
        val prediction = when (filter) {
            0 -> 0
            1 -> left
            2 -> above
            3 -> (left + above) / 2
            4 -> paeth(left, above, aboveLeft)
            else -> fail("invalid-filter")
        }
        row[index] = ((row[index].toInt() and 0xff) + prediction).toByte()
    }
}

private fun paeth(left: Int, above: Int, aboveLeft: Int): Int {
    val prediction = left + above - aboveLeft
    val leftDistance = abs(prediction - left)
    val aboveDistance = abs(prediction - above)
    val diagonalDistance = abs(prediction - aboveLeft)
    if (leftDistance <= aboveDistance && leftDistance <= diagonalDistance) return left
    if (aboveDistance <= diagonalDistance) return above
    return aboveLeft
}

private fun validChunkType(type: ByteArray): Boolean {
    if (type.size != 4 || (type[2].toInt() and 0x20) != 0) return false
    return type.all { value ->
        val unsigned = value.toInt() and 0xff
        unsigned in 'A'.code..'Z'.code || unsigned in 'a'.code..'z'.code
    }
}

private fun appendChunk(output: ByteArrayOutputStream, type: String, data: ByteArray) {
    val typeBytes = type.toByteArray(Charsets.US_ASCII)
    writeU32(output, data.size.toLong())
    output.writeBytes(typeBytes)
    output.writeBytes(data)
    var checksum = crc32(typeBytes)
    checksum = crc32(data, checksum)
    writeU32(output, checksum.toLong() and 0xffff_ffffL)
}

private fun readU16(data: ByteArray, offset: Int): Int =
    ((data[offset].toInt() and 0xff) shl 8) or (data[offset + 1].toInt() and 0xff)

private fun readU32(data: ByteArray, offset: Int): Long =
    ((data[offset].toLong() and 0xff) shl 24) or
        ((data[offset + 1].toLong() and 0xff) shl 16) or
        ((data[offset + 2].toLong() and 0xff) shl 8) or
        (data[offset + 3].toLong() and 0xff)

private fun writeU32(data: ByteArray, offset: Int, value: Long) {
    data[offset] = (value ushr 24).toByte()
    data[offset + 1] = (value ushr 16).toByte()
    data[offset + 2] = (value ushr 8).toByte()
    data[offset + 3] = value.toByte()
}

private fun writeU32(output: ByteArrayOutputStream, value: Long) {
    output.write((value ushr 24).toInt())
    output.write((value ushr 16).toInt())
    output.write((value ushr 8).toInt())
    output.write(value.toInt())
}

private fun fail(code: String): Nothing = throw PngError(code)
