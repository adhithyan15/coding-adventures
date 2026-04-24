/**
 * Unit tests for the Kotlin LZSS (CMP02) package.
 *
 * Test strategy mirrors the Rust reference implementation (lib.rs) and covers:
 *
 * 1. Round-trip correctness (encode → decode reproduces the original bytes)
 * 2. Token-stream properties (Literal / Match tokens are what we expect)
 * 3. Compression effectiveness (repetitive data gets smaller)
 * 4. Known-vector checks (specific token sequences produce specific bytes)
 * 5. Edge cases (empty, single byte, binary data, Unicode, large input)
 * 6. Wire-format integrity (header stores original_length, deterministic output)
 *
 * Each test is self-contained: it constructs its own input, calls Lzss, and
 * asserts against expected output.  No shared mutable state between tests.
 */
package com.codingadventures.lzss

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlin.test.assertFalse

class LzssTest {

    // ── Helpers ───────────────────────────────────────────────────────────────

    /**
     * Convenience: compress then decompress, returning the final ByteArray.
     *
     * This is the key round-trip check — if `rt(data)` equals `data`, the
     * implementation is lossless for that input.
     */
    private fun rt(data: ByteArray): ByteArray = Lzss.decompress(Lzss.compress(data))

    // ── 1. Round-trip: empty ──────────────────────────────────────────────────

    /**
     * Test 1: Round-trip empty byte array.
     *
     * The compressed form of an empty input should decompress back to empty.
     * This exercises the "zero tokens" code path and header parsing.
     */
    @Test
    fun roundTripEmpty() {
        val data = ByteArray(0)
        assertTrue(rt(data).isEmpty(), "Empty data should round-trip to empty")
    }

    // ── 2. Round-trip: single byte ────────────────────────────────────────────

    /**
     * Test 2: Round-trip a single byte.
     *
     * A single byte cannot match anything (the search window is empty), so it
     * must be emitted as a Literal.  This exercises the trivial Literal path.
     */
    @Test
    fun roundTripSingleByte() {
        val data = byteArrayOf(0x42)
        assertTrue(data.contentEquals(rt(data)), "Single byte should round-trip unchanged")
    }

    // ── 3. Round-trip: repetitive text ───────────────────────────────────────

    /**
     * Test 3: Round-trip a highly repetitive ASCII string.
     *
     * "banana" repeated 50 times gives the encoder plenty of back-references
     * to find.  Verifies that Match tokens are correctly serialised and
     * reconstructed.
     */
    @Test
    fun roundTripRepetitiveText() {
        val data = "banana".repeat(50).encodeToByteArray()
        assertTrue(data.contentEquals(rt(data)), "Repetitive text should round-trip unchanged")
    }

    // ── 4. Round-trip: all 256 byte values ───────────────────────────────────

    /**
     * Test 4: Round-trip a sequence containing all 256 possible byte values.
     *
     * Ensures no byte value is mishandled due to sign-extension bugs or
     * off-by-one errors in the bit-packing routines.  This is particularly
     * important for bytes in 0x80..0xFF which are negative in signed Kotlin Bytes.
     */
    @Test
    fun roundTripAll256Bytes() {
        val data = ByteArray(256) { it.toByte() }
        assertTrue(data.contentEquals(rt(data)), "All 256 byte values should round-trip unchanged")
    }

    // ── 5. Round-trip: 1 KB of structured data ───────────────────────────────

    /**
     * Test 5: Round-trip 1 024 bytes of patterned data.
     *
     * A repeating 5-byte pattern across 1 KB exercises the sliding window at
     * non-trivial scale and ensures the block grouping logic (8 tokens per
     * block) handles multi-block streams correctly.
     */
    @Test
    fun roundTrip1KbData() {
        val pattern = byteArrayOf(1, 2, 3, 4, 5)
        val data = ByteArray(1024) { pattern[it % pattern.size] }
        assertTrue(data.contentEquals(rt(data)), "1 KB patterned data should round-trip unchanged")
    }

