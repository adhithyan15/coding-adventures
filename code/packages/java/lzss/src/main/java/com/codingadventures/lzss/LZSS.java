// ============================================================================
// LZSS.java — CMP02: LZSS Lossless Compression Algorithm (1982)
// ============================================================================
//
// LZSS (Lempel-Ziv-Storer-Szymanski, 1982) is a refinement of LZ77 (CMP00)
// that eliminates a systematic waste: in LZ77, every token emits a trailing
// `next_char` byte even after a long back-reference.  LZSS replaces the fixed
// (offset, length, next_char) triple with a flag-bit scheme where each symbol
// is either a bare literal byte or a bare back-reference — never both.
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
// literals and which are matches.
//
// ============================================================================
// The Flag-Byte Scheme
// ============================================================================
//
// Symbols are grouped in chunks of 8.  Each chunk is preceded by one flag byte:
//
//   bit 0 (LSB) = type of symbol 0 in this chunk
//   bit 1       = type of symbol 1
//   ...
//   bit 7       = type of symbol 7
//
//   0 = Literal   → 1 byte  (the actual byte value)
//   1 = Match     → 3 bytes (offset as big-endian uint16 + length as uint8)
//
// The last block may have < 8 symbols; unused flag bits are 0.
//
// ============================================================================
// Break-Even Point
// ============================================================================
//
// A match costs 3 bytes.  Three literals also cost 3 bytes (1 each).  So a
// match of length ≥ 3 breaks even or saves space.  Traditionally `min_match=3`
// is used — the same break-even threshold as LZ77.
//
// ============================================================================
// Overlapping Matches
// ============================================================================
//
// LZSS inherits LZ77's self-referential matches (offset < length).  Decoding
// must copy byte-by-byte, not with a bulk memmove.
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

package com.codingadventures.lzss;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.ArrayList;
import java.util.List;

/**
 * CMP02: LZSS lossless compression.
 *
 * <p>Provides {@link #compress(byte[])} and {@link #decompress(byte[])} for
 * one-shot compression, and {@link #encode(byte[])}/{@link #decode(List,int)}
 * for working with the intermediate token stream.
 *
 * <pre>{@code
 * byte[] original   = "hello hello hello".getBytes();
 * byte[] compressed = LZSS.compress(original);
 * byte[] recovered  = LZSS.decompress(compressed);
 * assert Arrays.equals(original, recovered);
 * }</pre>
 */
public final class LZSS {

    // =========================================================================
    // Constants
    // =========================================================================

    /** Default sliding-window size (maximum back-reference distance). */
    public static final int DEFAULT_WINDOW_SIZE = 4096;

    /** Default maximum match length (fits in uint8). */
    public static final int DEFAULT_MAX_MATCH = 255;

    /** Default minimum match length for a back-reference to be worthwhile. */
    public static final int DEFAULT_MIN_MATCH = 3;

    /** Symbols per flag-byte group. */
    private static final int BLOCK_SIZE = 8;

    /** Wire-format header size: original_length(4) + block_count(4). */
    private static final int HEADER_SIZE = 8;

    /** Utility class — no instances. */
    private LZSS() {}

    // =========================================================================
    // Token types
    // =========================================================================

    /**
     * A single LZSS token — either a bare literal byte or a bare back-reference.
     *
     * <p>LZSS uses a sealed hierarchy: each token is one of:
     * <ul>
     *   <li>{@link Literal} — a single raw byte.</li>
     *   <li>{@link Match}   — a (offset, length) back-reference.</li>
     * </ul>
     */
    public sealed interface Token permits LZSS.Literal, LZSS.Match {
        /** True if this is a literal token. */
        boolean isLiteral();
    }

    /**
     * A single literal byte in the LZSS token stream.
     *
     * @param value the byte value (0–255)
     */
    public record Literal(int value) implements Token {
        @Override public boolean isLiteral() { return true; }
    }

    /**
     * A back-reference match in the LZSS token stream.
     *
     * @param offset distance back in the output where the match begins (1..window_size)
     * @param length number of bytes to copy (min_match..max_match)
     */
    public record Match(int offset, int length) implements Token {
        @Override public boolean isLiteral() { return false; }
    }

    // =========================================================================
    // Encoding
    // =========================================================================

    /**
     * Encode {@code data} into an LZSS token stream using default parameters.
     *
     * @param data the bytes to encode
     * @return token stream (mix of {@link Literal} and {@link Match})
     */
    public static List<Token> encode(byte[] data) {
        return encode(data, DEFAULT_WINDOW_SIZE, DEFAULT_MAX_MATCH, DEFAULT_MIN_MATCH);
    }

