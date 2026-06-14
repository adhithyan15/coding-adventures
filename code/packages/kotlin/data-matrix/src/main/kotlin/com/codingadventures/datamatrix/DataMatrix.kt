/**
 * DataMatrix.kt — Data Matrix ECC 200 encoder, ISO/IEC 16022:2006 compliant.
 *
 * Data Matrix was invented by RVSI Acuity CiMatrix in 1989 under the name
 * "DataCode" and standardised as ISO/IEC 16022:2006.  The ECC 200 variant uses
 * Reed-Solomon error correction over GF(256) and is the dominant form worldwide.
 *
 * ## Where Data Matrix is used
 *
 * - **PCBs**: every printed-circuit board carries a Data Matrix for traceability
 *   through automated assembly lines.
 * - **Pharmaceuticals**: the US FDA DSCSA mandate requires Data Matrix on unit-dose
 *   packages for drug traceability.
 * - **Aerospace**: etched / dot-peened marks on rivets, shims, and brackets survive
 *   decades of heat and abrasion that would destroy ink-printed labels.
 * - **Medical devices**: GS1 DataMatrix on surgical instruments and implants.
 * - **USPS** registered mail and customs forms.
 *
 * ## Key differences from QR Code
 *
 * | Feature                  | QR Code        | Data Matrix      |
 * |--------------------------|----------------|------------------|
 * | Primitive polynomial     | 0x11D          | **0x12D**        |
 * | Reed-Solomon roots       | α^0 … α^{n-1}  | **α^1 … α^n**    |
 * | Finder pattern           | 3 corner squares | **L-bar + clock**|
 * | Data placement           | column zigzag  | **Utah diagonal**|
 * | Masking                  | 8 mask patterns | **No masking**   |
 *
 * ## Encoding pipeline
 *
 * ```
 * input string
 *   → ASCII encoding      (chars+1; digit pairs packed into one codeword)
 *   → symbol selection    (smallest symbol whose capacity ≥ codeword count)
 *   → pad to capacity     (scrambled-pad codewords fill unused slots)
 *   → RS blocks + ECC     (GF(256)/0x12D, b=1 convention, gen poly per block size)
 *   → interleave blocks   (data round-robin then ECC round-robin)
 *   → grid init           (L-finder + timing border + alignment borders)
 *   → Utah placement      (diagonal codeword placement, NO masking)
 *   → Array<BooleanArray> (physical symbol grid, true = dark module)
 * ```
 *
 * ## Usage
 *
 * ```kotlin
 * val grid = DataMatrix.encode("Hello World")   // Array<BooleanArray>
 * assert(grid.size == 16)                        // 16×16 for 11 ASCII chars
 * assert(grid[0].size == 16)
 * // grid[r][c] == true means dark module at row r, column c
 * ```
 *
 * Spec: code/specs/data-matrix.md
 * Reference implementations: code/packages/typescript/data-matrix/ and
 *                             code/packages/go/data-matrix/
 */
package com.codingadventures.datamatrix

// ============================================================================
// Version
// ============================================================================

/** Package version string.  Follows Semantic Versioning 2.0. */
const val VERSION = "0.1.0"

// ============================================================================
// Error types
// ============================================================================

/**
 * Thrown when the encoded data exceeds the capacity of the largest Data Matrix
 * symbol (144×144, 1558 data codewords).
 *
 * @property encodedCW  Number of codewords the input encodes to.
 * @property maxCW      Maximum data capacity of the 144×144 symbol.
 */
class InputTooLongException(val encodedCW: Int, val maxCW: Int = 1558) :
    Exception(
        "data-matrix: input too long — encoded $encodedCW codewords, " +
            "maximum is $maxCW (144×144 symbol)"
    )

// ============================================================================
// Public options / types
// ============================================================================

/**
 * Controls which symbol shapes are considered during auto-selection.
 *
 * - [SQUARE]       — square symbols only (10×10 … 144×144).  Default.
 * - [RECTANGULAR]  — rectangular symbols only (8×18 … 16×48).
 * - [ANY]          — both; picks the smallest regardless of shape.
 */
enum class SymbolShape { SQUARE, RECTANGULAR, ANY }

// ============================================================================
// GF(256) over 0x12D — Data Matrix's Galois field
// ============================================================================
//
// Data Matrix uses GF(256) with primitive polynomial 0x12D:
//
//   p(x) = x^8 + x^5 + x^4 + x^2 + x + 1   =   0x12D   =   301
//
// IMPORTANT: this is DIFFERENT from QR Code's 0x11D polynomial.
// Both are degree-8 irreducible polynomials over GF(2), but the fields are
// non-isomorphic — never mix the tables.
//
// We pre-compute two tables at class-load time using the recurrence:
//   α^0 = 1
//   α^i = α^{i-1} × 2 (left-shift 1 bit), then if bit 8 is set XOR 0x12D
//
// These two 256-byte tables turn all GF(256) multiply / divide operations into
// three O(1) table lookups, which is essential for the per-codeword RS inner loop.

private const val GF_POLY = 0x12D   // Data Matrix primitive polynomial

// Build GF(256)/0x12D exp and log tables at module initialisation time.
//
// Algorithm:
//   Start with val = 1 (= α^0).
//   Each step: multiply by α = x, i.e. left-shift 1 bit.
//   If the result overflows 8 bits (bit 8 set), reduce by XOR-ing 0x12D.
//
// After 255 steps we cover all non-zero GF(256) elements exactly once,
// confirming α = 2 is a primitive element of this field.
private val _gfTables: Pair<IntArray, IntArray> = run {
    val exp = IntArray(256)
    val log = IntArray(256)
    var v = 1
    for (i in 0 until 255) {
        exp[i] = v
        log[v] = i
        v = v shl 1
        if (v and 0x100 != 0) v = v xor GF_POLY
    }
    exp[255] = exp[0]   // α^255 = α^0 = 1  (multiplicative order = 255)
    Pair(exp, log)
}

