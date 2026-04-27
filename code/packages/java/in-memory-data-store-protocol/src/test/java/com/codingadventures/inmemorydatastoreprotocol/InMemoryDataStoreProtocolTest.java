package com.codingadventures.inmemorydatastoreprotocol;

import com.codingadventures.respprotocol.RespValue;
import org.junit.jupiter.api.Test;

import java.nio.charset.StandardCharsets;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class InMemoryDataStoreProtocolTest {
    @Test
    void buildsCommandFramesFromRespArrays() {
        CommandFrame frame = CommandFrame.fromRespValue(new RespValue.ArrayValue(List.of(
                new RespValue.BulkString("SET".getBytes(StandardCharsets.UTF_8)),
                new RespValue.BulkString("alpha".getBytes(StandardCharsets.UTF_8)),
                new RespValue.BulkString("1".getBytes(StandardCharsets.UTF_8))
        )));

        assertNotNull(frame);
        assertEquals("SET", frame.command());
        assertEquals(2, frame.args().size());
    }

    @Test
    void convertsResponsesBackToRespValues() {
        RespValue value = EngineResponse.integer(42).toRespValue();
        assertInstanceOf(RespValue.IntegerValue.class, value);
        assertEquals(42, ((RespValue.IntegerValue) value).value());
    }
}