    /**
     * Encode {@code data} into an LZSS token stream with custom parameters.
     *
     * <p>Key difference from LZ77: on a match, the cursor advances by exactly
     * {@code length} (not {@code length + 1}).  There is no trailing next_char.
     *
     * @param data       the bytes to encode
     * @param windowSize maximum lookback distance
     * @param maxMatch   maximum match length
     * @param minMatch   minimum match length for a Match token
     * @return list of {@link Literal} and {@link Match} tokens
     */
    public static List<Token> encode(byte[] data, int windowSize, int maxMatch, int minMatch) {
        List<Token> tokens = new ArrayList<>();
        int cursor = 0;

        while (cursor < data.length) {
            int[] best = findLongestMatch(data, cursor, windowSize, maxMatch);
            int bestOffset = best[0];
            int bestLength = best[1];

            if (bestLength >= minMatch) {
                // Emit bare back-reference; cursor advances by length only.
                tokens.add(new Match(bestOffset, bestLength));
                cursor += bestLength;
            } else {
                // Emit bare literal; cursor advances by 1.
                tokens.add(new Literal(data[cursor] & 0xFF));
                cursor += 1;
            }
        }

        return tokens;
    }

    // =========================================================================
    // Decoding
    // =========================================================================

    /**
     * Decode an LZSS token stream back into the original bytes.
     *
     * <p>Processes each token:
     * <ul>
     *   <li>{@link Literal}: append the byte to output.</li>
     *   <li>{@link Match}: copy {@code length} bytes one-at-a-time from
     *       {@code output.size - offset}.  Byte-by-byte handles overlapping.</li>
     * </ul>
     *
     * @param tokens         the token stream from {@link #encode(byte[])}
     * @param originalLength if ≥ 0, truncate to this length; pass -1 for no truncation
     * @return the reconstructed bytes
     */
    public static byte[] decode(List<Token> tokens, int originalLength) {
        ByteArrayOutputStream out = new ByteArrayOutputStream();

        for (Token token : tokens) {
            if (token instanceof Literal lit) {
                out.write(lit.value());
            } else {
                Match m   = (Match) token;
                int start = out.size() - m.offset();
                // Guard: offset must not exceed what has already been written.
                // A crafted stream with offset > out.size() would produce a
                // negative start index and cause ArrayIndexOutOfBoundsException.
                if (start < 0) {
                    throw new IllegalArgumentException(
                        "LZSS: back-reference offset " + m.offset() +
                        " exceeds output buffer size " + out.size());
                }
                for (int i = 0; i < m.length(); i++) {
                    out.write(out.toByteArray()[start + i]);
                }
            }
        }

        byte[] result = out.toByteArray();
        // originalLength must be non-negative; negative values (from crafted headers)
        // are treated as "no truncation" to avoid NegativeArraySizeException.
        if (originalLength >= 0 && originalLength < result.length) {
            byte[] trimmed = new byte[originalLength];
            System.arraycopy(result, 0, trimmed, 0, originalLength);
            return trimmed;
        }
        return result;
    }

    // =========================================================================
    // One-shot compress / decompress
    // =========================================================================

    /**
     * Compress {@code data} using LZSS and return CMP02 wire-format bytes.
     *
     * @param data the bytes to compress (null treated as empty)
     * @return compressed bytes in CMP02 wire format
     */
    public static byte[] compress(byte[] data) {
        return compress(data, DEFAULT_WINDOW_SIZE, DEFAULT_MAX_MATCH, DEFAULT_MIN_MATCH);
    }

    /**
     * Compress with custom parameters.
     *
     * @param data       the bytes to compress
     * @param windowSize maximum lookback distance
     * @param maxMatch   maximum match length
     * @param minMatch   minimum match length for a back-reference
     * @return compressed bytes in CMP02 wire format
     */
    public static byte[] compress(byte[] data, int windowSize, int maxMatch, int minMatch) {
        if (data == null) data = new byte[0];
        List<Token> tokens = encode(data, windowSize, maxMatch, minMatch);
        return serialiseTokens(tokens, data.length);
    }

    /**
     * Decompress CMP02 wire-format bytes back to the original data.
     *
     * @param data compressed bytes (null treated as empty)
     * @return the original, uncompressed bytes
     */
    public static byte[] decompress(byte[] data) {
        if (data == null || data.length < HEADER_SIZE) return new byte[0];
        int[] header = parseHeader(data);
        int originalLength = header[0];
        List<Token> tokens = deserialiseTokens(data);
        return decode(tokens, originalLength);
    }

    // =========================================================================
    // Serialisation helpers
    // =========================================================================

