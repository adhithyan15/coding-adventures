package com.codingadventures.inmemorydatastore

import com.codingadventures.inmemorydatastoreengine.DataStoreEngine.Companion.currentTimeMs
import com.codingadventures.inmemorydatastoreengine.DataStoreEngine
import com.codingadventures.inmemorydatastoreprotocol.CommandFrame
import com.codingadventures.inmemorydatastoreprotocol.EngineResponse
import com.codingadventures.inmemorydatastoreprotocol.error
import com.codingadventures.respprotocol.RespCodec
import com.codingadventures.respprotocol.RespValue
import java.io.Closeable
import java.io.FileOutputStream
import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path

class DataStoreManager @JvmOverloads constructor(
    aofPath: Path? = null,
    private val aofSyncPolicy: AofSyncPolicy = AofSyncPolicy.ALWAYS,
) : Closeable {
    private val engine = DataStoreEngine()
    private val aofStream: FileOutputStream? = aofPath?.let { path ->
        replayAof(path)
        path.toAbsolutePath().parent?.let(Files::createDirectories)
        FileOutputStream(path.toFile(), true)
    }

    fun executeFrame(frame: CommandFrame?): EngineResponse = engine.executeFrame(frame).also { appendToAof(frame, it) }
    fun executeParts(parts: List<ByteArray>): EngineResponse = executeFrame(CommandFrame.fromParts(parts))

    fun executeRespBytes(request: ByteArray): ByteArray {
        val decoded = RespCodec.decode(request)
        val response = if (decoded == null) error("ERR incomplete RESP frame") else executeFrame(CommandFrame.fromRespValue(decoded.value))
        return RespCodec.encode(response.toRespValue())
    }

    fun executeRespValue(value: RespValue): RespValue = executeFrame(CommandFrame.fromRespValue(value)).toRespValue()

    override fun close() {
        aofStream?.close()
    }

    private fun replayAof(aofPath: Path) {
        if (!Files.exists(aofPath)) return
        val bytes = Files.readAllBytes(aofPath)
        var offset = 0
        while (offset < bytes.size) {
            val decoded = RespCodec.decode(bytes, offset) ?: break
            engine.executeFrame(CommandFrame.fromRespValue(decoded.value))
            offset = decoded.nextOffset
        }
    }

    private fun appendToAof(frame: CommandFrame?, response: EngineResponse) {
        if (frame == null || aofStream == null || response is EngineResponse.ErrorString || frame.command !in AOF_COMMANDS) return
        val command = canonicalCommand(frame)
        aofStream.write(RespCodec.encode(commandValue(command)))
        aofStream.flush()
        if (aofSyncPolicy == AofSyncPolicy.ALWAYS) {
            aofStream.channel.force(true)
        }
    }

    private fun canonicalCommand(frame: CommandFrame): CommandFrame {
        if (frame.command != "EXPIRE" || frame.args.size != 2) return frame
        val seconds = frame.args[1].decodeToString().toLong()
        val absoluteSeconds = (currentTimeMs() / 1000L) + seconds
        return CommandFrame(
            "EXPIREAT",
            listOf(frame.args[0], absoluteSeconds.toString().toByteArray(StandardCharsets.UTF_8)),
        )
    }

    private fun commandValue(frame: CommandFrame): RespValue.ArrayValue =
        RespValue.ArrayValue(listOf(RespValue.BulkString(frame.command.toByteArray(StandardCharsets.UTF_8))) + frame.args.map { RespValue.BulkString(it) })

    enum class AofSyncPolicy {
        ALWAYS,
        NONE,
    }

    companion object {
        private val AOF_COMMANDS = setOf(
            "SET", "DEL", "RENAME", "INCR", "DECR", "INCRBY", "DECRBY", "APPEND",
            "HSET", "HDEL", "LPUSH", "RPUSH", "LPOP", "RPOP",
            "SADD", "SREM", "ZADD", "ZREM", "PFADD", "PFMERGE",
            "EXPIRE", "EXPIREAT", "PERSIST", "SELECT", "FLUSHDB", "FLUSHALL",
        )
    }
}
