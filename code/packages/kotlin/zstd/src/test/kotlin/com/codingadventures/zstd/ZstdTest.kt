/**
 * Tests for the Kotlin ZStd (CMP07) implementation.
 *
 * Each test case corresponds to a specific scenario exercising a different
 * part of the compress/decompress pipeline:
 *
 *   TC01 — empty input
 *   TC02 — single byte
 *   TC03 — all 256 byte values
 *   TC04 — RLE block detection
 *   TC05 — English prose (repetitive → compressed block)
 *   TC06 — pseudo-random data (incompressible → raw block)
 *   TC07 — multi-block input (> 128 KB)
 *   TC08 — repeat-offset pattern
 *   TC09 — deterministic output
 *   TC10 — manually constructed wire-format frame
 *   TC11 — binary data (zeros and 0xFF bytes)
 *   TC12 — all zeros
 *   TC13 — all 0xFF bytes
 *   TC14 — hello world
 *   TC15 — repeated 6-byte pattern
 *   TC16 — LL-to-code mapping for small values
 *   TC17 — ML-to-code mapping for small values
 *   TC18 — literals section round-trip (short)
 *   TC19 — literals section round-trip (medium)
 *   TC20 — literals section round-trip (large)
 *   TC21 — RevBitWriter/RevBitReader round-trip
 *   TC22 — FSE decode table coverage check
 *   TC23 — sequence count encode/decode round-trip
 *   TC24 — two-sequence FSE round-trip
 */
package com.codingadventures.zstd

import org.junit.jupiter.api.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlin.test.assertContentEquals
import java.io.IOException
import org.junit.jupiter.api.assertThrows

class ZstdTest {

    // Helper: round-trip via our own compress/decompress.
    private fun rt(data: ByteArray): ByteArray {
        val compressed = Zstd.compress(data)
        return Zstd.decompress(compressed)
    }

    // ── TC01: empty input ──────────────────────────────────────────────────────

    @Test
    fun tc01_empty() {
        // An empty input must produce a valid ZStd frame and decompress back
        // to empty bytes without exception.
        assertContentEquals(byteArrayOf(), rt(byteArrayOf()))
    }

    // ── TC02: single byte ──────────────────────────────────────────────────────

    @Test
    fun tc02_singleByte() {
        // The smallest non-empty input: one byte.
        assertContentEquals(byteArrayOf(0x42), rt(byteArrayOf(0x42)))
    }

    // ── TC03: all 256 byte values ──────────────────────────────────────────────

    @Test
    fun tc03_allBytes() {
        // Every possible byte value 0x00..0xFF in order. This exercises
        // literal encoding of non-ASCII and zero bytes.
        val input = ByteArray(256) { it.toByte() }
        assertContentEquals(input, rt(input))
    }

    // ── TC04: RLE block ────────────────────────────────────────────────────────

    @Test
    fun tc04_rle() {
        // 1024 identical bytes should be detected as an RLE block.
        // Expected compressed size: 4 (magic) + 1 (FHD) + 8 (FCS) + 3 (block header)
        //                         + 1 (RLE byte) = 17 bytes < 30.
        val input = ByteArray(1024) { 'A'.code.toByte() }
        val compressed = Zstd.compress(input)
        assertContentEquals(input, Zstd.decompress(compressed))
        assertTrue(
            compressed.size < 30,
            "RLE of 1024 bytes compressed to ${compressed.size} (expected < 30)"
        )
    }

    // ── TC05: English prose ────────────────────────────────────────────────────

    @Test
    fun tc05_prose() {
        // Repeated English text has strong LZ77 matches. Must achieve ≥ 20%
        // compression (output ≤ 80% of input size).
        val text = "the quick brown fox jumps over the lazy dog ".repeat(25)
        val input = text.encodeToByteArray()
        val compressed = Zstd.compress(input)
        assertContentEquals(input, Zstd.decompress(compressed))
        val threshold = input.size * 80 / 100
        assertTrue(
            compressed.size < threshold,
            "prose: compressed ${compressed.size} bytes (input ${input.size}), expected < $threshold (80%)"
        )
    }

    // ── TC06: pseudo-random data ───────────────────────────────────────────────

    @Test
    fun tc06_random() {
        // LCG pseudo-random bytes. No significant compression expected, but
        // round-trip must be exact regardless of block type chosen.
        var seed = 42
        val input = ByteArray(512) {
            seed = seed * 1664525 + 1013904223
            (seed and 0xFF).toByte()
        }
        assertContentEquals(input, rt(input))
    }

    // ── TC07: multi-block (> 128 KB) ──────────────────────────────────────────

    @Test
    fun tc07_multiblock() {
        // 200 KB > MAX_BLOCK_SIZE (128 KB), so this requires at least 2 blocks.
        // Both should be RLE blocks since all bytes are identical.
        val input = ByteArray(200 * 1024) { 'x'.code.toByte() }
        assertContentEquals(input, rt(input))
    }

