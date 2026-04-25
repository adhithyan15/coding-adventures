// ============================================================================
// LZW.java — CMP03: LZW Lossless Compression Algorithm (1984)
// ============================================================================
//
// LZW (Lempel-Ziv-Welch, 1984) is LZ78 with one key change: the dictionary is
// pre-seeded with all 256 single-byte sequences before encoding begins.  This
// means:
//
//   1. The encoder never needs to emit a raw byte outside the code stream —
//      every byte already has a code (0–255).
//   2. Tokens are just codes (unsigned integers), not (dict_index, next_char)
//      tuples like LZ78.
//   3. With only codes to transmit, the stream can be bit-packed at variable
//      width — codes start at 9 bits and grow as the dictionary expands.
//      This is exactly how GIF compression works.
//
// ============================================================================
// Reserved Codes
// ============================================================================
//
//   0–255:  Pre-seeded. Code c decodes to the single byte c.
//   256:    CLEAR_CODE. Reset the dictionary and code_size.
//   257:    STOP_CODE.  End of stream.
//   258+:   Dynamic entries built during encoding.
//
// ============================================================================
// Variable-Width Bit-Packing (LSB-first)
// ============================================================================
//
// Codes are packed bit-by-bit, LSB-first, into a byte stream.  The code_size
// starts at 9 bits and grows when next_code crosses the next power-of-2
// boundary.  This matches GIF and Unix compress conventions.
//
//   Example: writing code 421 (0b110100101, 9 bits) then code 2 (0b10, 2 bits):
//
//     buffer = 0b110100101          (9 bits)
//     byte 0 = 0b10100101           (low 8 bits)
//     buffer = 0b1                  (1 bit remaining)
//     write 2 = 0b10 → buffer = 0b101
//     (flush) byte 1 = 0b101
//
// Maximum code_size = 16 (dictionary capped at 65536 entries).
//
// ============================================================================
// The Tricky Token
// ============================================================================
//
// During decoding there is a classic edge case: the decoder may receive a code
// equal to next_code — a code it has not yet added to its dictionary.  This
// happens when the encoded sequence has the form xyx...x (the new entry starts
// with the same byte as the previous entry).  In that case:
//
//   entry = dict[prev_code] + byte { dict[prev_code][0] }
//
// This always produces the correct sequence because the encoder only emits such
// a code when the new entry equals the previous entry extended by its own first
// byte.
//
// ============================================================================
// Wire Format (CMP03)
// ============================================================================
//
//   Bytes 0–3:  original_length  (big-endian uint32)
//   Bytes 4+:   bit-packed variable-width codes, LSB-first within each byte
//
//     - Starts at code_size = 9 bits
//     - Grows when next_code crosses the next power-of-2 boundary
//     - Maximum code_size = 16 (up to 65536 dictionary entries)
//     - Stream always begins with CLEAR_CODE and ends with STOP_CODE
//
// ============================================================================
// The CMP Series
// ============================================================================
//
//   CMP00 (LZ77,    1977) — Sliding-window back-references.
//   CMP01 (LZ78,    1978) — Explicit dictionary (trie), no sliding window.
//   CMP02 (LZSS,    1982) — LZ77 + flag bits; eliminates wasted literals.
//   CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; powers GIF. (this)
//   CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
//   CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.
//
// ============================================================================
// References
// ============================================================================
//
//   Welch, T.A. (1984). "A Technique for High-Performance Data Compression".
//   Computer, 17(6), 8–19. doi:10.1109/MC.1984.1659158
//
//   Ziv, J., & Lempel, A. (1978). "Compression of Individual Sequences via
//   Variable-Rate Coding". IEEE Transactions on Information Theory, 24(5),
//   530–536.
//
// ============================================================================

package com.codingadventures.lzw;

