/**
 * PDF417.kt — ISO/IEC 15438:2015-compliant PDF417 stacked linear barcode encoder for Kotlin.
 *
 * PDF417 (Portable Data File 417) was invented by Ynjiun P. Wang at Symbol
 * Technologies in 1991. The name encodes its geometry: each codeword has
 * exactly **4** bars and **4** spaces (8 elements), and every codeword
 * occupies exactly **17** modules of horizontal space. "417" = 4 × 17.
 *
 * ## Where PDF417 is deployed
 *
 * | Application    | Detail                                               |
 * |----------------|------------------------------------------------------|
 * | AAMVA          | North American driver's licences and government IDs  |
 * | IATA BCBP      | Airline boarding passes                              |
 * | USPS           | Domestic shipping labels                             |
 * | US immigration | Form I-94, customs declarations                      |
 * | Healthcare     | Patient wristbands, medication labels                |
 *
 * ## Encoding pipeline
 *
 * ```
 * raw bytes
 *   → byte compaction     (codeword 924 latch + 6-bytes-to-5-codewords base-900)
 *   → length descriptor   (codeword 0 = total codewords in symbol)
 *   → RS ECC              (GF(929) Reed-Solomon, b=3 convention, α=3)
 *   → dimension selection (auto: roughly square symbol)
 *   → padding             (codeword 900 fills unused slots)
 *   → row indicators      (LRI + RRI per row, encode R/C/ECC level)
 *   → cluster table lookup (codeword → 17-module bar/space pattern)
 *   → start/stop patterns (fixed per row)
 *   → ModuleGrid          (abstract boolean grid)
 * ```
 *
 * ## v0.1.0 scope
 *
 * This release implements **byte compaction only**. Text and numeric
 * compaction are planned for v0.2.0.
 *
 * ## GF(929) — Why a prime field?
 *
 * PDF417 uses Reed-Solomon over GF(929), not GF(256). The codeword alphabet
 * has exactly 929 elements (values 0–928). Since 929 is prime, GF(929) is
 * simply the integers modulo 929. No primitive polynomial needed — just
 * modular arithmetic.
 *
 * Generator (primitive root): α = 3. Verify: 3^928 ≡ 1 (mod 929) by
 * Fermat's little theorem.
 *
 * ## Three codeword clusters
 *
 * Each row uses one of three cluster tables (0, 3, or 6) cycling as
 * `row % 3`. This lets a scanner identify which row it is reading
 * without knowing the row number in advance — codeword patterns differ
 * by cluster.
 */
package com.codingadventures.pdf417

import com.codingadventures.barcode2d.ModuleGrid
import com.codingadventures.barcode2d.ModuleShape
import kotlin.math.ceil
import kotlin.math.sqrt

// ============================================================================
// Version
// ============================================================================

/** Package version string. Follows Semantic Versioning 2.0. */
const val VERSION = "0.1.0"

// ============================================================================
// Error types
// ============================================================================

/**
 * Base class for all PDF417 encoding errors.
 *
 * Using a sealed class hierarchy means the Kotlin compiler can exhaustively
 * check `when` expressions over [PDF417Error] subtypes — if a new error
 * variant is added here, the compiler flags every unhandled `when`.
 */
sealed class PDF417Error(message: String) : RuntimeException(message) {

    /**
     * The input data is too long to fit in any valid PDF417 symbol.
     *
     * A standard PDF417 symbol holds at most 90 rows × 30 data columns
     * = 2700 codewords. After subtracting ECC codewords (up to 512 at
     * level 8) and the length descriptor, the maximum data payload is
     * approximately 2187 byte-compacted codewords (≈ 2622 raw bytes for
     * full-groups of 6).
     *
     * @param msg Human-readable description of the overflow.
     */
    class InputTooLong(msg: String) : PDF417Error("InputTooLong: $msg")

    /**
     * The specified rows or columns are outside the valid range.
     *
     * Valid ranges:  rows 3–90,  columns 1–30.
     *
     * @param msg Human-readable description of the violation.
     */
    class InvalidDimensions(msg: String) : PDF417Error("InvalidDimensions: $msg")

    /**
     * The specified ECC level is outside the valid range 0–8.
     *
     * @param msg Human-readable description of the violation.
     */
    class InvalidECCLevel(msg: String) : PDF417Error("InvalidECCLevel: $msg")
}

// ============================================================================
// Options
// ============================================================================

/**
 * Configuration for the PDF417 encoder.
 *
 * All fields are optional. `null` means "auto-select":
 *   - [eccLevel]: auto-selected based on data length (see [autoEccLevel])
 *   - [columns]:  auto-selected for a roughly square symbol
 *   - [rowHeight]: defaults to 3 (minimum recommended for scan reliability)
 *
 * ### Example — defaults
 *
 * ```kotlin
 * val opts = PDF417Options()                    // all auto
 * val opts = PDF417Options(eccLevel = 4)        // force level 4, auto cols
 * val opts = PDF417Options(columns = 10)        // 10 data cols, auto ECC
 * val opts = PDF417Options(eccLevel = 2, columns = 5, rowHeight = 5)
 * ```
 *
 * @param eccLevel  Reed-Solomon ECC level (0–8).  `null` = auto-select.
 * @param columns   Number of data columns (1–30).  `null` = auto-select.
 * @param rowHeight Module-rows per logical PDF417 row (≥ 1).  Default: 3.
 */