    /**
     * Serialise an LZSS token list to the CMP02 wire format.
     *
     * <p>Groups tokens into blocks of up to 8.  Each block is preceded by a
     * flag byte (bit i → 0=Literal, 1=Match), followed by the token data:
     * <ul>
     *   <li>Literal: 1 byte</li>
     *   <li>Match: 3 bytes (offset big-endian uint16 + length uint8)</li>
     * </ul>
     *
     * @param tokens         the token list
     * @param originalLength length of the original uncompressed data
     * @return the CMP02 wire-format bytes
     */
    static byte[] serialiseTokens(List<Token> tokens, int originalLength) {
        List<byte[]> blocks = new ArrayList<>();
        int i = 0;
        while (i < tokens.size()) {
            List<Token> chunk = tokens.subList(i, Math.min(i + BLOCK_SIZE, tokens.size()));
            int flag = 0;
            ByteArrayOutputStream blockData = new ByteArrayOutputStream();

            for (int bit = 0; bit < chunk.size(); bit++) {
                Token tok = chunk.get(bit);
                if (tok instanceof Match m) {
                    flag |= (1 << bit);
                    try {
                        blockData.write(new byte[]{
                            (byte)(m.offset() >> 8),   // offset high byte
                            (byte)(m.offset() & 0xFF), // offset low byte
                            (byte) m.length()           // length
                        });
                    } catch (IOException e) {
                        throw new UncheckedIOException(e);
                    }
                } else {
                    blockData.write(((Literal) tok).value());
                }
            }

            ByteArrayOutputStream block = new ByteArrayOutputStream();
            block.write(flag);
            try { block.write(blockData.toByteArray()); }
            catch (IOException e) { throw new UncheckedIOException(e); }
            blocks.add(block.toByteArray());
            i += BLOCK_SIZE;
        }

        // Header: original_length(4B) + block_count(4B)
        ByteBuffer header = ByteBuffer.allocate(HEADER_SIZE).order(ByteOrder.BIG_ENDIAN);
        header.putInt(originalLength).putInt(blocks.size());

        ByteArrayOutputStream out = new ByteArrayOutputStream();
        try {
            out.write(header.array());
            for (byte[] b : blocks) out.write(b);
        } catch (IOException e) {
            throw new UncheckedIOException(e);
        }
        return out.toByteArray();
    }

    /**
     * Deserialise CMP02 wire-format bytes into a token list.
     *
     * @param data the CMP02 bytes
     * @return list of {@link Literal} and {@link Match} tokens
     */
    static List<Token> deserialiseTokens(byte[] data) {
        if (data == null || data.length < HEADER_SIZE) return new ArrayList<>();
        ByteBuffer header = ByteBuffer.wrap(data, 0, HEADER_SIZE).order(ByteOrder.BIG_ENDIAN);
        /* originalLength = */ header.getInt();
        int blockCount = header.getInt();

        // Cap block count against actual payload to prevent DoS.
        int maxBlocks = data.length - HEADER_SIZE; // at least 1 byte per block
        int safeCount = Math.min(blockCount, maxBlocks);

        List<Token> tokens = new ArrayList<>();
        int pos = HEADER_SIZE;

        for (int b = 0; b < safeCount && pos < data.length; b++) {
            int flag = data[pos++] & 0xFF;

            for (int bit = 0; bit < BLOCK_SIZE && pos < data.length; bit++) {
                if ((flag & (1 << bit)) != 0) {
                    // Match: 3 bytes
                    if (pos + 3 > data.length) break;
                    int offset = ((data[pos] & 0xFF) << 8) | (data[pos + 1] & 0xFF);
                    int length = data[pos + 2] & 0xFF;
                    tokens.add(new Match(offset, length));
                    pos += 3;
                } else {
                    // Literal: 1 byte
                    tokens.add(new Literal(data[pos++] & 0xFF));
                }
            }
        }

        return tokens;
    }

    // =========================================================================
    // Private helpers
    // =========================================================================

    /** Parse the 8-byte header; returns [original_length, block_count]. */
    private static int[] parseHeader(byte[] data) {
        ByteBuffer buf = ByteBuffer.wrap(data, 0, HEADER_SIZE).order(ByteOrder.BIG_ENDIAN);
        return new int[]{buf.getInt(), buf.getInt()};
    }

    /**
     * Find the longest match for {@code data[cursor:]} in the search buffer.
     *
     * <p>Unlike LZ77's implementation, the lookahead may extend all the way to
     * {@code data.length} — there is no next_char reservation in LZSS.
     *
     * @param data       full input bytes
     * @param cursor     current encode position
     * @param windowSize maximum lookback distance
     * @param maxMatch   maximum match length
     * @return {@code {bestOffset, bestLength}}; both 0 if no match found
     */
    private static int[] findLongestMatch(byte[] data, int cursor, int windowSize, int maxMatch) {
        int bestOffset = 0;
        int bestLength = 0;

        int searchStart  = Math.max(0, cursor - windowSize);
        // No next_char reservation: lookahead extends to end of input.
        int lookaheadEnd = Math.min(cursor + maxMatch, data.length);

        for (int pos = searchStart; pos < cursor; pos++) {
            int length = 0;
            while (cursor + length < lookaheadEnd
                && data[pos + length] == data[cursor + length]) {
                length++;
            }
            if (length > bestLength) {
                bestLength = length;
                bestOffset = cursor - pos;
            }
        }

        return new int[]{bestOffset, bestLength};
    }
}
