/**
 * LZSS lossless compression algorithm (1982) — CMP02.
 *
 * LZSS (Lempel-Ziv-Storer-Szymanski) refines the classic LZ77 algorithm from
 * 1977. LZ77 unconditionally emitted `(offset, length, next_char)` triples,
 * which wastes space when no back-reference exists. LZSS solves this with a
 * **flag-bit scheme**: each group of up to 8 tokens is preceded by a single
 * "flag byte" where each bit marks whether the corresponding token is a
 * back-reference Match (bit=1) or a raw Literal byte (bit=0).
 *
 * Result:
 *  - A Literal costs just 1 byte  (flag bit + 1 payload byte shared across 8 tokens)
 *  - A Match  costs just 3 bytes  (offset u16 BE + length u8)
 *
 * Break-even: a match is worthwhile only when its length is ≥ 3 bytes.
 *
 * ## Compression series
 * ```
 * CMP00 (LZ77,    1977) — Sliding-window back-references.
 * CMP01 (LZ78,    1978) — Explicit dictionary (trie).
 * CMP02 (LZSS,    1982) — LZ77 + flag bits.  ← this package
 * CMP03 (LZW,     1984) — LZ78 + pre-initialised alphabet; used in GIF.
 * CMP04 (Huffman, 1952) — Entropy coding.
 * CMP05 (DEFLATE, 1996) — LZ77 + Huffman; used in ZIP / gzip / PNG / zlib.
 * ```
 *
 * ## Wire format (CMP02)
 * ```
 * Bytes 0–3  : original_length  (big-endian uint32)
 * Bytes 4–7  : block_count      (big-endian uint32)
 * Bytes 8+   : blocks
 *   Each block: [1-byte flag] [symbol data]
 *     flag bit i = 0 → token i is a Literal (1 payload byte)
 *     flag bit i = 1 → token i is a Match   (2-byte offset BE + 1-byte length)
 * ```
 *
 * The flag byte's **bit 0** corresponds to the **first** token in the block,
 * matching the Rust reference implementation.
 *
 * ## Example
 * ```kotlin
 * val original = "hello hello hello".encodeToByteArray()
 * val compressed = Lzss.compress(original)
 * val restored   = Lzss.decompress(compressed)
 * assert(original.contentEquals(restored))
 * ```
 */
package com.codingadventures.lzss

import java.nio.ByteBuffer
import java.nio.ByteOrder

// ─── Token ADT ───────────────────────────────────────────────────────────────

/**
 * A single LZSS token.
 *
 * The sealed class hierarchy lets the compiler enforce exhaustive `when`
 * expressions — if we add a new token type later, every `when` site will
 * fail to compile until it handles it.
 *
 * Think of this as a tiny algebraic data type (ADT) from functional
 * programming: a token is *either* a Literal *or* a Match — never both.
 */
sealed class LzssToken {

    /**
     * A raw byte that had no useful back-reference in the search window.
     *
     * Costs 1 byte on the wire (plus 1/8 of the shared flag byte).
     */
    data class Literal(val value: Byte) : LzssToken()

    /**
     * A back-reference into the already-decoded output.
     *
     * - [offset]: how many bytes back the match starts (1 = previous byte,
     *   2 = two bytes ago, …).  Must be ≥ 1 and ≤ windowSize.
     * - [length]: how many bytes to copy.  Must be ≥ minMatch (default 3).
     *
     * Overlapping matches are legal and used for run-length encoding.
     * For example, offset=1 length=6 starting from 'A' yields "AAAAAAA".
     *
     * Costs 3 bytes on the wire (2-byte offset + 1-byte length, plus 1/8
     * of the shared flag byte — still a net saving vs. 3+ literals).
     */
    data class Match(val offset: Int, val length: Int) : LzssToken()
}

// ─── Constants ───────────────────────────────────────────────────────────────

/**
 * Default sliding-window size (4 096 bytes = 12 address bits).
 *
 * A larger window finds more matches but requires a longer offset field.
 * This value matches the CMP02 spec and the Rust reference implementation.
 */
const val DEFAULT_WINDOW_SIZE = 4096

/**
 * Default maximum match length (255 = max value of a single byte).
 *
 * Capped at 255 so the length fits in a `uint8` on the wire.
 */
const val DEFAULT_MAX_MATCH = 255

/**
 * Default minimum match length.
 *
 * Cost analysis:
 *  - Match token wire cost: 3 bytes  (2-byte offset + 1-byte length)
 *  - Literal token wire cost: 1 byte each
 *
 * A match of length 3 costs the same as 3 literals, but actually saves
 * because the flag bits are amortised across 8 tokens per block.  LZSS
 * convention is to emit a match only when length ≥ 3.
 */
const val DEFAULT_MIN_MATCH = 3

// ─── Main object ─────────────────────────────────────────────────────────────

