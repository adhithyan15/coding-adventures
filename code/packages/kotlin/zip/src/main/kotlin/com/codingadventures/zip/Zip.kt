/**
 * ZIP archive format (PKZIP, 1989) — CMP09.
 *
 * ZIP bundles one or more files into a single `.zip` archive, compressing each
 * entry independently with DEFLATE (method 8) or storing it verbatim (method 0).
 * The same container format underlies Java JARs, Office Open XML (.docx), Android
 * APKs, Python wheels, and many other formats.
 *
 * ## Architecture
 *
 * ```
 * ┌─────────────────────────────────────────────────────┐
 * │  [Local File Header + File Data]  ← entry 1         │
 * │  [Local File Header + File Data]  ← entry 2         │
 * │  ...                                                │
 * │  ══════════ Central Directory ══════════            │
 * │  [Central Dir Header]  ← entry 1 (has local offset)│
 * │  [Central Dir Header]  ← entry 2                   │
 * │  [End of Central Directory Record]                  │
 * └─────────────────────────────────────────────────────┘
 * ```
 *
 * The dual-header design supports two workflows:
 *   - **Sequential write**: append Local Headers one-by-one, write Central Directory at end.
 *   - **Random-access read**: seek to EOCD, read Central Directory, jump to any entry.
 *
 * ## Wire Format (all integers little-endian)
 *
 * Local File Header (30 + name_len + extra_len bytes):
 * ```
 * [0x04034B50]  signature
 * [version_needed u16]  20=DEFLATE, 10=Stored
 * [flags u16]           bit 11 = UTF-8 filename
 * [method u16]          0=Stored, 8=DEFLATE
 * [mod_time u16]        MS-DOS packed time
 * [mod_date u16]        MS-DOS packed date
 * [crc32 u32]
 * [compressed_size u32]
 * [uncompressed_size u32]
 * [name_len u16]
 * [extra_len u16]
 * [name bytes...]
 * [extra bytes...]
 * [file data...]
 * ```
 *
 * Central Directory Header (46 + name_len + extra_len + comment_len bytes):
 * ```
 * [0x02014B50]  signature
 * [version_made_by u16]
 * [version_needed u16]
 * [flags u16]
 * [method u16]
 * [mod_time u16]
 * [mod_date u16]
 * [crc32 u32]
 * [compressed_size u32]
 * [uncompressed_size u32]
 * [name_len u16]
 * [extra_len u16]
 * [comment_len u16]
 * [disk_start u16]
 * [int_attrs u16]
 * [ext_attrs u32]   Unix: (mode << 16)
 * [local_offset u32]
 * [name bytes...]
 * ```
 *
 * End of Central Directory Record (22 bytes):
 * ```
 * [0x06054B50]  signature
 * [disk_num u16]
 * [cd_disk u16]
 * [entries_this_disk u16]
 * [entries_total u16]
 * [cd_size u32]
 * [cd_offset u32]
 * [comment_len u16]
 * ```
 *
 * ## DEFLATE Inside ZIP
 *
 * ZIP method 8 stores **raw RFC 1951 DEFLATE** — no zlib wrapper (no CMF/FLG
 * header, no Adler-32 checksum). This implementation produces RFC 1951 fixed-
 * Huffman compressed blocks (BTYPE=01) using the `lzss` package for LZ77 match-
 * finding, giving real compression without transmitting dynamic Huffman tables.
 *
 * ## Series
 *
 * ```
 * CMP00 (LZ77,    1977) — Sliding-window backreferences.
 * CMP01 (LZ78,    1978) — Explicit dictionary (trie).
 * CMP02 (LZSS,    1982) — LZ77 + flag bits.
 * CMP03 (LZW,     1984) — LZ78 + pre-initialized alphabet; GIF.
 * CMP04 (Huffman, 1952) — Entropy coding.
 * CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
 * CMP09 (ZIP,     1989) — DEFLATE container; universal archive.  ← THIS FILE
 * ```
 */
package com.codingadventures.zip

import com.codingadventures.lzss.Lzss
import com.codingadventures.lzss.LzssToken
import java.io.IOException
import java.nio.ByteBuffer
import java.nio.ByteOrder

// =============================================================================
// Wire Format Constants
// =============================================================================
//
// ZIP uses four-byte "magic number" signatures to identify each structural
// region. All integers in the wire format are little-endian.

private const val LOCAL_SIG: Int = 0x04034B50.toInt()    // "PK\x03\x04"
private const val CD_SIG: Int = 0x02014B50.toInt()       // "PK\x01\x02"
private const val EOCD_SIG: Int = 0x06054B50.toInt()     // "PK\x05\x06"

// Fixed timestamp: 1980-01-01 00:00:00
// DOS date = (0<<9)|(1<<5)|1 = 0x0021; time = 0 → combined 0x00210000
private const val DOS_EPOCH: Int = 0x00210000

// General Purpose Bit Flag: bit 11 = UTF-8 filename encoding
private const val FLAG_UTF8: Short = 0x0800.toShort()

// Compression methods
private const val METHOD_STORED: Short = 0
private const val METHOD_DEFLATE: Short = 8

// Version needed to extract
private const val VERSION_DEFLATE: Short = 20
private const val VERSION_STORED: Short = 10

// Version made by: Unix OS (high byte 3), spec version 30 (low byte 0x1E)
private const val VERSION_MADE_BY: Short = 0x031E.toShort()

// Unix file modes embedded in Central Directory external_attrs (shifted left 16 bits).
// 0o100644 = regular file, rw-r--r--; 0o040755 = directory, rwxr-xr-x
private const val UNIX_MODE_FILE: Int = 33188  // 0o100644 in decimal
private const val UNIX_MODE_DIR: Int = 16877   // 0o040755 in decimal

