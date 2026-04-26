// ============================================================================
// LZW.kt — CMP03: LZW Lossless Compression Algorithm (1984)
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

package com.codingadventures.lzw

import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import java.nio.ByteOrder

// ============================================================================
// LZW
// ============================================================================

/**
 * CMP03: LZW lossless compression.
 *
 * ```kotlin
 * val original   = "hello hello hello".toByteArray()
 * val compressed = LZW.compress(original)
 * val recovered  = LZW.decompress(compressed)
 * check(original.contentEquals(recovered))
 * ```
 */
object LZW {

    // =========================================================================
    // Constants
    // =========================================================================

    /** Reset code — instructs the decoder to clear its dictionary and restart. */
    const val CLEAR_CODE = 256

    /** End-of-stream code — the decoder stops reading after this code. */
    const val STOP_CODE = 257

    /** First dynamically assigned dictionary code. */
    const val INITIAL_NEXT_CODE = 258

    /** Starting bit-width for codes (covers 0–511, more than enough for 258). */
    const val INITIAL_CODE_SIZE = 9

    /** Maximum bit-width; dictionary caps at 2^16 = 65536 entries. */
    const val MAX_CODE_SIZE = 16

    /**
     * Maximum decompressed output size (64 MiB by default).
     *
     * A fully-saturated LZW dictionary (65536 entries, each one byte longer
     * than the last) can theoretically require ~2 GB of heap.  This limit caps
     * the output to prevent heap exhaustion from crafted streams.  Call
     * [decodeCodes] directly with a higher limit when needed.
     */
    const val DEFAULT_MAX_OUTPUT = 64 * 1024 * 1024 // 64 MiB

    /** Wire-format header size: original_length(4). */
    private const val HEADER_SIZE = 4

    // =========================================================================
    // Public API
    // =========================================================================

    /**
     * Compress [data] using LZW and return CMP03 wire-format bytes.
     *
     * @param data the bytes to compress (null treated as empty)
     * @return compressed bytes in CMP03 wire format
     */
    fun compress(data: ByteArray?): ByteArray {
        val bytes  = data ?: ByteArray(0)
        val codes  = encodeCodes(bytes)
        return packCodes(codes, bytes.size)
    }

    /**
     * Decompress CMP03 wire-format bytes back to the original data.
     *
     * Output is capped at [DEFAULT_MAX_OUTPUT] bytes to prevent heap exhaustion
     * from crafted streams.
     *
     * @param data compressed bytes (null treated as empty)
     * @return the original, uncompressed bytes
     */
    fun decompress(data: ByteArray?): ByteArray {
        if (data == null || data.size < HEADER_SIZE) return ByteArray(0)
        val originalLength = ByteBuffer.wrap(data, 0, HEADER_SIZE)
            .order(ByteOrder.BIG_ENDIAN).getInt()
        val codes  = unpackCodes(data)
        val result = decodeCodes(codes, DEFAULT_MAX_OUTPUT)
        // Trim to original length to remove bit-padding artefacts.
        return if (originalLength in 0..result.size) result.copyOf(originalLength) else result
    }

    // =========================================================================
    // Encoder
    // =========================================================================

    /**
     * Encode [data] into a list of LZW codes (including CLEAR and STOP).
     *
     * Algorithm:
     * 1. Initialise the encode dictionary: byte → code for all 256 bytes.
     * 2. Emit CLEAR_CODE to mark the start of the stream.
     * 3. Walk the input byte-by-byte, extending the current prefix *w*:
     *    - If *w + b* is already in the dictionary, extend *w*.
     *    - Otherwise, emit code_for(*w*), add *w + b* as a new entry,
     *      reset *w* to just *b*.
     *    - When the dictionary is full, emit CLEAR_CODE and re-initialise.
     * 4. Flush the remaining prefix and emit STOP_CODE.
     */
    internal fun encodeCodes(data: ByteArray): List<Int> {
        val codes      = mutableListOf<Int>()
        val maxEntries = 1 shl MAX_CODE_SIZE // 65536

        // Encode dictionary: ByteList → code.
        // Using List<Byte> as key (value-equal, unlike ByteArray).
        var encodeDict = HashMap<List<Byte>, Int>(512)
        for (b in 0 until 256) {
            encodeDict[listOf(b.toByte())] = b
        }
        var nextCode = INITIAL_NEXT_CODE

        codes.add(CLEAR_CODE)

        var w = emptyList<Byte>() // current working prefix

        for (rawByte in data) {
            val wb = w + rawByte
            if (encodeDict.containsKey(wb)) {
                w = wb // extend the prefix
            } else {
                // Emit code for the current prefix.
                codes.add(encodeDict[w]!!)

                if (nextCode < maxEntries) {
                    encodeDict[wb] = nextCode
                    nextCode++
                } else {
                    // Dictionary full — emit CLEAR and reset.
                    codes.add(CLEAR_CODE)
                    encodeDict = HashMap(512)
                    for (b in 0 until 256) {
                        encodeDict[listOf(b.toByte())] = b
                    }
                    nextCode = INITIAL_NEXT_CODE
                }

                w = listOf(rawByte) // restart with the unmatched byte
            }
        }

        // Flush remaining prefix.
        if (w.isNotEmpty()) {
            codes.add(encodeDict[w]!!)
        }

        codes.add(STOP_CODE)
        return codes
    }

