package com.codingadventures.inmemorydatastoreprotocol

import com.codingadventures.respprotocol.RespValue
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

class InMemoryDataStoreProtocolTest {
    @Test
    fun buildsCommandFramesFromRespArrays() {
        val frame = CommandFrame.fromRespValue(
            RespValue.ArrayValue(
                listOf(
                    RespValue.BulkString("SET".encodeToByteArray()),
                    RespValue.BulkString("alpha".encodeToByteArray()),
                    RespValue.BulkString("1".encodeToByteArray()),
                ),
            ),
        )

        assertNotNull(frame)
        assertEquals("SET", frame.command)
        assertEquals(2, frame.args.size)
    }

    @Test
    fun convertsResponsesBackToRespValues() {
        assertTrue(integer(42).toRespValue() is RespValue.IntegerValue)
    }
}