// =============================================================================
// CRC-32
// =============================================================================
//
// CRC-32 uses polynomial 0xEDB88320 (reflected form of 0x04C11DB7).
// It detects accidental corruption of decompressed content. It is NOT a
// cryptographic hash — for tamper-detection use AES-GCM or a signed manifest.
//
// The table is computed once at class-load time — each entry is the CRC-32
// of a single byte value using the reflected polynomial.

/**
 * Precomputed CRC-32 lookup table (polynomial 0xEDB88320, reflected form).
 *
 * For each possible byte value (0–255), the table holds the CRC-32 of
 * that single byte. This lets us process one byte per table lookup
 * instead of doing 8 XOR/shift operations per byte inline.
 */
private val CRC_TABLE: IntArray = IntArray(256) { i ->
    var c = i
    repeat(8) {
        c = if (c and 1 != 0) (0xEDB88320.toInt() xor (c ushr 1)) else (c ushr 1)
    }
    c
}

/**
 * Compute CRC-32 over [data].
 *
 * Pass [initial] = 0 for a fresh hash, or the previous result to continue
 * an incremental computation.
 *
 * The internal pre/post XOR with 0xFFFFFFFF is handled inside this function.
 * Example:
 * ```kotlin
 * crc32("hello world".toByteArray(), 0) == 0x0D4A1185
 * ```
 */
fun crc32(data: ByteArray, initial: Int = 0): Int {
    // XOR in the initial state. For the first call initial=0 → crc starts at -1 (0xFFFFFFFF).
    var crc = initial xor -1
    for (b in data) {
        crc = CRC_TABLE[(crc xor b.toInt()) and 0xFF] xor (crc ushr 8)
    }
    // XOR out to produce the final CRC.
    return crc xor -1
}

// =============================================================================
// RFC 1951 DEFLATE — Bit I/O
// =============================================================================
//
// RFC 1951 packs bits LSB-first within bytes. Huffman codes are sent with the
// most-significant bit first — so before writing a Huffman code we reverse its
// bits and then write the reversed value LSB-first. Extra bits (length/distance
// extras, stored block headers) are written directly LSB-first without reversal.

/**
 * Writes bits into a byte stream, LSB-first.
 *
 * Internally maintains a 64-bit accumulator [reg] with [bits] valid bits.
 * Whenever there are 8 or more valid bits, the lowest byte is drained to [buf].
 *
 * Think of it like loading coins into a coin tube from the top and dispensing
 * from the bottom — bits come in at the high end and bytes drain from the low end.
 */
private class BitWriter {
    private val buf = mutableListOf<Byte>()
    private var reg: Long = 0L   // accumulator holding up to 63 unflushable bits
    private var bits: Int = 0    // number of valid bits in reg

    /**
     * Write the [n] low-order bits of [value] into the stream, LSB-first.
     * Used for extra bits and stored-block headers.
     */
    fun writeLsb(value: Long, n: Int) {
        // OR the masked value into the accumulator at the current bit position.
        val mask = if (n >= 64) -1L else (1L shl n) - 1L
        reg = reg or ((value and mask) shl bits)
        bits += n
        // Drain complete bytes from the accumulator.
        while (bits >= 8) {
            buf.add((reg and 0xFF).toByte())
            reg = reg ushr 8
            bits -= 8
        }
    }

    /**
     * Write a Huffman code of [nbits] bits.
     *
     * Huffman codes are logically MSB-first, so we bit-reverse them before
     * writing. Example: code=0b110, nbits=3 is written as 0b011.
     */
    fun writeHuffman(code: Int, nbits: Int) {
        // Reverse the bottom `nbits` bits of `code`.
        var rev = 0
        var c = code
        repeat(nbits) {
            rev = (rev shl 1) or (c and 1)
            c = c ushr 1
        }
        writeLsb(rev.toLong(), nbits)
    }

    /**
     * Flush any partial byte to the buffer (zero-pad the remaining bits).
     * Required before stored-block headers, which must be byte-aligned.
     */
    fun flush() {
        if (bits > 0) {
            buf.add((reg and 0xFF).toByte())
            reg = 0L
            bits = 0
        }
    }

    /** Return the completed byte array. Flushes any partial byte first. */
    fun toByteArray(): ByteArray {
        flush()
        return buf.toByteArray()
    }
}

/**
 * Reads bits from a byte array, LSB-first.
 *
 * Mirrors BitWriter: fills a 64-bit accumulator from bytes in the source array.
 * Huffman code decoding reads MSB-first by bit-reversing the extracted value.
 */
private class BitReader(private val data: ByteArray) {
    private var pos: Int = 0   // next byte to consume from data
    private var buf: Long = 0L // bit accumulator
    private var bits: Int = 0  // valid bits in buf

    /**
     * Ensure the accumulator holds at least [need] bits.
     * Returns false if the source is exhausted before that many bits are available.
     */
    private fun fill(need: Int): Boolean {
        while (bits < need) {
            if (pos >= data.size) return false
            buf = buf or ((data[pos].toInt() and 0xFF).toLong() shl bits)
            pos++
            bits += 8
        }
        return true
    }

    /**
     * Read [nbits] bits from the stream, LSB-first.
     * Returns null on end-of-input.
     */
    fun readLsb(nbits: Int): Int? {
        if (nbits == 0) return 0
        if (!fill(nbits)) return null
        val mask = (1L shl nbits) - 1L
        val value = (buf and mask).toInt()
        buf = buf ushr nbits
        bits -= nbits
        return value
    }

