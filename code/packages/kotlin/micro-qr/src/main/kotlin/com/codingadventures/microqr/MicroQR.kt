/**
 * MicroQR.kt — Micro QR Code encoder, ISO/IEC 18004:2015 Annex E compliant.
 *
 * Micro QR Code is the compact cousin of regular QR Code, designed for
 * applications where even the smallest standard QR symbol (21×21 at version 1)
 * is too large.  Common use cases: surface-mount component labels, circuit-board
 * markings, miniature industrial tags, and tiny stickers.
 *
 * ## Symbol sizes
 *
 * There are exactly four Micro QR symbol versions, each adding two rows and
 * two columns:
 *
 * ```
 * M1: 11×11   M2: 13×13   M3: 15×15   M4: 17×17
 * Formula: size = 2 × version_number + 9
 * ```
 *
 * ## Key differences from regular QR Code
 *
 * | Feature                  | Regular QR            | Micro QR              |
 * |--------------------------|-----------------------|-----------------------|
 * | Finder patterns          | 3 (corners)           | 1 (top-left only)     |
 * | Timing row/col           | Row 6 / Col 6         | Row 0 / Col 0         |
 * | Mask patterns            | 8                     | 4                     |
 * | Format info XOR mask     | 0x5412                | 0x4445                |
 * | Format info copies       | 2                     | 1                     |
 * | Quiet zone modules       | 4                     | 2                     |
 * | Mode indicator bits      | 4                     | 0–3 (grows with size) |
 * | Data block interleaving  | Yes (larger symbols)  | No (single block)     |
 *
 * ## Encoding pipeline
 *
 * ```
 * input string
 *   → auto-select smallest symbol (M1..M4) and most compact mode
 *   → build bit stream: mode indicator + char count + data + terminator + padding
 *   → Reed-Solomon ECC (GF(256)/0x11D, b=0, single block per symbol)
 *   → init grid: 7×7 finder, L-shaped separator, timing at row 0/col 0, format reserved
 *   → two-column zigzag data placement (bottom-right → top-left snake)
 *   → evaluate 4 mask patterns, select lowest penalty score
 *   → write 15-bit format information (XOR 0x4445, single copy)
 *   → return ModuleGrid
 * ```
 *
 * ## Usage
 *
 * ```kotlin
 * // Auto-select smallest symbol:
 * val grid = encode("HELLO")         // M2 (13×13), auto ECC
 *
 * // Force version and ECC:
 * val g4 = encode("https://a.b", MicroQROptions(symbol = "M4", eccLevel = ECCLevel.L))
 * assert(g4.rows == 17)
 *
 * // Force mask pattern 2:
 * val gm = encode("1", MicroQROptions(maskPattern = 2))
 * ```
 *
 * Spec: code/specs/DT2D06-micro-qr.md
 */
package com.codingadventures.microqr

import com.codingadventures.barcode2d.ModuleGrid
import com.codingadventures.barcode2d.ModuleShape
import com.codingadventures.gf256.GF256
import java.util.Collections

// ============================================================================
// Version
// ============================================================================

/** Package version string.  Follows Semantic Versioning 2.0. */
const val VERSION = "0.1.0"

// ============================================================================
// Error hierarchy
// ============================================================================

/**
 * Base class for all Micro QR encoding errors.
 *
 * Using a sealed class hierarchy means the Kotlin compiler can exhaustively
 * check `when` expressions over [MicroQRError] subtypes — if a new error
 * variant is added here, the compiler flags every unhandled `when`.
 *
 * Because all subclasses are also subtypes of [Exception], they can be caught
 * with `catch (e: MicroQRError)` or `catch (e: Exception)`.
 */
sealed class MicroQRError(message: String) : Exception(message) {

    /**
     * The input string is too long to fit in any valid Micro QR symbol at the
     * requested ECC level.
     *
     * The maximum capacity is 35 numeric characters in M4-L.  If you hit this
     * limit with a URL or byte string, consider switching to a full QR Code.
     *
     * @param message Human-readable description of the overflow.
     */
    data class InputTooLong(override val message: String) : MicroQRError(message)

    /**
     * The requested ECC level is not defined for the chosen symbol version.
     *
     * For example, M1 only supports [ECCLevel.DETECTION]; requesting M1 + L
     * throws this error.  Consult the [ECCLevel] documentation for valid
     * version/level pairings.
     *
     * @param message Human-readable description of the invalid combination.
     */
    data class InvalidECCLevel(override val message: String) : MicroQRError(message)

    /**
     * The provided options are internally inconsistent or out of range.
     *
     * Examples:
     * - `symbol = "M5"` (no such version)
     * - `maskPattern = 7` (only 0–3 are valid in Micro QR)
     *
     * @param message Human-readable description of the violation.
     */
    data class InvalidOptions(override val message: String) : MicroQRError(message)
}

// ============================================================================
// ECCLevel — error correction level
// ============================================================================

/**
 * Error correction level for Micro QR symbols.
 *
 * ### ECC level availability
 *
 * | Level     | Available in   | Recovery capability          |
 * |-----------|----------------|------------------------------|
 * | DETECTION | M1 only        | detects errors, no correction |
 * | L         | M2, M3, M4     | ~7% of codewords corrected    |
 * | M         | M2, M3, M4     | ~15% of codewords corrected   |
 * | Q         | M4 only        | ~25% of codewords corrected   |
 *
 * Level H (high, ~30%) is **not** available in any Micro QR symbol; the
 * smaller grid leaves too little room for that much ECC overhead.
 *
 * ### Why not H?
 *
 * A 17×17 (M4) symbol has only 289 modules total.  After structural overhead
 * (finder, timing, format info), roughly 100 modules remain for data + ECC.
 * Level H would need ~30% ECC, leaving just ~70 payload bits — not enough for
 * any meaningful data.
 */
enum class ECCLevel { DETECTION, L, M, Q }

// ============================================================================
// MicroQROptions — encoder configuration
// ============================================================================

/**
 * Configuration for the Micro QR encoder.
 *
 * All fields are optional; `null` means "auto-select the best value":
 *
 * - [symbol]: `null` → auto-select smallest symbol that fits the input.
 * - [eccLevel]: `null` → auto-select the highest level that fits; DETECTION
 *   for M1, L for M2/M3, L for M4.
 * - [maskPattern]: `null` → evaluate all 4 patterns and pick the one with
 *   the lowest penalty score.
 *
 * ### Example usage
 *
 * ```kotlin
 * // All auto:
 * encode("HELLO", MicroQROptions())
 *
 * // Force M4-Q, auto mask:
 * encode("HELLO", MicroQROptions(symbol = "M4", eccLevel = ECCLevel.Q))
 *
 * // Force mask pattern 1:
 * encode("1",     MicroQROptions(maskPattern = 1))
 * ```
 *
 * @param symbol      One of "M1", "M2", "M3", "M4" (case-insensitive), or `null` for auto.
 * @param eccLevel    Desired ECC level, or `null` for auto.
 * @param maskPattern Override mask pattern (0–3), or `null` for auto.
 */
