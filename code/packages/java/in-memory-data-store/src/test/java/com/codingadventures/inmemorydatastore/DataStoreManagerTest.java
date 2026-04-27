package com.codingadventures.inmemorydatastore;

import com.codingadventures.respprotocol.RespCodec;
import com.codingadventures.respprotocol.RespValue;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
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

    @Test
    void replaysAofAcrossRestart(@TempDir Path tempDir) throws IOException {
        Path aofPath = tempDir.resolve("appendonly.aof");
        try (DataStoreManager manager = new DataStoreManager(aofPath)) {
            assertInstanceOf(RespValue.SimpleString.class, decode(manager.executeRespBytes(command("SET", "persistent", "yes"))));
            assertEquals(1L, ((RespValue.IntegerValue) decode(manager.executeRespBytes(command("INCR", "count")))).value());
            assertEquals(2L, ((RespValue.IntegerValue) decode(manager.executeRespBytes(command("INCR", "count")))).value());
            assertInstanceOf(RespValue.SimpleString.class, decode(manager.executeRespBytes(command("SELECT", "1"))));
            assertInstanceOf(RespValue.SimpleString.class, decode(manager.executeRespBytes(command("SET", "db1", "value"))));
            assertInstanceOf(RespValue.SimpleString.class, decode(manager.executeRespBytes(command("SELECT", "0"))));
            assertInstanceOf(RespValue.SimpleString.class, decode(manager.executeRespBytes(command("SET", "ttl", "value"))));
            assertEquals(1L, ((RespValue.IntegerValue) decode(manager.executeRespBytes(command("EXPIRE", "ttl", "60")))).value());
        }

        byte[] rawAof = Files.readAllBytes(aofPath);
        ArrayList<List<String>> frames = new ArrayList<>();
        int offset = 0;
        while (offset < rawAof.length) {
            RespCodec.DecodeResult decoded = RespCodec.decode(rawAof, offset);
            assertNotNull(decoded);
            frames.add(arrayStrings((RespValue.ArrayValue) decoded.value()));
            offset = decoded.nextOffset();
        }
        assertEquals(List.of("SET", "persistent", "yes"), frames.get(0));
        assertTrue(frames.contains(List.of("SELECT", "1")));
        assertTrue(frames.contains(List.of("SET", "db1", "value")));
        assertTrue(frames.stream().anyMatch(parts -> parts.size() == 3 && parts.get(0).equals("EXPIREAT") && parts.get(1).equals("ttl")));

        try (DataStoreManager manager = new DataStoreManager(aofPath)) {
            RespValue.BulkString persisted = (RespValue.BulkString) decode(manager.executeRespBytes(command("GET", "persistent")));
            assertEquals("yes", new String(persisted.value(), StandardCharsets.UTF_8));

            RespValue.BulkString count = (RespValue.BulkString) decode(manager.executeRespBytes(command("GET", "count")));
            assertEquals("2", new String(count.value(), StandardCharsets.UTF_8));

            assertInstanceOf(RespValue.SimpleString.class, decode(manager.executeRespBytes(command("SELECT", "1"))));
            RespValue.BulkString db1 = (RespValue.BulkString) decode(manager.executeRespBytes(command("GET", "db1")));
            assertEquals("value", new String(db1.value(), StandardCharsets.UTF_8));

            assertInstanceOf(RespValue.SimpleString.class, decode(manager.executeRespBytes(command("SELECT", "0"))));
            RespValue.IntegerValue ttl = (RespValue.IntegerValue) decode(manager.executeRespBytes(command("TTL", "ttl")));
            assertTrue(ttl.value() >= 0 && ttl.value() <= 60);
        }
    }

    private static byte[] command(String... parts) {
        ArrayList<RespValue> values = new ArrayList<>();
        for (String part : parts) {
            values.add(new RespValue.BulkString(part.getBytes(StandardCharsets.UTF_8)));
        }
        return RespCodec.encode(new RespValue.ArrayValue(values));
    }

    private static RespValue decode(byte[] bytes) {
        RespCodec.DecodeResult decoded = RespCodec.decode(bytes);
        assertNotNull(decoded);
        return decoded.value();
    }

    private static List<String> arrayStrings(RespValue.ArrayValue value) {
        ArrayList<String> parts = new ArrayList<>();
        for (RespValue item : value.value()) {
            parts.add(new String(((RespValue.BulkString) item).value(), StandardCharsets.UTF_8));
        }
        return parts;
    }
}
