// ============================================================================
// LZ77.kt — CMP00: LZ77 Lossless Compression Algorithm (1977)
// ============================================================================
//
// LZ77 is the foundational sliding-window compression algorithm published by
// Abraham Lempel and Jacob Ziv in 1977.  It is the direct ancestor of LZSS,
// LZW, DEFLATE, zstd, LZ4, and virtually every modern compressor.
//
// The core idea: instead of storing every byte verbatim, notice when a
// sequence of bytes has appeared recently.  Replace that sequence with a
// cheap back-reference: (offset, length).  This exploits locality of real
// data — repeated words, duplicate instructions, adjacent colour runs.
//
// ============================================================================
// The Sliding Window Model
// ============================================================================
//
//   ┌─────────────────────────────────┬──────────────────┐
//   │         SEARCH BUFFER           │ LOOKAHEAD BUFFER  │
//   │  (last window_size bytes)       │  (next max_match) │
//   └─────────────────────────────────┴──────────────────┘
//                                     ↑ cursor
//
// ============================================================================
// Token: (offset, length, nextChar)
// ============================================================================
//
//   offset=0, length=0 → pure literal; payload is nextChar.
//   offset>0, length>0 → back-reference: copy length bytes from offset
//                         positions back, then emit nextChar.
//
// ============================================================================
// Overlapping Matches
// ============================================================================
//
// A match may extend into output bytes not yet written (offset < length).
// This naturally encodes runs.  The decoder must copy byte-by-byte.
//
// ============================================================================
// Wire Format (CMP00)
// ============================================================================
//
//   Bytes 0–3:    token_count  (big-endian uint32)
//   Bytes 4+:     token_count × 4 bytes:
//                   [0–1] offset    (big-endian uint16)
//                   [2]   length    (uint8)
//                   [3]   next_char (uint8)
//
// ============================================================================
// The CMP Series
// ============================================================================
//
//   CMP00 (LZ77,    1977) — Sliding-window back-references.  (this)
//   CMP01 (LZ78,    1978) — Explicit dictionary (trie), no sliding window.
//   CMP02 (LZSS,    1982) — LZ77 + flag bits; eliminates wasted literals.
//   CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; powers GIF.
//   CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
//   CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.
//
// ============================================================================

package com.codingadventures.lz77

import java.nio.ByteBuffer
import java.nio.ByteOrder

// ============================================================================
// Token
// ============================================================================

/**
 * A single LZ77 token: (offset, length, nextChar).
 *
 * - `offset=0, length=0` → pure literal; payload is [nextChar].
 * - `offset>0, length>0` → back-reference: copy [length] bytes from
 *   [offset] positions back in the output, then emit [nextChar].
 *
 * @param offset   distance back the match starts (1..windowSize), or 0
 * @param length   number of matched bytes (0 = no match)
 * @param nextChar literal byte after the match (0–255)
 */
data class Token(val offset: Int, val length: Int, val nextChar: Int) {

    /** True if this token is a pure literal (no back-reference). */
    val isLiteral: Boolean get() = length == 0

    companion object {
        /** Create a literal token for a single byte value. */
        fun literal(b: Int) = Token(0, 0, b and 0xFF)

        /** Create a back-reference token. */
        fun match(offset: Int, length: Int, nextChar: Int) =
            Token(offset, length, nextChar and 0xFF)
    }
}

// ============================================================================
// LZ77
// ============================================================================

/**
 * CMP00: LZ77 lossless compression.
 *
 * ```kotlin
 * val original   = "hello hello hello".toByteArray()
 * val compressed = LZ77.compress(original)
 * val recovered  = LZ77.decompress(compressed)
 * check(original.contentEquals(recovered))
 * ```
 */
object LZ77 {

    // =========================================================================
    // Constants
    // =========================================================================

    /** Default sliding-window size (maximum back-reference distance). */
    const val DEFAULT_WINDOW_SIZE = 4096

