/**
 * AztecCode.kt — Aztec Code encoder, ISO/IEC 24778:2008 compliant.
 *
 * Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995 and
 * published as a patent-free format. Unlike QR Code (which uses three square
 * finder patterns at three corners), Aztec Code places a single **bullseye
 * finder pattern at the center** of the symbol. The scanner finds the center
 * first, then reads outward in a spiral — no large quiet zone is needed.
 *
 * ## Where Aztec Code is used today
 *
 * - **IATA boarding passes** — the barcode on every airline boarding pass
 * - **Eurostar and Amtrak rail tickets** — printed and on-screen tickets
 * - **PostNL, Deutsche Post, La Poste** — European postal routing
 * - **US military ID cards**
 *
 * ## Symbol variants
 *
 * ```
 * Compact: 1-4 layers,  size = 11 + 4*layers  (15x15 to 27x27)
 * Full:    1-32 layers, size = 15 + 4*layers  (19x19 to 143x143)
 * ```
 *
 * ## Encoding pipeline (v0.1.0 — byte-mode only)
 *
 * ```
 * input string / bytes
 *   -> Binary-Shift codewords from Upper mode
 *   -> symbol size selection (smallest compact then full that fits at 23% ECC)
 *   -> pad to exact codeword count
 *   -> GF(256)/0x12D Reed-Solomon ECC (poly 0x12D, b=1 roots alpha^1..alpha^n)
 *   -> bit stuffing (insert complement after 4 consecutive identical bits)
 *   -> GF(16) mode message (layers + codeword count + 5 or 6 RS nibbles)
 *   -> ModuleGrid  (bullseye -> orientation marks -> mode msg -> data spiral)
 * ```
 *
 * ## v0.1.0 simplifications
 *
 * 1. Byte-mode only — all input encoded via Binary-Shift from Upper mode.
 *    Multi-mode (Digit/Upper/Lower/Mixed/Punct) optimization is v0.2.0.
 * 2. 8-bit codewords -> GF(256)/0x12D RS. Note 0x12D ≠ 0x11D (QR Code).
 *    0x12D is the same polynomial as Data Matrix ECC200.
 * 3. Default ECC = 23%.
 * 4. Auto-select compact vs full (force-compact option is v0.2.0).
 *
 * Spec: code/specs/aztec-code.md
 */
package com.codingadventures.azteccode

import com.codingadventures.barcode2d.ModuleGrid
import com.codingadventures.barcode2d.ModuleShape

/** Package version. */
const val VERSION = "0.1.0"

// =============================================================================
// Error types
// =============================================================================

/**
 * Base error class for all Aztec Code failures.
 *
 * Using a sealed class lets callers `when`-match exhaustively on error types
 * without needing a catch-all branch.
 */
sealed class AztecError(message: String) : Exception(message) {

    /**
     * Thrown when the input is too long to fit in a 32-layer full Aztec symbol.
     *
     * The maximum byte capacity at 23% ECC for a 32-layer full symbol is
     * approximately 1437 bytes (byte mode).
     */
    class InputTooLong(message: String) : AztecError(message)
}

// =============================================================================
// Options
// =============================================================================

/**
 * Options for [encode].
 *
 * All fields are optional. Defaults are chosen by the encoder.
 *
 * @param minEccPercent Minimum ECC percentage (default: 23, range: 10–90).
 *   Higher values trade symbol capacity for error recovery.
 */
data class AztecOptions(
    val minEccPercent: Int = 23,
)

// =============================================================================
// GF(16) arithmetic — for the mode message Reed-Solomon
// =============================================================================
//
// GF(16) is the finite field with 16 elements, built from the primitive
// polynomial:
//
//   p(x) = x^4 + x + 1   (binary: 10011 = 0x13)
//
// Every non-zero element can be written as a power of the primitive element
// alpha. alpha is the root of p(x), so alpha^4 = alpha + 1.
//
// The discrete log table maps a field element (1..15) to its exponent (0..14).
// The antilog table maps an exponent to its field element.
//
//   alpha^0 = 1,  alpha^1 = 2,  alpha^2 = 4,  alpha^3 = 8
//   alpha^4 = 3,  alpha^5 = 6,  alpha^6 = 12, alpha^7 = 11
//   alpha^8 = 5,  alpha^9 = 10, alpha^10= 7,  alpha^11= 14
//   alpha^12= 15, alpha^13= 13, alpha^14= 9,  alpha^15= 1 (period = 15)

