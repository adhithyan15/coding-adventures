package com.codingadventures.inmemorydatastoreengine;

import com.codingadventures.inmemorydatastoreprotocol.CommandFrame;
import com.codingadventures.inmemorydatastoreprotocol.EngineResponse;
import org.junit.jupiter.api.Test;

import java.nio.charset.StandardCharsets;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class DataStoreEngineTest {
    private static byte[] bytes(String value) {
        return value.getBytes(StandardCharsets.UTF_8);
    }

    @Test
    void handlesStringRoundTripsAndIntegers() {
        DataStoreEngine engine = new DataStoreEngine();
        assertInstanceOf(EngineResponse.SimpleString.class, engine.executeFrame(CommandFrame.fromParts(List.of(bytes("SET"), bytes("alpha"), bytes("1")))));
        EngineResponse.BulkString get = (EngineResponse.BulkString) engine.executeFrame(CommandFrame.fromParts(List.of(bytes("GET"), bytes("alpha"))));
        assertEquals("1", new String(get.value(), StandardCharsets.UTF_8));
        EngineResponse.IntegerValue incr = (EngineResponse.IntegerValue) engine.executeFrame(CommandFrame.fromParts(List.of(bytes("INCR"), bytes("alpha"))));
        assertEquals(2, incr.value());
    }

    @Test
    void handlesHashListAndSetCommands() {
        DataStoreEngine engine = new DataStoreEngine();
        EngineResponse.IntegerValue hset = (EngineResponse.IntegerValue) engine.executeFrame(CommandFrame.fromParts(List.of(bytes("HSET"), bytes("user:1"), bytes("name"), bytes("Ada"))));
        assertEquals(1, hset.value());
        EngineResponse.BulkString hget = (EngineResponse.BulkString) engine.executeFrame(CommandFrame.fromParts(List.of(bytes("HGET"), bytes("user:1"), bytes("name"))));
        assertEquals("Ada", new String(hget.value(), StandardCharsets.UTF_8));

        engine.executeFrame(CommandFrame.fromParts(List.of(bytes("LPUSH"), bytes("queue"), bytes("b"), bytes("a"))));
        EngineResponse.ArrayValue range = (EngineResponse.ArrayValue) engine.executeFrame(CommandFrame.fromParts(List.of(bytes("LRANGE"), bytes("queue"), bytes("0"), bytes("-1"))));
        assertEquals(2, range.value().size());

        engine.executeFrame(CommandFrame.fromParts(List.of(bytes("SADD"), bytes("tags"), bytes("red"), bytes("blue"))));
        EngineResponse.IntegerValue scard = (EngineResponse.IntegerValue) engine.executeFrame(CommandFrame.fromParts(List.of(bytes("SCARD"), bytes("tags"))));
        assertEquals(2, scard.value());
    }

    @Test
    void handlesExpiryAndDatabaseSelection() {
        DataStoreEngine engine = new DataStoreEngine();
        engine.executeFrame(CommandFrame.fromParts(List.of(bytes("SET"), bytes("ttl"), bytes("value"))));
        long expired = (DataStoreEngine.currentTimeMs() / 1000L) - 1L;
        engine.executeFrame(CommandFrame.fromParts(List.of(bytes("EXPIREAT"), bytes("ttl"), bytes(Long.toString(expired)))));
        EngineResponse.BulkString value = (EngineResponse.BulkString) engine.executeFrame(CommandFrame.fromParts(List.of(bytes("GET"), bytes("ttl"))));
        assertNull(value.value());

        engine.executeFrame(CommandFrame.fromParts(List.of(bytes("SELECT"), bytes("1"))));
        engine.executeFrame(CommandFrame.fromParts(List.of(bytes("SET"), bytes("alpha"), bytes("db1"))));
        EngineResponse.IntegerValue dbsize = (EngineResponse.IntegerValue) engine.executeFrame(CommandFrame.fromParts(List.of(bytes("DBSIZE"))));
        assertEquals(1, dbsize.value());
    }
}