    /**
     * Read [nbits] bits and bit-reverse the result.
     * Used when decoding Huffman codes (logically MSB-first).
     */
    fun readMsb(nbits: Int): Int? {
        val v = readLsb(nbits) ?: return null
        var rev = 0
        var u = v
        repeat(nbits) {
            rev = (rev shl 1) or (u and 1)
            u = u ushr 1
        }
        return rev
    }

    /**
     * Discard any partial-byte bits, aligning to the next byte boundary.
     * Required before reading stored-block length fields.
     */
    fun align() {
        val discard = bits % 8
        if (discard > 0) {
            buf = buf ushr discard
            bits -= discard
        }
    }
}

// =============================================================================
// RFC 1951 DEFLATE — Fixed Huffman Tables
// =============================================================================
//
// RFC 1951 §3.2.6 specifies fixed (pre-defined) Huffman code lengths.
// Using fixed Huffman blocks (BTYPE=01) means we never transmit code tables —
// both encoder and decoder know the tables in advance. This is simpler than
// dynamic Huffman (BTYPE=10) and still achieves real compression via LZ77.
//
// Literal/Length code lengths:
//   Symbols   0–143: 8-bit codes, starting at 0b00110000 (= 48)
//   Symbols 144–255: 9-bit codes, starting at 0b110010000 (= 400)
//   Symbols 256–279: 7-bit codes, starting at 0b0000000 (= 0)
//   Symbols 280–287: 8-bit codes, starting at 0b11000000 (= 192)
//
// Distance codes:
//   Symbols 0–29: 5-bit codes equal to the symbol number.

/**
 * Return the (code, nbits) pair for encoding literal/length symbol 0–287
 * using the RFC 1951 fixed Huffman table.
 */
private fun fixedLlEncode(sym: Int): Pair<Int, Int> = when (sym) {
    in 0..143   -> Pair(sym + 0x30, 8)              // 8-bit codes, base 48
    in 144..255 -> Pair(sym - 144 + 0x190, 9)       // 9-bit codes, base 400
    in 256..279 -> Pair(sym - 256, 7)               // 7-bit codes, base 0
    in 280..287 -> Pair(sym - 280 + 0xC0, 8)        // 8-bit codes, base 192
    else        -> throw IOException("fixedLlEncode: invalid LL symbol $sym")
}

/**
 * Decode one literal/length symbol from [br] using the RFC 1951 fixed Huffman table.
 *
 * Reads bits incrementally — first 7, then up to 9 — and decodes in order
 * of increasing code length per the canonical Huffman property.
 * Returns null on end-of-input.
 *
 * Decode order:
 *   1. Try 7 bits: codes 0–23 → symbols 256–279 (end-of-block + length codes)
 *   2. Need 8 bits: codes 48–191 → literals 0–143; codes 192–199 → symbols 280–287
 *   3. Need 9 bits: codes 400–511 → literals 144–255
 */
private fun fixedLlDecode(br: BitReader): Int? {
    val v7 = br.readMsb(7) ?: return null
    if (v7 <= 23) {
        // 7-bit code: symbols 256–279.
        return v7 + 256
    }
    // Need one more bit for 8-bit codes.
    val extra1 = br.readLsb(1) ?: return null
    val v8 = (v7 shl 1) or extra1
    return when {
        v8 in 48..191  -> v8 - 48                  // literals 0–143
        v8 in 192..199 -> v8 + 88                  // symbols 280–287 (192+88=280)
        else           -> {
            // Need one more bit for 9-bit codes (literals 144–255).
            val extra2 = br.readLsb(1) ?: return null
            val v9 = (v8 shl 1) or extra2
            if (v9 in 400..511) v9 - 256 else null // literals 144–255 (400-256=144)
        }
    }
}

// =============================================================================
// RFC 1951 DEFLATE — Length / Distance Tables
// =============================================================================
//
// Match lengths (3-255) map to LL symbols 257-284 + extra bits.
// Match distances (1-32768) map to distance codes 0-29 + extra bits.
// The tables come directly from RFC 1951 §3.2.5.

/**
 * (base_length, extra_bits) for LL symbols 257..284, indexed by (symbol - 257).
 *
 * To encode a match of length L: find the largest base ≤ L, emit the
 * corresponding LL symbol + (L - base) as extra bits.
 */
private val LENGTH_TABLE: Array<Pair<Int, Int>> = arrayOf(
    Pair(3, 0), Pair(4, 0), Pair(5, 0), Pair(6, 0),
    Pair(7, 0), Pair(8, 0), Pair(9, 0), Pair(10, 0), // 257–264
    Pair(11, 1), Pair(13, 1), Pair(15, 1), Pair(17, 1),                 // 265–268
    Pair(19, 2), Pair(23, 2), Pair(27, 2), Pair(31, 2),                 // 269–272
    Pair(35, 3), Pair(43, 3), Pair(51, 3), Pair(59, 3),                 // 273–276
    Pair(67, 4), Pair(83, 4), Pair(99, 4), Pair(115, 4),               // 277–280
    Pair(131, 5), Pair(163, 5), Pair(195, 5), Pair(227, 5)             // 281–284
)

/**
 * (base_distance, extra_bits) for distance codes 0..29.
 *
 * To encode a match at distance D: find the largest base ≤ D, emit the
 * distance code number (5 bits) + (D - base) as extra bits.
 */