    /** Default maximum match length (fits in one uint8). */
    const val DEFAULT_MAX_MATCH = 255

    /**
     * Default minimum match length for a back-reference to be emitted.
     *
     * A match of length 3 costs 4 bytes (offset=2 + length=1 + next_char=1).
     * Three literals each as tokens also cost 4 bytes each in this format, so
     * 3 is the natural break-even point.
     */
    const val DEFAULT_MIN_MATCH = 3

    private const val HEADER_SIZE = 4
    private const val TOKEN_SIZE  = 4

    // =========================================================================
    // Encoding
    // =========================================================================

    /**
     * Encode [data] into an LZ77 token stream using default parameters.
     *
     * @param data the bytes to encode
     * @return token stream suitable for [decode]
     */
    fun encode(data: ByteArray): List<Token> =
        encode(data, DEFAULT_WINDOW_SIZE, DEFAULT_MAX_MATCH, DEFAULT_MIN_MATCH)

    /**
     * Encode [data] into an LZ77 token stream with custom parameters.
     *
     * Greedy O(n × window) algorithm:
     * 1. For each cursor, scan the last [windowSize] bytes for the longest
     *    match of the lookahead.
     * 2. If best match length ≥ [minMatch], emit a back-reference token and
     *    advance cursor by `length + 1`.
     * 3. Otherwise emit a literal token and advance by 1.
     *
     * One byte is always reserved for nextChar, so the lookahead extends at
     * most to `data.size - 1`.
     */
    fun encode(
        data: ByteArray,
        windowSize: Int = DEFAULT_WINDOW_SIZE,
        maxMatch:   Int = DEFAULT_MAX_MATCH,
        minMatch:   Int = DEFAULT_MIN_MATCH
    ): List<Token> {
        val tokens  = mutableListOf<Token>()
        var cursor  = 0

        while (cursor < data.size) {
            val (bestOffset, bestLength) = findLongestMatch(data, cursor, windowSize, maxMatch)

            if (bestLength >= minMatch) {
                val nextChar = data[cursor + bestLength].toInt() and 0xFF
                tokens.add(Token.match(bestOffset, bestLength, nextChar))
                cursor += bestLength + 1
            } else {
                tokens.add(Token.literal(data[cursor].toInt() and 0xFF))
                cursor += 1
            }
        }
        return tokens
    }

    // =========================================================================
    // Decoding
    // =========================================================================

    /**
     * Decode an LZ77 token stream back into the original bytes.
     *
     * For each token:
     * - If `length > 0`: copy [Token.length] bytes one-at-a-time from position
     *   `output.size - offset`.  Byte-by-byte copy handles overlapping matches.
     * - Always append [Token.nextChar].
     */
    fun decode(tokens: List<Token>, initialBuffer: ByteArray = ByteArray(0)): ByteArray {
        // Security / performance: the previous implementation re-read
        // out.toByteArray() (a full copy of the output) for every copied byte,
        // which is quadratic in the output size.  Keep a plain growable array
        // so a back-reference copy is O(length).
        var buf = initialBuffer.copyOf(maxOf(initialBuffer.size, 16))
        var len = initialBuffer.size

        for (token in tokens) {
            if (token.length > 0) {
                // Guard: a zero offset, or one reaching before the start of the
                // output, is invalid (corrupt or hostile input).
                require(token.offset in 1..len) {
                    "LZ77: back-reference offset ${token.offset} is invalid for output buffer size $len"
                }
                buf = ensureCapacity(buf, len, token.length + 1)
                val startPos = len - token.offset
                // Byte-by-byte: overlapping matches read bytes written earlier
                // in this same loop.
                for (i in 0 until token.length) {
                    buf[len++] = buf[startPos + i]
                }
            } else {
                buf = ensureCapacity(buf, len, 1)
            }
            buf[len++] = token.nextChar.toByte()
        }
        return buf.copyOf(len)
    }