/**
 * LZSS (CMP02) compression and decompression.
 *
 * All public entry points are on this singleton `object`.  Internal helpers
 * are private functions, keeping the API surface minimal.
 *
 * Typical use:
 * ```kotlin
 * val compressed = Lzss.compress(data)
 * val restored   = Lzss.decompress(compressed)
 * ```
 *
 * Lower-level access to the token stream:
 * ```kotlin
 * val tokens  = Lzss.encode(data)
 * val decoded = Lzss.decode(tokens)
 * ```
 */
object Lzss {

    // ── Public API ────────────────────────────────────────────────────────────

    /**
     * Encode [data] into a list of LZSS tokens.
     *
     * At each cursor position the algorithm:
     * 1. Defines the search window as the last `windowSize` bytes of output.
     * 2. Finds the **longest** match of [data][cursor..] inside that window.
     * 3. If the match length ≥ [minMatch], emits a [LzssToken.Match] and
     *    advances the cursor by the match length.
     * 4. Otherwise emits a [LzssToken.Literal] and advances by 1.
     *
     * Overlapping matches (where the match extends past the cursor) are
     * intentionally supported — they enable run-length compression like
     * "AAAAAAA" encoded as `Literal('A') + Match(offset=1, length=6)`.
     *
     * Time complexity: O(n × windowSize) — scanning each cursor against the
     * window.  Production implementations use suffix arrays or hash chains
     * to reach O(n), but the naive scan is easiest to verify correct.
     */
    fun encode(
        data: ByteArray,
        windowSize: Int = DEFAULT_WINDOW_SIZE,
        maxMatch: Int = DEFAULT_MAX_MATCH,
        minMatch: Int = DEFAULT_MIN_MATCH
    ): List<LzssToken> {
        val tokens = mutableListOf<LzssToken>()
        var cursor = 0

        while (cursor < data.size) {
            // The search window starts at max(0, cursor - windowSize).
            // Using coerceAtLeast(0) avoids a negative index.
            val winStart = (cursor - windowSize).coerceAtLeast(0)
            val (offset, length) = findLongestMatch(data, cursor, winStart, maxMatch)

            if (length >= minMatch) {
                tokens += LzssToken.Match(offset, length)
                cursor += length
            } else {
                tokens += LzssToken.Literal(data[cursor])
                cursor += 1
            }
        }

        return tokens
    }

    /**
     * Decode a list of LZSS tokens back to the original byte sequence.
     *
     * For each [LzssToken.Literal], appends its byte directly.
     * For each [LzssToken.Match], copies bytes from `offset` positions back
     * in the output buffer — **one byte at a time** so that overlapping
     * matches (run-length) work correctly.
     *
     * Malformed tokens (offset = 0, or offset > output length) are silently
     * skipped to prevent crashes on corrupt or adversarial input.
     */
    fun decode(tokens: List<LzssToken>): ByteArray {
        // Pre-allocate a reasonable capacity to avoid repeated resizing.
        val output = ArrayList<Byte>(tokens.size * 2)

        for (token in tokens) {
            when (token) {
                is LzssToken.Literal -> output.add(token.value)
                is LzssToken.Match -> {
                    val off = token.offset
                    // Guard: offset must be positive and within the existing output.
                    // An offset of 0 or larger than output.size means the match
                    // points to data that hasn't been written yet — that's invalid.
                    if (off < 1 || off > output.size) continue
                    val start = output.size - off
                    // Copy byte-by-byte so that extending matches (offset < length)
                    // work as run-length expansion — e.g. "AAAA" from offset=1, length=4.
                    repeat(token.length) { i ->
                        output.add(output[start + i])
                    }
                }
            }
        }

        return output.toByteArray()
    }

    /**
     * One-shot compression: encode [data] and serialise to the CMP02 wire format.
     *
     * Internally calls [encode] and then [serialiseTokens].
     */
    fun compress(data: ByteArray): ByteArray {
        val tokens = encode(data)
        return serialiseTokens(tokens, data.size)
    }

