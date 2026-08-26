package com.codingadventures.canonicalcbor

import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import java.nio.charset.CharacterCodingException
import java.nio.charset.CodingErrorAction
import java.nio.charset.StandardCharsets

/**
 * Zero-production-dependency RFC 8949 section 4.2.3 encoding and decoding.
 *
 * The encoder sorts already-encoded keys by length and then unsigned bytes.
 * The decoder compares the exact key spans it consumed under the same order.
 * This symmetry is the important trick: the parser accepts one spelling for
 * every supported logical value, so byte-bound hashes remain reproducible.
 */
object CanonicalCbor {
    const val MAX_NESTING_DEPTH: Int = 128
    const val MAX_ENCODED_BYTES: Int = 1_048_576

    /** Encode one value without publishing partial bytes on failure. */
    fun encodeChecked(value: CborValue): ByteArray = Encoder().also {
        it.writeValue(value, 0)
    }.bytes()

    /** Append one complete encoding, leaving [destination] unchanged on failure. */
    fun encodeIntoChecked(value: CborValue, destination: ByteArrayOutputStream) {
        destination.writeBytes(encodeChecked(value))
    }

    /** Decode exactly one canonical item. */
    fun decode(bytes: ByteArray): CborValue {
        val cursor = Cursor(bytes)
        val value = cursor.readValue(0)
        if (cursor.remaining != 0) fail("trailing-bytes")
        return value
    }

    private fun fail(id: String): Nothing = throw CborException(id)

    private class Encoder {
        private val output = ByteArrayOutputStream()

        fun bytes(): ByteArray = output.toByteArray()

        fun writeValue(value: CborValue, depth: Int) {
            if (depth > MAX_NESTING_DEPTH) fail("encode-too-deep")
            when (value) {
                is CborValue.Unsigned -> writeArgument(0, value.value)
                is CborValue.Negative -> writeArgument(1, value.value)
                is CborValue.Bytes -> {
                    writeArgument(2, value.rawValue.size.toULong())
                    writeBytes(value.rawValue)
                }
                is CborValue.Text -> {
                    val payload = value.value.toByteArray(StandardCharsets.UTF_8)
                    writeArgument(3, payload.size.toULong())
                    writeBytes(payload)
                }
                is CborValue.Array -> {
                    writeArgument(4, value.values.size.toULong())
                    value.values.forEach { writeValue(it, depth + 1) }
                }
                is CborValue.Map -> writeMap(value, depth)
                is CborValue.Tag -> {
                    writeArgument(6, value.number)
                    writeValue(value.value, depth + 1)
                }
                is CborValue.Bool -> writeByte(if (value.value) 0xf5 else 0xf4)
                CborValue.Null -> writeByte(0xf6)
            }
        }

        private fun writeMap(map: CborValue.Map, depth: Int) {
            val entries = map.entries.map { entry ->
                val keyEncoder = Encoder()
                keyEncoder.writeValue(entry.key, depth + 1)
                EncodedEntry(keyEncoder.bytes(), entry.value)
            }.sortedWith { left, right -> compareLengthFirst(left.key, right.key) }

            entries.zipWithNext().forEach { (left, right) ->
                if (left.key.contentEquals(right.key)) fail("duplicate-map-key")
            }
            writeArgument(5, entries.size.toULong())
            entries.forEach { entry ->
                writeBytes(entry.key)
                writeValue(entry.value, depth + 1)
            }
        }

        private fun writeArgument(major: Int, argument: ULong) {
            val prefix = major shl 5
            when {
                argument <= 23u -> writeByte(prefix or argument.toInt())
                argument <= 0xffu -> {
                    writeByte(prefix or 24)
                    writeByte(argument.toInt())
                }
                argument <= 0xffffu -> {
                    writeByte(prefix or 25)
                    writeByte((argument shr 8).toInt())
                    writeByte(argument.toInt())
                }
                argument <= 0xffff_ffffu -> {
                    writeByte(prefix or 26)
                    for (shift in 24 downTo 0 step 8) writeByte((argument shr shift).toInt())
                }
                else -> {
                    writeByte(prefix or 27)
                    for (shift in 56 downTo 0 step 8) writeByte((argument shr shift).toInt())
                }
            }
        }

        private fun writeByte(value: Int) {
            if (output.size() >= MAX_ENCODED_BYTES) fail("encode-too-large")
            output.write(value)
        }

        private fun writeBytes(bytes: ByteArray) {
            if (bytes.size > MAX_ENCODED_BYTES - output.size()) fail("encode-too-large")
            output.writeBytes(bytes)
        }
    }

    private data class EncodedEntry(val key: ByteArray, val value: CborValue)
    private data class Header(val major: Int, val info: Int, val argument: ULong)