/** GF(16) discrete logarithm: LOG16[e] = i means alpha^i = e. LOG16[0] = -1 (undefined). */
private val LOG16 = intArrayOf(
    -1,  // log(0) undefined
     0,  // log(1) = 0
     1,  // log(2) = 1
     4,  // log(3) = 4
     2,  // log(4) = 2
     8,  // log(5) = 8
     5,  // log(6) = 5
    10,  // log(7) = 10
     3,  // log(8) = 3
    14,  // log(9) = 14
     9,  // log(10) = 9
     7,  // log(11) = 7
     6,  // log(12) = 6
    13,  // log(13) = 13
    11,  // log(14) = 11
    12,  // log(15) = 12
)

/** GF(16) antilogarithm: ALOG16[i] = alpha^i. The table is 16 entries with ALOG16[15]=ALOG16[0]. */
private val ALOG16 = intArrayOf(
    1, 2, 4, 8, 3, 6, 12, 11, 5, 10, 7, 14, 15, 13, 9, 1
)

/**
 * Multiply two GF(16) elements.
 *
 * Uses the log/antilog trick:
 *   a * b = alpha^( (log(a) + log(b)) mod 15 )
 *
 * Returns 0 if either operand is 0 (zero is the additive identity and has
 * no logarithm).
 */
internal fun gf16Mul(a: Int, b: Int): Int {
    if (a == 0 || b == 0) return 0
    return ALOG16[(LOG16[a] + LOG16[b]) % 15]
}

/**
 * Build the GF(16) RS generator polynomial with roots alpha^1 through alpha^n.
 *
 * Returns little-endian coefficients [g_0, g_1, ..., g_{n-1}, g_n] where g_n = 1.
 *
 * ## Algorithm
 *
 * Start with g(x) = 1. For each root alpha^i, multiply by (x - alpha^i).
 * In GF(16), subtraction is XOR (same as addition), so (x - alpha^i) = (x + alpha^i).
 *
 * The result is a degree-n polynomial with monic leading coefficient.
 */
internal fun buildGf16Generator(n: Int): IntArray {
    var g = intArrayOf(1)
    for (i in 1..n) {
        val ai = ALOG16[i % 15]
        val next = IntArray(g.size + 1)
        for (j in g.indices) {
            next[j + 1] = next[j + 1] xor g[j]
            next[j] = next[j] xor gf16Mul(ai, g[j])
        }
        g = next
    }
    return g
}

/**
 * Compute n GF(16) RS check nibbles for the given data nibbles.
 *
 * Uses the LFSR (shift-register) polynomial division algorithm:
 *   - Feed each data nibble through the shift register.
 *   - The register state after all data is the remainder (check symbols).
 *
 * @param data  Array of 4-bit values (0..15).
 * @param n     Number of check nibbles to produce.
 * @return      n check nibbles.
 */
internal fun gf16RsEncode(data: IntArray, n: Int): IntArray {
    val g = buildGf16Generator(n)
    val rem = IntArray(n)
    for (byte in data) {
        val fb = byte xor rem[0]
        for (i in 0 until n - 1) {
            rem[i] = rem[i + 1] xor gf16Mul(g[i + 1], fb)
        }
        rem[n - 1] = gf16Mul(g[n], fb)
    }
    return rem
}

// =============================================================================
// GF(256)/0x12D arithmetic — for 8-bit data codewords
// =============================================================================
//
// Aztec Code uses GF(256) with primitive polynomial:
//   p(x) = x^8 + x^5 + x^4 + x^2 + x + 1  =  0x12D
//
// IMPORTANT: This is the SAME polynomial as Data Matrix ECC200, but DIFFERENT
// from QR Code (0x11D). The existing reed-solomon package in this repo uses
// 0x11D, so we implement GF(256)/0x12D inline rather than reusing it.
//
// Generator convention: b=1, roots alpha^1..alpha^n (MA02 style).

private const val GF256_POLY = 0x12d

// Build GF(256)/0x12D log/antilog tables via a private object so we can use
// an init block. Kotlin files do not have a class-level init scope; an object
// initializer runs once when the object is first accessed (lazy singleton).
private object Gf256Tables {
    /** EXP_12D[i] = alpha^i in GF(256)/0x12D, doubled for wrap-free multiply. */
    val EXP_12D = IntArray(512)

    /** LOG_12D[e] = discrete log of e in GF(256)/0x12D. */
    val LOG_12D = IntArray(256)

    init {
        // alpha = 2 is the primitive element.
        var x = 1
        for (i in 0 until 255) {
            EXP_12D[i] = x
            EXP_12D[i + 255] = x
            LOG_12D[x] = i
            x = x shl 1
            if (x and 0x100 != 0) x = x xor GF256_POLY
            x = x and 0xff
        }
        EXP_12D[255] = 1
    }
}

