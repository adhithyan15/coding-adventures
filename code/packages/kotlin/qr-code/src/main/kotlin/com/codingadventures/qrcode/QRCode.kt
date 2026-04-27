/**
 * # qr-code — ISO/IEC 18004:2015 compliant QR Code encoder
 *
 * QR Code (Quick Response code) was invented by Masahiro Hara at Denso Wave in
 * 1994 to track automotive parts on assembly lines.  It was designed to be read
 * 10× faster than a 1D barcode, and to survive physical damage to up to 30% of
 * the symbol's area.  It is now the most widely deployed 2D barcode format on
 * earth — on every product label, restaurant menu, bus stop, and business card.
 *
 * ## Encoding pipeline
 *
 * ```
 * input string
 *   → mode selection    (numeric / alphanumeric / byte)
 *   → version selection (smallest v1–40 that fits at the ECC level)
 *   → bit stream        (mode indicator + char count + data + padding)
 *   → blocks + RS ECC   (GF(256) b=0 convention, poly 0x11D)
 *   → interleave        (data CWs round-robin, then ECC CWs)
 *   → grid init         (finder × 3, separators, timing, alignment, format, dark)
 *   → zigzag placement  (two-column snake from bottom-right)
 *   → mask evaluation   (8 patterns, 4-rule penalty, pick lowest)
 *   → finalize          (format info + version info v7+)
 *   → ModuleGrid
 * ```
 *
 * ## Understanding the symbol
 *
 * A QR Code symbol is a square grid of **modules** — dark (true) or light
 * (false) cells.  The grid size is `(4V + 17) × (4V + 17)` where V is the
 * version from 1 to 40.
 *
 * The symbol is divided into:
 * - **Finder patterns** — three 7×7 squares in the three corners (not bottom-right).
 *   Their distinctive 1:1:3:1:1 ratio lets scanners find and orient the symbol.
 * - **Separators** — 1-module quiet strips around each finder pattern.
 * - **Timing patterns** — alternating dark/light strips along row 6 and column 6.
 * - **Alignment patterns** — small 5×5 squares (version 2+) for distortion correction.
 * - **Format information** — 15-bit BCH-protected word encoding ECC level + mask.
 * - **Version information** — 18-bit BCH-protected word (versions 7+).
 * - **Dark module** — always-dark module at (4V+9, 8).
 * - **Data + ECC modules** — the rest, filled by the encoder.
 *
 * Spec reference: code/specs/qr-code.md
 * Literate reference: code/packages/rust/qr-code/src/lib.rs
 */
package com.codingadventures.qrcode

import com.codingadventures.barcode2d.Barcode2DLayoutConfig
import com.codingadventures.barcode2d.ModuleGrid
import com.codingadventures.barcode2d.ModuleShape
import com.codingadventures.barcode2d.layout
import com.codingadventures.gf256.GF256
import com.codingadventures.paintinstructions.PaintScene

/** Package version. */
const val VERSION = "0.1.0"

// =============================================================================
// Public types — ECC level and errors
// =============================================================================

/**
 * Error correction level.  Higher levels allow more of the symbol to be
 * damaged while still decoding correctly, but consume more of the symbol's
 * module capacity for redundancy.
 *
 * | Level | Recovery capacity |
 * |-------|------------------|
 * | L     | ~7% of codewords |
 * | M     | ~15% (common default) |
 * | Q     | ~25% |
 * | H     | ~30% |
 */
enum class EccLevel {
    /** ~7% recovery — maximum data capacity. */
    L,
    /** ~15% recovery — good balance of data and error tolerance. */
    M,
    /** ~25% recovery — suitable for printing on rough surfaces. */
    Q,
    /** ~30% recovery — maximum damage tolerance. */
    H,
}

/**
 * Errors produced by the QR Code encoder.
 *
 * These are sealed so callers can exhaustively pattern-match:
 * ```kotlin
 * when (err) {
 *     is QRCodeError.InputTooLong -> println("Too long: ${err.message}")
 *     is QRCodeError.LayoutError  -> println("Layout: ${err.message}")
 * }
 * ```
 */
sealed class QRCodeError(message: String) : Exception(message) {
    /** Input is too long to fit in any QR Code version at the chosen ECC level. */
    class InputTooLong(message: String) : QRCodeError(message)

    /** Layout or rendering configuration error. */
    class LayoutError(message: String) : QRCodeError(message)
}

// =============================================================================
// ECC level helpers
// =============================================================================

/**
 * The 2-bit ECC level indicator embedded in the format information word.
 *
 * ISO 18004 assigns these deliberately non-sequential values to maximise
 * Hamming distance between valid format info words:
 *
 *   L → 01   M → 00   Q → 11   H → 10
 */
private fun eccIndicator(ecc: EccLevel): Int = when (ecc) {
    EccLevel.L -> 0b01
    EccLevel.M -> 0b00
    EccLevel.Q -> 0b11
    EccLevel.H -> 0b10
}

/** Array index for looking up capacity/block tables. */
private fun eccIdx(ecc: EccLevel): Int = when (ecc) {
    EccLevel.L -> 0
    EccLevel.M -> 1
    EccLevel.Q -> 2
    EccLevel.H -> 3
}

// =============================================================================
// ISO 18004:2015 — Capacity tables (Table 9)
// =============================================================================
//
// These tables encode the block structure for every version (1–40) and ECC
// level (L, M, Q, H).  They are sourced directly from ISO/IEC 18004:2015
// Annex I and match the values used by every other QR encoder in this repo.
//
// ECC_CODEWORDS_PER_BLOCK[ecc_idx][version]:
//   how many ECC codewords each block has.
//
// NUM_BLOCKS[ecc_idx][version]:
//   total number of blocks (both groups combined).
//
// Index 0 is a dummy padding value (-1) since versions are 1-indexed.

/** ECC codewords per block, indexed [eccIdx][version]. Index 0 is unused (-1). */
private val ECC_CODEWORDS_PER_BLOCK: Array<IntArray> = arrayOf(
    // L:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    intArrayOf(-1, 7, 10, 15, 20, 26, 18, 20, 24, 30, 18, 20, 24, 26, 30, 22, 24, 28, 30, 28, 28, 28, 28, 30, 30, 26, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30),
    // M:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    intArrayOf(-1, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26, 30, 22, 22, 24, 24, 28, 28, 26, 26, 26, 26, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28),
    // Q:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    intArrayOf(-1, 13, 22, 18, 26, 18, 24, 18, 22, 20, 24, 28, 26, 24, 20, 30, 24, 28, 28, 26, 30, 28, 30, 30, 30, 30, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30),
    // H:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    intArrayOf(-1, 17, 28, 22, 16, 22, 28, 26, 26, 24, 28, 24, 28, 22, 24, 24, 30, 28, 28, 26, 28, 30, 24, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30),
)