data class MicroQROptions(
    val symbol: String? = null,
    val eccLevel: ECCLevel? = null,
    val maskPattern: Int? = null,
)

// ============================================================================
// Internal: encoding mode
// ============================================================================

/**
 * Data encoding mode — determines how input characters are packed into bits.
 *
 * Selection priority (most compact first): NUMERIC > ALPHANUMERIC > BYTE.
 * A character set check is made for each mode in order; the first mode whose
 * character set contains all input characters is selected.
 *
 * | Mode         | Characters                         | Bits per char |
 * |--------------|------------------------------------|---------------|
 * | NUMERIC      | 0–9                                | 3.33 (groups) |
 * | ALPHANUMERIC | 0–9, A–Z, space, $%*+-./: (45 ch) | 5.5 (pairs)   |
 * | BYTE         | any UTF-8 byte (0–255)             | 8             |
 */
private enum class EncodingMode { NUMERIC, ALPHANUMERIC, BYTE }

// ============================================================================
// Internal: symbol configuration table
// ============================================================================

/**
 * Compile-time constants for one (version, ECC level) combination.
 *
 * There are exactly 8 valid Micro QR configurations:
 *   M1/Detection, M2/L, M2/M, M3/L, M3/M, M4/L, M4/M, M4/Q
 *
 * All numbers come directly from ISO 18004:2015 Annex E tables.  Embedding
 * them as constants avoids runtime calculation errors and makes all language
 * ports produce bit-identical output.
 *
 * @param version           Symbol version name ("M1"–"M4").
 * @param ecc               ECC level.
 * @param symbolIndicator   3-bit value placed in the format information word (0–7).
 * @param size              Side length in modules (11, 13, 15, or 17).
 * @param dataCw            Number of data codewords (bytes).
 * @param eccCw             Number of ECC codewords.
 * @param numericCap        Max numeric characters.  0 = mode not supported.
 * @param alphaCap          Max alphanumeric characters.  0 = mode not supported.
 * @param byteCap           Max byte-mode bytes.  0 = mode not supported.
 * @param terminatorBits    Number of zero bits in the stream terminator (3/5/7/9).
 * @param modeIndicatorBits Width of mode indicator (0=M1, 1=M2, 2=M3, 3=M4).
 * @param ccBitsNumeric     Char-count field width for numeric mode.
 * @param ccBitsAlpha       Char-count field width for alphanumeric mode.
 * @param ccBitsByte        Char-count field width for byte mode.
 * @param m1HalfCw          True for M1 only: last data codeword contributes only 4 bits.
 */
private data class SymbolConfig(
    val version: String,
    val ecc: ECCLevel,
    val symbolIndicator: Int,
    val size: Int,
    val dataCw: Int,
    val eccCw: Int,
    val numericCap: Int,
    val alphaCap: Int,
    val byteCap: Int,
    val terminatorBits: Int,
    val modeIndicatorBits: Int,
    val ccBitsNumeric: Int,
    val ccBitsAlpha: Int,
    val ccBitsByte: Int,
    val m1HalfCw: Boolean,
)

/**
 * All 8 valid Micro QR symbol configurations, ordered smallest → largest.
 *
 * The auto-selection algorithm iterates this list and stops at the first
 * configuration that satisfies the version/ECC filter AND whose capacity fits
 * the input data.
 *
 * ### Capacity table (ISO 18004:2015 Annex E)
 *
 * ```
 * Symbol | ECC | Numeric | Alpha | Byte | DataCWs | EccCWs
 * -------|-----|---------|-------|------|---------|-------
 * M1     | Det |       5 |     — |    — |       3 |      2
 * M2     | L   |      10 |     6 |    4 |       5 |      5
 * M2     | M   |       8 |     5 |    3 |       4 |      6
 * M3     | L   |      23 |    14 |    9 |      11 |      6
 * M3     | M   |      18 |    11 |    7 |       9 |      8
 * M4     | L   |      35 |    21 |   15 |      16 |      8
 * M4     | M   |      30 |    18 |   13 |      14 |     10
 * M4     | Q   |      21 |    13 |    9 |      10 |     14
 * ```
 */
private val SYMBOL_CONFIGS: List<SymbolConfig> = listOf(
    // M1 / Detection — smallest, numeric only, error detection not correction
    SymbolConfig("M1", ECCLevel.DETECTION,  0, 11,  3,  2,  5,  0,  0, 3, 0, 3, 0, 0, true),
    // M2 / L — adds alphanumeric and byte modes, lowest ECC overhead
    SymbolConfig("M2", ECCLevel.L,          1, 13,  5,  5, 10,  6,  4, 5, 1, 4, 3, 4, false),
    // M2 / M — same size as M2-L but more ECC, fewer data codewords
    SymbolConfig("M2", ECCLevel.M,          2, 13,  4,  6,  8,  5,  3, 5, 1, 4, 3, 4, false),
    // M3 / L — first size with meaningful byte capacity (9)
    SymbolConfig("M3", ECCLevel.L,          3, 15, 11,  6, 23, 14,  9, 7, 2, 5, 4, 4, false),
    // M3 / M — same grid, medium ECC
    SymbolConfig("M3", ECCLevel.M,          4, 15,  9,  8, 18, 11,  7, 7, 2, 5, 4, 4, false),
    // M4 / L — largest grid, best capacity (35 numeric / 15 byte)
    SymbolConfig("M4", ECCLevel.L,          5, 17, 16,  8, 35, 21, 15, 9, 3, 6, 5, 5, false),
    // M4 / M — good balance of data and ECC
    SymbolConfig("M4", ECCLevel.M,          6, 17, 14, 10, 30, 18, 13, 9, 3, 6, 5, 5, false),
    // M4 / Q — highest ECC available in Micro QR (~25% recovery)
    SymbolConfig("M4", ECCLevel.Q,          7, 17, 10, 14, 21, 13,  9, 9, 3, 6, 5, 5, false),
)

// ============================================================================
// Reed-Solomon generator polynomials (compile-time constants)
// ============================================================================

/**
 * Monic RS generator polynomial for GF(256)/0x11D with b=0 convention.
 *
 * The degree-n generator is:
 * ```
 *   g(x) = (x + α⁰)(x + α¹) ··· (x + α^{n-1})
 * ```
 *
 * The returned array has length n+1; the first element is always 0x01 (leading
 * monic coefficient).  The six counts used by Micro QR are {2, 5, 6, 8, 10, 14}.
 *
 * These are identical to the generators used by regular QR Code for matching
 * ECC block sizes.  Embedding them as constants means all language ports
 * produce bit-identical ECC bytes.
 *
 * @param eccCount Number of ECC codewords (= degree of generator polynomial).
 * @return Monic generator polynomial coefficients, length = eccCount + 1.
 * @throws IllegalArgumentException if eccCount is not a valid Micro QR ECC count.
 */