/** gfExp[i] = α^i mod 0x12D  (index 0..255; gfExp[255] wraps back to 1) */
internal val GF_EXP: IntArray = _gfTables.first

/** gfLog[v] = k such that α^k = v  (for v = 1..255; gfLog[0] = 0 sentinel, unused) */
internal val GF_LOG: IntArray = _gfTables.second

/**
 * Multiply two GF(256)/0x12D elements using log/antilog tables.
 *
 * For a, b ≠ 0:   a × b = α^{(log[a] + log[b]) mod 255}
 * If either operand is 0, the product is 0 — zero is absorbing.
 *
 * Example:
 *   gfMul(2, 2) == 4        (α^1 × α^1 = α^2 = 4)
 *   gfMul(0x80, 2) == 0x2D  (α^7 × α^1 = α^8 = 0x2D)
 */
internal fun gfMul(a: Int, b: Int): Int {
    if (a == 0 || b == 0) return 0
    return GF_EXP[(GF_LOG[a] + GF_LOG[b]) % 255]
}

// ============================================================================
// Symbol size table
// ============================================================================

/**
 * Descriptor for one Data Matrix ECC 200 symbol size.
 *
 * A "data region" is a rectangular interior sub-area.  Small symbols (≤ 26×26)
 * have a single 1×1 region.  Larger symbols are subdivided into a grid of
 * regions separated by 2-module-wide alignment borders.
 *
 * The Utah placement algorithm works on the **logical data matrix** — all region
 * interiors concatenated into one flat grid — then maps back to physical coords.
 *
 * @param symbolRows        Total rows including outer border.
 * @param symbolCols        Total cols including outer border.
 * @param regionRows        Number of data-region rows (rr).
 * @param regionCols        Number of data-region cols (rc).
 * @param dataRegionHeight  Interior data height per region.
 * @param dataRegionWidth   Interior data width per region.
 * @param dataCW            Total data codeword capacity.
 * @param eccCW             Total ECC codeword count.
 * @param numBlocks         Number of interleaved RS blocks.
 * @param eccPerBlock       ECC codewords per block (equal across all blocks).
 */
internal data class SymbolEntry(
    val symbolRows: Int,
    val symbolCols: Int,
    val regionRows: Int,
    val regionCols: Int,
    val dataRegionHeight: Int,
    val dataRegionWidth: Int,
    val dataCW: Int,
    val eccCW: Int,
    val numBlocks: Int,
    val eccPerBlock: Int,
)

/**
 * All 24 square symbol sizes for Data Matrix ECC 200.
 *
 * Source: ISO/IEC 16022:2006, Table 7 (square symbols).
 * Ordered by ascending capacity (10×10 first, 144×144 last).
 *
 * Column order: symbolRows, symbolCols, regionRows, regionCols,
 *               dataRegionHeight, dataRegionWidth, dataCW, eccCW,
 *               numBlocks, eccPerBlock
 */
internal val SQUARE_SIZES: List<SymbolEntry> = listOf(
    SymbolEntry(10,  10,  1, 1, 8,  8,  3,    5,  1,  5),
    SymbolEntry(12,  12,  1, 1, 10, 10, 5,    7,  1,  7),
    SymbolEntry(14,  14,  1, 1, 12, 12, 8,    10, 1, 10),
    SymbolEntry(16,  16,  1, 1, 14, 14, 12,   12, 1, 12),
    SymbolEntry(18,  18,  1, 1, 16, 16, 18,   14, 1, 14),
    SymbolEntry(20,  20,  1, 1, 18, 18, 22,   18, 1, 18),
    SymbolEntry(22,  22,  1, 1, 20, 20, 30,   20, 1, 20),
    SymbolEntry(24,  24,  1, 1, 22, 22, 36,   24, 1, 24),
    SymbolEntry(26,  26,  1, 1, 24, 24, 44,   28, 1, 28),
    SymbolEntry(32,  32,  2, 2, 14, 14, 62,   36, 2, 18),
    SymbolEntry(36,  36,  2, 2, 16, 16, 86,   42, 2, 21),
    SymbolEntry(40,  40,  2, 2, 18, 18, 114,  48, 2, 24),
    SymbolEntry(44,  44,  2, 2, 20, 20, 144,  56, 4, 14),
    SymbolEntry(48,  48,  2, 2, 22, 22, 174,  68, 4, 17),
    SymbolEntry(52,  52,  2, 2, 24, 24, 204,  84, 4, 21),
    SymbolEntry(64,  64,  4, 4, 14, 14, 280, 112, 4, 28),
    SymbolEntry(72,  72,  4, 4, 16, 16, 368, 144, 4, 36),
    SymbolEntry(80,  80,  4, 4, 18, 18, 456, 192, 4, 48),
    SymbolEntry(88,  88,  4, 4, 20, 20, 576, 224, 4, 56),
    SymbolEntry(96,  96,  4, 4, 22, 22, 696, 272, 4, 68),
    SymbolEntry(104, 104, 4, 4, 24, 24, 816, 336, 6, 56),
    SymbolEntry(120, 120, 6, 6, 18, 18,1050, 408, 6, 68),
    SymbolEntry(132, 132, 6, 6, 20, 20,1304, 496, 8, 62),
    SymbolEntry(144, 144, 6, 6, 22, 22,1558, 620,10, 62),
)

/**
 * All 6 rectangular symbol sizes for Data Matrix ECC 200.
 *
 * Source: ISO/IEC 16022:2006, Table 7 (rectangular symbols).
 * Rectangular symbols are used when print-area aspect ratio matters (e.g.
 * on a narrow cylindrical object or a thin label strip).
 */
