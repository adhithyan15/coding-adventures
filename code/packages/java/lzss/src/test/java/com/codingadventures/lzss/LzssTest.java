package com.codingadventures.lzss;

import org.junit.jupiter.api.Test;

import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for the LZSS CMP02 implementation.
 *
 * <p>The tests are organised into five sections:</p>
 * <ol>
 *   <li>Round-trip tests — compress then decompress must reproduce the input.</li>
 *   <li>Encoder structural tests — verify token shapes for known inputs.</li>
 *   <li>Decoder tests — verify decoding from a hand-crafted token list.</li>
 *   <li>Compression effectiveness — repetitive data must shrink.</li>
 *   <li>Wire-format / known-vector tests — check header bytes and exact output.</li>
 * </ol>
 *
 * <p>Each test is self-contained and labelled so that test-report output is
 * human-readable.  JUnit 5's {@code @Test} annotation discovers them
 * automatically; no runner configuration is needed beyond
 * {@code useJUnitPlatform()} in {@code build.gradle.kts}.</p>
 */
class LzssTest {

    // ─── Helpers ─────────────────────────────────────────────────────────────

    /** Convenience: compress then decompress. */
    private static byte[] roundTrip(byte[] data) {
        return Lzss.decompress(Lzss.compress(data));
    }

    /** Convenience: UTF-8 string → bytes. */
    private static byte[] utf8(String s) {
        return s.getBytes(StandardCharsets.UTF_8);
    }

    // ─── 1. Round-trip tests ─────────────────────────────────────────────────

    /** Test 1: empty input compresses and decompresses to empty output. */
    @Test
    void roundTripEmpty() {
        assertArrayEquals(new byte[0], roundTrip(new byte[0]));
    }

    /** Test 2: a single byte survives the round trip. */
    @Test
    void roundTripSingleByte() {
        assertArrayEquals(new byte[]{0x42}, roundTrip(new byte[]{0x42}));
    }

    /**
     * Test 3: a highly repetitive string round-trips correctly.
     *
     * <p>"AAAAAAAA" is the canonical LZSS demo: the first byte is a Literal,
     * everything else is a single back-reference Match(offset=1, length=7).</p>
     */
    @Test
    void roundTripRepetitiveText() {
        byte[] data = utf8("AAAAAAAA");
        assertArrayEquals(data, roundTrip(data));
    }

    /**
     * Test 4: all 256 possible byte values survive compression intact.
     *
     * <p>This ensures no byte value is treated specially (e.g., as a sentinel
     * or escape code) and that the serialiser handles values 0x00–0xFF.</p>
     */
    @Test
    void roundTripAll256ByteValues() {
        byte[] data = new byte[256];
        for (int i = 0; i < 256; i++) {
            data[i] = (byte) i;
        }
        assertArrayEquals(data, roundTrip(data));
    }

    /**
     * Test 5: a 1 KB pseudo-random-ish byte sequence round-trips.
     *
     * <p>We use a simple linear-congruential pattern to generate data that
     * has some repeats but is not trivially compressible, exercising both
     * Literal and Match tokens.</p>
     */
    @Test
    void roundTripOneKilobyteData() {
        byte[] data = new byte[1024];
        int v = 7;
        for (int i = 0; i < data.length; i++) {
            v = (v * 31 + 17) & 0xFF;
            data[i] = (byte) v;
        }
        assertArrayEquals(data, roundTrip(data));
    }

    // ─── 2. Encoder structural tests ─────────────────────────────────────────

    /**
     * Test 6: non-repetitive data should produce only Literal tokens.
     *
     * <p>"ABCDE" has no repeated substrings, so every byte must be emitted
     * as a Literal.</p>
     */
    @Test
    void encodeReturnsLiteralsForNonRepetitiveData() {
        List<LzssToken> tokens = Lzss.encode(
                utf8("ABCDE"),
                Lzss.DEFAULT_WINDOW_SIZE,
                Lzss.DEFAULT_MAX_MATCH,
                Lzss.DEFAULT_MIN_MATCH);

        assertEquals(5, tokens.size(), "expecting one token per byte");
        assertTrue(tokens.stream().allMatch(t -> t instanceof LzssToken.Literal),
                "all tokens must be Literals for non-repetitive input");
    }