    /**
     * One-shot decompression: deserialise CMP02 wire-format bytes back to the
     * original data.
     *
     * Internally calls [deserialiseTokens] and then [decode], truncating to
     * the original length stored in the header.
     */
    fun decompress(data: ByteArray): ByteArray {
        val (tokens, originalLength) = deserialiseTokens(data)
        val output = decode(tokens)
        // Truncate to the stored original length as a safety measure against
        // over-read from a padded or misaligned compressed stream.
        return if (output.size > originalLength) output.copyOf(originalLength) else output
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    /**
     * Find the longest match for `data[cursor..]` in the window `data[winStart..<cursor]`.
     *
     * Scans every position `pos` in `[winStart, cursor)` and counts how many
     * consecutive bytes starting at `pos` and `cursor` agree.  The scan may
     * read past `cursor` (overlap), which is intentional — it lets the encoder
     * represent runs like "AAAA" efficiently.
     *
     * Returns `(offset, length)` where `offset = cursor - pos` (distance back)
     * and `length` is the byte count of the best match found.
     *
     * Returns `(0, 0)` if the window is empty or no match of any length was
     * found (i.e., `winStart >= cursor`).
     */
    private fun findLongestMatch(
        data: ByteArray,
        cursor: Int,
        winStart: Int,
        maxMatch: Int
    ): Pair<Int, Int> {
        var bestLen = 0
        var bestOff = 0
        val lookaheadEnd = (cursor + maxMatch).coerceAtMost(data.size)

        for (pos in winStart until cursor) {
            var len = 0
            // Extend the match as long as bytes agree and we haven't exceeded maxMatch.
            // Note: `cursor + len` may go past `cursor` — that's the overlap/run-length case.
            while (cursor + len < lookaheadEnd && data[pos + len] == data[cursor + len]) {
                len++
            }
            if (len > bestLen) {
                bestLen = len
                bestOff = cursor - pos
            }
        }

        return Pair(bestOff, bestLen)
    }

    /**
     * Serialise an LZSS token list into the CMP02 binary wire format.
     *
     * Layout:
     * ```
     * [0..3]  original_length  (big-endian uint32)
     * [4..7]  block_count      (big-endian uint32)
     * [8+]    blocks:
     *           [flag_byte]          — 1 byte; bit i = 0 → Literal, 1 → Match
     *           [symbol_data...]     — 1 byte per Literal, 3 bytes per Match
     * ```
     *
     * Tokens are grouped into blocks of up to 8.  For the last block, unused
     * flag bits are left at 0 (Literal-style) but no symbol bytes are appended
     * for those absent tokens — the stored block_count and original_length tell
     * the decoder when to stop.
     */
    fun serialiseTokens(tokens: List<LzssToken>, originalLength: Int): ByteArray {
        // Build a list of (flag_byte, symbol_bytes) pairs — one entry per block.
        val blocks = mutableListOf<Pair<Byte, ByteArray>>()

        for (chunk in tokens.chunked(8)) {
            var flag = 0
            val symbolData = mutableListOf<Byte>()

            chunk.forEachIndexed { bit, token ->
                when (token) {
                    is LzssToken.Match -> {
                        // Set flag bit for this position to indicate Match.
                        flag = flag or (1 shl bit)
                        // Encode offset as big-endian 16-bit integer.
                        symbolData.add((token.offset ushr 8).toByte())
                        symbolData.add(token.offset.toByte())
                        // Encode length as a single byte.
                        symbolData.add(token.length.toByte())
                    }
                    is LzssToken.Literal -> {
                        // Flag bit stays 0 for Literal — just append the byte.
                        symbolData.add(token.value)
                    }
                }
            }

            blocks.add(Pair(flag.toByte(), symbolData.toByteArray()))
        }

        // Calculate total byte count so we can pre-allocate the buffer.
        // Header is 8 bytes; each block is 1 (flag) + len(symbol_data).
        val bodySize = blocks.sumOf { (_, sym) -> 1 + sym.size }
        val buf = ByteBuffer.allocate(8 + bodySize).order(ByteOrder.BIG_ENDIAN)

        buf.putInt(originalLength)
        buf.putInt(blocks.size)
        for ((flag, sym) in blocks) {
            buf.put(flag)
            buf.put(sym)
        }

        return buf.array()
    }

    /**
     * Deserialise CMP02 wire-format bytes into a token list and the original length.
     *
     * Returns `Pair(tokens, originalLength)`.  Returns an empty list with
     * length 0 if [data] is too short to contain even the 8-byte header.
     *
     * Security: the `block_count` from the header is capped against the
     * actual payload size.  A crafted header claiming billions of blocks
     * cannot force unbounded allocation — we can have at most one block
     * per byte of payload.
     */
    fun deserialiseTokens(data: ByteArray): Pair<List<LzssToken>, Int> {
        if (data.size < 8) return Pair(emptyList(), 0)

        val buf = ByteBuffer.wrap(data).order(ByteOrder.BIG_ENDIAN)
        val originalLength = buf.int               // bytes 0–3
        var blockCount = buf.int.toLong()           // bytes 4–7 (Long to handle u32 safely)

        // Cap block_count: at minimum 1 byte per block, so we can't have
        // more blocks than remaining bytes.  This prevents DoS via huge headers.
        val maxPossible = (data.size - 8).toLong()
        if (blockCount > maxPossible) blockCount = maxPossible

        val tokens = mutableListOf<LzssToken>()
        var pos = 8

        for (b in 0 until blockCount) {
            if (pos >= data.size) break

            val flag = data[pos].toInt() and 0xFF
            pos++

            // Each block holds up to 8 tokens, identified by bits 0..7 of flag.
            for (bit in 0 until 8) {
                if (pos >= data.size) break

                if (flag and (1 shl bit) != 0) {
                    // Match token: need 3 bytes (offset u16 BE + length u8).
                    if (pos + 3 > data.size) break
                    val offset = ((data[pos].toInt() and 0xFF) shl 8) or (data[pos + 1].toInt() and 0xFF)
                    val length = data[pos + 2].toInt() and 0xFF
                    tokens.add(LzssToken.Match(offset, length))
                    pos += 3
                } else {
                    // Literal token: 1 byte.
                    tokens.add(LzssToken.Literal(data[pos]))
                    pos++
                }
            }
        }

        return Pair(tokens, originalLength)
    }
}
