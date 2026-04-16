package com.codingadventures.inmemorydatastoreengine

import com.codingadventures.inmemorydatastoreprotocol.CommandFrame
import com.codingadventures.inmemorydatastoreprotocol.EngineResponse
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class DataStoreEngineTest {
    private fun bytes(value: String) = value.encodeToByteArray()

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
}
