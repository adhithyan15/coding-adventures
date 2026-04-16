package com.codingadventures.inmemorydatastore

import com.codingadventures.respprotocol.RespCodec
import com.codingadventures.respprotocol.RespValue
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

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
}