    // ── TC08: repeat-offset pattern ───────────────────────────────────────────

    @Test
    fun tc08_repeatOffset() {
        // Alternating pattern with long runs of 'X' and repeated "ABCDEFGH".
        // The 'X' runs and repeated patterns both give strong LZ77 matches.
        val pattern = "ABCDEFGH".encodeToByteArray()
        val input = ArrayList<Byte>()
        for (b in pattern) input.add(b)
        repeat(10) {
            repeat(128) { input.add('X'.code.toByte()) }
            for (b in pattern) input.add(b)
        }
        val inputArr = input.toByteArray()
        val compressed = Zstd.compress(inputArr)
        assertContentEquals(inputArr, Zstd.decompress(compressed))
        val threshold = inputArr.size * 70 / 100
        assertTrue(
            compressed.size < threshold,
            "repeat-offset: compressed ${compressed.size} (input ${inputArr.size}), expected < $threshold (70%)"
        )
    }

    // ── TC09: deterministic output ─────────────────────────────────────────────

    @Test
    fun tc09_deterministic() {
        // Compressing the same data twice must produce identical bytes.
        // Required for reproducible builds and cache invalidation.
        val data = "hello, ZStd world! ".repeat(50).encodeToByteArray()
        assertContentEquals(Zstd.compress(data), Zstd.compress(data))
    }

    // ── TC10: manual wire-format frame ────────────────────────────────────────

    @Test
    fun tc10_wireFormat() {
        // Manually constructed ZStd frame to verify our decoder reads the
        // wire format correctly without depending on our encoder.
        //
        // Frame layout:
        //   [0..3]  Magic = 0xFD2FB528 LE = [0x28, 0xB5, 0x2F, 0xFD]
        //   [4]     FHD = 0x20:
        //             bits [7:6] = 00 → FCS flag 0
        //             bit  [5]   = 1  → Single_Segment = 1
        //             bits [4:0] = 0  → no checksum, no dict
        //           With Single_Segment=1 and FCS_flag=00, FCS is 1 byte.
        //   [5]     FCS = 0x05 (content_size = 5)
        //   [6..8]  Block header: Last=1, Type=Raw, Size=5
        //             = (5 shl 3) or (0 shl 1) or 1 = 41 = 0x29
        //             = [0x29, 0x00, 0x00]
        //   [9..13] b"hello"
        val frame = byteArrayOf(
            0x28, 0xB5.toByte(), 0x2F, 0xFD.toByte(),  // magic
            0x20,                                         // FHD: Single_Segment=1, FCS=1byte
            0x05,                                         // FCS = 5
            0x29, 0x00, 0x00,                             // block header: last=1, raw, size=5
            'h'.code.toByte(), 'e'.code.toByte(), 'l'.code.toByte(), 'l'.code.toByte(), 'o'.code.toByte(),
        )
        assertContentEquals("hello".encodeToByteArray(), Zstd.decompress(frame))
    }

    // ── TC11: binary data ──────────────────────────────────────────────────────

    @Test
    fun tc11_binaryData() {
        // Binary data with repeating pattern — lots of zeros and 0xFF bytes.
        val input = ByteArray(300) { (it % 256).toByte() }
        assertContentEquals(input, rt(input))
    }

    // ── TC12: all zeros ────────────────────────────────────────────────────────

    @Test
    fun tc12_allZeros() {
        val input = ByteArray(1000) { 0 }
        assertContentEquals(input, rt(input))
    }

    // ── TC13: all 0xFF ─────────────────────────────────────────────────────────

    @Test
    fun tc13_allFF() {
        val input = ByteArray(1000) { 0xFF.toByte() }
        assertContentEquals(input, rt(input))
    }

    // ── TC14: hello world ─────────────────────────────────────────────────────

    @Test
    fun tc14_helloWorld() {
        val input = "hello world".encodeToByteArray()
        assertContentEquals(input, rt(input))
    }

    // ── TC15: repeated 6-byte pattern ─────────────────────────────────────────

    @Test
    fun tc15_repeatedPattern() {
        val data = ByteArray(3000)
        val pat = "ABCDEF".encodeToByteArray()
        for (i in data.indices) data[i] = pat[i % pat.size]
        assertContentEquals(data, rt(data))
    }

    // ── TC16: LL-to-code mapping for small values ──────────────────────────────

    @Test
    fun tc16_llToCodeSmall() {
        for (i in 0 until 16) {
            assertEquals(i, llToCode(i.toLong()), "LL code for $i")
        }
    }

    // ── TC17: ML-to-code mapping for small values ──────────────────────────────

    @Test
    fun tc17_mlToCodeSmall() {
        for (i in 3 until 35) {
            assertEquals(i - 3, mlToCode(i.toLong()), "ML code for $i")
        }
    }

    // ── TC18: literals section round-trip (short) ──────────────────────────────

    @Test
    fun tc18_literalsSectionShort() {
        val lits = ByteArray(20) { it.toByte() }
        val encoded = encodeLiteralsSection(lits)
        val (decoded, _) = decodeLiteralsSection(encoded)
        assertContentEquals(lits, decoded)
    }

