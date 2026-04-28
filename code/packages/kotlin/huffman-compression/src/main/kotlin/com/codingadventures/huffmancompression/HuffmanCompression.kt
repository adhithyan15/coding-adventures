// ============================================================================
// HuffmanCompression.kt — CMP04: Huffman Lossless Compression
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

package com.codingadventures.huffmancompression

import com.codingadventures.huffmantree.HuffmanTree
import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import java.nio.ByteOrder

/**
 * CMP04: Huffman lossless compression.
 *
 * Provides [compress] and [decompress] functions that encode/decode byte
 * arrays using canonical Huffman codes.  The compressed bytes follow the
 * CMP04 wire format (see module comment above).
 *
 * ```kotlin
 * val original   = "AAABBC".toByteArray()
 * val compressed = compress(original)
 * val recovered  = decompress(compressed)
 * assert(original.contentEquals(recovered))
 * ```
 */
object HuffmanCompression {

    // =========================================================================
    // Public API
    // =========================================================================

    /**
     * Compress [data] using canonical Huffman coding.
     *
     * Algorithm:
     * 1. Count byte frequencies (histogram).
     * 2. Build a Huffman tree via [HuffmanTree.build].
     * 3. Obtain canonical codes via [HuffmanTree.canonicalCodeTable].
     * 4. Build the code-lengths list sorted by (length, symbol) for the header.
     * 5. Encode each byte using its canonical bit string.
     * 6. Pack the bit string LSB-first into bytes.
     * 7. Assemble header + code-lengths table + bit stream.
     *
     * Edge cases:
     * - Empty / null input: returns an 8-byte header (original_length=0,
     *   symbol_count=0) with no bit stream.
     * - Single distinct byte: DT27 assigns code "0"; each occurrence encodes
     *   to 1 bit.
     *
     * @param data the bytes to compress (null treated as empty)
     * @return compressed bytes in CMP04 wire format
     */
    fun compress(data: ByteArray?): ByteArray {
        // Empty input — 8-byte header only.
        if (data == null || data.isEmpty()) {
            return ByteBuffer.allocate(8).order(ByteOrder.BIG_ENDIAN)
                .putInt(0).putInt(0).array()
        }

        val originalLength = data.size

        // Step 1: Count frequencies.
        val freq = IntArray(256)
        for (b in data) freq[b.toInt() and 0xFF]++

        // Step 2: Build Huffman tree.
        val weights = (0 until 256)
            .filter { freq[it] > 0 }
            .map { intArrayOf(it, freq[it]) }
        val tree = HuffmanTree.build(weights)

        // Step 3: Canonical code table {symbol → bit_string}.
        val table = tree.canonicalCodeTable()

        // Step 4: Code-lengths list sorted by (length, symbol) for the header.
        val lengths = table.entries
            .map { (sym, code) -> intArrayOf(sym, code.length) }
            .sortedWith(compareBy({ it[1] }, { it[0] }))

        // Step 5: Encode each byte using its canonical code.
        val bits = buildString {
            for (b in data) append(table.getValue(b.toInt() and 0xFF))
        }

        // Step 6: Pack bits LSB-first into bytes.
        val bitBytes = packBitsLsbFirst(bits)

        // Step 7: Assemble wire format.
        val symbolCount = lengths.size
        val header = ByteBuffer.allocate(8).order(ByteOrder.BIG_ENDIAN)
            .putInt(originalLength).putInt(symbolCount).array()

        val out = ByteArrayOutputStream()
        out.write(header)
        for (entry in lengths) {
            out.write(entry[0])   // symbol
            out.write(entry[1])   // code length
        }
        out.write(bitBytes)
        return out.toByteArray()
    }

