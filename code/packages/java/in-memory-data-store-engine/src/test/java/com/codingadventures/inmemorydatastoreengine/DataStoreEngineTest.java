package com.codingadventures.inmemorydatastoreengine;

import com.codingadventures.inmemorydatastoreprotocol.CommandFrame;
import com.codingadventures.inmemorydatastoreprotocol.EngineResponse;
import org.junit.jupiter.api.Test;

import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class DataStoreEngineTest {
    private static byte[] bytes(String value) {
        return value.getBytes(StandardCharsets.UTF_8);
    }

    private static List<String> arrayStrings(EngineResponse.ArrayValue value) {
        ArrayList<String> decoded = new ArrayList<>();
        assertNotNull(value.value());
        for (EngineResponse item : value.value()) {
            EngineResponse.BulkString bulk = (EngineResponse.BulkString) item;
            decoded.add(new String(bulk.value(), StandardCharsets.UTF_8));
        }
        return decoded;
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

        EngineResponse.IntegerValue zadd = (EngineResponse.IntegerValue) engine.executeFrame(CommandFrame.fromParts(List.of(
            bytes("ZADD"), bytes("scores"), bytes("1"), bytes("alice"), bytes("2"), bytes("bob"), bytes("1.5"), bytes("cara")
        )));
        assertEquals(3, zadd.value());
        EngineResponse.ArrayValue zrange = (EngineResponse.ArrayValue) engine.executeFrame(CommandFrame.fromParts(List.of(
            bytes("ZRANGE"), bytes("scores"), bytes("0"), bytes("-1"), bytes("WITHSCORES")
        )));
        assertEquals(6, zrange.value().size());
        EngineResponse.IntegerValue zrank = (EngineResponse.IntegerValue) engine.executeFrame(CommandFrame.fromParts(List.of(
            bytes("ZRANK"), bytes("scores"), bytes("cara")
        )));
        assertEquals(1, zrank.value());

        EngineResponse.IntegerValue pfadd = (EngineResponse.IntegerValue) engine.executeFrame(CommandFrame.fromParts(List.of(
            bytes("PFADD"), bytes("visitors"), bytes("alice"), bytes("bob")
        )));
        assertEquals(1, pfadd.value());
        EngineResponse.IntegerValue pfcount = (EngineResponse.IntegerValue) engine.executeFrame(CommandFrame.fromParts(List.of(
            bytes("PFCOUNT"), bytes("visitors")
        )));
        assertTrue(pfcount.value() >= 2);
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

    @Test
    void handlesIndexedKeysRenameAndTtl() {
        DataStoreEngine engine = new DataStoreEngine();
        engine.executeFrame(CommandFrame.fromParts(List.of(bytes("SET"), bytes("user:1"), bytes("Ada"))));
        engine.executeFrame(CommandFrame.fromParts(List.of(bytes("SET"), bytes("user:2"), bytes("Lin"))));
        engine.executeFrame(CommandFrame.fromParts(List.of(bytes("RENAME"), bytes("user:1"), bytes("user:one"))));

        EngineResponse.ArrayValue userKeys = (EngineResponse.ArrayValue) engine.executeFrame(
            CommandFrame.fromParts(List.of(bytes("KEYS"), bytes("user:*")))
        );
        assertEquals(List.of("user:2", "user:one"), arrayStrings(userKeys));

        engine.executeFrame(CommandFrame.fromParts(List.of(bytes("EXPIRE"), bytes("user:one"), bytes("10"))));
        EngineResponse.IntegerValue ttl = (EngineResponse.IntegerValue) engine.executeFrame(
            CommandFrame.fromParts(List.of(bytes("TTL"), bytes("user:one")))
        );
        assertTrue(ttl.value() >= 0 && ttl.value() <= 10);
    }

    @Test
    void handlesSetAlgebraCommands() {
        DataStoreEngine engine = new DataStoreEngine();
        engine.executeFrame(CommandFrame.fromParts(List.of(bytes("SADD"), bytes("left"), bytes("a"), bytes("b"), bytes("c"))));
        engine.executeFrame(CommandFrame.fromParts(List.of(bytes("SADD"), bytes("right"), bytes("b"), bytes("c"), bytes("d"))));

        EngineResponse.ArrayValue union = (EngineResponse.ArrayValue) engine.executeFrame(
            CommandFrame.fromParts(List.of(bytes("SUNION"), bytes("left"), bytes("right")))
        );
        assertEquals(List.of("a", "b", "c", "d"), arrayStrings(union));

        EngineResponse.ArrayValue intersection = (EngineResponse.ArrayValue) engine.executeFrame(
            CommandFrame.fromParts(List.of(bytes("SINTER"), bytes("left"), bytes("right")))
        );
        assertEquals(List.of("b", "c"), arrayStrings(intersection));

        EngineResponse.ArrayValue difference = (EngineResponse.ArrayValue) engine.executeFrame(
            CommandFrame.fromParts(List.of(bytes("SDIFF"), bytes("left"), bytes("right")))
        );
        assertEquals(List.of("a"), arrayStrings(difference));
    }
}