data class PDF417Options(
    val eccLevel: Int? = null,
    val columns: Int? = null,
    val rowHeight: Int? = null,
)

// ============================================================================
// Constants
// ============================================================================

/**
 * GF(929) prime modulus.
 *
 * All arithmetic in the Reed-Solomon encoder is performed modulo 929.
 * 929 is prime, so GF(929) = ℤ/929ℤ — the integers modulo 929. No
 * primitive polynomial is needed; every non-zero element has a
 * multiplicative inverse by Fermat's little theorem.
 */
const val GF929_PRIME = 929

/**
 * Generator element α = 3 (primitive root mod 929, per ISO/IEC 15438 Annex A.4).
 *
 * A primitive root of GF(929) is an element whose powers cycle through all
 * 928 non-zero elements before returning to 1. 3 is the primitive root
 * chosen by the PDF417 standard. Verification: 3^928 ≡ 1 (mod 929), and
 * 3^k ≠ 1 for any k < 928.
 */
const val GF929_ALPHA = 3

/**
 * Multiplicative group order = PRIME − 1 = 928.
 *
 * In GF(p), the multiplicative group has order p − 1 by Fermat's little
 * theorem. All exponent arithmetic wraps modulo this value.
 */
const val GF929_ORDER = 928

/**
 * Latch-to-byte-compaction codeword (alternate form).
 *
 * Using 924 (not 901/902) because it works for both even and odd lengths
 * of remaining bytes and is the most universally compatible latch.
 */
const val LATCH_BYTE = 924

/**
 * Padding codeword (value 900 = "latch to text compaction").
 *
 * When the data codewords do not fill all grid slots exactly, 900 is used
 * as a harmless filler: it silently switches the decoder to text mode
 * without emitting output, so it is transparent in the data stream.
 */
const val PADDING_CW = 900

/** Minimum allowed rows in a PDF417 symbol (ISO/IEC 15438 §5.8). */
const val MIN_ROWS = 3

/** Maximum allowed rows in a PDF417 symbol (ISO/IEC 15438 §5.8). */
const val MAX_ROWS = 90

/** Minimum allowed data columns in a PDF417 symbol (ISO/IEC 15438 §5.8). */
const val MIN_COLS = 1

/** Maximum allowed data columns in a PDF417 symbol (ISO/IEC 15438 §5.8). */
const val MAX_COLS = 30

// ============================================================================
// GF(929) arithmetic
// ============================================================================
//
// GF(929) is the integers modulo 929. Since 929 is prime, every non-zero
// element has a multiplicative inverse. We use log/antilog lookup tables for
// O(1) multiplication, built once at class-load time.
//
// The tables take ~3.7 KB total (929 × 2 bytes × 2 arrays) and are built
// in a negligible fraction of a millisecond.

/**
 * GF_EXP[i] = α^i mod 929   (α = 3).
 *
 * Built once at class-load time. GF_EXP[928] = GF_EXP[0] = 1, for
 * wrap-around convenience when the sum of two logs equals 928.
 *
 * ### Construction
 *
 * ```
 * GF_EXP[0] = 1
 * GF_EXP[i] = GF_EXP[i-1] * 3 mod 929   for i = 1..927
 * GF_EXP[928] = 1   (wrap sentinel = GF_EXP[0])
 * ```
 */
private val GF_EXP: IntArray = IntArray(929).also { tbl ->
    var v = 1
    for (i in 0 until GF929_ORDER) {
        tbl[i] = v
        v = (v * GF929_ALPHA) % GF929_PRIME
    }
    // Wrap-around sentinel: GF_EXP[928] = GF_EXP[0] = 1.
    tbl[GF929_ORDER] = tbl[0]
}

/**
 * GF_LOG[v] = discrete logarithm base α of v, for v in 1..928.
 *
 * GF_LOG[0] is intentionally left as 0 (log of zero is undefined;
 * [gfMul] short-circuits before consulting GF_LOG for zero operands).
 *
 * ### Construction
 *
 * ```
 * for i = 0..927:
 *     GF_LOG[ GF_EXP[i] ] = i
 * ```
 */
private val GF_LOG: IntArray = IntArray(929).also { tbl ->
    var v = 1
    for (i in 0 until GF929_ORDER) {
        tbl[v] = i
        v = (v * GF929_ALPHA) % GF929_PRIME
    }
}