/**
 * Multiply two GF(256)/0x12D elements.
 *
 * Uses the doubled EXP table for O(1) multiply without a modular-add:
 *   a * b = EXP_12D[ LOG_12D[a] + LOG_12D[b] ]
 * (indices up to 509 are valid in the doubled table.)
 */
internal fun gf256Mul(a: Int, b: Int): Int {
    if (a == 0 || b == 0) return 0
    return Gf256Tables.EXP_12D[Gf256Tables.LOG_12D[a] + Gf256Tables.LOG_12D[b]]
}

/**
 * Build the GF(256)/0x12D RS generator polynomial with roots alpha^1..alpha^n.
 *
 * Returns little-endian coefficients. EXP_12D[i] gives alpha^i.
 */
private fun buildGf256Generator(n: Int): IntArray {
    var g = intArrayOf(1)
    for (i in 1..n) {
        val ai = Gf256Tables.EXP_12D[i]
        val next = IntArray(g.size + 1)
        for (j in g.indices) {
            next[j] = next[j] xor g[j]
            next[j + 1] = next[j + 1] xor gf256Mul(g[j], ai)
        }
        g = next
    }
    return g
}

/**
 * Compute [nCheck] GF(256)/0x12D RS check bytes for [data].
 *
 * LFSR polynomial division: feed each data byte, update shift register.
 *
 * @param data    Data byte values (0..255).
 * @param nCheck  Number of RS check bytes to produce.
 * @return        [nCheck] check bytes.
 */
internal fun gf256RsEncode(data: IntArray, nCheck: Int): IntArray {
    val g = buildGf256Generator(nCheck)
    val rem = IntArray(nCheck)
    for (b in data) {
        val fb = b xor rem[0]
        for (i in 0 until nCheck - 1) {
            rem[i] = rem[i + 1] xor gf256Mul(g[i + 1], fb)
        }
        rem[nCheck - 1] = gf256Mul(g[nCheck], fb)
    }
    return rem
}

// =============================================================================
// Capacity tables
// =============================================================================
//
// Derived from ISO/IEC 24778:2008 Table 1.
// totalBits  = total data+ECC bit positions in the symbol.
// maxBytes8  = max 8-bit codewords that can be stored (= totalBits / 8).

/** Capacity entry: (totalBits, maxBytes8 for 8-bit codewords). */
private data class CapEntry(val totalBits: Int, val maxBytes8: Int)

/**
 * Compact Aztec capacity per layer count (index = layer count, index 0 unused).
 *
 * Symbol size = 11 + 4*layers.
 *
 * | Layers | Size  | Total bits | Max 8-bit CW |
 * |--------|-------|------------|--------------|
 * | 1      | 15×15 |  72        |  9           |
 * | 2      | 19×19 | 200        | 25           |
 * | 3      | 23×23 | 392        | 49           |
 * | 4      | 27×27 | 648        | 81           |
 */
private val COMPACT_CAPACITY = arrayOf(
    CapEntry(0, 0),   // index 0 unused
    CapEntry(72, 9),
    CapEntry(200, 25),
    CapEntry(392, 49),
    CapEntry(648, 81),
)

/**
 * Full Aztec capacity per layer count (index = layer count, index 0 unused).
 *
 * Symbol size = 15 + 4*layers.
 *
 * The capacities below are the total usable data-layer modules (each holds
 * one bit), from ISO/IEC 24778:2008 Table 1.
 */
private val FULL_CAPACITY = arrayOf(
    CapEntry(0, 0),
    CapEntry(88, 11),
    CapEntry(216, 27),
    CapEntry(360, 45),
    CapEntry(520, 65),
    CapEntry(696, 87),
    CapEntry(888, 111),
    CapEntry(1096, 137),
    CapEntry(1320, 165),
    CapEntry(1560, 195),
    CapEntry(1816, 227),
    CapEntry(2088, 261),
    CapEntry(2376, 297),
    CapEntry(2680, 335),
    CapEntry(3000, 375),
    CapEntry(3336, 417),
    CapEntry(3688, 461),
    CapEntry(4056, 507),
    CapEntry(4440, 555),
    CapEntry(4840, 605),
    CapEntry(5256, 657),
    CapEntry(5688, 711),
    CapEntry(6136, 767),
    CapEntry(6600, 825),
    CapEntry(7080, 885),
    CapEntry(7576, 947),
    CapEntry(8088, 1011),
    CapEntry(8616, 1077),
    CapEntry(9160, 1145),
    CapEntry(9720, 1215),
    CapEntry(10296, 1287),
    CapEntry(10888, 1361),
    CapEntry(11496, 1437),
)