internal val RECT_SIZES: List<SymbolEntry> = listOf(
    SymbolEntry( 8, 18, 1, 1,  6, 16,  5,  7, 1,  7),
    SymbolEntry( 8, 32, 1, 2,  6, 14, 10, 11, 1, 11),
    SymbolEntry(12, 26, 1, 1, 10, 24, 16, 14, 1, 14),
    SymbolEntry(12, 36, 1, 2, 10, 16, 22, 18, 1, 18),
    SymbolEntry(16, 36, 1, 2, 14, 16, 32, 24, 1, 24),
    SymbolEntry(16, 48, 1, 2, 14, 22, 49, 28, 1, 28),
)

// ============================================================================
// Reed-Solomon generator polynomials (GF(256)/0x12D, b=1 convention)
// ============================================================================
//
// The RS generator for nEcc ECC bytes is:
//
//   g(x) = (x + α^1)(x + α^2) ··· (x + α^{nEcc})
//
// This is the b=1 convention used by Data Matrix (and Aztec Code), which
// differs from QR Code's b=0 roots.  All coefficients live in GF(256)/0x12D.
//
// Format: coefficient array, highest degree first, length = nEcc + 1.
// Example for nEcc = 5:  g(x) = x^5 + a·x^4 + ... + e  → [1, a, b, c, d, e]

/** Cache of built generator polynomials, keyed by nEcc. */
private val GEN_POLY_CACHE = HashMap<Int, IntArray>()

/**
 * Build the RS generator polynomial for [nEcc] ECC bytes.
 *
 * Algorithm: start with g = [1], then for i = 1 … nEcc multiply g by the
 * linear factor (x + α^i).
 *
 * Multiplying g(x) by (x + α^i) means:
 *   - newG[j]   ^= g[j]                 (the x coefficient)
 *   - newG[j+1] ^= gfMul(g[j], α^i)     (the constant coefficient)
 */
private fun buildGenerator(nEcc: Int): IntArray {
    var g = intArrayOf(1)
    for (i in 1..nEcc) {
        val ai = GF_EXP[i]   // α^i
        val next = IntArray(g.size + 1)
        for (j in g.indices) {
            next[j] = next[j] xor g[j]
            next[j + 1] = next[j + 1] xor gfMul(g[j], ai)
        }
        g = next
    }
    return g
}

/** Return (and cache) the generator polynomial for [nEcc] ECC bytes. */
private fun getGenerator(nEcc: Int): IntArray =
    GEN_POLY_CACHE.getOrPut(nEcc) { buildGenerator(nEcc) }

// Pre-build all generators needed for the symbol size tables at class-load time.
// This avoids any first-use latency during encoding.
private val _genInit: Unit = run {
    val seen = HashSet<Int>()
    for (e in SQUARE_SIZES + RECT_SIZES) {
        if (seen.add(e.eccPerBlock)) getGenerator(e.eccPerBlock)
    }
}

// ============================================================================
// Reed-Solomon encoding
// ============================================================================

/**
 * Compute [nEcc] ECC bytes for one data block using LFSR polynomial division.
 *
 * Algorithm: R(x) = D(x) × x^{nEcc} mod G(x)
 *
 * Implemented as a shift-register (LFSR):
 *
 * ```
 * for each data byte d:
 *     feedback = d XOR rem[0]
 *     shift rem left: rem[i] ← rem[i+1]
 *     rem[i] ^= gen[i+1] × feedback   for i = 0..nEcc-1
 * ```
 *
 * This is the standard systematic RS encoding approach used by QR Code,
 * Data Matrix, Aztec Code, and many other formats.
 *
 * @param data       Input data bytes for this block.
 * @param generator  Generator polynomial (length nEcc+1, highest degree first).
 * @return           ECC bytes (length = nEcc).
 */
private fun rsEncodeBlock(data: IntArray, generator: IntArray): IntArray {
    val nEcc = generator.size - 1
    val rem = IntArray(nEcc)
    for (d in data) {
        val fb = d xor rem[0]
        // Shift register left: rem[i] ← rem[i+1]
        for (i in 0 until nEcc - 1) rem[i] = rem[i + 1]
        rem[nEcc - 1] = 0
        if (fb != 0) {
            for (i in 0 until nEcc) {
                rem[i] = rem[i] xor gfMul(generator[i + 1], fb)
            }
        }
    }
    return rem
}

// ============================================================================
// ASCII data encoding
// ============================================================================

/**
 * Encode [input] bytes in Data Matrix ASCII mode.
 *
 * ASCII mode encodes each character or digit-pair into a single codeword:
 *
 * | Input                          | Codeword value                    |
 * |--------------------------------|-----------------------------------|
 * | Two consecutive digits 0–9     | 130 + (d1 × 10 + d2)  (130–229)  |
 * | Single ASCII char 0–127        | ASCII_value + 1       (1–128)     |
 * | Extended ASCII 128–255         | 235, then value − 127             |
 *
 * The digit-pair optimization is critical for lot codes and serial numbers:
 * "1234" → [142, 174]  (2 codewords, not 4).
 *
 * Examples:
 * ```
 * encodeAscii("A".toByteArray())    → [66]        (65 + 1)
 * encodeAscii(" ".toByteArray())    → [33]        (32 + 1)
 * encodeAscii("12".toByteArray())   → [142]       (130 + 12)
 * encodeAscii("1234".toByteArray()) → [142, 174]  (2 digit pairs)
 * encodeAscii("1A".toByteArray())   → [50, 66]    (digit then letter — no pair)
 * encodeAscii("00".toByteArray())   → [130]       (130 + 0)
 * encodeAscii("99".toByteArray())   → [229]       (130 + 99)
 * ```
 */