/** Number of blocks (all groups), indexed [eccIdx][version]. Index 0 is unused (-1). */
private val NUM_BLOCKS: Array<IntArray> = arrayOf(
    // L:
    intArrayOf(-1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 4, 4, 4, 4, 4, 6, 6, 6, 6, 7, 8, 8, 9, 9, 10, 12, 12, 12, 13, 14, 15, 16, 17, 18, 19, 19, 20, 21, 22, 24, 25),
    // M:
    intArrayOf(-1, 1, 1, 1, 2, 2, 4, 4, 4, 5, 5, 5, 8, 9, 9, 10, 10, 11, 13, 14, 16, 17, 17, 18, 20, 21, 23, 25, 26, 28, 29, 31, 33, 35, 37, 38, 40, 43, 45, 47, 49),
    // Q:
    intArrayOf(-1, 1, 1, 2, 2, 4, 4, 6, 6, 8, 8, 8, 10, 12, 16, 12, 17, 16, 18, 21, 20, 23, 23, 25, 27, 29, 34, 34, 35, 38, 40, 43, 45, 48, 51, 53, 56, 59, 62, 65, 68),
    // H:
    intArrayOf(-1, 1, 1, 2, 4, 4, 4, 5, 6, 8, 8, 11, 11, 16, 16, 18, 16, 19, 21, 25, 25, 25, 34, 30, 32, 35, 37, 40, 42, 45, 48, 51, 54, 57, 60, 63, 66, 70, 74, 77, 80),
)

/**
 * Alignment pattern center coordinates, indexed by version - 1.
 *
 * Alignment patterns (version 2+) are small 5×5 finder-like squares placed
 * at predetermined positions in the data area.  They help scanners correct
 * for perspective distortion.
 *
 * The center coordinates form a grid of all pairwise combinations of the
 * listed values, EXCLUDING positions that overlap with a finder pattern.
 *
 * Source: ISO/IEC 18004:2015, Annex E.
 */
private val ALIGNMENT_POSITIONS: Array<IntArray> = arrayOf(
    intArrayOf(),                                       // v1  — no alignment patterns
    intArrayOf(6, 18),                                  // v2
    intArrayOf(6, 22),                                  // v3
    intArrayOf(6, 26),                                  // v4
    intArrayOf(6, 30),                                  // v5
    intArrayOf(6, 34),                                  // v6
    intArrayOf(6, 22, 38),                              // v7
    intArrayOf(6, 24, 42),                              // v8
    intArrayOf(6, 26, 46),                              // v9
    intArrayOf(6, 28, 50),                              // v10
    intArrayOf(6, 30, 54),                              // v11
    intArrayOf(6, 32, 58),                              // v12
    intArrayOf(6, 34, 62),                              // v13
    intArrayOf(6, 26, 46, 66),                          // v14
    intArrayOf(6, 26, 48, 70),                          // v15
    intArrayOf(6, 26, 50, 74),                          // v16
    intArrayOf(6, 30, 54, 78),                          // v17
    intArrayOf(6, 30, 56, 82),                          // v18
    intArrayOf(6, 30, 58, 86),                          // v19
    intArrayOf(6, 34, 62, 90),                          // v20
    intArrayOf(6, 28, 50, 72, 94),                      // v21
    intArrayOf(6, 26, 50, 74, 98),                      // v22
    intArrayOf(6, 30, 54, 78, 102),                     // v23
    intArrayOf(6, 28, 54, 80, 106),                     // v24
    intArrayOf(6, 32, 58, 84, 110),                     // v25
    intArrayOf(6, 30, 58, 86, 114),                     // v26
    intArrayOf(6, 34, 62, 90, 118),                     // v27
    intArrayOf(6, 26, 50, 74, 98, 122),                 // v28
    intArrayOf(6, 30, 54, 78, 102, 126),                // v29
    intArrayOf(6, 26, 52, 78, 104, 130),                // v30
    intArrayOf(6, 30, 56, 82, 108, 134),                // v31
    intArrayOf(6, 34, 60, 86, 112, 138),                // v32
    intArrayOf(6, 30, 58, 86, 114, 142),                // v33
    intArrayOf(6, 34, 62, 90, 118, 146),                // v34
    intArrayOf(6, 30, 54, 78, 102, 126, 150),           // v35
    intArrayOf(6, 24, 50, 76, 102, 128, 154),           // v36
    intArrayOf(6, 28, 54, 80, 106, 132, 158),           // v37
    intArrayOf(6, 32, 58, 84, 110, 136, 162),           // v38
    intArrayOf(6, 26, 54, 82, 110, 138, 166),           // v39
    intArrayOf(6, 30, 58, 86, 114, 142, 170),           // v40
)

// =============================================================================
// Grid geometry helpers
// =============================================================================

/**
 * The side length of a QR Code symbol of the given version.
 *
 * Formula: 4V + 17  (V is version 1–40)
 *
 * Examples:
 *   version 1 → 21×21   (minimum)
 *   version 7 → 45×45
 *   version 40 → 177×177 (maximum)
 */
private fun symbolSize(version: Int): Int = 4 * version + 17

/**
 * Total number of raw data + ECC bits (before dividing into codewords).
 *
 * This formula, from Nayuki's reference implementation (public domain), accounts
 * for all the structural modules subtracted from the total grid:
 *   - 3 finder patterns (each 7×7 = 49 modules)
 *   - 3 separators (each 8 + 7 = 15 modules)
 *   - 2 timing strips
 *   - alignment patterns (variable by version)
 *   - format information (2 × 15 = 30 modules)
 *   - version information (2 × 18 = 36 modules, version ≥ 7)
 *   - dark module (1 module)
 */
private fun numRawDataModules(version: Int): Int {
    val v = version.toLong()
    var result = (16 * v + 128) * v + 64
    if (version >= 2) {
        val numAlign = v / 7 + 2
        result -= (25 * numAlign - 10) * numAlign - 55
        if (version >= 7) {
            result -= 36
        }
    }
    return result.toInt()
}

/**
 * Total data codewords (bytes) available for this version and ECC level.
 *
 * = total raw codewords − ECC codewords consumed by all blocks
 */
private fun numDataCodewords(version: Int, ecc: EccLevel): Int {
    val e = eccIdx(ecc)
    val rawCw = numRawDataModules(version) / 8
    val eccCw = NUM_BLOCKS[e][version] * ECC_CODEWORDS_PER_BLOCK[e][version]
    return rawCw - eccCw
}

/**
 * Remainder bits appended after all codewords during zigzag placement.
 *
 * Some versions don't have an exact multiple of 8 modules for data+ECC.
 * The remainder bits (always 0) fill the gap.
 *
 * Example: version 2 has 0 remainder bits; version 14 has 3.
 */
private fun numRemainderBits(version: Int): Int = numRawDataModules(version) % 8

// =============================================================================
// Reed-Solomon encoder (b=0 convention for QR)
// =============================================================================
//
// QR Code uses a specific RS convention:
//
//   generator polynomial g(x) = ∏(x + α^i) for i = 0, 1, …, n-1
//
// This is the "b=0 convention" — the first root is α^0 = 1 rather than α^1.
// This differs from the MA02 reed-solomon package (which uses b=1).
//
// The GF(256) field uses the same primitive polynomial as gf256 (MA01):
//   p(x) = x^8 + x^4 + x^3 + x^2 + 1 = 0x11D
//
// We build the generator polynomial fresh here rather than calling MA02,
// because the b=0 vs b=1 difference matters for correctness.

