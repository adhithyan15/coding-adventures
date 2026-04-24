package com.codingadventures.lzss;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.ArrayList;
import java.util.List;
import java.util.Objects;

/**
 * LZSS lossless compression algorithm (1982) — CMP02.
 *
 * <h2>Background</h2>
 *
 * <p>LZSS (Lempel–Ziv–Storer–Szymanski) is a refinement of LZ77.  The key
 * idea of both is the <em>sliding window</em>: as we scan the input we keep a
 * look-back buffer of the last {@code windowSize} bytes.  If the bytes at the
 * current position also appear somewhere in that window, we can replace them
 * with a compact (offset, length) reference instead of copying them literally.
 * </p>
 *
 * <p>LZ77 always emitted a triple {@code (offset, length, next_char)} — even
 * when there was no useful match, wasting 2 bytes on a zero-length reference.
 * LZSS fixes this with a <em>flag-bit scheme</em>:</p>
 * <ul>
 *   <li>Tokens are grouped into blocks of 8.</li>
 *   <li>Each block starts with a 1-byte flag.  Bit {@code i} (LSB = bit 0)
 *       describes the {@code i}-th token in the block.</li>
 *   <li>Flag bit = 0 → Literal (1 byte on wire).</li>
 *   <li>Flag bit = 1 → Match   (3 bytes on wire: 2-byte BE offset, 1-byte length).</li>
 * </ul>
 *
 * <p>Break-even point: a Match is 3 bytes; a sequence of 3 Literals is also
 * 3 bytes (3 data bytes + 3/8 of a flag byte ≈ 3.4 bytes).  So the default
 * {@link #DEFAULT_MIN_MATCH} of 3 is where matches start to save space.</p>
 *
 * <h2>CMP02 Wire Format</h2>
 *
 * <pre>{@code
 * Bytes 0–3:  original_length  (big-endian uint32)
 * Bytes 4–7:  block_count      (big-endian uint32)
 * Bytes 8+:   blocks
 *   Each block: [1-byte flag][symbol data]
 *     Literal symbol: 1 byte
 *     Match symbol:   2-byte BE offset + 1-byte length
 * }</pre>
 *
 * <h2>Series context</h2>
 *
 * <pre>{@code
 * CMP00 (LZ77,    1977) — Sliding-window back-references.
 * CMP01 (LZ78,    1978) — Explicit dictionary (trie).
 * CMP02 (LZSS,   1982) — LZ77 + flag bits.       ← this package
 * CMP03 (LZW,    1984) — LZ78 + pre-init alphabet; used in GIF.
 * CMP04 (Huffman, 1952) — Entropy coding.
 * CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
 * }</pre>
 *
 * <h2>Quick example</h2>
 *
 * <pre>{@code
 * byte[] data = "hello hello hello".getBytes(StandardCharsets.UTF_8);
 * byte[] compressed = Lzss.compress(data);
 * byte[] restored   = Lzss.decompress(compressed);
 * assert Arrays.equals(data, restored);
 * }</pre>
 */
public final class Lzss {

    // ─── Constants ────────────────────────────────────────────────────────────

    /**
     * Default sliding-window size in bytes.
     *
     * <p>The encoder looks back at most this many bytes when searching for
     * matches.  4096 bytes matches the CMP02 specification and fits the offset
     * in a 12-bit field (though we use a full 16-bit BE short on the wire,
     * matching the Rust reference).</p>
     */
    public static final int DEFAULT_WINDOW_SIZE = 4096;

    /**
     * Default maximum match length.
     *
     * <p>255 is the largest value that fits in an unsigned byte on the wire.
     * Longer runs are split into multiple Match tokens.</p>
     */
    public static final int DEFAULT_MAX_MATCH = 255;

    /**
     * Default minimum match length.
     *
     * <p>A Match token costs 3 bytes on the wire.  Encoding fewer than 3
     * bytes as a match would expand, not compress.  So we only emit a Match
     * when its length is at least this value.</p>
     */
    public static final int DEFAULT_MIN_MATCH = 3;

    // Private constructor — this is a static utility class.
    private Lzss() {}

    // ─── Sliding-window encoder ───────────────────────────────────────────────