// =============================================================================
// Data encoding — Binary-Shift from Upper mode
// =============================================================================
//
// All input in v0.1.0 is encoded as one Binary-Shift block from Upper mode:
//   1. Emit 5 bits = 0b11111 (Binary-Shift escape in Upper mode)
//   2. If len <= 31: 5 bits for length
//      If len > 31:  5 bits = 0b00000, then 11 bits for length
//   3. Each byte as 8 bits, MSB first

/**
 * Encode input bytes as a flat bit array using the Binary-Shift escape.
 *
 * The resulting bit stream is the raw data to be packed into codewords.
 * Bits are stored as Int (0 or 1), MSB first within each byte.
 *
 * Example for "A" (one byte = 0x41):
 * ```
 * [1,1,1,1,1]      = 5-bit Binary-Shift escape (31 decimal)
 * [0,0,0,0,1]      = 5-bit length (1 byte)
 * [0,1,0,0,0,0,0,1] = byte 'A' = 0x41
 * ```
 */
internal fun encodeBytesAsBits(input: ByteArray): IntArray {
    val bits = mutableListOf<Int>()

    fun writeBits(value: Int, count: Int) {
        for (i in count - 1 downTo 0) {
            bits.add((value shr i) and 1)
        }
    }

    val len = input.size
    writeBits(31, 5) // Binary-Shift escape codeword

    if (len <= 31) {
        writeBits(len, 5)
    } else {
        writeBits(0, 5)
        writeBits(len, 11)
    }

    for (byte in input) {
        writeBits(byte.toInt() and 0xff, 8)
    }

    return bits.toIntArray()
}

// =============================================================================
// Symbol size selection
// =============================================================================

/**
 * Describes the selected symbol configuration.
 *
 * @param compact      true = compact Aztec (1-4 layers), false = full (1-32).
 * @param layers       Number of data layers.
 * @param dataCwCount  Number of 8-bit data codewords.
 * @param eccCwCount   Number of 8-bit RS check codewords.
 * @param totalBits    Total data+ECC bit capacity of the symbol.
 */
internal data class SymbolSpec(
    val compact: Boolean,
    val layers: Int,
    val dataCwCount: Int,
    val eccCwCount: Int,
    val totalBits: Int,
)

/**
 * Select the smallest Aztec symbol that can hold [dataBitCount] bits at [minEccPct].
 *
 * Tries compact layers 1-4 first (smallest symbols), then full layers 1-32.
 * Adds 20% stuffing overhead as a conservative safety margin.
 *
 * @throws AztecError.InputTooLong if no symbol can fit the data.
 */
internal fun selectSymbol(dataBitCount: Int, minEccPct: Int): SymbolSpec {
    // Add 20% overhead for worst-case bit stuffing expansion.
    val stuffedBitCount = Math.ceil(dataBitCount * 1.2).toInt()

    for (layers in 1..4) {
        val cap = COMPACT_CAPACITY[layers]
        val totalBytes = cap.maxBytes8
        val eccCwCount = Math.ceil((minEccPct.toDouble() / 100) * totalBytes).toInt()
        val dataCwCount = totalBytes - eccCwCount
        if (dataCwCount <= 0) continue
        if (Math.ceil(stuffedBitCount.toDouble() / 8).toInt() <= dataCwCount) {
            return SymbolSpec(
                compact = true,
                layers = layers,
                dataCwCount = dataCwCount,
                eccCwCount = eccCwCount,
                totalBits = cap.totalBits,
            )
        }
    }

    for (layers in 1..32) {
        val cap = FULL_CAPACITY[layers]
        val totalBytes = cap.maxBytes8
        val eccCwCount = Math.ceil((minEccPct.toDouble() / 100) * totalBytes).toInt()
        val dataCwCount = totalBytes - eccCwCount
        if (dataCwCount <= 0) continue
        if (Math.ceil(stuffedBitCount.toDouble() / 8).toInt() <= dataCwCount) {
            return SymbolSpec(
                compact = false,
                layers = layers,
                dataCwCount = dataCwCount,
                eccCwCount = eccCwCount,
                totalBits = cap.totalBits,
            )
        }
    }

    throw AztecError.InputTooLong(
        "Input is too long ($dataBitCount bits) to fit in any Aztec Code symbol"
    )
}

// =============================================================================
// Padding
// =============================================================================

/**
 * Pad a bit array to exactly [targetBytes] * 8 bits.
 *
 * - First, zero-pad to the next byte boundary.
 * - Then, zero-pad to the full target byte count.
 * - Truncate if already longer.
 *
 * @param bits         Source bit array (Int 0/1 values).
 * @param targetBytes  Target codeword count.
 * @return             Exactly targetBytes*8 bits.
 */
internal fun padToBytes(bits: IntArray, targetBytes: Int): IntArray {
    val out = bits.toMutableList()
    while (out.size % 8 != 0) out.add(0)
    while (out.size < targetBytes * 8) out.add(0)
    return out.take(targetBytes * 8).toIntArray()
}