/**
 * Build the monic RS generator polynomial of degree [n] with roots α^0, α^1, …, α^{n-1}.
 *
 * g(x) = ∏(x + α^i) for i in 0 until n
 *
 * The result has n+1 elements.  Index 0 is the leading coefficient (always 1
 * because the polynomial is monic).  Index n is the constant term.
 *
 * Example for n=2:
 *   g(x) = (x + α^0)(x + α^1)
 *         = (x + 1)(x + 2)   [since α=2 in GF(256) with poly 0x11D]
 *         = x^2 + (1 XOR 2)x + (1 * 2)
 *         = x^2 + 3x + 2
 *   output: [1, 3, 2]
 *
 * @param n  Number of ECC codewords (= degree of the generator polynomial).
 * @return   Coefficient list of length n+1, index 0 = leading term.
 */
private fun buildGenerator(n: Int): IntArray {
    // Start with the polynomial g(x) = 1 (the multiplicative identity).
    var g = intArrayOf(1)

    for (i in 0 until n) {
        // Multiply g(x) by the linear factor (x + α^i).
        // α^i = GF256.pow(2, i) because the primitive element α = 2.
        val ai = GF256.pow(2, i)
        val next = IntArray(g.size + 1)
        for (j in g.indices) {
            // Coefficient of x^(deg-j) term in next = g[j]*1 XOR g[j-1]*ai
            // (polynomial long multiplication in GF(256), where + = XOR)
            next[j] = next[j] xor g[j]
            next[j + 1] = next[j + 1] xor GF256.mul(g[j], ai)
        }
        g = next
    }
    return g
}

/**
 * Compute [n] ECC bytes for the given data bytes by LFSR polynomial division.
 *
 * Computes the remainder R(x) = D(x) · x^n mod G(x), which are the ECC codewords.
 *
 * The LFSR (linear feedback shift register) algorithm processes one data byte
 * at a time, maintaining a running remainder in a register of length n:
 *
 * ```
 * for each data byte b:
 *   feedback = b XOR rem[0]
 *   shift rem left by 1 (drop rem[0], append 0 at rem[n-1])
 *   for each position i:
 *     rem[i] XOR= generator[i+1] * feedback
 * ```
 *
 * @param data       Data codeword bytes.
 * @param generator  Monic generator polynomial from [buildGenerator], length n+1.
 * @return           ECC bytes of length n.
 */
private fun rsEncode(data: ByteArray, generator: IntArray): ByteArray {
    val n = generator.size - 1
    val rem = IntArray(n)

    for (b in data) {
        val fb = (b.toInt() and 0xFF) xor rem[0]
        // Shift the register: rem[0..n-2] = rem[1..n-1], rem[n-1] = 0
        for (i in 0 until n - 1) rem[i] = rem[i + 1]
        rem[n - 1] = 0
        if (fb != 0) {
            // XOR in feedback multiplied by each generator coefficient.
            // generator[0] is always 1 (monic), so generator[i+1] for i in 0..n-1.
            for (i in 0 until n) {
                rem[i] = rem[i] xor GF256.mul(generator[i + 1], fb)
            }
        }
    }

    return ByteArray(n) { rem[it].toByte() }
}

// =============================================================================
// Data encoding modes
// =============================================================================
//
// QR Code supports four modes; this implementation handles three:
//
//   1. Numeric      — only digits 0-9
//   2. Alphanumeric — digits, uppercase A-Z, space, $ % * + - . / :
//   3. Byte         — raw UTF-8 bytes (fallback for everything else)
//
// The encoder automatically picks the most compact mode for the input.
// Numeric encodes ~3.3 digits/11 bits; alphanumeric ~2.7 chars/11 bits;
// byte encodes 1 byte/8 bits.  For purely numeric inputs, numeric mode
// saves ~50% capacity vs byte mode.

/** The 45-character alphanumeric set, indexed 0-44. */
private const val ALPHANUM_CHARS = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ \$%*+-./:"

private enum class EncodingMode { NUMERIC, ALPHANUMERIC, BYTE }

/**
 * Select the most compact encoding mode for [input].
 *
 * The selection algorithm:
 * 1. If every character is an ASCII digit (0-9): use Numeric.
 * 2. If every character is in the 45-char alphanumeric set: use Alphanumeric.
 * 3. Otherwise: use Byte (encodes any UTF-8 string).
 *
 * This greedy whole-message selection is the simplest correct approach.
 * Mixed-mode segmentation (v0.2.0 enhancement) can improve capacity for
 * inputs like "12345HELLO" by encoding each segment in its optimal mode.
 */
private fun selectMode(input: String): EncodingMode {
    if (input.all { it.isDigit() }) return EncodingMode.NUMERIC
    if (input.all { ALPHANUM_CHARS.contains(it) }) return EncodingMode.ALPHANUMERIC
    return EncodingMode.BYTE
}

/**
 * The 4-bit mode indicator placed at the start of the bit stream.
 *
 * | Mode          | Bits |
 * |---------------|------|
 * | Numeric       | 0001 |
 * | Alphanumeric  | 0010 |
 * | Byte          | 0100 |
 */
private fun modeIndicator(mode: EncodingMode): Int = when (mode) {
    EncodingMode.NUMERIC      -> 0b0001
    EncodingMode.ALPHANUMERIC -> 0b0010
    EncodingMode.BYTE         -> 0b0100
}

/**
 * Width in bits of the character count field, which depends on mode and version.
 *
 * The count field grows for higher versions to accommodate larger capacity.
 * For example, Byte mode allows up to 255 bytes in versions 1-9 (8-bit count)
 * but up to 65535 bytes in versions 10-40 (16-bit count).
 *
 * | Mode          | Versions 1-9 | Versions 10-26 | Versions 27-40 |
 * |---------------|--------------|----------------|----------------|
 * | Numeric       | 10           | 12             | 14             |
 * | Alphanumeric  | 9            | 11             | 13             |
 * | Byte          | 8            | 16             | 16             |
 */
private fun charCountBits(mode: EncodingMode, version: Int): Int = when (mode) {
    EncodingMode.NUMERIC      -> if (version <= 9) 10 else if (version <= 26) 12 else 14
    EncodingMode.ALPHANUMERIC -> if (version <= 9) 9  else if (version <= 26) 11 else 13
    EncodingMode.BYTE         -> if (version <= 9) 8  else 16
}

// =============================================================================
// Bit stream builder
// =============================================================================

/**
 * Accumulates individual bits and flushes them to a byte array.
 *
 * Bits are appended MSB-first: `write(0b1011, 4)` appends the bits 1, 0, 1, 1.
 * The internal representation stores one bit per element for clarity; [toBytes]
 * packs them into bytes, zero-padding the last byte if needed.
 *
 * This bit-by-bit approach mirrors the TypeScript and Rust reference
 * implementations and keeps the encoding logic easy to read and test.
 */
private class BitWriter {
    private val bits = mutableListOf<Int>() // each element is 0 or 1

    /** Number of bits accumulated so far. */
    fun bitLen(): Int = bits.size

    /**
     * Append [count] bits from [value], MSB first.
     *
     * For example, `write(0b101, 3)` appends bits 1, 0, 1.
     * [count] must be between 1 and 32.
     */
    fun write(value: Int, count: Int) {
        for (i in count - 1 downTo 0) {
            bits.add((value shr i) and 1)
        }
    }