    /**
     * Decompress CMP04 wire-format [data] and return the original bytes.
     *
     * Algorithm:
     * 1. Parse the 8-byte header: original_length and symbol_count.
     * 2. Parse the code-lengths table (symbol_count × 2 bytes).
     * 3. Reconstruct canonical codes from the sorted (symbol, length) list.
     * 4. Unpack the LSB-first bit stream.
     * 5. Decode original_length symbols by accumulating bits until a hit in
     *    the canonical code table, then reset.
     *
     * @param data compressed bytes in CMP04 wire format (null treated as empty)
     * @return the original, uncompressed bytes
     * @throws IllegalArgumentException if the bit stream is exhausted before
     *                                  all symbols are decoded
     */
    fun decompress(data: ByteArray?): ByteArray {
        if (data == null || data.size < 8) return ByteArray(0)

        val buf = ByteBuffer.wrap(data).order(ByteOrder.BIG_ENDIAN)
        val originalLength = buf.getInt()
        val symbolCount    = buf.getInt()

        if (originalLength == 0) return ByteArray(0)

        // Parse code-lengths table.
        // Wire format guarantees entries are sorted by (code_length, symbol_value).
        val lengths = ArrayList<IntArray>(symbolCount)
        repeat(symbolCount) {
            val sym    = buf.get().toInt() and 0xFF
            val length = buf.get().toInt() and 0xFF
            lengths.add(intArrayOf(sym, length))
        }

        // Reconstruct canonical codes from the sorted lengths list.
        //
        // The DEFLATE canonical reconstruction rule:
        //   code = 0
        //   prev_length = first entry's length
        //   for each entry in sorted order:
        //     if length > prev_length: code = code shl (length - prev_length)
        //     assign bit_string = left-padded binary of code
        //     code += 1
        val codeToSymbol = HashMap<String, Int>()
        var codeVal = 0
        var prevLen = if (lengths.isEmpty()) 0 else lengths[0][1]

        for ((sym, length) in lengths.map { it[0] to it[1] }) {
            if (length > prevLen) codeVal = codeVal shl (length - prevLen)
            val bitStr = Integer.toBinaryString(codeVal).padStart(length, '0')
            codeToSymbol[bitStr] = sym
            codeVal++
            prevLen = length
        }

        // Unpack the remaining bytes as a bit string (LSB-first).
        val bitString = buildString {
            while (buf.hasRemaining()) {
                val byteVal = buf.get().toInt() and 0xFF
                repeat(8) { i -> append((byteVal shr i) and 1) }
            }
        }

        // Decode exactly original_length symbols.
        val output = ByteArray(originalLength)
        var pos = 0
        val accumulated = StringBuilder()

        var decoded = 0
        while (decoded < originalLength) {
            if (pos >= bitString.length) {
                throw IllegalArgumentException(
                    "Bit stream exhausted after $decoded symbols; expected $originalLength"
                )
            }
            accumulated.append(bitString[pos++])
            val sym = codeToSymbol[accumulated.toString()]
            if (sym != null) {
                output[decoded++] = sym.toByte()
                accumulated.clear()
            }
        }
        return output
    }

    // =========================================================================
    // Private helpers
    // =========================================================================

    /**
     * Pack a bit string into bytes, filling each byte from bit 0 (LSB) upward.
     *
     * This is the same convention used by LZW and GIF: the first bit of the
     * stream occupies the least-significant bit of the first byte.
     *
     * The final partial byte is zero-padded in the high bits.
     *
     * Example — packing "000101011" (9 bits):
     * ```
     * Byte 0: bits[0..7] = 0b10101000 = 0xA8
     *   bit 0 ('0') → bit 0  (LSB)
     *   bit 3 ('1') → bit 3
     *   bit 5 ('1') → bit 5
     *   bit 7 ('1') → bit 7
     * Byte 1: bit[8] ('1') → 0b00000001 = 0x01
     * ```
     *
     * @param bits a string of '0' and '1' characters
     * @return the packed bytes
     */
    private fun packBitsLsbFirst(bits: String): ByteArray {
        val byteCount = (bits.length + 7) / 8
        val output = ByteArray(byteCount)
        for (i in bits.indices) {
            if (bits[i] == '1') {
                val byteIdx = i / 8
                val bitPos  = i % 8
                output[byteIdx] = (output[byteIdx].toInt() or (1 shl bitPos)).toByte()
            }
        }
        return output
    }
}
