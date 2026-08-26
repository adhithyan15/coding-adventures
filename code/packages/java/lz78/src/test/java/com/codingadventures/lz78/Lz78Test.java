package com.codingadventures.lz78;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.charset.StandardCharsets;
import java.util.List;
import org.junit.jupiter.api.Test;

final class Lz78Test {
    @Test
    void matchesThePublishedTokenVectors() {
        assertEquals(List.of(), Lz78.encode(bytes("")));
        assertEquals(List.of(new Token(0, 65)), Lz78.encode(bytes("A")));
        assertEquals(
                List.of(
                        new Token(0, 65),
                        new Token(0, 66),
                        new Token(0, 67),
                        new Token(0, 68),
                        new Token(0, 69)),
                Lz78.encode(bytes("ABCDE")));
        assertEquals(
                List.of(
                        new Token(0, 65),
                        new Token(1, 65),
                        new Token(2, 65),
                        new Token(1, 0)),
                Lz78.encode(bytes("AAAAAAA")));
        assertEquals(
                List.of(
                        new Token(0, 65),
                        new Token(1, 66),
                        new Token(0, 67),
                        new Token(0, 66),
                        new Token(4, 65),
                        new Token(4, 67)),
                Lz78.encode(bytes("AABCBBABC")));
        assertEquals(
                List.of(new Token(0, 65), new Token(0, 66), new Token(1, 66), new Token(3, 0)),
                Lz78.encode(bytes("ABABAB")));
    }

    @Test
    void roundTripsTextBinaryAndDictionaryBoundaries() {
        for (byte[] input : List.of(
                bytes(""),
                bytes("ABCDE"),
                bytes("AAAAAAA"),
                bytes("ABABABAB"),
                bytes("hello world hello world"),
                new byte[] {0, 0, 0, (byte) 255, (byte) 255, 0, 1, 2, 0, 1, 2})) {
            assertArrayEquals(input, Lz78.decompress(Lz78.compress(input)));
        }

        List<Token> literals = Lz78.encode(bytes("AAAA"), 1);
        assertTrue(literals.stream().allMatch(token -> token.dictionaryIndex() == 0));
        assertArrayEquals(bytes("AAAA"), Lz78.decode(literals, 4));
        assertArrayEquals(bytes("AAAA"), Lz78.decompress(Lz78.compress(bytes("AAAA"), 1)));
    }

    @Test
    void serialisesTheExactBigEndianTeachingFormat() {
        byte[] wire = Lz78.serialize(List.of(new Token(0, 65), new Token(1, 66)), 3);
        assertArrayEquals(
                new byte[] {0, 0, 0, 3, 0, 0, 0, 2, 0, 0, 65, 0, 0, 1, 66, 0},
                wire);

        EncodedTokens decoded = Lz78.deserialize(wire);
        assertEquals(3, decoded.originalLength());
        assertEquals(List.of(new Token(0, 65), new Token(1, 66)), decoded.tokens());
        assertArrayEquals(new byte[8], Lz78.compress(new byte[0]));
    }

    @Test
    void rejectsMalformedOrNonCanonicalStreams() {
        assertThrows(IllegalArgumentException.class, () -> Lz78.encode(bytes("x"), 0));
        assertThrows(IllegalArgumentException.class, () -> Lz78.encode(bytes("x"), 65_537));
        assertThrows(IllegalArgumentException.class, () -> new Token(0, 256));
        assertThrows(IllegalArgumentException.class, () -> new EncodedTokens(List.of(), -1));
        assertThrows(IllegalArgumentException.class, () -> Lz78.decode(List.of(), -1));
        assertThrows(IllegalArgumentException.class, () -> Lz78.decode(List.of(), 1, 0));
        assertThrows(IllegalArgumentException.class, () -> Lz78.decode(List.of(), 0, -1));
        assertThrows(
                IllegalArgumentException.class,
                () -> Lz78.decode(List.of(new Token(1, 65)), 1));
        assertThrows(
                IllegalArgumentException.class,
                () -> Lz78.decode(List.of(new Token(0, 65), new Token(1, 66)), 1));
        assertThrows(IllegalArgumentException.class, () -> Lz78.serialize(List.of(), -1));
        assertThrows(
                IllegalArgumentException.class,
                () -> Lz78.serialize(List.of(new Token(65_536, 0)), 0));
        assertThrows(IllegalArgumentException.class, () -> Lz78.deserialize(new byte[7]));
        assertThrows(
                IllegalArgumentException.class,
                () -> Lz78.deserialize(new byte[] {(byte) 0x80, 0, 0, 0, 0, 0, 0, 0}));
        assertThrows(
                IllegalArgumentException.class,
                () -> Lz78.decompress(new byte[] {0x7f, -1, -1, -1, 0, 0, 0, 0}));

        byte[] truncated = {0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 65};
        assertThrows(IllegalArgumentException.class, () -> Lz78.deserialize(truncated));

        byte[] reserved = {0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 65, 1};
        assertThrows(IllegalArgumentException.class, () -> Lz78.deserialize(reserved));

        byte[] trailing = {0, 0, 0, 0, 0, 0, 0, 0, 99};
        assertThrows(IllegalArgumentException.class, () -> Lz78.deserialize(trailing));

        byte[] wrongLength = {0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 65, 0};
        assertThrows(IllegalArgumentException.class, () -> Lz78.decompress(wrongLength));
    }

    private static byte[] bytes(String value) {
        return value.getBytes(StandardCharsets.UTF_8);
    }
}
