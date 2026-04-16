package com.codingadventures.inmemorydatastore;

import com.codingadventures.respprotocol.RespCodec;
import com.codingadventures.respprotocol.RespValue;
import org.junit.jupiter.api.Test;

import java.nio.charset.StandardCharsets;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class DataStoreManagerTest {
    @Test
    void executesRespCommandsEndToEnd() {
        DataStoreManager manager = new DataStoreManager();
        byte[] setResponse = manager.executeRespBytes(RespCodec.encode(new RespValue.ArrayValue(List.of(
                new RespValue.BulkString("SET".getBytes(StandardCharsets.UTF_8)),
                new RespValue.BulkString("alpha".getBytes(StandardCharsets.UTF_8)),
                new RespValue.BulkString("1".getBytes(StandardCharsets.UTF_8))
        ))));
        assertInstanceOf(RespValue.SimpleString.class, RespCodec.decode(setResponse).value());

        byte[] getResponse = manager.executeRespBytes(RespCodec.encode(new RespValue.ArrayValue(List.of(
                new RespValue.BulkString("GET".getBytes(StandardCharsets.UTF_8)),
                new RespValue.BulkString("alpha".getBytes(StandardCharsets.UTF_8))
        ))));
        RespValue.BulkString bulk = (RespValue.BulkString) RespCodec.decode(getResponse).value();
        assertEquals("1", new String(bulk.value(), StandardCharsets.UTF_8));
    }
}