    /**
     * Pack the accumulated bits into a byte array, MSB-first.
     *
     * If the total bit count is not a multiple of 8, the last byte is
     * zero-padded on the right (least significant bits are 0).
     */
    fun toBytes(): ByteArray {
        val nBytes = (bits.size + 7) / 8
        val result = ByteArray(nBytes)
        for (i in bits.indices) {
            if (bits[i] == 1) {
                result[i / 8] = (result[i / 8].toInt() or (1 shl (7 - (i % 8)))).toByte()
            }
        }
        return result
    }
}

/**
 * Encode the digit string [input] in Numeric mode.
 *
 * Groups of 3 digits → 10 bits (encoding values 000–999)
 * Groups of 2 digits → 7 bits  (encoding values 00–99)
 * Single digit       → 4 bits  (encoding values 0–9)
 *
 * This packing is why numeric mode is so efficient: three 10-bit characters
 * occupy only 10 bits vs 24 bits in byte mode.
 */
private fun encodeNumeric(input: String, w: BitWriter) {
    val digits = input.map { it - '0' }
    var i = 0
    while (i + 2 < digits.size) {
        w.write(digits[i] * 100 + digits[i + 1] * 10 + digits[i + 2], 10)
        i += 3
    }
    if (i + 1 < digits.size) {
        w.write(digits[i] * 10 + digits[i + 1], 7)
        i += 2
    }
    if (i < digits.size) {
        w.write(digits[i], 4)
    }
}

/**
 * Encode [input] in Alphanumeric mode.
 *
 * Pairs of characters are encoded as: first_idx * 45 + second_idx (11 bits).
 * A single trailing character is encoded as its index (6 bits).
 *
 * The 45-character set is indexed as: 0-9 → 0-9, A-Z → 10-35, SP → 36,
 * $ → 37, % → 38, * → 39, + → 40, - → 41, . → 42, / → 43, : → 44.
 *
 * Precondition: all characters in [input] are in [ALPHANUM_CHARS].
 */
private fun encodeAlphanumeric(input: String, w: BitWriter) {
    val indices = input.map { c ->
        val idx = ALPHANUM_CHARS.indexOf(c)
        check(idx >= 0) { "encodeAlphanumeric: '$c' not in alphanumeric set (precondition violated)" }
        idx
    }
    var i = 0
    while (i + 1 < indices.size) {
        w.write(indices[i] * 45 + indices[i + 1], 11)
        i += 2
    }
    if (i < indices.size) {
        w.write(indices[i], 6)
    }
}

/**
 * Encode [input] in Byte mode.
 *
 * Each UTF-8 byte is encoded as 8 bits, MSB first.  Any UTF-8 string can be
 * encoded this way.  Modern QR scanners default to UTF-8, so no ECI header is
 * needed in practice (ECI mode is a v0.2.0 enhancement).
 */
private fun encodeByteMode(input: String, w: BitWriter) {
    for (b in input.toByteArray(Charsets.UTF_8)) {
        w.write(b.toInt() and 0xFF, 8)
    }
}

/**
 * Build the data codeword sequence for the given [input], [version], and [ecc] level.
 *
 * The bit stream structure (from ISO 18004 Section 7.4):
 * ```
 * [4-bit mode indicator]
 * [character count, mode- and version-dependent width]
 * [encoded data bits]
 * [4-bit terminator 0000 (or fewer bits if capacity is full)]
 * [0-7 zero bits to reach a byte boundary]
 * [alternating 0xEC 0x11 pad bytes to fill remaining data codewords]
 * ```
 *
 * The padding bytes 0xEC (11101100) and 0x11 (00010001) are chosen by ISO so
 * that they alternate while keeping the bit stream balanced.
 */
private fun buildDataCodewords(input: String, version: Int, ecc: EccLevel): ByteArray {
    val mode = selectMode(input)
    val capacity = numDataCodewords(version, ecc)
    val w = BitWriter()

    // Mode indicator (4 bits).
    w.write(modeIndicator(mode), 4)

    // Character count.
    val charCount = if (mode == EncodingMode.BYTE) {
        input.toByteArray(Charsets.UTF_8).size  // byte length, not char count
    } else {
        input.length
    }
    w.write(charCount, charCountBits(mode, version))

    // Data bits.
    when (mode) {
        EncodingMode.NUMERIC      -> encodeNumeric(input, w)
        EncodingMode.ALPHANUMERIC -> encodeAlphanumeric(input, w)
        EncodingMode.BYTE         -> encodeByteMode(input, w)
    }

    // Terminator: up to 4 zero bits.  May be truncated if near capacity.
    val available = capacity * 8
    val termLen = minOf(available - w.bitLen(), 4)
    if (termLen > 0) w.write(0, termLen)

    // Byte-boundary padding: zero bits to reach the next byte boundary.
    val rem = w.bitLen() % 8
    if (rem != 0) w.write(0, 8 - rem)

    // Pad bytes: alternate 0xEC and 0x11 until we fill all data codewords.
    val bytes = w.toBytes().toMutableList()
    var pad = 0xEC
    while (bytes.size < capacity) {
        bytes.add(pad.toByte())
        pad = if (pad == 0xEC) 0x11 else 0xEC
    }

    return bytes.toByteArray()
}

// =============================================================================
// Block processing and interleaving
// =============================================================================

/**
 * One RS block: data codewords + computed ECC codewords.
 *
 * Multiple blocks are used for most versions to improve burst-error resilience:
 * a physical scratch that destroys a contiguous area only destroys one or two
 * blocks, leaving the others intact.
 */
private data class Block(val data: ByteArray, val ecc: ByteArray)

/**
 * Split [data] into RS blocks, compute ECC for each, and return all blocks.
 *
 * The block structure follows the ISO table:
 *   Group 1: (total_blocks - num_long) blocks of short_len data codewords.
 *   Group 2: num_long blocks of (short_len + 1) data codewords.
 *   Each block: same number of ECC codewords.
 *
 * Where short_len = total_data / total_blocks and num_long = total_data % total_blocks.
 */
private fun computeBlocks(data: ByteArray, version: Int, ecc: EccLevel): List<Block> {
    val e = eccIdx(ecc)
    val totalBlocks = NUM_BLOCKS[e][version]
    val eccLen = ECC_CODEWORDS_PER_BLOCK[e][version]
    val totalData = numDataCodewords(version, ecc)
    val shortLen = totalData / totalBlocks
    val numLong = totalData % totalBlocks   // number of "long" blocks (shortLen+1 data cw each)
    val gen = buildGenerator(eccLen)

    val blocks = mutableListOf<Block>()
    var offset = 0
    val g1Count = totalBlocks - numLong

    // Group 1 blocks: each has shortLen data codewords.
    repeat(g1Count) {
        val d = data.copyOfRange(offset, offset + shortLen)
        val eccCw = rsEncode(d, gen)
        blocks.add(Block(d, eccCw))
        offset += shortLen
    }

    // Group 2 blocks: each has shortLen+1 data codewords.
    repeat(numLong) {
        val d = data.copyOfRange(offset, offset + shortLen + 1)
        val eccCw = rsEncode(d, gen)
        blocks.add(Block(d, eccCw))
        offset += shortLen + 1
    }

    return blocks
}

