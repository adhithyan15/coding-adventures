// ============================================================================
// LZSS.kt — CMP02: LZSS Lossless Compression Algorithm (1982)
// ============================================================================
//
// LZSS (Lempel-Ziv-Storer-Szymanski, 1982) is a refinement of LZ77 (CMP00)
// that eliminates the wasted `next_char` byte present in every LZ77 token.
// Instead of the fixed (offset, length, next_char) triple, LZSS uses a
// flag-bit scheme: each symbol is either a bare literal byte or a bare
// back-reference — never both.
//
// ============================================================================
// The Improvement Over LZ77
// ============================================================================
//
//   LZ77:  every token = 4 bytes  (offset=2 + length=1 + next_char=1)
//   LZSS:  literal   = 1 byte     (just the byte value)
//           match    = 3 bytes     (offset=2 + length=1)
//
// A flag byte precedes every group of 8 symbols to tell the decoder which are
// literals (bit=0) and which are matches (bit=1).
//
// ============================================================================
// Break-Even Point
// ============================================================================
//
// A match costs 3 bytes.  Three literals also cost 3 bytes (1 each).  So a
// match of length ≥ 3 breaks even or saves space — `minMatch=3` is standard.
//
// ============================================================================
// Overlapping Matches
// ============================================================================
//
// LZSS inherits LZ77's self-referential matches (offset < length).  For
// example, a literal 'A' followed by Match(offset=1, length=6) decodes to
// "AAAAAAA".  Decoding must copy byte-by-byte, not via a bulk memmove.
//
// ============================================================================
// Wire Format (CMP02)
// ============================================================================
//
//   Bytes 0–3:  original_length  (big-endian uint32)
//   Bytes 4–7:  block_count      (big-endian uint32)
//   Bytes 8+:   blocks
//
//   Each block:
//     [1 byte]    flag_byte (bit i → 0=Literal, 1=Match for symbol i)
//     [variable]  symbol data:
//         Literal: 1 byte  (byte value)
//         Match:   3 bytes (offset big-endian uint16 + length uint8)
//
// `original_length` is stored because LZSS has no sentinel — the decoder
// needs the exact count to know when to stop.
//
// ============================================================================
// The CMP Series
// ============================================================================
//
//   CMP00 (LZ77,    1977) — Sliding-window back-references.
//   CMP01 (LZ78,    1978) — Explicit dictionary (trie), no sliding window.
//   CMP02 (LZSS,    1982) — LZ77 + flag bits; eliminates wasted literals. (this)
//   CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; powers GIF.
//   CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
//   CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.
//
// ============================================================================
// References
// ============================================================================
//
//   Storer, J.A., & Szymanski, T.G. (1982). "Data Compression via Textual
//   Substitution". Journal of the ACM, 29(4), 928–951.
//
//   Lempel, A., & Ziv, J. (1977). "A Universal Algorithm for Sequential Data
//   Compression". IEEE Transactions on Information Theory, 23(3), 337–343.
//
// ============================================================================

package com.codingadventures.lzss

import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import java.nio.ByteOrder

// ============================================================================
// Token types
// ============================================================================

/**
 * A single LZSS token — either a bare literal byte or a bare back-reference.
 *
 * LZSS's key insight is that every symbol is *either* a literal *or* a match,
 * never both.  The type is signalled by a dedicated flag bit — one flag byte
 * covers 8 symbols — so neither representation wastes a next_char field.
 */
sealed interface Token {
    /** True if this token is a literal (no back-reference). */
    val isLiteral: Boolean
}

/**
 * A single literal byte in the LZSS token stream.
 *
 * @param value the byte value (0–255)
 */
data class Literal(val value: Int) : Token {
    override val isLiteral: Boolean get() = true
}

/**
 * A back-reference match in the LZSS token stream.
 *
 * Matches may overlap (offset < length), encoding run-length behaviour.
 * For example, Literal('A') + Match(offset=1, length=6) → "AAAAAAA".
 *
 * @param offset distance back in the output where the match begins (1..window_size)
 * @param length number of bytes to copy (min_match..max_match)
 */
data class Match(val offset: Int, val length: Int) : Token {
    override val isLiteral: Boolean get() = false
}

// ============================================================================
// LZSS
// ============================================================================

/**
 * CMP02: LZSS lossless compression.
 *
 * ```kotlin
 * val original   = "hello hello hello".toByteArray()
 * val compressed = LZSS.compress(original)
 * val recovered  = LZSS.decompress(compressed)
 * check(original.contentEquals(recovered))
 * ```
 */
object LZSS {

    // =========================================================================
    // Constants
    // =========================================================================

    /** Default sliding-window size (maximum back-reference distance). */
    const val DEFAULT_WINDOW_SIZE = 4096

    /** Default maximum match length (fits in uint8). */
    const val DEFAULT_MAX_MATCH = 255

