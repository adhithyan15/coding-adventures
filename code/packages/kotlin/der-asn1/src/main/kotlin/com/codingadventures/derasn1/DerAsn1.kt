package com.codingadventures.derasn1

import com.codingadventures.dertlv.DerTlv
import java.nio.ByteBuffer
import java.util.Collections

/** Bounded typed ASN.1 DER values over payload-blind DER framing. */
object DerAsn1 {
    const val DEFAULT_MAX_DEPTH = 32
    const val DEFAULT_MAX_TOTAL_ELEMENTS = 16_384L
    const val DEFAULT_MAX_OID_ARCS = 128

    data class Asn1Limits(
        val der: DerTlv.Limits = DerTlv.Limits(),
        val maxDepth: Int = DEFAULT_MAX_DEPTH,
        val maxTotalElements: Long = DEFAULT_MAX_TOTAL_ELEMENTS,
        val maxOidArcs: Int = DEFAULT_MAX_OID_ARCS,
    ) {
        init {
            require(maxDepth >= 0 && maxTotalElements >= 0 && maxOidArcs >= 0) {
                "ASN.1 limits must be non-negative"
            }
        }
    }

    enum class Asn1ErrorKind(val id: String) {
        FRAMING("framing"),
        UNEXPECTED_TAG("unexpected-tag"),
        DECODER_LIMIT_MISMATCH("decoder-limit-mismatch"),
        DEPTH_LIMIT_EXCEEDED("depth-limit-exceeded"),
        ELEMENT_LIMIT_EXCEEDED("element-limit-exceeded"),
        INVALID_BOOLEAN_LENGTH("invalid-boolean-length"),
        INVALID_BOOLEAN_VALUE("invalid-boolean-value"),
        EMPTY_INTEGER("empty-integer"),
        NON_MINIMAL_INTEGER("non-minimal-integer"),
        NEGATIVE_INTEGER("negative-integer"),
        INTEGER_OVERFLOW("integer-overflow"),
        MISSING_UNUSED_BIT_COUNT("missing-unused-bit-count"),
        INVALID_UNUSED_BIT_COUNT("invalid-unused-bit-count"),
        NON_ZERO_BIT_PADDING("non-zero-bit-padding"),
        BIT_LENGTH_OVERFLOW("bit-length-overflow"),
        NON_EMPTY_NULL("non-empty-null"),
        NON_ASCII_IA5_STRING("non-ascii-ia5-string"),
        EMPTY_OBJECT_IDENTIFIER("empty-object-identifier"),
        UNTERMINATED_OBJECT_IDENTIFIER("unterminated-object-identifier"),
        NON_MINIMAL_OBJECT_IDENTIFIER("non-minimal-object-identifier"),
        OBJECT_IDENTIFIER_OVERFLOW("object-identifier-overflow"),
        OID_ARC_LIMIT_EXCEEDED("oid-arc-limit-exceeded"),
    }

    class Asn1Error(
        val kind: Asn1ErrorKind,
        val offset: Int,
        val framingKind: String? = null,
    ) : RuntimeException("ASN.1 DER value error ${kind.id} at byte $offset")

    class Asn1Element private constructor(
        val tag: DerTlv.Tag,
        val depth: Int,
        header: ByteArray,
        value: ByteArray,
        encoded: ByteArray,
    ) {
        private val headerBytes = header.copyOf()
        private val valueBytes = value.copyOf()
        private val encodedBytes = encoded.copyOf()

        val header: ByteBuffer get() = readOnly(headerBytes)
        val value: ByteBuffer get() = readOnly(valueBytes)
        val encoded: ByteBuffer get() = readOnly(encodedBytes)
        @JvmSynthetic
        internal fun copyValue(): ByteArray = valueBytes.copyOf()

        @get:JvmSynthetic
        internal val valueOffset: Int get() = headerBytes.size

        companion object {
            @JvmSynthetic
            internal fun from(element: DerTlv.Element, depth: Int): Asn1Element =
                Asn1Element(
                    element.tag,
                    depth,
                    bytes(element.header),
                    bytes(element.value),
                    bytes(element.encoded),
                )
        }
    }

    class Asn1Decoder(val limits: Asn1Limits = Asn1Limits()) {
        var elementsRead: Long = 0
            private set

