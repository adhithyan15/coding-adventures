package com.codingadventures.canonicalcbor

/**
 * The closed value algebra supported by CBR01.
 *
 * Unsigned Kotlin values make the CBOR model explicit: [Negative.value]
 * stores the wire argument `n` for the mathematical value `-1 - n`.
 */
sealed interface CborValue {
    data class Unsigned(val value: ULong) : CborValue
    data class Negative(val value: ULong) : CborValue

    class Bytes(value: ByteArray) : CborValue {
        private val storage = value.copyOf()
        val value: ByteArray get() = storage.copyOf()
        internal val rawValue: ByteArray get() = storage

        override fun equals(other: Any?): Boolean =
            other is Bytes && storage.contentEquals(other.storage)

        override fun hashCode(): Int = storage.contentHashCode()
        override fun toString(): String = "Bytes(length=${storage.size})"
    }

    data class Text(val value: String) : CborValue
    data class Array(val values: List<CborValue>) : CborValue {
        constructor(vararg values: CborValue) : this(values.toList())
    }

    data class MapEntry(val key: CborValue, val value: CborValue)
    data class Map(val entries: List<MapEntry>) : CborValue
    data class Tag(val number: ULong, val value: CborValue) : CborValue
    data class Bool(val value: Boolean) : CborValue
    data object Null : CborValue
}