private val DIST_TABLE: Array<Pair<Int, Int>> = arrayOf(
    Pair(1, 0), Pair(2, 0), Pair(3, 0), Pair(4, 0),
    Pair(5, 1), Pair(7, 1), Pair(9, 2), Pair(13, 2),
    Pair(17, 3), Pair(25, 3), Pair(33, 4), Pair(49, 4),
    Pair(65, 5), Pair(97, 5), Pair(129, 6), Pair(193, 6),
    Pair(257, 7), Pair(385, 7), Pair(513, 8), Pair(769, 8),
    Pair(1025, 9), Pair(1537, 9), Pair(2049, 10), Pair(3073, 10),
    Pair(4097, 11), Pair(6145, 11), Pair(8193, 12), Pair(12289, 12),
    Pair(16385, 13), Pair(24577, 13)
)

/**
 * Map a match length (3–255) to its RFC 1951 LL symbol, base, and extra-bit count.
 *
 * Returns `Triple(ll_symbol, base, extra_bits)`.
 * Scans from the highest entry downward to find the rightmost entry where base ≤ length.
 */
private fun encodeLength(length: Int): Triple<Int, Int, Int> {
    for (i in LENGTH_TABLE.indices.reversed()) {
        val (base, extra) = LENGTH_TABLE[i]
        if (length >= base) {
            return Triple(257 + i, base, extra)
        }
    }
    throw IOException("encodeLength: unreachable for length=$length")
}

/**
 * Map a match offset (1–32768) to its RFC 1951 distance code, base, and extra-bit count.
 *
 * Returns `Triple(dist_code, base, extra_bits)`.
 */
private fun encodeDist(offset: Int): Triple<Int, Int, Int> {
    for (code in DIST_TABLE.indices.reversed()) {
        val (base, extra) = DIST_TABLE[code]
        if (offset >= base) {
            return Triple(code, base, extra)
        }
    }
    throw IOException("encodeDist: unreachable for offset=$offset")
}

// =============================================================================
// RFC 1951 DEFLATE — Compress (fixed Huffman, BTYPE=01)
// =============================================================================
//
// Strategy:
//   1. Run LZ77/LZSS match-finding (window=32768, max match=255, min=3).
//   2. Emit a single BTYPE=01 (fixed Huffman) block containing the token stream.
//   3. Literal bytes → fixed LL Huffman code.
//   4. Match (offset, length) → length LL code + extra bits + distance code + extra.
//   5. End-of-block symbol (256) → fixed LL Huffman code.
//
// We produce the entire input as one block. RFC 1951 does not limit Huffman
// block sizes (only stored blocks are capped at 65535 bytes).

/**
 * Compress [data] to a raw RFC 1951 DEFLATE bit-stream (fixed Huffman, single block).
 *
 * The output starts directly with the 3-bit block header — no zlib wrapper.
 * Empty input produces a stored block (BTYPE=00, LEN=0) which is the canonical
 * representation for zero bytes in raw DEFLATE.
 */
internal fun deflateCompress(data: ByteArray): ByteArray {
    val bw = BitWriter()

    if (data.isEmpty()) {
        // Empty stored block: BFINAL=1 BTYPE=00 + 2-byte LEN=0 + 2-byte NLEN.
        bw.writeLsb(1L, 1)      // BFINAL = 1 (last block)
        bw.writeLsb(0L, 2)      // BTYPE = 00 (stored)
        bw.flush()               // align to byte boundary
        bw.writeLsb(0x0000L, 16) // LEN = 0
        bw.writeLsb(0xFFFFL, 16) // NLEN = ~0
        return bw.toByteArray()
    }

    // Run LZ77/LZSS tokenizer.
    // Window = 32768 so every match distance fits in the RFC 1951 distance table.
    // Max match = 255 to fit the RFC 1951 length table (symbols 257–284).
    val tokens = Lzss.encode(data, windowSize = 32768, maxMatch = 255, minMatch = 3)

    // Block header: BFINAL=1 (single last block), BTYPE=01 (fixed Huffman).
    // Bits are written LSB-first: BFINAL in bit 0, BTYPE in bits 1–2.
    bw.writeLsb(1L, 1) // BFINAL = 1
    bw.writeLsb(1L, 1) // BTYPE bit 0 = 1  }  → BTYPE = 0b01 = fixed Huffman
    bw.writeLsb(0L, 1) // BTYPE bit 1 = 0  }

    for (token in tokens) {
        when (token) {
            is LzssToken.Literal -> {
                // Literal byte: emit its fixed Huffman code.
                val (code, nbits) = fixedLlEncode(token.value.toInt() and 0xFF)
                bw.writeHuffman(code, nbits)
            }
            is LzssToken.Match -> {
                // ── Length ──────────────────────────────────────────────────
                val (lenSym, lenBase, lenExtraBits) = encodeLength(token.length)
                val (lenCode, lenBits) = fixedLlEncode(lenSym)
                bw.writeHuffman(lenCode, lenBits)
                if (lenExtraBits > 0) {
                    bw.writeLsb((token.length - lenBase).toLong(), lenExtraBits)
                }

                // ── Distance ─────────────────────────────────────────────────
                // Distance codes are 5-bit fixed codes equal to the code number.
                val (distCode, distBase, distExtraBits) = encodeDist(token.offset)
                bw.writeHuffman(distCode, 5)
                if (distExtraBits > 0) {
                    bw.writeLsb((token.offset - distBase).toLong(), distExtraBits)
                }
            }
        }
    }

    // End-of-block symbol (256) — signals the decoder to stop.
    val (eobCode, eobBits) = fixedLlEncode(256)
    bw.writeHuffman(eobCode, eobBits)

    return bw.toByteArray()
}