/**
 * Interleave data codewords then ECC codewords from all blocks.
 *
 * Interleaving maximises burst-error resilience: adjacent modules in the
 * final grid come from different blocks, so physical damage to a contiguous
 * area destroys only a few codewords from each block rather than an entire
 * block's worth.
 *
 * Algorithm:
 * 1. Round-robin through all blocks, taking the i-th data codeword from each.
 * 2. Round-robin through all blocks, taking the i-th ECC codeword from each.
 *
 * Example (3 blocks, 4 data cw each):
 *   Interleaved = [d0[0], d1[0], d2[0], d0[1], d1[1], d2[1], d0[2], …]
 */
private fun interleaveBlocks(blocks: List<Block>): ByteArray {
    val result = mutableListOf<Byte>()
    val maxData = blocks.maxOfOrNull { it.data.size } ?: 0
    val maxEcc = blocks.maxOfOrNull { it.ecc.size } ?: 0

    // Interleave data codewords.
    for (i in 0 until maxData) {
        for (b in blocks) {
            if (i < b.data.size) result.add(b.data[i])
        }
    }
    // Interleave ECC codewords.
    for (i in 0 until maxEcc) {
        for (b in blocks) {
            if (i < b.ecc.size) result.add(b.ecc[i])
        }
    }

    return result.toByteArray()
}

// =============================================================================
// WorkGrid — mutable grid for the encoder
// =============================================================================

/**
 * A mutable square grid used during encoding.
 *
 * Each module has two flags:
 *   - [modules]: whether the module is currently dark.
 *   - [reserved]: whether the module is structural (must not be overwritten by data).
 *
 * Structural modules (finders, separators, timing, alignment, format info,
 * version info, dark module) are placed first and marked reserved.  The data
 * placement step skips reserved modules.
 *
 * After data placement, masking modifies only non-reserved modules.
 */
private class WorkGrid(val size: Int) {
    val modules = Array(size) { BooleanArray(size) }
    val reserved = Array(size) { BooleanArray(size) }

    /** Set a module's darkness and optionally mark it reserved. */
    fun set(r: Int, c: Int, dark: Boolean, reserve: Boolean = false) {
        modules[r][c] = dark
        if (reserve) reserved[r][c] = true
    }

    /** Convert to an immutable [ModuleGrid] for output. */
    fun toModuleGrid(): ModuleGrid = ModuleGrid(
        rows = size,
        cols = size,
        modules = modules.map { row -> row.map { it }.toList() }.toList(),
        moduleShape = ModuleShape.SQUARE,
    )
}

// =============================================================================
// Structural pattern placement
// =============================================================================

/**
 * Place a 7×7 finder pattern with its top-left corner at ([top], [left]).
 *
 * A finder pattern looks like this (1=dark, 0=light):
 * ```
 * 1 1 1 1 1 1 1
 * 1 0 0 0 0 0 1
 * 1 0 1 1 1 0 1
 * 1 0 1 1 1 0 1
 * 1 0 1 1 1 0 1
 * 1 0 0 0 0 0 1
 * 1 1 1 1 1 1 1
 * ```
 *
 * The pattern has:
 * - A solid outer border (the perimeter ring).
 * - A hollow gap (the white ring).
 * - A 3×3 solid core at the center.
 *
 * A scanner finds three such patterns (at three corners, never all four) and
 * uses the 1:1:3:1:1 dark-to-light ratio to locate the symbol's corners.
 */
private fun placeFinder(g: WorkGrid, top: Int, left: Int) {
    for (dr in 0..6) {
        for (dc in 0..6) {
            val onBorder = dr == 0 || dr == 6 || dc == 0 || dc == 6
            val inCore = dr in 2..4 && dc in 2..4
            g.set(top + dr, left + dc, onBorder || inCore, reserve = true)
        }
    }
}

/**
 * Place a 5×5 alignment pattern centered at ([row], [col]).
 *
 * Alignment patterns look like a smaller finder pattern (ring + center):
 * ```
 * 1 1 1 1 1
 * 1 0 0 0 1
 * 1 0 1 0 1
 * 1 0 0 0 1
 * 1 1 1 1 1
 * ```
 *
 * They are placed at predetermined positions in version 2+ symbols.
 * A scanner uses them to correct perspective distortion and estimate
 * the module grid spacing in the distorted symbol.
 */
private fun placeAlignment(g: WorkGrid, row: Int, col: Int) {
    for (dr in -2..2) {
        for (dc in -2..2) {
            val r = row + dr
            val c = col + dc
            val onBorder = dr == -2 || dr == 2 || dc == -2 || dc == 2
            val isCenter = dr == 0 && dc == 0
            g.set(r, c, onBorder || isCenter, reserve = true)
        }
    }
}

/**
 * Place all alignment patterns for the given [version].
 *
 * The positions list from [ALIGNMENT_POSITIONS] gives center coordinates.
 * All pairwise combinations (row × col) are used, EXCEPT positions that
 * would overlap with a finder pattern (already reserved = true).
 */
private fun placeAllAlignments(g: WorkGrid, version: Int) {
    val positions = ALIGNMENT_POSITIONS[version - 1]
    for (row in positions) {
        for (col in positions) {
            // Skip if the center is already reserved (overlaps a finder).
            if (g.reserved[row][col]) continue
            placeAlignment(g, row, col)
        }
    }
}

/**
 * Place the two timing strips.
 *
 * **Horizontal timing** occupies row 6, columns 8 to (size-9).
 * **Vertical timing** occupies column 6, rows 8 to (size-9).
 *
 * Both strips alternate dark/light starting with dark at the finder boundary.
 * Even-indexed positions are dark; odd-indexed are light.
 *
 * Timing strips allow the scanner to measure module size and detect any
 * uniform distortion or scaling, especially critical for large symbols.
 */
private fun placeTiming(g: WorkGrid) {
    val sz = g.size
    for (c in 8..sz - 9) g.set(6, c, c % 2 == 0, reserve = true)
    for (r in 8..sz - 9) g.set(r, 6, r % 2 == 0, reserve = true)
}

/**
 * Reserve all format information module positions (without writing values).
 *
 * Format information is written in two copies (for redundancy):
 * - Copy 1: L-shaped strip around the top-left finder (row 8 and col 8).
 * - Copy 2: strip in row 8 near the top-right finder, and col 8 near the
 *           bottom-left finder.
 *
 * We mark these reserved now so the data placement step skips them.
 * The actual format bits are written later in [writeFormatInfo].
 */
private fun reserveFormatInfo(g: WorkGrid) {
    val sz = g.size
    // Copy 1 row: row 8, cols 0-8 (skipping col 6 which is timing).
    for (c in 0..8) { if (c != 6) g.reserved[8][c] = true }
    // Copy 1 col: col 8, rows 0-8 (skipping row 6 which is timing).
    for (r in 0..8) { if (r != 6) g.reserved[r][8] = true }
    // Copy 2 col: col 8, rows (sz-7) to (sz-1).
    for (r in sz - 7 until sz) g.reserved[r][8] = true
    // Copy 2 row: row 8, cols (sz-8) to (sz-1).
    for (c in sz - 8 until sz) g.reserved[8][c] = true
}