internal fun encodeAscii(input: ByteArray): IntArray {
    val codewords = ArrayList<Int>(input.size)
    var i = 0
    while (i < input.size) {
        val c = input[i].toInt() and 0xFF
        // Digit-pair check: both current and next bytes are ASCII digits 0x30–0x39
        if (c in 0x30..0x39 && i + 1 < input.size) {
            val next = input[i + 1].toInt() and 0xFF
            if (next in 0x30..0x39) {
                val d1 = c - 0x30       // first digit value (0–9)
                val d2 = next - 0x30    // second digit value (0–9)
                codewords.add(130 + d1 * 10 + d2)
                i += 2
                continue
            }
        }
        if (c <= 127) {
            // Standard single ASCII character: value + 1
            codewords.add(c + 1)
        } else {
            // Extended ASCII (128–255): UPPER_SHIFT (235) then value − 127
            codewords.add(235)
            codewords.add(c - 127)
        }
        i++
    }
    return codewords.toIntArray()
}

// ============================================================================
// Pad codewords (ISO/IEC 16022:2006 §5.2.3)
// ============================================================================

/**
 * Pad encoded codewords to exactly [dataCW] values.
 *
 * Padding rules (ISO/IEC 16022:2006 §5.2.3):
 *   1. The **first** pad codeword is always the literal value **129**.
 *   2. **Subsequent** pads use a scrambled formula based on their 1-indexed
 *      position k in the full codeword stream:
 *      ```
 *      scrambled = 129 + (149 × k mod 253) + 1
 *      if scrambled > 254: scrambled -= 254
 *      ```
 *
 * The scrambling prevents a run of "129 129 129 …" from creating a degenerate
 * placement pattern in the Utah algorithm — identical codewords would place
 * identical bit patterns at regular intervals, biasing the error-correction
 * structure.
 *
 * Example for "A" (codeword [66]) in a 10×10 symbol (dataCW = 3):
 * ```
 * k=2: 129                               ← first pad, always literal
 * k=3: 129 + (149×3 mod 253) + 1 = 324; 324 > 254 → 70
 * Result: [66, 129, 70]
 * ```
 */
internal fun padCodewords(codewords: IntArray, dataCW: Int): IntArray {
    val padded = IntArray(dataCW)
    codewords.copyInto(padded)

    var k = codewords.size + 1   // 1-indexed position of first pad byte
    var isFirst = true
    var pos = codewords.size
    while (pos < dataCW) {
        if (isFirst) {
            padded[pos] = 129
            isFirst = false
        } else {
            var scrambled = 129 + (149 * k % 253) + 1
            if (scrambled > 254) scrambled -= 254
            padded[pos] = scrambled
        }
        k++
        pos++
    }
    return padded
}

// ============================================================================
// Symbol selection
// ============================================================================

/**
 * Select the smallest symbol whose [SymbolEntry.dataCW] capacity fits
 * [codewordCount], respecting the requested [shape] preference.
 *
 * Iterates candidates in ascending capacity order and returns the first match.
 * Throws [InputTooLongException] if no symbol is large enough.
 */
internal fun selectSymbol(codewordCount: Int, shape: SymbolShape): SymbolEntry {
    val candidates: List<SymbolEntry> = when (shape) {
        SymbolShape.SQUARE       -> SQUARE_SIZES
        SymbolShape.RECTANGULAR  -> RECT_SIZES
        SymbolShape.ANY          -> (SQUARE_SIZES + RECT_SIZES)
            .sortedWith(compareBy({ it.dataCW }, { it.symbolRows * it.symbolCols }))
    }
    return candidates.firstOrNull { it.dataCW >= codewordCount }
        ?: throw InputTooLongException(codewordCount)
}

// ============================================================================
// Block splitting, ECC computation, and interleaving
// ============================================================================

/**
 * Split [data] across RS blocks, compute ECC for each, then interleave
 * data and ECC round-robin for placement.
 *
 * **Block splitting** (ISO interleaving convention):
 * ```
 * baseLen     = dataCW / numBlocks
 * extraBlocks = dataCW mod numBlocks
 * Blocks 0 .. extraBlocks-1 get baseLen+1 codewords (rounded up).
 * Blocks extraBlocks .. numBlocks-1 get baseLen codewords.
 * ```
 *
 * **Interleaving** distributes burst errors across all blocks, so a physical
 * scratch destroying N contiguous modules damages at most ⌈N/numBlocks⌉
 * codewords per block — well within each block's correction capacity.
 *
 * **Output order**:
 * ```
 * data round-robin: for pos in 0..maxDataPerBlock: for blk: append data[blk][pos]
 * ECC  round-robin: for pos in 0..eccPerBlock:     for blk: append ecc[blk][pos]
 * ```
 */
internal fun computeInterleaved(data: IntArray, entry: SymbolEntry): IntArray {
    val numBlocks   = entry.numBlocks
    val eccPerBlock = entry.eccPerBlock
    val dataCW      = entry.dataCW
    val gen         = getGenerator(eccPerBlock)

    // Split data into blocks
    val baseLen     = dataCW / numBlocks
    val extraBlocks = dataCW % numBlocks

    val dataBlocks = Array(numBlocks) { IntArray(0) }
    var offset = 0
    for (b in 0 until numBlocks) {
        val len = if (b < extraBlocks) baseLen + 1 else baseLen
        dataBlocks[b] = data.copyOfRange(offset, offset + len)
        offset += len
    }

    // Compute ECC for each block independently
    val eccBlocks = Array(numBlocks) { b -> rsEncodeBlock(dataBlocks[b], gen) }

    // Interleave data round-robin
    val total = dataCW + numBlocks * eccPerBlock
    val interleaved = IntArray(total)
    val maxDataLen = dataBlocks.maxOf { it.size }
    var outIdx = 0

    for (pos in 0 until maxDataLen) {
        for (b in 0 until numBlocks) {
            if (pos < dataBlocks[b].size) {
                interleaved[outIdx++] = dataBlocks[b][pos]
            }
        }
    }

    // Interleave ECC round-robin
    for (pos in 0 until eccPerBlock) {
        for (b in 0 until numBlocks) {
            interleaved[outIdx++] = eccBlocks[b][pos]
        }
    }

    return interleaved
}