        fun decodeExact(input: ByteArray): Asn1Element {
            if (limits.maxDepth == 0) fail(Asn1ErrorKind.DEPTH_LIMIT_EXCEEDED, 0)
            requireCapacity(0)
            return try {
                val element = DerTlv.decodeExact(input, limits.der)
                elementsRead++
                Asn1Element.from(element, 0)
            } catch (failure: DerTlv.DerError) {
                throw framing(failure)
            }
        }

        fun sequence(element: Asn1Element): Asn1Cursor = constructed(element, "universal", 16)
        fun set(element: Asn1Element): Asn1Cursor = constructed(element, "universal", 17)

        fun explicit(element: Asn1Element, tagNumber: Long): Asn1Element {
            expectTag(element, "context-specific", true, tagNumber)
            val depth = childDepth(element)
            requireCapacity(element.valueOffset)
            return try {
                val child = DerTlv.decodeExact(element.copyValue(), limits.der)
                elementsRead++
                Asn1Element.from(child, depth)
            } catch (failure: DerTlv.DerError) {
                throw framing(failure)
            }
        }

        private fun constructed(element: Asn1Element, tagClass: String, number: Long): Asn1Cursor {
            expectTag(element, tagClass, true, number)
            val depth = childDepth(element)
            return try {
                Asn1Cursor.create(DerTlv.Cursor(element.copyValue(), limits.der), depth, limits)
            } catch (failure: DerTlv.DerError) {
                throw framing(failure)
            }
        }

        private fun childDepth(element: Asn1Element): Int {
            val depth = element.depth + 1
            if (depth >= limits.maxDepth) fail(Asn1ErrorKind.DEPTH_LIMIT_EXCEEDED, 0)
            return depth
        }

        @JvmSynthetic
        internal fun requireCapacity(offset: Int) {
            if (elementsRead >= limits.maxTotalElements) fail(Asn1ErrorKind.ELEMENT_LIMIT_EXCEEDED, offset)
        }

        @JvmSynthetic
        internal fun countElement() { elementsRead++ }
    }

    class Asn1Cursor private constructor(
        private val cursor: DerTlv.Cursor,
        private val childDepth: Int,
        private val limits: Asn1Limits,
    ) {
        val remaining: ByteBuffer get() = cursor.remaining.asReadOnlyBuffer()

        fun read(decoder: Asn1Decoder): Asn1Element? {
            if (!cursor.remaining.hasRemaining()) return null
            if (decoder.limits != limits) fail(Asn1ErrorKind.DECODER_LIMIT_MISMATCH, 0)
            decoder.requireCapacity(0)
            return try {
                val element = cursor.read() ?: return null
                decoder.countElement()
                Asn1Element.from(element, childDepth)
            } catch (failure: DerTlv.DerError) {
                throw framing(failure)
            }
        }

        fun finish() {
            try {
                cursor.finish()
            } catch (failure: DerTlv.DerError) {
                throw framing(failure)
            }
        }

        companion object {
            @JvmSynthetic
            internal fun create(cursor: DerTlv.Cursor, childDepth: Int, limits: Asn1Limits): Asn1Cursor =
                Asn1Cursor(cursor, childDepth, limits)
        }
    }

    class DerInteger private constructor(signedBytes: ByteArray, private val valueOffset: Int) {
        private val content = signedBytes.copyOf()
        val signedBytes: ByteBuffer get() = readOnly(content)
        val isNegative: Boolean get() = content[0].toInt() and 0x80 != 0

        fun toU64(): ULong {
            if (isNegative) fail(Asn1ErrorKind.NEGATIVE_INTEGER, valueOffset)
            val start = if (content[0] == 0.toByte()) 1 else 0
            if (content.size - start > 8) fail(Asn1ErrorKind.INTEGER_OVERFLOW, valueOffset)
            var value = 0uL
            for (index in start until content.size) value = value * 256uL + unsigned(content[index]).toULong()
            return value
        }

        companion object {
            @JvmSynthetic
            internal fun create(signedBytes: ByteArray, valueOffset: Int): DerInteger =
                DerInteger(signedBytes, valueOffset)
        }
    }

