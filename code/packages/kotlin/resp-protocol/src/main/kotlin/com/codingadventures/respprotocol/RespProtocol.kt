package com.codingadventures.respprotocol

import java.io.ByteArrayOutputStream

sealed interface RespValue {
    data class SimpleString(val value: String) : RespValue
    data class ErrorString(val value: String) : RespValue
    data class IntegerValue(val value: Long) : RespValue
    data class BulkString(val value: ByteArray?) : RespValue
    data class ArrayValue(val value: List<RespValue>?) : RespValue
}

data class DecodeResult(val value: RespValue, val nextOffset: Int)

object RespCodec {
    fun encode(value: RespValue): ByteArray = ByteArrayOutputStream().also { writeValue(it, value) }.toByteArray()

    fun decode(bytes: ByteArray, offset: Int = 0): DecodeResult? {
        if (offset >= bytes.size) return null
        return when (bytes[offset].toInt().toChar()) {
            '+' -> readLine(bytes, offset + 1)?.let { DecodeResult(RespValue.SimpleString(it.first), it.second) }
            '-' -> readLine(bytes, offset + 1)?.let { DecodeResult(RespValue.ErrorString(it.first), it.second) }
            ':' -> readLine(bytes, offset + 1)?.let { DecodeResult(RespValue.IntegerValue(it.first.toLong()), it.second) }
            '$' -> decodeBulkString(bytes, offset + 1)
            '*' -> decodeArray(bytes, offset + 1)
            else -> error("Unknown RESP prefix: ${bytes[offset].toInt().toChar()}")
        }
    }

    private fun decodeBulkString(bytes: ByteArray, offset: Int): DecodeResult? {
        val (lengthText, nextOffset) = readLine(bytes, offset) ?: return null
        val length = lengthText.toInt()
        if (length == -1) return DecodeResult(RespValue.BulkString(null), nextOffset)
        if (nextOffset + length + 2 > bytes.size) return null
        val payload = bytes.copyOfRange(nextOffset, nextOffset + length)
        return DecodeResult(RespValue.BulkString(payload), nextOffset + length + 2)
    }

    private fun decodeArray(bytes: ByteArray, offset: Int): DecodeResult? {
        val (countText, startOffset) = readLine(bytes, offset) ?: return null
        val count = countText.toInt()
        if (count == -1) return DecodeResult(RespValue.ArrayValue(null), startOffset)
        val values = mutableListOf<RespValue>()
        var cursor = startOffset
        repeat(count) {
            val decoded = decode(bytes, cursor) ?: return null
            values += decoded.value
            cursor = decoded.nextOffset
        }
        return DecodeResult(RespValue.ArrayValue(values), cursor)
    }

    private fun writeValue(out: ByteArrayOutputStream, value: RespValue) {
        when (value) {
            is RespValue.SimpleString -> writeRaw(out, "+${value.value}\r\n")
            is RespValue.ErrorString -> writeRaw(out, "-${value.value}\r\n")
            is RespValue.IntegerValue -> writeRaw(out, ":${value.value}\r\n")
            is RespValue.BulkString -> {
                if (value.value == null) {
                    writeRaw(out, "$-1\r\n")
                } else {
                    writeRaw(out, "$${value.value.size}\r\n")
                    out.write(value.value)
                    writeRaw(out, "\r\n")
                }
            }
            is RespValue.ArrayValue -> {
                if (value.value == null) {
                    writeRaw(out, "*-1\r\n")
                } else {
                    writeRaw(out, "*${value.value.size}\r\n")
                    value.value.forEach { writeValue(out, it) }
                }
            }
        }
    }

    private fun writeRaw(out: ByteArrayOutputStream, value: String) = out.write(value.toByteArray())

    private fun readLine(bytes: ByteArray, offset: Int): Pair<String, Int>? {
        for (index in offset until bytes.size - 1) {
            if (bytes[index] == '\r'.code.toByte() && bytes[index + 1] == '\n'.code.toByte()) {
                return String(bytes, offset, index - offset) to (index + 2)
            }
        }
        return null
    }
}