// ============================================================================
// Grid initialization (finder border + alignment borders)
// ============================================================================

/**
 * Allocate and fill the physical module grid with the fixed structural elements.
 *
 * **Finder + clock border** (the outermost ring):
 *
 * ```
 * Top row    (row 0):      alternating dark/light starting dark at col 0   ← timing clock
 * Right col  (col C-1):    alternating dark/light starting dark at row 0   ← timing clock
 * Left col   (col 0):      all dark   ← L-finder left leg
 * Bottom row (row R-1):    all dark   ← L-finder bottom leg  (wins all conflicts)
 * ```
 *
 * The L-shaped solid-dark bar (left+bottom) is asymmetric, which lets a scanner
 * distinguish all four 90° rotations.  The alternating timing rows/cols tell the
 * scanner the module pitch so it can compensate for slight distortion.
 *
 * **Alignment borders** (multi-region symbols, e.g. 32×32 = 2×2 regions):
 *
 * Each alignment border is 2 modules wide and uses the same visual language:
 * ```
 * Row/Col AB+0: all dark
 * Row/Col AB+1: alternating dark/light starting dark
 * ```
 *
 * **Writing order** (later writes win at intersections):
 * 1. Alignment borders
 * 2. Top-row timing  (overrides AB at top-row intersections)
 * 3. Right-column timing  (overrides AB at right-col intersections)
 * 4. Left-column L-bar  (overrides timing at (0,0))
 * 5. Bottom-row L-bar  (written last — highest precedence, wins everywhere)
 */
internal fun initGrid(entry: SymbolEntry): Array<BooleanArray> {
    val R = entry.symbolRows
    val C = entry.symbolCols
    val grid = Array(R) { BooleanArray(C) }   // all false = light

    // ── 1. Alignment borders (multi-region symbols only) ──────────────────────
    // Written first so the outer borders can override at intersections.
    for (rr in 0 until entry.regionRows - 1) {
        // Physical row of the first AB row after data region rr+1:
        //   outer border (1) + (rr+1) × dataRegionHeight + rr × 2 (previous ABs)
        val abRow0 = 1 + (rr + 1) * entry.dataRegionHeight + rr * 2
        val abRow1 = abRow0 + 1
        for (c in 0 until C) {
            grid[abRow0][c] = true         // all dark
            grid[abRow1][c] = (c % 2 == 0) // alternating, starts dark
        }
    }
    for (rc in 0 until entry.regionCols - 1) {
        val abCol0 = 1 + (rc + 1) * entry.dataRegionWidth + rc * 2
        val abCol1 = abCol0 + 1
        for (r in 0 until R) {
            grid[r][abCol0] = true         // all dark
            grid[r][abCol1] = (r % 2 == 0) // alternating, starts dark
        }
    }

    // ── 2. Top row: timing clock — alternating dark/light, starts dark ────────
    for (c in 0 until C) grid[0][c] = (c % 2 == 0)

    // ── 3. Right column: timing clock — alternating, starts dark ─────────────
    for (r in 0 until R) grid[r][C - 1] = (r % 2 == 0)

    // ── 4. Left column: L-finder left leg — all dark ──────────────────────────
    // Written after timing to override timing at (0,0).
    for (r in 0 until R) grid[r][0] = true

    // ── 5. Bottom row: L-finder bottom leg — all dark ─────────────────────────
    // Written LAST: overrides alignment borders, right-column timing, everything.
    for (c in 0 until C) grid[R - 1][c] = true

    return grid
}

// ============================================================================
// Utah placement algorithm — boundary wrapping
// ============================================================================

/**
 * Apply the boundary wrap rules from ISO/IEC 16022:2006, Annex F.
 *
 * When the standard Utah shape extends beyond the logical grid edge, these rules
 * fold the out-of-bounds coordinate back into the valid range.
 *
 * Four wrap rules (applied in order):
 * 1. row < 0 AND col == 0          → (1, 3)           top-left singularity
 * 2. row < 0 AND col == nCols      → (0, col-2)       past right edge at top
 * 3. row < 0                       → (row+nRows, col-4)  top → bottom, left shift
 * 4. col < 0                       → (row-4, col+nCols)  left → right, up shift
 *
 * @return Pair(row, col) after wrapping.
 */
private fun applyWrap(row: Int, col: Int, nRows: Int, nCols: Int): Pair<Int, Int> {
    if (row < 0 && col == 0)     return Pair(1, 3)
    if (row < 0 && col == nCols) return Pair(0, col - 2)
    if (row < 0)                 return Pair(row + nRows, col - 4)
    if (col < 0)                 return Pair(row - 4, col + nCols)
    return Pair(row, col)
}

// ============================================================================
// Utah placement algorithm — codeword and corner placement functions
// ============================================================================