    // ── 6. Encode: unique data → all Literals ────────────────────────────────

    /**
     * Test 6: Encoding data with no repeated sequences must produce only Literals.
     *
     * If every byte is unique (all 256 in order), no back-reference can beat
     * the min_match threshold of 3.  Every token in the list must therefore
     * be a Literal.
     */
    @Test
    fun encodeUniqueDataReturnsOnlyLiterals() {
        val data = ByteArray(256) { it.toByte() }
        val tokens = Lzss.encode(data)
        assertTrue(
            tokens.all { it is LzssToken.Literal },
            "256 unique bytes should all encode as Literals"
        )
        assertEquals(256, tokens.size, "Should have exactly 256 Literal tokens")
    }

    // ── 7. Encode: repeated data → Match tokens ───────────────────────────────

    /**
     * Test 7: Encoding a highly repeated pattern must produce Match tokens.
     *
     * "ABABAB" produces exactly 3 tokens: two Literals ('A', 'B') followed by
     * one Match.  This is the canonical LZSS demonstration case.
     *
     * Verification:
     *  - tokens[0] = Literal('A')
     *  - tokens[1] = Literal('B')
     *  - tokens[2] = Match(offset=2, length=4)
     *    offset=2 → go 2 bytes back from position 4, landing at position 2 ('A')
     *    length=4 → copy 4 bytes → "ABAB"
     */
    @Test
    fun encodeRepeatedDataReturnsMatchTokens() {
        val data = "ABABAB".encodeToByteArray()
        val tokens = Lzss.encode(data)

        assertEquals(3, tokens.size, "ABABAB should compress to exactly 3 tokens")
        assertEquals(LzssToken.Literal('A'.code.toByte()), tokens[0])
        assertEquals(LzssToken.Literal('B'.code.toByte()), tokens[1])
        assertTrue(tokens[2] is LzssToken.Match, "Third token must be a Match")

        val match = tokens[2] as LzssToken.Match
        assertEquals(2, match.offset, "Match offset must be 2 (back to 'A')")
        assertEquals(4, match.length, "Match length must be 4 (the remaining 'ABAB')")
    }

    // ── 8. Compressed < original for repetitive data ──────────────────────────

    /**
     * Test 8: Compressing repetitive data must produce fewer bytes than the original.
     *
     * "ABC" repeated 1 000 times = 3 000 bytes.  The compressed form should be
     * substantially smaller, proving that LZSS is providing actual compression.
     */
    @Test
    fun compressedSmallerThanOriginalForRepetitiveData() {
        val data = "ABC".repeat(1000).encodeToByteArray()
        val compressed = Lzss.compress(data)
        assertTrue(
            compressed.size < data.size,
            "Compressed size (${compressed.size}) should be less than original (${data.size})"
        )
    }

    // ── 9. Decode: correct output from a known token list ────────────────────

    /**
     * Test 9: Decoding a hand-crafted token list produces the expected bytes.
     *
     * Token list:
     *   Literal('A') → output: "A"
     *   Match(offset=1, length=6) → copy 6 bytes starting 1 back:
     *       each byte references the previous one, producing run-length "AAAAAA"
     *   Final output: "AAAAAAA" (7 bytes)
     *
     * This is the classic run-length case for LZSS and validates the
     * byte-at-a-time copy loop in `decode`.
     */
    @Test
    fun decodeCorrectFromKnownTokenList() {
        val tokens = listOf(
            LzssToken.Literal('A'.code.toByte()),
            LzssToken.Match(offset = 1, length = 6)
        )
        val decoded = Lzss.decode(tokens)
        assertTrue(
            "AAAAAAA".encodeToByteArray().contentEquals(decoded),
            "Literal('A') + Match(1,6) should decode to 'AAAAAAA'"
        )
    }

    // ── 10. Known vector ─────────────────────────────────────────────────────

