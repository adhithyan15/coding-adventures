// ============================================================================
// LZ77.java — CMP00: LZ77 Lossless Compression Algorithm (1977)
// ============================================================================
//
// LZ77 is the foundational sliding-window compression algorithm published by
// Abraham Lempel and Jacob Ziv in 1977.  It is the direct ancestor of LZSS,
// LZW, DEFLATE, zstd, LZ4, and virtually every modern compressor used in
// ZIP, gzip, PNG, and zlib.
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
//   │  (already processed — the       │  (not yet seen —  │
//   │   last window_size bytes)       │  next max_match)  │
//   └─────────────────────────────────┴──────────────────┘
//                                     ↑
//                                 cursor (current position)
//
// At each step the encoder searches the search buffer for the longest prefix
// of the lookahead buffer.  If it finds a match long enough (≥ min_match),
// it emits a back-reference token; otherwise it emits a literal token for
// the current byte.
//
// ============================================================================
// Token: (offset, length, next_char)
// ============================================================================
//
//   offset    (uint16, 0–65535): distance back to the match start, or 0.
//   length    (uint8,  0–255):   number of bytes in the match, or 0.
//   next_char (uint8,  0–255):   the literal byte after the match.
//
// A "literal" token has offset=0, length=0; the payload is just next_char.
// The decoder always appends next_char after copying length bytes.
//
// ============================================================================
// Overlapping Matches
// ============================================================================
//
// A match may extend into output bytes not yet written (when offset < length).
// This naturally encodes runs.  Example: if output so far is [A,B] and we
// emit Token(2, 5, 'Z'), the decoder copies byte-by-byte:
//
//   1. copy output[0]='A' → [A,B,A]
//   2. copy output[1]='B' → [A,B,A,B]
//   3. copy output[2]='A' (just written) → [A,B,A,B,A]
//   4–5. continues → [A,B,A,B,A,B,A]
//   finally append 'Z' → [A,B,A,B,A,B,A,Z]
//
// Bulk memmove would be wrong here; byte-by-byte copy handles the overlap.
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
// This is a teaching format.  Production compressors use variable-width
// bit-packing (DEFLATE, zstd) for further size reduction.
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
// Reference
// ============================================================================
//
//   Lempel, A., & Ziv, J. (1977). "A Universal Algorithm for Sequential Data
//   Compression". IEEE Transactions on Information Theory, 23(3), 337–343.
//
// ============================================================================

package com.codingadventures.lz77;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.ArrayList;
import java.util.List;

/**
 * CMP00: LZ77 lossless compression.
 *
 * <p>Provides {@link #compress(byte[])} and {@link #decompress(byte[])} for
 * one-shot compression/decompression, and {@link #encode(byte[])}/
 * {@link #decode(List)} for working with the intermediate token stream.
 *
 * <pre>{@code
 * byte[] original   = "AAABBC hello hello".getBytes();
 * byte[] compressed = LZ77.compress(original);
 * byte[] recovered  = LZ77.decompress(compressed);
 * assert Arrays.equals(original, recovered);
 * }</pre>
 */
public final class LZ77 {

    // =========================================================================
    // Constants
    // =========================================================================

    /** Default sliding-window size (maximum back-reference distance). */
    public static final int DEFAULT_WINDOW_SIZE = 4096;

    /** Default maximum match length (fits in one uint8). */
    public static final int DEFAULT_MAX_MATCH = 255;

    /**
     * Default minimum match length for a back-reference to be emitted.
     *
     * <p>A match of length 3 in LZ77 costs exactly 4 bytes (offset=2 + length=1
     * + next_char=1).  Three literals also cost 3×1+3×overhead = 4 bytes in
     * this wire format (a token per literal still costs 4 bytes), so length=3
     * is the break-even point.
     */
    public static final int DEFAULT_MIN_MATCH = 3;

    /** Wire-format header size in bytes (token_count). */
    private static final int HEADER_SIZE = 4;

    /** Wire-format bytes per token (offset=2, length=1, next_char=1). */
    private static final int TOKEN_SIZE = 4;

    /** Utility class — no instances. */
    private LZ77() {}

    // =========================================================================
    // Token record
    // =========================================================================

    /**
     * A single LZ77 token: (offset, length, next_char).
     *
     * <p>Represents one unit of the compressed stream.
     *
     * <ul>
     *   <li>{@code offset=0, length=0} → pure literal; payload is next_char.</li>
     *   <li>{@code offset>0, length>0} → back-reference: copy {@code length}
     *       bytes from {@code offset} positions back, then emit next_char.</li>
     * </ul>
     *
     * @param offset    distance back the match starts (1..window_size), or 0
     * @param length    number of matched bytes (0 = no match)
     * @param nextChar  literal byte immediately after the match (0..255)
     */
    public record Token(int offset, int length, int nextChar) {