    /**
     * Find the longest match for {@code data[cursor..]} within the look-back
     * window {@code data[windowStart..cursor]}.
     *
     * <p>We iterate over every start position {@code pos} in the window and
     * count how many bytes at {@code pos + k} equal {@code data[cursor + k]}.
     * Overlapping matches — where the match extends past {@code cursor} —
     * are intentionally allowed; they implement run-length expansion
     * (e.g., "A" → repeated "AAAA…").</p>
     *
     * <p>Time complexity: O(window × maxMatch) per call — fine for moderate
     * inputs; a production compressor would use a hash table for O(1) look-ups.</p>
     *
     * @param data        the full input byte array
     * @param cursor      current write position (start of look-ahead)
     * @param windowStart first valid window position (= max(0, cursor - windowSize))
     * @param maxMatch    cap on match length
     * @return int[2] = {bestOffset, bestLength}; {0, 0} if no match found
     */
    private static int[] findLongestMatch(byte[] data, int cursor, int windowStart, int maxMatch) {
        int bestLength = 0;
        int bestOffset = 0;
        // The farthest position in the look-ahead we are allowed to reach.
        int lookaheadEnd = Math.min(cursor + maxMatch, data.length);

        for (int pos = windowStart; pos < cursor; pos++) {
            int length = 0;
            // Extend the match byte-by-byte.  The index arithmetic is safe
            // because lookaheadEnd is bounded by data.length.
            while (cursor + length < lookaheadEnd
                    && data[pos + length] == data[cursor + length]) {
                length++;
            }
            if (length > bestLength) {
                bestLength = length;
                // Offset = distance back from the current cursor.  We store
                // it as a positive integer; offset 1 means "one byte back".
                bestOffset = cursor - pos;
            }
        }

        return new int[]{bestOffset, bestLength};
    }

    /**
     * Encode {@code data} into an LZSS token stream.
     *
     * <p>At each cursor position:</p>
     * <ol>
     *   <li>Search the last {@code windowSize} bytes for the longest match.</li>
     *   <li>If the match length is &ge; {@code minMatch}, emit a
     *       {@link LzssToken.Match} and advance by {@code length}.</li>
     *   <li>Otherwise emit a {@link LzssToken.Literal} for the current byte
     *       and advance by 1.</li>
     * </ol>
     *
     * <p>Greedy selection (pick the longest match available right now) is the
     * classic LZSS strategy.  Optimal parsing (look-ahead to pick a globally
     * better sequence) is possible but far more complex and not implemented
     * here.</p>
     *
     * @param data       input bytes to compress (must not be null)
     * @param windowSize look-back buffer size in bytes
     * @param maxMatch   maximum match length (1–255)
     * @param minMatch   minimum match length for a Match token (usually 3)
     * @return ordered list of {@link LzssToken}s representing the input
     * @throws IllegalArgumentException if any parameter is out of range
     */
    public static List<LzssToken> encode(byte[] data, int windowSize, int maxMatch, int minMatch) {
        Objects.requireNonNull(data, "data must not be null");
        validateParameters(windowSize, maxMatch, minMatch);

        List<LzssToken> tokens = new ArrayList<>();
        int cursor = 0;

        while (cursor < data.length) {
            // The window is the last windowSize bytes before cursor.
            int windowStart = Math.max(0, cursor - windowSize);
            int[] best = findLongestMatch(data, cursor, windowStart, maxMatch);
            int bestOffset = best[0];
            int bestLength = best[1];

            if (bestLength >= minMatch) {
                // Emit a back-reference and skip over the matched region.
                tokens.add(new LzssToken.Match(bestOffset, bestLength));
                cursor += bestLength;
            } else {
                // No useful match — emit the raw byte.
                tokens.add(new LzssToken.Literal(data[cursor]));
                cursor++;
            }
        }

        return tokens;
    }

    // ─── Decoder ─────────────────────────────────────────────────────────────