/**
 * Place one codeword using the standard "Utah" 8-module pattern.
 *
 * The shape is named "Utah" because it resembles the US state: a rectangle with
 * the top-left corner missing.
 *
 * ```
 * col: c-2  c-1   c
 * row-2:  .   [1]  [2]
 * row-1: [3]  [4]  [5]
 * row  : [6]  [7]  [8]
 * ```
 *
 * Bit numbering: [8] = MSB (bit 7 of the byte), [1] = LSB (bit 0).
 * Placement order: MSB at (row, col), LSB at (row-2, col-1).
 *
 * @param cw    Codeword byte value (0–255).
 * @param row   Reference row in logical grid.
 * @param col   Reference col in logical grid.
 * @param nRows Logical grid height.
 * @param nCols Logical grid width.
 * @param grid  Logical grid to write into.
 * @param used  Tracks which cells have already been written.
 */
private fun placeUtah(
    cw: Int, row: Int, col: Int, nRows: Int, nCols: Int,
    grid: Array<BooleanArray>, used: Array<BooleanArray>,
) {
    // [rawRow, rawCol, bitShift (7=MSB, 0=LSB)]
    val placements = arrayOf(
        intArrayOf(row,     col,     7),   // bit 8 (MSB)
        intArrayOf(row,     col - 1, 6),   // bit 7
        intArrayOf(row,     col - 2, 5),   // bit 6
        intArrayOf(row - 1, col,     4),   // bit 5
        intArrayOf(row - 1, col - 1, 3),   // bit 4
        intArrayOf(row - 1, col - 2, 2),   // bit 3
        intArrayOf(row - 2, col,     1),   // bit 2
        intArrayOf(row - 2, col - 1, 0),   // bit 1 (LSB)
    )
    for ((r, c, bit) in placements.map { Triple(it[0], it[1], it[2]) }) {
        val (wr, wc) = applyWrap(r, c, nRows, nCols)
        if (wr in 0 until nRows && wc in 0 until nCols && !used[wr][wc]) {
            grid[wr][wc] = (cw shr bit) and 1 == 1
            used[wr][wc] = true
        }
    }
}

/**
 * Corner pattern 1 — triggered at the top-left boundary.
 *
 * Absolute positions within the logical grid (nRows × nCols):
 * ```
 * bit 8: (0,      nCols-2)
 * bit 7: (0,      nCols-1)
 * bit 6: (1,      0)
 * bit 5: (2,      0)
 * bit 4: (nRows-2, 0)
 * bit 3: (nRows-1, 0)
 * bit 2: (nRows-1, 1)
 * bit 1: (nRows-1, 2)
 * ```
 */
private fun placeCorner1(
    cw: Int, nRows: Int, nCols: Int,
    grid: Array<BooleanArray>, used: Array<BooleanArray>,
) {
    val positions = arrayOf(
        intArrayOf(0,        nCols - 2, 7),
        intArrayOf(0,        nCols - 1, 6),
        intArrayOf(1,        0,         5),
        intArrayOf(2,        0,         4),
        intArrayOf(nRows - 2, 0,        3),
        intArrayOf(nRows - 1, 0,        2),
        intArrayOf(nRows - 1, 1,        1),
        intArrayOf(nRows - 1, 2,        0),
    )
    for ((r, c, bit) in positions.map { Triple(it[0], it[1], it[2]) }) {
        if (r in 0 until nRows && c in 0 until nCols && !used[r][c]) {
            grid[r][c] = (cw shr bit) and 1 == 1
            used[r][c] = true
        }
    }
}

/**
 * Corner pattern 2 — triggered at the top-right boundary.
 *
 * ```
 * bit 8: (0,      nCols-2)
 * bit 7: (0,      nCols-1)
 * bit 6: (1,      nCols-1)
 * bit 5: (2,      nCols-1)
 * bit 4: (nRows-1, 0)
 * bit 3: (nRows-1, 1)
 * bit 2: (nRows-1, 2)
 * bit 1: (nRows-1, 3)
 * ```
 */
private fun placeCorner2(
    cw: Int, nRows: Int, nCols: Int,
    grid: Array<BooleanArray>, used: Array<BooleanArray>,
) {
    val positions = arrayOf(
        intArrayOf(0,        nCols - 2, 7),
        intArrayOf(0,        nCols - 1, 6),
        intArrayOf(1,        nCols - 1, 5),
        intArrayOf(2,        nCols - 1, 4),
        intArrayOf(nRows - 1, 0,        3),
        intArrayOf(nRows - 1, 1,        2),
        intArrayOf(nRows - 1, 2,        1),
        intArrayOf(nRows - 1, 3,        0),
    )
    for ((r, c, bit) in positions.map { Triple(it[0], it[1], it[2]) }) {
        if (r in 0 until nRows && c in 0 until nCols && !used[r][c]) {
            grid[r][c] = (cw shr bit) and 1 == 1
            used[r][c] = true
        }
    }
}

/**
 * Corner pattern 3 — triggered at the bottom-left boundary.
 *
 * ```
 * bit 8: (0,       nCols-1)
 * bit 7: (1,       0)
 * bit 6: (2,       0)
 * bit 5: (nRows-2, 0)
 * bit 4: (nRows-1, 0)
 * bit 3: (nRows-1, 1)
 * bit 2: (nRows-1, 2)
 * bit 1: (nRows-1, 3)
 * ```
 */
private fun placeCorner3(
    cw: Int, nRows: Int, nCols: Int,
    grid: Array<BooleanArray>, used: Array<BooleanArray>,
) {
    val positions = arrayOf(
        intArrayOf(0,        nCols - 1, 7),
        intArrayOf(1,        0,         6),
        intArrayOf(2,        0,         5),
        intArrayOf(nRows - 2, 0,        4),
        intArrayOf(nRows - 1, 0,        3),
        intArrayOf(nRows - 1, 1,        2),
        intArrayOf(nRows - 1, 2,        1),
        intArrayOf(nRows - 1, 3,        0),
    )
    for ((r, c, bit) in positions.map { Triple(it[0], it[1], it[2]) }) {
        if (r in 0 until nRows && c in 0 until nCols && !used[r][c]) {
            grid[r][c] = (cw shr bit) and 1 == 1
            used[r][c] = true
        }
    }
}