// =============================================================================
// RFC 1951 DEFLATE — Decompress
// =============================================================================
//
// Handles stored blocks (BTYPE=00) and fixed Huffman blocks (BTYPE=01).
// Dynamic Huffman blocks (BTYPE=10) throw IOException — we only produce BTYPE=01,
// but we must be able to decompress stored blocks written by other tools.
//
// Security limits:
//   - Maximum output: 256 MB (decompression bomb guard)
//   - LEN/NLEN validation on stored blocks

private const val MAX_OUTPUT_BYTES = 256 * 1024 * 1024  // 256 MB

/**
 * Decompress a raw RFC 1951 DEFLATE bit-stream into its original bytes.
 *
 * Throws [IOException] on malformed or unsupported (BTYPE=10 dynamic Huffman) input.
 *
 * Security: a 256 MB hard limit prevents decompression bomb attacks where
 * a tiny compressed input expands to gigabytes of output.
 */
internal fun deflateDecompress(data: ByteArray): ByteArray {
    val br = BitReader(data)
    val out = mutableListOf<Byte>()

    while (true) {
        val bfinal = br.readLsb(1)
            ?: throw IOException("deflate: unexpected EOF reading BFINAL")
        val btype = br.readLsb(2)
            ?: throw IOException("deflate: unexpected EOF reading BTYPE")

        when (btype) {
            0b00 -> {
                // ── Stored block ──────────────────────────────────────────────
                // Align to byte boundary before reading the length fields.
                br.align()
                val len = br.readLsb(16)
                    ?: throw IOException("deflate: EOF reading stored LEN")
                val nlen = br.readLsb(16)
                    ?: throw IOException("deflate: EOF reading stored NLEN")
                // RFC 1951 §3.2.4: NLEN is the one's complement of LEN.
                if ((nlen xor 0xFFFF) != len) {
                    throw IOException("deflate: stored LEN/NLEN mismatch: $len vs $nlen")
                }
                if (out.size + len > MAX_OUTPUT_BYTES) {
                    throw IOException("deflate: output size limit exceeded")
                }
                repeat(len) {
                    val b = br.readLsb(8)
                        ?: throw IOException("deflate: EOF inside stored block data")
                    out.add(b.toByte())
                }
            }
            0b01 -> {
                // ── Fixed Huffman block ───────────────────────────────────────
                while (true) {
                    val sym = fixedLlDecode(br)
                        ?: throw IOException("deflate: EOF decoding fixed Huffman symbol")
                    when {
                        sym in 0..255 -> {
                            // Guard against decompression bombs.
                            if (out.size >= MAX_OUTPUT_BYTES) {
                                throw IOException("deflate: output size limit exceeded")
                            }
                            out.add(sym.toByte())
                        }
                        sym == 256 -> break  // end-of-block
                        sym in 257..285 -> {
                            // Back-reference: decode length, then distance.
                            val idx = sym - 257
                            if (idx >= LENGTH_TABLE.size) {
                                throw IOException("deflate: invalid length sym $sym")
                            }
                            val (baseLen, extraLenBits) = LENGTH_TABLE[idx]
                            val extraLen = if (extraLenBits > 0) {
                                br.readLsb(extraLenBits)
                                    ?: throw IOException("deflate: EOF reading length extra bits")
                            } else 0
                            val matchLen = baseLen + extraLen

                            // Distance code: 5-bit fixed, read MSB-first.
                            val distCode = br.readMsb(5)
                                ?: throw IOException("deflate: EOF reading distance code")
                            if (distCode >= DIST_TABLE.size) {
                                throw IOException("deflate: invalid dist code $distCode")
                            }
                            val (baseDist, extraDistBits) = DIST_TABLE[distCode]
                            val extraDist = if (extraDistBits > 0) {
                                br.readLsb(extraDistBits)
                                    ?: throw IOException("deflate: EOF reading distance extra bits")
                            } else 0
                            val offset = baseDist + extraDist

                            // Bounds check: offset must not exceed decoded output.
                            if (offset > out.size) {
                                throw IOException(
                                    "deflate: back-reference offset $offset > output len ${out.size}"
                                )
                            }
                            if (out.size + matchLen > MAX_OUTPUT_BYTES) {
                                throw IOException("deflate: output size limit exceeded")
                            }
                            // Copy byte-by-byte to handle overlapping matches
                            // (e.g. offset=1, length=10 encodes a run of one byte × 10).
                            repeat(matchLen) {
                                out.add(out[out.size - offset])
                            }
                        }
                        else -> throw IOException("deflate: invalid LL symbol $sym")
                    }
                }
            }
            0b10 -> throw IOException("deflate: dynamic Huffman blocks (BTYPE=10) not supported")
            else -> throw IOException("deflate: reserved BTYPE=11")
        }

        if (bfinal == 1) break
    }

    return out.toByteArray()
}

// =============================================================================
// MS-DOS Date / Time Encoding
// =============================================================================
//
// ZIP stores timestamps in the 16-bit MS-DOS packed format inherited from FAT:
//
//   Time (16-bit): bits 15-11=hours, bits 10-5=minutes, bits 4-0=seconds/2
//   Date (16-bit): bits 15-9=year-1980, bits 8-5=month, bits 4-0=day
//
// The combined 32-bit value is (date << 16) | time.
// Year 0 in DOS time = 1980; max representable = 2107.

/**
 * Encode a (year, month, day, hour, minute, second) tuple into the 32-bit
 * MS-DOS datetime used by ZIP Local and Central Directory headers.
 *
 * Example: dosDt(1980, 1, 1, 0, 0, 0) = 0x00210000
 */
fun dosDt(year: Int, month: Int, day: Int, hour: Int, min: Int, sec: Int): Int {
    val t = (hour shl 11) or (min shl 5) or (sec / 2)
    val d = ((year - 1980).coerceAtLeast(0) shl 9) or (month shl 5) or day
    return (d shl 16) or t
}

