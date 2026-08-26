package com.codingadventures.deflate;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.List;
import java.util.zip.DataFormatException;
import java.util.zip.Inflater;
import org.junit.jupiter.api.Test;

final class DeflateTest {
    private static final String DYNAMIC_INPUT =
            "4141424142414142414141414143414142454241474144414141414341424241424141414241414241414342414242444241474141434142414243424441424242414345414241424248434141424141414141434141414441414244434441414144464141414141424141414142464242424241444344434141414241414141414141414141414442414141414143484141414141414341414542414241424744424141414342434141434743434343414241414141414246414548464141414141424443414243";
    private static final String DYNAMIC_STREAM =
            "4d8dd111c0200c4267031275ff890a36bd2bfa21bc0301fae45af2a3898d7a1d2f1b2e1b566808289603421dc7a3df4a0658ca4cad1b5ec4c5340cf445a39aeac137d1f99bfb02d181b6ac6971a1cf2c7b8c7a00";

    @Test
    void emitsTheExactEmptyAndFixedVectors() {
        assertArrayEquals(hex("0300"), Deflate.compress(new byte[0]));
        assertArrayEquals(hex("7374747472720600"), Deflate.compress(bytes("AAABBC")));
    }

    @Test
    void roundTripsRepresentativeInputs() {
        byte[] allBytes = new byte[256];
        for (int index = 0; index < allBytes.length; index++) {
            allBytes[index] = (byte) index;
        }
        byte[] repetition = bytes("the quick brown fox jumps over the lazy dog ".repeat(80));

        for (byte[] input : List.of(
                new byte[0],
                bytes("A"),
                bytes("AAAAAAA"),
                bytes("AABCBBABC"),
                allBytes,
                repetition)) {
            byte[] compressed = Deflate.compress(input);
            assertArrayEquals(input, Deflate.decompress(compressed));
            assertArrayEquals(input, inflateWithJdk(compressed));
            assertEquals(1, compressed[0] & 1, "compress emits one final block");
            assertTrue(firstBlockType(compressed) == 1 || firstBlockType(compressed) == 2);
        }
        assertTrue(Deflate.compress(repetition).length < repetition.length / 4);
    }

    @Test
    void readsStoredFixedAndIndependentDynamicStreams() {
        assertArrayEquals(bytes("foo"), Deflate.inflate(hex("010300fcff666f6f")));
        assertArrayEquals(bytes("AAABBC"), Deflate.inflate(hex("7374747472720600")));
        assertArrayEquals(hex(DYNAMIC_INPUT), Deflate.inflate(hex(DYNAMIC_STREAM)));
    }

    @Test
    void selectsDynamicCodingWhenItWins() {
        byte[] input = hex(DYNAMIC_INPUT);
        byte[] compressed = Deflate.compress(input);
        assertEquals(2, firstBlockType(compressed));
        assertTrue(compressed.length < input.length);
        assertArrayEquals(input, inflateWithJdk(compressed));
    }

    @Test
    void usesTheExactCandidateBitCostsForTheBlockDecision() {
        byte[] input = hex(DYNAMIC_INPUT);
        long[] costs = Deflate.candidateBitCosts(input);
        assertTrue(costs[1] < costs[0]);
        byte[] compressed = Deflate.compress(input);
        assertEquals(2, firstBlockType(compressed));
        assertEquals((costs[1] + 7) / 8, compressed.length);
    }

    @Test
    void rejectsTruncationTrailingBytesInvalidBlocksAndBombs() {
        byte[] compressed = Deflate.compress(bytes("hello hello hello hello"));
        assertThrows(
                IllegalArgumentException.class,
                () -> Deflate.inflate(Arrays.copyOf(compressed, compressed.length - 1)));

        byte[] trailing = Arrays.copyOf(compressed, compressed.length + 1);
        assertThrows(IllegalArgumentException.class, () -> Deflate.inflate(trailing));
        assertThrows(IllegalArgumentException.class, () -> Deflate.inflate(hex("07")));

        byte[] bomb = Deflate.compress(bytes("A".repeat(1_000)));
        assertThrows(IllegalArgumentException.class, () -> Deflate.inflate(bomb, 999));
        assertThrows(IllegalArgumentException.class, () -> Deflate.inflate(bomb, -1));
    }

    private static int firstBlockType(byte[] compressed) {
        return (compressed[0] >>> 1) & 0x03;
    }

    private static byte[] inflateWithJdk(byte[] compressed) {
        Inflater inflater = new Inflater(true);
        inflater.setInput(compressed);
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        byte[] buffer = new byte[256];
        try {
            while (!inflater.finished()) {
                int count = inflater.inflate(buffer);
                if (count == 0 && !inflater.finished()
                        && (inflater.needsInput() || inflater.needsDictionary())) {
                    throw new IllegalArgumentException("independent inflater stalled");
                }
                output.write(buffer, 0, count);
            }
            return output.toByteArray();
        } catch (DataFormatException exception) {
            throw new IllegalArgumentException("independent inflater rejected stream", exception);
        } finally {
            inflater.end();
        }
    }

    private static byte[] bytes(String value) {
        return value.getBytes(StandardCharsets.UTF_8);
    }

    private static byte[] hex(String value) {
        return java.util.HexFormat.of().parseHex(value);
    }
}