private fun getGenerator(eccCount: Int): IntArray = when (eccCount) {
    2  -> intArrayOf(0x01, 0x03, 0x02)
    5  -> intArrayOf(0x01, 0x1f, 0xf6, 0x44, 0xd9, 0x68)
    6  -> intArrayOf(0x01, 0x3f, 0x4e, 0x17, 0x9b, 0x05, 0x37)
    8  -> intArrayOf(0x01, 0x63, 0x0d, 0x60, 0x6d, 0x5b, 0x10, 0xa2, 0xa3)
    10 -> intArrayOf(0x01, 0xf6, 0x75, 0xa8, 0xd0, 0xc3, 0xe3, 0x36, 0xe1, 0x3c, 0x45)
    14 -> intArrayOf(0x01, 0xf6, 0x9a, 0x60, 0x97, 0x8a, 0xf1, 0xa4, 0xa1, 0x8e, 0xfc, 0x7a, 0x52, 0xad, 0xac)
    else -> throw IllegalArgumentException("No RS generator for eccCount=$eccCount")
}

// ============================================================================
// Pre-computed format information table
// ============================================================================

/**
 * All 32 pre-computed, XOR-masked format words for Micro QR Code.
 *
 * Indexed as `FORMAT_TABLE[symbolIndicator][maskPattern]`.
 *
 * ### 15-bit format word structure
 *
 * ```
 * Bits 14–12 : symbol indicator (version + ECC level)
 * Bits 11–10 : mask pattern (0–3)
 * Bits  9–0  : BCH(15,5) error correction for bits 14–10
 * ```
 *
 * The entire 15-bit word is then XOR-ed with 0x4445 to prevent a Micro QR
 * symbol from being misread as a standard QR symbol, which uses the same
 * finder pattern but XOR mask 0x5412.
 *
 * ### Pre-computed table
 *
 * ```
 * Symbol+ECC  | Mask 0 | Mask 1 | Mask 2 | Mask 3
 * ------------|--------|--------|--------|--------
 * M1  (000)   | 0x4445 | 0x4172 | 0x4E2B | 0x4B1C
 * M2-L (001)  | 0x5528 | 0x501F | 0x5F46 | 0x5A71
 * M2-M (010)  | 0x6649 | 0x637E | 0x6C27 | 0x6910
 * M3-L (011)  | 0x7764 | 0x7253 | 0x7D0A | 0x783D
 * M3-M (100)  | 0x06DE | 0x03E9 | 0x0CB0 | 0x0987
 * M4-L (101)  | 0x17F3 | 0x12C4 | 0x1D9D | 0x18AA
 * M4-M (110)  | 0x24B2 | 0x2185 | 0x2EDC | 0x2BEB
 * M4-Q (111)  | 0x359F | 0x30A8 | 0x3FF1 | 0x3AC6
 * ```
 */
private val FORMAT_TABLE: Array<IntArray> = arrayOf(
    intArrayOf(0x4445, 0x4172, 0x4E2B, 0x4B1C),  // M1
    intArrayOf(0x5528, 0x501F, 0x5F46, 0x5A71),  // M2-L
    intArrayOf(0x6649, 0x637E, 0x6C27, 0x6910),  // M2-M
    intArrayOf(0x7764, 0x7253, 0x7D0A, 0x783D),  // M3-L
    intArrayOf(0x06DE, 0x03E9, 0x0CB0, 0x0987),  // M3-M
    intArrayOf(0x17F3, 0x12C4, 0x1D9D, 0x18AA),  // M4-L
    intArrayOf(0x24B2, 0x2185, 0x2EDC, 0x2BEB),  // M4-M
    intArrayOf(0x359F, 0x30A8, 0x3FF1, 0x3AC6),  // M4-Q
)

// ============================================================================
// Alphanumeric character set
// ============================================================================

/**
 * The 45-character set shared with regular QR Code.
 *
 * Characters are assigned indices 0–44 in left-to-right order:
 * ```
 *  0–9  : '0'–'9'
 * 10–35 : 'A'–'Z'
 * 36    : ' '
 * 37    : '$'
 * 38    : '%'
 * 39    : '*'
 * 40    : '+'
 * 41    : '-'
 * 42    : '.'
 * 43    : '/'
 * 44    : ':'
 * ```
 *
 * Pairs of characters are encoded as `first_index × 45 + second_index` → 11 bits.
 * A trailing single character uses 6 bits.
 */
private const val ALPHANUM_CHARS = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ \$%*+-./:"

// ============================================================================
// Public API
// ============================================================================

/**
 * Encode a string to a Micro QR Code [ModuleGrid].
 *
 * Automatically selects the smallest symbol (M1..M4) and most compact encoding
 * mode that can hold the input, unless overridden via [opts].
 *
 * ### Auto-selection rules
 *
 * 1. Iterate SYMBOL_CONFIGS in order (smallest first).
 * 2. Skip entries that don't match any filter in [opts].
 * 3. For each candidate, determine the best encoding mode.
 * 4. Return the first candidate whose capacity ≥ input length.
 *
 * ### Mask selection
 *
 * When [MicroQROptions.maskPattern] is `null`, all 4 masks (0–3) are evaluated
 * using the standard QR Code four-rule penalty scorer.  The mask with the
 * lowest total penalty is applied.  Ties are broken by lowest mask index.
 *
 * @param data  The string to encode.  Must not be null.
 * @param opts  Encoder options (all optional, see [MicroQROptions]).
 * @return A complete [ModuleGrid] ready for rendering with [com.codingadventures.barcode2d.layout].
 * @throws MicroQRError.InputTooLong      if the input exceeds M4 capacity at the requested level.
 * @throws MicroQRError.InvalidECCLevel   if the version+ECC combination is not valid.
 * @throws MicroQRError.InvalidOptions    if opts contains out-of-range values.
 */
fun encode(data: String, opts: MicroQROptions = MicroQROptions()): ModuleGrid {
    // Validate mask pattern override if provided.
    if (opts.maskPattern != null && (opts.maskPattern < 0 || opts.maskPattern > 3)) {
        throw MicroQRError.InvalidOptions(
            "maskPattern must be 0–3, got ${opts.maskPattern}"
        )
    }

    // Validate symbol string if provided.
    val versionFilter: String? = opts.symbol?.uppercase()
    if (versionFilter != null && versionFilter !in listOf("M1", "M2", "M3", "M4")) {
        throw MicroQRError.InvalidOptions(
            "symbol must be one of M1/M2/M3/M4 or null, got '${opts.symbol}'"
        )
    }

    // Select the symbol configuration that fits the data.
    val cfg = selectConfig(data, versionFilter, opts.eccLevel)

    // Determine the encoding mode (numeric / alphanumeric / byte).
    val mode = selectMode(data, cfg)

    // Run the full encoding pipeline.
    return encodeWithConfig(data, cfg, mode, opts.maskPattern)
}

// ============================================================================
// Symbol selection
// ============================================================================