    private class Cursor(source: ByteArray) {
        private val bytes = source.copyOf()
        private var position = 0
        val remaining: Int get() = bytes.size - position

        private fun readByte(): Int {
            if (position >= bytes.size) fail("unexpected-eof")
            return bytes[position++].toInt() and 0xff
        }

        private fun readBytes(length: Int): ByteArray {
            if (length > remaining) fail("unexpected-eof")
            val start = position
            position += length
            return bytes.copyOfRange(start, position)
        }

        private fun readHeader(): Header {
            val initial = readByte()
            val major = initial ushr 5
            val info = initial and 0x1f
            val enforceMinimal = major != 7
            val argument = when (info) {
                in 0..23 -> info.toULong()
                24 -> readUnsigned(1).also { ensureMinimal(it, 23u, enforceMinimal) }
                25 -> readUnsigned(2).also { ensureMinimal(it, 0xffu, enforceMinimal) }
                26 -> readUnsigned(4).also { ensureMinimal(it, 0xffffu, enforceMinimal) }
                27 -> readUnsigned(8).also { ensureMinimal(it, 0xffff_ffffu, enforceMinimal) }
                in 28..30 -> fail("reserved")
                else -> fail("indefinite")
            }
            return Header(major, info, argument)
        }

        private fun readUnsigned(width: Int): ULong {
            var value = 0uL
            repeat(width) { value = (value shl 8) or readByte().toULong() }
            return value
        }

        private fun ensureMinimal(argument: ULong, previousMaximum: ULong, enabled: Boolean) {
            if (enabled && argument <= previousMaximum) fail("non-minimal-integer")
        }

        fun readValue(depth: Int): CborValue {
            if (depth > MAX_NESTING_DEPTH) fail("too-deep")
            val header = readHeader()
            return when (header.major) {
                0 -> CborValue.Unsigned(header.argument)
                1 -> CborValue.Negative(header.argument)
                2 -> CborValue.Bytes(readBytes(checkedLength(header.argument, 1)))
                3 -> CborValue.Text(readText(checkedLength(header.argument, 1)))
                4 -> readArray(checkedLength(header.argument, 1), depth)
                5 -> readMap(checkedLength(header.argument, 2), depth)
                6 -> CborValue.Tag(header.argument, readValue(depth + 1))
                7 -> readSimple(header.info)
                else -> error("three-bit major type escaped range")
            }
        }

        private fun checkedLength(declared: ULong, minimumBytesPerUnit: Int): Int {
            val maximum = remaining / minimumBytesPerUnit
            if (declared > maximum.toULong()) fail("length-too-large")
            return declared.toInt()
        }

        private fun readText(length: Int): String {
            val payload = readBytes(length)
            return try {
                StandardCharsets.UTF_8.newDecoder()
                    .onMalformedInput(CodingErrorAction.REPORT)
                    .onUnmappableCharacter(CodingErrorAction.REPORT)
                    .decode(ByteBuffer.wrap(payload)).toString()
            } catch (_: CharacterCodingException) {
                fail("invalid-utf8")
            }
        }

        private fun readArray(count: Int, depth: Int): CborValue.Array =
            CborValue.Array(List(count) { readValue(depth + 1) })

        private fun readMap(count: Int, depth: Int): CborValue.Map {
            val entries = ArrayList<CborValue.MapEntry>(count)
            var previousKey: ByteArray? = null
            repeat(count) {
                val keyStart = position
                val key = readValue(depth + 1)
                val encodedKey = bytes.copyOfRange(keyStart, position)
                previousKey?.let {
                    if (compareLengthFirst(it, encodedKey) >= 0) fail("non-canonical-map-order")
                }
                previousKey = encodedKey
                entries += CborValue.MapEntry(key, readValue(depth + 1))
            }
            return CborValue.Map(entries)
        }

        private fun readSimple(info: Int): CborValue = when (info) {
            20 -> CborValue.Bool(false)
            21 -> CborValue.Bool(true)
            22 -> CborValue.Null
            in 25..27 -> fail("float-not-supported")
            else -> fail("unsupported-simple")
        }
    }

    private fun compareLengthFirst(left: ByteArray, right: ByteArray): Int {
        val length = left.size.compareTo(right.size)
        return if (length != 0) length else compareUnsigned(left, right)
    }

    private fun compareUnsigned(left: ByteArray, right: ByteArray): Int {
        for (index in 0 until minOf(left.size, right.size)) {
            val comparison = (left[index].toInt() and 0xff).compareTo(right[index].toInt() and 0xff)
            if (comparison != 0) return comparison
        }
        return left.size.compareTo(right.size)
    }
}

