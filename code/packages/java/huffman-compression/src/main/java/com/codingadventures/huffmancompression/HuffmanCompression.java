// ============================================================================
// HuffmanCompression.java — CMP04: Huffman Lossless Compression
// ============================================================================
//
// Huffman coding (1952) is an entropy coding algorithm: it assigns
// variable-length, prefix-free binary codes to symbols based on their
// frequency of occurrence.  Frequent symbols get short codes; rare symbols
// get long codes.  The resulting code is provably optimal — no other
// prefix-free code can achieve a smaller expected bit-length for the same
// symbol distribution.
//
// Unlike the LZ-family algorithms (LZ77, LZ78, LZSS, LZW) which exploit
// repetition (duplicate substrings), Huffman coding exploits symbol
// statistics.  This makes it complementary to LZ compression.  DEFLATE
// combines both: LZ77 to eliminate repeated substrings, then Huffman to
// optimally encode the remaining symbol stream.
//
// ============================================================================
// Dependency on DT27 (HuffmanTree)
// ============================================================================
//
// This package does NOT build its own Huffman tree.  It delegates all tree
// construction and code derivation to com.codingadventures.huffmantree.
// This mirrors the pattern used by LZ78 which delegates trie operations to
// the trie package.
//
//   CMP04 (Huffman) → uses DT27 (HuffmanTree) for code construction/decode
//
// ============================================================================
// Wire Format (CMP04)
// ============================================================================
//
//   Bytes 0–3:    original_length  (big-endian uint32)
//   Bytes 4–7:    symbol_count     (big-endian uint32) — distinct symbols
//   Bytes 8–8+2N: code-lengths table — N entries, each 2 bytes:
//                   [0]  symbol value  (uint8, 0–255)
//                   [1]  code length   (uint8, 1–16)
//                 Sorted by (code_length, symbol_value) ascending.
//   Bytes 8+2N+:  bit stream — packed LSB-first, zero-padded to byte boundary.
//
// Packing convention: LSB-first (same as LZW/GIF).
//
//   Example — packing bit string "000101011" (9 bits):
//     Byte 0: bits[0..7] = 0b10101000 = 0xA8
//       bit 0 ('0') → byte bit 0  (LSB)
//       bit 1 ('0') → byte bit 1
//       ...
//       bit 7 ('1') → byte bit 7  (MSB)
//     Byte 1: bit[8] ('1') → 0b00000001 = 0x01
//
// ============================================================================
// The CMP series
// ============================================================================
//
//   CMP00 (LZ77,    1977) — Sliding-window back-references.
//   CMP01 (LZ78,    1978) — Explicit dictionary (trie), no sliding window.
//   CMP02 (LZSS,    1982) — LZ77 + flag bits; eliminates wasted literals.
//   CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; powers GIF.
//   CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE. (this)
//   CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.
//
// ============================================================================

package com.codingadventures.huffmancompression;