/**
 * Find the smallest [SymbolConfig] that can hold [input] at the given constraints.
 *
 * Iterates [SYMBOL_CONFIGS] in order (smallest first).  For each candidate
 * that satisfies the version/ECC filter:
 *
 * 1. Determine the best encoding mode.
 * 2. Check that the input length fits within the mode's capacity.
 * 3. Return the first match.
 *
 * ### Error conditions
 *
 * - If a version+ECC filter is set but matches no entry → [MicroQRError.InvalidECCLevel].
 * - If the filter matches entries but none fits the input → [MicroQRError.InputTooLong].
 *
 * @param input         The string to encode.
 * @param versionFilter "M1"/"M2"/"M3"/"M4" or `null` for any.
 * @param eccFilter     ECC level or `null` for any.
 * @return The matching [SymbolConfig].
 */
private fun selectConfig(
    input: String,
    versionFilter: String?,
    eccFilter: ECCLevel?,
): SymbolConfig {
    var foundMatchingFilter = false

    for (cfg in SYMBOL_CONFIGS) {
        // Apply version filter.
        if (versionFilter != null && cfg.version != versionFilter) continue
        // Apply ECC filter.
        if (eccFilter != null && cfg.ecc != eccFilter) continue

        foundMatchingFilter = true

        // Try to find a supported encoding mode for this config.
        val mode = trySelectMode(input, cfg) ?: continue

        // Capacity check: byte mode counts bytes, others count characters.
        val len = if (mode == EncodingMode.BYTE) input.toByteArray().size else input.length
        val cap = when (mode) {
            EncodingMode.NUMERIC      -> cfg.numericCap
            EncodingMode.ALPHANUMERIC -> cfg.alphaCap
            EncodingMode.BYTE         -> cfg.byteCap
        }

        if (cap > 0 && len <= cap) {
            return cfg
        }
    }

    if (!foundMatchingFilter) {
        throw MicroQRError.InvalidECCLevel(
            "No symbol configuration matches version=$versionFilter ecc=$eccFilter"
        )
    }

    throw MicroQRError.InputTooLong(
        "Input (length ${input.length}) does not fit in any Micro QR symbol " +
        "(version=$versionFilter, ecc=$eccFilter). " +
        "Maximum is 35 numeric chars in M4-L."
    )
}

/**
 * Returns the most compact mode supported by [cfg] that can encode [input],
 * or `null` if no supported mode handles the input.
 *
 * Priority: NUMERIC > ALPHANUMERIC > BYTE.
 */
private fun trySelectMode(input: String, cfg: SymbolConfig): EncodingMode? {
    if (cfg.ccBitsNumeric > 0 && isNumeric(input)) return EncodingMode.NUMERIC
    if (cfg.alphaCap > 0 && isAlphanumeric(input)) return EncodingMode.ALPHANUMERIC
    if (cfg.byteCap > 0) return EncodingMode.BYTE
    return null
}

/**
 * Select the most compact mode for [input] in [cfg], throwing if no mode works.
 *
 * Used after config selection (where the config is already known to fit).
 */
private fun selectMode(input: String, cfg: SymbolConfig): EncodingMode =
    trySelectMode(input, cfg)
        ?: throw MicroQRError.InvalidOptions(
            "Input cannot be encoded in any mode supported by ${cfg.version}-${cfg.ecc}"
        )

/** Returns `true` if every character in [s] is an ASCII digit '0'–'9'. */
private fun isNumeric(s: String): Boolean = s.all { it in '0'..'9' }

/** Returns `true` if every character in [s] is in the 45-character alphanumeric set. */
private fun isAlphanumeric(s: String): Boolean = s.all { ALPHANUM_CHARS.indexOf(it) >= 0 }

// ============================================================================
// Data encoding — bit stream construction
// ============================================================================

/**
 * Build the data codeword byte sequence for the given input, config, and mode.
 *
 * ### Regular symbols (M2, M3, M4)
 *
 * ```
 * [mode indicator (0–3 bits)]
 * [character count (3–6 bits)]
 * [data bits (variable)]
 * [terminator (up to terminatorBits zero bits, truncated if full)]
 * [zero-pad to byte boundary]
 * [fill codewords: 0xEC, 0x11, 0xEC, 0x11, …]
 * → exactly cfg.dataCw bytes
 * ```
 *
 * ### M1 special case ([SymbolConfig.m1HalfCw] = `true`)
 *
 * M1 has a data capacity of 20 bits = 2 full bytes + a 4-bit nibble.
 * The RS encoder receives 3 bytes where `byte[2]` has data in its upper 4 bits
 * and zeros in its lower 4 bits.  This matches the Java reference behaviour.
 *
 * @param input The string to encode.
 * @param cfg   The symbol configuration.
 * @param mode  The chosen encoding mode.
 * @return Exactly `cfg.dataCw` bytes ready for RS encoding.
 */
private fun buildDataCodewords(input: String, cfg: SymbolConfig, mode: EncodingMode): ByteArray {
    // Total usable data bit capacity:
    // M1: 3 data CWs − 4 bits (last CW is a half-byte) = 20 bits.
    // Others: dataCw × 8 bits.
    val totalBits = if (cfg.m1HalfCw) cfg.dataCw * 8 - 4 else cfg.dataCw * 8

    val w = BitWriter()

    // ── Mode indicator ────────────────────────────────────────────────────────
    // M1 has 0 mode indicator bits (only numeric mode exists).
    // M2 has 1 bit: 0=numeric, 1=alphanumeric.
    // M3 has 2 bits: 00=numeric, 01=alpha, 10=byte.
    // M4 has 3 bits: 000=numeric, 001=alpha, 010=byte.
    if (cfg.modeIndicatorBits > 0) {
        w.write(modeIndicatorValue(mode, cfg), cfg.modeIndicatorBits)
    }

    // ── Character count ───────────────────────────────────────────────────────
    // Byte mode counts UTF-8 bytes; other modes count characters.
    val charCount = if (mode == EncodingMode.BYTE) input.toByteArray().size else input.length
    val ccBits = charCountBits(mode, cfg)
    w.write(charCount, ccBits)

    // ── Encoded data ──────────────────────────────────────────────────────────
    when (mode) {
        EncodingMode.NUMERIC      -> encodeNumeric(input, w)
        EncodingMode.ALPHANUMERIC -> encodeAlphanumeric(input, w)
        EncodingMode.BYTE         -> encodeByteMode(input, w)
    }

    // ── Terminator ────────────────────────────────────────────────────────────
    // Append up to terminatorBits zero bits, truncated if the capacity is full.
    val remaining = totalBits - w.bitLen
    if (remaining > 0) {
        w.write(0, minOf(cfg.terminatorBits, remaining))
    }

    // ── M1 special packing ────────────────────────────────────────────────────
    if (cfg.m1HalfCw) {
        // Pack 20 bits into 3 bytes: bytes 0 and 1 are full bytes; byte 2
        // holds data in the upper nibble, zeros in the lower nibble.
        val bits = w.toBitArray()
        val padded = IntArray(20)
        for (i in 0 until minOf(bits.size, 20)) padded[i] = bits[i]

        fun packByte(start: Int, count: Int): Byte {
            var b = 0
            for (i in 0 until count) b = b or (padded[start + i] shl (7 - i))
            return b.toByte()
        }

        return byteArrayOf(packByte(0, 8), packByte(8, 8), packByte(16, 4))
    }

    // ── Byte-align ────────────────────────────────────────────────────────────
    val rem = w.bitLen % 8
    if (rem != 0) w.write(0, 8 - rem)

    // ── Fill remaining codewords with 0xEC / 0x11 alternating ────────────────
    // These are the same fill bytes used in regular QR Code.  They switch the
    // decoder's pad-byte state machine through a safe cycle so stray bits do
    // not confuse the decoder.
    val bytes = w.toBytes()
    val result = ByteArray(cfg.dataCw)
    bytes.copyInto(result, 0, 0, minOf(bytes.size, cfg.dataCw))

    var pad = 0xEC.toByte()
    for (i in bytes.size until cfg.dataCw) {
        result[i] = pad
        pad = if (pad == 0xEC.toByte()) 0x11.toByte() else 0xEC.toByte()
    }
    return result
}