        /** Create a literal token for a single byte {@code b}. */
        public static Token literal(int b) {
            return new Token(0, 0, b & 0xFF);
        }

        /** Create a back-reference token. */
        public static Token match(int offset, int length, int nextChar) {
            return new Token(offset, length, nextChar & 0xFF);
        }

        /** Return true if this token is a pure literal (no back-reference). */
        public boolean isLiteral() { return length == 0; }
    }

    // =========================================================================
    // Encoding
    // =========================================================================

    /**
     * Encode {@code data} into an LZ77 token stream using the default parameters.
     *
     * @param data the bytes to encode
     * @return token stream suitable for passing to {@link #decode(List)}
     */
    public static List<Token> encode(byte[] data) {
        return encode(data, DEFAULT_WINDOW_SIZE, DEFAULT_MAX_MATCH, DEFAULT_MIN_MATCH);
    }

    /**
     * Encode {@code data} into an LZ77 token stream with custom parameters.
     *
     * <p>Algorithm (greedy O(n × window)):
     * <ol>
     *   <li>For each cursor position, scan the last {@code windowSize} bytes for
     *       the longest match prefix of the lookahead.</li>
     *   <li>If the best match is ≥ {@code minMatch}, emit a back-reference token
     *       and advance cursor by {@code length + 1}.</li>
     *   <li>Otherwise emit a literal token and advance cursor by 1.</li>
     * </ol>
     *
     * <p>Note: one byte is always reserved for next_char, so the lookahead can
     * extend at most to {@code data.length - 1}.
     *
     * @param data       the bytes to encode
     * @param windowSize maximum back-reference distance (search buffer size)
     * @param maxMatch   maximum match length
     * @param minMatch   minimum match length to emit a back-reference
     * @return token stream
     */
    public static List<Token> encode(byte[] data, int windowSize, int maxMatch, int minMatch) {
        List<Token> tokens = new ArrayList<>();
        int cursor = 0;

        while (cursor < data.length) {
            int[] best = findLongestMatch(data, cursor, windowSize, maxMatch);
            int bestOffset = best[0];
            int bestLength = best[1];

            if (bestLength >= minMatch) {
                // Back-reference token: next_char is the byte after the match.
                int nextChar = data[cursor + bestLength] & 0xFF;
                tokens.add(Token.match(bestOffset, bestLength, nextChar));
                cursor += bestLength + 1;
            } else {
                // Literal token.
                tokens.add(Token.literal(data[cursor] & 0xFF));
                cursor += 1;
            }
        }

        return tokens;
    }

    // =========================================================================
    // Decoding
    // =========================================================================

    /**
     * Decode an LZ77 token stream back into the original bytes.
     *
     * <p>For each token:
     * <ul>
     *   <li>If {@code length > 0}: copy {@code length} bytes one-at-a-time from
     *       position {@code output.size() - offset}.  Byte-by-byte copy handles
     *       overlapping matches (offset &lt; length).</li>
     *   <li>Always append {@code next_char}.</li>
     * </ul>
     *
     * @param tokens the token stream from {@link #encode(byte[])}
     * @return the reconstructed bytes
     */
    public static byte[] decode(List<Token> tokens) {
        return decode(tokens, new byte[0]);
    }

    /**
     * Decode with an optional pre-seeded output buffer (for streaming use).
     *
     * @param tokens        the token stream
     * @param initialBuffer optional seed for the search buffer
     * @return the reconstructed bytes (includes the seed)
     */
    public static byte[] decode(List<Token> tokens, byte[] initialBuffer) {
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        try {
            out.write(initialBuffer);
        } catch (IOException e) {
            throw new UncheckedIOException(e);
        }

        for (Token token : tokens) {
            if (token.length() > 0) {
                // Back-reference: copy byte-by-byte for overlap safety.
                int start = out.size() - token.offset();
                // Guard: offset must not exceed what has already been written.
                // A crafted stream with offset > out.size() would produce a
                // negative start index and cause ArrayIndexOutOfBoundsException.
                if (start < 0) {
                    throw new IllegalArgumentException(
                        "LZ77: back-reference offset " + token.offset() +
                        " exceeds output buffer size " + out.size());
                }
                for (int i = 0; i < token.length(); i++) {
                    // Re-read each time: if offset < length, we read bytes
                    // written during this very loop iteration.
                    out.write(out.toByteArray()[start + i]);
                }
            }
            // Always append next_char.
            out.write(token.nextChar());
        }

        return out.toByteArray();
    }

    // =========================================================================
    // One-shot compress / decompress
    // =========================================================================