    /**
     * Default minimum match length for a back-reference to be worthwhile.
     *
     * Break-even analysis: a Match costs 3 bytes (offset=2 + length=1), and
     * three Literals also cost 3 bytes (1 each).  So `minMatch=3` is optimal.
     */
    const val DEFAULT_MIN_MATCH = 3

    /** Symbols per flag-byte group. */
    private const val BLOCK_SIZE = 8

    /** Wire-format header size: original_length(4) + block_count(4). */
    private const val HEADER_SIZE = 8

    // =========================================================================
    // Encoding
    // =========================================================================

    /**
     * Encode [data] into an LZSS token stream using default parameters.
     *
     * @param data the bytes to encode
     * @return token stream (mix of [Literal] and [Match])
     */
    fun encode(data: ByteArray): List<Token> =
        encode(data, DEFAULT_WINDOW_SIZE, DEFAULT_MAX_MATCH, DEFAULT_MIN_MATCH)

    /**
     * Encode [data] into an LZSS token stream with custom parameters.
     *
     * Key difference from LZ77: on a match, the cursor advances by exactly
     * [length] (not `length + 1`).  There is no trailing next_char field.
     *
     * @param data       the bytes to encode
     * @param windowSize maximum lookback distance
     * @param maxMatch   maximum match length
     * @param minMatch   minimum match length for a [Match] token
     * @return list of [Literal] and [Match] tokens
     */
    fun encode(
        data:       ByteArray,
        windowSize: Int = DEFAULT_WINDOW_SIZE,
        maxMatch:   Int = DEFAULT_MAX_MATCH,
        minMatch:   Int = DEFAULT_MIN_MATCH
    ): List<Token> {
        val tokens = mutableListOf<Token>()
        var cursor = 0

        while (cursor < data.size) {
            val (bestOffset, bestLength) = findLongestMatch(data, cursor, windowSize, maxMatch)

            if (bestLength >= minMatch) {
                // Emit bare back-reference; cursor advances by length only.
                tokens.add(Match(bestOffset, bestLength))
                cursor += bestLength
            } else {
                // Emit bare literal; cursor advances by 1.
                tokens.add(Literal(data[cursor].toInt() and 0xFF))
                cursor += 1
            }
        }

        return tokens
    }

    // =========================================================================
    // Decoding
    // =========================================================================

    /**
     * Decode an LZSS token stream back into the original bytes.
     *
     * Processes each token:
     * - [Literal]: append the byte to output.
     * - [Match]: copy [Match.length] bytes **one-at-a-time** from
     *   `output.size - offset`.  Byte-by-byte copy handles overlapping
     *   matches where the source overlaps the destination being written.
     *
     * @param tokens         the token stream from [encode]
     * @param originalLength if ≥ 0, truncate to this length; pass -1 for no truncation
     * @return the reconstructed bytes
     */
    fun decode(tokens: List<Token>, originalLength: Int = -1): ByteArray {
        val out = ByteArrayOutputStream()

        for (token in tokens) {
            when (token) {
                is Literal -> out.write(token.value)
                is Match   -> {
                    val startPos = out.size() - token.offset
                    // Guard: offset must not exceed what has already been written.
                    // A crafted stream with offset > out.size() produces a negative
                    // startPos and causes ArrayIndexOutOfBoundsException.
                    require(startPos >= 0) {
                        "LZSS: back-reference offset ${token.offset} exceeds output buffer size ${out.size()}"
                    }
                    repeat(token.length) { i ->
                        // Re-read live output each step: handles overlapping matches
                        // where the source overlaps the destination being written.
                        out.write(out.toByteArray()[startPos + i].toInt() and 0xFF)
                    }
                }
            }
        }

        val result = out.toByteArray()
        return if (originalLength in 0 until result.size) result.copyOf(originalLength) else result
    }

    // =========================================================================
    // One-shot compress / decompress
    // =========================================================================

    /**
     * Compress [data] using LZSS and return CMP02 wire-format bytes.
     *
     * @param data the bytes to compress (null treated as empty)
     * @return compressed bytes in CMP02 wire format
     */
    fun compress(
        data:       ByteArray?,
        windowSize: Int = DEFAULT_WINDOW_SIZE,
        maxMatch:   Int = DEFAULT_MAX_MATCH,
        minMatch:   Int = DEFAULT_MIN_MATCH
    ): ByteArray {
        val bytes  = data ?: ByteArray(0)
        val tokens = encode(bytes, windowSize, maxMatch, minMatch)
        return serialiseTokens(tokens, bytes.size)
    }

    /**
     * Decompress CMP02 wire-format bytes back to the original data.
     *
     * @param data compressed bytes (null treated as empty)
     * @return the original, uncompressed bytes
     */
    fun decompress(data: ByteArray?): ByteArray {
        if (data == null || data.size < HEADER_SIZE) return ByteArray(0)
        val buf             = ByteBuffer.wrap(data, 0, HEADER_SIZE).order(ByteOrder.BIG_ENDIAN)
        val originalLength  = buf.getInt()
        val tokens          = deserialiseTokens(data)
        return decode(tokens, originalLength)
    }