    /**
     * Test 7: repeated data should produce at least one Match token.
     *
     * <p>"ABABABABAB" has obvious 2-byte periodicity; after the first two
     * Literals, the encoder must emit a Match back-reference.</p>
     */
    @Test
    void encodeReturnsMatchForRepeatedData() {
        List<LzssToken> tokens = Lzss.encode(
                utf8("ABABABABAB"),
                Lzss.DEFAULT_WINDOW_SIZE,
                Lzss.DEFAULT_MAX_MATCH,
                Lzss.DEFAULT_MIN_MATCH);

        assertTrue(tokens.stream().anyMatch(t -> t instanceof LzssToken.Match),
                "repeated input must produce at least one Match token");
    }

    /**
     * Test 8 (encode structure): encoding "ABABAB" produces exactly the
     * expected token sequence: Literal('A'), Literal('B'), Match(2, 4).
     *
     * <p>After 'A' and 'B' are in the window, "ABAB" at position 2 is a
     * 4-byte match starting 2 bytes back.</p>
     */
    @Test
    void encodeAbababProducesKnownTokens() {
        List<LzssToken> tokens = Lzss.encode(
                utf8("ABABAB"),
                Lzss.DEFAULT_WINDOW_SIZE,
                Lzss.DEFAULT_MAX_MATCH,
                Lzss.DEFAULT_MIN_MATCH);

        assertEquals(3, tokens.size());
        assertEquals(new LzssToken.Literal((byte) 'A'), tokens.get(0));
        assertEquals(new LzssToken.Literal((byte) 'B'), tokens.get(1));
        assertEquals(new LzssToken.Match(2, 4),          tokens.get(2));
    }

    /**
     * Test 9 (encode structure): all-identical input collapses to one Literal
     * followed by one long Match.
     *
     * <p>"AAAAAAA" (7 bytes): first 'A' is a Literal, then the remaining 6
     * bytes are a Match(offset=1, length=6) — an overlapping match that
     * expands the single 'A' like a run-length code.</p>
     */
    @Test
    void encodeAllIdenticalProducesOneLiteralAndOneMatch() {
        List<LzssToken> tokens = Lzss.encode(
                utf8("AAAAAAA"),
                Lzss.DEFAULT_WINDOW_SIZE,
                Lzss.DEFAULT_MAX_MATCH,
                Lzss.DEFAULT_MIN_MATCH);

        assertEquals(2, tokens.size());
        assertEquals(new LzssToken.Literal((byte) 'A'), tokens.get(0));
        assertEquals(new LzssToken.Match(1, 6),          tokens.get(1));
    }

    // ─── 3. Decoder tests ────────────────────────────────────────────────────

    /**
     * Test 10: decode reconstructs correctly from a known hand-crafted token list.
     *
     * <p>Token list: Literal('A'), Literal('B'), Match(offset=2, length=4)
     * should decode to "ABABAB" (6 bytes).</p>
     *
     * <p>Trace:
     * <pre>
     *   output = []
     *   Literal 'A' → output = [A]
     *   Literal 'B' → output = [A, B]
     *   Match(2,4):  start = 2-2 = 0
     *     copy output[0] = A → [A, B, A]
     *     copy output[1] = B → [A, B, A, B]
     *     copy output[2] = A → [A, B, A, B, A]
     *     copy output[3] = B → [A, B, A, B, A, B]
     * </pre>
     * </p>
     */
    @Test
    void decodeReconstructsFromKnownTokenList() {
        List<LzssToken> tokens = List.of(
                new LzssToken.Literal((byte) 'A'),
                new LzssToken.Literal((byte) 'B'),
                new LzssToken.Match(2, 4)
        );
        byte[] expected = utf8("ABABAB");
        assertArrayEquals(expected, Lzss.decode(tokens, expected.length));
    }