// =============================================================================
// ZIP Write — ZipWriter
// =============================================================================
//
// ZipWriter accumulates entries in memory: for each file it writes a Local
// File Header immediately, then the (possibly compressed) data, records the
// metadata needed for the Central Directory, and assembles the full archive
// on finish().
//
// Auto-compression policy:
//   - Try DEFLATE. If the compressed output is smaller than the original,
//     use method=8 (DEFLATE).
//   - Otherwise use method=0 (Stored) — common for already-compressed formats
//     like JPEG, PNG, or ZIP inside ZIP.

/** Metadata recorded per entry during writing, used to build the Central Directory. */
private data class CdRecord(
    val name: ByteArray,
    val method: Short,
    val dosDt: Int,
    val crc: Int,
    val compressedSize: Int,
    val uncompressedSize: Int,
    val localOffset: Int,
    val externalAttrs: Int
)

/**
 * Builds a ZIP archive incrementally in memory.
 *
 * Usage:
 * ```kotlin
 * val w = ZipWriter()
 * w.addFile("hello.txt", "hello, world!".toByteArray())
 * w.addDirectory("mydir/")
 * val bytes = w.finish()
 * // bytes is a valid .zip file
 * ```
 */
class ZipWriter {
    private val buf = mutableListOf<Byte>()
    private val entries = mutableListOf<CdRecord>()

    /**
     * Add a file entry.
     *
     * If [compress] is true, DEFLATE is attempted; the compressed form is used
     * only if it is strictly smaller than the uncompressed original.
     */
    fun addFile(name: String, data: ByteArray, compress: Boolean = true) {
        addEntry(name, data, compress, UNIX_MODE_FILE)
    }

    /**
     * Add a directory entry (name should end with '/').
     * Directory entries have empty data and are always stored.
     */
    fun addDirectory(name: String) {
        addEntry(name, ByteArray(0), compress = false, UNIX_MODE_DIR)
    }

    /**
     * Internal: add any entry (file or directory) with the given Unix mode.
     *
     * Determines compression method, writes the Local File Header to [buf],
     * then records metadata for the Central Directory pass in [finish].
     */
    private fun addEntry(name: String, data: ByteArray, compress: Boolean, unixMode: Int) {
        val nameBytes = name.toByteArray(Charsets.UTF_8)
        val crc = crc32(data)
        val uncompressedSize = data.size

        // Decide compression: try DEFLATE and fall back to Stored if it doesn't help.
        val (method, fileData) = if (compress && data.isNotEmpty()) {
            val compressed = deflateCompress(data)
            if (compressed.size < data.size) {
                Pair(METHOD_DEFLATE, compressed)
            } else {
                Pair(METHOD_STORED, data)
            }
        } else {
            Pair(METHOD_STORED, data)
        }

        val compressedSize = fileData.size
        val localOffset = buf.size
        val versionNeeded = if (method == METHOD_DEFLATE) VERSION_DEFLATE else VERSION_STORED

        // ── Local File Header ─────────────────────────────────────────────────
        // All integers are little-endian per the ZIP specification.
        writeU32(LOCAL_SIG)
        writeU16(versionNeeded.toInt())
        writeU16(FLAG_UTF8.toInt())          // bit 11 = UTF-8 filename
        writeU16(method.toInt())
        writeU16(DOS_EPOCH and 0xFFFF)       // mod_time (low 16 bits of DosEpoch)
        writeU16(DOS_EPOCH ushr 16)          // mod_date (high 16 bits of DosEpoch)
        writeU32(crc)
        writeU32(compressedSize)
        writeU32(uncompressedSize)
        writeU16(nameBytes.size)
        writeU16(0)                          // extra_field_length = 0
        buf.addAll(nameBytes.toList())
        // (no extra field)
        buf.addAll(fileData.toList())

        // Record for Central Directory.
        entries.add(CdRecord(
            name = nameBytes,
            method = method,
            dosDt = DOS_EPOCH,
            crc = crc,
            compressedSize = compressedSize,
            uncompressedSize = uncompressedSize,
            localOffset = localOffset,
            externalAttrs = unixMode shl 16
        ))
    }

    /**
     * Finish writing: append Central Directory and EOCD, return the archive bytes.
     *
     * After this call, the ZipWriter should not be used again — all state is
     * consumed into the returned byte array.
     */
    fun finish(): ByteArray {
        val cdOffset = buf.size
        val numEntries = entries.size

        // ── Central Directory Headers ──────────────────────────────────────────
        // One 46-byte fixed record per entry, followed by the variable-length name.
        val cdStart = buf.size
        for (e in entries) {
            val versionNeeded = if (e.method == METHOD_DEFLATE) VERSION_DEFLATE else VERSION_STORED
            writeU32(CD_SIG)
            writeU16(VERSION_MADE_BY.toInt())
            writeU16(versionNeeded.toInt())
            writeU16(FLAG_UTF8.toInt())
            writeU16(e.method.toInt())
            writeU16(e.dosDt and 0xFFFF)          // mod_time
            writeU16(e.dosDt ushr 16)              // mod_date
            writeU32(e.crc)
            writeU32(e.compressedSize)
            writeU32(e.uncompressedSize)
            writeU16(e.name.size)
            writeU16(0)                            // extra_len = 0
            writeU16(0)                            // comment_len = 0
            writeU16(0)                            // disk_start = 0
            writeU16(0)                            // internal_attrs = 0
            writeU32(e.externalAttrs)
            writeU32(e.localOffset)
            buf.addAll(e.name.toList())
            // (no extra, no comment)
        }
        val cdSize = buf.size - cdStart

        // ── End of Central Directory Record (22 bytes) ─────────────────────────
        writeU32(EOCD_SIG)
        writeU16(0)             // disk_number = 0
        writeU16(0)             // disk_with_cd_start = 0
        writeU16(numEntries)    // entries_on_this_disk
        writeU16(numEntries)    // entries_total
        writeU32(cdSize)        // Central Directory byte size
        writeU32(cdOffset)      // Central Directory byte offset from archive start
        writeU16(0)             // comment_length = 0

        return buf.toByteArray()
    }