// =============================================================================
// Bit stuffing
// =============================================================================
//
// After every 4 consecutive identical bits (all 0 or all 1), insert one
// complement bit. Applies only to the data+ECC bit stream.
//
// Example (showing inserted bits in square brackets):
//
//   Input:   1 1 1 1 0 0 0 0
//   After 4× 1: insert [0]  →  1 1 1 1 [0] 0 0 0 0
//   After 4× 0: insert [1]  →  1 1 1 1 [0] 0 0 0 0 [1]

/**
 * Apply Aztec bit stuffing to the combined data+ECC bit stream.
 *
 * After every run of 4 identical consecutive bits, a complement bit is
 * inserted. This prevents long uniform runs that could confuse the scanner
 * when reading the spiral, and ensures the reference grid remains distinct.
 *
 * The decoder reverses this by removing the bit after every group of 4 identical bits.
 */
internal fun stuffBits(bits: IntArray): IntArray {
    val stuffed = mutableListOf<Int>()
    var runVal = -1
    var runLen = 0

    for (bit in bits) {
        if (bit == runVal) {
            runLen++
        } else {
            runVal = bit
            runLen = 1
        }

        stuffed.add(bit)

        if (runLen == 4) {
            // Insert opposite bit; it starts a new run of length 1.
            val stuffBit = 1 - bit
            stuffed.add(stuffBit)
            runVal = stuffBit
            runLen = 1
        }
    }

    return stuffed.toIntArray()
}

// =============================================================================
// Mode message encoding
// =============================================================================
//
// The mode message encodes layer count and data codeword count, then adds
// GF(16) Reed-Solomon error correction. It is placed in the ring immediately
// outside the bullseye.
//
// Compact (28 bits = 7 nibbles):
//   m = ((layers-1) << 6) | (dataCwCount-1)
//   2 data nibbles + 5 ECC nibbles
//
// Full (40 bits = 10 nibbles):
//   m = ((layers-1) << 11) | (dataCwCount-1)
//   4 data nibbles + 6 ECC nibbles
//
// Nibble ordering: LSB nibble first, then MSB nibble.
// Each nibble is flattened MSB-first into 4 bits.

/**
 * Encode the mode message as a flat bit array (28 bits compact, 40 bits full).
 *
 * ### Compact encoding
 *
 * ```
 * m = ((layers - 1) << 6) | (dataCwCount - 1)   // 8-bit value
 * nibble[0] = m & 0xF      (bits 3..0 of m)
 * nibble[1] = (m >> 4) & 0xF  (bits 7..4 of m)
 * + 5 GF(16) RS ECC nibbles = 7 nibbles total
 * ```
 *
 * ### Full encoding
 *
 * ```
 * m = ((layers - 1) << 11) | (dataCwCount - 1)  // 16-bit value
 * nibble[0..3] = m bits packed LSB-nibble-first
 * + 6 GF(16) RS ECC nibbles = 10 nibbles total
 * ```
 */
internal fun encodeModeMessage(compact: Boolean, layers: Int, dataCwCount: Int): IntArray {
    val dataNibbles: IntArray
    val numEcc: Int

    if (compact) {
        val m = ((layers - 1) shl 6) or (dataCwCount - 1)
        dataNibbles = intArrayOf(m and 0xf, (m shr 4) and 0xf)
        numEcc = 5
    } else {
        val m = ((layers - 1) shl 11) or (dataCwCount - 1)
        dataNibbles = intArrayOf(
            m and 0xf,
            (m shr 4) and 0xf,
            (m shr 8) and 0xf,
            (m shr 12) and 0xf,
        )
        numEcc = 6
    }

    val eccNibbles = gf16RsEncode(dataNibbles, numEcc)
    val allNibbles = dataNibbles + eccNibbles

    // Flatten nibbles to bits, MSB first within each nibble.
    val bits = mutableListOf<Int>()
    for (nibble in allNibbles) {
        for (i in 3 downTo 0) {
            bits.add((nibble shr i) and 1)
        }
    }
    return bits.toIntArray()
}

// =============================================================================
// Grid geometry helpers
// =============================================================================

/**
 * Compute symbol size.
 *
 * - Compact: 11 + 4 * layers  (11×11 bullseye + 4 modules per side per layer)
 * - Full:    15 + 4 * layers  (15×15 bullseye + 4 modules per side per layer)
 */
internal fun symbolSize(compact: Boolean, layers: Int): Int =
    if (compact) 11 + 4 * layers else 15 + 4 * layers

