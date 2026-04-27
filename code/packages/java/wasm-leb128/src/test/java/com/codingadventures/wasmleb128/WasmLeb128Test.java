package com.codingadventures.wasmleb128;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

class WasmLeb128Test {
    @Test
    void decodesUnsignedValues() {
        WasmLeb128.UnsignedDecoding decoding = WasmLeb128.decodeUnsigned(
                new byte[]{(byte) 0xE5, (byte) 0x8E, 0x26}
        );

        assertEquals(624485L, decoding.value());
        assertEquals(3, decoding.bytesConsumed());
    }

    @Test
    void decodesSignedValues() {
        WasmLeb128.SignedDecoding decoding = WasmLeb128.decodeSigned(new byte[]{0x7E});

        assertEquals(-2, decoding.value());
        assertEquals(1, decoding.bytesConsumed());
    }

    @Test
    void encodesUnsignedAndSignedValues() {
        assertArrayEquals(new byte[]{(byte) 0xE5, (byte) 0x8E, 0x26}, WasmLeb128.encodeUnsigned(624485));
        assertArrayEquals(new byte[]{0x7E}, WasmLeb128.encodeSigned(-2));
    }

    @Test
    void rejectsUnterminatedSequence() {
        assertThrows(
                WasmLeb128.LEB128Error.class,
                () -> WasmLeb128.decodeUnsigned(new byte[]{(byte) 0x80, (byte) 0x80})
        );
    }
}
