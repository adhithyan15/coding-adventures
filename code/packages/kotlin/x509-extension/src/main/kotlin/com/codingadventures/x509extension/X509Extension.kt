package com.codingadventures.x509extension

import com.codingadventures.derasn1.DerAsn1
import java.nio.ByteBuffer

/** Bounded decoding for one generic RFC 5280 Extension. */
object X509Extension {
    private const val BOOLEAN_TAG = 1L

    /** Stable payload-free Extension failure identifiers. */
    enum class ErrorKind(val id: String) {
        STRUCTURE("structure"),
        MISSING_EXTENSION_ID("missing-extension-id"),
        INVALID_EXTENSION_ID("invalid-extension-id"),
        INVALID_CRITICAL("invalid-critical"),
        ENCODED_DEFAULT_CRITICAL("encoded-default-critical"),
        MISSING_EXTENSION_VALUE("missing-extension-value"),
        INVALID_EXTENSION_VALUE("invalid-extension-value"),
        TRAILING_ELEMENT("trailing-element"),
    }

    /** Redacted Extension failure at an Extension-local byte offset. */
    class Error private constructor(
        val kind: ErrorKind,
        val offset: Int,
        val asn1Kind: DerAsn1.Asn1ErrorKind? = null,
        val framingKind: String? = null,
    ) : RuntimeException("X.509 extension error ${kind.id} at byte $offset") {
        companion object {
            @JvmSynthetic
            internal fun create(
                kind: ErrorKind,
                offset: Int,
                asn1Kind: DerAsn1.Asn1ErrorKind? = null,
                framingKind: String? = null,
            ): Error = Error(kind, offset, asn1Kind, framingKind)
        }
    }

    /** Validated immutable Extension value. */
    class Value private constructor(
        val extensionId: DerAsn1.ObjectIdentifier,
        val critical: Boolean,
        extensionValue: ByteArray,
    ) {
        private val content = extensionValue.copyOf()
        val extensionValue: ByteBuffer get() = ByteBuffer.wrap(content).asReadOnlyBuffer()

        companion object {
            @JvmSynthetic
            internal fun create(
                extensionId: DerAsn1.ObjectIdentifier,
                critical: Boolean,
                extensionValue: ByteBuffer,
            ): Value = Value(extensionId, critical, bytes(extensionValue))
        }
    }

    /** Decode one generic Extension under the decoder's shared budgets. */
    fun decodeX509Extension(
        decoder: DerAsn1.Asn1Decoder,
        element: DerAsn1.Asn1Element,
    ): Value {
        val valueOffset = element.header.remaining()
        val valueLength = element.value.remaining()
        val fields = try {
            decoder.sequence(element)
        } catch (failure: DerAsn1.Asn1Error) {
            throw structure(failure, failure.offset)
        }

        val extensionIdOffset = childOffset(valueOffset, valueLength, fields)
        val extensionIdElement = readChild(decoder, fields, valueOffset, extensionIdOffset)
            ?: fail(ErrorKind.MISSING_EXTENSION_ID, extensionIdOffset)
        val extensionId = try {
            DerAsn1.decodeObjectIdentifier(extensionIdElement, decoder.limits)
        } catch (failure: DerAsn1.Asn1Error) {
            throw semantic(ErrorKind.INVALID_EXTENSION_ID, failure, extensionIdOffset)
        }

        val secondOffset = childOffset(valueOffset, valueLength, fields)
        val second = readChild(decoder, fields, valueOffset, secondOffset)
            ?: fail(ErrorKind.MISSING_EXTENSION_VALUE, secondOffset)

        val critical: Boolean
        val extensionValueElement: DerAsn1.Asn1Element
        val extensionValueOffset: Int
        if (second.tag.number == BOOLEAN_TAG) {
            critical = try {
                DerAsn1.decodeBoolean(second)
            } catch (failure: DerAsn1.Asn1Error) {
                throw semantic(ErrorKind.INVALID_CRITICAL, failure, secondOffset)
            }
            if (!critical) fail(ErrorKind.ENCODED_DEFAULT_CRITICAL, secondOffset)
            extensionValueOffset = childOffset(valueOffset, valueLength, fields)
            extensionValueElement = readChild(decoder, fields, valueOffset, extensionValueOffset)
                ?: fail(ErrorKind.MISSING_EXTENSION_VALUE, extensionValueOffset)
        } else {
            critical = false
            extensionValueElement = second
            extensionValueOffset = secondOffset
        }

        val extensionValue = try {
            DerAsn1.decodeOctetString(extensionValueElement)
        } catch (failure: DerAsn1.Asn1Error) {
            throw semantic(ErrorKind.INVALID_EXTENSION_VALUE, failure, extensionValueOffset)
        }

        val trailingOffset = childOffset(valueOffset, valueLength, fields)
        if (readChild(decoder, fields, valueOffset, trailingOffset) != null) {
            fail(ErrorKind.TRAILING_ELEMENT, trailingOffset)
        }
        return Value.create(extensionId, critical, extensionValue)
    }

    private fun readChild(
        decoder: DerAsn1.Asn1Decoder,
        fields: DerAsn1.Asn1Cursor,
        valueOffset: Int,
        childOffset: Int,
    ): DerAsn1.Asn1Element? = try {
        fields.read(decoder)
    } catch (failure: DerAsn1.Asn1Error) {
        val offset = if (failure.kind == DerAsn1.Asn1ErrorKind.FRAMING) {
            valueOffset + failure.offset
        } else {
            childOffset + failure.offset
        }
        throw structure(failure, offset)
    }

    private fun childOffset(
        valueOffset: Int,
        valueLength: Int,
        fields: DerAsn1.Asn1Cursor,
    ): Int = valueOffset + valueLength - fields.remaining.remaining()

    private fun semantic(
        kind: ErrorKind,
        failure: DerAsn1.Asn1Error,
        childOffset: Int,
    ): Error = Error.create(
        kind,
        childOffset + failure.offset,
        failure.kind,
        failure.framingKind,
    )

    private fun structure(failure: DerAsn1.Asn1Error, offset: Int): Error = Error.create(
        ErrorKind.STRUCTURE,
        offset,
        failure.kind,
        failure.framingKind,
    )

    private fun fail(kind: ErrorKind, offset: Int): Nothing = throw Error.create(kind, offset)

    private fun bytes(buffer: ByteBuffer): ByteArray {
        val copy = buffer.duplicate()
        return ByteArray(copy.remaining()).also(copy::get)
    }
}