    // =========================================================================
    // Serialisation helpers
    // =========================================================================

    /**
     * Serialise an LZSS token list to the CMP02 wire format.
     *
     * Groups tokens into blocks of up to 8.  Each block is preceded by a
     * flag byte (bit i → 0=Literal, 1=Match), followed by the token data:
     * - Literal: 1 byte
     * - Match: 3 bytes (offset big-endian uint16 + length uint8)
     *
     * @param tokens         the token list
     * @param originalLength length of the original uncompressed data
     * @return the CMP02 wire-format bytes
     */
    fun serialiseTokens(tokens: List<Token>, originalLength: Int): ByteArray {
        val blocks = mutableListOf<ByteArray>()
        var i = 0

        while (i < tokens.size) {
            val chunk     = tokens.subList(i, minOf(i + BLOCK_SIZE, tokens.size))
            var flag      = 0
            val blockData = ByteArrayOutputStream()

            for ((bit, tok) in chunk.withIndex()) {
                when (tok) {
                    is Match -> {
                        flag = flag or (1 shl bit)
                        blockData.write(tok.offset shr 8)          // offset high byte
                        blockData.write(tok.offset and 0xFF)       // offset low byte
                        blockData.write(tok.length)                // length
                    }
                    is Literal -> blockData.write(tok.value)
                }
            }

            val block = ByteArrayOutputStream()
            block.write(flag)
            block.write(blockData.toByteArray())
            blocks.add(block.toByteArray())
            i += BLOCK_SIZE
        }

        // Header: original_length(4B) + block_count(4B)
        val header = ByteBuffer.allocate(HEADER_SIZE).order(ByteOrder.BIG_ENDIAN)
        header.putInt(originalLength).putInt(blocks.size)

        val out = ByteArrayOutputStream()
        out.write(header.array())
        for (b in blocks) out.write(b)
        return out.toByteArray()
    }

    /**
     * Deserialise CMP02 wire-format bytes into a token list.
     *
     * @param data the CMP02 bytes
     * @return list of [Literal] and [Match] tokens
     */
    fun deserialiseTokens(data: ByteArray): List<Token> {
        if (data.size < HEADER_SIZE) return emptyList()

        val header     = ByteBuffer.wrap(data, 0, HEADER_SIZE).order(ByteOrder.BIG_ENDIAN)
        /* originalLength = */ header.getInt()
        val blockCount = header.getInt()

        // Cap block count against actual payload to prevent DoS.
        val safeCount = minOf(blockCount, data.size - HEADER_SIZE)
        val tokens    = mutableListOf<Token>()
        var pos       = HEADER_SIZE

        for (b in 0 until safeCount) {
            if (pos >= data.size) break
            val flag = data[pos++].toInt() and 0xFF

            for (bit in 0 until BLOCK_SIZE) {
                if (pos >= data.size) break
                if (flag and (1 shl bit) != 0) {
                    // Match: 3 bytes
                    if (pos + 3 > data.size) break
                    val offset = ((data[pos].toInt() and 0xFF) shl 8) or (data[pos + 1].toInt() and 0xFF)
                    val length = data[pos + 2].toInt() and 0xFF
                    tokens.add(Match(offset, length))
                    pos += 3
                } else {
                    // Literal: 1 byte
                    tokens.add(Literal(data[pos++].toInt() and 0xFF))
                }
            }
        }

        return tokens
    }

    // =========================================================================
    // Private helpers
    // =========================================================================

    /**
     * Find the longest match for `data[cursor:]` in the search buffer.
     *
     * Unlike LZ77's implementation, the lookahead may extend all the way to
     * `data.size` — there is no next_char reservation in LZSS.
     *
     * @param data       full input bytes
     * @param cursor     current encode position
     * @param windowSize maximum lookback distance
     * @param maxMatch   maximum match length
     * @return `Pair(bestOffset, bestLength)`; both 0 if no match found
     */
    private fun findLongestMatch(
        data:       ByteArray,
        cursor:     Int,
        windowSize: Int,
        maxMatch:   Int
    ): Pair<Int, Int> {
        var bestOffset = 0
        var bestLength = 0

        val searchStart  = maxOf(0, cursor - windowSize)
        // No next_char reservation: lookahead extends to end of input.
        val lookaheadEnd = minOf(cursor + maxMatch, data.size)

        for (pos in searchStart until cursor) {
            var length = 0
            while (cursor + length < lookaheadEnd &&
                   data[pos + length] == data[cursor + length]) {
                length++
            }
            if (length > bestLength) {
                bestLength = length
                bestOffset = cursor - pos
            }
        }

        return Pair(bestOffset, bestLength)
    }
}
