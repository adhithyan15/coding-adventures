package com.codingadventures.inmemorydatastore

import com.codingadventures.respprotocol.RespCodec
import com.codingadventures.respprotocol.RespValue
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlin.test.assertNotNull
import java.nio.file.Files
import kotlin.io.path.createTempDirectory

class DataStoreManagerTest {
    @Test
    fun executesRespCommandsEndToEnd() {
        val manager = DataStoreManager()
        manager.executeRespBytes(
            RespCodec.encode(
                RespValue.ArrayValue(
                    listOf(
                        RespValue.BulkString("SET".encodeToByteArray()),
                        RespValue.BulkString("alpha".encodeToByteArray()),
                        RespValue.BulkString("1".encodeToByteArray()),
                    ),
                ),
            ),
        )
        val decoded = RespCodec.decode(
            manager.executeRespBytes(
                RespCodec.encode(
                    RespValue.ArrayValue(
                        listOf(
                            RespValue.BulkString("GET".encodeToByteArray()),
                            RespValue.BulkString("alpha".encodeToByteArray()),
                        ),
                    ),
                ),
            ),
        )
        val bulk = decoded?.value as RespValue.BulkString
        assertEquals("1", bulk.value?.decodeToString())
    }

    @Test
    fun replaysAofAcrossRestart() {
        val tempDir = createTempDirectory("aof-replay")
        val aofPath = tempDir.resolve("appendonly.aof")
        DataStoreManager(aofPath).use { manager ->
            manager.executeRespBytes(command("SET", "persistent", "yes"))
            assertEquals(1, (decode(manager.executeRespBytes(command("INCR", "count"))) as RespValue.IntegerValue).value)
            assertEquals(2, (decode(manager.executeRespBytes(command("INCR", "count"))) as RespValue.IntegerValue).value)
            manager.executeRespBytes(command("SELECT", "1"))
            manager.executeRespBytes(command("SET", "db1", "value"))
            manager.executeRespBytes(command("SELECT", "0"))
            manager.executeRespBytes(command("SET", "ttl", "value"))
            assertEquals(1, (decode(manager.executeRespBytes(command("EXPIRE", "ttl", "60"))) as RespValue.IntegerValue).value)
        }

        val rawAof = Files.readAllBytes(aofPath)
        val frames = mutableListOf<List<String>>()
        var offset = 0
        while (offset < rawAof.size) {
            val decoded = RespCodec.decode(rawAof, offset)
            assertNotNull(decoded)
            frames += arrayStrings(decoded.value as RespValue.ArrayValue)
            offset = decoded.nextOffset
        }
        assertEquals(listOf("SET", "persistent", "yes"), frames.first())
        assertTrue(frames.contains(listOf("SELECT", "1")))
        assertTrue(frames.contains(listOf("SET", "db1", "value")))
        assertTrue(frames.any { it.size == 3 && it[0] == "EXPIREAT" && it[1] == "ttl" })

        DataStoreManager(aofPath).use { manager ->
            val persisted = decode(manager.executeRespBytes(command("GET", "persistent"))) as RespValue.BulkString
            assertEquals("yes", persisted.value?.decodeToString())

            val count = decode(manager.executeRespBytes(command("GET", "count"))) as RespValue.BulkString
            assertEquals("2", count.value?.decodeToString())

            manager.executeRespBytes(command("SELECT", "1"))
            val db1 = decode(manager.executeRespBytes(command("GET", "db1"))) as RespValue.BulkString
            assertEquals("value", db1.value?.decodeToString())

            manager.executeRespBytes(command("SELECT", "0"))
            val ttl = decode(manager.executeRespBytes(command("TTL", "ttl"))) as RespValue.IntegerValue
            assertTrue(ttl.value in 0..60)
        }
    }

    private fun command(vararg parts: String): ByteArray =
        RespCodec.encode(RespValue.ArrayValue(parts.map { RespValue.BulkString(it.encodeToByteArray()) }))

    private fun decode(bytes: ByteArray): RespValue {
        val decoded = RespCodec.decode(bytes)
        assertNotNull(decoded)
        return decoded.value
    }

    private fun arrayStrings(value: RespValue.ArrayValue): List<String> =
        value.value!!.map { (it as RespValue.BulkString).value!!.decodeToString() }
}