import java.io.ByteArrayOutputStream;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * CMP03: LZW lossless compression.
 *
 * <p>Provides {@link #compress(byte[])} and {@link #decompress(byte[])} for
 * one-shot compression.
 *
 * <pre>{@code
 * byte[] original   = "hello hello hello".getBytes();
 * byte[] compressed = LZW.compress(original);
 * byte[] recovered  = LZW.decompress(compressed);
 * assert Arrays.equals(original, recovered);
 * }</pre>
 */
public final class LZW {

    // =========================================================================
    // Constants
    // =========================================================================

    /** Reset code — instructs the decoder to clear its dictionary and restart. */
    public static final int CLEAR_CODE = 256;

    /** End-of-stream code — the decoder stops reading after this code. */
    public static final int STOP_CODE = 257;

    /** First dynamically assigned dictionary code. */
    public static final int INITIAL_NEXT_CODE = 258;

    /** Starting bit-width for codes (covers 0–511, more than enough for 258). */
    public static final int INITIAL_CODE_SIZE = 9;

    /** Maximum bit-width; dictionary caps at 2^16 = 65536 entries. */
    public static final int MAX_CODE_SIZE = 16;

    /**
     * Maximum decompressed output size (64 MiB by default).
     *
     * <p>A fully-saturated LZW dictionary (65536 entries, each one byte longer
     * than the last) can theoretically require ~2 GB of heap.  This limit caps
     * the output to prevent heap exhaustion from crafted streams.  Call
     * {@link #decodeCodes(List, int)} directly with a higher limit when needed.
     */
    public static final int DEFAULT_MAX_OUTPUT = 64 * 1024 * 1024; // 64 MiB

    /** Wire-format header size: original_length(4). */
    private static final int HEADER_SIZE = 4;

    /** Utility class — no instances. */
    private LZW() {}

    // =========================================================================
    // Public API
    // =========================================================================

    /**
     * Compress {@code data} using LZW and return CMP03 wire-format bytes.
     *
     * @param data the bytes to compress (null treated as empty)
     * @return compressed bytes in CMP03 wire format
     */
    public static byte[] compress(byte[] data) {
        if (data == null) data = new byte[0];
        List<Integer> codes = encodeCodes(data);
        return packCodes(codes, data.length);
    }

    /**
     * Decompress CMP03 wire-format bytes back to the original data.
     *
     * <p>Output is capped at {@link #DEFAULT_MAX_OUTPUT} bytes to prevent heap
     * exhaustion from crafted streams.
     *
     * @param data compressed bytes (null treated as empty)
     * @return the original, uncompressed bytes
     */
    public static byte[] decompress(byte[] data) {
        if (data == null || data.length < HEADER_SIZE) return new byte[0];
        int originalLength = ByteBuffer.wrap(data, 0, HEADER_SIZE)
            .order(ByteOrder.BIG_ENDIAN).getInt();
        List<Integer> codes = unpackCodes(data);
        byte[] result = decodeCodes(codes, DEFAULT_MAX_OUTPUT);
        // Trim to original length to remove bit-padding artefacts.
        if (originalLength >= 0 && originalLength <= result.length) {
            return Arrays.copyOf(result, originalLength);
        }
        return result;
    }

    // =========================================================================
    // Encoder
    // =========================================================================

    /**
     * Encode {@code data} into a list of LZW codes (including CLEAR and STOP).
     *
     * <p>Algorithm:
     * <ol>
     *   <li>Initialise the encode dictionary: byte → code for all 256 bytes.</li>
     *   <li>Emit CLEAR_CODE to mark the start of the stream.</li>
     *   <li>Walk the input byte-by-byte, extending the current prefix {@code w}:
     *       <ul>
     *         <li>If {@code w + b} is already in the dictionary, extend {@code w}.</li>
     *         <li>Otherwise, emit code_for({@code w}), add {@code w + b} as a new
     *             entry, reset {@code w} to just {@code b}.</li>
     *         <li>When the dictionary is full, emit CLEAR_CODE and re-initialise.</li>
     *       </ul>
     *   </li>
     *   <li>Flush the remaining prefix and emit STOP_CODE.</li>
     * </ol>
     *
     * @param data the bytes to encode
     * @return list of LZW codes
     */
    static List<Integer> encodeCodes(byte[] data) {
        List<Integer> codes = new ArrayList<>();
        int maxEntries = 1 << MAX_CODE_SIZE; // 65536

        // Build encode dictionary: byte[] → code (using ByteKey wrapper).
        Map<ByteKey, Integer> encodeDict = new HashMap<>(512);
        for (int b = 0; b < 256; b++) {
            encodeDict.put(new ByteKey(new byte[]{(byte) b}), b);
        }
        int nextCode = INITIAL_NEXT_CODE;

        codes.add(CLEAR_CODE);

        byte[] w = new byte[0]; // current working prefix

        for (byte rawByte : data) {
            byte[] wb = append(w, rawByte);
            if (encodeDict.containsKey(new ByteKey(wb))) {
                w = wb; // extend the prefix
            } else {
                // Emit code for the current prefix.
                codes.add(encodeDict.get(new ByteKey(w)));

                if (nextCode < maxEntries) {
                    encodeDict.put(new ByteKey(wb), nextCode);
                    nextCode++;
                } else {
                    // Dictionary full — emit CLEAR and reset.
                    codes.add(CLEAR_CODE);
                    encodeDict.clear();
                    for (int b = 0; b < 256; b++) {
                        encodeDict.put(new ByteKey(new byte[]{(byte) b}), b);
                    }
                    nextCode = INITIAL_NEXT_CODE;
                }

                w = new byte[]{rawByte}; // restart with the unmatched byte
            }
        }

        // Flush remaining prefix.
        if (w.length > 0) {
            codes.add(encodeDict.get(new ByteKey(w)));
        }

        codes.add(STOP_CODE);
        return codes;
    }

    // =========================================================================
    // Decoder
    // =========================================================================

    /**
     * Decode a list of LZW codes back to the original bytes.
     *
     * <p>Uses {@link #DEFAULT_MAX_OUTPUT} as the output size limit.
     *
     * @param codes the code list from {@link #encodeCodes(byte[])}
     * @return the decoded bytes
     * @throws IllegalArgumentException if the code stream is corrupt or too large
     */
    static byte[] decodeCodes(List<Integer> codes) {
        return decodeCodes(codes, DEFAULT_MAX_OUTPUT);
    }

    /**
     * Decode a list of LZW codes back to the original bytes, with an output cap.
     *
     * <p>Handles:
     * <ul>
     *   <li>CLEAR_CODE: reset dictionary and code_size.</li>
     *   <li>STOP_CODE: stop decoding.</li>
     *   <li>Tricky token (code == next_code): construct the entry as
     *       {@code dict[prev_code] + byte{dict[prev_code][0]}}.</li>
     * </ul>
     *
     * <p>Security: without a size cap, a crafted stream that never emits
     * CLEAR_CODE can force the decoder to build up to 65278 dictionary entries,
     * each growing by 1 byte.  Total worst-case allocation ≈ 2 GB.  The
     * {@code maxOutput} limit stops decoding before heap exhaustion occurs.
     *
     * @param codes     the code list from {@link #encodeCodes(byte[])}
     * @param maxOutput maximum bytes to emit; throw if exceeded
     * @return the decoded bytes
     * @throws IllegalArgumentException if the code stream is corrupt or output exceeds maxOutput
     */
    static byte[] decodeCodes(List<Integer> codes, int maxOutput) {
        // Decode dictionary: code → byte sequence.
        List<byte[]> decodeDict = new ArrayList<>(512);
        for (int b = 0; b < 256; b++) {
            decodeDict.add(new byte[]{(byte) b});
        }
        decodeDict.add(new byte[0]); // 256 = CLEAR_CODE placeholder
        decodeDict.add(new byte[0]); // 257 = STOP_CODE  placeholder
        int nextCode = INITIAL_NEXT_CODE;

        ByteArrayOutputStream out = new ByteArrayOutputStream();
        Integer prevCode = null;

        for (int code : codes) {
            if (code == CLEAR_CODE) {
                // Reset dictionary to 256 single-byte entries.
                decodeDict.clear();
                for (int b = 0; b < 256; b++) {
                    decodeDict.add(new byte[]{(byte) b});
                }
                decodeDict.add(new byte[0]); // 256
                decodeDict.add(new byte[0]); // 257
                nextCode = INITIAL_NEXT_CODE;
                prevCode = null;
                continue;
            }

            if (code == STOP_CODE) {
                break;
            }

            byte[] entry;
            if (code < decodeDict.size()) {
                entry = decodeDict.get(code);
            } else if (code == nextCode) {
                // Tricky token: code not yet in dict.
                // Only valid when prev_code exists; entry = dict[prev] + dict[prev][0].
                if (prevCode == null) {
                    throw new IllegalArgumentException(
                        "Invalid LZW stream: tricky token with no previous code");
                }
                byte[] prevEntry = decodeDict.get(prevCode);
                entry = append(prevEntry, prevEntry[0]);
            } else {
                throw new IllegalArgumentException(
                    "Invalid LZW code: exceeds expected next code");
            }

            // Guard against heap exhaustion from crafted streams.
            if (out.size() + entry.length > maxOutput) {
                throw new IllegalArgumentException(
                    "LZW: decompressed output exceeds limit of " + maxOutput + " bytes");
            }

            try { out.write(entry); } catch (Exception e) { throw new RuntimeException(e); }

            // Add new entry to the decode dictionary.
            if (prevCode != null && nextCode < (1 << MAX_CODE_SIZE)) {
                byte[] prevEntry = decodeDict.get(prevCode);
                decodeDict.add(append(prevEntry, entry[0]));
                nextCode++;
            }

            prevCode = code;
        }

        return out.toByteArray();
    }

    // =========================================================================
    // Serialisation
    // =========================================================================

    /**
     * Pack a list of LZW codes into the CMP03 wire format.
     *
     * <p>Wire format:
     * <ul>
     *   <li>Bytes 0–3: original_length (big-endian uint32)</li>
     *   <li>Bytes 4+: codes as variable-width LSB-first bit-packed integers</li>
     * </ul>
     *
     * <p>The code size starts at {@link #INITIAL_CODE_SIZE} (9) and grows each
     * time next_code crosses the next power-of-2 boundary.  CLEAR_CODE resets
     * the code size back to {@link #INITIAL_CODE_SIZE}.
     *
     * @param codes          the code list
     * @param originalLength length of the original uncompressed data
     * @return CMP03 wire-format bytes
     */
    static byte[] packCodes(List<Integer> codes, int originalLength) {
        BitWriter writer = new BitWriter();
        int codeSize = INITIAL_CODE_SIZE;
        int nextCode = INITIAL_NEXT_CODE;

        for (int code : codes) {
            writer.write(code, codeSize);

            if (code == CLEAR_CODE) {
                codeSize = INITIAL_CODE_SIZE;
                nextCode = INITIAL_NEXT_CODE;
            } else if (code != STOP_CODE) {
                if (nextCode < (1 << MAX_CODE_SIZE)) {
                    nextCode++;
                    if (nextCode > (1 << codeSize) && codeSize < MAX_CODE_SIZE) {
                        codeSize++;
                    }
                }
            }
        }
        writer.flush();

        ByteBuffer header = ByteBuffer.allocate(HEADER_SIZE).order(ByteOrder.BIG_ENDIAN);
        header.putInt(originalLength);

        ByteArrayOutputStream out = new ByteArrayOutputStream();
        try {
            out.write(header.array());
            out.write(writer.toByteArray());
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
        return out.toByteArray();
    }

    /**
     * Unpack CMP03 wire-format bytes into a list of LZW codes.
     *
     * <p>The decoder stops on STOP_CODE or stream exhaustion, so a crafted
     * stream cannot cause unbounded iteration.
     *
     * @param data the CMP03 bytes
     * @return list of LZW codes
     */
    static List<Integer> unpackCodes(byte[] data) {
        if (data.length < HEADER_SIZE) {
            return List.of(CLEAR_CODE, STOP_CODE);
        }

        BitReader reader = new BitReader(data, HEADER_SIZE);
        List<Integer> codes = new ArrayList<>();
        int codeSize = INITIAL_CODE_SIZE;
        int nextCode = INITIAL_NEXT_CODE;

        while (!reader.exhausted()) {
            if (!reader.hasEnough(codeSize)) break;
            int code = reader.read(codeSize);
            codes.add(code);

            if (code == STOP_CODE) {
                break;
            } else if (code == CLEAR_CODE) {
                codeSize = INITIAL_CODE_SIZE;
                nextCode = INITIAL_NEXT_CODE;
            } else {
                if (nextCode < (1 << MAX_CODE_SIZE)) {
                    nextCode++;
                    if (nextCode > (1 << codeSize) && codeSize < MAX_CODE_SIZE) {
                        codeSize++;
                    }
                }
            }
        }

        return codes;
    }

    // =========================================================================
    // Bit I/O
    // =========================================================================

    /**
     * Accumulates variable-width codes into a byte buffer, LSB-first.
     *
     * <p>LSB-first packing: the first code written occupies bits 0..N-1 of the
     * first byte, spilling into subsequent bytes as necessary.  This matches the
     * GIF and Unix compress conventions.
     */
    static final class BitWriter {
        private long buffer = 0L;
        private int  bitPos = 0;
        private final ByteArrayOutputStream out = new ByteArrayOutputStream();

        /**
         * Write {@code code} using exactly {@code codeSize} bits.
         *
         * @param code     the value to write
         * @param codeSize number of bits to write (1..24)
         */
        void write(int code, int codeSize) {
            buffer |= ((long) code) << bitPos;
            bitPos += codeSize;
            while (bitPos >= 8) {
                out.write((int)(buffer & 0xFF));
                buffer >>>= 8;
                bitPos -= 8;
            }
        }

        /** Flush any remaining bits as a final partial byte. */
        void flush() {
            if (bitPos > 0) {
                out.write((int)(buffer & 0xFF));
                buffer = 0;
                bitPos = 0;
            }
        }

        /** Return the accumulated output. */
        byte[] toByteArray() { return out.toByteArray(); }
    }

    /**
     * Reads variable-width codes from a byte buffer, LSB-first.
     *
     * <p>Mirrors {@link BitWriter} exactly: bits within each byte are consumed
     * from the least-significant end first.
     */
    static final class BitReader {
        private final byte[] data;
        private int  pos;     // next byte index
        private long buffer = 0L;
        private int  bitPos = 0; // number of valid bits in buffer

        BitReader(byte[] data, int startPos) {
            this.data = data;
            this.pos  = startPos;
        }

        /**
         * Read and return the next {@code codeSize}-bit code.
         *
         * @param codeSize number of bits to read
         * @return the decoded code value
         */
        int read(int codeSize) {
            while (bitPos < codeSize) {
                buffer |= ((long)(data[pos++] & 0xFF)) << bitPos;
                bitPos += 8;
            }
            int code = (int)(buffer & ((1L << codeSize) - 1));
            buffer >>>= codeSize;
            bitPos -= codeSize;
            return code;
        }

        /** True when the byte stream is exhausted and no buffered bits remain. */
        boolean exhausted() { return pos >= data.length && bitPos == 0; }

        /** True when at least {@code n} bits can still be read. */
        boolean hasEnough(int n) {
            int available = bitPos + (data.length - pos) * 8;
            return available >= n;
        }
    }

    // =========================================================================
    // Private helpers
    // =========================================================================

    /** Append a single byte to the end of an array. */
    private static byte[] append(byte[] arr, byte b) {
        byte[] result = Arrays.copyOf(arr, arr.length + 1);
        result[arr.length] = b;
        return result;
    }

    /**
     * A byte-array wrapper with value-based equals/hashCode.
     *
     * <p>Used as a HashMap key because raw {@code byte[]} uses identity equality.
     */
    static final class ByteKey {
        private final byte[] data;

        ByteKey(byte[] data) { this.data = data; }

        @Override
        public boolean equals(Object o) {
            if (this == o) return true;
            if (!(o instanceof ByteKey bk)) return false;
            return Arrays.equals(data, bk.data);
        }

        @Override
        public int hashCode() { return Arrays.hashCode(data); }
    }
}