    /**
     * Compress {@code data} using LZ77 with default parameters and return
     * the CMP00 wire-format bytes.
     *
     * @param data the bytes to compress (null treated as empty)
     * @return compressed bytes in CMP00 wire format
     */
    public static byte[] compress(byte[] data) {
        return compress(data, DEFAULT_WINDOW_SIZE, DEFAULT_MAX_MATCH, DEFAULT_MIN_MATCH);
    }

    /**
     * Compress {@code data} with custom parameters.
     *
     * @param data       the bytes to compress
     * @param windowSize maximum back-reference distance
     * @param maxMatch   maximum match length
     * @param minMatch   minimum match length for a back-reference
     * @return compressed bytes in CMP00 wire format
     */
    public static byte[] compress(byte[] data, int windowSize, int maxMatch, int minMatch) {
        if (data == null) data = new byte[0];
        List<Token> tokens = encode(data, windowSize, maxMatch, minMatch);
        return serialiseTokens(tokens);
    }

    /**
     * Decompress CMP00 wire-format bytes back to the original data.
     *
     * @param data compressed bytes (null treated as empty)
     * @return the original, uncompressed bytes
     */
    public static byte[] decompress(byte[] data) {
        if (data == null || data.length < HEADER_SIZE) return new byte[0];
        List<Token> tokens = deserialiseTokens(data);
        return decode(tokens);
    }

    // =========================================================================
    // Serialisation helpers
    // =========================================================================

    /**
     * Serialise a token list to the CMP00 wire format.
     *
     * <p>Format: {@code [token_count (4B BE)] [N × (offset BE uint16, length uint8, next_char uint8)]}
     *
     * @param tokens the token list
     * @return the serialised bytes
     */
    static byte[] serialiseTokens(List<Token> tokens) {
        int n = tokens.size();
        ByteBuffer buf = ByteBuffer.allocate(HEADER_SIZE + n * TOKEN_SIZE)
            .order(ByteOrder.BIG_ENDIAN);
        buf.putInt(n);
        for (Token t : tokens) {
            buf.putShort((short) t.offset());  // uint16
            buf.put((byte) t.length());         // uint8
            buf.put((byte) t.nextChar());       // uint8
        }
        return buf.array();
    }

    /**
     * Deserialise CMP00 wire-format bytes into a token list.
     *
     * @param data the serialised bytes
     * @return the token list
     */
    static List<Token> deserialiseTokens(byte[] data) {
        if (data == null || data.length < HEADER_SIZE) return new ArrayList<>();
        ByteBuffer buf = ByteBuffer.wrap(data).order(ByteOrder.BIG_ENDIAN);
        int tokenCount = buf.getInt();

        // Cap against payload size to prevent DoS from crafted headers.
        int maxTokens = (data.length - HEADER_SIZE) / TOKEN_SIZE;
        int safeCount = Math.min(tokenCount, maxTokens);

        List<Token> tokens = new ArrayList<>(safeCount);
        for (int i = 0; i < safeCount && buf.remaining() >= TOKEN_SIZE; i++) {
            int offset   = buf.getShort() & 0xFFFF;  // uint16
            int length   = buf.get() & 0xFF;          // uint8
            int nextChar = buf.get() & 0xFF;          // uint8
            tokens.add(new Token(offset, length, nextChar));
        }
        return tokens;
    }

    // =========================================================================
    // Private helpers
    // =========================================================================

    /**
     * Find the longest match for {@code data[cursor:]} in the search buffer.
     *
     * <p>Scans the {@code window_size} bytes before {@code cursor} for the
     * longest prefix of {@code data[cursor:]} that also appears there.
     * Matches may overlap the lookahead region (extend past cursor) — the
     * decoder handles this correctly by copying byte-by-byte.
     *
     * <p>One byte is always reserved for next_char, so the lookahead can
     * match at most to {@code data.length - 1}.
     *
     * @param data       full input bytes
     * @param cursor     current encode position (start of lookahead)
     * @param windowSize maximum lookback distance
     * @param maxMatch   maximum match length
     * @return {@code {bestOffset, bestLength}}; both 0 if no match found
     */
    private static int[] findLongestMatch(byte[] data, int cursor, int windowSize, int maxMatch) {
        int bestOffset = 0;
        int bestLength = 0;

        int searchStart  = Math.max(0, cursor - windowSize);
        // One byte reserved for next_char → lookahead ends at data.length - 1.
        int lookaheadEnd = Math.min(cursor + maxMatch, data.length - 1);

        for (int pos = searchStart; pos < cursor; pos++) {
            int length = 0;
            while (cursor + length < lookaheadEnd
                && data[pos + length] == data[cursor + length]) {
                length++;
            }
            if (length > bestLength) {
                bestLength = length;
                bestOffset = cursor - pos;  // distance back from cursor
            }
        }

        return new int[]{bestOffset, bestLength};
    }
}