/**
 * Mode indicator bit value for the given mode in the given symbol.
 *
 * The indicator width grows with the symbol version (it needs room for more modes):
 * ```
 * M1 (0 bits): no indicator — only numeric mode, value irrelevant
 * M2 (1 bit):  numeric=0, alphanumeric=1
 * M3 (2 bits): numeric=00, alpha=01, byte=10
 * M4 (3 bits): numeric=000, alpha=001, byte=010
 * ```
 */
private fun modeIndicatorValue(mode: EncodingMode, cfg: SymbolConfig): Int =
    when (cfg.modeIndicatorBits) {
        0 -> 0
        1 -> if (mode == EncodingMode.NUMERIC) 0 else 1
        2 -> when (mode) {
            EncodingMode.NUMERIC      -> 0b00
            EncodingMode.ALPHANUMERIC -> 0b01
            EncodingMode.BYTE         -> 0b10
        }
        3 -> when (mode) {
            EncodingMode.NUMERIC      -> 0b000
            EncodingMode.ALPHANUMERIC -> 0b001
            EncodingMode.BYTE         -> 0b010
        }
        else -> 0
    }

/**
 * Width (in bits) of the character count field for the given mode and config.
 *
 * ```
 * Mode         | M1 | M2 | M3 | M4
 * -------------|----|----|----|----|
 * Numeric      |  3 |  4 |  5 |  6
 * Alphanumeric |  — |  3 |  4 |  5
 * Byte         |  — |  — |  4 |  5
 * ```
 */
private fun charCountBits(mode: EncodingMode, cfg: SymbolConfig): Int = when (mode) {
    EncodingMode.NUMERIC      -> cfg.ccBitsNumeric
    EncodingMode.ALPHANUMERIC -> cfg.ccBitsAlpha
    EncodingMode.BYTE         -> cfg.ccBitsByte
}

/**
 * Encode numeric string: groups of 3 digits → 10 bits, pair → 7 bits, single → 4 bits.
 *
 * Example: "12345" → groups "123" (123 in 10 bits), "45" (45 in 7 bits).
 *
 * This packing is the same as regular QR Code numeric encoding.  The key idea
 * is that three decimal digits have 1000 combinations (0–999) which fits in
 * 10 bits (1024 values), so we lose only 2.4% efficiency vs. raw decimal.
 */
private fun encodeNumeric(input: String, w: BitWriter) {
    var i = 0
    while (i + 2 < input.length) {
        val v = (input[i] - '0') * 100 + (input[i + 1] - '0') * 10 + (input[i + 2] - '0')
        w.write(v, 10)
        i += 3
    }
    if (i + 1 < input.length) {
        val v = (input[i] - '0') * 10 + (input[i + 1] - '0')
        w.write(v, 7)
        i += 2
    }
    if (i < input.length) {
        w.write(input[i] - '0', 4)
    }
}

/**
 * Encode alphanumeric string: pairs → 11 bits, single trailing char → 6 bits.
 *
 * A pair `(a, b)` at indices `iA` and `iB` in [ALPHANUM_CHARS] is packed as:
 * ```
 * iA × 45 + iB   →  11 bits
 * ```
 * Maximum pair value = 44 × 45 + 44 = 2024, which fits in 11 bits (2048 max).
 *
 * A single trailing character uses its index in 6 bits (max 44 < 64).
 */
private fun encodeAlphanumeric(input: String, w: BitWriter) {
    var i = 0
    while (i + 1 < input.length) {
        val a = ALPHANUM_CHARS.indexOf(input[i])
        val b = ALPHANUM_CHARS.indexOf(input[i + 1])
        w.write(a * 45 + b, 11)
        i += 2
    }
    if (i < input.length) {
        w.write(ALPHANUM_CHARS.indexOf(input[i]), 6)
    }
}

/**
 * Encode byte-mode string: each UTF-8 byte → 8 bits (MSB first).
 *
 * Multi-byte UTF-8 sequences are treated as individual byte values.  The
 * character count field counts bytes, not Unicode code points.  For ASCII
 * strings, bytes and code points are the same.
 */
private fun encodeByteMode(input: String, w: BitWriter) {
    for (b in input.toByteArray()) {
        w.write(b.toInt() and 0xFF, 8)
    }
}

// ============================================================================
// Reed-Solomon encoder
// ============================================================================

/**
 * Compute ECC bytes via LFSR polynomial division over GF(256)/0x11D, b=0.
 *
 * Returns the remainder of `D(x) · x^n mod G(x)` where:
 * - `D(x)` is the data polynomial (coefficients = [data] bytes, most significant first)
 * - `G(x)` is the [generator] polynomial of degree n
 * - `n` = number of ECC codewords
 *
 * ### Algorithm (polynomial long division via LFSR)
 *
 * ```
 * rem = IntArray(n)  // LFSR register, initially all zeros
 * for each data byte b:
 *     feedback = b XOR rem[0]
 *     shift rem left by one (discard rem[0], push 0 at rem[n-1])
 *     for i in 0..n-1:
 *         rem[i] ^= GF.mul(generator[i+1], feedback)
 * result = rem
 * ```
 *
 * This runs in O(k·n) time where k = data codewords.  For Micro QR's largest
 * config (M4-Q: 10 data + 14 ECC), that is 140 GF multiplications — negligible.
 *
 * This function is `internal` (accessible from tests in the same module).
 *
 * @param data      Data codewords to protect.
 * @param generator Monic generator polynomial (length = eccCount + 1).
 * @return ECC codewords (length = eccCount = generator.size - 1).
 */
internal fun rsEncode(data: ByteArray, generator: IntArray): ByteArray {
    val n = generator.size - 1  // ECC count = degree of generator
    val rem = IntArray(n)       // LFSR shift register

    for (b in data) {
        val fb = (b.toInt() and 0xFF) xor rem[0]
        // Shift register left: drop rem[0], move rem[1..n-1] → rem[0..n-2], push 0.
        rem.copyInto(rem, 0, 1, n)
        rem[n - 1] = 0
        if (fb != 0) {
            for (i in 0 until n) {
                rem[i] = rem[i] xor GF256.mul(generator[i + 1], fb)
            }
        }
    }

    return ByteArray(n) { i -> rem[i].toByte() }
}