    /** Grow [buf] (doubling) so that [extra] more bytes fit after [len]. */
    private fun ensureCapacity(buf: ByteArray, len: Int, extra: Int): ByteArray {
        val needed = len.toLong() + extra
        if (needed <= buf.size) return buf
        require(needed <= Int.MAX_VALUE - 8) { "LZ77: decoded output too large" }
        val grown = maxOf(needed, minOf(buf.size.toLong() * 2, (Int.MAX_VALUE - 8).toLong()))
        return buf.copyOf(grown.toInt())
    }

    // =========================================================================
    // One-shot compress / decompress
    // =========================================================================

    /**
     * Compress [data] using LZ77 and return CMP00 wire-format bytes.
     *
     * @param data bytes to compress (null treated as empty)
     */
    fun compress(
        data:       ByteArray?,
        windowSize: Int = DEFAULT_WINDOW_SIZE,
        maxMatch:   Int = DEFAULT_MAX_MATCH,
        minMatch:   Int = DEFAULT_MIN_MATCH
    ): ByteArray {
        val bytes  = data ?: ByteArray(0)
        val tokens = encode(bytes, windowSize, maxMatch, minMatch)
        return serialiseTokens(tokens)
    }

    /**
     * Decompress CMP00 wire-format bytes back to the original data.
     *
     * @param data compressed bytes (null treated as empty)
     */
    fun decompress(data: ByteArray?): ByteArray {
        if (data == null || data.size < HEADER_SIZE) return ByteArray(0)
        val tokens = deserialiseTokens(data)
        return decode(tokens)
    }

    // =========================================================================
    // Serialisation helpers
    // =========================================================================

    /**
     * Serialise a token list to the CMP00 wire format.
     *
     * `[token_count (4B BE)] [N × (offset BE uint16, length uint8, nextChar uint8)]`
     */
    fun serialiseTokens(tokens: List<Token>): ByteArray {
        val buf = ByteBuffer.allocate(HEADER_SIZE + tokens.size * TOKEN_SIZE)
            .order(ByteOrder.BIG_ENDIAN)
        buf.putInt(tokens.size)
        for (t in tokens) {
            buf.putShort(t.offset.toShort())
            buf.put(t.length.toByte())
            buf.put(t.nextChar.toByte())
        }
        return buf.array()
    }

    /**
     * Deserialise CMP00 wire-format bytes into a token list.
     */
    fun deserialiseTokens(data: ByteArray): List<Token> {
        if (data.size < HEADER_SIZE) return emptyList()
        val buf = ByteBuffer.wrap(data).order(ByteOrder.BIG_ENDIAN)
        // Security: the header count is an unsigned 32-bit attacker-controlled
        // value.  Read it unsigned (a raw Int goes negative above 2^31, which
        // silently decoded nothing) and cap it against the payload size so
        // iteration never follows the claim.
        val tokenCount = buf.getInt().toLong() and 0xFFFFFFFFL
        val maxTokens  = (data.size - HEADER_SIZE) / TOKEN_SIZE
        val safeCount  = minOf(tokenCount, maxTokens.toLong()).toInt()

        return buildList {
            repeat(safeCount) {
                if (buf.remaining() >= TOKEN_SIZE) {
                    val offset   = buf.getShort().toInt() and 0xFFFF
                    val length   = buf.get().toInt()      and 0xFF
                    val nextChar = buf.get().toInt()      and 0xFF
                    add(Token(offset, length, nextChar))
                }
            }
        }
    }

    // =========================================================================
    // Private helpers
    // =========================================================================

    /**
     * Find the longest match for `data[cursor:]` in the search buffer.
     *
     * Returns `Pair(bestOffset, bestLength)` where both are 0 if no match
     * meeting [minMatch] length was found.  One byte is always reserved for
     * nextChar, so lookahead ends at `data.size - 1`.
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
        val lookaheadEnd = minOf(cursor + maxMatch, data.size - 1)

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