import com.codingadventures.huffmantree.HuffmanTree;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * CMP04: Huffman lossless compression.
 *
 * <p>Provides {@link #compress(byte[])} and {@link #decompress(byte[])} methods
 * that encode/decode byte arrays using canonical Huffman codes.  The compressed
 * bytes follow the CMP04 wire format.
 *
 * <pre>{@code
 * byte[] original   = "AAABBC".getBytes();
 * byte[] compressed = HuffmanCompression.compress(original);
 * byte[] recovered  = HuffmanCompression.decompress(compressed);
 * assert Arrays.equals(original, recovered);
 * }</pre>
 */
public final class HuffmanCompression {

    /** Utility class — no instances. */
    private HuffmanCompression() {}

    /** Symbols are single bytes, so a valid table has at most 256 entries. */
    private static final int MAX_SYMBOLS = 256;

    /** Longest code length the decoder accepts; keeps every 64-bit shift safe. */
    private static final int MAX_CODE_LEN = 56;

    // =========================================================================
    // Public API
    // =========================================================================

    /**
     * Compress {@code data} using canonical Huffman coding.
     *
     * <p>Algorithm:
     * <ol>
     *   <li>Count byte frequencies (histogram).</li>
     *   <li>Build a Huffman tree via {@link HuffmanTree#build}.</li>
     *   <li>Obtain canonical codes via {@link HuffmanTree#canonicalCodeTable}.</li>
     *   <li>Build the code-lengths table sorted by (length, symbol).</li>
     *   <li>Encode each byte using its canonical bit string.</li>
     *   <li>Pack the bit string LSB-first into bytes.</li>
     *   <li>Assemble header + code-lengths table + bit stream.</li>
     * </ol>
     *
     * <p>Edge cases:
     * <ul>
     *   <li>Empty input: returns an 8-byte header (original_length=0,
     *       symbol_count=0) with no bit stream.</li>
     *   <li>Single distinct byte: DT27 assigns code "0"; each occurrence
     *       encodes to 1 bit.</li>
     * </ul>
     *
     * @param data the bytes to compress
     * @return compressed bytes in CMP04 wire format
     */
    public static byte[] compress(byte[] data) {
        // Empty input — 8-byte header only.
        if (data == null || data.length == 0) {
            ByteBuffer buf = ByteBuffer.allocate(8).order(ByteOrder.BIG_ENDIAN);
            buf.putInt(0).putInt(0);
            return buf.array();
        }

        int originalLength = data.length;

        // Step 1: Count frequencies.
        int[] freq = new int[256];
        for (byte b : data) freq[b & 0xFF]++;

        // Step 2: Build Huffman tree.
        List<int[]> weights = new ArrayList<>();
        for (int sym = 0; sym < 256; sym++) {
            if (freq[sym] > 0) weights.add(new int[]{sym, freq[sym]});
        }
        HuffmanTree tree = HuffmanTree.build(weights);

        // Step 3: Canonical code table {symbol → bit_string}.
        Map<Integer, String> table = tree.canonicalCodeTable();

        // Step 4: Code-lengths list sorted by (length, symbol) for the header.
        List<int[]> lengths = new ArrayList<>();
        for (Map.Entry<Integer, String> e : table.entrySet()) {
            lengths.add(new int[]{e.getKey(), e.getValue().length()});
        }
        lengths.sort((a, b) -> a[1] != b[1] ? Integer.compare(a[1], b[1])
                                             : Integer.compare(a[0], b[0]));

        // Step 5: Encode each byte using its canonical code.
        StringBuilder bits = new StringBuilder();
        for (byte b : data) bits.append(table.get(b & 0xFF));

        // Step 6: Pack bits LSB-first into bytes.
        byte[] bitBytes = packBitsLsbFirst(bits.toString());

        // Step 7: Assemble wire format.
        //   header:             original_length (4B) + symbol_count (4B)
        //   code-lengths table: symbol_count × 2 bytes
        //   bit stream:         variable
        int symbolCount = lengths.size();
        ByteBuffer header = ByteBuffer.allocate(8).order(ByteOrder.BIG_ENDIAN);
        header.putInt(originalLength).putInt(symbolCount);

        ByteArrayOutputStream out = new ByteArrayOutputStream();
        try {
            out.write(header.array());
            for (int[] entry : lengths) {
                out.write(entry[0]);   // symbol
                out.write(entry[1]);   // code length
            }
            out.write(bitBytes);
        } catch (IOException e) {
            throw new UncheckedIOException(e);
        }
        return out.toByteArray();
    }

    /**
     * Decompress CMP04 wire-format {@code data} and return the original bytes.
     *
     * <p>Algorithm:
     * <ol>
     *   <li>Parse the 8-byte header: original_length and symbol_count.</li>
     *   <li>Parse the code-lengths table (symbol_count × 2 bytes).</li>
     *   <li>Reconstruct canonical codes from the sorted (symbol, length) list.</li>
     *   <li>Unpack the LSB-first bit stream.</li>
     *   <li>Decode original_length symbols by accumulating bits until a hit in
     *       the canonical code table, then reset.</li>
     * </ol>
     *
     * @param data compressed bytes in CMP04 wire format
     * @return the original, uncompressed bytes
     * @throws IllegalArgumentException if the bit stream is exhausted before
     *                                  all symbols are decoded
     */
    public static byte[] decompress(byte[] data) {
        if (data == null || data.length < 8) return new byte[0];

        ByteBuffer buf = ByteBuffer.wrap(data).order(ByteOrder.BIG_ENDIAN);
        // Read the header as unsigned: a signed int would turn 0xFFFFFFFF into -1.
        long originalLength = buf.getInt() & 0xFFFFFFFFL;
        long symbolCount    = buf.getInt() & 0xFFFFFFFFL;

        if (originalLength == 0) return new byte[0];

        // SECURITY: the header is attacker-controlled.  Symbols are single
        // bytes, so a table can name at most 256 of them.  Checking first also
        // keeps every size computation below tiny.
        if (symbolCount > MAX_SYMBOLS) {
            throw new IllegalArgumentException(
                "symbol_count " + symbolCount + " exceeds " + MAX_SYMBOLS);
        }
        int tableEnd = 8 + (int) symbolCount * 2;
        if (data.length < tableEnd) {
            throw new IllegalArgumentException("input too short for the code-lengths table");
        }

        // Parse code-lengths table.
        // Each entry: 2 bytes — symbol (uint8), code_length (uint8).
        // Wire format guarantees entries are sorted by (code_length, symbol_value).
        // A byte value can have only one code, so a repeated symbol is rejected.
        List<int[]> lengths = new ArrayList<>((int) symbolCount);
        boolean[] seen = new boolean[256];
        for (int i = 0; i < symbolCount; i++) {
            int sym    = buf.get() & 0xFF;
            int length = buf.get() & 0xFF;
            if (seen[sym]) {
                throw new IllegalArgumentException(
                    "symbol " + sym + " appears twice in code-lengths table");
            }
            seen[sym] = true;
            lengths.add(new int[]{sym, length});
        }

        // Reconstruct canonical codes from the sorted lengths list.
        //
        // The canonical reconstruction rule (same as DEFLATE):
        //   code = 0
        //   prev_length = first entry's length
        //   for each entry in sorted order:
        //     if length > prev_length: code <<= (length - prev_length)
        //     assign (length, code) -> symbol
        //     code += 1
        //
        // This produces a prefix-free code table that matches the encoder exactly.
        //
        // SECURITY: the table is hostile input, so we reject
        //   * lengths of 0 or above MAX_CODE_LEN (a zero-length code matches
        //     nothing; a long one could overflow the 64-bit shift),
        //   * tables not sorted by length (the encoder always sorts), and
        //   * over-subscribed tables (Kraft sum > 1): before each assignment the
        //     running code must still fit in `length` bits (code >>> length == 0).
        // Since length <= 56 the long shifts below never have their count masked.
        // Codes are keyed by (length << 56 | code) so no strings are built.
        Map<Long, Integer> codeToSymbol = new HashMap<>();
        long codeVal = 0;
        int prevLen  = lengths.isEmpty() ? 0 : lengths.get(0)[1];
        int maxLen   = 0;
        for (int[] entry : lengths) {
            int sym    = entry[0];
            int length = entry[1];
            if (length == 0 || length > MAX_CODE_LEN) {
                throw new IllegalArgumentException("invalid code length " + length + " in table");
            }
            if (length < prevLen) {
                throw new IllegalArgumentException("code-lengths table is not sorted by length");
            }
            codeVal <<= (length - prevLen);
            if ((codeVal >>> length) != 0) {
                throw new IllegalArgumentException(
                    "code-lengths table over-subscribes the code space");
            }
            codeToSymbol.put(((long) length << 56) | codeVal, sym);
            codeVal++;
            prevLen = length;
            maxLen = length;
        }

        // SECURITY: every code is at least one bit, so N payload bytes hold at
        // most 8*N symbols.  Reject a larger declared length before allocating
        // anything sized from it (also guards the int array size).
        long totalBits = (long) (data.length - tableEnd) * 8;
        if (originalLength > totalBits || originalLength > Integer.MAX_VALUE - 8) {
            throw new IllegalArgumentException(
                "declared length " + originalLength + " exceeds the " + totalBits
                + " bits available");
        }

        // Decode exactly original_length symbols straight from the payload.
        //
        // Bits are LSB-first within each byte, so bit i is
        // (data[tableEnd + i/8] >> (i%8)) & 1 -- O(1), no expanded bit string.
        // The candidate code is built MSB-first as a long.  Canonical codes are
        // prefix-free, so the first (length, value) hit is the symbol.  Once the
        // accumulated length reaches the longest code without a match, no later
        // bit can match (an incomplete table fed unmatched bits): fail fast.
        byte[] output = new byte[(int) originalLength];
        long pos = 0;
        long acc = 0;
        int accLen = 0;
        for (int decoded = 0; decoded < output.length; ) {
            if (pos >= totalBits) {
                throw new IllegalArgumentException(
                    "Bit stream exhausted before decoding all symbols"
                );
            }
            int bit = (data[tableEnd + (int) (pos >>> 3)] >> (int) (pos & 7)) & 1;
            pos++;
            acc = (acc << 1) | bit;
            accLen++;
            Integer sym = codeToSymbol.get(((long) accLen << 56) | acc);
            if (sym != null) {
                output[decoded++] = (byte) (int) sym;
                acc = 0;
                accLen = 0;
            } else if (accLen >= maxLen) {
                throw new IllegalArgumentException("bit pattern does not match any code");
            }
        }
        return output;
    }

    // =========================================================================
    // Private helpers
    // =========================================================================

    /**
     * Pack a bit string into bytes, filling each byte from bit 0 (LSB) upward.
     *
     * <p>This is the same convention used by LZW and GIF: the first bit of the
     * stream occupies the least-significant bit of the first byte.
     *
     * <p>The final partial byte is zero-padded in the high bits.
     *
     * <p>Example — packing "000101011" (9 bits):
     * <pre>
     * Byte 0: bits[0..7] = 0b10101000 = 0xA8
     *   bit 0 ('0') → bit 0  (LSB)
     *   bit 3 ('1') → bit 3
     *   bit 5 ('1') → bit 5
     *   bit 7 ('1') → bit 7
     * Byte 1: bit[8] ('1') → 0b00000001 = 0x01
     * </pre>
     *
     * @param bits a string of '0' and '1' characters
     * @return the packed bytes
     */
    private static byte[] packBitsLsbFirst(String bits) {
        int byteCount = (bits.length() + 7) / 8;
        byte[] output = new byte[byteCount];
        for (int i = 0; i < bits.length(); i++) {
            if (bits.charAt(i) == '1') {
                int byteIdx = i / 8;
                int bitPos  = i % 8;
                output[byteIdx] |= (byte) (1 << bitPos);
            }
        }
        return output;
    }
}