/**
 * Compute the 15-bit format information word.
 *
 * The format word encodes:
 * - 2-bit ECC level indicator (L=01, M=00, Q=11, H=10).
 * - 3-bit mask pattern index (0-7).
 *
 * Construction:
 * ```
 * data = [ecc_level(2)] [mask_pattern(3)]
 * rem  = polynomial remainder of (data << 10) / G(x),
 *        where G(x) = x^10 + x^8 + x^5 + x^4 + x^2 + x + 1 (= 0x537)
 * fmt  = (data << 10) | rem
 * result = fmt XOR 0x5412   (ISO masking to prevent all-zero format info)
 * ```
 *
 * The BCH(15,5) error correction protects the format info against 3-bit
 * burst errors.  The XOR mask 0x5412 ensures the format bits are never
 * all zeros (which would look like no format info at all).
 *
 * @param ecc   ECC level.
 * @param mask  Mask pattern index (0-7).
 * @return      15-bit format information word.
 */
private fun computeFormatBits(ecc: EccLevel, mask: Int): Int {
    val data = (eccIndicator(ecc) shl 3) or mask
    var rem = data shl 10
    for (i in 14 downTo 10) {
        if ((rem shr i) and 1 == 1) rem = rem xor (0x537 shl (i - 10))
    }
    return (((data shl 10) or (rem and 0x3FF)) xor 0x5412)
}

/**
 * Write the 15-bit format information word [fmt] into both copies.
 *
 * **Critical bit ordering** (from lessons.md — a hard-won lesson):
 *
 * The format word is labeled f14 (MSB) down to f0 (LSB).
 *
 * Copy 1 (around the top-left finder):
 * ```
 * Row 8, cols 0-5: f14 → f9  (MSB-first, left-to-right)
 * Row 8, col 7:    f8
 * Row 8, col 8:    f7
 * Col 8, row 7:    f6
 * Col 8, rows 0-5: f0 → f5  (LSB at top, ascending)
 * ```
 *
 * Copy 2 (around top-right / bottom-left finders):
 * ```
 * Row 8, cols (sz-1) → (sz-8): f0 → f7 (LSB at right)
 * Col 8, rows (sz-7) → (sz-1): f8 → f14 (ascending)
 * ```
 *
 * This asymmetric ordering matches the ISO standard.  Getting it wrong produces
 * a QR code that LOOKS correct but cannot be decoded by any standard scanner.
 * Always verify with zbarimg or equivalent after changing this function.
 */
private fun writeFormatInfo(g: WorkGrid, fmt: Int) {
    val sz = g.size

    // ── Copy 1 ──────────────────────────────────────────────────────────────
    // Row 8, cols 0-5: f14 down to f9 (MSB-first).
    for (i in 0..5) g.modules[8][i] = (fmt shr (14 - i)) and 1 == 1
    g.modules[8][7] = (fmt shr 8) and 1 == 1  // f8
    g.modules[8][8] = (fmt shr 7) and 1 == 1  // f7
    g.modules[7][8] = (fmt shr 6) and 1 == 1  // f6
    // Col 8, rows 0-5: f0 at row 0, f5 at row 5 (LSB at top, ascending).
    for (i in 0..5) g.modules[i][8] = (fmt shr i) and 1 == 1

    // ── Copy 2 ──────────────────────────────────────────────────────────────
    // Row 8, cols (sz-1) down to (sz-8): f0 at col sz-1, f7 at col sz-8.
    for (i in 0..7) g.modules[8][sz - 1 - i] = (fmt shr i) and 1 == 1
    // Col 8, rows (sz-7) to (sz-1): f8 at row sz-7, f14 at row sz-1.
    for (i in 8..14) g.modules[sz - 15 + i][8] = (fmt shr i) and 1 == 1
}

/**
 * Reserve the 6×3 version information blocks (versions 7+).
 *
 * Version information is an 18-bit BCH-protected code placed in two blocks:
 * - Near the top-right finder (6 rows × 3 cols, starting at col sz-11).
 * - Near the bottom-left finder (3 rows × 6 cols, starting at row sz-11).
 *
 * Not present in versions 1-6 (too small to need version info).
 */
private fun reserveVersionInfo(g: WorkGrid, version: Int) {
    if (version < 7) return
    val sz = g.size
    for (r in 0..5) for (dc in 0..2) g.reserved[r][sz - 11 + dc] = true
    for (dr in 0..2) for (c in 0..5) g.reserved[sz - 11 + dr][c] = true
}

/**
 * Compute the 18-bit version information word for versions 7-40.
 *
 * The 6-bit version number is encoded with a 12-bit BCH error correction code.
 *
 * Construction:
 * ```
 * rem = polynomial remainder of (version << 12) / G(x),
 *       where G(x) = x^12 + x^11 + x^10 + x^9 + x^8 + x^5 + x^2 + 1 (= 0x1F25)
 * result = (version << 12) | rem
 * ```
 *
 * @param version  QR Code version (7-40).
 * @return         18-bit version information word.
 */
private fun computeVersionBits(version: Int): Int {
    var rem = version shl 12
    for (i in 17 downTo 12) {
        if ((rem shr i) and 1 == 1) rem = rem xor (0x1F25 shl (i - 12))
    }
    return (version shl 12) or (rem and 0xFFF)
}

/**
 * Write the 18-bit version information blocks for versions 7+.
 *
 * The 18 bits are arranged as a 6×3 grid.  Bit i corresponds to:
 *   row = 5 - (i / 3), col = sz - 9 - (i % 3)    (top-right block)
 *   Transposed copy:
 *   row = sz - 9 - (i % 3), col = 5 - (i / 3)    (bottom-left block)
 */
private fun writeVersionInfo(g: WorkGrid, version: Int) {
    if (version < 7) return
    val sz = g.size
    val bits = computeVersionBits(version)
    for (i in 0..17) {
        val dark = (bits shr i) and 1 == 1
        val a = 5 - i / 3
        val b = sz - 9 - i % 3
        g.modules[a][b] = dark
        g.modules[b][a] = dark
    }
}

/**
 * Place the always-dark module at (4V+9, 8).
 *
 * This single module is always dark regardless of masking.  The ISO standard
 * reserves it to ensure format information is never interpreted incorrectly.
 * Scanners use it as a sanity check.
 */
private fun placeDarkModule(g: WorkGrid, version: Int) {
    g.set(4 * version + 9, 8, dark = true, reserve = true)
}

/**
 * Place the interleaved codeword bits into data modules using the two-column
 * zigzag scan.
 *
 * The zigzag visits all non-reserved modules in a very specific order:
 * - Start at the bottom-right corner.
 * - Process 2-column strips, moving left.
 * - Within each strip, zig upward, then zag downward.
 * - Skip column 6 (timing column) by treating it as if it doesn't exist.
 *
 * This ordering ensures adjacent bits in the codeword stream end up in
 * adjacent modules, which minimises the impact of physical damage on the
 * number of affected codewords.
 *
 * ```
 * col = size - 1   direction = upward
 * loop:
 *   for each row in current direction:
 *     for sub_col in [col, col-1]:
 *       if sub_col == 6: skip (timing column)
 *       if module is reserved: skip
 *       place next bit here
 *   flip direction, move col left by 2
 *   if col == 6: col = 5 (skip timing column)
 * ```
 */