    // ── Little-endian write helpers ────────────────────────────────────────────

    private fun writeU16(v: Int) {
        buf.add((v and 0xFF).toByte())
        buf.add(((v ushr 8) and 0xFF).toByte())
    }

    private fun writeU32(v: Int) {
        buf.add((v and 0xFF).toByte())
        buf.add(((v ushr 8) and 0xFF).toByte())
        buf.add(((v ushr 16) and 0xFF).toByte())
        buf.add(((v ushr 24) and 0xFF).toByte())
    }
}

// =============================================================================
// ZIP Read — ZipEntry and ZipReader
// =============================================================================
//
// ZipReader uses the "EOCD-first" strategy for reliable random-access:
//
//   1. Scan backwards for the EOCD signature (PK\x05\x06).
//      Limit the scan to the last 65535 + 22 bytes (EOCD comment max = 65535).
//   2. Read the CD offset and size from EOCD.
//   3. Parse all Central Directory headers into ZipEntry objects.
//   4. On read(name): seek to the Local Header via local_offset, skip
//      the variable-length name + extra fields, read compressed data,
//      decompress, verify CRC-32.
//
// We use CD entries as the authoritative source for sizes and compression
// method. Local headers are only consulted for their variable-length fields
// (name_len + extra_len) so we can skip to the data.

/**
 * A single file or directory entry in a ZIP archive.
 *
 * [name] is UTF-8. For directory entries, [data] is empty and [name] ends with '/'.
 * This matches the C# reference ZipEntry(Name, Data) record design.
 */
data class ZipEntry(val name: String, val data: ByteArray) {
    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is ZipEntry) return false
        return name == other.name && data.contentEquals(other.data)
    }

    override fun hashCode(): Int = 31 * name.hashCode() + data.contentHashCode()
}

/**
 * Internal metadata per entry needed for lazy reads.
 * The public ZipEntry only exposes (name, data); we keep the rest hidden.
 */
private data class ZipEntryMeta(
    val name: String,
    val localOffset: Int,
    val method: Short,
    val crc: Int,
    val compressedSize: Int,
    val uncompressedSize: Int,
    val isDirectory: Boolean
)

/**
 * Reads entries from an in-memory ZIP archive.
 *
 * Usage:
 * ```kotlin
 * val reader = ZipReader(archiveBytes)
 * for (entry in reader.entries) {
 *     println("${entry.name}: ${entry.data.size} bytes")
 * }
 * val data = reader.read("hello.txt")
 * ```
 *
 * Throws [IOException] if no valid EOCD record is found or the archive is
 * structurally malformed.
 */
class ZipReader(private val data: ByteArray) {
    private val meta: List<ZipEntryMeta>

    /**
     * All entries in the archive (files and directories) in Central Directory order.
     * The [ZipEntry.data] field is empty — call [read] to get the actual content.
     */
    val entries: List<ZipEntry>

    init {
        val eocdOffset = findEocd(data)
            ?: throw IOException("zip: no End of Central Directory record found")

        // Read EOCD fields: cd_size at +12, cd_offset at +16.
        val cdOffset = readU32(data, eocdOffset + 16)
        val cdSize = readU32(data, eocdOffset + 12)

        if (cdOffset + cdSize > data.size) {
            throw IOException(
                "zip: Central Directory [$cdOffset, ${cdOffset + cdSize}) out of bounds (file size ${data.size})"
            )
        }

        // Parse Central Directory headers.
        val parsedMeta = mutableListOf<ZipEntryMeta>()
        var pos = cdOffset
        while (pos + 4 <= cdOffset + cdSize) {
            val sig = readU32(data, pos)
            if (sig != CD_SIG) break  // end of CD or padding

            val method = readU16(data, pos + 10).toShort()
            val crc = readU32(data, pos + 16)
            val compressedSize = readU32(data, pos + 20)
            val uncompressedSize = readU32(data, pos + 24)
            val nameLen = readU16(data, pos + 28)
            val extraLen = readU16(data, pos + 30)
            val commentLen = readU16(data, pos + 32)
            val localOffset = readU32(data, pos + 42)

            val nameStart = pos + 46
            val nameEnd = nameStart + nameLen
            if (nameEnd > data.size) {
                throw IOException("zip: CD entry name out of bounds")
            }
            val name = String(data, nameStart, nameLen, Charsets.UTF_8)

            parsedMeta.add(ZipEntryMeta(
                name = name,
                localOffset = localOffset,
                method = method,
                crc = crc,
                compressedSize = compressedSize,
                uncompressedSize = uncompressedSize,
                isDirectory = name.endsWith('/')
            ))

            pos = nameEnd + extraLen + commentLen
        }

        meta = parsedMeta
        // Build the public entries list (name only; data read on demand).
        entries = meta.map { ZipEntry(it.name, ByteArray(0)) }
    }

    /**
     * Decompress and return the data for the named entry. Verifies CRC-32.
     *
     * Throws [IOException] on CRC mismatch, unsupported method, or corrupt data.
     */
    fun read(name: String): ByteArray {
        val m = meta.find { it.name == name }
            ?: throw IOException("zip: entry '$name' not found")
        return readEntry(m)
    }