/**
 * GF(929) multiply using log/antilog tables. Returns 0 if either operand is 0.
 *
 * ### Identity used
 *
 * ```
 * a × b = α^(log(a) + log(b))   (mod 929)
 * ```
 *
 * The mod-928 wrap is required because α^928 = α^0 = 1 by Fermat's little
 * theorem: α^{p-1} ≡ 1 (mod p) for prime p and α not divisible by p.
 *
 * @param a First operand (0–928).
 * @param b Second operand (0–928).
 * @return Product a × b in GF(929).
 */
private fun gfMul(a: Int, b: Int): Int {
    if (a == 0 || b == 0) return 0
    return GF_EXP[(GF_LOG[a] + GF_LOG[b]) % GF929_ORDER]
}

/**
 * GF(929) add: (a + b) mod 929.
 *
 * Unlike GF(256) where addition = XOR (characteristic 2), GF(929) has
 * odd characteristic p = 929. Addition is ordinary integer addition
 * modulo the prime — no XOR tricks here.
 *
 * @param a First operand (0–928).
 * @param b Second operand (0–928).
 * @return Sum a + b in GF(929).
 */
private fun gfAdd(a: Int, b: Int): Int = (a + b) % GF929_PRIME

// ============================================================================
// Reed-Solomon generator polynomial
// ============================================================================
//
// For ECC level L, k = 2^(L+1) ECC codewords are appended to the data.
// The generator polynomial uses the b=3 convention (roots start at α^3):
//
//   g(x) = (x − α^3)(x − α^4) ··· (x − α^{k+2})
//
// We build g iteratively by multiplying in each linear factor (x − α^j).
// Note: −α^j in GF(929) = 929 − α^j (since (a + (929 − a)) mod 929 = 0).

/**
 * Build the RS generator polynomial for a given ECC level.
 *
 * Returns an array of k+1 int coefficients [g_k, g_{k-1}, ..., g_0]
 * where k = 2^(eccLevel+1) and g_k = 1 (monic polynomial).
 *
 * The polynomial is: g(x) = x^k + g_{k-1}·x^{k-1} + ... + g_0
 *
 * ### Building g iteratively
 *
 * Start with g(x) = [1] (the polynomial "1"). For each root j from 3 to k+2:
 *
 * ```
 * root    = α^j mod 929
 * negRoot = 929 - root       // additive inverse of root in GF(929)
 * g(x)   ← g(x) * (x + negRoot)
 * ```
 *
 * The multiply distributes as: each new coefficient is the sum of the
 * old coefficient at the same position plus (old coefficient one step up)
 * times negRoot.
 *
 * @param eccLevel ECC level (0–8).
 * @return Generator polynomial coefficients, highest degree first.
 */
private fun buildGenerator(eccLevel: Int): IntArray {
    // Number of ECC codewords = 2^(eccLevel+1).
    val k = 1 shl (eccLevel + 1)
    var g = intArrayOf(1)

    // Multiply g by (x − α^j) for each root j from 3 to k+2 inclusive.
    for (j in 3..k + 2) {
        // α^j mod 929
        val root = GF_EXP[j % GF929_ORDER]
        // Additive inverse of root in GF(929): negRoot = 929 - root.
        // Because (root + negRoot) mod 929 = 0.
        val negRoot = GF929_PRIME - root
        // New polynomial = old polynomial × (x + negRoot).
        // newG has degree one higher than g.
        val newG = IntArray(g.size + 1)
        for (i in g.indices) {
            newG[i]     = gfAdd(newG[i],     g[i])
            newG[i + 1] = gfAdd(newG[i + 1], gfMul(g[i], negRoot))
        }
        g = newG
    }
    return g
}

// ============================================================================
// Reed-Solomon encoder
// ============================================================================
//
// Given data codewords D = [d₀, d₁, ..., d_{n-1}] and generator g(x)
// of degree k, compute k ECC codewords by the LFSR shift-register method:
//
//   R(x) = D(x) × x^k mod g(x)
//
// This is the standard polynomial long-division algorithm, equivalent to
// feeding each data symbol through a k-stage feedback register.
//
// No interleaving: all data feeds a single RS encoder (simpler than QR Code).

/**
 * Compute `k` RS ECC codewords for [data] over GF(929) with b=3 convention.
 *
 * ### Algorithm (LFSR / shift-register)
 *
 * ```
 * ecc = [0, 0, ..., 0]   (k zeros)
 * for each data symbol d:
 *     feedback = d + ecc[0]
 *     shift ecc left (discard ecc[0], set ecc[k-1] = 0)
 *     for each position i:
 *         ecc[i] += g[k-i] * feedback
 * ```
 *
 * The g[k-i] indexing comes from the conventional polynomial encoding
 * formulation where g is stored highest-degree-first.
 *
 * @param data     Data codewords to protect (including length descriptor).
 * @param eccLevel ECC level determining how many ECC codewords to generate.
 * @return Array of `2^(eccLevel+1)` ECC codewords.
 */
