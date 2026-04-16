package com.codingadventures.respprotocol

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull

class RespProtocolTest {
    @Test
    fun encodesAndDecodesArrayCommands() {
        val encoded = RespCodec.encode(
            RespValue.ArrayValue(
                listOf(
                    RespValue.BulkString("PING".encodeToByteArray()),
                    RespValue.BulkString("hello".encodeToByteArray()),
                ),
            ),
        )
        val decoded = RespCodec.decode(encoded)
        assertNotNull(decoded)
        assertEquals(encoded.size, decoded.nextOffset)
    }

    @Test
    fun returnsNullForIncompleteFrames() {
        assertNull(RespCodec.decode("$5\r\nhel".encodeToByteArray()))
    }
}