    /**
     * Internal: read and decompress one entry using its local_offset.
     *
     * Uses the Central Directory as the authoritative source for compressed/
     * uncompressed sizes and method. The Local Header is consulted only for
     * the variable-length name_len + extra_len to skip to the file data.
     */
    private fun readEntry(m: ZipEntryMeta): ByteArray {
        if (m.isDirectory) return ByteArray(0)

        val lhOff = m.localOffset

        // Reject encrypted entries (GP flag bit 0 = 1).
        val localFlags = readU16(data, lhOff + 6)
        if (localFlags and 1 != 0) {
            throw IOException("zip: entry '${m.name}' is encrypted; not supported")
        }

        // The Local Header name_len and extra_len can differ from the CD header,
        // so we must re-read them to find the actual start of the file data.
        val lhNameLen = readU16(data, lhOff + 26)
        val lhExtraLen = readU16(data, lhOff + 28)
        val dataStart = lhOff + 30 + lhNameLen + lhExtraLen
        val dataEnd = dataStart + m.compressedSize

        if (dataEnd > data.size) {
            throw IOException(
                "zip: entry '${m.name}' data [$dataStart, $dataEnd) out of bounds"
            )
        }

        val compressed = data.copyOfRange(dataStart, dataEnd)

        // Decompress according to method.
        val decompressed: ByteArray = when (m.method.toInt()) {
            0 -> compressed                         // Stored — verbatim copy
            8 -> deflateDecompress(compressed)      // DEFLATE
            else -> throw IOException(
                "zip: unsupported compression method ${m.method} for '${m.name}'"
            )
        }

        // Trim to declared uncompressed size (guards against decompressor over-read).
        val trimmed = if (decompressed.size > m.uncompressedSize) {
            decompressed.copyOf(m.uncompressedSize)
        } else {
            decompressed
        }

        // Verify CRC-32 — detects corruption of the decompressed content.
        val actualCrc = crc32(trimmed)
        if (actualCrc != m.crc) {
            throw IOException(
                "zip: CRC-32 mismatch for '${m.name}': expected ${Integer.toHexString(m.crc).uppercase()}, " +
                "got ${Integer.toHexString(actualCrc).uppercase()}"
            )
        }

        return trimmed
    }

    // ── Private helpers ────────────────────────────────────────────────────────

    companion object {
        /**
         * Scan backwards from the end of [data] for the EOCD signature 0x06054B50.
         *
         * The EOCD record is at most 22 + 65535 bytes from the end (the comment
         * field can be 0–65535 bytes). We limit the scan to prevent unbounded
         * searches on malformed archives.
         *
         * The validation step (comment_len must exactly account for the remaining
         * bytes) prevents false positives when the EOCD signature bytes happen to
         * appear inside compressed file data.
         */
        private fun findEocd(data: ByteArray): Int? {
            val eocdMinSize = 22
            val maxComment = 65535

            if (data.size < eocdMinSize) return null

            val scanStart = (data.size - eocdMinSize - maxComment).coerceAtLeast(0)

            // Scan from end backwards.
            for (i in (data.size - eocdMinSize) downTo scanStart) {
                if (readU32(data, i) == EOCD_SIG) {
                    // Validate: comment_len at offset +20 must account for all remaining bytes.
                    val commentLen = readU16(data, i + 20)
                    if (i + eocdMinSize + commentLen == data.size) {
                        return i
                    }
                }
            }
            return null
        }

        /** Read a little-endian u16 from [data] at [offset]. */
        private fun readU16(data: ByteArray, offset: Int): Int {
            return (data[offset].toInt() and 0xFF) or
                   ((data[offset + 1].toInt() and 0xFF) shl 8)
        }

        /** Read a little-endian u32 from [data] at [offset]. Returns as Int (reinterpret). */
        private fun readU32(data: ByteArray, offset: Int): Int {
            return (data[offset].toInt() and 0xFF) or
                   ((data[offset + 1].toInt() and 0xFF) shl 8) or
                   ((data[offset + 2].toInt() and 0xFF) shl 16) or
                   ((data[offset + 3].toInt() and 0xFF) shl 24)
        }
    }
}

// =============================================================================
// Convenience API — ZipArchive
// =============================================================================

/**
 * Convenience functions for one-shot ZIP archive creation and extraction.
 *
 * These wrap [ZipWriter] and [ZipReader] for callers that don't need
 * incremental control.
 */
object ZipArchive {
    /**
     * Compress a list of [ZipEntry] objects into a ZIP archive.
     *
     * Each entry is compressed with DEFLATE if that reduces size; otherwise stored.
     * Directory entries (names ending with '/') are stored verbatim.
     */
    fun zip(entries: List<ZipEntry>): ByteArray {
        val writer = ZipWriter()
        for (entry in entries) {
            if (entry.name.endsWith('/')) {
                writer.addDirectory(entry.name)
            } else {
                writer.addFile(entry.name, entry.data)
            }
        }
        return writer.finish()
    }

    /**
     * Extract all entries from a ZIP archive.
     *
     * Returns all entries (files and directories) in Central Directory order.
     * File entries have their data decompressed; directory entries have empty data.
     * Throws [IOException] on corrupt archives.
     */
    fun unzip(data: ByteArray): List<ZipEntry> {
        val reader = ZipReader(data)
        return reader.entries.map { entry ->
            if (entry.name.endsWith('/')) {
                ZipEntry(entry.name, ByteArray(0))
            } else {
                ZipEntry(entry.name, reader.read(entry.name))
            }
        }
    }
}
