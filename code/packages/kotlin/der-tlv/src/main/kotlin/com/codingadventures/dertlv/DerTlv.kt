package com.codingadventures.dertlv

import java.nio.ByteBuffer

/** Bounded, payload-blind DER tag-length-value framing. */
object DerTlv {
    private val tagClasses = arrayOf("universal", "application", "context-specific", "private")

    data class Limits(
        val maxInputLength: Long = 1_048_576,
        val maxValueLength: Long = 1_048_576,
        val maxElements: Long = 4_096,
        val maxTagNumber: Long = 0xffff_ffffL,
    ) {
        init {
            require(maxInputLength >= 0 && maxValueLength >= 0 && maxElements >= 0 && maxTagNumber >= 0) {
                "limits must be non-negative"
            }
            require(maxTagNumber <= 0xffff_ffffL) { "maxTagNumber must fit u32" }
        }
    }

    class DerError(val kind: String, val offset: Int) :
        RuntimeException("DER framing error $kind at byte $offset")

    data class Tag(val tagClass: String, val constructed: Boolean, val number: Long)

    class Element internal constructor(
        val tag: Tag,
        private val input: ByteArray,
        private val start: Int,
        private val headerLength: Int,
        private val encodedLength: Int,
    ) {
        val header: ByteBuffer get() = view(start, headerLength)
        val value: ByteBuffer get() = view(start + headerLength, encodedLength - headerLength)
        val encoded: ByteBuffer get() = view(start, encodedLength)

        private fun view(offset: Int, length: Int): ByteBuffer =
            ByteBuffer.wrap(input, offset, length).slice().asReadOnlyBuffer()
    }

    data class Decoded(val element: Element, val remainder: ByteBuffer)

    fun decodeOne(input: ByteArray, limits: Limits = Limits()): Decoded {
        val result = decodeAt(input, 0, input.size, limits)
        val remainder = ByteBuffer.wrap(input, result.nextOffset, input.size - result.nextOffset)
            .slice()
            .asReadOnlyBuffer()
        return Decoded(result.element, remainder)
    }

    fun decodeExact(input: ByteArray, limits: Limits = Limits()): Element {
        val result = decodeAt(input, 0, input.size, limits)
        if (result.nextOffset != input.size) throw error("trailing-data", result.nextOffset)
        return result.element
    }

    class Cursor(private val input: ByteArray, private val limits: Limits = Limits()) {
        var elementsRead: Long = 0
            private set
        private var offset = 0

        init {
            if (input.size > limits.maxInputLength) throw error("input-limit-exceeded", 0)
        }

        fun read(): Element? {
            if (offset == input.size) return null
            if (elementsRead >= limits.maxElements) throw error("element-limit-exceeded", offset)
            val result = decodeAt(input, offset, input.size - offset, limits)
            offset = result.nextOffset
            elementsRead++
            return result.element
        }

        fun finish() {
            if (offset != input.size) throw error("trailing-data", offset)
        }

        val remaining: ByteBuffer
            get() = ByteBuffer.wrap(input, offset, input.size - offset).slice().asReadOnlyBuffer()
    }

    private data class DecodeResult(val element: Element, val nextOffset: Int)

    private fun decodeAt(input: ByteArray, start: Int, available: Int, limits: Limits): DecodeResult {
        if (available > limits.maxInputLength) throw error("input-limit-exceeded", start)
        if (available == 0) throw error("empty-input", start)

        val first = unsigned(input[start])
        val tagClass = tagClasses[first ushr 6]
        val constructed = first and 0x20 != 0
        val low = first and 0x1f
        val number: Long
        val identifierLength: Int
        if (low != 0x1f) {
            number = low.toLong()
            identifierLength = 1
            if (number > limits.maxTagNumber) throw error("tag-limit-exceeded", start)
        } else {
            val high = decodeHighTag(input, start, available, limits)
            number = high.first
            identifierLength = high.second
        }
        if (tagClass == "universal" && number == 0L) throw error("end-of-contents", start)

        val length = decodeLength(input, start, available, identifierLength)
        if (length.value > limits.maxValueLength) throw error("value-limit-exceeded", length.offset)
        val headerLength = identifierLength + length.octets
        if (length.value > Int.MAX_VALUE.toLong() - headerLength) {
            throw error("length-host-overflow", length.offset)
        }
        val encodedLength = headerLength + length.value.toInt()
        if (encodedLength > available) throw error("truncated-value", start + available)
        return DecodeResult(
            Element(Tag(tagClass, constructed, number), input, start, headerLength, encodedLength),
            start + encodedLength,
        )
    }

    private fun decodeHighTag(input: ByteArray, start: Int, available: Int, limits: Limits): Pair<Long, Int> {
        var number = 0L
        var index = 1
        while (true) {
            if (index >= available) throw error("truncated-high-tag", start + index)
            val octet = unsigned(input[start + index])
            val payload = octet and 0x7f
            if (index == 1 && payload == 0) throw error("non-minimal-tag", start + index)
            if (number > (0xffff_ffffL - payload) / 128) throw error("tag-overflow", start + index)
            number = number * 128 + payload
            if (number > limits.maxTagNumber) throw error("tag-limit-exceeded", start + index)
            index++
            if (octet and 0x80 == 0) break
        }
        if (number < 31) throw error("non-minimal-tag", start)
        return number to index
    }

    private data class Length(val value: Long, val octets: Int, val offset: Int)

    private fun decodeLength(input: ByteArray, start: Int, available: Int, identifierLength: Int): Length {
        val lengthOffset = start + identifierLength
        if (identifierLength >= available) throw error("truncated-length", lengthOffset)
        val first = unsigned(input[lengthOffset])
        if (first < 0x80) return Length(first.toLong(), 1, lengthOffset)
        if (first == 0x80) throw error("indefinite-length", lengthOffset)
        if (first == 0xff) throw error("reserved-length", lengthOffset)
        val count = first and 0x7f
        if (count > 8) throw error("length-too-wide", lengthOffset)
        if (identifierLength + 1 + count > available) throw error("truncated-length", start + available)
        val leading = unsigned(input[lengthOffset + 1])
        if (leading == 0) throw error("non-minimal-length", lengthOffset + 1)
        if (count == 8 && leading and 0x80 != 0) throw error("length-host-overflow", lengthOffset)
        var value = 0L
        repeat(count) { index -> value = value * 256 + unsigned(input[lengthOffset + 1 + index]) }
        if (value < 128) throw error("non-minimal-length", lengthOffset)
        return Length(value, count + 1, lengthOffset)
    }

    private fun unsigned(value: Byte): Int = value.toInt() and 0xff
    private fun error(kind: String, offset: Int): DerError = DerError(kind, offset)
}