/**
 * Bullseye Chebyshev radius from center.
 *
 * - Compact: radius 5 → 11×11 solid bullseye area
 * - Full:    radius 7 → 15×15 solid bullseye area
 *
 * The outermost ring of the bullseye is always DARK.
 */
internal fun bullseyeRadius(compact: Boolean): Int = if (compact) 5 else 7

// =============================================================================
// Grid construction — bullseye, reference grid, orientation + mode message
// =============================================================================

/**
 * Draw the bullseye concentric ring pattern.
 *
 * The Chebyshev distance `d = max(|col - cx|, |row - cy|)` determines the ring:
 *
 * ```
 * d == 0 or 1  →  DARK   (inner 3×3 solid core)
 * d even       →  LIGHT  (gap rings)
 * d odd        →  DARK   (ring bands)
 * ```
 *
 * The result is a distinctive bull's-eye visible from any scan angle with a
 * 1:1:1:1:1 cross-ratio, allowing the scanner to determine orientation.
 *
 * @param modules  Mutable grid (row-major, true = dark).
 * @param reserved Mark-as-reserved grid (true = cannot be used for data).
 * @param cx       Center column.
 * @param cy       Center row.
 * @param compact  Whether this is a compact symbol.
 */
internal fun drawBullseye(
    modules: Array<BooleanArray>,
    reserved: Array<BooleanArray>,
    cx: Int,
    cy: Int,
    compact: Boolean,
) {
    val br = bullseyeRadius(compact)
    for (row in cy - br..cy + br) {
        for (col in cx - br..cx + br) {
            val d = maxOf(Math.abs(col - cx), Math.abs(row - cy))
            // d==0 and d==1 are both DARK (inner solid 3×3 core)
            val dark = d <= 1 || d % 2 == 1
            modules[row][col] = dark
            reserved[row][col] = true
        }
    }
}

/**
 * Draw the reference grid for full (not compact) Aztec symbols.
 *
 * Reference grid lines appear at rows/columns that are multiples of 16 from
 * the center. Module values alternate dark/light from the center module.
 *
 * This helps scanners correct for severe perspective distortion on large symbols.
 * Reference grid modules are structural and do not carry data.
 *
 * ```
 * At intersection of two grid lines:    DARK
 * On horizontal grid line only:        dark if (cx - col) % 2 == 0
 * On vertical grid line only:          dark if (cy - row) % 2 == 0
 * ```
 */
internal fun drawReferenceGrid(
    modules: Array<BooleanArray>,
    reserved: Array<BooleanArray>,
    cx: Int,
    cy: Int,
    size: Int,
) {
    for (row in 0 until size) {
        for (col in 0 until size) {
            val onH = (cy - row) % 16 == 0
            val onV = (cx - col) % 16 == 0
            if (!onH && !onV) continue

            val dark = when {
                onH && onV -> true
                onH        -> (cx - col) % 2 == 0
                else       -> (cy - row) % 2 == 0
            }
            modules[row][col] = dark
            reserved[row][col] = true
        }
    }
}

/**
 * Place orientation marks and mode message bits in the mode message ring.
 *
 * The mode message ring is the perimeter at Chebyshev radius (bullseyeRadius+1)
 * from center. Its four corner modules are orientation marks (always DARK).
 * The remaining non-corner perimeter positions carry mode message bits clockwise
 * from the top-left corner + 1 position.
 *
 * ### Why orientation marks?
 *
 * The concentric bullseye rings have 4-fold rotational symmetry — you cannot
 * tell which way is "up" just from the bullseye. The four dark corner modules
 * break this symmetry: the scanner reads the mode message clockwise starting
 * from the top-right of the mode ring, so knowing which corner is top-left
 * lets it decode layers and codeword count from the correct position.
 *
 * @return The non-corner ring positions NOT used by the mode message
 *   (these will be filled by the start of the data bit stream).
 */
internal fun drawOrientationAndModeMessage(
    modules: Array<BooleanArray>,
    reserved: Array<BooleanArray>,
    cx: Int,
    cy: Int,
    compact: Boolean,
    modeMessageBits: IntArray,
): List<Pair<Int, Int>> {
    val r = bullseyeRadius(compact) + 1  // Chebyshev radius of the mode ring

    // Enumerate non-corner perimeter positions clockwise, starting from TL+1.
    val nonCorner = mutableListOf<Pair<Int, Int>>()

    // Top edge (skip TL and TR corners)
    for (col in cx - r + 1..cx + r - 1) nonCorner.add(Pair(col, cy - r))
    // Right edge (skip TR and BR corners)
    for (row in cy - r + 1..cy + r - 1) nonCorner.add(Pair(cx + r, row))
    // Bottom edge: right to left (skip BR and BL corners)
    for (col in cx + r - 1 downTo cx - r + 1) nonCorner.add(Pair(col, cy + r))
    // Left edge: bottom to top (skip BL and TL corners)
    for (row in cy + r - 1 downTo cy - r + 1) nonCorner.add(Pair(cx - r, row))

    // Place the 4 orientation mark corners as DARK.
    val corners = listOf(
        Pair(cx - r, cy - r),
        Pair(cx + r, cy - r),
        Pair(cx + r, cy + r),
        Pair(cx - r, cy + r),
    )
    for ((col, row) in corners) {
        modules[row][col] = true
        reserved[row][col] = true
    }

    // Place mode message bits into the non-corner ring positions.
    for (i in modeMessageBits.indices) {
        if (i >= nonCorner.size) break
        val (col, row) = nonCorner[i]
        modules[row][col] = modeMessageBits[i] == 1
        reserved[row][col] = true
    }

    // Return the remaining positions for data bits (after mode msg).
    return nonCorner.drop(modeMessageBits.size)
}