private fun rsEncode(data: IntArray, eccLevel: Int): IntArray {
    val g = buildGenerator(eccLevel)
    val k = g.size - 1
    val ecc = IntArray(k)

    for (d in data) {
        val feedback = gfAdd(d, ecc[0])
        // Shift register left: discard ecc[0], slide others down by one.
        for (i in 0 until k - 1) {
            ecc[i] = ecc[i + 1]
        }
        ecc[k - 1] = 0
        // Add feedback × generator coefficient to each stage.
        // g[k - i] is the coefficient for position i after shifting.
        for (i in 0 until k) {
            ecc[i] = gfAdd(ecc[i], gfMul(g[k - i], feedback))
        }
    }
    return ecc
}

// ============================================================================
// Byte compaction
// ============================================================================
//
// Byte compaction converts raw bytes to PDF417 codewords (values 0–928):
//
//   1. Emit latch codeword 924.
//   2. For every full group of 6 bytes:
//      - Treat them as a 48-bit big-endian integer n.
//      - Express n in base 900 → exactly 5 codewords.
//      - This packs 6 bytes into 5 codewords (1.2 bytes/codeword).
//   3. For remaining 1–5 bytes: one codeword per byte (direct mapping).
//
// The 6→5 compression works because:
//   256^6 = 281,474,976,710,656  <  590,490,000,000,000 = 900^5
// So every 48-bit value fits in five base-900 digits.
//
// We use Long (64-bit signed) for the 48-bit arithmetic.
// 256^6 ≈ 2.81×10^14, which fits in Long (max ≈ 9.2×10^18).

/**
 * Encode raw bytes using byte compaction (codeword 924 latch).
 *
 * Returns `[924, c1, c2, ...]` where each c_i is a byte-compacted codeword.
 *
 * ### 6-byte groups → 5 codewords
 *
 * For bytes b0..b5 (big-endian order):
 * ```
 * n  = b0×256^5 + b1×256^4 + b2×256^3 + b3×256^2 + b4×256 + b5
 * c4 = n mod 900
 * c3 = (n / 900) mod 900
 * c2 = (n / 900²) mod 900
 * c1 = (n / 900³) mod 900
 * c0 = (n / 900⁴)
 * ```
 *
 * Emitted in big-endian order: c0, c1, c2, c3, c4.
 *
 * ### Remainder bytes (1–5)
 *
 * Bytes that don't form a complete group of 6 are emitted directly, one
 * codeword per byte (values 0–255). This is legal because codeword values
 * up to 928 are valid in PDF417.
 *
 * @param bytes Raw input bytes.
 * @return Codeword sequence starting with latch 924.
 */
private fun byteCompact(bytes: ByteArray): IntArray {
    val result = mutableListOf<Int>()
    result.add(LATCH_BYTE)

    val len = bytes.size
    var i = 0

    // Process full 6-byte groups → 5 codewords each.
    while (i + 6 <= len) {
        // Build the 48-bit big-endian integer from 6 bytes.
        var n = 0L
        for (j in 0..5) {
            // bytes[i+j] is a Kotlin Byte (signed); mask to unsigned 0–255.
            n = n * 256L + (bytes[i + j].toLong() and 0xFFL)
        }
        // Convert n to base 900 → 5 codewords, most-significant first.
        val group = IntArray(5)
        var nn = n
        for (j in 4 downTo 0) {
            group[j] = (nn % 900L).toInt()
            nn /= 900L
        }
        for (cw in group) result.add(cw)
        i += 6
    }

    // Remaining bytes (0–5): one codeword per byte (direct byte value).
    while (i < len) {
        result.add(bytes[i].toInt() and 0xFF)
        i++
    }

    return result.toIntArray()
}

// ============================================================================
// ECC level auto-selection
// ============================================================================
//
// The recommended minimum ECC level depends on the total number of data
// codewords (including the byte-compaction prefix). Higher data density
// benefits from more ECC redundancy.

/**
 * Select the minimum recommended ECC level based on data codeword count.
 *
 * From the ISO/IEC 15438 spec, recommended minimums:
 *
 * | Data codewords | Recommended ECC level | ECC codewords |
 * |---------------:|:----------------------:|:-------------:|
 * |     ≤ 40       |           2            |       8       |
 * |     ≤ 160      |           3            |      16       |
 * |     ≤ 320      |           4            |      32       |
 * |     ≤ 863      |           5            |      64       |
 * |      > 863     |           6            |     128       |
 *
 * Higher levels reduce scanner range and label lifetime requirements at
 * the cost of more physical space. For archival or high-damage applications,
 * consider levels 7 or 8.
 *
 * @param dataCount Total number of data codewords (including latch/length).
 * @return Recommended ECC level (2–6).
 */
fun autoEccLevel(dataCount: Int): Int = when {
    dataCount <= 40  -> 2
    dataCount <= 160 -> 3
    dataCount <= 320 -> 4
    dataCount <= 863 -> 5
    else             -> 6
}

