package com.codingadventures.inmemorydatastoreengine

import com.codingadventures.inmemorydatastoreprotocol.CommandFrame
import com.codingadventures.inmemorydatastoreprotocol.EngineResponse
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class DataStoreEngineTest {
    private fun bytes(value: String) = value.encodeToByteArray()
    private fun arrayStrings(value: EngineResponse.ArrayValue): List<String> = value.value!!.map {
        (it as EngineResponse.BulkString).value!!.decodeToString()
    }

    @Test
    fun handlesStringRoundTripsAndIntegers() {
        val engine = DataStoreEngine()
        engine.executeFrame(CommandFrame.fromParts(listOf(bytes("SET"), bytes("alpha"), bytes("1"))))
        val get = engine.executeFrame(CommandFrame.fromParts(listOf(bytes("GET"), bytes("alpha")))) as EngineResponse.BulkString
        assertEquals("1", get.value?.decodeToString())
        val incr = engine.executeFrame(CommandFrame.fromParts(listOf(bytes("INCR"), bytes("alpha")))) as EngineResponse.IntegerValue
        assertEquals(2, incr.value)
    }

    @Test
    fun handlesHashListAndSetCommands() {
        val engine = DataStoreEngine()
        val hset = engine.executeFrame(CommandFrame.fromParts(listOf(bytes("HSET"), bytes("user:1"), bytes("name"), bytes("Ada")))) as EngineResponse.IntegerValue
        assertEquals(1, hset.value)
        val hget = engine.executeFrame(CommandFrame.fromParts(listOf(bytes("HGET"), bytes("user:1"), bytes("name")))) as EngineResponse.BulkString
        assertEquals("Ada", hget.value?.decodeToString())

        engine.executeFrame(CommandFrame.fromParts(listOf(bytes("LPUSH"), bytes("queue"), bytes("b"), bytes("a"))))
        val range = engine.executeFrame(CommandFrame.fromParts(listOf(bytes("LRANGE"), bytes("queue"), bytes("0"), bytes("-1")))) as EngineResponse.ArrayValue
        assertEquals(2, range.value?.size)

        engine.executeFrame(CommandFrame.fromParts(listOf(bytes("SADD"), bytes("tags"), bytes("red"), bytes("blue"))))
        val scard = engine.executeFrame(CommandFrame.fromParts(listOf(bytes("SCARD"), bytes("tags")))) as EngineResponse.IntegerValue
        assertEquals(2, scard.value)

        val zadd = engine.executeFrame(
            CommandFrame.fromParts(listOf(bytes("ZADD"), bytes("scores"), bytes("1"), bytes("alice"), bytes("2"), bytes("bob"), bytes("1.5"), bytes("cara"))),
        ) as EngineResponse.IntegerValue
        assertEquals(3, zadd.value)
        val zrange = engine.executeFrame(
            CommandFrame.fromParts(listOf(bytes("ZRANGE"), bytes("scores"), bytes("0"), bytes("-1"), bytes("WITHSCORES"))),
        ) as EngineResponse.ArrayValue
        assertEquals(6, zrange.value?.size)
        val zrank = engine.executeFrame(
            CommandFrame.fromParts(listOf(bytes("ZRANK"), bytes("scores"), bytes("cara"))),
        ) as EngineResponse.IntegerValue
        assertEquals(1, zrank.value)

        val pfadd = engine.executeFrame(
            CommandFrame.fromParts(listOf(bytes("PFADD"), bytes("visitors"), bytes("alice"), bytes("bob"))),
        ) as EngineResponse.IntegerValue
        assertEquals(1, pfadd.value)
        val pfcount = engine.executeFrame(
            CommandFrame.fromParts(listOf(bytes("PFCOUNT"), bytes("visitors"))),
        ) as EngineResponse.IntegerValue
        assertTrue(pfcount.value >= 2)
    }

    @Test
    fun handlesExpiryAndDatabaseSelection() {
        val engine = DataStoreEngine()
        engine.executeFrame(CommandFrame.fromParts(listOf(bytes("SET"), bytes("ttl"), bytes("value"))))
        val expired = (DataStoreEngine.currentTimeMs() / 1000L) - 1L
        engine.executeFrame(CommandFrame.fromParts(listOf(bytes("EXPIREAT"), bytes("ttl"), bytes(expired.toString()))))
        val value = engine.executeFrame(CommandFrame.fromParts(listOf(bytes("GET"), bytes("ttl")))) as EngineResponse.BulkString
        assertNull(value.value)

        engine.executeFrame(CommandFrame.fromParts(listOf(bytes("SELECT"), bytes("1"))))
        engine.executeFrame(CommandFrame.fromParts(listOf(bytes("SET"), bytes("alpha"), bytes("db1"))))
        val dbsize = engine.executeFrame(CommandFrame.fromParts(listOf(bytes("DBSIZE")))) as EngineResponse.IntegerValue
        assertEquals(1, dbsize.value)
    }

    @Test
    fun handlesIndexedKeysRenameAndTtl() {
        val engine = DataStoreEngine()
        engine.executeFrame(CommandFrame.fromParts(listOf(bytes("SET"), bytes("user:1"), bytes("Ada"))))
        engine.executeFrame(CommandFrame.fromParts(listOf(bytes("SET"), bytes("user:2"), bytes("Lin"))))
        engine.executeFrame(CommandFrame.fromParts(listOf(bytes("RENAME"), bytes("user:1"), bytes("user:one"))))

        val userKeys = engine.executeFrame(
            CommandFrame.fromParts(listOf(bytes("KEYS"), bytes("user:*"))),
        ) as EngineResponse.ArrayValue
        assertEquals(listOf("user:2", "user:one"), arrayStrings(userKeys))

        engine.executeFrame(CommandFrame.fromParts(listOf(bytes("EXPIRE"), bytes("user:one"), bytes("10"))))
        val ttl = engine.executeFrame(
            CommandFrame.fromParts(listOf(bytes("TTL"), bytes("user:one"))),
        ) as EngineResponse.IntegerValue
        assertTrue(ttl.value in 0..10)
    }

    @Test
    fun handlesSetAlgebraCommands() {
        val engine = DataStoreEngine()
        engine.executeFrame(CommandFrame.fromParts(listOf(bytes("SADD"), bytes("left"), bytes("a"), bytes("b"), bytes("c"))))
        engine.executeFrame(CommandFrame.fromParts(listOf(bytes("SADD"), bytes("right"), bytes("b"), bytes("c"), bytes("d"))))

        val union = engine.executeFrame(
            CommandFrame.fromParts(listOf(bytes("SUNION"), bytes("left"), bytes("right"))),
        ) as EngineResponse.ArrayValue
        assertEquals(listOf("a", "b", "c", "d"), arrayStrings(union))

        val intersection = engine.executeFrame(
            CommandFrame.fromParts(listOf(bytes("SINTER"), bytes("left"), bytes("right"))),
        ) as EngineResponse.ArrayValue
        assertEquals(listOf("b", "c"), arrayStrings(intersection))

        val difference = engine.executeFrame(
            CommandFrame.fromParts(listOf(bytes("SDIFF"), bytes("left"), bytes("right"))),
        ) as EngineResponse.ArrayValue
        assertEquals(listOf("a"), arrayStrings(difference))
    }
}
