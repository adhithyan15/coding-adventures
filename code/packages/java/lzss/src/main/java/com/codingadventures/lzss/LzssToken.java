package com.codingadventures.lzss;

/**
 * A single LZSS token: either a literal byte or a back-reference match.
 *
 * <p>LZSS replaces LZ77's mandatory "next character after every reference" with
 * a smarter flag-bit scheme.  Each token is one of two shapes:</p>
 *
 * <ul>
 *   <li>{@link Literal} — one raw byte that had no useful match in the
 *       look-back window.  Costs exactly 1 byte on the wire.</li>
 *   <li>{@link Match}   — a back-reference saying "go {@code offset} bytes
 *       backward in what we already decompressed and copy {@code length} bytes
 *       from there".  Costs 3 bytes on the wire (2-byte BE offset + 1-byte
 *       length), so it only wins over a literal when {@code length >= 3}.</li>
 * </ul>
 *
 * <p>Java 17+ {@code sealed interface} + {@code record} syntax lets the
 * compiler enforce that {@code LzssToken} has exactly these two variants and
 * nothing else — the same guarantee Rust gives with {@code enum}.</p>
 */
public sealed interface LzssToken permits LzssToken.Literal, LzssToken.Match {

    /**
     * A single raw byte that could not be matched in the look-back window.
     *
     * <p>Example: the first occurrence of any byte in the stream is always a
     * literal, because there is nothing in the window to reference yet.</p>
     *
     * @param value the raw byte value
     */
    record Literal(byte value) implements LzssToken {}

    /**
     * A back-reference into previously decoded output.
     *
     * <p>To decode: go {@code offset} positions back from the current write
     * position, then copy {@code length} bytes forward byte-by-byte.
     * "Byte-by-byte" matters for <em>overlapping matches</em>: if
     * {@code offset == 1} and {@code length == 6} the effect is a run-length
     * expansion (e.g., "A" → "AAAAAAA").</p>
     *
     * <p>Invariants enforced by the encoder:</p>
     * <ul>
     *   <li>{@code offset >= 1} (cannot reference the byte we are writing)</li>
     *   <li>{@code offset <= windowSize} (cannot look further back than the window)</li>
     *   <li>{@code length >= minMatch} (otherwise a literal is cheaper)</li>
     *   <li>{@code length <= maxMatch} (fits in one unsigned byte: 1–255)</li>
     * </ul>
     *
     * @param offset distance back in the output buffer (1-based, big-endian u16 on wire)
     * @param length number of bytes to copy (fits in u8: 1–255)
     */
    record Match(int offset, int length) implements LzssToken {}
}