    // =========================================================================
    // Decoder
    // =========================================================================

    /**
     * Decode a list of LZW codes back to the original bytes.
     *
     * Uses [DEFAULT_MAX_OUTPUT] as the output size limit.
     *
     * @param codes the code list from [encodeCodes]
     * @return the decoded bytes
     * @throws IllegalArgumentException if the code stream is corrupt or too large
     */
    internal fun decodeCodes(codes: List<Int>): ByteArray =
        decodeCodes(codes, DEFAULT_MAX_OUTPUT)

    /**
     * Decode a list of LZW codes back to the original bytes, with an output cap.
     *
     * Handles:
     * - [CLEAR_CODE]: reset dictionary and code_size.
     * - [STOP_CODE]: stop decoding.
     * - Tricky token (code == next_code): construct the entry as
     *   `dict[prev_code] + byteArrayOf(dict[prev_code][0])`.
     *
     * Security: without a size cap, a crafted stream that never emits
     * CLEAR_CODE can force the decoder to build up to 65278 dictionary entries,
     * each growing by 1 byte.  Total worst-case allocation ≈ 2 GB.  The
     * [maxOutput] limit stops decoding before heap exhaustion occurs.
     *
     * @param codes     the code list from [encodeCodes]
     * @param maxOutput maximum bytes to emit; throw if exceeded
     * @return the decoded bytes
     * @throws IllegalArgumentException if the code stream is corrupt or output exceeds maxOutput
     */
    internal fun decodeCodes(codes: List<Int>, maxOutput: Int): ByteArray {
        // Decode dictionary: code → byte sequence.
        val decodeDict = ArrayList<ByteArray>(512)
        for (b in 0 until 256) decodeDict.add(byteArrayOf(b.toByte()))
        decodeDict.add(ByteArray(0)) // 256 = CLEAR_CODE placeholder
        decodeDict.add(ByteArray(0)) // 257 = STOP_CODE  placeholder
        var nextCode = INITIAL_NEXT_CODE

        val out      = ByteArrayOutputStream()
        var prevCode: Int? = null

        for (code in codes) {
            if (code == CLEAR_CODE) {
                // Reset dictionary to 256 single-byte entries.
                decodeDict.clear()
                for (b in 0 until 256) decodeDict.add(byteArrayOf(b.toByte()))
                decodeDict.add(ByteArray(0)) // 256
                decodeDict.add(ByteArray(0)) // 257
                nextCode = INITIAL_NEXT_CODE
                prevCode = null
                continue
            }

            if (code == STOP_CODE) break

            val entry: ByteArray = when {
                code < decodeDict.size -> decodeDict[code]
                code == nextCode -> {
                    // Tricky token: code not yet in dict.
                    // Only valid when prevCode exists.
                    val pc = prevCode ?: throw IllegalArgumentException(
                        "Invalid LZW stream: tricky token with no previous code")
                    val prevEntry = decodeDict[pc]
                    prevEntry + byteArrayOf(prevEntry[0])
                }
                else -> throw IllegalArgumentException(
                    "Invalid LZW code: exceeds expected next code")
            }

            // Guard against heap exhaustion from crafted streams.
            require(out.size() + entry.size <= maxOutput) {
                "LZW: decompressed output exceeds limit of $maxOutput bytes"
            }

            out.write(entry)

            // Add new entry to the decode dictionary.
            if (prevCode != null && nextCode < (1 shl MAX_CODE_SIZE)) {
                val prevEntry = decodeDict[prevCode!!]
                decodeDict.add(prevEntry + byteArrayOf(entry[0]))
                nextCode++
            }

            prevCode = code
        }

        return out.toByteArray()
    }

    // =========================================================================
    // Serialisation
    // =========================================================================