    // ─── 4. Compression effectiveness ────────────────────────────────────────

    /**
     * Test 11: compressed size is smaller than original for repetitive data.
     *
     * <p>3000 bytes of cycling "ABC" pattern should compress substantially.
     * If the compressed output is larger, either the algorithm is broken or
     * there is a bug in serialisation.</p>
     */
    @Test
    void compressedSizeSmallerForRepetitiveData() {
        byte[] base = utf8("ABC");
        byte[] data = new byte[3000];
        for (int i = 0; i < data.length; i++) {
            data[i] = base[i % base.length];
        }
        byte[] compressed = Lzss.compress(data);
        assertTrue(compressed.length < data.length,
                "compressed size (" + compressed.length + ") should be < original ("
                        + data.length + ")");
    }

    // ─── 5. Wire-format / known-vector tests ─────────────────────────────────

    /**
     * Test 12 (known vector): compress("hello") stores original_length = 5 in
     * the first 4 bytes of the wire format.
     *
     * <p>The header is always at a fixed position and in big-endian order,
     * so this is a strong sanity-check for the serialiser.</p>
     */
    @Test
    void compressStoresOriginalLengthInHeader() {
        byte[] compressed = Lzss.compress(utf8("hello"));
        // Bytes 0–3: original_length as big-endian uint32.
        int storedLength = ((compressed[0] & 0xFF) << 24)
                         | ((compressed[1] & 0xFF) << 16)
                         | ((compressed[2] & 0xFF) << 8)
                         |  (compressed[3] & 0xFF);
        assertEquals(5, storedLength,
                "header original_length should be 5 for input \"hello\"");
    }

    /**
     * Test 13: a crafted compressed buffer with a huge block_count in the
     * header must not cause an infinite loop or OOM (DoS resilience test).
     *
     * <p>We write block_count = 2^30 into bytes 4–7 but provide only 16 bytes
     * of total data.  The deserialiser must cap block_count at
     * (data.length - 8) = 8 and return without panic.</p>
     */
    @Test
    void craftedLargeBlockCountIsSafe() {
        byte[] bad = new byte[16];
        // original_length = 0
        bad[0] = bad[1] = bad[2] = bad[3] = 0;
        // block_count = 0x40000000 = 2^30
        bad[4] = 0x40;
        bad[5] = bad[6] = bad[7] = 0;
        // rest are zeros — will be decoded as Literal(0) tokens
        assertDoesNotThrow(() -> Lzss.decompress(bad),
                "decompress must not throw on a crafted over-sized block_count");
    }

    /**
     * Test 14: Unicode text (Japanese "Hello World") survives the round trip
     * as UTF-8 bytes.
     *
     * <p>Multi-byte UTF-8 sequences are just byte sequences to LZSS; this
     * test confirms no ASCII-only assumption is hiding in the code.</p>
     */
    @Test
    void roundTripUnicodeText() {
        String text = "こんにちは世界"; // "Hello World" in Japanese
        byte[] data = utf8(text);
        assertArrayEquals(data, roundTrip(data));
    }

    /**
     * Test 15: a large (10 KB) input round-trips correctly.
     *
     * <p>This exercises the multi-block serialiser path (each block holds 8
     * tokens; 10 KB of mixed data will produce hundreds of blocks) and checks
     * that the header's block_count field is written and read correctly.</p>
     */
    @Test
    void roundTripLargeInput() {
        // Generate 10 KB of data with a 7-byte cycling pattern so that
        // both Literal and Match tokens appear in the compressed stream.
        byte[] pattern = utf8("ABCDEFG");
        byte[] data = new byte[10 * 1024];
        for (int i = 0; i < data.length; i++) {
            data[i] = pattern[i % pattern.length];
        }
        assertArrayEquals(data, roundTrip(data),
                "10 KB round-trip must reproduce the original data exactly");
    }
}