// ============================================================================
// Dimension selection
// ============================================================================
//
// We need to choose rows (3–90) and data columns (1–30) such that
// rows × cols ≥ total_codewords.
//
// Heuristic: c = ceil(sqrt(total / 3)), clamped to 1–30.
// Then r = ceil(total / c), clamped to 3–90.
//
// The divisor of 3 approximates the typical aspect ratio of a PDF417 symbol:
// each row is about 3 modules tall and each column is 17 modules wide,
// so a "square" symbol has roughly 3× as many rows as data-columns.

/**
 * Choose the number of data columns and rows for the symbol.
 *
 * ### Heuristic
 *
 * ```
 * c = clamp( ceil(sqrt(total / 3.0)), MIN_COLS, MAX_COLS )
 * r = clamp( ceil(total / c),         MIN_ROWS, MAX_ROWS )
 * ```
 *
 * The divisor 3 accounts for the 3:1 aspect ratio between module-rows
 * and module-columns in a typical PDF417 symbol (each data column is 17
 * modules wide, each logical row is rendered 3 pixels tall by default).
 *
 * @param total Total codewords (data + ECC) that must fit in the symbol.
 * @return Pair of (cols, rows) on success, or throws [PDF417Error.InputTooLong].
 * @throws PDF417Error.InputTooLong if the data exceeds the maximum symbol capacity.
 */
private fun chooseDimensions(total: Int): Pair<Int, Int> {
    val c = maxOf(MIN_COLS, minOf(MAX_COLS, ceil(sqrt(total / 3.0)).toInt()))
    val r = maxOf(MIN_ROWS, ceil(total.toDouble() / c).toInt())
    if (r > MAX_ROWS) {
        throw PDF417Error.InputTooLong(
            "Cannot fit $total codewords in any valid symbol (max $MAX_ROWS rows × $MAX_COLS cols)"
        )
    }
    return Pair(c, r)
}

// ============================================================================
// Row indicator computation
// ============================================================================
//
// Each logical PDF417 row carries two "indicator" codewords that encode
// metadata about the whole symbol:
//
//   R_info = (R - 1) / 3         R = total number of rows (3..90)
//   C_info = C - 1               C = number of data columns (1..30)
//   L_info = 3×L + (R-1) mod 3  L = ECC level (0..8)
//
// The three quantities are distributed across the three cluster types so
// that any three consecutive rows (one of each cluster) can reconstruct
// R, C, and L independently of reading from the top.
//
// Note: the RRI formula (Cluster 0 → C_info, Cluster 1 → R_info, Cluster 2 → L_info)
// follows the Python pdf417 library and TypeScript reference rather than the
// original spec text, because the Python/TS libraries produce verified
// scannable symbols.

/**
 * Compute the Left Row Indicator (LRI) codeword value for row [r].
 *
 * The LRI is looked up in the cluster table just like any data codeword
 * — its 17-module bar/space pattern carries the indicator value.
 *
 * ### Distribution by cluster (row % 3)
 *
 * | cluster | LRI value              |
 * |---------|------------------------|
 * | 0       | 30 × rowGroup + rInfo  |
 * | 1       | 30 × rowGroup + lInfo  |
 * | 2       | 30 × rowGroup + cInfo  |
 *
 * where:
 * - `rowGroup = r / 3`
 * - `rInfo    = (rows - 1) / 3`
 * - `cInfo    = cols - 1`
 * - `lInfo    = 3 * eccLevel + (rows - 1) % 3`
 *
 * @param r        Current row index (0-based).
 * @param rows     Total number of rows in the symbol.
 * @param cols     Number of data columns.
 * @param eccLevel Reed-Solomon ECC level.
 * @return LRI codeword value (0–928).
 */
fun computeLRI(r: Int, rows: Int, cols: Int, eccLevel: Int): Int {
    val rInfo    = (rows - 1) / 3
    val cInfo    = cols - 1
    val lInfo    = 3 * eccLevel + (rows - 1) % 3
    val rowGroup = r / 3
    return when (r % 3) {
        0    -> 30 * rowGroup + rInfo
        1    -> 30 * rowGroup + lInfo
        else -> 30 * rowGroup + cInfo
    }
}

/**
 * Compute the Right Row Indicator (RRI) codeword value for row [r].
 *
 * The RRI uses a different distribution from the LRI so that any single
 * row still carries unique structural information even if the other
 * indicator is corrupted.
 *
 * ### Distribution by cluster (row % 3)
 *
 * | cluster | RRI value              |
 * |---------|------------------------|
 * | 0       | 30 × rowGroup + cInfo  |
 * | 1       | 30 × rowGroup + rInfo  |
 * | 2       | 30 × rowGroup + lInfo  |
 *
 * @param r        Current row index (0-based).
 * @param rows     Total number of rows in the symbol.
 * @param cols     Number of data columns.
 * @param eccLevel Reed-Solomon ECC level.
 * @return RRI codeword value (0–928).
 */