    /**
     * Pack a list of LZW codes into the CMP03 wire format.
     *
     * Wire format:
     * - Bytes 0–3: original_length (big-endian uint32)
     * - Bytes 4+: codes as variable-width LSB-first bit-packed integers
     *
     * The code size starts at [INITIAL_CODE_SIZE] (9) and grows each time
     * next_code crosses the next power-of-2 boundary.  CLEAR_CODE resets the
     * code size back to [INITIAL_CODE_SIZE].
     */
    internal fun packCodes(codes: List<Int>, originalLength: Int): ByteArray {
        val writer   = BitWriter()
        var codeSize = INITIAL_CODE_SIZE
        var nextCode = INITIAL_NEXT_CODE

        for (code in codes) {
            writer.write(code, codeSize)

            when {
                code == CLEAR_CODE -> {
                    codeSize = INITIAL_CODE_SIZE
                    nextCode = INITIAL_NEXT_CODE
                }
                code != STOP_CODE -> {
                    if (nextCode < (1 shl MAX_CODE_SIZE)) {
                        nextCode++
                        if (nextCode > (1 shl codeSize) && codeSize < MAX_CODE_SIZE) {
                            codeSize++
                        }
                    }
                }
            }
        }
        writer.flush()

        val header = ByteBuffer.allocate(HEADER_SIZE).order(ByteOrder.BIG_ENDIAN)
        header.putInt(originalLength)

        val out = ByteArrayOutputStream()
        out.write(header.array())
        out.write(writer.toByteArray())
        return out.toByteArray()
    }

    /**
     * Unpack CMP03 wire-format bytes into a list of LZW codes.
     *
     * The decoder stops on STOP_CODE or stream exhaustion, so a crafted stream
     * cannot cause unbounded iteration.
     */
    internal fun unpackCodes(data: ByteArray): List<Int> {
        if (data.size < HEADER_SIZE) return listOf(CLEAR_CODE, STOP_CODE)

        val reader   = BitReader(data, HEADER_SIZE)
        val codes    = mutableListOf<Int>()
        var codeSize = INITIAL_CODE_SIZE
        var nextCode = INITIAL_NEXT_CODE

        while (!reader.exhausted()) {
            if (!reader.hasEnough(codeSize)) break
            val code = reader.read(codeSize)
            codes.add(code)

            when {
                code == STOP_CODE  -> break
                code == CLEAR_CODE -> {
                    codeSize = INITIAL_CODE_SIZE
                    nextCode = INITIAL_NEXT_CODE
                }
                else -> {
                    if (nextCode < (1 shl MAX_CODE_SIZE)) {
                        nextCode++
                        if (nextCode > (1 shl codeSize) && codeSize < MAX_CODE_SIZE) {
                            codeSize++
                        }
                    }
                }
            }
        }

        return codes
    }

    // =========================================================================
    // Bit I/O
    // =========================================================================

    /**
     * Accumulates variable-width codes into a byte buffer, LSB-first.
     *
     * LSB-first packing: the first code written occupies bits 0..N-1 of the
     * first byte, spilling into subsequent bytes as necessary.  This matches the
     * GIF and Unix compress conventions.
     */
    class BitWriter {
        private var buffer = 0L
        private var bitPos = 0
        private val out    = ByteArrayOutputStream()

        /**
         * Write [code] using exactly [codeSize] bits.
         */
        fun write(code: Int, codeSize: Int) {
            buffer = buffer or (code.toLong() shl bitPos)
            bitPos += codeSize
            while (bitPos >= 8) {
                out.write((buffer and 0xFF).toInt())
                buffer = buffer ushr 8
                bitPos -= 8
            }
        }

        /** Flush any remaining bits as a final partial byte. */
        fun flush() {
            if (bitPos > 0) {
                out.write((buffer and 0xFF).toInt())
                buffer = 0L
                bitPos = 0
            }
        }

        /** Return the accumulated output. */
        fun toByteArray(): ByteArray = out.toByteArray()
    }

    /**
     * Reads variable-width codes from a byte buffer, LSB-first.
     *
     * Mirrors [BitWriter] exactly: bits within each byte are consumed from the
     * least-significant end first.
     */
    class BitReader(private val data: ByteArray, startPos: Int) {
        private var pos    = startPos
        private var buffer = 0L
        private var bitPos = 0

        /**
         * Read and return the next [codeSize]-bit code.
         */
        fun read(codeSize: Int): Int {
            while (bitPos < codeSize) {
                buffer = buffer or ((data[pos++].toLong() and 0xFFL) shl bitPos)
                bitPos += 8
            }
            val code = (buffer and ((1L shl codeSize) - 1L)).toInt()
            buffer = buffer ushr codeSize
            bitPos -= codeSize
            return code
        }

        /** True when the byte stream is exhausted and no buffered bits remain. */
        fun exhausted(): Boolean = pos >= data.size && bitPos == 0

        /** True when at least [n] bits can still be read. */
        fun hasEnough(n: Int): Boolean {
            val available = bitPos + (data.size - pos) * 8
            return available >= n
        }
    }
}