/**
 * Corner pattern 4 — right-edge wrap for odd-dimension matrices.
 *
 * Used when both nRows and nCols are odd (occurs in rectangular symbols).
 *
 * ```
 * bit 8: (nRows-3, nCols-1)
 * bit 7: (nRows-2, nCols-1)
 * bit 6: (nRows-1, nCols-3)
 * bit 5: (nRows-1, nCols-2)
 * bit 4: (nRows-1, nCols-1)
 * bit 3: (0,       0)
 * bit 2: (1,       0)
 * bit 1: (2,       0)
 * ```
 */
private fun placeCorner4(
    cw: Int, nRows: Int, nCols: Int,
    grid: Array<BooleanArray>, used: Array<BooleanArray>,
) {
    val positions = arrayOf(
        intArrayOf(nRows - 3, nCols - 1, 7),
        intArrayOf(nRows - 2, nCols - 1, 6),
        intArrayOf(nRows - 1, nCols - 3, 5),
        intArrayOf(nRows - 1, nCols - 2, 4),
        intArrayOf(nRows - 1, nCols - 1, 3),
        intArrayOf(0,         0,         2),
        intArrayOf(1,         0,         1),
        intArrayOf(2,         0,         0),
    )
    for ((r, c, bit) in positions.map { Triple(it[0], it[1], it[2]) }) {
        if (r in 0 until nRows && c in 0 until nCols && !used[r][c]) {
            grid[r][c] = (cw shr bit) and 1 == 1
            used[r][c] = true
        }
    }
}

// ============================================================================
// Utah placement — main loop
// ============================================================================

/**
 * Run the Utah diagonal placement algorithm on a logical grid of [nRows] × [nCols].
 *
 * ## Algorithm overview
 *
 * The reference position (row, col) starts at (4, 0) and zigzags diagonally
 * across the logical grid.  Each outer-loop iteration has two legs:
 *
 * 1. **Upward-right leg** (`row -= 2, col += 2`): place codewords until out of bounds.
 *    Step to next diagonal start: `row += 1, col += 3`.
 * 2. **Downward-left leg** (`row += 2, col -= 2`): place codewords until out of bounds.
 *    Step to next diagonal start: `row += 3, col += 1`.
 *
 * Between legs, four **corner patterns** fire when the reference matches specific
 * trigger conditions (one per corner type).
 *
 * ## Termination
 *
 * When `row >= nRows && col >= nCols` all modules have been visited.
 * Any unvisited modules at the end receive the ISO **right-and-bottom fill**:
 * `grid[r][c] = (r + c) mod 2 == 1` (dark at odd-sum positions).
 *
 * ## No masking
 *
 * Unlike QR Code, Data Matrix applies **no masking** after placement.
 * The diagonal traversal distributes bits naturally across the symbol.
 *
 * @param codewords  Full interleaved codeword stream (data + ECC).
 * @param nRows      Logical data matrix height.
 * @param nCols      Logical data matrix width.
 * @return           nRows × nCols boolean grid (true = dark module).
 */
internal fun utahPlacement(codewords: IntArray, nRows: Int, nCols: Int): Array<BooleanArray> {
    val grid = Array(nRows) { BooleanArray(nCols) }
    val used = Array(nRows) { BooleanArray(nCols) }

    var cwIdx = 0
    var row = 4
    var col = 0

    fun place(fn: (Int, Int, Int, Array<BooleanArray>, Array<BooleanArray>) -> Unit) {
        if (cwIdx < codewords.size) {
            fn(codewords[cwIdx], nRows, nCols, grid, used)
            cwIdx++
        }
    }

    while (true) {
        // ── Corner special cases ──────────────────────────────────────────────
        // Corner 1: reference at (nRows, 0) when nRows or nCols divisible by 4.
        if (row == nRows && col == 0 && (nRows % 4 == 0 || nCols % 4 == 0)) {
            place(::placeCorner1)
        }
        // Corner 2: reference at (nRows-2, 0) when nCols mod 4 ≠ 0.
        if (row == nRows - 2 && col == 0 && nCols % 4 != 0) {
            place(::placeCorner2)
        }
        // Corner 3: reference at (nRows-2, 0) when nCols mod 8 == 4.
        if (row == nRows - 2 && col == 0 && nCols % 8 == 4) {
            place(::placeCorner3)
        }
        // Corner 4: reference at (nRows+4, 2) when nCols mod 8 == 0.
        if (row == nRows + 4 && col == 2 && nCols % 8 == 0) {
            place(::placeCorner4)
        }

        // ── Upward-right diagonal leg (row -= 2, col += 2) ───────────────────
        do {
            if (row in 0 until nRows && col in 0 until nCols && !used[row][col]) {
                if (cwIdx < codewords.size) {
                    placeUtah(codewords[cwIdx], row, col, nRows, nCols, grid, used)
                    cwIdx++
                }
            }
            row -= 2
            col += 2
        } while (row >= 0 && col < nCols)

        // Step to next diagonal start
        row++
        col += 3

        // ── Downward-left diagonal leg (row += 2, col -= 2) ──────────────────
        do {
            if (row in 0 until nRows && col in 0 until nCols && !used[row][col]) {
                if (cwIdx < codewords.size) {
                    placeUtah(codewords[cwIdx], row, col, nRows, nCols, grid, used)
                    cwIdx++
                }
            }
            row += 2
            col -= 2
        } while (row < nRows && col >= 0)

        // Step to next diagonal start
        row += 3
        col++

        // ── Termination check ─────────────────────────────────────────────────
        if (row >= nRows && col >= nCols) break
        if (cwIdx >= codewords.size) break
    }

    // ── Fill remaining unset modules (ISO right-and-bottom fill rule) ─────────
    // Some symbol sizes have residual modules the diagonal walk does not reach.
    // ISO/IEC 16022 §10: these receive (r+c) mod 2 == 1 (dark).
    for (r in 0 until nRows) {
        for (c in 0 until nCols) {
            if (!used[r][c]) {
                grid[r][c] = (r + c) % 2 == 1
            }
        }
    }

    return grid
}