fun computeRRI(r: Int, rows: Int, cols: Int, eccLevel: Int): Int {
    val rInfo    = (rows - 1) / 3
    val cInfo    = cols - 1
    val lInfo    = 3 * eccLevel + (rows - 1) % 3
    val rowGroup = r / 3
    return when (r % 3) {
        0    -> 30 * rowGroup + cInfo
        1    -> 30 * rowGroup + rInfo
        else -> 30 * rowGroup + lInfo
    }
}

// ============================================================================
// Pattern expansion helpers
// ============================================================================

/**
 * Start pattern for every PDF417 row: 8 elements, 17 modules.
 *
 * Binary representation: `11111111 010101 000` (left to right)
 * Bar/space widths: [8, 1, 1, 1, 1, 1, 1, 3]
 *
 * The start pattern is the same for all rows (no cluster cycling).
 */
private val START_WIDTHS = intArrayOf(8, 1, 1, 1, 1, 1, 1, 3)

/**
 * Stop pattern for every PDF417 row: 9 elements, 18 modules.
 *
 * Binary representation: `111111101 000101001` (left to right)
 * Bar/space widths: [7, 1, 1, 3, 1, 1, 1, 2, 1]
 *
 * The stop pattern is the same for all rows (no cluster cycling).
 */
private val STOP_WIDTHS = intArrayOf(7, 1, 1, 3, 1, 1, 1, 2, 1)

/**
 * Expand a packed bar/space pattern (from the cluster tables) into module booleans.
 *
 * ### Packing format
 *
 * The cluster table stores 8 element widths in a single 32-bit Int:
 *
 * ```
 * bits 31..28 = b1 (first bar width)
 * bits 27..24 = s1 (first space width)
 * bits 23..20 = b2
 * bits 19..16 = s2
 * bits 15..12 = b3
 * bits 11..8  = s3
 * bits  7..4  = b4
 * bits  3..0  = s4
 * ```
 *
 * We alternate: bar (dark = `true`), space (dark = `false`), bar, space, …
 * producing exactly 17 module values (sum of all widths = 17).
 *
 * @param packed  Packed bar/space pattern from the cluster table.
 * @param buf     Output boolean array to fill.
 * @param offset  Starting index in [buf].
 */
private fun expandPattern(packed: Int, buf: BooleanArray, offset: Int) {
    val b1 = (packed ushr 28) and 0xF
    val s1 = (packed ushr 24) and 0xF
    val b2 = (packed ushr 20) and 0xF
    val s2 = (packed ushr 16) and 0xF
    val b3 = (packed ushr 12) and 0xF
    val s3 = (packed ushr  8) and 0xF
    val b4 = (packed ushr  4) and 0xF
    val s4 = (packed        ) and 0xF

    var pos = offset
    repeat(b1) { buf[pos++] = true  }
    repeat(s1) { buf[pos++] = false }
    repeat(b2) { buf[pos++] = true  }
    repeat(s2) { buf[pos++] = false }
    repeat(b3) { buf[pos++] = true  }
    repeat(s3) { buf[pos++] = false }
    repeat(b4) { buf[pos++] = true  }
    repeat(s4) { buf[pos++] = false }
}

/**
 * Expand a bar/space width array into module booleans.
 *
 * The first element is always a bar (dark = `true`). Each subsequent
 * element alternates: bar → space → bar → space → …
 *
 * Used for the fixed start and stop patterns (which are not in the cluster
 * tables — they don't vary per codeword value).
 *
 * @param widths Bar/space width sequence.
 * @param buf    Output boolean array to fill.
 * @param offset Starting index in [buf].
 */
private fun expandWidths(widths: IntArray, buf: BooleanArray, offset: Int) {
    var dark = true
    var pos = offset
    for (w in widths) {
        repeat(w) { buf[pos++] = dark }
        dark = !dark
    }
}

// ============================================================================
// Rasterization
// ============================================================================

/**
 * Convert the flat codeword sequence into a [ModuleGrid].
 *
 * ### Row anatomy (modules, left to right)
 *
 * ```
 * start(17) | LRI(17) | data[0..c-1] (17 each) | RRI(17) | stop(18)
 * ```
 *
 * Total module columns per row:
 * ```
 * 17 + 17 + 17×cols + 17 + 18 = 69 + 17×cols
 * ```
 *
 * ### Row height
 *
 * Each logical PDF417 row is written [rowHeight] consecutive times into
 * the grid (identical boolean rows, stacked vertically). Scanners detect
 * rows by scanning the PDF417 symbol across multiple physical scan lines;
 * taller rows give more scan-line attempts per logical row.
 *
 * The minimum recommended [rowHeight] is 3 for reliable scanning. For
 * high-quality printers and readers at close range, 1 is acceptable.
 *
 * ### Grid construction strategy
 *
 * We build a mutable `BooleanArray[]` (raw 2D Java array) first because it
 * is ~10× faster than calling the immutable `setModule` function for every
 * pixel. At the end we wrap it in a [ModuleGrid] using `List.map { it.toList() }`.
 *
 * @param sequence  Flat array of codewords, row-major: sequence[r*cols + j].
 * @param rows      Number of logical PDF417 rows.
 * @param cols      Number of data columns.
 * @param eccLevel  ECC level (needed for row indicator calculation).
 * @param rowHeight Pixel height of each logical row (≥ 1).
 * @return Completed [ModuleGrid] ready for rendering.
 */