    /**
     * Decode an LZSS token stream back into the original bytes.
     *
     * <p>For each {@link LzssToken.Literal}, append the byte directly.
     * For each {@link LzssToken.Match}, copy {@code length} bytes starting
     * {@code offset} positions back in the output buffer, byte-by-byte so
     * that overlapping (run-length) matches work correctly.</p>
     *
     * <p>Example of an overlapping match:</p>
     * <pre>
     *   Output so far: [A]
     *   Token: Match(offset=1, length=6)
     *   Step 1: copy output[0] = A → output = [A, A]
     *   Step 2: copy output[1] = A → output = [A, A, A]
     *   ... → output = [A, A, A, A, A, A, A]
     * </pre>
     *
     * @param tokens         list of tokens produced by {@link #encode}
     * @param originalLength expected output length; the result is truncated to
     *                       this length if non-negative
     * @return the decoded byte array
     * @throws IllegalArgumentException if any Match has an invalid offset
     */
    public static byte[] decode(List<LzssToken> tokens, int originalLength) {
        Objects.requireNonNull(tokens, "tokens must not be null");

        List<Byte> output = new ArrayList<>();

        for (LzssToken token : tokens) {
            switch (token) {
                case LzssToken.Literal lit -> output.add(lit.value());

                case LzssToken.Match match -> {
                    int off = match.offset();
                    // Guard: offset 0 or beyond current output is malformed data —
                    // skip silently to match the Rust reference's resilience.
                    if (off == 0 || off > output.size()) {
                        continue;
                    }
                    int start = output.size() - off;
                    // Copy byte-by-byte — crucial for overlapping matches.
                    for (int i = 0; i < match.length(); i++) {
                        output.add(output.get(start + i));
                    }
                }
            }
        }

        // Materialise the List<Byte> into a primitive byte[].
        int size = (originalLength >= 0) ? Math.min(originalLength, output.size()) : output.size();
        byte[] result = new byte[size];
        for (int i = 0; i < size; i++) {
            result[i] = output.get(i);
        }
        return result;
    }

    // ─── Serialisation ───────────────────────────────────────────────────────

    /**
     * Serialise an LZSS token list to the CMP02 wire format.
     *
     * <p>Tokens are grouped into blocks of (at most) 8.  Each block has:</p>
     * <ul>
     *   <li>1-byte flag — bit {@code i} is 1 if token {@code i} is a Match,
     *       0 if it is a Literal.</li>
     *   <li>Per-token data — 1 byte for Literal, 3 bytes for Match
     *       (2-byte BE offset + 1-byte length).</li>
     * </ul>
     *
     * <p>The 8-byte header stores the original (uncompressed) length and the
     * number of blocks so the decoder knows exactly how many blocks to read.</p>
     *
     * @param tokens         the token list from {@link #encode}
     * @param originalLength byte length of the original uncompressed data
     * @return CMP02-formatted byte array
     */
    static byte[] serialiseTokens(List<LzssToken> tokens, int originalLength) {
        Objects.requireNonNull(tokens, "tokens must not be null");

        // --- Pass 1: build all block byte arrays ---
        List<byte[]> blocks = new ArrayList<>();

        for (int tokenIndex = 0; tokenIndex < tokens.size(); tokenIndex += 8) {
            // A chunk is up to 8 consecutive tokens.
            int chunkEnd = Math.min(tokenIndex + 8, tokens.size());
            List<LzssToken> chunk = tokens.subList(tokenIndex, chunkEnd);

            byte flag = 0;
            List<Byte> symbolBytes = new ArrayList<>();

            for (int bit = 0; bit < chunk.size(); bit++) {
                LzssToken tok = chunk.get(bit);
                switch (tok) {
                    case LzssToken.Match match -> {
                        // Set bit i in the flag byte to signal "this is a Match".
                        flag |= (byte) (1 << bit);
                        // Offset as big-endian unsigned short (2 bytes).
                        symbolBytes.add((byte) ((match.offset() >> 8) & 0xFF));
                        symbolBytes.add((byte) (match.offset() & 0xFF));
                        // Length as unsigned byte (1 byte).
                        symbolBytes.add((byte) (match.length() & 0xFF));
                    }
                    case LzssToken.Literal lit -> {
                        // Literal: just one raw byte.
                        symbolBytes.add(lit.value());
                    }
                }
            }

            // Prepend the flag byte to the symbol data.
            byte[] block = new byte[1 + symbolBytes.size()];
            block[0] = flag;
            for (int i = 0; i < symbolBytes.size(); i++) {
                block[i + 1] = symbolBytes.get(i);
            }
            blocks.add(block);
        }

        // --- Pass 2: write header + blocks into one buffer ---
        int totalBody = blocks.stream().mapToInt(b -> b.length).sum();
        ByteBuffer buf = ByteBuffer.allocate(8 + totalBody).order(ByteOrder.BIG_ENDIAN);
        buf.putInt(originalLength);
        buf.putInt(blocks.size());
        for (byte[] block : blocks) {
            buf.put(block);
        }

        return buf.array();
    }