    /**
     * Test 10: Known-vector check — compress "AAAAAAA" and verify the wire bytes.
     *
     * Expected token stream: Literal('A') + Match(offset=1, length=6)
     * → 2 tokens, 1 block
     *
     * Block flag: bit 0 = 0 (Literal), bit 1 = 1 (Match) → flag byte = 0b00000010 = 0x02
     * Symbol data: 0x41 (Literal 'A'), 0x00 0x01 (offset=1 BE), 0x06 (length=6)
     *
     * Full wire bytes:
     *   [00 00 00 07]  original_length = 7
     *   [00 00 00 01]  block_count = 1
     *   [02]           flag byte  (bit 1 set → token 1 is a Match)
     *   [41]           Literal 'A'
     *   [00 01]        offset = 1 (big-endian uint16)
     *   [06]           length = 6
     */
    @Test
    fun knownVectorAAAAAA() {
        val compressed = Lzss.compress("AAAAAAA".encodeToByteArray())

        // Header: original_length = 7
        assertEquals(0x00, compressed[0].toInt() and 0xFF)
        assertEquals(0x00, compressed[1].toInt() and 0xFF)
        assertEquals(0x00, compressed[2].toInt() and 0xFF)
        assertEquals(0x07, compressed[3].toInt() and 0xFF)

        // Header: block_count = 1
        assertEquals(0x00, compressed[4].toInt() and 0xFF)
        assertEquals(0x00, compressed[5].toInt() and 0xFF)
        assertEquals(0x00, compressed[6].toInt() and 0xFF)
        assertEquals(0x01, compressed[7].toInt() and 0xFF)

        // Block flag = 0x02 (bit 1 set = token index 1 is a Match)
        assertEquals(0x02, compressed[8].toInt() and 0xFF)

        // Literal 'A' = 0x41
        assertEquals(0x41, compressed[9].toInt() and 0xFF)

        // Match offset = 1 (big-endian uint16)
        assertEquals(0x00, compressed[10].toInt() and 0xFF)
        assertEquals(0x01, compressed[11].toInt() and 0xFF)

        // Match length = 6
        assertEquals(0x06, compressed[12].toInt() and 0xFF)

        // Total: 8 header + 1 flag + 1 literal + 3 match = 13 bytes
        assertEquals(13, compressed.size)
    }

    // ── 11. Unicode text round-trip ───────────────────────────────────────────

    /**
     * Test 11: Round-trip a multi-byte UTF-8 string.
     *
     * "こんにちは" (Japanese "hello") encodes to 15 UTF-8 bytes (3 bytes each).
     * Tests that the algorithm handles multi-byte character encodings correctly
     * by operating at the raw byte level — the encoder knows nothing about
     * Unicode; it just sees bytes.
     */
    @Test
    fun roundTripUnicodeText() {
        val text = "こんにちは世界"  // "Hello World" in Japanese
        val data = text.encodeToByteArray()
        assertTrue(
            data.contentEquals(rt(data)),
            "Unicode UTF-8 text should round-trip unchanged"
        )
    }

    // ── 12. Large input ───────────────────────────────────────────────────────

    /**
     * Test 12: Round-trip 10 KB of pseudo-random patterned data.
     *
     * Uses a 7-byte repeating pattern to generate 10 240 bytes — enough to
     * span multiple sliding windows (10 240 / 4 096 ≈ 2.5 windows) and
     * stress-test multi-window boundary handling.
     *
     * Also asserts that the compressed output is smaller than the original,
     * confirming real compression at scale.
     */
    @Test
    fun roundTripLargeInput() {
        val pattern = byteArrayOf(0x12, 0x34, 0x56, 0x78, 0x9A.toByte(), 0xBC.toByte(), 0xDE.toByte())
        val data = ByteArray(10240) { pattern[it % pattern.size] }

        val compressed = Lzss.compress(data)
        val decompressed = Lzss.decompress(compressed)

        assertTrue(data.contentEquals(decompressed), "10 KB large input should round-trip unchanged")
        assertTrue(
            compressed.size < data.size,
            "Compressed large input (${compressed.size}) should be smaller than original (${data.size})"
        )
    }
}