private fun placeBits(g: WorkGrid, codewords: ByteArray, version: Int) {
    val sz = g.size

    // Expand codewords to individual bits (MSB first within each byte).
    val bits = mutableListOf<Boolean>()
    for (cw in codewords) {
        for (b in 7 downTo 0) bits.add((cw.toInt() shr b) and 1 == 1)
    }
    // Append zero remainder bits.
    repeat(numRemainderBits(version)) { bits.add(false) }

    var bitIdx = 0
    var up = true
    var col = sz - 1

    while (col >= 1) {
        // Process two-column strip: columns col and col-1.
        for (vi in 0 until sz) {
            val row = if (up) sz - 1 - vi else vi
            for (dc in 0..1) {
                val c = col - dc
                if (c == 6) continue          // skip timing column
                if (g.reserved[row][c]) continue  // skip structural modules
                g.modules[row][c] = if (bitIdx < bits.size) bits[bitIdx] else false
                bitIdx++
            }
        }
        up = !up
        col -= 2
        if (col == 6) col = 5  // jump over timing column
    }
}

/**
 * Build the complete grid structure (finder patterns, separators, timing,
 * alignment, format and version reservations, dark module).
 *
 * After this function, the grid has all structural modules placed and marked
 * reserved.  Data bits can then be placed with [placeBits].
 */
private fun buildGrid(version: Int): WorkGrid {
    val sz = symbolSize(version)
    val g = WorkGrid(sz)

    // Three finder patterns at corners (NOT bottom-right).
    placeFinder(g, 0, 0)          // top-left
    placeFinder(g, 0, sz - 7)     // top-right
    placeFinder(g, sz - 7, 0)     // bottom-left

    // Separators: 1-module-wide light strips surrounding each finder.
    // Top-left separator:
    for (i in 0..7) {
        g.set(7, i, dark = false, reserve = true)
        g.set(i, 7, dark = false, reserve = true)
    }
    // Top-right separator:
    for (i in 0..7) {
        g.set(7, sz - 1 - i, dark = false, reserve = true)
        g.set(i, sz - 8, dark = false, reserve = true)
    }
    // Bottom-left separator:
    for (i in 0..7) {
        g.set(sz - 8, i, dark = false, reserve = true)
        g.set(sz - 1 - i, 7, dark = false, reserve = true)
    }

    placeTiming(g)
    placeAllAlignments(g, version)
    reserveFormatInfo(g)
    reserveVersionInfo(g, version)
    placeDarkModule(g, version)

    return g
}

// =============================================================================
// Masking
// =============================================================================

/**
 * Evaluate the mask condition for the given [mask] pattern at position ([r], [c]).
 *
 * Each mask pattern is a simple arithmetic condition.  When true, the data
 * module at that position is flipped.  The ISO standard defines 8 patterns:
 *
 * | Pattern | Condition                              |
 * |---------|----------------------------------------|
 * | 0       | (row + col) mod 2 == 0                 |
 * | 1       | row mod 2 == 0                         |
 * | 2       | col mod 3 == 0                         |
 * | 3       | (row + col) mod 3 == 0                 |
 * | 4       | (row/2 + col/3) mod 2 == 0             |
 * | 5       | (row×col) mod 2 + (row×col) mod 3 == 0 |
 * | 6       | ((row×col) mod 2 + (row×col) mod 3) mod 2 == 0 |
 * | 7       | ((row+col) mod 2 + (row×col) mod 3) mod 2 == 0 |
 *
 * The encoder tries all 8 patterns and chooses the one with the lowest
 * penalty score.  This prevents degenerate patterns that could confuse scanners.
 */
private fun maskCondition(mask: Int, r: Int, c: Int): Boolean = when (mask) {
    0 -> (r + c) % 2 == 0
    1 -> r % 2 == 0
    2 -> c % 3 == 0
    3 -> (r + c) % 3 == 0
    4 -> (r / 2 + c / 3) % 2 == 0
    5 -> (r * c) % 2 + (r * c) % 3 == 0
    6 -> ((r * c) % 2 + (r * c) % 3) % 2 == 0
    7 -> ((r + c) % 2 + (r * c) % 3) % 2 == 0
    else -> false
}

/**
 * Apply [mask] to the given module grid, returning a new module array.
 *
 * Only non-reserved modules (data and ECC modules) are affected.
 * Structural modules (finders, timing, format info, etc.) are never masked.
 */
private fun applyMask(modules: Array<BooleanArray>, reserved: Array<BooleanArray>, sz: Int, mask: Int): Array<BooleanArray> {
    val result = Array(sz) { r -> BooleanArray(sz) { c -> modules[r][c] } }
    for (r in 0 until sz) {
        for (c in 0 until sz) {
            if (!reserved[r][c]) {
                result[r][c] = modules[r][c] != maskCondition(mask, r, c)
            }
        }
    }
    return result
}

/**
 * Compute the penalty score for a masked grid according to ISO 18004 Section 8.8.
 *
 * The penalty has four components that penalise:
 *
 * **Rule 1 — runs of ≥5 same-color modules in a row or column.**
 * Each run of length L (L ≥ 5) adds L − 2 to the penalty.
 * ```
 * run of 5 → +3,  run of 6 → +4,  run of 7 → +5, ...
 * ```
 *
 * **Rule 2 — 2×2 same-color blocks.**
 * Each 2×2 square of same-color modules adds 3.
 *
 * **Rule 3 — finder-pattern-like sequences.**
 * The patterns 1 0 1 1 1 0 1 0 0 0 0 and its reverse in rows or columns
 * each add 40.  These look like finder patterns and confuse scanners.
 *
 * **Rule 4 — dark module proportion deviation from 50%.**
 * Deviations from a 50:50 dark:light ratio add penalty.
 * The penalty is min(|prev5 - 50|, |next5 - 50|) / 5 × 10, where prev5
 * and next5 are the nearest multiples of 5 below and above the dark%.
 * A symbol with exactly 50% dark modules scores 0 for this rule.
 */
private fun computePenalty(modules: Array<BooleanArray>, sz: Int): Int {
    var penalty = 0

    // Rule 1: runs of ≥5 same-color modules.
    for (a in 0 until sz) {
        for (horiz in listOf(true, false)) {
            var run = 1
            var prev = if (horiz) modules[a][0] else modules[0][a]
            for (i in 1 until sz) {
                val cur = if (horiz) modules[a][i] else modules[i][a]
                if (cur == prev) {
                    run++
                } else {
                    if (run >= 5) penalty += run - 2
                    run = 1
                    prev = cur
                }
            }
            if (run >= 5) penalty += run - 2
        }
    }

    // Rule 2: 2×2 same-color blocks.
    for (r in 0 until sz - 1) {
        for (c in 0 until sz - 1) {
            val d = modules[r][c]
            if (d == modules[r][c + 1] && d == modules[r + 1][c] && d == modules[r + 1][c + 1]) {
                penalty += 3
            }
        }
    }

    // Rule 3: finder-pattern-like sequences.
    // Pattern 1: 1 0 1 1 1 0 1 0 0 0 0 (and its reverse 0 0 0 0 1 0 1 1 1 0 1).
    val p1 = booleanArrayOf(true, false, true, true, true, false, true, false, false, false, false)
    val p2 = booleanArrayOf(false, false, false, false, true, false, true, true, true, false, true)
    for (a in 0 until sz) {
        for (b in 0..sz - 11) {
            var mh1 = true; var mh2 = true; var mv1 = true; var mv2 = true
            for (k in 0..10) {
                val bh = modules[a][b + k]
                val bv = modules[b + k][a]
                if (bh != p1[k]) mh1 = false
                if (bh != p2[k]) mh2 = false
                if (bv != p1[k]) mv1 = false
                if (bv != p2[k]) mv2 = false
            }
            if (mh1) penalty += 40
            if (mh2) penalty += 40
            if (mv1) penalty += 40
            if (mv2) penalty += 40
        }
    }

    // Rule 4: dark module proportion.
    var dark = 0
    for (r in 0 until sz) for (c in 0 until sz) if (modules[r][c]) dark++
    val total = sz * sz
    val ratio = dark.toDouble() / total * 100.0
    val prev5 = (ratio / 5.0).toInt() * 5
    val a = Math.abs(prev5 - 50)
    val b = Math.abs(prev5 + 5 - 50)
    penalty += (minOf(a, b) / 5) * 10

    return penalty
}

