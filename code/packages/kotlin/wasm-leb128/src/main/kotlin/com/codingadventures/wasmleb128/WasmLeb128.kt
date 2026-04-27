package com.codingadventures.wasmleb128

object WasmLeb128 {
    private const val CONTINUATION_BIT = 0x80
    private const val PAYLOAD_MASK = 0x7F
    private const val MAX_LEB128_BYTES_32 = 5

    class LEB128Error(message: String) : RuntimeException(message)

    data class UnsignedDecoding(val value: Long, val bytesConsumed: Int)

    data class SignedDecoding(val value: Int, val bytesConsumed: Int)

    fun decodeUnsigned(data: ByteArray, offset: Int = 0): UnsignedDecoding {
        var result = 0L
        var shift = 0
        var bytesConsumed = 0

        for (index in offset until data.size) {
            if (bytesConsumed >= MAX_LEB128_BYTES_32) {
                throw LEB128Error("LEB128 sequence exceeds maximum $MAX_LEB128_BYTES_32 bytes for a 32-bit value")
            }

            val current = data[index].toInt() and 0xFF
            val payload = current and PAYLOAD_MASK
            result = result or (payload.toLong() shl shift)
            shift += 7
            bytesConsumed++

            if ((current and CONTINUATION_BIT) == 0) {
                return UnsignedDecoding(result and 0xFFFF_FFFFL, bytesConsumed)
            }
        }

        throw LEB128Error(
            "LEB128 sequence is unterminated: reached end of data at offset ${offset + bytesConsumed} " +
                "without finding a byte with continuation bit = 0"
        )
    }

    fun decodeSigned(data: ByteArray, offset: Int = 0): SignedDecoding {
        var result = 0
        var shift = 0
        var bytesConsumed = 0
        var lastByte = 0

        for (index in offset until data.size) {
            if (bytesConsumed >= MAX_LEB128_BYTES_32) {
                throw LEB128Error("LEB128 sequence exceeds maximum $MAX_LEB128_BYTES_32 bytes for a 32-bit value")
            }

            val current = data[index].toInt() and 0xFF
            lastByte = current
            val payload = current and PAYLOAD_MASK
            result = result or (payload shl shift)
            shift += 7
            bytesConsumed++

            if ((current and CONTINUATION_BIT) == 0) {
                if (shift < 32 && (lastByte and 0x40) != 0) {
                    result = result or -(1 shl shift)
                }
                return SignedDecoding(result, bytesConsumed)
            }
        }

        throw LEB128Error(
            "LEB128 sequence is unterminated: reached end of data at offset ${offset + bytesConsumed} " +
                "without finding a byte with continuation bit = 0"
        )
    }

    fun encodeUnsigned(value: Long): ByteArray {
        var remaining = value and 0xFFFF_FFFFL
        val bytes = mutableListOf<Byte>()

        do {
            var current = (remaining and PAYLOAD_MASK.toLong()).toInt()
            remaining = remaining ushr 7
            if (remaining != 0L) {
                current = current or CONTINUATION_BIT
            }
            bytes += current.toByte()
        } while (remaining != 0L)

        return bytes.toByteArray()
    }

    fun encodeSigned(value: Int): ByteArray {
        var remaining = value
        val bytes = mutableListOf<Byte>()
        var done = false

        while (!done) {
            var current = remaining and PAYLOAD_MASK
            remaining = remaining shr 7

            done =
                (remaining == 0 && (current and 0x40) == 0) ||
                    (remaining == -1 && (current and 0x40) != 0)

            if (!done) {
                current = current or CONTINUATION_BIT
            }

            bytes += current.toByte()
        }

        return bytes.toByteArray()
    }
}