// ============================================================================
// Logical → Physical coordinate mapping
// ============================================================================

/**
 * Map a logical data matrix coordinate (r, c) to its physical symbol coordinate.
 *
 * The **logical data matrix** is the concatenation of all data region interiors,
 * treated as one flat grid.  The Utah algorithm works in this logical space.
 * After placement we map back to the **physical grid**, which includes:
 * - 1-module outer border (finder + timing) on all four sides
 * - 2-module alignment borders between data regions
 *
 * For a symbol with regionRows × regionCols data regions, each of size
 * (dataRegionHeight × dataRegionWidth):
 *
 * ```
 * physRow = floor(r / rh) × (rh + 2) + (r mod rh) + 1
 * physCol = floor(c / rw) × (rw + 2) + (c mod rw) + 1
 * ```
 *
 * The `+2` accounts for the 2-module alignment border between regions.
 * The `+1` accounts for the 1-module outer border.
 *
 * For single-region symbols (1×1) this simplifies to physRow = r+1, physCol = c+1.
 *
 * @return Pair(physRow, physCol)
 */
private fun logicalToPhysical(r: Int, c: Int, entry: SymbolEntry): Pair<Int, Int> {
    val rh = entry.dataRegionHeight
    val rw = entry.dataRegionWidth
    val physRow = (r / rh) * (rh + 2) + (r % rh) + 1
    val physCol = (c / rw) * (rw + 2) + (c % rw) + 1
    return Pair(physRow, physCol)
}

// ============================================================================
// Public API — DataMatrix object
// ============================================================================

/**
 * Data Matrix ECC 200 encoder.
 *
 * Provides a single entry point [encode] that accepts a [String] input and
 * returns a physical module grid as `Array<BooleanArray>`.
 *
 * `grid[r][c] == true` means a **dark** module at row r, column c.
 * The grid is in scanner orientation: row 0 is the top (timing row),
 * column 0 is the left edge (L-finder left leg).
 */
object DataMatrix {

    /**
     * Encode [data] into a Data Matrix ECC 200 symbol.
     *
     * The smallest symbol that can hold the encoded data is selected
     * automatically from the square symbol sizes (10×10 … 144×144).
     * Pass [shape] = [SymbolShape.RECTANGULAR] to prefer rectangular symbols,
     * or [SymbolShape.ANY] to try both and pick the smallest.
     *
     * The encoding pipeline:
     * 1. ASCII encode (with digit-pair optimization)
     * 2. Select smallest fitting symbol
     * 3. Pad to data capacity with scrambled pad codewords
     * 4. Compute RS ECC for each block over GF(256)/0x12D
     * 5. Interleave data + ECC blocks round-robin
     * 6. Initialize physical grid (L-finder + timing + alignment borders)
     * 7. Run Utah diagonal placement on logical data matrix
     * 8. Map logical coords to physical coords
     * 9. Return physical module grid (no masking — Data Matrix never masks)
     *
     * @param data    Input string (all characters 0–127 for efficient encoding;
     *                characters 128–255 use UPPER_SHIFT and consume 2 codewords).
     * @param shape   Symbol shape preference.  Default: [SymbolShape.SQUARE].
     * @return        `Array<BooleanArray>` of size `symbolRows × symbolCols`.
     *                `grid[r][c] == true` means dark module.
     * @throws InputTooLongException if the input exceeds 144×144 capacity.
     *
     * @sample
     * ```kotlin
     * val grid = DataMatrix.encode("Hello World")
     * assert(grid.size == 16)      // 16×16 for "Hello World"
     * assert(grid[0].size == 16)
     * // L-finder: left column all dark
     * assert(grid.all { row -> row[0] })
     * // L-finder: bottom row all dark
     * assert(grid.last().all { it })
     * ```
     */
    fun encode(data: String, shape: SymbolShape = SymbolShape.SQUARE): Array<BooleanArray> {
        val bytes = data.toByteArray(Charsets.ISO_8859_1)

        // Step 1: ASCII encode
        val codewords = encodeAscii(bytes)

        // Step 2: Select smallest fitting symbol
        val entry = selectSymbol(codewords.size, shape)

        // Step 3: Pad to data capacity
        val padded = padCodewords(codewords, entry.dataCW)

        // Steps 4–5: RS ECC + interleave
        val interleaved = computeInterleaved(padded, entry)

        // Step 6: Initialize physical grid
        val physGrid = initGrid(entry)

        // Steps 7–8: Utah placement + logical → physical mapping
        val nRows = entry.regionRows * entry.dataRegionHeight
        val nCols = entry.regionCols * entry.dataRegionWidth
        val logicalGrid = utahPlacement(interleaved, nRows, nCols)

        for (r in 0 until nRows) {
            for (c in 0 until nCols) {
                val (pr, pc) = logicalToPhysical(r, c, entry)
                physGrid[pr][pc] = logicalGrid[r][c]
            }
        }

        // Step 9: Return physical grid (no masking)
        return physGrid
    }
}

// ============================================================================
// Internal helpers exported for testing (package-internal visibility)
// ============================================================================

// Note: encodeAscii, padCodewords, selectSymbol, computeInterleaved,
// initGrid, and utahPlacement are all `internal` so test files in the same
// package can access them directly without exposing them as public API.