// =============================================================================
// Version selection
// =============================================================================

/**
 * Select the minimum QR Code version (1-40) that can fit [input] at the given
 * [ecc] level.
 *
 * The selection checks versions in ascending order, computing the number of
 * bits required for the input in its chosen mode and comparing against the
 * available data codewords.
 *
 * A version fits if:
 * ```
 * ceil((4 + char_count_bits + data_bits) / 8) <= data_codewords
 * ```
 *
 * Where data_bits depends on the encoding mode:
 * - Byte: 8 bits per UTF-8 byte.
 * - Alphanumeric: 11 bits per pair + 6 bits for a trailing single char.
 * - Numeric: 10 bits per triple + 7 bits for a pair + 4 bits for a single.
 *
 * @throws [QRCodeError.InputTooLong] if no version fits.
 */
private fun selectVersion(input: String, ecc: EccLevel, minVersion: Int = 1): Result<Int> {
    val mode = selectMode(input)
    val utf8Bytes = input.toByteArray(Charsets.UTF_8)

    for (v in minVersion..40) {
        val capacity = numDataCodewords(v, ecc)
        val dataBits: Int = when (mode) {
            EncodingMode.BYTE -> utf8Bytes.size * 8
            EncodingMode.NUMERIC -> {
                val n = input.length
                (n * 10 + 2) / 3  // ceil(n * 10 / 3)
            }
            EncodingMode.ALPHANUMERIC -> {
                val n = input.length
                (n * 11 + 1) / 2  // ceil(n * 11 / 2)
            }
        }
        val bitsNeeded = 4 + charCountBits(mode, v) + dataBits
        val cwNeeded = (bitsNeeded + 7) / 8
        if (cwNeeded <= capacity) return Result.success(v)
    }

    return Result.failure(
        QRCodeError.InputTooLong(
            "Input (${utf8Bytes.size} bytes, ECC=$ecc) exceeds version-40 capacity."
        )
    )
}

// =============================================================================
// Public API
// =============================================================================

/**
 * Encode a UTF-8 string into a QR Code [ModuleGrid].
 *
 * Returns a `(4V+17) × (4V+17)` boolean grid, `true` = dark module.
 * Automatically selects the minimum version (1-40) that fits the input at
 * the given ECC level.
 *
 * ## Example
 * ```kotlin
 * val grid = encode("HELLO WORLD", EccLevel.M).getOrThrow()
 * assert(grid.rows == 21)  // version 1
 * ```
 *
 * @param input       The UTF-8 string to encode.
 * @param ecc         Error correction level.
 * @param minVersion  Minimum version to use (default 1).  Use to force a larger symbol.
 * @return            [Result] containing the [ModuleGrid] or a [QRCodeError].
 */
fun encode(input: String, ecc: EccLevel, minVersion: Int = 1): Result<ModuleGrid> {
    // Guard: QR Code v40 holds at most 7089 numeric characters (~2953 bytes).
    // Reject enormous inputs early to avoid large allocations before version
    // selection can reject them.
    if (input.toByteArray(Charsets.UTF_8).size > 7089) {
        return Result.failure(
            QRCodeError.InputTooLong(
                "Input byte length ${input.toByteArray(Charsets.UTF_8).size} " +
                "exceeds 7089 (the QR Code v40 maximum)."
            )
        )
    }

    val version = selectVersion(input, ecc, minVersion).getOrElse { return Result.failure(it) }
    val sz = symbolSize(version)

    // Build data codewords, compute RS ECC, and interleave.
    val dataCw = buildDataCodewords(input, version, ecc)
    val blocks = computeBlocks(dataCw, version, ecc)
    val interleaved = interleaveBlocks(blocks)

    // Build the structural grid (finders, separators, timing, alignment, dark module).
    val grid = buildGrid(version)

    // Place the interleaved message stream into non-reserved modules.
    placeBits(grid, interleaved, version)

    // Evaluate all 8 mask patterns, pick the one with the lowest penalty.
    var bestMask = 0
    var bestPenalty = Int.MAX_VALUE
    for (m in 0..7) {
        val masked = applyMask(grid.modules, grid.reserved, sz, m)
        val fmt = computeFormatBits(ecc, m)
        // Temporarily apply format info to score the complete grid.
        val testGrid = WorkGrid(sz)
        for (r in 0 until sz) for (c in 0 until sz) {
            testGrid.modules[r][c] = masked[r][c]
            testGrid.reserved[r][c] = grid.reserved[r][c]
        }
        writeFormatInfo(testGrid, fmt)
        val p = computePenalty(testGrid.modules, sz)
        if (p < bestPenalty) { bestPenalty = p; bestMask = m }
    }

    // Finalize: apply the best mask and write format/version info.
    val finalMods = applyMask(grid.modules, grid.reserved, sz, bestMask)
    val finalGrid = WorkGrid(sz)
    for (r in 0 until sz) for (c in 0 until sz) {
        finalGrid.modules[r][c] = finalMods[r][c]
        finalGrid.reserved[r][c] = grid.reserved[r][c]
    }
    writeFormatInfo(finalGrid, computeFormatBits(ecc, bestMask))
    writeVersionInfo(finalGrid, version)

    return Result.success(finalGrid.toModuleGrid())
}

/**
 * Encode a UTF-8 string and convert the resulting [ModuleGrid] to a pixel-
 * resolved [PaintScene] ready for the PaintVM.
 *
 * This is a convenience wrapper that chains [encode] and [layout].
 *
 * @param input   The UTF-8 string to encode.
 * @param ecc     Error correction level.
 * @param config  Layout configuration (module size, quiet zone, colors).
 * @return        [Result] containing the [PaintScene] or a [QRCodeError].
 */
fun encodeAndLayout(
    input: String,
    ecc: EccLevel,
    config: Barcode2DLayoutConfig = Barcode2DLayoutConfig(),
): Result<PaintScene> {
    val grid = encode(input, ecc).getOrElse { return Result.failure(it) }
    return try {
        Result.success(layout(grid, config))
    } catch (e: Exception) {
        Result.failure(
            QRCodeError.LayoutError(
                "barcode-2d layout failed: ${e.message ?: "check config (moduleSizePx > 0, moduleShape = SQUARE)"}"
            )
        )
    }
}