private fun rasterize(
    sequence: IntArray,
    rows: Int,
    cols: Int,
    eccLevel: Int,
    rowHeight: Int,
): ModuleGrid {
    // Total module columns = start(17) + LRI(17) + data(17*cols) + RRI(17) + stop(18).
    val moduleWidth  = 69 + 17 * cols
    // Total pixel rows = logical rows * rowHeight.
    val moduleHeight = rows * rowHeight

    // Pre-expand the start and stop patterns (same for every row).
    val startModules = BooleanArray(17)
    expandWidths(START_WIDTHS, startModules, 0)

    val stopModules = BooleanArray(18)
    expandWidths(STOP_WIDTHS, stopModules, 0)

    // Build the grid as a mutable BooleanArray[][] (Java-style) for speed.
    // We will convert to List<List<Boolean>> at the end.
    val rawGrid = Array(moduleHeight) { BooleanArray(moduleWidth) }

    for (r in 0 until rows) {
        val cluster      = r % 3
        val clusterTable = ClusterTables.CLUSTER_TABLES[cluster]

        // Build the complete boolean row for logical row r.
        val rowBuf = BooleanArray(moduleWidth)

        // 1. Start pattern (17 modules at positions 0..16).
        startModules.copyInto(rowBuf, 0, 0, 17)
        var pos = 17

        // 2. Left Row Indicator (17 modules).
        val lri = computeLRI(r, rows, cols, eccLevel)
        require(lri in 0 until clusterTable.size) {
            "LRI value $lri out of cluster table range [0, ${clusterTable.size})"
        }
        expandPattern(clusterTable[lri], rowBuf, pos)
        pos += 17

        // 3. Data codewords (17 modules each).
        for (j in 0 until cols) {
            val cw = sequence[r * cols + j]
            expandPattern(clusterTable[cw], rowBuf, pos)
            pos += 17
        }

        // 4. Right Row Indicator (17 modules).
        val rri = computeRRI(r, rows, cols, eccLevel)
        require(rri in 0 until clusterTable.size) {
            "RRI value $rri out of cluster table range [0, ${clusterTable.size})"
        }
        expandPattern(clusterTable[rri], rowBuf, pos)
        pos += 17

        // 5. Stop pattern (18 modules at positions pos..pos+17).
        stopModules.copyInto(rowBuf, pos, 0, 18)

        // Write this logical row rowHeight times (vertical repetition).
        val moduleRowBase = r * rowHeight
        for (h in 0 until rowHeight) {
            rawGrid[moduleRowBase + h] = rowBuf.copyOf()
        }
    }

    // Convert the raw BooleanArray[][] to List<List<Boolean>> for ModuleGrid.
    val modules: List<List<Boolean>> = rawGrid.map { row -> row.toList() }

    return ModuleGrid(
        rows        = moduleHeight,
        cols        = moduleWidth,
        modules     = modules,
        moduleShape = ModuleShape.SQUARE,
    )
}

// ============================================================================
// Main encoder: encode / encodeString
// ============================================================================

/**
 * Encode raw bytes as a PDF417 symbol and return the [ModuleGrid].
 *
 * ### Algorithm (full pipeline)
 *
 * 1. **Validate** options (ECC level 0–8, columns 1–30).
 * 2. **Byte-compact** the input: latch codeword 924, then 6-bytes-to-5-codewords
 *    groups (base-900), then residual bytes directly.
 * 3. **Choose ECC level** from [options] or auto-select via [autoEccLevel].
 * 4. **Build length descriptor**: the first codeword of the data region is
 *    `1 + len(dataCwords) + len(eccCwords)` — the total occupied codeword count.
 * 5. **Reed-Solomon encode** over GF(929) with b=3 convention.
 * 6. **Choose dimensions** (rows × cols) from [options] or auto-select
 *    via [chooseDimensions].
 * 7. **Pad** unused grid slots with codeword 900.
 * 8. **Rasterize**: start pattern + LRI + data + RRI + stop per row.
 * 9. Return the [ModuleGrid].
 *
 * ### Error conditions
 *
 * | Condition                          | Exception thrown                  |
 * |------------------------------------|-----------------------------------|
 * | `eccLevel` out of range 0–8        | [PDF417Error.InvalidECCLevel]     |
 * | `columns` out of range 1–30        | [PDF417Error.InvalidDimensions]   |
 * | Data too large for any valid symbol| [PDF417Error.InputTooLong]        |
 * | Data too large for chosen columns  | [PDF417Error.InputTooLong]        |
 *
 * @param bytes   Raw byte payload to encode.
 * @param options Encoding options. Use [PDF417Options] defaults for auto-selection.
 * @return [ModuleGrid] representing the barcode's boolean module pattern.
 * @throws PDF417Error on invalid options or data too large.
 */