// ============================================================================
// Grid construction — mutable working grid
// ============================================================================

/**
 * Mutable working grid used only during encoding.
 *
 * Unlike the immutable [ModuleGrid] returned to callers, this internal class
 * is mutated in place during the encoding pipeline, then discarded after the
 * final grid is assembled.
 *
 * `modules[row][col] = true` → dark module.
 * `reserved[row][col] = true` → structural module (must not be changed by
 *   data placement or masking).
 *
 * @param size Symbol side length in modules (11, 13, 15, or 17).
 */
private class WorkGrid(val size: Int) {
    val modules  = Array(size) { BooleanArray(size) }  // all false = all light
    val reserved = Array(size) { BooleanArray(size) }  // no reservations

    /**
     * Set module at (row, col) to [dark], optionally marking it [reserve]d.
     *
     * Calling `reserve = true` on a module prevents it from being modified by
     * [placeBits] or [applyMask].
     */
    fun set(row: Int, col: Int, dark: Boolean, reserve: Boolean = false) {
        modules[row][col] = dark
        if (reserve) reserved[row][col] = true
    }
}

/**
 * Place the 7×7 finder pattern at the top-left corner (rows 0–6, cols 0–6).
 *
 * The finder pattern has a distinctive 1:1:3:1:1 dark-to-light ratio that
 * barcode scanners use to locate and orient the symbol.  It looks like:
 *
 * ```
 * ■ ■ ■ ■ ■ ■ ■     ← outer ring (dark border)
 * ■ □ □ □ □ □ ■
 * ■ □ ■ ■ ■ □ ■     ← inner core (3×3 dark square)
 * ■ □ ■ ■ ■ □ ■
 * ■ □ ■ ■ ■ □ ■
 * ■ □ □ □ □ □ ■
 * ■ ■ ■ ■ ■ ■ ■     ← outer ring (dark border)
 * ```
 *
 * The pattern is identical to regular QR Code.  Because Micro QR has only ONE
 * finder (top-left only), the data area is always bottom-right — no ambiguity
 * about orientation.
 */
private fun placeFinder(g: WorkGrid) {
    for (dr in 0..6) {
        for (dc in 0..6) {
            val onBorder = dr == 0 || dr == 6 || dc == 0 || dc == 6
            val inCore   = dr in 2..4 && dc in 2..4
            g.set(dr, dc, onBorder || inCore, reserve = true)
        }
    }
}

/**
 * Place the L-shaped separator (8 light modules at row 7 cols 0–7, and
 * 8 light modules at col 7 rows 0–7).
 *
 * The separator creates a 1-module quiet zone around the finder on its two
 * interior sides (bottom and right).  The symbol boundary forms the quiet zone
 * on the other two sides.
 *
 * The corner module at (row=7, col=7) is written twice — once by each strip —
 * but both writes set it light, so the double-write is harmless.
 */
private fun placeSeparator(g: WorkGrid) {
    for (i in 0..7) {
        g.set(7, i, dark = false, reserve = true)  // bottom strip: row 7, cols 0–7
        g.set(i, 7, dark = false, reserve = true)  // right strip:  col 7, rows 0–7
    }
}

/**
 * Place timing pattern extensions along row 0 and col 0.
 *
 * Micro QR places timing patterns on the outer edges (row 0 and col 0) of the
 * finder, extending them to the far edge of the symbol.  Regular QR uses row 6
 * and col 6 instead.
 *
 * Positions 0–6: already filled by the finder pattern.
 * Position 7: the separator (always light, already written).
 * Positions 8 onward: alternating dark (even index) / light (odd index).
 *
 * Example for M2 (size=13):
 * ```
 * row 0: ■ ■ ■ ■ ■ ■ ■ □ ■ □ ■ □ ■
 *         └───finder────┘ sep timing
 * ```
 */
private fun placeTiming(g: WorkGrid) {
    for (c in 8 until g.size) g.set(0, c, c % 2 == 0, reserve = true)  // horizontal
    for (r in 8 until g.size) g.set(r, 0, r % 2 == 0, reserve = true)  // vertical
}

/**
 * Reserve the 15 format information module positions (do not write data yet).
 *
 * Reserving before data placement ensures the zigzag algorithm skips these
 * modules.  The actual format word is written later, after the best mask is
 * selected.
 *
 * ### Format information layout (Micro QR, single copy)
 *
 * ```
 * Row 8, cols 1–8  → bits f14 (MSB) down to f7
 * Col 8, rows 7–1  → bits f6 down to f0 (LSB)
 * ```
 *
 * 8 + 7 = 15 modules = 15 bits.
 */
private fun reserveFormatInfo(g: WorkGrid) {
    for (c in 1..8) g.set(8, c, dark = false, reserve = true)  // horizontal strip
    for (r in 1..7) g.set(r, 8, dark = false, reserve = true)  // vertical strip
}

/**
 * Write the 15-bit [fmt] word into the reserved format information positions.
 *
 * Bit f14 (MSB) is placed first, going right along row 8, then upward along
 * col 8:
 *
 * ```
 * Row 8, col 1  ← f14  (MSB)
 * Row 8, col 2  ← f13
 * …
 * Row 8, col 8  ← f7
 * Col 8, row 7  ← f6
 * Col 8, row 6  ← f5
 * …
 * Col 8, row 1  ← f0   (LSB)
 * ```
 *
 * The [modules] array is modified directly (not the [WorkGrid], because this
 * is called after the final mask is applied).
 */
private fun writeFormatInfo(modules: Array<BooleanArray>, fmt: Int) {
    // Row 8, cols 1–8: bits f14 down to f7 (8 bits)
    for (i in 0..7) {
        modules[8][1 + i] = (fmt shr (14 - i)) and 1 == 1
    }
    // Col 8, rows 7 down to 1: bits f6 down to f0 (7 bits)
    for (i in 0..6) {
        modules[7 - i][8] = (fmt shr (6 - i)) and 1 == 1
    }
}

/**
 * Initialize a [WorkGrid] with all structural modules placed and reserved.
 *
 * Calls the four placement functions in order:
 * 1. [placeFinder] — 7×7 finder pattern
 * 2. [placeSeparator] — L-shaped light separator
 * 3. [placeTiming] — timing strips at row 0 / col 0
 * 4. [reserveFormatInfo] — reserves row 8 / col 8 format info area
 *
 * After this call, `g.reserved` marks every structural module.  The remaining
 * modules are all light and ready for data placement via [placeBits].
 */
private fun buildGrid(cfg: SymbolConfig): WorkGrid {
    val g = WorkGrid(cfg.size)
    placeFinder(g)
    placeSeparator(g)
    placeTiming(g)
    reserveFormatInfo(g)
    return g
}

// ============================================================================
// Data placement — two-column zigzag
// ============================================================================