// =============================================================================
// Data layer spiral placement
// =============================================================================
//
// After bullseye and mode message, data bits are placed in a clockwise spiral
// starting from the innermost data layer. Each layer band is 2 modules wide.
// At each position along the layer perimeter, bits are placed in pairs:
// outer row/column first, then inner.
//
// For compact: first data layer inner radius = bullseyeRadius + 2 = 7
// For full:    first data layer inner radius = bullseyeRadius + 2 = 9
//
// Each layer L (0-indexed) has:
//   dI = bullseyeRadius + 2 + 2*L   (inner radius)
//   dO = dI + 1                      (outer radius)

/**
 * Place all data bits using the clockwise layer spiral.
 *
 * The spiral visits each layer from innermost (L=0) to outermost (L=layers-1).
 * Within each layer, bits are placed in pairs along the four sides:
 *
 * ```
 * Top edge:    outer row first (cy - dO), then inner (cy - dI), left to right
 * Right edge:  outer col first (cx + dO), then inner (cx + dI), top to bottom
 * Bottom edge: outer row first (cy + dO), then inner (cy + dI), right to left
 * Left edge:   outer col first (cx - dO), then inner (cx - dI), bottom to top
 * ```
 *
 * The mode ring has [modeRingRemainingPositions] modules that are filled first
 * before the spiral starts. Any reserved module (bullseye, reference grid, etc.)
 * is silently skipped — the bit index only advances on actual placements.
 *
 * @param modules                    Mutable grid to write into.
 * @param reserved                   Which positions are already locked.
 * @param bits                       Stuffed bit stream (data + ECC).
 * @param cx                         Center column.
 * @param cy                         Center row.
 * @param compact                    Symbol variant.
 * @param layers                     Number of data layers.
 * @param modeRingRemainingPositions Mode ring positions not used by the mode message.
 */
internal fun placeDataBits(
    modules: Array<BooleanArray>,
    reserved: Array<BooleanArray>,
    bits: IntArray,
    cx: Int,
    cy: Int,
    compact: Boolean,
    layers: Int,
    modeRingRemainingPositions: List<Pair<Int, Int>>,
) {
    val size = modules.size
    var bitIndex = 0

    fun placeBit(col: Int, row: Int) {
        if (row < 0 || row >= size || col < 0 || col >= size) return
        if (!reserved[row][col]) {
            modules[row][col] = (bits.getOrElse(bitIndex) { 0 }) == 1
            bitIndex++
        }
    }

    // Fill the remaining mode ring positions first.
    for ((col, row) in modeRingRemainingPositions) {
        modules[row][col] = (bits.getOrElse(bitIndex) { 0 }) == 1
        bitIndex++
    }

    val br = bullseyeRadius(compact)
    val dStart = br + 2  // first data layer inner radius

    for (L in 0 until layers) {
        val dI = dStart + 2 * L  // inner radius of this layer
        val dO = dI + 1           // outer radius of this layer

        // Top edge: left to right (starts one column right of top-left)
        for (col in cx - dI + 1..cx + dI) {
            placeBit(col, cy - dO)
            placeBit(col, cy - dI)
        }
        // Right edge: top to bottom
        for (row in cy - dI + 1..cy + dI) {
            placeBit(cx + dO, row)
            placeBit(cx + dI, row)
        }
        // Bottom edge: right to left
        for (col in cx + dI downTo cx - dI + 1) {
            placeBit(col, cy + dO)
            placeBit(col, cy + dI)
        }
        // Left edge: bottom to top
        for (row in cy + dI downTo cy - dI + 1) {
            placeBit(cx - dO, row)
            placeBit(cx - dI, row)
        }
    }
}

// =============================================================================
// Public API
// =============================================================================