    // ── TC19: literals section round-trip (medium) ─────────────────────────────

    @Test
    fun tc19_literalsSectionMedium() {
        val lits = ByteArray(200) { (it % 256).toByte() }
        val encoded = encodeLiteralsSection(lits)
        val (decoded, _) = decodeLiteralsSection(encoded)
        assertContentEquals(lits, decoded)
    }

    // ── TC20: literals section round-trip (large) ──────────────────────────────

    @Test
    fun tc20_literalsSectionLarge() {
        val lits = ByteArray(5000) { (it % 256).toByte() }
        val encoded = encodeLiteralsSection(lits)
        val (decoded, _) = decodeLiteralsSection(encoded)
        assertContentEquals(lits, decoded)
    }

    // ── TC21: RevBitWriter/RevBitReader round-trip ─────────────────────────────

    @Test
    fun tc21_revBitStreamRoundTrip() {
        // The backward bit stream stores bits so the LAST-written bits are
        // read FIRST by the decoder. This mirrors how ZStd's sequence codec
        // writes the initial FSE states last (so the decoder reads them first).
        //
        // Write order:  A=0b101 (3 bits), B=0b11001100 (8 bits), C=0b1 (1 bit)
        // Read order:   C first, then B, then A  (reversed)
        val bw = RevBitWriter()
        bw.addBits(0b101L, 3)       // A — written first → read last
        bw.addBits(0b11001100L, 8)  // B
        bw.addBits(0b1L, 1)         // C — written last → read first
        bw.flush()
        val buf = bw.finish()

        val br = RevBitReader(buf)
        assertEquals(0b1L, br.readBits(1), "C: last written, first read")
        assertEquals(0b11001100L, br.readBits(8), "B")
        assertEquals(0b101L, br.readBits(3), "A: first written, last read")
    }

    // ── TC22: FSE decode table coverage ───────────────────────────────────────

    @Test
    fun tc22_fseDecodeTableCoverage() {
        // Every slot in the decode table should be reachable (sym is valid).
        val dt = buildDecodeTable(LL_NORM, LL_ACC_LOG)
        assertEquals(1 shl LL_ACC_LOG, dt.size)
        for (cell in dt) {
            assertTrue(cell.sym < LL_NORM.size, "sym ${cell.sym} out of range")
        }
    }

    // ── TC23: sequence count encode/decode round-trip ──────────────────────────

    @Test
    fun tc23_seqCountRoundTrip() {
        for (n in listOf(0, 1, 50, 127, 128, 1000, 0x7FFE)) {
            val enc = encodeSeqCount(n)
            val (dec, _) = decodeSeqCount(enc)
            assertEquals(n, dec, "seq count $n")
        }
    }

    // ── TC24: two-sequence FSE round-trip ──────────────────────────────────────

    @Test
    fun tc24_twoSequenceFseRoundTrip() {
        // Test encoding and decoding two sequences to verify FSE state transitions.
        val seqs = listOf(
            Seq(2L, 4L, 1L),
            Seq(0L, 3L, 2L),
        )
        val bitstream = encodeSequencesSection(seqs)

        val dtLl = buildDecodeTable(LL_NORM, LL_ACC_LOG)
        val dtMl = buildDecodeTable(ML_NORM, ML_ACC_LOG)
        val dtOf = buildDecodeTable(OF_NORM, OF_ACC_LOG)

        val br = RevBitReader(bitstream)
        var stateLl = br.readBits(LL_ACC_LOG).toInt()
        var stateMl = br.readBits(ML_ACC_LOG).toInt()
        var stateOf = br.readBits(OF_ACC_LOG).toInt()

        for ((i, expected) in seqs.withIndex()) {
            // Decode LL symbol
            val eLl = dtLl[stateLl]
            val llCode = eLl.sym
            stateLl = (eLl.base + br.readBits(eLl.nb)).toInt()

            // Decode OF symbol
            val eOf = dtOf[stateOf]
            val ofCode = eOf.sym
            stateOf = (eOf.base + br.readBits(eOf.nb)).toInt()

            // Decode ML symbol
            val eMl = dtMl[stateMl]
            val mlCode = eMl.sym
            stateMl = (eMl.base + br.readBits(eMl.nb)).toInt()

            // Decode extra bits
            val llInfo = LL_CODES[llCode]
            val mlInfo = ML_CODES[mlCode]
            val llDec = llInfo[0] + br.readBits(llInfo[1].toInt())
            val mlDec = mlInfo[0] + br.readBits(mlInfo[1].toInt())
            val ofRaw = (1L shl ofCode) or br.readBits(ofCode)
            val offDec = ofRaw - 3L

            assertEquals(expected.ll, llDec, "seq $i LL")
            assertEquals(expected.ml, mlDec, "seq $i ML")
            assertEquals(expected.off, offDec, "seq $i OFF")
        }
    }
}