/**
 * Place data bits into the grid using the two-column zigzag scan.
 *
 * Scans from the bottom-right corner, moving left two columns at a time,
 * alternating upward and downward.  Reserved modules are skipped; all others
 * receive the next bit from [bits] in sequence.
 *
 * ### Zigzag scan pattern (M2, 13×13, showing column pairs)
 *
 * ```
 * cols 12–11: scan upward   (rows 12 → 0)
 * cols 10–9:  scan downward (rows 0 → 12)
 * cols 8–7:   scan upward   (rows 12 → 0)  ← but row 8 and col 7/8 are reserved
 * …
 * col 1:      (col 0 is timing, always reserved, so last pair is 2–1)
 * ```
 *
 * Unlike regular QR, there is no col 6 timing strip to skip; Micro QR timing
 * is at col 0, which is already in the reserved set.
 *
 * @param g    Work grid to fill.
 * @param bits Bit stream (true = dark module).  Extra bits are ignored; missing
 *             bits produce light (false) modules.
 */
private fun placeBits(g: WorkGrid, bits: BooleanArray) {
    var bitIdx = 0
    var up = true  // first column pair scans upward (from bottom to top)

    var col = g.size - 1
    while (col >= 1) {
        for (vi in 0 until g.size) {
            val row = if (up) g.size - 1 - vi else vi
            for (dc in 0..1) {
                val c = col - dc
                if (g.reserved[row][c]) continue
                g.modules[row][c] = bitIdx < bits.size && bits[bitIdx++]
            }
        }
        up = !up
        col -= 2
    }
}

// ============================================================================
// Masking
// ============================================================================

/**
 * Returns `true` if mask pattern [maskIdx] should flip module (row, col).
 *
 * Micro QR defines 4 mask patterns (subset of regular QR's 8):
 *
 * ```
 * Pattern 0: (row + col) mod 2 == 0   — checkerboard
 * Pattern 1: row mod 2 == 0           — horizontal stripes
 * Pattern 2: col mod 3 == 0           — vertical stripes
 * Pattern 3: (row + col) mod 3 == 0   — diagonal stripes
 * ```
 *
 * The simpler patterns 4–7 from regular QR are unnecessary for the smaller
 * Micro QR grid sizes.
 *
 * This function is `internal` (accessible from tests).
 */
internal fun maskCondition(maskIdx: Int, row: Int, col: Int): Boolean = when (maskIdx) {
    0 -> (row + col) % 2 == 0
    1 -> row % 2 == 0
    2 -> col % 3 == 0
    3 -> (row + col) % 3 == 0
    else -> false
}

/**
 * Apply mask [maskIdx] to all non-reserved modules, returning a new module array.
 *
 * A mask XORs (flips) every data/ECC module where [maskCondition] is true.
 * Structural modules (finder, separator, timing, format info) are never touched.
 *
 * Flipping breaks up long runs of dark or light modules that could interfere
 * with the scanner's ability to track timing patterns and decode data.
 *
 * @param modules   The unmasked module array.
 * @param reserved  The reservation flags.
 * @param sz        Symbol side length.
 * @param maskIdx   Mask pattern to apply (0–3).
 * @return A new module array with the mask applied (original unchanged).
 */
private fun applyMask(
    modules: Array<BooleanArray>,
    reserved: Array<BooleanArray>,
    sz: Int,
    maskIdx: Int,
): Array<BooleanArray> {
    return Array(sz) { r ->
        BooleanArray(sz) { c ->
            if (!reserved[r][c]) {
                modules[r][c] != maskCondition(maskIdx, r, c)
            } else {
                modules[r][c]
            }
        }
    }
}

// ============================================================================
// Penalty scoring
// ============================================================================

/**
 * Compute the four-rule penalty score for a masked grid.
 *
 * All four masks are evaluated and the one with the lowest penalty is chosen.
 * This is the same penalty system as regular QR Code.  Ties go to the
 * lower-index mask.
 *
 * ### Rule 1 — Adjacent same-color runs
 *
 * Scan every row and every column for runs of ≥ 5 consecutive modules of the
 * same color.  For each qualifying run of length L, add `L − 2` to penalty.
 * (Run of 5 → +3, run of 6 → +4, etc.)
 *
 * ### Rule 2 — 2×2 same-color blocks
 *
 * For each 2×2 square where all four modules have the same color (all dark or
 * all light), add 3 to penalty.
 *
 * ### Rule 3 — Finder-pattern-like sequences
 *
 * Scan every row and column for the 11-module sequence:
 * ```
 * 1 0 1 1 1 0 1 0 0 0 0
 * ```
 * or its mirror:
 * ```
 * 0 0 0 0 1 0 1 1 1 0 1
 * ```
 * Each occurrence adds 40 to penalty.  These patterns resemble the finder
 * pattern and would confuse scanner detection algorithms.
 *
 * ### Rule 4 — Dark-module proportion
 *
 * A healthy symbol has approximately 50% dark modules.  The penalty for
 * imbalance:
 * ```
 * darkPct = (darkCount × 100) / totalModules
 * prev5   = (darkPct / 5) × 5       ← largest multiple of 5 ≤ darkPct
 * next5   = prev5 + 5
 * r4      = min(|prev5 − 50|, |next5 − 50|) / 5 × 10
 * ```
 *
 * This function is `internal` (accessible from tests).
 *
 * @param modules The (already masked) module array.
 * @param sz      Symbol side length.
 * @return The total penalty score (lower is better).
 */