/**
 * Encode data as an Aztec Code symbol.
 *
 * Returns a [ModuleGrid] where `modules[row][col] == true` means a dark module.
 * The grid origin (0,0) is the top-left corner. All rows and the outer list are
 * immutable (backed by `Collections.unmodifiableList`).
 *
 * ## Pipeline
 *
 * 1. Encode input bytes via Binary-Shift from Upper mode.
 * 2. Select the smallest symbol at the requested ECC level.
 * 3. Pad to the exact data codeword count.
 * 4. Compute GF(256)/0x12D RS ECC.
 * 5. Apply bit stuffing.
 * 6. Compute GF(16) mode message.
 * 7. Initialise grid (bullseye → reference grid → orientation → mode msg).
 * 8. Place data+ECC bits in the clockwise layer spiral.
 * 9. Wrap mutable arrays as an immutable [ModuleGrid].
 *
 * @param data    Input string (UTF-8) or raw bytes.
 * @param options Encoding options (defaults: minEccPercent = 23).
 * @return        An immutable [ModuleGrid] of size N×N (N depends on layer count).
 * @throws AztecError.InputTooLong if the data exceeds the 32-layer full symbol capacity.
 *
 * ### Usage example
 *
 * ```kotlin
 * val grid = encode("Hello")
 * println("${grid.rows}×${grid.cols} symbol")
 *
 * val grid2 = encode("HELLO WORLD", AztecOptions(minEccPercent = 33))
 * ```
 */
fun encode(data: String, options: AztecOptions = AztecOptions()): ModuleGrid =
    encode(data.toByteArray(Charsets.UTF_8), options)

/**
 * Encode raw bytes as an Aztec Code symbol.
 *
 * This overload accepts a [ByteArray], useful for binary data (non-UTF-8).
 *
 * @see encode
 */
fun encode(data: ByteArray, options: AztecOptions = AztecOptions()): ModuleGrid {
    val minEccPct = options.minEccPercent

    // ---- Step 1: encode input bytes as a bit stream ----
    val dataBits = encodeBytesAsBits(data)

    // ---- Step 2: select symbol size ----
    val spec = selectSymbol(dataBits.size, minEccPct)
    val (compact, layers, dataCwCount, eccCwCount, _) = spec

    // ---- Step 3: pad to dataCwCount bytes ----
    val paddedBits = padToBytes(dataBits, dataCwCount)

    // Convert bit stream to byte array for RS processing.
    val dataBytes = IntArray(dataCwCount) { i ->
        var byte = 0
        for (b in 0 until 8) {
            byte = (byte shl 1) or (paddedBits[i * 8 + b])
        }
        // All-zero codeword avoidance: last codeword 0x00 → 0xFF.
        // The ISO standard requires the last codeword before RS to be non-zero
        // to avoid complications in the RS decoder.
        if (byte == 0 && i == dataCwCount - 1) byte = 0xff
        byte
    }

    // ---- Step 4: compute GF(256)/0x12D RS ECC ----
    val eccBytes = gf256RsEncode(dataBytes, eccCwCount)

    // ---- Step 5: build raw bit stream and apply stuffing ----
    val allBytes = dataBytes + eccBytes
    val rawBits = IntArray(allBytes.size * 8) { i ->
        (allBytes[i / 8] shr (7 - (i % 8))) and 1
    }
    val stuffedBits = stuffBits(rawBits)

    // ---- Step 6: mode message ----
    val modeMsg = encodeModeMessage(compact, layers, dataCwCount)

    // ---- Step 7: initialise grid ----
    val size = symbolSize(compact, layers)
    val cx = size / 2
    val cy = size / 2

    val modules = Array(size) { BooleanArray(size) }
    val reserved = Array(size) { BooleanArray(size) }

    // Reference grid first for full symbols (bullseye will overwrite intersection).
    if (!compact) {
        drawReferenceGrid(modules, reserved, cx, cy, size)
    }
    drawBullseye(modules, reserved, cx, cy, compact)

    val modeRingRemaining = drawOrientationAndModeMessage(
        modules, reserved, cx, cy, compact, modeMsg
    )

    // ---- Step 8: place data spiral ----
    placeDataBits(modules, reserved, stuffedBits, cx, cy, compact, layers, modeRingRemaining)

    // ---- Step 9: wrap as immutable ModuleGrid ----
    // Convert Array<BooleanArray> → List<List<Boolean>> (immutable view)
    val immutableRows: List<List<Boolean>> = modules.map { row ->
        java.util.Collections.unmodifiableList(row.map { it })
    }
    return ModuleGrid(
        rows = size,
        cols = size,
        modules = java.util.Collections.unmodifiableList(immutableRows),
        moduleShape = ModuleShape.SQUARE,
    )
}
