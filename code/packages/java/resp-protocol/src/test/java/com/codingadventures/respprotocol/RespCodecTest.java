package com.codingadventures.respprotocol;

import org.junit.jupiter.api.Test;

import java.nio.charset.StandardCharsets;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class RespCodecTest {
    @Test
    void encodesAndDecodesCommandArrays() {
        RespValue.ArrayValue command = new RespValue.ArrayValue(List.of(
                new RespValue.BulkString("PING".getBytes(StandardCharsets.UTF_8)),
                new RespValue.BulkString("hello".getBytes(StandardCharsets.UTF_8))
        ));

        byte[] encoded = RespCodec.encode(command);
        RespCodec.DecodeResult decoded = RespCodec.decode(encoded);

        assertNotNull(decoded);
        assertEquals(encoded.length, decoded.nextOffset());
        RespValue.ArrayValue array = (RespValue.ArrayValue) decoded.value();
        assertEquals(2, array.value().size());
    }

    @Test
    void returnsNullForIncompleteFrames() {
        assertNull(RespCodec.decode("$5\r\nhel".getBytes(StandardCharsets.UTF_8)));
    }
}
