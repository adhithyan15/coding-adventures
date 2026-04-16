package com.codingadventures.inmemorydatastore

import com.codingadventures.inmemorydatastoreengine.DataStoreEngine
import com.codingadventures.inmemorydatastoreprotocol.CommandFrame
import com.codingadventures.inmemorydatastoreprotocol.EngineResponse
import com.codingadventures.inmemorydatastoreprotocol.error
import com.codingadventures.respprotocol.RespCodec
import com.codingadventures.respprotocol.RespValue

class DataStoreManager {
    private val engine = DataStoreEngine()

    fun executeFrame(frame: CommandFrame?): EngineResponse = engine.executeFrame(frame)
    fun executeParts(parts: List<ByteArray>): EngineResponse = executeFrame(CommandFrame.fromParts(parts))

    fun executeRespBytes(request: ByteArray): ByteArray {
        val decoded = RespCodec.decode(request)
        val response = if (decoded == null) error("ERR incomplete RESP frame") else executeFrame(CommandFrame.fromRespValue(decoded.value))
        return RespCodec.encode(response.toRespValue())
    }

    fun executeRespValue(value: RespValue): RespValue = executeFrame(CommandFrame.fromRespValue(value)).toRespValue()
}
