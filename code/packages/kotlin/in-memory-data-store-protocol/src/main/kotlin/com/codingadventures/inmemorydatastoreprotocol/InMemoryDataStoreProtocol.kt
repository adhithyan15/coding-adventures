package com.codingadventures.inmemorydatastoreprotocol

import com.codingadventures.respprotocol.RespValue

import java.util.Locale

data class CommandFrame(val command: String, val args: List<ByteArray>) {
    companion object {
        fun fromParts(parts: List<ByteArray>): CommandFrame? {
            if (parts.isEmpty()) return null
            return CommandFrame(parts.first().decodeToString().uppercase(Locale.ROOT), parts.drop(1))
        }

        fun fromRespValue(value: RespValue): CommandFrame? {
            val array = value as? RespValue.ArrayValue ?: return null
            val parts = array.value?.map { (it as? RespValue.BulkString)?.value ?: return null } ?: return null
            return fromParts(parts)
        }
    }
}

sealed interface EngineResponse {
    fun toRespValue(): RespValue

    data class SimpleString(val value: String) : EngineResponse {
        override fun toRespValue(): RespValue = RespValue.SimpleString(value)
    }
    data class ErrorString(val value: String) : EngineResponse {
        override fun toRespValue(): RespValue = RespValue.ErrorString(value)
    }
    data class IntegerValue(val value: Long) : EngineResponse {
        override fun toRespValue(): RespValue = RespValue.IntegerValue(value)
    }
    data class BulkString(val value: ByteArray?) : EngineResponse {
        override fun toRespValue(): RespValue = RespValue.BulkString(value)
    }
    data class ArrayValue(val value: List<EngineResponse>?) : EngineResponse {
        override fun toRespValue(): RespValue = RespValue.ArrayValue(value?.map { it.toRespValue() })
    }
}

fun ok(): EngineResponse = EngineResponse.SimpleString("OK")
fun error(message: String): EngineResponse = EngineResponse.ErrorString(message)
fun integer(value: Long): EngineResponse = EngineResponse.IntegerValue(value)
fun bulkString(value: ByteArray?): EngineResponse = EngineResponse.BulkString(value)
