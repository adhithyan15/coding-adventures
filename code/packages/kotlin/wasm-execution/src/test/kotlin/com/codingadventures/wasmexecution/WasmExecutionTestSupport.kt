package com.codingadventures.wasmexecution

object WasmExecutionTestSupport {
    fun encodeUnsigned(value: Long): ByteArray {
        var remaining = value
        val bytes = mutableListOf<Byte>()
        do {
            var current = (remaining and 0x7F).toInt()
            remaining = remaining ushr 7
            if (remaining != 0L) {
                current = current or 0x80
            }
            bytes += current.toByte()
        } while (remaining != 0L)
        return bytes.toByteArray()
    }

    fun encodeSigned32(value: Int): ByteArray {
        var remaining = value
        val bytes = mutableListOf<Byte>()
        var done = false
        while (!done) {
            var current = remaining and 0x7F
            remaining = remaining shr 7
            done = (remaining == 0 && (current and 0x40) == 0) || (remaining == -1 && (current and 0x40) != 0)
            if (!done) {
                current = current or 0x80
            }
            bytes += current.toByte()
        }
        return bytes.toByteArray()
    }

    fun encodeSigned64(value: Long): ByteArray {
        var remaining = value
        val bytes = mutableListOf<Byte>()
        var done = false
        while (!done) {
            var current = (remaining and 0x7F).toInt()
            remaining = remaining shr 7
            done = (remaining == 0L && (current and 0x40) == 0) || (remaining == -1L && (current and 0x40) != 0)
            if (!done) {
                current = current or 0x80
            }
            bytes += current.toByte()
        }
        return bytes.toByteArray()
    }
}