    class DerBitString private constructor(bytes: ByteArray, val unusedBits: Int, val bitLength: Long) {
        private val content = bytes.copyOf()
        val bytes: ByteBuffer get() = readOnly(content)

        companion object {
            @JvmSynthetic
            internal fun create(bytes: ByteArray, unusedBits: Int, bitLength: Long): DerBitString =
                DerBitString(bytes, unusedBits, bitLength)
        }
    }

    class ObjectIdentifier private constructor(encoded: ByteArray, arcs: List<ULong>) {
        private val content = encoded.copyOf()
        val encoded: ByteBuffer get() = readOnly(content)
        val arcs: List<ULong> = Collections.unmodifiableList(arcs.toList())
        val arcCount: Int get() = this.arcs.size
        fun equalsArcs(expected: List<ULong>): Boolean = this.arcs == expected

        companion object {
            @JvmSynthetic
            internal fun create(encoded: ByteArray, arcs: List<ULong>): ObjectIdentifier =
                ObjectIdentifier(encoded, arcs)
        }
    }

    fun decodeBoolean(element: Asn1Element): Boolean {
        expectUniversalPrimitive(element, 1)
        val value = element.copyValue()
        if (value.size != 1) fail(Asn1ErrorKind.INVALID_BOOLEAN_LENGTH, element.valueOffset)
        return when (unsigned(value[0])) {
            0 -> false
            0xff -> true
            else -> fail(Asn1ErrorKind.INVALID_BOOLEAN_VALUE, element.valueOffset)
        }
    }

    fun decodeInteger(element: Asn1Element): DerInteger {
        expectUniversalPrimitive(element, 2)
        val value = element.copyValue()
        if (value.isEmpty()) fail(Asn1ErrorKind.EMPTY_INTEGER, element.valueOffset)
        if (value.size > 1) {
            val first = unsigned(value[0])
            val second = unsigned(value[1])
            if ((first == 0 && second and 0x80 == 0) || (first == 0xff && second and 0x80 != 0)) {
                fail(Asn1ErrorKind.NON_MINIMAL_INTEGER, element.valueOffset)
            }
        }
        return DerInteger.create(value, element.valueOffset)
    }

    fun decodeBitString(element: Asn1Element): DerBitString {
        expectUniversalPrimitive(element, 3)
        val value = element.copyValue()
        if (value.isEmpty()) fail(Asn1ErrorKind.MISSING_UNUSED_BIT_COUNT, element.valueOffset)
        val unused = unsigned(value[0])
        val payload = value.copyOfRange(1, value.size)
        if (unused > 7 || (payload.isEmpty() && unused != 0)) {
            fail(Asn1ErrorKind.INVALID_UNUSED_BIT_COUNT, element.valueOffset)
        }
        if (unused != 0 && (unsigned(payload.last()) and ((1 shl unused) - 1)) != 0) {
            fail(Asn1ErrorKind.NON_ZERO_BIT_PADDING, element.valueOffset + value.size - 1)
        }
        val bitLength = try {
            Math.multiplyExact(payload.size.toLong(), 8L) - unused
        } catch (_: ArithmeticException) {
            fail(Asn1ErrorKind.BIT_LENGTH_OVERFLOW, element.valueOffset)
        }
        return DerBitString.create(payload, unused, bitLength)
    }

    fun decodeOctetString(element: Asn1Element): ByteBuffer {
        expectUniversalPrimitive(element, 4)
        return readOnly(element.copyValue())
    }

    fun decodeImplicitOctetString(element: Asn1Element, tagNumber: Long): ByteBuffer {
        expectContextPrimitive(element, tagNumber)
        return readOnly(element.copyValue())
    }

    fun decodeIA5String(element: Asn1Element): String {
        expectUniversalPrimitive(element, 22)
        return decodeAscii(element)
    }

    fun decodeImplicitIA5String(element: Asn1Element, tagNumber: Long): String {
        expectContextPrimitive(element, tagNumber)
        return decodeAscii(element)
    }

    fun decodeNull(element: Asn1Element) {
        expectUniversalPrimitive(element, 5)
        if (element.copyValue().isNotEmpty()) fail(Asn1ErrorKind.NON_EMPTY_NULL, element.valueOffset)
    }