fun encode(bytes: ByteArray, options: PDF417Options = PDF417Options()): ModuleGrid {
    // ── Validate ECC level ──────────────────────────────────────────────────
    options.eccLevel?.let { l ->
        if (l < 0 || l > 8) {
            throw PDF417Error.InvalidECCLevel("ECC level must be 0–8, got $l")
        }
    }

    // ── Validate columns ────────────────────────────────────────────────────
    options.columns?.let { c ->
        if (c < MIN_COLS || c > MAX_COLS) {
            throw PDF417Error.InvalidDimensions(
                "columns must be $MIN_COLS–$MAX_COLS, got $c"
            )
        }
    }

    // ── Byte compaction ─────────────────────────────────────────────────────
    val dataCwords = byteCompact(bytes)

    // ── Auto-select ECC level ───────────────────────────────────────────────
    val eccLevel  = options.eccLevel ?: autoEccLevel(dataCwords.size + 1)
    val eccCount  = 1 shl (eccLevel + 1)   // 2^(eccLevel+1)

    // ── Length descriptor ───────────────────────────────────────────────────
    // The length descriptor (codeword index 0) counts: itself + all data
    // codewords + all ECC codewords. It does NOT include padding codewords.
    val lengthDesc = 1 + dataCwords.size + eccCount
    val fullData   = intArrayOf(lengthDesc) + dataCwords

    // ── RS ECC ──────────────────────────────────────────────────────────────
    val eccCwords = rsEncode(fullData, eccLevel)

    // ── Choose dimensions ────────────────────────────────────────────────────
    val totalCwords = fullData.size + eccCwords.size
    val rowHeight   = maxOf(1, options.rowHeight ?: 3)

    val (cols, rows) = when (val userCols = options.columns) {
        null -> chooseDimensions(totalCwords)   // auto-select

        else -> {
            // User-specified columns: compute rows needed.
            val r = maxOf(MIN_ROWS, ceil(totalCwords.toDouble() / userCols).toInt())
            if (r > MAX_ROWS) {
                throw PDF417Error.InputTooLong(
                    "Data requires $r rows (max $MAX_ROWS) with $userCols columns"
                )
            }
            if (userCols * r < totalCwords) {
                throw PDF417Error.InputTooLong(
                    "Cannot fit $totalCwords codewords in $r×$userCols grid"
                )
            }
            Pair(userCols, r)
        }
    }

    // ── Pad to fill the grid exactly ────────────────────────────────────────
    val paddingCount = cols * rows - totalCwords
    val paddedData   = fullData + IntArray(paddingCount) { PADDING_CW }

    // Full sequence: padded data codewords followed by ECC codewords.
    val fullSequence = paddedData + eccCwords

    // ── Rasterize ────────────────────────────────────────────────────────────
    return rasterize(fullSequence, rows, cols, eccLevel, rowHeight)
}

/**
 * Encode a UTF-8 string as a PDF417 symbol and return the [ModuleGrid].
 *
 * Convenience wrapper around [encode]. The string is encoded to UTF-8 bytes
 * before compaction. For non-Latin scripts this means characters outside
 * ASCII will require more than one byte each.
 *
 * @param text    Text to encode (UTF-8).
 * @param options Encoding options.
 * @return [ModuleGrid] representing the barcode's boolean module pattern.
 * @throws PDF417Error on invalid options or data too large.
 */
fun encodeString(text: String, options: PDF417Options = PDF417Options()): ModuleGrid =
    encode(text.toByteArray(Charsets.UTF_8), options)

// ============================================================================
// Internal helpers exposed for testing
// ============================================================================

/**
 * Internal functions exposed for unit testing.
 *
 * These are implementation details — do **not** depend on them in
 * production code. They may change without notice.
 */
object Internal {
    /** GF(929) multiply (log/antilog). */
    fun gfMulExported(a: Int, b: Int): Int = gfMul(a, b)

    /** GF(929) add (mod 929). */
    fun gfAddExported(a: Int, b: Int): Int = gfAdd(a, b)

    /** GF(929) exponent table (α^i). */
    val GF_EXP_TABLE: IntArray get() = GF_EXP

    /** GF(929) discrete logarithm table. */
    val GF_LOG_TABLE: IntArray get() = GF_LOG

    /** Byte compaction (codeword sequence starting with latch 924). */
    fun byteCompactExported(bytes: ByteArray): IntArray = byteCompact(bytes)

    /** Reed-Solomon encode over GF(929). */
    fun rsEncodeExported(data: IntArray, eccLevel: Int): IntArray = rsEncode(data, eccLevel)

    /** Build the RS generator polynomial for a given ECC level. */
    fun buildGeneratorExported(eccLevel: Int): IntArray = buildGenerator(eccLevel)

    /** Auto-select the ECC level based on data codeword count. */
    fun autoEccLevelExported(dataCount: Int): Int = autoEccLevel(dataCount)
}