internal fun computePenalty(modules: Array<BooleanArray>, sz: Int): Int {
    var penalty = 0

    // ── Rule 1: adjacent same-color runs of ≥ 5 ──────────────────────────────
    for (a in 0 until sz) {
        // Scan row a
        var run = 1
        var prev = modules[a][0]
        for (i in 1 until sz) {
            val cur = modules[a][i]
            if (cur == prev) {
                run++
            } else {
                if (run >= 5) penalty += run - 2
                run = 1
                prev = cur
            }
        }
        if (run >= 5) penalty += run - 2

        // Scan column a
        run = 1
        prev = modules[0][a]
        for (i in 1 until sz) {
            val cur = modules[i][a]
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

    // ── Rule 2: 2×2 same-color blocks ────────────────────────────────────────
    for (r in 0 until sz - 1) {
        for (c in 0 until sz - 1) {
            val d = modules[r][c]
            if (d == modules[r][c + 1] && d == modules[r + 1][c] && d == modules[r + 1][c + 1]) {
                penalty += 3
            }
        }
    }

    // ── Rule 3: finder-pattern-like sequences ─────────────────────────────────
    // Need at least 11 modules in a line; M1 (11×11) is the minimum that can match.
    if (sz >= 11) {
        val p1 = intArrayOf(1, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0)
        val p2 = intArrayOf(0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1)
        val limit = sz - 11
        for (a in 0 until sz) {
            for (b in 0..limit) {
                var mh1 = true; var mh2 = true
                var mv1 = true; var mv2 = true
                for (k in 0..10) {
                    val bh = if (modules[a][b + k]) 1 else 0
                    val bv = if (modules[b + k][a]) 1 else 0
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
    }

    // ── Rule 4: dark proportion deviation from 50% ───────────────────────────
    var dark = 0
    for (row in modules) for (m in row) if (m) dark++
    val total = sz * sz
    val darkPct = dark * 100 / total
    val prev5 = darkPct / 5 * 5
    val next5 = prev5 + 5
    val r4 = minOf(Math.abs(prev5 - 50), Math.abs(next5 - 50))
    penalty += r4 / 5 * 10

    return penalty
}

// ============================================================================
// Core encoding pipeline
// ============================================================================

/**
 * Core encoding pipeline: given a resolved config, mode, and optional mask
 * override, produce the final [ModuleGrid].
 *
 * ### Steps
 *
 * 1. Build data codewords (mode indicator + char count + data + terminator + padding).
 * 2. Compute RS ECC codewords and concatenate.
 * 3. Flatten codeword stream to a boolean bit array.
 * 4. Initialize structural grid (finder, separator, timing, format reservation).
 * 5. Place data bits via two-column zigzag.
 * 6. Evaluate all 4 masks (or use forced mask) and pick lowest penalty.
 * 7. Apply best mask and write final format information word.
 * 8. Wrap in immutable [ModuleGrid] and return.
 *
 * @param input        String to encode.
 * @param cfg          Resolved symbol configuration.
 * @param mode         Chosen encoding mode.
 * @param forcedMask   If non-null, skip evaluation and use this mask directly.
 * @return The final [ModuleGrid].
 */
private fun encodeWithConfig(
    input: String,
    cfg: SymbolConfig,
    mode: EncodingMode,
    forcedMask: Int?,
): ModuleGrid {
    // ── Step 1: Build data codewords ─────────────────────────────────────────
    val dataCw = buildDataCodewords(input, cfg, mode)

    // ── Step 2: Reed-Solomon ECC ──────────────────────────────────────────────
    val gen = getGenerator(cfg.eccCw)
    val eccCw = rsEncode(dataCw, gen)

    // ── Step 3: Flatten codewords → bit array ────────────────────────────────
    // For M1: the last data codeword contributes only 4 bits (upper nibble).
    // For all other configs: every codeword contributes 8 bits.
    val totalBits = run {
        var n = 0
        for (i in dataCw.indices) n += if (cfg.m1HalfCw && i == cfg.dataCw - 1) 4 else 8
        n += eccCw.size * 8
        n
    }

    val bits = BooleanArray(totalBits)
    var bitIdx = 0

    for (i in dataCw.indices) {
        val bitsInCw = if (cfg.m1HalfCw && i == cfg.dataCw - 1) 4 else 8
        val cw = dataCw[i].toInt() and 0xFF
        // Extract bits from MSB to LSB, but only the top [bitsInCw] bits.
        // For a half-codeword (4 bits), the data is in bits 7..4 of the byte.
        for (b in bitsInCw - 1 downTo 0) {
            bits[bitIdx++] = (cw shr (b + (8 - bitsInCw))) and 1 == 1
        }
    }
    for (b in eccCw) {
        val cw = b.toInt() and 0xFF
        for (bit in 7 downTo 0) {
            bits[bitIdx++] = (cw shr bit) and 1 == 1
        }
    }

    // ── Step 4: Initialize structural grid ───────────────────────────────────
    val grid = buildGrid(cfg)

    // ── Step 5: Place data bits ───────────────────────────────────────────────
    placeBits(grid, bits)

    // ── Step 6: Mask selection ────────────────────────────────────────────────
    val sz = cfg.size

    val bestMask: Int = if (forcedMask != null) {
        forcedMask
    } else {
        var best = 0
        var bestPenalty = Int.MAX_VALUE
        for (m in 0..3) {
            val masked = applyMask(grid.modules, grid.reserved, sz, m)
            val fmt = FORMAT_TABLE[cfg.symbolIndicator][m]
            // Write format info into a temporary copy for penalty scoring.
            val tmp = Array(sz) { r -> masked[r].copyOf() }
            writeFormatInfo(tmp, fmt)
            val p = computePenalty(tmp, sz)
            if (p < bestPenalty) {
                bestPenalty = p
                best = m
            }
        }
        best
    }

    // ── Step 7: Apply best mask and write format information ──────────────────
    val finalModules = applyMask(grid.modules, grid.reserved, sz, bestMask)
    val finalFmt = FORMAT_TABLE[cfg.symbolIndicator][bestMask]
    writeFormatInfo(finalModules, finalFmt)

    // ── Step 8: Wrap in immutable ModuleGrid ──────────────────────────────────
    // Use Collections.unmodifiableList on both layers so that callers cannot
    // mutate the grid after it is returned.  This matches the Java reference
    // implementation and the ModuleGrid contract documented in barcode-2d.
    val modulesList: List<List<Boolean>> = Collections.unmodifiableList(
        List(sz) { r ->
            Collections.unmodifiableList(
                List(sz) { c -> finalModules[r][c] }
            )
        }
    )

    return ModuleGrid(
        rows = sz,
        cols = sz,
        modules = modulesList,
        moduleShape = ModuleShape.SQUARE,
    )
}

// ============================================================================
// Bit writer helper
// ============================================================================

/**
 * Accumulates bits MSB-first, then converts to a byte array or a raw int array.
 *
 * Each call to [write] appends [count] least-significant bits of [value] to
 * the internal stream, MSB of those bits first.  This matches QR/Micro-QR's
 * big-endian bit ordering within each codeword.
 *
 * ### Example
 *
 * ```kotlin
 * val w = BitWriter()
 * w.write(0b101, 3)      // appends bits: 1, 0, 1
 * w.write(0b11, 2)       // appends bits: 1, 1
 * w.toBytes()            // → [0b10111000] = [0xB8]
 * ```
 */
private class BitWriter {
    private val bits = mutableListOf<Int>()  // each element is 0 or 1

    /** Number of bits written so far. */
    val bitLen: Int get() = bits.size

    /**
     * Append the [count] least-significant bits of [value], MSB first.
     *
     * Example: `write(0b101, 3)` → appends 1, 0, 1 (not 0, 1, 1).
     */
    fun write(value: Int, count: Int) {
        for (i in count - 1 downTo 0) {
            bits.add((value shr i) and 1)
        }
    }

    /**
     * Convert the bit stream to a byte array.
     *
     * Groups of 8 bits are packed into one byte (MSB first).  If the number
     * of bits is not a multiple of 8, the last byte is zero-padded on the right.
     */
    fun toBytes(): ByteArray {
        val nBytes = (bits.size + 7) / 8
        val result = ByteArray(nBytes)
        for (i in bits.indices) {
            if (bits[i] == 1) {
                result[i / 8] = (result[i / 8].toInt() or (1 shl (7 - i % 8))).toByte()
            }
        }
        return result
    }

    /**
     * Return the raw bit stream as an [IntArray] (each element 0 or 1).
     *
     * Used by the M1 half-codeword packing logic, which needs to index
     * individual bits before packing them into three bytes.
     */
    fun toBitArray(): IntArray = bits.toIntArray()
}