    /**
     * Deserialise CMP02 wire-format bytes into a token list and original length.
     *
     * <p>Security note: the {@code block_count} field in the header is capped
     * against the actual remaining payload size (1 byte minimum per block) to
     * prevent a crafted header with an enormous block count from causing a
     * denial-of-service via a giant loop that reads no data.</p>
     *
     * @param data the CMP02-formatted compressed bytes
     * @return int[2] array where index 0 is originalLength; tokens returned
     *         via the companion {@code result} field (we use a small holder)
     */
    private static DeserialiseResult deserialiseTokens(byte[] data) {
        Objects.requireNonNull(data, "data must not be null");

        if (data.length < 8) {
            return new DeserialiseResult(new ArrayList<>(), 0);
        }

        ByteBuffer buf = ByteBuffer.wrap(data).order(ByteOrder.BIG_ENDIAN);
        int originalLength = buf.getInt();   // bytes 0–3
        long blockCountLong = Integer.toUnsignedLong(buf.getInt()); // bytes 4–7

        // Cap to prevent DoS: at minimum 1 byte per block.
        long maxPossible = data.length - 8L;
        if (blockCountLong > maxPossible) {
            blockCountLong = maxPossible;
        }
        int blockCount = (int) blockCountLong;

        List<LzssToken> tokens = new ArrayList<>();
        int pos = 8;

        for (int b = 0; b < blockCount; b++) {
            if (pos >= data.length) break;

            byte flag = data[pos++];

            for (int bit = 0; bit < 8 && pos < data.length; bit++) {
                if ((flag & (1 << bit)) != 0) {
                    // Match token: needs 3 bytes.
                    if (pos + 3 > data.length) break;
                    int offset = ((data[pos] & 0xFF) << 8) | (data[pos + 1] & 0xFF);
                    int length = data[pos + 2] & 0xFF;
                    tokens.add(new LzssToken.Match(offset, length));
                    pos += 3;
                } else {
                    // Literal token: needs 1 byte.
                    tokens.add(new LzssToken.Literal(data[pos++]));
                }
            }
        }

        return new DeserialiseResult(tokens, originalLength);
    }

    /**
     * Simple holder returned by {@link #deserialiseTokens}.
     *
     * <p>Java does not have tuples, so we use a private record.  Records are
     * compact classes that generate {@code equals}, {@code hashCode}, and
     * {@code toString} automatically — ideal for small data holders.</p>
     */
    private record DeserialiseResult(List<LzssToken> tokens, int originalLength) {}

    // ─── One-shot public API ──────────────────────────────────────────────────

    /**
     * Compress bytes using LZSS with default parameters, returning the CMP02
     * wire format.
     *
     * <p>This is the main entry point for most callers.  It encodes {@code data}
     * into a token stream and then serialises those tokens to bytes.</p>
     *
     * <pre>{@code
     * byte[] compressed = Lzss.compress("hello hello hello".getBytes(UTF_8));
     * }</pre>
     *
     * @param data the bytes to compress (must not be null)
     * @return CMP02-formatted compressed bytes
     */
    public static byte[] compress(byte[] data) {
        Objects.requireNonNull(data, "data must not be null");
        List<LzssToken> tokens = encode(data, DEFAULT_WINDOW_SIZE, DEFAULT_MAX_MATCH, DEFAULT_MIN_MATCH);
        return serialiseTokens(tokens, data.length);
    }

    /**
     * Decompress bytes produced by {@link #compress}.
     *
     * <pre>{@code
     * byte[] original = Lzss.decompress(compressed);
     * }</pre>
     *
     * @param data CMP02-formatted compressed bytes (must not be null)
     * @return the original uncompressed bytes
     */
    public static byte[] decompress(byte[] data) {
        Objects.requireNonNull(data, "data must not be null");
        DeserialiseResult result = deserialiseTokens(data);
        return decode(result.tokens(), result.originalLength());
    }

    // ─── Parameter validation ─────────────────────────────────────────────────

    /**
     * Validate encoder parameters and throw {@link IllegalArgumentException}
     * with a descriptive message if any are out of range.
     */
    private static void validateParameters(int windowSize, int maxMatch, int minMatch) {
        if (windowSize <= 0) {
            throw new IllegalArgumentException(
                    "windowSize must be positive, got: " + windowSize);
        }
        if (maxMatch <= 0) {
            throw new IllegalArgumentException(
                    "maxMatch must be positive, got: " + maxMatch);
        }
        if (minMatch <= 0) {
            throw new IllegalArgumentException(
                    "minMatch must be positive, got: " + minMatch);
        }
    }
}