    fun decodeObjectIdentifier(element: Asn1Element, limits: Asn1Limits = Asn1Limits()): ObjectIdentifier {
        expectUniversalPrimitive(element, 6)
        return decodeOid(element, limits)
    }

    fun decodeImplicitObjectIdentifier(
        element: Asn1Element,
        tagNumber: Long,
        limits: Asn1Limits = Asn1Limits(),
    ): ObjectIdentifier {
        expectContextPrimitive(element, tagNumber)
        return decodeOid(element, limits)
    }

    private fun decodeAscii(element: Asn1Element): String {
        val value = element.copyValue()
        for (index in value.indices) {
            if (unsigned(value[index]) > 0x7f) fail(Asn1ErrorKind.NON_ASCII_IA5_STRING, element.valueOffset + index)
        }
        return value.toString(Charsets.US_ASCII)
    }

    private fun decodeOid(element: Asn1Element, limits: Asn1Limits): ObjectIdentifier {
        val encoded = element.copyValue()
        if (encoded.isEmpty()) fail(Asn1ErrorKind.EMPTY_OBJECT_IDENTIFIER, element.valueOffset)
        val first = parseArc(encoded, 0, element.valueOffset)
        val arcs = mutableListOf<ULong>()
        when {
            first.value < 40uL -> { arcs += 0uL; arcs += first.value }
            first.value < 80uL -> { arcs += 1uL; arcs += first.value - 40uL }
            else -> { arcs += 2uL; arcs += first.value - 80uL }
        }
        if (arcs.size > limits.maxOidArcs) fail(Asn1ErrorKind.OID_ARC_LIMIT_EXCEEDED, element.valueOffset)
        var offset = first.nextOffset
        while (offset < encoded.size) {
            val start = offset
            val parsed = parseArc(encoded, offset, element.valueOffset)
            arcs += parsed.value
            if (arcs.size > limits.maxOidArcs) {
                fail(Asn1ErrorKind.OID_ARC_LIMIT_EXCEEDED, element.valueOffset + start)
            }
            offset = parsed.nextOffset
        }
        return ObjectIdentifier.create(encoded, arcs)
    }

    private data class ParsedArc(val value: ULong, val nextOffset: Int)

    private fun parseArc(encoded: ByteArray, start: Int, valueOffset: Int): ParsedArc {
        if (unsigned(encoded[start]) == 0x80) fail(Asn1ErrorKind.NON_MINIMAL_OBJECT_IDENTIFIER, valueOffset + start)
        var value = 0uL
        var offset = start
        while (true) {
            if (offset >= encoded.size) fail(Asn1ErrorKind.UNTERMINATED_OBJECT_IDENTIFIER, valueOffset + offset)
            val octet = unsigned(encoded[offset])
            val payload = (octet and 0x7f).toULong()
            if (value > (ULong.MAX_VALUE - payload) / 128uL) {
                fail(Asn1ErrorKind.OBJECT_IDENTIFIER_OVERFLOW, valueOffset + offset)
            }
            value = value * 128uL + payload
            offset++
            if (octet and 0x80 == 0) return ParsedArc(value, offset)
        }
    }

    private fun expectUniversalPrimitive(element: Asn1Element, number: Long) =
        expectTag(element, "universal", false, number)

    private fun expectContextPrimitive(element: Asn1Element, number: Long) =
        expectTag(element, "context-specific", false, number)

    private fun expectTag(element: Asn1Element, tagClass: String, constructed: Boolean, number: Long) {
        val tag = element.tag
        if (tag.tagClass != tagClass || tag.constructed != constructed || tag.number != number) {
            fail(Asn1ErrorKind.UNEXPECTED_TAG, 0)
        }
    }

    private fun unsigned(value: Byte): Int = value.toInt() and 0xff
    private fun readOnly(bytes: ByteArray): ByteBuffer = ByteBuffer.wrap(bytes).asReadOnlyBuffer()
    private fun bytes(buffer: ByteBuffer): ByteArray {
        val view = buffer.duplicate()
        return ByteArray(view.remaining()).also(view::get)
    }
    private fun fail(kind: Asn1ErrorKind, offset: Int): Nothing = throw Asn1Error(kind, offset)
    private fun framing(error: DerTlv.DerError): Asn1Error =
        Asn1Error(Asn1ErrorKind.FRAMING, error.offset, error.kind)
}
